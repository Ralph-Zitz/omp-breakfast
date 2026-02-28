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
use tracing::instrument;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v1.0/teams",
    responses(
        (status = 200, description = "List of Teams", body = [TeamEntry]),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_teams(state: Data<State>) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    let teams = db::get_teams(&client).await?;
    Ok(HttpResponse::Ok().json(teams))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/teams/{id}",
    responses(
        (status = 200, description = "Team found", body = TeamEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 404, description = "Team not found", body = ErrorResponse),
    ),
    params(
        ("id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_team(state: Data<State>, path: Path<Uuid>) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    let team = db::get_team(&client, path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(team))
}

#[utoipa::path(
    post,
    path = "/api/v1.0/teams",
    request_body = CreateTeamEntry,
    responses(
        (status = 201, description = "Team created", body = TeamEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 409, description = "Team already exists", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn create_team(
    state: Data<State>,
    json: Json<CreateTeamEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let client: Client = get_client(state.pool.clone()).await?;
    let team = db::create_team(&client, json.into_inner()).await?;
    Ok(HttpResponse::Created()
        .append_header((
            header::LOCATION,
            req.url_for("/teams/team_id", [team.team_id.to_string()])
                .unwrap()
                .as_str(),
        ))
        .json(team))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{id}",
    responses(
        (status = 200, description = "Team deleted successfully", body = DeletedResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 404, description = "Team not deleted", body = DeletedResponse),
    ),
    params(
        ("id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn delete_team(state: Data<State>, tid: Path<Uuid>) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    let deleted = db::delete_team(&client, tid.into_inner()).await?;
    if deleted {
        Ok(HttpResponse::Ok().json(DeletedResponse { deleted }))
    } else {
        Ok(HttpResponse::NotFound().json(DeletedResponse { deleted }))
    }
}

#[utoipa::path(
    put,
    path = "/api/v1.0/teams/{id}",
    request_body = UpdateTeamEntry,
    responses(
        (status = 200, description = "Team updated successfully", body = TeamEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 404, description = "Team not updated", body = ErrorResponse),
    ),
    params(
        ("id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn update_team(
    state: Data<State>,
    path: Path<Uuid>,
    json: Json<UpdateTeamEntry>,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let client: Client = get_client(state.pool.clone()).await?;
    let team = db::update_team(&client, path.into_inner(), json.into_inner()).await?;
    Ok(HttpResponse::Ok().json(team))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/teams/{id}/users",
    responses(
        (status = 200, description = "List of Users in the Team", body = [UsersInTeam]),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 404, description = "No users found", body = ErrorResponse),
    ),
    params(
        ("id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn team_users(state: Data<State>, path: Path<Uuid>) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    let users = db::get_team_users(&client, path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(users))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/teams/{team_id}/orders",
    responses(
        (status = 200, description = "List of orders for the team", body = [ErrorResponse]),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 501, description = "Not implemented", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(_state), level = "debug")]
pub async fn get_team_orders(
    _state: Data<State>,
    _team_id: Path<Uuid>,
) -> Result<impl Responder, Error> {
    Ok(HttpResponse::NotImplemented().json(ErrorResponse {
        error: "Not Implemented".to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}",
    responses(
        (status = 200, description = "Order found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 501, description = "Not implemented", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Order")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(_state), level = "debug")]
pub async fn get_team_order(
    _state: Data<State>,
    _path: Path<(Uuid, Uuid)>,
) -> Result<impl Responder, Error> {
    Ok(HttpResponse::NotImplemented().json(ErrorResponse {
        error: "Not Implemented".to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1.0/teams/{team_id}/orders",
    responses(
        (status = 201, description = "Order created", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 501, description = "Not implemented", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(_state), level = "debug")]
pub async fn create_team_order(
    _state: Data<State>,
    _team_id: Path<Uuid>,
) -> Result<impl Responder, Error> {
    Ok(HttpResponse::NotImplemented().json(ErrorResponse {
        error: "Not Implemented".to_string(),
    }))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}",
    responses(
        (status = 200, description = "Order deleted", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 501, description = "Not implemented", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Order")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(_state), level = "debug")]
pub async fn delete_team_order(
    _state: Data<State>,
    _path: Path<(Uuid, Uuid)>,
) -> Result<impl Responder, Error> {
    Ok(HttpResponse::NotImplemented().json(ErrorResponse {
        error: "Not Implemented".to_string(),
    }))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{team_id}/orders",
    responses(
        (status = 200, description = "All orders for team deleted", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 501, description = "Not implemented", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(_state), level = "debug")]
pub async fn delete_team_orders(
    _state: Data<State>,
    _team_id: Path<Uuid>,
) -> Result<impl Responder, Error> {
    Ok(HttpResponse::NotImplemented().json(ErrorResponse {
        error: "Not Implemented".to_string(),
    }))
}

#[utoipa::path(
    put,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}",
    responses(
        (status = 200, description = "Order updated", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 501, description = "Not implemented", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Order")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(_state), level = "debug")]
pub async fn update_team_order(
    _state: Data<State>,
    _path: Path<(Uuid, Uuid)>,
) -> Result<impl Responder, Error> {
    Ok(HttpResponse::NotImplemented().json(ErrorResponse {
        error: "Not Implemented".to_string(),
    }))
}
