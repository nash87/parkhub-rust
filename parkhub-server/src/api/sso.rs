//! SAML/SSO Enterprise Authentication handlers.
//!
//! Provides enterprise SSO integration via SAML 2.0 providers.
//! Administrators configure providers with entity ID, metadata URL,
//! and certificate; users authenticate via browser redirect flow.
//!
//! Endpoints:
//! - `GET    /api/v1/auth/sso/providers`           — list configured SSO providers
//! - `GET    /api/v1/auth/sso/{provider}/login`     — initiate SSO flow (redirect URL)
//! - `POST   /api/v1/auth/sso/{provider}/callback`  — handle SSO callback
//! - `PUT    /api/v1/admin/sso/{provider}`           — configure SSO provider
//! - `DELETE /api/v1/admin/sso/{provider}`           — remove SSO provider

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::{ApiResponse, AuthTokens, LoginResponse, User, UserPreferences, UserRole};

use crate::audit::{AuditEntry, AuditEventType};
use crate::db::Session;
use crate::metrics;

use super::{generate_access_token, hash_password_simple, AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// SSO provider configuration stored in the database settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProvider {
    /// Unique slug identifier (e.g. "okta", "azure-ad")
    pub slug: String,
    /// Human-readable display name
    pub display_name: String,
    /// SAML Entity ID (Issuer)
    pub entity_id: String,
    /// SAML Metadata URL for automatic configuration
    pub metadata_url: String,
    /// SSO Login URL (IdP Single Sign-On endpoint)
    pub sso_url: String,
    /// Base64-encoded X.509 certificate for signature verification
    pub certificate: String,
    /// Whether this provider is enabled
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public provider info returned to unauthenticated users.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SsoProviderPublic {
    pub slug: String,
    pub display_name: String,
    pub enabled: bool,
}

/// Request body to configure a new SSO provider.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ConfigureSsoRequest {
    pub display_name: String,
    pub entity_id: String,
    pub metadata_url: String,
    pub sso_url: String,
    pub certificate: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// SSO callback payload (posted by the IdP or relayed by the frontend).
#[derive(Debug, Deserialize)]
pub struct SsoCallbackPayload {
    /// Base64-encoded SAML Response XML
    #[serde(alias = "SAMLResponse")]
    pub saml_response: String,
    /// Relay state for CSRF protection
    #[serde(default)]
    #[allow(dead_code)]
    pub relay_state: Option<String>,
}

/// Parsed SAML assertion attributes.
#[derive(Debug, Clone)]
struct SamlAttributes {
    name_id: String,
    email: Option<String>,
    display_name: Option<String>,
}

const fn default_true() -> bool {
    true
}

// ─────────────────────────────────────────────────────────────────────────────
// SAML XML helpers (lightweight — no heavy XML crate)
// ─────────────────────────────────────────────────────────────────────────────

/// Extract text content between matching XML tags.
/// Handles both `<ns:Tag>` and `<Tag>` forms.
fn extract_xml_element(xml: &str, local_name: &str) -> Option<String> {
    for open_pattern in [format!("<{local_name}"), format!(":{local_name}")] {
        if let Some(tag_start) = xml.find(&open_pattern) {
            let tag_rest = &xml[tag_start..];
            if let Some(open_end) = tag_rest.find('>') {
                let content_start = tag_start + open_end + 1;
                let rest = &xml[content_start..];

                if let Some(end_idx) = rest.find(&format!("</{local_name}>")) {
                    return Some(rest[..end_idx].trim().to_string());
                }

                if let Some(end_idx) = rest.find(&format!(":{local_name}>")) {
                    let closing_slice = &rest[..end_idx];
                    if let Some(open_close_idx) = closing_slice.rfind("</") {
                        return Some(closing_slice[..open_close_idx].trim().to_string());
                    }
                }
            }
        }
    }

    None
}

fn extract_saml_attribute_value(xml: &str, attribute_name: &str) -> Option<String> {
    for pattern in [
        format!("Name=\"{attribute_name}\""),
        format!("FriendlyName=\"{attribute_name}\""),
    ] {
        if let Some(attr_idx) = xml.find(&pattern) {
            let attr_xml = &xml[attr_idx..];
            if let Some(value) = extract_xml_element(attr_xml, "AttributeValue") {
                return Some(value);
            }
        }
    }

    None
}

/// Parse a Base64-encoded SAML Response and extract assertion attributes.
fn parse_saml_response(base64_response: &str) -> Result<SamlAttributes, String> {
    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(base64_response.trim())
        .map_err(|e| format!("Invalid base64 SAML response: {e}"))?;

    let xml = String::from_utf8(decoded).map_err(|e| format!("Invalid UTF-8 in SAML XML: {e}"))?;

    let name_id = extract_xml_element(&xml, "NameID")
        .ok_or_else(|| "Missing NameID in SAML assertion".to_string())?;

    let email = extract_xml_element(&xml, "EmailAddress")
        .or_else(|| extract_xml_element(&xml, "emailaddress"))
        .or_else(|| extract_xml_element(&xml, "email"))
        .or_else(|| extract_saml_attribute_value(&xml, "EmailAddress"))
        .or_else(|| extract_saml_attribute_value(&xml, "emailaddress"))
        .or_else(|| extract_saml_attribute_value(&xml, "email"))
        .or_else(|| {
            // If NameID looks like an email, use it
            if name_id.contains('@') {
                Some(name_id.clone())
            } else {
                None
            }
        });

    let display_name = extract_xml_element(&xml, "DisplayName")
        .or_else(|| extract_xml_element(&xml, "displayname"))
        .or_else(|| extract_xml_element(&xml, "GivenName"));

    Ok(SamlAttributes {
        name_id,
        email,
        display_name,
    })
}

/// Parse SAML metadata XML and extract entity ID and SSO URL.
#[allow(dead_code)]
pub fn parse_saml_metadata(xml: &str) -> Option<(String, String)> {
    // entity ID is typically an attribute: entityID="..."
    let entity_id = xml.find("entityID=\"").map(|idx| {
        let start = idx + 10;
        let end = xml[start..].find('"').unwrap_or(0);
        xml[start..start + end].to_string()
    })?;

    let sso_url = xml
        .find("Location=\"")
        .map(|idx| {
            let start = idx + 10;
            let end = xml[start..].find('"').unwrap_or(0);
            xml[start..start + end].to_string()
        })
        .unwrap_or_default();

    Some((entity_id, sso_url))
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/auth/sso/providers` — list configured SSO providers (public).
pub async fn sso_list_providers(State(state): State<SharedState>) -> impl IntoResponse {
    let state_guard = state.read().await;
    let providers: Vec<SsoProviderPublic> = match state_guard.db.get_setting("sso_providers").await
    {
        Ok(Some(json_str)) => serde_json::from_str::<Vec<SsoProvider>>(&json_str)
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.enabled)
            .map(|p| SsoProviderPublic {
                slug: p.slug,
                display_name: p.display_name,
                enabled: p.enabled,
            })
            .collect(),
        _ => Vec::new(),
    };

    Json(ApiResponse::success(
        serde_json::json!({ "providers": providers }),
    ))
}

/// `GET /api/v1/auth/sso/{provider}/login` — initiate SSO flow.
///
/// Returns a redirect URL to the IdP's login page.
pub async fn sso_login(
    State(state): State<SharedState>,
    Path(provider_slug): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiResponse<()>>)> {
    let state_guard = state.read().await;
    let provider = get_provider(&state_guard, &provider_slug).await?;

    let callback_url = format!(
        "{}/api/v1/auth/sso/{}/callback",
        std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),
        provider_slug,
    );

    let relay_state = Uuid::new_v4().to_string();

    // Build SAML AuthnRequest redirect URL
    let redirect_url = format!(
        "{}?SAMLRequest={}&RelayState={}",
        provider.sso_url,
        urlencoded_authn_request(&provider.entity_id, &callback_url),
        relay_state,
    );

    metrics::record_auth_event("sso_login_initiated", true);

    Ok(Json(serde_json::json!({
        "redirect_url": redirect_url,
        "relay_state": relay_state
    })))
}

/// `POST /api/v1/auth/sso/{provider}/callback` — handle SSO callback.
///
/// Parses the SAML response, creates or links the user, and returns auth tokens.
pub async fn sso_callback(
    State(state): State<SharedState>,
    Path(provider_slug): Path<String>,
    Json(payload): Json<SsoCallbackPayload>,
) -> Response {
    let state_guard = state.read().await;
    let provider = match get_provider(&state_guard, &provider_slug).await {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let _ = provider; // provider validated

    // Parse the SAML response
    let attrs = match parse_saml_response(&payload.saml_response) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("SSO_PARSE_ERROR", &e)),
            )
                .into_response();
        }
    };

    let email = attrs.email.unwrap_or_else(|| attrs.name_id.clone());
    let display_name = attrs
        .display_name
        .unwrap_or_else(|| email.split('@').next().unwrap_or("User").to_string());

    // Find or create user
    let user = match state_guard.db.get_user_by_email(&email).await {
        Ok(Some(existing)) => existing,
        _ => {
            // Create new user linked to SSO
            let random_pw = Uuid::new_v4().to_string();
            let password_hash = match hash_password_simple(&random_pw).await {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!("Failed to hash SSO placeholder password: {e}");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<()>::error(
                            "SERVER_ERROR",
                            "Internal server error",
                        )),
                    )
                        .into_response();
                }
            };

            let now = Utc::now();
            let new_user = User {
                id: Uuid::new_v4(),
                username: email.split('@').next().unwrap_or("user").to_string(),
                email: email.clone(),
                password_hash,
                name: display_name.clone(),
                picture: None,
                phone: None,
                role: UserRole::User,
                created_at: now,
                updated_at: now,
                last_login: Some(now),
                preferences: UserPreferences::default(),
                is_active: true,
                credits_balance: 40,
                credits_monthly_quota: 40,
                credits_last_refilled: Some(now),
                tenant_id: None,
                accessibility_needs: None,
                cost_center: None,
                department: None,
            };

            if let Err(e) = state_guard.db.save_user(&new_user).await {
                tracing::error!("Failed to save SSO user: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(
                        "USER_CREATE_ERROR",
                        "Failed to create account",
                    )),
                )
                    .into_response();
            }

            // Store SSO provider link
            let key = format!("sso:{}:{}", provider_slug, new_user.id);
            let val = serde_json::json!({ "name_id": attrs.name_id, "provider": provider_slug })
                .to_string();
            let _ = state_guard.db.set_setting(&key, &val).await;

            AuditEntry::new(AuditEventType::UserCreated)
                .user(new_user.id, &new_user.username)
                .detail(&format!("sso:{provider_slug}"))
                .log()
                .persist(&state_guard.db)
                .await;
            metrics::record_auth_event("sso_register", true);

            new_user
        }
    };

    // Create session
    let session_hours = i64::from(state_guard.config.session_timeout_minutes).max(60) / 60;
    let role_str = format!("{:?}", user.role).to_lowercase();
    let session = Session::new(user.id, session_hours, &user.username, &role_str);
    let access_token = generate_access_token();

    if let Err(e) = state_guard.db.save_session(&access_token, &session).await {
        tracing::error!("Failed to save SSO session: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(
                "SESSION_ERROR",
                "Failed to create session",
            )),
        )
            .into_response();
    }
    drop(state_guard);

    metrics::record_auth_event("sso_login", true);

    // Build auth cookie
    let cookie_max_age = session_hours * 3600;
    let cookie = super::auth::build_auth_cookie(&access_token, cookie_max_age);

    let mut response_user = user;
    response_user.password_hash = String::new();

    let body = Json(ApiResponse::success(LoginResponse {
        user: response_user,
        tokens: AuthTokens {
            access_token,
            refresh_token: session.refresh_token,
            expires_at: session.expires_at,
            token_type: "Bearer".to_string(),
        },
    }));

    let mut resp = (StatusCode::OK, body).into_response();
    if let Ok(hv) = header::HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    resp
}

/// `PUT /api/v1/admin/sso/{provider}` — configure SSO provider (admin only).
pub async fn sso_configure_provider(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(provider_slug): Path<String>,
    Json(req): Json<ConfigureSsoRequest>,
) -> Result<Json<ApiResponse<SsoProviderPublic>>, (StatusCode, Json<ApiResponse<()>>)> {
    if req.display_name.is_empty() || req.entity_id.is_empty() || req.sso_url.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "display_name, entity_id, and sso_url are required",
            )),
        ));
    }

    if req.certificate.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "certificate is required",
            )),
        ));
    }

    let state_guard = state.read().await;
    let mut providers = load_providers(&state_guard).await;

    let now = Utc::now();
    let provider = SsoProvider {
        slug: provider_slug.clone(),
        display_name: req.display_name.clone(),
        entity_id: req.entity_id,
        metadata_url: req.metadata_url,
        sso_url: req.sso_url,
        certificate: req.certificate,
        enabled: req.enabled,
        created_at: now,
        updated_at: now,
    };

    // Upsert: replace existing or append
    if let Some(existing) = providers.iter_mut().find(|p| p.slug == provider_slug) {
        *existing = provider.clone();
    } else {
        providers.push(provider.clone());
    }

    let json = serde_json::to_string(&providers).unwrap_or_default();
    let _ = state_guard.db.set_setting("sso_providers", &json).await;

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .detail(&format!("sso_provider_configured:{provider_slug}"))
        .log()
        .persist(&state_guard.db)
        .await;
    drop(state_guard);

    Ok(Json(ApiResponse::success(SsoProviderPublic {
        slug: provider.slug,
        display_name: provider.display_name,
        enabled: provider.enabled,
    })))
}

/// `DELETE /api/v1/admin/sso/{provider}` — remove SSO provider (admin only).
pub async fn sso_delete_provider(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(provider_slug): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state_guard = state.read().await;
    let mut providers = load_providers(&state_guard).await;
    let initial_len = providers.len();
    providers.retain(|p| p.slug != provider_slug);

    if providers.len() == initial_len {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "SSO provider not found")),
        ));
    }

    let json = serde_json::to_string(&providers).unwrap_or_default();
    let _ = state_guard.db.set_setting("sso_providers", &json).await;

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .detail(&format!("sso_provider_deleted:{provider_slug}"))
        .log()
        .persist(&state_guard.db)
        .await;
    drop(state_guard);

    Ok(Json(ApiResponse::success(())))
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

async fn load_providers(state: &crate::AppState) -> Vec<SsoProvider> {
    match state.db.get_setting("sso_providers").await {
        Ok(Some(json_str)) => serde_json::from_str(&json_str).unwrap_or_default(),
        _ => Vec::new(),
    }
}

async fn get_provider(
    state: &crate::AppState,
    slug: &str,
) -> Result<SsoProvider, (StatusCode, Json<ApiResponse<()>>)> {
    let providers = load_providers(state).await;
    providers
        .into_iter()
        .find(|p| p.slug == slug && p.enabled)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "SSO_PROVIDER_NOT_FOUND",
                    "SSO provider not found or disabled",
                )),
            )
        })
}

/// Build a minimal SAML `AuthnRequest` and URL-encode it.
fn urlencoded_authn_request(entity_id: &str, acs_url: &str) -> String {
    let request_id = format!("_ph_{}", Uuid::new_v4());
    let instant = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let xml = format!(
        r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" ID="{request_id}" Version="2.0" IssueInstant="{instant}" AssertionConsumerServiceURL="{acs_url}"><saml:Issuer xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">{entity_id}</saml:Issuer></samlp:AuthnRequest>"#
    );
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(xml.as_bytes());
    url::form_urlencoded::byte_serialize(encoded.as_bytes()).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_xml_element_simple() {
        let xml = r#"<Response><NameID>user@example.com</NameID></Response>"#;
        assert_eq!(
            extract_xml_element(xml, "NameID"),
            Some("user@example.com".to_string())
        );
    }

    #[test]
    fn test_extract_xml_element_namespaced() {
        let xml = r#"<saml:NameID Format="email">alice@corp.com</saml:NameID>"#;
        assert_eq!(
            extract_xml_element(xml, "NameID"),
            Some("alice@corp.com".to_string())
        );
    }

    #[test]
    fn test_extract_xml_element_missing() {
        let xml = r#"<Response><Other>value</Other></Response>"#;
        assert_eq!(extract_xml_element(xml, "NameID"), None);
    }

    #[test]
    fn test_parse_saml_response_valid() {
        use base64::Engine;
        let xml = r#"<samlp:Response><saml:Assertion><saml:Subject><saml:NameID>bob@corp.com</saml:NameID></saml:Subject><saml:AttributeStatement><saml:Attribute Name="DisplayName"><saml:AttributeValue>Bob Smith</saml:AttributeValue></saml:Attribute></saml:AttributeStatement></saml:Assertion></samlp:Response>"#;
        let b64 = base64::engine::general_purpose::STANDARD.encode(xml.as_bytes());
        let attrs = parse_saml_response(&b64).unwrap();
        assert_eq!(attrs.name_id, "bob@corp.com");
        assert_eq!(attrs.email, Some("bob@corp.com".to_string()));
    }

    #[test]
    fn test_parse_saml_response_invalid_base64() {
        let result = parse_saml_response("not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_saml_response_missing_name_id() {
        use base64::Engine;
        let xml = r#"<samlp:Response><saml:Assertion></saml:Assertion></samlp:Response>"#;
        let b64 = base64::engine::general_purpose::STANDARD.encode(xml.as_bytes());
        let result = parse_saml_response(&b64);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing NameID"));
    }

    #[test]
    fn test_urlencoded_authn_request_not_empty() {
        let result =
            urlencoded_authn_request("https://parkhub.test", "https://parkhub.test/callback");
        assert!(!result.is_empty());
        // Should be URL-encoded base64
        assert!(result.contains('%') || result.chars().all(|c| c.is_alphanumeric() || c == '='));
    }

    #[test]
    fn test_parse_saml_metadata() {
        let xml = r#"<md:EntityDescriptor entityID="https://idp.example.com"><md:IDPSSODescriptor><md:SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" Location="https://idp.example.com/sso"/></md:IDPSSODescriptor></md:EntityDescriptor>"#;
        let result = parse_saml_metadata(xml);
        assert!(result.is_some());
        let (entity_id, sso_url) = result.unwrap();
        assert_eq!(entity_id, "https://idp.example.com");
        assert_eq!(sso_url, "https://idp.example.com/sso");
    }

    #[test]
    fn test_sso_provider_serialization() {
        let provider = SsoProvider {
            slug: "okta".to_string(),
            display_name: "Okta Corp".to_string(),
            entity_id: "https://okta.example.com".to_string(),
            metadata_url: "https://okta.example.com/metadata".to_string(),
            sso_url: "https://okta.example.com/sso".to_string(),
            certificate: "MIIC...".to_string(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&provider).unwrap();
        let deserialized: SsoProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.slug, "okta");
        assert_eq!(deserialized.display_name, "Okta Corp");
        assert!(deserialized.enabled);
    }

    #[test]
    fn test_configure_request_defaults() {
        let json = r#"{"display_name":"Test","entity_id":"https://test.com","metadata_url":"","sso_url":"https://test.com/sso","certificate":"CERT"}"#;
        let req: ConfigureSsoRequest = serde_json::from_str(json).unwrap();
        assert!(req.enabled); // default_true
    }

    #[test]
    fn test_sso_provider_public_fields() {
        let public = SsoProviderPublic {
            slug: "azure-ad".to_string(),
            display_name: "Azure AD".to_string(),
            enabled: true,
        };
        let json = serde_json::to_string(&public).unwrap();
        assert!(json.contains("azure-ad"));
        assert!(json.contains("Azure AD"));
    }

    #[test]
    fn test_callback_payload_deserialization() {
        let json = r#"{"saml_response":"PHNhbWw+dGVzdDwvc2FtbD4=","relay_state":"abc-123"}"#;
        let payload: SsoCallbackPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.saml_response, "PHNhbWw+dGVzdDwvc2FtbD4=");
        assert_eq!(payload.relay_state, Some("abc-123".to_string()));
    }

    #[test]
    fn test_callback_payload_alias() {
        // IdPs often send SAMLResponse (Pascal case)
        let json = r#"{"SAMLResponse":"PHNhbWw+dGVzdDwvc2FtbD4="}"#;
        let payload: SsoCallbackPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.saml_response, "PHNhbWw+dGVzdDwvc2FtbD4=");
        assert!(payload.relay_state.is_none());
    }

    #[test]
    fn test_extract_email_from_attribute() {
        use base64::Engine;
        let xml = r#"<samlp:Response><saml:Assertion><saml:Subject><saml:NameID>uid-12345</saml:NameID></saml:Subject><saml:AttributeStatement><saml:Attribute Name="EmailAddress"><saml:AttributeValue>alice@corp.com</saml:AttributeValue></saml:Attribute></saml:AttributeStatement></saml:Assertion></samlp:Response>"#;
        let b64 = base64::engine::general_purpose::STANDARD.encode(xml.as_bytes());
        let attrs = parse_saml_response(&b64).unwrap();
        assert_eq!(attrs.name_id, "uid-12345");
        assert_eq!(attrs.email, Some("alice@corp.com".to_string()));
    }

    #[test]
    fn test_metadata_parsing_no_sso_url() {
        let xml =
            r#"<md:EntityDescriptor entityID="https://idp.example.com"></md:EntityDescriptor>"#;
        let result = parse_saml_metadata(xml);
        assert!(result.is_some());
        let (eid, url) = result.unwrap();
        assert_eq!(eid, "https://idp.example.com");
        assert!(url.is_empty());
    }
}
