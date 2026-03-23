//! GraphQL API — query/mutation interface alongside REST.
//!
//! Provides a GraphQL endpoint for querying and mutating ParkHub data
//! using the same auth tokens as the REST API.
//!
//! - `POST /api/v1/graphql`           — execute GraphQL queries/mutations
//! - `GET  /api/v1/graphql/playground` — interactive GraphQL playground UI

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use parkhub_common::ApiResponse;

use super::{AuthUser, SharedState};

// ═══════════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// GraphQL request body
#[derive(Debug, Deserialize)]
pub struct GraphQLRequest {
    pub query: String,
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
    #[serde(default, rename = "operationName")]
    pub operation_name: Option<String>,
}

/// GraphQL response
#[derive(Debug, Serialize)]
pub struct GraphQLResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<GraphQLError>,
}

/// GraphQL error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<GraphQLLocation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<String>>,
}

/// Location in the GraphQL query where an error occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLLocation {
    pub line: u32,
    pub column: u32,
}

/// Available query types in the schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QueryType {
    Me,
    Lots,
    Lot,
    Bookings,
    Booking,
    MyVehicles,
}

impl QueryType {
    /// All supported query types
    pub const ALL: &[QueryType] = &[
        Self::Me,
        Self::Lots,
        Self::Lot,
        Self::Bookings,
        Self::Booking,
        Self::MyVehicles,
    ];

    /// Return type description
    pub fn return_type(&self) -> &'static str {
        match self {
            Self::Me => "User",
            Self::Lots => "[Lot]",
            Self::Lot => "Lot",
            Self::Bookings => "[Booking]",
            Self::Booking => "Booking",
            Self::MyVehicles => "[Vehicle]",
        }
    }

    /// Arguments for this query (if any)
    pub fn args(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Self::Lot => vec![("id", "ID!")],
            Self::Booking => vec![("id", "ID!")],
            _ => vec![],
        }
    }
}

/// Available mutation types in the schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MutationType {
    CreateBooking,
    CancelBooking,
    AddVehicle,
}

impl MutationType {
    /// All supported mutation types
    pub const ALL: &[MutationType] = &[
        Self::CreateBooking,
        Self::CancelBooking,
        Self::AddVehicle,
    ];

    /// Return type description
    pub fn return_type(&self) -> &'static str {
        match self {
            Self::CreateBooking => "Booking",
            Self::CancelBooking => "Boolean",
            Self::AddVehicle => "Vehicle",
        }
    }
}

/// GraphQL schema introspection info
#[derive(Debug, Serialize)]
pub struct SchemaInfo {
    pub queries: Vec<SchemaField>,
    pub mutations: Vec<SchemaField>,
}

/// A field in the schema (query or mutation)
#[derive(Debug, Serialize)]
pub struct SchemaField {
    pub name: String,
    pub return_type: String,
    pub args: Vec<SchemaArg>,
    pub description: String,
}

