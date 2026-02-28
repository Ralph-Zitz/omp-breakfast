pub mod items;
pub mod orders;
pub mod roles;
pub mod teams;
pub mod users;

use crate::{db, errors::Error, models::*};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web::Data};
use deadpool_postgres::{Client, Pool};
use tracing::{error, instrument};
use uuid::Uuid;

/* Utility Functions */
pub async fn get_client(pool: Pool) -> Result<Client, Error> {
    pool.get().await.map_err(|err| {
        error!(error = %err, "Failed to acquire DB client from pool");
        err.into()
    })
}

/// Extract the requesting user's ID from JWT claims in request extensions.
pub fn requesting_user_id(req: &HttpRequest) -> Option<Uuid> {
    req.extensions().get::<Claims>().map(|c| c.sub)
}

/// Require the requesting user to be a member of the specified team (any role).
/// Global admins bypass this check.
pub async fn require_team_member(
    client: &Client,
    req: &HttpRequest,
    team_id: Uuid,
) -> Result<(), Error> {
    let user_id = requesting_user_id(req)
        .ok_or_else(|| Error::Forbidden("Authentication required".to_string()))?;
    // Global admin bypass
    if db::is_admin(client, user_id).await? {
        return Ok(());
    }
    match db::get_member_role(client, team_id, user_id).await? {
        Some(_) => Ok(()),
        None => Err(Error::Forbidden("Team membership required".to_string())),
    }
}

/// Require the requesting user to hold the "Admin" role in any team (global admin check).
pub async fn require_admin(client: &Client, req: &HttpRequest) -> Result<(), Error> {
    let user_id = requesting_user_id(req)
        .ok_or_else(|| Error::Forbidden("Authentication required".to_string()))?;
    if db::is_admin(client, user_id).await? {
        Ok(())
    } else {
        Err(Error::Forbidden("Admin role required".to_string()))
    }
}

/// Require the requesting user to be a Team Admin of the specified team,
/// or a global Admin. Global admins bypass the team-scoped check.
pub async fn require_team_admin(
    client: &Client,
    req: &HttpRequest,
    team_id: Uuid,
) -> Result<(), Error> {
    let user_id = requesting_user_id(req)
        .ok_or_else(|| Error::Forbidden("Authentication required".to_string()))?;
    // Global admin bypass
    if db::is_admin(client, user_id).await? {
        return Ok(());
    }
    match db::get_member_role(client, team_id, user_id).await? {
        Some(role) if role == "Team Admin" => Ok(()),
        Some(_) => Err(Error::Forbidden("Team admin role required".to_string())),
        None => Err(Error::Forbidden("Team membership required".to_string())),
    }
}

/// Require the requesting user to be the target user themselves, or a global Admin.
pub async fn require_self_or_admin(
    client: &Client,
    req: &HttpRequest,
    target_user_id: Uuid,
) -> Result<(), Error> {
    let user_id = requesting_user_id(req)
        .ok_or_else(|| Error::Forbidden("Authentication required".to_string()))?;
    if user_id == target_user_id {
        return Ok(());
    }
    if db::is_admin(client, user_id).await? {
        return Ok(());
    }
    Err(Error::Forbidden(
        "You can only modify your own account".to_string(),
    ))
}

// API Health endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health check", body = StatusResponse)
    ),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_health(state: Data<State>) -> Result<impl Responder, Error> {
    //    let client: Client = get_client(state.pool.clone()).await?;
    let Ok(client) = get_client(state.pool.clone()).await else {
        return Ok(HttpResponse::Ok().json(StatusResponse { up: false }));
    };
    let result = db::check_db(&client).await;

    result.map(|ok| HttpResponse::Ok().json(StatusResponse { up: ok }))
}
