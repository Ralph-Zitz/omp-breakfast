use crate::{
    db,
    errors::{Error, ErrorResponse},
    handlers::*,
    models::*,
    validate::validate,
};
use actix_web::{web::Data, web::Json, web::Path, HttpRequest, HttpResponse, Responder};
use tracing::instrument;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}/items",
    responses(
        (status = 200, description = "List of order items", body = [OrderEntry]),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Team Order")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_order_items(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
) -> Result<impl Responder, Error> {
    let (_team_id, order_id) = path.into_inner();
    let client: Client = get_client(state.pool.clone()).await?;
    let items = db::get_order_items(&client, order_id).await?;
    Ok(HttpResponse::Ok().json(items))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}/items/{item_id}",
    responses(
        (status = 200, description = "Order item found", body = OrderEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Order item not found", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Team Order"),
        ("item_id", description = "Unique UUID of the Item")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_order_item(
    state: Data<State>,
    path: Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, Error> {
    let (_team_id, order_id, item_id) = path.into_inner();
    let client: Client = get_client(state.pool.clone()).await?;
    let item = db::get_order_item(&client, order_id, item_id).await?;
    Ok(HttpResponse::Ok().json(item))
}

#[utoipa::path(
    post,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}/items",
    request_body = CreateOrderEntry,
    responses(
        (status = 201, description = "Order item created", body = OrderEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 409, description = "Item already in order", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Team Order")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn create_order_item(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
    json: Json<CreateOrderEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let (team_id, order_id) = path.into_inner();
    let client: Client = get_client(state.pool.clone()).await?;
    require_team_member(&client, &req, team_id).await?;
    let order = db::create_order_item(&client, order_id, team_id, json.into_inner()).await?;
    Ok(HttpResponse::Created().json(order))
}

#[utoipa::path(
    put,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}/items/{item_id}",
    request_body = UpdateOrderEntry,
    responses(
        (status = 200, description = "Order item updated", body = OrderEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Order item not found", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Team Order"),
        ("item_id", description = "Unique UUID of the Item")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn update_order_item(
    state: Data<State>,
    path: Path<(Uuid, Uuid, Uuid)>,
    json: Json<UpdateOrderEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let (team_id, order_id, item_id) = path.into_inner();
    let client: Client = get_client(state.pool.clone()).await?;
    require_team_member(&client, &req, team_id).await?;
    let order = db::update_order_item(&client, order_id, item_id, json.into_inner()).await?;
    Ok(HttpResponse::Ok().json(order))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}/items/{item_id}",
    responses(
        (status = 200, description = "Order item deleted", body = DeletedResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Order item not found", body = DeletedResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Team Order"),
        ("item_id", description = "Unique UUID of the Item")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn delete_order_item(
    state: Data<State>,
    path: Path<(Uuid, Uuid, Uuid)>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let (team_id, order_id, item_id) = path.into_inner();
    let client: Client = get_client(state.pool.clone()).await?;
    require_team_member(&client, &req, team_id).await?;
    let deleted = db::delete_order_item(&client, order_id, item_id).await?;
    if deleted {
        Ok(HttpResponse::Ok().json(DeletedResponse { deleted }))
    } else {
        Ok(HttpResponse::NotFound().json(DeletedResponse { deleted }))
    }
}
