use crate::{
    db,
    errors::{Error, ErrorResponse},
    handlers::*,
    models::*,
    validate::validate,
};
use actix_web::{
    HttpRequest, HttpResponse, Responder, http::header, web::Data, web::Json, web::Path, web::Query,
};
use tracing::instrument;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}/items",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of order items", body = PaginatedResponse<OrderEntry>),
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
    pagination: Query<PaginationParams>,
) -> Result<impl Responder, Error> {
    let (limit, offset) = pagination.sanitize();
    let (team_id, order_id) = path.into_inner();
    let client: Client = get_client(&state.pool).await?;
    let (items, total) = db::get_order_items(&client, order_id, team_id, limit, offset).await?;
    Ok(HttpResponse::Ok().json(PaginatedResponse {
        items,
        total,
        limit,
        offset,
    }))
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
    let (team_id, order_id, item_id) = path.into_inner();
    let client: Client = get_client(&state.pool).await?;
    let item = db::get_order_item(&client, order_id, item_id, team_id).await?;
    Ok(HttpResponse::Ok().json(item))
}

#[utoipa::path(
    post,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}/items",
    request_body = CreateOrderEntry,
    responses(
        (status = 201, description = "Order item created", body = OrderEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - team membership required (any role, by design)", body = ErrorResponse),
        (status = 404, description = "Team order or item not found", body = ErrorResponse),
        (status = 409, description = "Item already in order", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
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
    let mut client: Client = get_client(&state.pool).await?;
    require_team_member(&client, &req, team_id).await?;
    let order = db::create_order_item(&mut client, order_id, team_id, json.into_inner()).await?;
    let mut response = HttpResponse::Created();
    if let Ok(url) = req.url_for(
        "/teams/team_id/orders/order_id/items/item_id",
        [
            team_id.to_string(),
            order_id.to_string(),
            order.orders_item_id.to_string(),
        ],
    ) {
        response.append_header((header::LOCATION, url.as_str().to_owned()));
    }
    Ok(response.json(order))
}

#[utoipa::path(
    put,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}/items/{item_id}",
    request_body = UpdateOrderEntry,
    responses(
        (status = 200, description = "Order item updated", body = OrderEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - only order owner, team admin, or global admin", body = ErrorResponse),
        (status = 404, description = "Order item not found", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
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
    let mut client: Client = get_client(&state.pool).await?;
    let order = db::get_team_order(&client, team_id, order_id).await?;
    require_order_owner_or_team_admin(&client, &req, team_id, order.teamorders_user_id).await?;
    let order =
        db::update_order_item(&mut client, order_id, item_id, team_id, json.into_inner()).await?;
    Ok(HttpResponse::Ok().json(order))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}/items/{item_id}",
    responses(
        (status = 200, description = "Order item deleted", body = DeletedResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - only order owner, team admin, or global admin", body = ErrorResponse),
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
    let mut client: Client = get_client(&state.pool).await?;
    let order = db::get_team_order(&client, team_id, order_id).await?;
    require_order_owner_or_team_admin(&client, &req, team_id, order.teamorders_user_id).await?;
    let deleted = db::delete_order_item(&mut client, order_id, item_id, team_id).await?;
    if deleted {
        Ok(HttpResponse::Ok().json(DeletedResponse { deleted }))
    } else {
        Ok(HttpResponse::NotFound().json(DeletedResponse { deleted }))
    }
}
