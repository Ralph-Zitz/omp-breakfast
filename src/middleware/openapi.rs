use std::sync::Arc;

use crate::errors::{Error, ErrorResponse};
use crate::handlers::*;
use crate::models::*;
use actix_web::web::Bytes;
use actix_web::Responder;
use actix_web::{get, web::Data, web::Path, HttpResponse};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

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
        UpdateUserEntry,
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
        let spec = ApiDoc::openapi().to_json().unwrap();
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(spec));
    }
    let conf = Arc::new(openapi_conf.as_ref().clone());
    match utoipa_swagger_ui::serve(&tail, conf).unwrap() {
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
