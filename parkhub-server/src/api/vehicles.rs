//! Vehicle handlers: list, create, update, delete, photo upload/download,
//! and German licence-plate city-code reference data.

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use base64::Engine;
use chrono::Utc;
use uuid::Uuid;

use parkhub_common::{ApiResponse, Vehicle, VehicleType};

use crate::audit::{AuditEntry, AuditEventType};
use crate::requests::VehicleRequest;

use super::{AuthUser, SharedState, MAX_PHOTO_BYTES};

// ─────────────────────────────────────────────────────────────────────────────
// Request types
// ─────────────────────────────────────────────────────────────────────────────

/// Request body for uploading a vehicle photo as base64-encoded image data.
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct VehiclePhotoUpload {
    /// Base64-encoded image, optionally prefixed with a data URI scheme
    /// (e.g. `data:image/jpeg;base64,...`).
    photo: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Detect image format from decoded bytes via magic number.
/// Returns the MIME content-type string or `None` if unrecognised.
fn detect_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        Some("image/jpeg")
    } else if bytes.len() >= 4
        && bytes[0] == 0x89
        && bytes[1] == 0x50
        && bytes[2] == 0x4E
        && bytes[3] == 0x47
    {
        Some("image/png")
    } else {
        None
    }
}

/// Strip an optional `data:<mime>;base64,` prefix and return the raw base64 payload.
fn strip_data_uri_prefix(input: &str) -> &str {
    input
        .find(";base64,")
        .map_or(input, |pos| &input[pos + 8..])
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

#[utoipa::path(get, path = "/api/v1/vehicles", tag = "Vehicles",
    summary = "List user's vehicles",
    description = "Returns all vehicles registered by the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "List of vehicles"))
)]
pub async fn list_vehicles(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<Vehicle>>> {
    let state = state.read().await;

    match state
        .db
        .list_vehicles_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(vehicles) => Json(ApiResponse::success(vehicles)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list vehicles",
            ))
        }
    }
}

#[utoipa::path(post, path = "/api/v1/vehicles", tag = "Vehicles",
    summary = "Register a new vehicle",
    description = "Adds a vehicle to the authenticated user's account.",
    security(("bearer_auth" = [])),
    request_body = VehicleRequest,
    responses((status = 201, description = "Vehicle created"))
)]
pub async fn create_vehicle(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<VehicleRequest>,
) -> (StatusCode, Json<ApiResponse<Vehicle>>) {
    let vehicle = Vehicle {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        license_plate: req.license_plate,
        make: req.make,
        model: req.model,
        color: req.color,
        vehicle_type: req
            .vehicle_type
            .map(|t| serde_json::from_value(serde_json::Value::String(t)).unwrap_or_default())
            .unwrap_or_default(),
        is_default: req.is_default,
        created_at: Utc::now(),
    };

    let state_guard = state.read().await;
    if let Err(e) = state_guard.db.save_vehicle(&vehicle).await {
        tracing::error!("Failed to save vehicle: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create vehicle",
            )),
        );
    }

    let username = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();

    AuditEntry::new(AuditEventType::VehicleAdded)
        .user(auth_user.user_id, &username)
        .log();

    (StatusCode::CREATED, Json(ApiResponse::success(vehicle)))
}

