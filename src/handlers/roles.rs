use crate::{
    db,
    errors::{Error, ErrorResponse},
    handlers::*,
    models::*,
    validate::validate,
};
use actix_web::{
    http::header, web::Data, web::Json, web::Path, HttpRequest, HttpResponse, Responder,
};
use deadpool_postgres::Client;
use tracing::instrument;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v1.0/roles",
    responses(
        (status = 200, description = "List of Roles", body = [RoleEntry]),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_roles(state: Data<State>) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    let roles = db::get_roles(&client).await?;
    Ok(HttpResponse::Ok().json(roles))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/roles/{id}",
    responses(
        (status = 200, description = "Role found", body = RoleEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 404, description = "Role not found", body = ErrorResponse),
    ),
    params(
        ("id", description = "Unique UUID of the Role")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_role(state: Data<State>, path: Path<Uuid>) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    let role = db::get_role(&client, path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(role))
}

#[utoipa::path(
    post,
    path = "/api/v1.0/roles",
    request_body = CreateRoleEntry,
    responses(
        (status = 201, description = "Role created", body = RoleEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 409, description = "Role already exists", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn create_role(
    state: Data<State>,
    json: Json<CreateRoleEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let client: Client = get_client(state.pool.clone()).await?;
    require_admin(&client, &req).await?;
    let role = db::create_role(&client, json.into_inner()).await?;
    let mut response = HttpResponse::Created();
    if let Ok(url) = req.url_for("/roles/role_id", [role.role_id.to_string()]) {
        response.append_header((header::LOCATION, url.as_str().to_owned()));
    }
    Ok(response.json(role))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/roles/{id}",
    responses(
        (status = 200, description = "Role deleted successfully", body = DeletedResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 404, description = "Role not deleted", body = DeletedResponse),
    ),
    params(
        ("id", description = "Unique UUID of the Role")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn delete_role(state: Data<State>, rid: Path<Uuid>, req: HttpRequest) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    require_admin(&client, &req).await?;
    let deleted = db::delete_role(&client, rid.into_inner()).await?;
    if deleted {
        Ok(HttpResponse::Ok().json(DeletedResponse { deleted }))
    } else {
        Ok(HttpResponse::NotFound().json(DeletedResponse { deleted }))
    }
}

#[utoipa::path(
    put,
    path = "/api/v1.0/roles/{id}",
    request_body = UpdateRoleEntry,
    responses(
        (status = 200, description = "Role updated successfully", body = RoleEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 404, description = "Role not updated", body = ErrorResponse),
    ),
    params(
        ("id", description = "Unique UUID of the Role")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn update_role(
    state: Data<State>,
    path: Path<Uuid>,
    json: Json<UpdateRoleEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let client: Client = get_client(state.pool.clone()).await?;
    require_admin(&client, &req).await?;
    let role = db::update_role(&client, path.into_inner(), json.into_inner()).await?;
    Ok(HttpResponse::Ok().json(role))
}