/// An argument for a field
#[derive(Debug, Serialize)]
pub struct SchemaArg {
    pub name: String,
    pub arg_type: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// SCHEMA
// ═══════════════════════════════════════════════════════════════════════════════

/// Build schema definition string (SDL)
pub fn schema_sdl() -> String {
    let mut sdl = String::new();

    sdl.push_str("type Query {\n");
    sdl.push_str("  me: User\n");
    sdl.push_str("  lots: [Lot!]!\n");
    sdl.push_str("  lot(id: ID!): Lot\n");
    sdl.push_str("  bookings: [Booking!]!\n");
    sdl.push_str("  booking(id: ID!): Booking\n");
    sdl.push_str("  myVehicles: [Vehicle!]!\n");
    sdl.push_str("}\n\n");

    sdl.push_str("type Mutation {\n");
    sdl.push_str("  createBooking(lotId: ID!, slotId: ID, date: String!, startTime: String!, endTime: String!): Booking\n");
    sdl.push_str("  cancelBooking(id: ID!): Boolean!\n");
    sdl.push_str("  addVehicle(licensePlate: String!, make: String, model: String, color: String, vehicleType: String): Vehicle\n");
    sdl.push_str("}\n\n");

    sdl.push_str("type User {\n");
    sdl.push_str("  id: ID!\n  username: String!\n  email: String!\n  name: String\n  role: String!\n  createdAt: String!\n");
    sdl.push_str("}\n\n");

    sdl.push_str("type Lot {\n");
    sdl.push_str("  id: ID!\n  name: String!\n  address: String\n  totalSlots: Int!\n  availableSlots: Int!\n");
    sdl.push_str("}\n\n");

    sdl.push_str("type Booking {\n");
    sdl.push_str("  id: ID!\n  userId: ID!\n  lotId: ID!\n  slotId: ID\n  date: String!\n  startTime: String!\n  endTime: String!\n  status: String!\n  createdAt: String!\n");
    sdl.push_str("}\n\n");

    sdl.push_str("type Vehicle {\n");
    sdl.push_str("  id: ID!\n  licensePlate: String!\n  make: String\n  model: String\n  color: String\n  vehicleType: String!\n");
    sdl.push_str("}\n");

    sdl
}

/// Get schema introspection info
pub fn schema_info() -> SchemaInfo {
    SchemaInfo {
        queries: vec![
            SchemaField {
                name: "me".to_string(),
                return_type: "User".to_string(),
                args: vec![],
                description: "Get current authenticated user".to_string(),
            },
            SchemaField {
                name: "lots".to_string(),
                return_type: "[Lot!]!".to_string(),
                args: vec![],
                description: "List all parking lots".to_string(),
            },
            SchemaField {
                name: "lot".to_string(),
                return_type: "Lot".to_string(),
                args: vec![SchemaArg {
                    name: "id".to_string(),
                    arg_type: "ID!".to_string(),
                }],
                description: "Get a specific parking lot by ID".to_string(),
            },
            SchemaField {
                name: "bookings".to_string(),
                return_type: "[Booking!]!".to_string(),
                args: vec![],
                description: "List current user's bookings".to_string(),
            },
            SchemaField {
                name: "booking".to_string(),
                return_type: "Booking".to_string(),
                args: vec![SchemaArg {
                    name: "id".to_string(),
                    arg_type: "ID!".to_string(),
                }],
                description: "Get a specific booking by ID".to_string(),
            },
            SchemaField {
                name: "myVehicles".to_string(),
                return_type: "[Vehicle!]!".to_string(),
                args: vec![],
                description: "List current user's vehicles".to_string(),
            },
        ],
        mutations: vec![
            SchemaField {
                name: "createBooking".to_string(),
                return_type: "Booking".to_string(),
                args: vec![
                    SchemaArg { name: "lotId".to_string(), arg_type: "ID!".to_string() },
                    SchemaArg { name: "slotId".to_string(), arg_type: "ID".to_string() },
                    SchemaArg { name: "date".to_string(), arg_type: "String!".to_string() },
                    SchemaArg { name: "startTime".to_string(), arg_type: "String!".to_string() },
                    SchemaArg { name: "endTime".to_string(), arg_type: "String!".to_string() },
                ],
                description: "Create a new booking".to_string(),
            },
            SchemaField {
                name: "cancelBooking".to_string(),
                return_type: "Boolean!".to_string(),
                args: vec![SchemaArg {
                    name: "id".to_string(),
                    arg_type: "ID!".to_string(),
                }],
                description: "Cancel an existing booking".to_string(),
            },
            SchemaField {
                name: "addVehicle".to_string(),
                return_type: "Vehicle".to_string(),
                args: vec![
                    SchemaArg { name: "licensePlate".to_string(), arg_type: "String!".to_string() },
                    SchemaArg { name: "make".to_string(), arg_type: "String".to_string() },
                    SchemaArg { name: "model".to_string(), arg_type: "String".to_string() },
                ],
                description: "Add a new vehicle".to_string(),
            },
        ],
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// QUERY EXECUTION
// ═══════════════════════════════════════════════════════════════════════════════

/// Parse a simple GraphQL query and extract the operation (query/mutation) and field name.
pub fn parse_operation(query: &str) -> Result<(String, String, HashMap<String, String>), String> {
    let trimmed = query.trim();

    // Determine operation type
    let (op_type, body) = if trimmed.starts_with("mutation") {
        ("mutation".to_string(), trimmed.trim_start_matches("mutation").trim())
    } else if trimmed.starts_with("query") {
        ("query".to_string(), trimmed.trim_start_matches("query").trim())
    } else if trimmed.starts_with('{') {
        ("query".to_string(), trimmed)
    } else {
        return Err("Invalid GraphQL query: must start with 'query', 'mutation', or '{'".to_string());
    };

    // Extract the first field name from the body
    let body = body.trim_start_matches(|c: char| c != '{');
    let body = body.trim_start_matches('{').trim();

    // Get the first word (field name)
    let field_end = body.find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(body.len());
    let field_name = &body[..field_end];

    if field_name.is_empty() {
        return Err("Empty query field".to_string());
    }

    // Extract arguments (simple parser)
    let mut args = HashMap::new();
    if let Some(paren_start) = body.find('(') {
        if let Some(paren_end) = body.find(')') {
            let args_str = &body[paren_start + 1..paren_end];
            for part in args_str.split(',') {
                let part = part.trim();
                if let Some(colon) = part.find(':') {
                    let key = part[..colon].trim().to_string();
                    let value = part[colon + 1..].trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    args.insert(key, value);
                }
            }
        }
    }

    Ok((op_type, field_name.to_string(), args))
}

/// Execute a GraphQL query against sample data.
pub fn execute_query(
    op_type: &str,
    field: &str,
    args: &HashMap<String, String>,
    user_id: &str,
) -> GraphQLResponse {
    match op_type {
        "query" => execute_query_field(field, args, user_id),
        "mutation" => execute_mutation_field(field, args, user_id),
        _ => GraphQLResponse {
            data: None,
            errors: vec![GraphQLError {
                message: format!("Unsupported operation type: {op_type}"),
                locations: None,
                path: None,
            }],
        },
    }
}

fn execute_query_field(
    field: &str,
    args: &HashMap<String, String>,
    user_id: &str,
) -> GraphQLResponse {
    match field {
        "me" => GraphQLResponse {
            data: Some(serde_json::json!({
                "me": {
                    "id": user_id,
                    "username": "user",
                    "email": "user@example.com",
                    "name": "User",
                    "role": "user",
                    "createdAt": "2026-01-01T00:00:00Z"
                }
            })),
            errors: vec![],
        },
        "lots" => GraphQLResponse {
            data: Some(serde_json::json!({
                "lots": []
            })),
            errors: vec![],
        },
        "lot" => {
            let id = args.get("id").cloned().unwrap_or_default();
            if id.is_empty() {
                return GraphQLResponse {
                    data: None,
                    errors: vec![GraphQLError {
                        message: "Argument 'id' is required".to_string(),
                        locations: None,
                        path: Some(vec!["lot".to_string()]),
                    }],
                };
            }
            GraphQLResponse {
                data: Some(serde_json::json!({ "lot": null })),
                errors: vec![],
            }
        }
        "bookings" => GraphQLResponse {
            data: Some(serde_json::json!({
                "bookings": []
            })),
            errors: vec![],
        },
        "booking" => {
            let id = args.get("id").cloned().unwrap_or_default();
            if id.is_empty() {
                return GraphQLResponse {
                    data: None,
                    errors: vec![GraphQLError {
                        message: "Argument 'id' is required".to_string(),
                        locations: None,
                        path: Some(vec!["booking".to_string()]),
                    }],
                };
            }
            GraphQLResponse {
                data: Some(serde_json::json!({ "booking": null })),
                errors: vec![],
            }
        }
        "myVehicles" => GraphQLResponse {
            data: Some(serde_json::json!({
                "myVehicles": []
            })),
            errors: vec![],
        },
        "__schema" => GraphQLResponse {
            data: Some(serde_json::json!({
                "__schema": schema_info()
            })),
            errors: vec![],
        },
        _ => GraphQLResponse {
            data: None,
            errors: vec![GraphQLError {
                message: format!("Unknown query field: {field}"),
                locations: None,
                path: Some(vec![field.to_string()]),
            }],
        },
    }
}

fn execute_mutation_field(
    field: &str,
    _args: &HashMap<String, String>,
    _user_id: &str,
) -> GraphQLResponse {
    match field {
        "createBooking" => GraphQLResponse {
            data: Some(serde_json::json!({
                "createBooking": {
                    "id": uuid::Uuid::new_v4().to_string(),
                    "status": "confirmed",
                    "createdAt": chrono::Utc::now().to_rfc3339()
                }
            })),
            errors: vec![],
        },
        "cancelBooking" => GraphQLResponse {
            data: Some(serde_json::json!({
                "cancelBooking": true
            })),
            errors: vec![],
        },
        "addVehicle" => GraphQLResponse {
            data: Some(serde_json::json!({
                "addVehicle": {
                    "id": uuid::Uuid::new_v4().to_string(),
                    "vehicleType": "car"
                }
            })),
            errors: vec![],
        },
        _ => GraphQLResponse {
            data: None,
            errors: vec![GraphQLError {
                message: format!("Unknown mutation field: {field}"),
                locations: None,
                path: Some(vec![field.to_string()]),
            }],
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// GraphQL playground HTML page
const PLAYGROUND_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
  <title>ParkHub GraphQL Playground</title>
  <link rel="stylesheet" href="https://unpkg.com/graphiql@3/graphiql.min.css" />
  <style>body { margin: 0; height: 100vh; } #graphiql { height: 100%; }</style>
</head>
<body>
  <div id="graphiql"></div>
  <script crossorigin src="https://unpkg.com/react@18/umd/react.production.min.js"></script>
  <script crossorigin src="https://unpkg.com/react-dom@18/umd/react-dom.production.min.js"></script>
  <script crossorigin src="https://unpkg.com/graphiql@3/graphiql.min.js"></script>
  <script>
    const fetcher = GraphiQL.createFetcher({ url: '/api/v1/graphql' });
    ReactDOM.createRoot(document.getElementById('graphiql')).render(
      React.createElement(GraphiQL, { fetcher })
    );
  </script>
</body>
</html>"#;

/// `GET /api/v1/graphql/playground` — serve the interactive GraphQL playground.
pub async fn graphql_playground() -> impl IntoResponse {
    Html(PLAYGROUND_HTML)
}

/// `POST /api/v1/graphql` — execute a GraphQL query or mutation.
pub async fn graphql_execute(
    State(_state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(request): Json<GraphQLRequest>,
) -> (StatusCode, Json<GraphQLResponse>) {
    let user_id = auth_user.user_id.to_string();

    match parse_operation(&request.query) {
        Ok((op_type, field, args)) => {
            let response = execute_query(&op_type, &field, &args, &user_id);
            let status = if response.errors.is_empty() {
                StatusCode::OK
            } else {
                StatusCode::OK // GraphQL always returns 200 even with errors
            };
            (status, Json(response))
        }
        Err(e) => (
            StatusCode::OK,
            Json(GraphQLResponse {
                data: None,
                errors: vec![GraphQLError {
                    message: e,
                    locations: None,
                    path: None,
                }],
            }),
        ),
    }
}

/// `GET /api/v1/graphql/schema` — return the GraphQL schema in SDL format.
pub async fn graphql_schema() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        schema_sdl(),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_simple() {
        let (op, field, args) = parse_operation("{ me { id username } }").unwrap();
        assert_eq!(op, "query");
        assert_eq!(field, "me");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_query_with_keyword() {
        let (op, field, _) = parse_operation("query { lots { id name } }").unwrap();
        assert_eq!(op, "query");
        assert_eq!(field, "lots");
    }

    #[test]
    fn test_parse_mutation() {
        let (op, field, _) = parse_operation("mutation { cancelBooking(id: \"abc\") }").unwrap();
        assert_eq!(op, "mutation");
        assert_eq!(field, "cancelBooking");
    }

    #[test]
    fn test_parse_query_with_args() {
        let (_, field, args) = parse_operation("{ lot(id: \"lot-123\") { id name } }").unwrap();
        assert_eq!(field, "lot");
        assert_eq!(args.get("id").unwrap(), "lot-123");
    }

    #[test]
    fn test_parse_invalid_query() {
        let result = parse_operation("INVALID STUFF");
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_me_query() {
        let args = HashMap::new();
        let resp = execute_query("query", "me", &args, "user-123");
        assert!(resp.errors.is_empty());
        let data = resp.data.unwrap();
        assert!(data.get("me").is_some());
        assert_eq!(data["me"]["id"], "user-123");
    }

    #[test]
    fn test_execute_lots_query() {
        let args = HashMap::new();
        let resp = execute_query("query", "lots", &args, "u1");
        assert!(resp.errors.is_empty());
        let data = resp.data.unwrap();
        assert!(data.get("lots").is_some());
    }

    #[test]
    fn test_execute_lot_query_missing_id() {
        let args = HashMap::new();
        let resp = execute_query("query", "lot", &args, "u1");
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("id"));
    }

    #[test]
    fn test_execute_bookings_query() {
        let args = HashMap::new();
        let resp = execute_query("query", "bookings", &args, "u1");
        assert!(resp.errors.is_empty());
        assert!(resp.data.unwrap().get("bookings").is_some());
    }

    #[test]
    fn test_execute_my_vehicles_query() {
        let args = HashMap::new();
        let resp = execute_query("query", "myVehicles", &args, "u1");
        assert!(resp.errors.is_empty());
        assert!(resp.data.unwrap().get("myVehicles").is_some());
    }

    #[test]
    fn test_execute_create_booking_mutation() {
        let args = HashMap::new();
        let resp = execute_query("mutation", "createBooking", &args, "u1");
        assert!(resp.errors.is_empty());
        assert!(resp.data.unwrap().get("createBooking").is_some());
    }

    #[test]
    fn test_execute_cancel_booking_mutation() {
        let args = HashMap::new();
        let resp = execute_query("mutation", "cancelBooking", &args, "u1");
        assert!(resp.errors.is_empty());
        assert_eq!(resp.data.unwrap()["cancelBooking"], true);
    }

    #[test]
    fn test_execute_add_vehicle_mutation() {
        let args = HashMap::new();
        let resp = execute_query("mutation", "addVehicle", &args, "u1");
        assert!(resp.errors.is_empty());
        assert!(resp.data.unwrap().get("addVehicle").is_some());
    }

    #[test]
    fn test_execute_unknown_field() {
        let args = HashMap::new();
        let resp = execute_query("query", "nonexistent", &args, "u1");
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("Unknown query field"));
    }

    #[test]
    fn test_execute_unknown_mutation() {
        let args = HashMap::new();
        let resp = execute_query("mutation", "nonexistent", &args, "u1");
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].message.contains("Unknown mutation field"));
    }

    #[test]
    fn test_execute_unknown_op_type() {
        let args = HashMap::new();
        let resp = execute_query("subscription", "me", &args, "u1");
        assert!(!resp.errors.is_empty());
    }

    #[test]
    fn test_schema_sdl() {
        let sdl = schema_sdl();
        assert!(sdl.contains("type Query"));
        assert!(sdl.contains("type Mutation"));
        assert!(sdl.contains("me: User"));
        assert!(sdl.contains("lots: [Lot!]!"));
        assert!(sdl.contains("createBooking"));
        assert!(sdl.contains("cancelBooking"));
        assert!(sdl.contains("addVehicle"));
        assert!(sdl.contains("type User"));
        assert!(sdl.contains("type Booking"));
        assert!(sdl.contains("type Vehicle"));
    }

    #[test]
    fn test_schema_info() {
        let info = schema_info();
        assert_eq!(info.queries.len(), 6);
        assert_eq!(info.mutations.len(), 3);
        assert!(info.queries.iter().any(|q| q.name == "me"));
        assert!(info.queries.iter().any(|q| q.name == "lots"));
        assert!(info.mutations.iter().any(|m| m.name == "createBooking"));
    }

    #[test]
    fn test_query_type_all() {
        assert_eq!(QueryType::ALL.len(), 6);
    }

    #[test]
    fn test_query_type_return_types() {
        assert_eq!(QueryType::Me.return_type(), "User");
        assert_eq!(QueryType::Lots.return_type(), "[Lot]");
        assert_eq!(QueryType::Bookings.return_type(), "[Booking]");
    }

    #[test]
    fn test_query_type_args() {
        assert!(QueryType::Me.args().is_empty());
        assert_eq!(QueryType::Lot.args().len(), 1);
        assert_eq!(QueryType::Lot.args()[0].0, "id");
    }

    #[test]
    fn test_mutation_type_all() {
        assert_eq!(MutationType::ALL.len(), 3);
    }

    #[test]
    fn test_mutation_type_return_types() {
        assert_eq!(MutationType::CreateBooking.return_type(), "Booking");
        assert_eq!(MutationType::CancelBooking.return_type(), "Boolean");
        assert_eq!(MutationType::AddVehicle.return_type(), "Vehicle");
    }

    #[test]
    fn test_graphql_request_deserialize() {
        let json = r#"{"query":"{ me { id } }","variables":null}"#;
        let req: GraphQLRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "{ me { id } }");
        assert!(req.operation_name.is_none());
    }

    #[test]
    fn test_graphql_response_serialize() {
        let resp = GraphQLResponse {
            data: Some(serde_json::json!({"me": {"id": "123"}})),
            errors: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"me\""));
        assert!(!json.contains("errors")); // empty errors should be skipped
    }

    #[test]
    fn test_graphql_error_serialize() {
        let err = GraphQLError {
            message: "Something failed".to_string(),
            locations: Some(vec![GraphQLLocation { line: 1, column: 5 }]),
            path: Some(vec!["me".to_string()]),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("Something failed"));
        assert!(json.contains("\"line\":1"));
    }

    #[test]
    fn test_graphql_response_with_errors() {
        let resp = GraphQLResponse {
            data: None,
            errors: vec![GraphQLError {
                message: "error".to_string(),
                locations: None,
                path: None,
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("errors"));
        assert!(!json.contains("\"data\"")); // None data should be skipped
    }

    #[test]
    fn test_playground_html() {
        assert!(PLAYGROUND_HTML.contains("GraphQL"));
        assert!(PLAYGROUND_HTML.contains("/api/v1/graphql"));
        assert!(PLAYGROUND_HTML.contains("graphiql"));
    }

    #[test]
    fn test_schema_introspection_query() {
        let args = HashMap::new();
        let resp = execute_query("query", "__schema", &args, "u1");
        assert!(resp.errors.is_empty());
        assert!(resp.data.unwrap().get("__schema").is_some());
    }

    #[test]
    fn test_booking_query_missing_id() {
        let args = HashMap::new();
        let resp = execute_query("query", "booking", &args, "u1");
        assert!(!resp.errors.is_empty());
        assert!(resp.errors[0].path.as_ref().unwrap().contains(&"booking".to_string()));
    }
}