/// Delete a vehicle owned by the authenticated user.
#[utoipa::path(delete, path = "/api/v1/vehicles/{id}", tag = "Vehicles",
    summary = "Delete a vehicle", description = "Removes a vehicle. Only the owner can delete.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "Vehicle UUID")),
    responses((status = 200, description = "Deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
pub async fn delete_vehicle(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    let vehicle = match state_guard.db.get_vehicle(&id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Vehicle not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching vehicle: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if vehicle.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    let username = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();

    match state_guard.db.delete_vehicle(&id).await {
        Ok(true) => {
            AuditEntry::new(AuditEventType::VehicleRemoved)
                .user(auth_user.user_id, &username)
                .log();
            tracing::info!(
                user_id = %auth_user.user_id,
                vehicle_id = %id,
                "Vehicle deleted"
            );
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Vehicle not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete vehicle {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to delete vehicle",
                )),
            )
        }
    }
}

/// `PUT /api/v1/vehicles/{id}` — update vehicle details
#[utoipa::path(put, path = "/api/v1/vehicles/{id}", tag = "Vehicles",
    summary = "Update a vehicle", description = "Updates vehicle details. Only the owner can update.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "Vehicle UUID")),
    responses((status = 200, description = "Updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
pub async fn update_vehicle(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<Vehicle>>) {
    let state_guard = state.read().await;

    let mut vehicle = match state_guard.db.get_vehicle(&id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Vehicle not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if vehicle.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    if let Some(plate) = req.get("license_plate").and_then(|v| v.as_str()) {
        vehicle.license_plate = plate.to_string();
    }
    if let Some(make) = req.get("make").and_then(|v| v.as_str()) {
        vehicle.make = Some(make.to_string());
    }
    if let Some(model) = req.get("model").and_then(|v| v.as_str()) {
        vehicle.model = Some(model.to_string());
    }
    if let Some(color) = req.get("color").and_then(|v| v.as_str()) {
        vehicle.color = Some(color.to_string());
    }
    if let Some(is_default) = req.get("is_default").and_then(serde_json::Value::as_bool) {
        vehicle.is_default = is_default;
    }

    if let Err(e) = state_guard.db.save_vehicle(&vehicle).await {
        tracing::error!("Failed to update vehicle: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update vehicle",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(vehicle)))
}

/// `POST /api/v1/vehicles/{id}/photo` — upload a vehicle photo (base64 JSON body).
#[utoipa::path(post, path = "/api/v1/vehicles/{id}/photo", tag = "Vehicles",
    summary = "Upload vehicle photo",
    description = "Uploads a base64-encoded vehicle photo (JPEG or PNG, max 2 MB).",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "Vehicle UUID")),
    responses((status = 200, description = "Photo uploaded"), (status = 400, description = "Invalid image"), (status = 404, description = "Not found"))
)]
pub async fn upload_vehicle_photo(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<VehiclePhotoUpload>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    let vehicle = match state_guard.db.get_vehicle(&id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Vehicle not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if vehicle.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    let b64_payload = strip_data_uri_prefix(&req.photo);

    let Ok(raw_bytes) = base64::engine::general_purpose::STANDARD.decode(b64_payload) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "Invalid base64 data")),
        );
    };

    if raw_bytes.len() > MAX_PHOTO_BYTES {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "PAYLOAD_TOO_LARGE",
                "Photo exceeds 2 MB limit",
            )),
        );
    }

    if detect_image_mime(&raw_bytes).is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Unsupported image format. Only JPEG and PNG are accepted.",
            )),
        );
    }

    let key = format!("vehicle_photo_{id}");
    let value = req.photo.clone();

    if let Err(e) = state_guard.db.set_setting(&key, &value).await {
        tracing::error!("Failed to save vehicle photo: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save photo")),
        );
    }

    tracing::info!(vehicle_id = %id, bytes = raw_bytes.len(), "Vehicle photo uploaded");
    (StatusCode::OK, Json(ApiResponse::success(())))
}

/// `GET /api/v1/vehicles/{id}/photo` — download a vehicle photo.
#[utoipa::path(get, path = "/api/v1/vehicles/{id}/photo", tag = "Vehicles",
    summary = "Download vehicle photo", description = "Returns the stored vehicle photo as binary.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "Vehicle UUID")),
    responses((status = 200, description = "Photo bytes"), (status = 404, description = "No photo"))
)]
pub async fn get_vehicle_photo(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Response {
    let state_guard = state.read().await;

    let Ok(Some(vehicle)) = state_guard.db.get_vehicle(&id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("NOT_FOUND", "Vehicle not found")),
        )
            .into_response();
    };

    if vehicle.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<()>::error("FORBIDDEN", "Access denied")),
        )
            .into_response();
    }

    let key = format!("vehicle_photo_{id}");
    let Ok(Some(stored)) = state_guard.db.get_setting(&key).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("NOT_FOUND", "No photo found")),
        )
            .into_response();
    };

    let b64_payload = strip_data_uri_prefix(&stored);

    let Ok(raw_bytes) = base64::engine::general_purpose::STANDARD.decode(b64_payload) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(
                "SERVER_ERROR",
                "Corrupt photo data",
            )),
        )
            .into_response();
    };

    let content_type = detect_image_mime(&raw_bytes).unwrap_or("application/octet-stream");

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        raw_bytes,
    )
        .into_response()
}

