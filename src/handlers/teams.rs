use crate::{
    db,
    errors::{Error, ErrorResponse},
    handlers::*,
    models::*,
    validate::validate,
};
use actix_web::{
    HttpRequest, HttpResponse, Responder, web::Data, web::Json, web::Path, web::Query,
};
use chrono::Utc;
use deadpool_postgres::Client;
use tracing::instrument;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v1.0/teams",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of Teams", body = PaginatedResponse<TeamEntry>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_teams(
    state: Data<State>,
    pagination: Query<PaginationParams>,
) -> Result<impl Responder, Error> {
    let (limit, offset) = pagination.sanitize();
    let client: Client = get_client(&state.pool).await?;
    let (teams, total) = db::get_teams(&client, limit, offset).await?;
    Ok(HttpResponse::Ok().json(PaginatedResponse {
        items: teams,
        total,
        limit,
        offset,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/teams/{team_id}",
    responses(
        (status = 200, description = "Team found", body = TeamEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 404, description = "Team not found", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_team(state: Data<State>, path: Path<Uuid>) -> Result<impl Responder, Error> {
    let client: Client = get_client(&state.pool).await?;
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
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 409, description = "Team already exists", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn create_team(
    state: Data<State>,
    json: Json<CreateTeamEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let client: Client = get_client(&state.pool).await?;
    require_admin(&client, &req).await?;
    let team = db::create_team(&client, json.into_inner()).await?;
    Ok(created_with_location(
        &req,
        &team,
        "/teams/team_id",
        &[team.team_id.to_string()],
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{team_id}",
    responses(
        (status = 200, description = "Team deleted successfully", body = DeletedResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 404, description = "Team not deleted", body = DeletedResponse),
        (status = 409, description = "Conflict - team has existing orders", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn delete_team(
    state: Data<State>,
    tid: Path<Uuid>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let team_id = tid.into_inner();
    let client: Client = get_client(&state.pool).await?;
    require_admin(&client, &req).await?;

    // Guard against silent cascade deletion of order history
    let order_count = db::count_team_orders(&client, team_id).await?;
    if order_count > 0 {
        return Err(Error::Conflict(format!(
            "Cannot delete team — it has {order_count} order(s). Delete the orders first."
        )));
    }

    let deleted = db::delete_team(&client, team_id).await?;
    Ok(delete_response(deleted))
}

#[utoipa::path(
    put,
    path = "/api/v1.0/teams/{team_id}",
    request_body = UpdateTeamEntry,
    responses(
        (status = 200, description = "Team updated successfully", body = TeamEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin role required", body = ErrorResponse),
        (status = 404, description = "Team not updated", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn update_team(
    state: Data<State>,
    path: Path<Uuid>,
    json: Json<UpdateTeamEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let team_id = path.into_inner();
    let client: Client = get_client(&state.pool).await?;
    require_admin(&client, &req).await?;
    let team = db::update_team(&client, team_id, json.into_inner()).await?;
    Ok(HttpResponse::Ok().json(team))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/teams/{team_id}/users",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of Users in the Team", body = PaginatedResponse<UsersInTeam>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn team_users(
    state: Data<State>,
    path: Path<Uuid>,
    pagination: Query<PaginationParams>,
) -> Result<impl Responder, Error> {
    let (limit, offset) = pagination.sanitize();
    let client: Client = get_client(&state.pool).await?;
    let (users, total) = db::get_team_users(&client, path.into_inner(), limit, offset).await?;
    Ok(HttpResponse::Ok().json(PaginatedResponse {
        items: users,
        total,
        limit,
        offset,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/teams/{team_id}/orders",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of orders for the team", body = PaginatedResponse<TeamOrderEntry>),
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
    pagination: Query<PaginationParams>,
) -> Result<impl Responder, Error> {
    let (limit, offset) = pagination.sanitize();
    let client: Client = get_client(&state.pool).await?;
    let (orders, total) = db::get_team_orders(&client, team_id.into_inner(), limit, offset).await?;
    Ok(HttpResponse::Ok().json(PaginatedResponse {
        items: orders,
        total,
        limit,
        offset,
    }))
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
    let client: Client = get_client(&state.pool).await?;
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
        (status = 403, description = "Forbidden - team membership required", body = ErrorResponse),
        (status = 409, description = "Conflict", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn create_team_order(
    state: Data<State>,
    team_id: Path<Uuid>,
    json: Json<CreateTeamOrderEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let tid = team_id.into_inner();
    let client: Client = get_client(&state.pool).await?;
    require_team_member(&client, &req, tid).await?;
    let user_id = requesting_user_id(&req)
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;

    // Validate that the due date (if specified) is not in the past
    if let Some(date) = json.duedate
        && date < Utc::now().date_naive()
    {
        return Err(Error::Validation(
            "Due date cannot be in the past".to_string(),
        ));
    }

    // Validate that the pickup user (if specified) is a member of this team
    if let Some(pickup_id) = json.pickup_user_id {
        let role = db::get_member_role(&client, tid, pickup_id).await?;
        if role.is_none() {
            return Err(Error::Validation(
                "Pickup user must be a member of this team".to_string(),
            ));
        }
    }

    let order = db::create_team_order(&client, tid, user_id, json.into_inner()).await?;
    Ok(created_with_location(
        &req,
        &order,
        "/teams/team_id/order_id",
        &[tid.to_string(), order.teamorders_id.to_string()],
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}",
    responses(
        (status = 200, description = "Order deleted", body = DeletedResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - must be order owner, team admin, or global admin", body = ErrorResponse),
        (status = 404, description = "Order not found", body = DeletedResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Order")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn delete_team_order(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let (team_id, order_id) = path.into_inner();
    let client: Client = get_client(&state.pool).await?;
    // Fetch the order to check ownership
    let order = db::get_team_order(&client, team_id, order_id).await?;
    require_order_owner_or_team_admin(&client, &req, team_id, order.teamorders_user_id).await?;
    let deleted = db::delete_team_order(&client, team_id, order_id).await?;
    Ok(delete_response(deleted))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{team_id}/orders",
    responses(
        (status = 200, description = "All orders for team deleted", body = DeletedResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - team admin role required", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn delete_team_orders(
    state: Data<State>,
    team_id: Path<Uuid>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let tid = team_id.into_inner();
    let client: Client = get_client(&state.pool).await?;
    require_team_admin(&client, &req, tid).await?;
    let count = db::delete_team_orders(&client, tid).await?;
    Ok(HttpResponse::Ok().json(DeletedResponse { deleted: count > 0 }))
}

#[utoipa::path(
    put,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}",
    request_body = UpdateTeamOrderEntry,
    responses(
        (status = 200, description = "Order updated", body = TeamOrderEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - must be order owner, team admin, or global admin", body = ErrorResponse),
        (status = 404, description = "Order not found", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the Order")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn update_team_order(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
    json: Json<UpdateTeamOrderEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let (team_id, order_id) = path.into_inner();
    let client: Client = get_client(&state.pool).await?;
    // Fetch the order to check ownership
    let order = db::get_team_order(&client, team_id, order_id).await?;
    require_order_owner_or_team_admin(&client, &req, team_id, order.teamorders_user_id).await?;

    // Validate that the new due date (if provided) is not in the past
    if let Some(Some(date)) = json.duedate
        && date < Utc::now().date_naive()
    {
        return Err(Error::Validation(
            "Due date cannot be in the past".to_string(),
        ));
    }

    // If the order already has a pickup user and the request wants to change it,
    // only a global Admin or Team Admin for this team may do so.
    if let Some(ref new_pickup) = json.pickup_user_id {
        if order.pickup_user_id.is_some() {
            // The pickup user is being changed — require Admin or Team Admin
            require_team_admin(&client, &req, team_id).await?;
        }
        // Validate that the new pickup user (if not clearing) is a team member
        if let Some(pickup_id) = new_pickup {
            let role = db::get_member_role(&client, team_id, *pickup_id).await?;
            if role.is_none() {
                return Err(Error::Validation(
                    "Pickup user must be a member of this team".to_string(),
                ));
            }
        }
    }

    let order = db::update_team_order(&client, team_id, order_id, json.into_inner()).await?;
    Ok(HttpResponse::Ok().json(order))
}

#[utoipa::path(
    post,
    path = "/api/v1.0/teams/{team_id}/orders/{order_id}/reopen",
    responses(
        (status = 201, description = "Order reopened (duplicated as a new open order)", body = TeamOrderEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - team membership required", body = ErrorResponse),
        (status = 404, description = "Order not found", body = ErrorResponse),
        (status = 422, description = "Validation error - order is not closed", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("order_id", description = "Unique UUID of the closed Order to reopen")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn reopen_team_order(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let (team_id, order_id) = path.into_inner();
    let mut client: Client = get_client(&state.pool).await?;
    require_team_member(&client, &req, team_id).await?;
    let user_id = requesting_user_id(&req)
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;

    let order = db::reopen_team_order(&mut client, team_id, order_id, user_id).await?;
    Ok(HttpResponse::Created().json(order))
}

// ── Team member management ──────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/v1.0/teams/{team_id}/users",
    request_body = AddMemberEntry,
    responses(
        (status = 201, description = "Member added to team", body = UsersInTeam),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - team admin role required, or only global admins can assign the Admin role", body = ErrorResponse),
        (status = 404, description = "User or role not found", body = ErrorResponse),
        (status = 409, description = "Member already in team", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn add_team_member(
    state: Data<State>,
    team_id: Path<Uuid>,
    json: Json<AddMemberEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let tid = team_id.into_inner();
    let member = json.into_inner();
    let mut client: Client = get_client(&state.pool).await?;
    require_team_admin(&client, &req, tid).await?;
    guard_admin_role_assignment(&client, &req, member.role_id).await?;

    let result = db::add_team_member(&mut client, tid, member.user_id, member.role_id).await?;
    Ok(created_with_location(
        &req,
        &result,
        "/teams/team_id/users/user_id",
        &[tid.to_string(), member.user_id.to_string()],
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/teams/{team_id}/users/{user_id}",
    responses(
        (status = 200, description = "Member removed from team", body = DeletedResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - team admin role required", body = ErrorResponse),
        (status = 404, description = "Member not found in team", body = DeletedResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("user_id", description = "Unique UUID of the User")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn remove_team_member(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let (team_id, user_id) = path.into_inner();
    let client: Client = get_client(&state.pool).await?;
    require_team_admin(&client, &req, team_id).await?;
    guard_admin_demotion(&client, &req, user_id).await?;
    guard_last_admin_membership(&client, team_id, user_id).await?;
    let deleted = db::remove_team_member(&client, team_id, user_id).await?;
    Ok(delete_response(deleted))
}

#[utoipa::path(
    put,
    path = "/api/v1.0/teams/{team_id}/users/{user_id}",
    request_body = UpdateMemberRoleEntry,
    responses(
        (status = 200, description = "Member role updated", body = UsersInTeam),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - team admin role required, or only global admins can assign the Admin role", body = ErrorResponse),
        (status = 404, description = "Member not found in team", body = ErrorResponse),
    ),
    params(
        ("team_id", description = "Unique UUID of the Team"),
        ("user_id", description = "Unique UUID of the User")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn update_member_role(
    state: Data<State>,
    path: Path<(Uuid, Uuid)>,
    json: Json<UpdateMemberRoleEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let (team_id, user_id) = path.into_inner();
    let role_id = json.into_inner().role_id;
    let mut client: Client = get_client(&state.pool).await?;
    require_team_admin(&client, &req, team_id).await?;
    guard_admin_demotion(&client, &req, user_id).await?;
    guard_last_admin_membership(&client, team_id, user_id).await?;
    guard_admin_role_assignment(&client, &req, role_id).await?;

    let result = db::update_member_role(&mut client, team_id, user_id, role_id).await?;
    Ok(HttpResponse::Ok().json(result))
}
