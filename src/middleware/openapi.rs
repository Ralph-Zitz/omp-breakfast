use std::sync::Arc;

use crate::errors::{Error, ErrorResponse};
use crate::handlers::*;
use crate::models::*;
use actix_web::Responder;
use actix_web::web::Bytes;
use actix_web::{HttpResponse, get, web::Data, web::Path};
use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_default();
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some("Enter the JWT access token"))
                    .build(),
            ),
        );
        components.add_security_scheme(
            "basic_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Basic)
                    .description(Some("Enter email and password"))
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    paths(
        // Health
        get_health,
        // Users
        users::get_users,
        users::get_user,
        users::auth_user,
        users::refresh_token,
        users::revoke_user_token,
        users::create_user,
        users::delete_user,
        users::delete_user_by_email,
        users::update_user,
        users::user_teams,
        // Teams
        teams::get_teams,
        teams::get_team,
        teams::create_team,
        teams::delete_team,
        teams::update_team,
        teams::team_users,
        // Team Orders
        teams::get_team_orders,
        teams::get_team_order,
        teams::create_team_order,
        teams::delete_team_order,
        teams::delete_team_orders,
        teams::update_team_order,
        // Team Members
        teams::add_team_member,
        teams::remove_team_member,
        teams::update_member_role,
        // Items
        items::get_items,
        items::get_item,
        items::create_item,
        items::delete_item,
        items::update_item,
        // Order Items
        orders::get_order_items,
        orders::get_order_item,
        orders::create_order_item,
        orders::update_order_item,
        orders::delete_order_item,
        // Roles
        roles::get_roles,
        roles::get_role,
        roles::create_role,
        roles::delete_role,
        roles::update_role,
    ),
    components(schemas(
        StatusResponse,
        Auth,
        TokenRequest,
        UserEntry,
        UserInTeams,
        UpdateUserRequest,
        DeletedResponse,
        CreateUserEntry,
        ErrorResponse,
        TeamEntry,
        CreateTeamEntry,
        UpdateTeamEntry,
        UsersInTeam,
        RoleEntry,
        CreateRoleEntry,
        UpdateRoleEntry,
        ItemEntry,
        CreateItemEntry,
        UpdateItemEntry,
        TeamOrderEntry,
        CreateTeamOrderEntry,
        UpdateTeamOrderEntry,
        AddMemberEntry,
        UpdateMemberRoleEntry,
        OrderEntry,
        CreateOrderEntry,
        UpdateOrderEntry,
    ))
)]
pub(crate) struct ApiDoc;

