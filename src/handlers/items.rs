use crate::{
    db,
    errors::{Error, ErrorResponse},
    handlers::*,
    models::*,
    validate::validate,
};
use actix_web::{
    HttpRequest, HttpResponse, Responder, http::header, web::Data, web::Json, web::Path,
    web::Query,
};
use tracing::instrument;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v1.0/items",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of Items", body = PaginatedResponse<ItemEntry>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_items(
    state: Data<State>,
    pagination: Query<PaginationParams>,
) -> Result<impl Responder, Error> {
    let (limit, offset) = pagination.sanitize();
    let client: Client = get_client(&state.pool).await?;
    let (items, total) = db::get_items(&client, limit, offset).await?;
    Ok(HttpResponse::Ok().json(PaginatedResponse { items, total, limit, offset }))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/items/{item_id}",
    responses(
        (status = 200, description = "Item found", body = ItemEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 404, description = "Item not found", body = ErrorResponse),
    ),
    params(
        ("item_id", description = "Unique UUID of the Item")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_item(state: Data<State>, path: Path<Uuid>) -> Result<impl Responder, Error> {
    let client: Client = get_client(&state.pool).await?;
    let item = db::get_item(&client, path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(item))
}

#[utoipa::path(
    post,
    path = "/api/v1.0/items",
    request_body = CreateItemEntry,
    responses(
        (status = 201, description = "Item created", body = ItemEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 409, description = "Item already exists", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn create_item(
    state: Data<State>,
    json: Json<CreateItemEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let client: Client = get_client(&state.pool).await?;
    require_admin(&client, &req).await?;
    let item = db::create_item(&client, json.into_inner()).await?;
    let mut response = HttpResponse::Created();
    if let Ok(url) = req.url_for("/items/item_id", [item.item_id.to_string()]) {
        response.append_header((header::LOCATION, url.as_str().to_owned()));
    }
    Ok(response.json(item))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/items/{item_id}",
    responses(
        (status = 200, description = "Item deleted successfully", body = DeletedResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 404, description = "Item not deleted", body = DeletedResponse),
    ),
    params(
        ("item_id", description = "Unique UUID of the Item")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn delete_item(
    state: Data<State>,
    path: Path<Uuid>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let client: Client = get_client(&state.pool).await?;
    require_admin(&client, &req).await?;
    let deleted = db::delete_item(&client, path.into_inner()).await?;
    if deleted {
        Ok(HttpResponse::Ok().json(DeletedResponse { deleted }))
    } else {
        Ok(HttpResponse::NotFound().json(DeletedResponse { deleted }))
    }
}

#[utoipa::path(
    put,
    path = "/api/v1.0/items/{item_id}",
    request_body = UpdateItemEntry,
    responses(
        (status = 200, description = "Item updated successfully", body = ItemEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 404, description = "Item not updated", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    params(
        ("item_id", description = "Unique UUID of the Item")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn update_item(
    state: Data<State>,
    path: Path<Uuid>,
    json: Json<UpdateItemEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let client: Client = get_client(&state.pool).await?;
    require_admin(&client, &req).await?;
    let item = db::update_item(&client, path.into_inner(), json.into_inner()).await?;
    Ok(HttpResponse::Ok().json(item))
}