/// `GET /api/v1/vehicles/city-codes` — return German licence-plate city codes.
#[utoipa::path(get, path = "/api/v1/vehicles/city-codes", tag = "Vehicles",
    summary = "German license plate city codes",
    description = "Returns a map of German Kfz-Kennzeichen area codes to city names.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "City code map"))
)]
pub async fn vehicle_city_codes(
) -> Json<ApiResponse<std::collections::HashMap<&'static str, &'static str>>> {
    // Top ~75 German Kfz-Kennzeichen area codes → city/district names.
    // Built once per call (tiny cost) to avoid the serde_json::json! recursion limit.
    let codes: std::collections::HashMap<&str, &str> = [
        ("A", "Augsburg"),
        ("AA", "Aalen"),
        ("AB", "Aschaffenburg"),
        ("AC", "Aachen"),
        ("AK", "Altenkirchen"),
        ("B", "Berlin"),
        ("BA", "Bamberg"),
        ("BB", "Böblingen"),
        ("BC", "Biberach"),
        ("BN", "Bonn"),
        ("C", "Chemnitz"),
        ("CB", "Cottbus"),
        ("CE", "Celle"),
        ("CO", "Coburg"),
        ("CW", "Calw"),
        ("D", "Düsseldorf"),
        ("DA", "Darmstadt"),
        ("DD", "Dresden"),
        ("DE", "Dessau"),
        ("DO", "Dortmund"),
        ("DU", "Duisburg"),
        ("E", "Essen"),
        ("EM", "Emmendingen"),
        ("ER", "Erlangen"),
        ("ES", "Esslingen"),
        ("F", "Frankfurt am Main"),
        ("FB", "Fulda"),
        ("FR", "Freiburg im Breisgau"),
        ("FRG", "Freyung-Grafenau"),
        ("FDS", "Freudenstadt"),
        ("G", "Gera"),
        ("GE", "Gelsenkirchen"),
        ("GI", "Gießen"),
        ("GL", "Rheinisch-Bergischer Kreis"),
        ("GM", "Oberbergischer Kreis"),
        ("GÖ", "Göttingen"),
        ("H", "Hannover"),
        ("HA", "Hagen"),
        ("HB", "Bremen"),
        ("HD", "Heidelberg"),
        ("HE", "Helmstedt"),
        ("HH", "Hamburg"),
        ("HN", "Heilbronn"),
        ("HR", "Schwalm-Eder-Kreis"),
        ("HS", "Heinsberg"),
        ("HX", "Höxter"),
        ("I", "Innsbruck (AT)"),
        ("IN", "Ingolstadt"),
        ("J", "Jena"),
        ("K", "Köln"),
        ("KA", "Karlsruhe"),
        ("KB", "Waldeck-Frankenberg"),
        ("KI", "Kiel"),
        ("KL", "Kaiserslautern"),
        ("KLE", "Kleve"),
        ("KN", "Konstanz"),
        ("KS", "Kassel"),
        ("L", "Leipzig"),
        ("LA", "Landshut"),
        ("LD", "Landau"),
        ("LDS", "Dahme-Spreewald"),
        ("LE", "Esslingen am Neckar"),
        ("LEV", "Leverkusen"),
        ("LG", "Lüneburg"),
        ("LI", "Lindau"),
        ("LK", "Ludwigslust-Parchim"),
        ("LN", "Lahn-Dill-Kreis"),
        ("LU", "Ludwigshafen"),
        ("M", "München"),
        ("MA", "Mannheim"),
        ("MB", "Miesbach"),
        ("MD", "Magdeburg"),
        ("ME", "Mettmann"),
        ("MG", "Mönchengladbach"),
        ("MI", "Minden-Lübbecke"),
        ("MIL", "Miltenberg"),
        ("MK", "Märkisches Sauerland"),
        ("MM", "Memmingen"),
        ("MN", "Unterallgäu"),
        ("MO", "Montabaur"),
        ("MS", "Münster"),
        ("N", "Nürnberg"),
        ("NA", "Neustadt an der Aisch"),
        ("NB", "Neubrandenburg"),
        ("NE", "Rhein-Kreis Neuss"),
        ("NEA", "Neustadt/Aisch-Bad Windsheim"),
        ("NES", "Rhön-Grabfeld"),
        ("NH", "Nordhausen"),
        ("NK", "Neunkirchen"),
        ("NMS", "Neumünster"),
        ("NO", "Nordfriesland"),
        ("NOM", "Northeim"),
        ("NOR", "Aurich"),
        ("NR", "Neustadt/Rheinland-Pfalz"),
        ("NU", "Neu-Ulm"),
        ("NW", "Neustadt an der Weinstraße"),
        ("O", "Oldenburg (Oldb)"),
        ("OA", "Oberallgäu"),
        ("OAL", "Ostallgäu"),
        ("OB", "Oberhausen"),
        ("OD", "Stormarn"),
        ("OE", "Olpe"),
        ("OG", "Ortenaukreis"),
        ("OH", "Ostholstein"),
        ("OK", "Ohrekreis"),
        ("OL", "Oldenburg (Land)"),
        ("OPP", "Amberg-Sulzbach"),
        ("OR", "Ortenaukreis"),
        ("OS", "Osnabrück"),
        ("OVP", "Vorpommern-Greifswald"),
        ("P", "Potsdam"),
        ("PA", "Passau"),
        ("PAN", "Rottal-Inn"),
        ("PB", "Paderborn"),
        ("PE", "Peine"),
        ("PF", "Pforzheim"),
        ("PI", "Pinneberg"),
        ("PL", "Plauen"),
        ("PM", "Potsdam-Mittelmark"),
        ("PN", "Pfarrkirchen"),
        ("PRZ", "Prignitz"),
        ("R", "Regensburg"),
        ("RA", "Rastatt"),
        ("RB", "Rottweil"),
        ("RE", "Recklinghausen"),
        ("REG", "Regen"),
        ("REI", "Berchtesgadener Land"),
        ("REK", "Rhein-Erft-Kreis"),
        ("RG", "Rotenburg (Wümme)"),
        ("RH", "Roth"),
        ("RI", "Rinteln"),
        ("RK", "Rhein-Kreis Neuss"),
        ("RM", "Rosenheim"),
        ("RN", "Rottenburg am Neckar"),
        ("RO", "Rosenheim"),
        ("ROK", "Rokycany"),
        ("ROT", "Rotenburg"),
        ("RP", "Rhein-Pfalz-Kreis"),
        ("RS", "Remscheid"),
        ("RSO", "Ravensburg"),
        ("RT", "Reutlingen"),
        ("RÜD", "Rheingau-Taunus-Kreis"),
        ("RW", "Rottweil"),
        ("S", "Stuttgart"),
        ("SAW", "Altmarkkreis Salzwedel"),
        ("SB", "Saarbrücken"),
        ("SC", "Schwabach"),
        ("SDH", "Nordhausen"),
        ("SE", "Segeberg"),
        ("SH", "Schleswig"),
        ("SI", "Siegen-Wittgenstein"),
        ("SK", "Saale-Kronach"),
        ("SL", "Schleswig-Flensburg"),
        ("SLF", "Saalfeld-Rudolstadt"),
        ("SM", "St. Wendel"),
        ("SN", "Schwerin"),
        ("SO", "Soest"),
        ("SOM", "Sonneberg"),
        ("SPD", "Spree-Neiße"),
        ("SPG", "Südliche Weinstraße"),
        ("SPK", "Spandau"),
        ("SPN", "Spree-Neiße"),
        ("SR", "Straubing"),
        ("ST", "Steinfurt"),
        ("STD", "Stade"),
        ("STE", "Stendal"),
        ("STL", "Stollberg"),
        ("STR", "Straubing-Bogen"),
        ("SU", "Rhein-Sieg-Kreis"),
        ("SüW", "Südliche Weinstraße"),
        ("SW", "Schweinfurt"),
        ("T", "Traunstein"),
        ("TA", "Darmstadt-Dieburg"),
        ("TBB", "Main-Tauber-Kreis"),
        ("TF", "Teltow-Fläming"),
        ("TG", "Torgau-Oschatz"),
        ("TIR", "Tirschenreuth"),
        ("TÖL", "Bad Tölz-Wolfratshausen"),
        ("TRI", "Trier-Saarburg"),
        ("TS", "Traunstein"),
        ("TS2", "Traunstein"),
        ("TÜ", "Tübingen"),
        ("TUT", "Tuttlingen"),
        ("UB", "Uffenheim"),
        ("UCK", "Uckermark"),
        ("UE", "Uelzen"),
        ("UH", "Unstrut-Hainich-Kreis"),
        ("UL", "Ulm"),
        ("UN", "Unna"),
        ("UNS", "Unterallgäu"),
        ("USI", "Usingen"),
        ("V", "Vogtlandkreis"),
        ("VB", "Vogelsbergkreis"),
        ("VEC", "Vechta"),
        ("VER", "Verden"),
        ("VIE", "Viersen"),
        ("VK", "Völklingen"),
        ("VKL", "Vorpommern-Rügen"),
        ("VLD", "Waldshut"),
        ("VR", "Vorpommern-Rügen"),
        ("VRK", "Vogelsbergkreis"),
        ("VRN", "Vorpommern-Rügen"),
        ("VW", "Wolfsburg"),
        ("W", "Wuppertal"),
        ("WA", "Waldeck-Frankenberg"),
        ("WAF", "Warendorf"),
        ("WAT", "Wattenscheid"),
        ("WB", "Wittenberg"),
        ("WE", "Weimar"),
        ("WEN", "Weiden in der Oberpfalz"),
        ("WES", "Wesel"),
        ("WI", "Wiesbaden"),
        ("WIL", "Bernkastel-Wittlich"),
        ("WIT", "Witten"),
        ("WK", "Wittmund"),
        ("WL", "Harburg"),
        ("WM", "Weilheim-Schongau"),
        ("WN", "Waiblingen"),
        ("WND", "St. Wendel"),
        ("WOR", "Waldshut"),
        ("WOS", "Wonsheim"),
        ("WÜ", "Würzburg"),
        ("WUG", "Weißenburg-Gunzenhausen"),
        ("WUL", "Ludwigslust"),
        ("WUN", "Wunsiedel"),
        ("WW", "Westerwaldkreis"),
        ("WZL", "Weißenburg-Gunzenhausen"),
        ("X", "Externsteine"),
        ("Y", "Bundeswehr"),
        ("Z", "Zwickau"),
        ("ZE", "Anhalt-Bitterfeld"),
        ("ZW", "Zweibrücken"),
    ]
    .into();
    Json(ApiResponse::success(codes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_image_mime_jpeg() {
        let jpeg_magic = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_image_mime(&jpeg_magic), Some("image/jpeg"));
    }

    #[test]
    fn test_detect_image_mime_png() {
        let png_magic = [0x89, 0x50, 0x4E, 0x47];
        assert_eq!(detect_image_mime(&png_magic), Some("image/png"));
    }

    #[test]
    fn test_detect_image_mime_unknown() {
        let unknown = [0x00, 0x01, 0x02, 0x03];
        assert_eq!(detect_image_mime(&unknown), None);
    }

    #[test]
    fn test_detect_image_mime_too_short() {
        assert_eq!(detect_image_mime(&[0xFF, 0xD8]), None);
    }

    #[test]
    fn test_strip_data_uri_prefix_with_prefix() {
        let input = "data:image/jpeg;base64,/9j/4AAQ";
        assert_eq!(strip_data_uri_prefix(input), "/9j/4AAQ");
    }

    #[test]
    fn test_strip_data_uri_prefix_without_prefix() {
        let input = "/9j/4AAQ";
        assert_eq!(strip_data_uri_prefix(input), "/9j/4AAQ");
    }

    #[test]
    fn test_vehicle_photo_upload_deserialize() {
        let json = r#"{"photo": "data:image/png;base64,abc123"}"#;
        let req: VehiclePhotoUpload = serde_json::from_str(json).unwrap();
        assert_eq!(req.photo, "data:image/png;base64,abc123");
    }
}
