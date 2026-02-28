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
        (status = 200, description = "List of orders for the team", body = [TeamOrderEntry]),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_team_orders(
    state: Data<State>,
    team_id: Path<Uuid>,
) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    let orders = db::get_team_orders(&client, team_id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(orders))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}",
    responses(
        (status = 200, description = "Order found", body = TeamOrderEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Order not found", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Order")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_team_order(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
) -> Result<impl Responder, Error> {
    let (team_id, order_id) = path.into_inner();
    let client: Client = get_client(state.pool.clone()).await?;
    let order = db::get_team_order(&client, team_id, order_id).await?;
    Ok(HttpResponse::Ok().json(order))
}

#[utoipa::path(
    post,
    path = "/api/v1.0/teams/{team_id}/orders",
    request_body = CreateTeamOrderEntry,
    responses(
        (status = 201, description = "Order created", body = TeamOrderEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn create_team_order(
    state: Data<State>,
    team_id: Path<Uuid>,
    json: Json<CreateTeamOrderEntry>,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let client: Client = get_client(state.pool.clone()).await?;
    let order =
        db::create_team_order(&client, team_id.into_inner(), json.into_inner()).await?;
    Ok(HttpResponse::Created().json(order))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}",
    responses(
        (status = 200, description = "Order deleted", body = DeletedResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Order not found", body = DeletedResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Order")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn delete_team_order(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
) -> Result<impl Responder, Error> {
    let (team_id, order_id) = path.into_inner();
    let client: Client = get_client(state.pool.clone()).await?;
    let deleted = db::delete_team_order(&client, team_id, order_id).await?;
    if deleted {
        Ok(HttpResponse::Ok().json(DeletedResponse { deleted }))
    } else {
        Ok(HttpResponse::NotFound().json(DeletedResponse { deleted }))
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{team_id}/orders",
    responses(
        (status = 200, description = "All orders for team deleted", body = DeletedResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn delete_team_orders(
    state: Data<State>,
    team_id: Path<Uuid>,
) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    let count = db::delete_team_orders(&client, team_id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(DeletedResponse {
        deleted: count > 0,
    }))
}

#[utoipa::path(
    put,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}",
    request_body = UpdateTeamOrderEntry,
    responses(
        (status = 200, description = "Order updated", body = TeamOrderEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Order not found", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Order")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn update_team_order(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
    json: Json<UpdateTeamOrderEntry>,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let (team_id, order_id) = path.into_inner();
    let client: Client = get_client(state.pool.clone()).await?;
    let order =
        db::update_team_order(&client, team_id, order_id, json.into_inner()).await?;
    Ok(HttpResponse::Ok().json(order))
}

// ── Team member management ──────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/v1.0/teams/{team_id}/users",
    request_body = AddMemberEntry,
    responses(
        (status = 201, description = "Member added to team", body = UsersInTeam),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 409, description = "Member already in team", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn add_team_member(
    state: Data<State>,
    team_id: Path<Uuid>,
    json: Json<AddMemberEntry>,
) -> Result<impl Responder, Error> {
    let member = json.into_inner();
    let client: Client = get_client(state.pool.clone()).await?;
    let result =
        db::add_team_member(&client, team_id.into_inner(), member.user_id, member.role_id)
            .await?;
    Ok(HttpResponse::Created().json(result))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{team_id}/users/{user_id}",
    responses(
        (status = 200, description = "Member removed from team", body = DeletedResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Member not found in team", body = DeletedResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("user_id", description = "Unique UUID of the User")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn remove_team_member(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
) -> Result<impl Responder, Error> {
    let (team_id, user_id) = path.into_inner();
    let client: Client = get_client(state.pool.clone()).await?;
    let deleted = db::remove_team_member(&client, team_id, user_id).await?;
    if deleted {
        Ok(HttpResponse::Ok().json(DeletedResponse { deleted }))
    } else {
        Ok(HttpResponse::NotFound().json(DeletedResponse { deleted }))
    }
}

#[utoipa::path(
    put,
    path = "/api/v1.0/teams/{team_id}/users/{user_id}",
    request_body = UpdateMemberRoleEntry,
    responses(
        (status = 200, description = "Member role updated", body = UsersInTeam),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Member not found in team", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("user_id", description = "Unique UUID of the User")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn update_member_role(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
    json: Json<UpdateMemberRoleEntry>,
) -> Result<impl Responder, Error> {
    let (team_id, user_id) = path.into_inner();
    let client: Client = get_client(state.pool.clone()).await?;
    let result =
        db::update_member_role(&client, team_id, user_id, json.into_inner().role_id).await?;
    Ok(HttpResponse::Ok().json(result))
}