#[get("/{tail}*")]
async fn get_swagger(
    tail: Path<String>,
    openapi_conf: Data<utoipa_swagger_ui::Config<'static>>,
) -> Result<impl Responder, Error> {
    if tail.as_ref() == "swagger.json" {
        let spec = ApiDoc::openapi()
            .to_json()
            .map_err(|e| Error::Utoipa(format!("Failed to serialize OpenAPI spec: {}", e)))?;
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(spec));
    }
    let conf = Arc::new(openapi_conf.as_ref().clone());
    match utoipa_swagger_ui::serve(&tail, conf)
        .map_err(|e| Error::Utoipa(format!("Swagger UI error: {}", e)))?
    {
        None => Ok(HttpResponse::from_error(Error::Utoipa(format!(
            "path not found: {}",
            tail
        )))),
        Some(file) => Ok({
            let bytes = Bytes::from(file.bytes.to_vec());
            HttpResponse::Ok()
                .content_type(file.content_type)
                .body(bytes)
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// Helper: generate the OpenAPI JSON and parse it.
    fn openapi_json() -> Value {
        let spec = ApiDoc::openapi()
            .to_json()
            .expect("OpenAPI spec should serialize to JSON");
        serde_json::from_str(&spec).expect("OpenAPI JSON should parse")
    }

    #[test]
    fn spec_has_openapi_version() {
        let doc = openapi_json();
        let version = doc["openapi"].as_str().unwrap();
        assert!(
            version.starts_with("3."),
            "should be OpenAPI 3.x, got: {}",
            version
        );
    }

    // ── Paths ───────────────────────────────────────────────────────────

    #[test]
    fn spec_contains_health_endpoint() {
        let doc = openapi_json();
        assert!(
            doc["paths"]["/health"].is_object(),
            "should contain /health path"
        );
    }

    #[test]
    fn spec_contains_auth_endpoints() {
        let doc = openapi_json();
        let paths = &doc["paths"];
        assert!(paths["/auth"].is_object(), "should contain /auth");
        assert!(
            paths["/auth/refresh"].is_object(),
            "should contain /auth/refresh"
        );
        assert!(
            paths["/auth/revoke"].is_object(),
            "should contain /auth/revoke"
        );
    }

    #[test]
    fn spec_contains_user_endpoints() {
        let doc = openapi_json();
        let paths = &doc["paths"];
        assert!(
            paths["/api/v1.0/users"].is_object(),
            "should contain /api/v1.0/users"
        );
        assert!(
            paths["/api/v1.0/users/{user_id}"].is_object(),
            "should contain /api/v1.0/users/{{user_id}}"
        );
        assert!(
            paths["/api/v1.0/users/{user_id}/teams"].is_object(),
            "should contain /api/v1.0/users/{{user_id}}/teams"
        );
        assert!(
            paths["/api/v1.0/users/email/{email}"].is_object(),
            "should contain /api/v1.0/users/email/{{email}}"
        );
    }

    #[test]
    fn spec_contains_team_endpoints() {
        let doc = openapi_json();
        let paths = &doc["paths"];
        assert!(
            paths["/api/v1.0/teams"].is_object(),
            "should contain /api/v1.0/teams"
        );
        assert!(
            paths["/api/v1.0/teams/{team_id}"].is_object(),
            "should contain /api/v1.0/teams/{{team_id}}"
        );
        // team_users (GET) and add_team_member (POST) now both use {team_id}
        assert!(
            paths["/api/v1.0/teams/{team_id}/users"].is_object(),
            "should contain /api/v1.0/teams/{{team_id}}/users"
        );
        assert!(
            paths["/api/v1.0/teams/{team_id}/users/{user_id}"].is_object(),
            "should contain /api/v1.0/teams/{{team_id}}/users/{{user_id}}"
        );
    }

    #[test]
    fn spec_contains_team_order_endpoints() {
        let doc = openapi_json();
        let paths = &doc["paths"];
        assert!(
            paths["/api/v1.0/teams/{team_id}/orders"].is_object(),
            "should contain team orders path"
        );
        assert!(
            paths["/api/v1.0/teams/{team_id}/orders/{order_id}"].is_object(),
            "should contain team order item path"
        );
    }

    #[test]
    fn spec_contains_order_item_endpoints() {
        let doc = openapi_json();
        let paths = &doc["paths"];
        assert!(
            paths["/api/v1.0/teams/{team_id}/orders/{order_id}/items"].is_object(),
            "should contain order items path"
        );
        assert!(
            paths["/api/v1.0/teams/{team_id}/orders/{order_id}/items/{item_id}"].is_object(),
            "should contain order item path"
        );
    }

    #[test]
    fn spec_contains_item_endpoints() {
        let doc = openapi_json();
        let paths = &doc["paths"];
        assert!(
            paths["/api/v1.0/items"].is_object(),
            "should contain /api/v1.0/items"
        );
        assert!(
            paths["/api/v1.0/items/{item_id}"].is_object(),
            "should contain /api/v1.0/items/{{item_id}}"
        );
    }

    #[test]
    fn spec_contains_role_endpoints() {
        let doc = openapi_json();
        let paths = &doc["paths"];
        assert!(
            paths["/api/v1.0/roles"].is_object(),
            "should contain /api/v1.0/roles"
        );
        assert!(
            paths["/api/v1.0/roles/{role_id}"].is_object(),
            "should contain /api/v1.0/roles/{{role_id}}"
        );
    }

    #[test]
    fn spec_has_expected_handler_operation_count() {
        let doc = openapi_json();
        let paths = doc["paths"].as_object().expect("paths should be an object");
        // Count individual operations (get, post, put, delete) across all paths
        let op_count: usize = paths
            .values()
            .map(|path_item| {
                ["get", "post", "put", "delete"]
                    .iter()
                    .filter(|method| path_item[*method].is_object())
                    .count()
            })
            .sum();
        // 41 operations: health(1) + auth(3) + users(8) + teams(5) +
        // team_orders(6) + team_members(4) + items(5) + order_items(5) + roles(5)
        // Note: some utoipa paths diverge from actix route params (e.g. {id} vs
        // {team_id}), which can produce separate path entries for the same
        // logical resource when the param name differs across handlers.
        assert_eq!(
            op_count, 41,
            "should have exactly 41 handler operations, got {}",
            op_count
        );
    }

    // ── Schemas ─────────────────────────────────────────────────────────

    #[test]
    fn spec_contains_all_registered_schemas() {
        let doc = openapi_json();
        let schemas = &doc["components"]["schemas"];
        let expected = [
            "StatusResponse",
            "RevokedResponse",
            "Auth",
            "TokenRequest",
            "UserEntry",
            "UserInTeams",
            "UpdateUserRequest",
            "DeletedResponse",
            "CreateUserEntry",
            "ErrorResponse",
            "TeamEntry",
            "CreateTeamEntry",
            "UpdateTeamEntry",
            "UsersInTeam",
            "RoleEntry",
            "CreateRoleEntry",
            "UpdateRoleEntry",
            "ItemEntry",
            "CreateItemEntry",
            "UpdateItemEntry",
            "TeamOrderEntry",
            "CreateTeamOrderEntry",
            "UpdateTeamOrderEntry",
            "AddMemberEntry",
            "UpdateMemberRoleEntry",
            "OrderEntry",
            "CreateOrderEntry",
            "UpdateOrderEntry",
        ];
        for name in &expected {
            assert!(
                schemas[name].is_object(),
                "schema '{}' should be present in components/schemas",
                name
            );
        }
        let schema_map = schemas.as_object().expect("schemas should be an object");
        assert_eq!(
            schema_map.len(),
            expected.len(),
            "should have exactly {} schemas, got {}",
            expected.len(),
            schema_map.len()
        );
    }

    // ── Security Schemes ────────────────────────────────────────────────

    #[test]
    fn spec_has_bearer_auth_security_scheme() {
        let doc = openapi_json();
        let scheme = &doc["components"]["securitySchemes"]["bearer_auth"];
        assert!(scheme.is_object(), "bearer_auth scheme should exist");
        assert_eq!(scheme["type"].as_str(), Some("http"));
        assert_eq!(scheme["scheme"].as_str(), Some("bearer"));
        assert_eq!(scheme["bearerFormat"].as_str(), Some("JWT"));
    }

    #[test]
    fn spec_has_basic_auth_security_scheme() {
        let doc = openapi_json();
        let scheme = &doc["components"]["securitySchemes"]["basic_auth"];
        assert!(scheme.is_object(), "basic_auth scheme should exist");
        assert_eq!(scheme["type"].as_str(), Some("http"));
        assert_eq!(scheme["scheme"].as_str(), Some("basic"));
    }

    // ── Spec is valid JSON ──────────────────────────────────────────────

    #[test]
    fn spec_round_trips_through_json() {
        // Verify the spec can be serialized and deserialized without loss
        let spec1 = ApiDoc::openapi()
            .to_json()
            .expect("first serialization should succeed");
        let parsed: Value = serde_json::from_str(&spec1).expect("should parse");
        let spec2 = serde_json::to_string(&parsed).expect("should re-serialize");
        let reparsed: Value = serde_json::from_str(&spec2).expect("should re-parse");
        assert_eq!(parsed, reparsed, "spec should survive JSON round-trip");
    }
}
