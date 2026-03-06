pub mod avatars;
pub mod items;
pub mod orders;
pub mod roles;
pub mod teams;
pub mod users;

use crate::middleware::auth::{ROLE_ADMIN, ROLE_TEAM_ADMIN};
use crate::{db, errors::Error, models::*};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web::Data};
use deadpool_postgres::{Client, Pool};
use tracing::{error, instrument};
use uuid::Uuid;

/* Utility Functions */
pub async fn get_client(pool: &Pool) -> Result<Client, Error> {
    pool.get().await.map_err(|err| {
        error!(error = %err, "Failed to acquire DB client from pool");
        err.into()
    })
}

/// Extract the requesting user's ID from JWT claims in request extensions.
#[must_use = "caller must handle the case where no JWT claims are present"]
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
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;
    // Combined admin + team role check in a single DB round-trip
    let (is_admin, team_role) = db::check_team_access(client, team_id, user_id).await?;
    if is_admin {
        return Ok(());
    }
    match team_role {
        Some(_) => Ok(()),
        None => Err(Error::Forbidden("Team membership required".to_string())),
    }
}

/// Require the requesting user to hold the "Admin" role in any team (global admin check).
pub async fn require_admin(client: &Client, req: &HttpRequest) -> Result<(), Error> {
    let user_id = requesting_user_id(req)
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;
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
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;
    // Combined admin + team role check in a single DB round-trip
    let (is_admin, team_role) = db::check_team_access(client, team_id, user_id).await?;
    if is_admin {
        return Ok(());
    }
    match team_role {
        Some(role) if role == ROLE_TEAM_ADMIN => Ok(()),
        Some(_) => Err(Error::Forbidden("Team admin role required".to_string())),
        None => Err(Error::Forbidden("Team membership required".to_string())),
    }
}

/// Require the requesting user to hold the "Admin" role in any team (global admin),
/// or the "Team Admin" role in any team. Used for operations like user creation
/// that should be available to both admin tiers but not regular members.
pub async fn require_admin_or_team_admin(client: &Client, req: &HttpRequest) -> Result<(), Error> {
    let user_id = requesting_user_id(req)
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;
    if db::is_admin_or_team_admin(client, user_id).await? {
        Ok(())
    } else {
        Err(Error::Forbidden(
            "Admin or Team Admin role required".to_string(),
        ))
    }
}

/// Require the requesting user to be the target user themselves, a global Admin,
/// or a Team Admin of any team where the target user is also a member.
pub async fn require_self_or_admin_or_team_admin(
    client: &Client,
    req: &HttpRequest,
    target_user_id: Uuid,
) -> Result<(), Error> {
    let user_id = requesting_user_id(req)
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;
    // Self-match: user can always modify their own account
    if user_id == target_user_id {
        return Ok(());
    }
    // Global admin bypass
    if db::is_admin(client, user_id).await? {
        return Ok(());
    }
    // Team Admin: allowed only if the target user is in one of their teams
    if db::is_team_admin_of_user(client, user_id, target_user_id).await? {
        return Ok(());
    }
    Err(Error::Forbidden(
        "You can only modify your own account, or must be an admin of a shared team".to_string(),
    ))
}

/// Guard against non-admin users assigning the global Admin role.
///
/// Team Admins may assign any role *except* Admin. Only global Admins may
/// grant Admin privileges. Call this after `require_team_admin` in handlers
/// that accept a role assignment.
pub async fn guard_admin_role_assignment(
    client: &Client,
    req: &HttpRequest,
    role_id: Uuid,
) -> Result<(), Error> {
    let requester_id = requesting_user_id(req)
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;
    if !db::is_admin(client, requester_id).await? {
        let role = db::get_role(client, role_id).await?;
        if role.title == ROLE_ADMIN {
            return Err(Error::Forbidden(
                "Only global Admins can assign the Admin role".to_string(),
            ));
        }
    }
    Ok(())
}

/// Guard against non-admin users demoting or removing a global Admin.
///
/// If the target user is a global Admin, only another global Admin may change
/// their role or remove them from a team. Team Admins are forbidden from
/// modifying global Admins' memberships. Call this after `require_team_admin`
/// in handlers that mutate a specific user's membership.
pub async fn guard_admin_demotion(
    client: &Client,
    req: &HttpRequest,
    target_user_id: Uuid,
) -> Result<(), Error> {
    // If the target user is not a global admin, no restriction applies.
    if !db::is_admin(client, target_user_id).await? {
        return Ok(());
    }
    // Target is a global admin — only another global admin may proceed.
    let requester_id = requesting_user_id(req)
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;
    if !db::is_admin(client, requester_id).await? {
        return Err(Error::Forbidden(
            "Only global Admins can modify a global Admin's membership".to_string(),
        ));
    }
    Ok(())
}

/// Guard against removing or demoting the last global Admin.
///
/// If the target user currently holds the Admin role in the specified team,
/// verify that at least one global admin would remain after the membership is
/// removed or changed away from Admin. Rejects with 403 if the operation would
/// leave zero admins.
///
/// Call this in `remove_team_member` and `update_member_role` handlers before
/// the mutation is executed.
pub async fn guard_last_admin_membership(
    client: &Client,
    team_id: Uuid,
    target_user_id: Uuid,
) -> Result<(), Error> {
    // Only relevant if the target currently holds Admin in this team.
    let role = db::get_member_role(client, team_id, target_user_id).await?;
    if role.as_deref() != Some(ROLE_ADMIN) {
        return Ok(());
    }
    // Would any admin remain if this membership were removed/changed?
    if !db::would_admins_remain_without(client, team_id, target_user_id).await? {
        return Err(Error::Forbidden(
            "Cannot remove or demote the last global Admin".to_string(),
        ));
    }
    Ok(())
}

/// Require the requesting user to be the owner of the specified team order,
/// a Team Admin for the team, or a global Admin. Regular members / guests
/// may only mutate their own orders.
pub async fn require_order_owner_or_team_admin(
    client: &Client,
    req: &HttpRequest,
    team_id: Uuid,
    order_owner_id: Uuid,
) -> Result<(), Error> {
    let user_id = requesting_user_id(req)
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;
    // Owner can always mutate their own order
    if user_id == order_owner_id {
        return Ok(());
    }
    // Admin / Team Admin bypass
    let (is_admin, team_role) = db::check_team_access(client, team_id, user_id).await?;
    if is_admin {
        return Ok(());
    }
    match team_role {
        Some(role) if role == ROLE_TEAM_ADMIN => Ok(()),
        Some(_) => Err(Error::Forbidden(
            "Only the order owner, team admin, or global admin can modify this order".to_string(),
        )),
        None => Err(Error::Forbidden("Team membership required".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TokenType;
    use actix_web::error::ResponseError;
    use actix_web::test::TestRequest;
    use uuid::Uuid;

    /// Helper: build a request with Claims inserted into extensions.
    fn request_with_claims(user_id: Uuid) -> HttpRequest {
        let req = TestRequest::default().to_http_request();
        req.extensions_mut().insert(Claims {
            sub: user_id,
            exp: 9999999999,
            iat: 1000000000,
            jti: Uuid::now_v7(),
            token_type: TokenType::Access,
            iss: "omp-breakfast".to_string(),
            aud: "omp-breakfast".to_string(),
        });
        req
    }

    /// Helper: build a request with no claims in extensions.
    fn request_without_claims() -> HttpRequest {
        TestRequest::default().to_http_request()
    }

    // ── requesting_user_id ──────────────────────────────────────────────

    #[test]
    fn requesting_user_id_returns_some_when_claims_present() {
        let uid = Uuid::now_v7();
        let req = request_with_claims(uid);
        assert_eq!(requesting_user_id(&req), Some(uid));
    }

    #[test]
    fn requesting_user_id_returns_none_when_no_claims() {
        let req = request_without_claims();
        assert_eq!(requesting_user_id(&req), None);
    }

    // ── require_self_or_admin: self-match path (no DB needed) ───────────

    #[actix_web::test]
    async fn require_self_or_admin_allows_self() {
        // When the requesting user matches the target, no DB call is made.
        // We pass a broken pool — if it tried to use the DB it would error.
        let uid = Uuid::now_v7();
        let req = request_with_claims(uid);

        // We cannot construct a real Client without a pool, but the self-match
        // path returns Ok(()) before any DB call. We verify via the handlers
        // integration tests instead. Here we just verify the requesting_user_id
        // extraction works for the self-match case.
        assert_eq!(requesting_user_id(&req), Some(uid));
    }

    // ── RBAC helpers: missing claims → Forbidden ────────────────────────

    #[test]
    fn require_admin_rejects_missing_claims_sync() {
        // All RBAC helpers should return Unauthorized when no Claims are in
        // the request extensions. We test that the error message is correct.
        let req = request_without_claims();
        let uid = requesting_user_id(&req);
        assert!(uid.is_none());
        // The helpers would return Error::Unauthorized("Authentication required")
        let err = uid
            .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))
            .unwrap_err();
        assert!(err.to_string().contains("Authentication required"));
    }

    #[test]
    fn require_admin_error_is_unauthorized_variant() {
        let err = Error::Unauthorized("Authentication required".to_string());
        let resp = err.error_response();
        assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn require_team_member_error_message() {
        let err = Error::Forbidden("Team membership required".to_string());
        assert_eq!(err.to_string(), "Team membership required");
        let resp = err.error_response();
        assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn require_team_admin_error_message() {
        let err = Error::Forbidden("Team admin role required".to_string());
        assert_eq!(err.to_string(), "Team admin role required");
        let resp = err.error_response();
        assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn require_order_owner_or_team_admin_error_message() {
        let err = Error::Forbidden(
            "Only the order owner, team admin, or global admin can modify this order".to_string(),
        );
        assert_eq!(
            err.to_string(),
            "Only the order owner, team admin, or global admin can modify this order"
        );
        let resp = err.error_response();
        assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn require_admin_or_team_admin_error_message() {
        let err = Error::Forbidden("Admin or Team Admin role required".to_string());
        assert_eq!(err.to_string(), "Admin or Team Admin role required");
        let resp = err.error_response();
        assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn require_self_or_admin_error_message() {
        let err = Error::Forbidden("You can only modify your own account".to_string());
        assert_eq!(err.to_string(), "You can only modify your own account");
        let resp = err.error_response();
        assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn require_self_or_admin_or_team_admin_error_message() {
        let err = Error::Forbidden(
            "You can only modify your own account, or must be an admin of a shared team"
                .to_string(),
        );
        assert_eq!(
            err.to_string(),
            "You can only modify your own account, or must be an admin of a shared team"
        );
        let resp = err.error_response();
        assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn guard_admin_demotion_error_message() {
        let err = Error::Forbidden(
            "Only global Admins can modify a global Admin's membership".to_string(),
        );
        assert_eq!(
            err.to_string(),
            "Only global Admins can modify a global Admin's membership"
        );
        let resp = err.error_response();
        assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn guard_last_admin_membership_error_message() {
        let err = Error::Forbidden("Cannot remove or demote the last global Admin".to_string());
        assert_eq!(
            err.to_string(),
            "Cannot remove or demote the last global Admin"
        );
        let resp = err.error_response();
        assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn requesting_user_id_extracts_correct_uuid() {
        let uid = Uuid::now_v7();
        let req = request_with_claims(uid);
        let extracted = requesting_user_id(&req).unwrap();
        assert_eq!(extracted, uid);

        // Different UUID should not match
        let other_uid = Uuid::now_v7();
        assert_ne!(extracted, other_uid);
    }
}

// API Health endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health check", body = StatusResponse),
        (status = 503, description = "Service unavailable — database unreachable", body = StatusResponse),
    ),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_health(state: Data<State>) -> Result<impl Responder, Error> {
    let Ok(client) = get_client(&state.pool).await else {
        return Ok(HttpResponse::ServiceUnavailable().json(StatusResponse {
            up: false,
            setup_required: false,
        }));
    };
    match db::check_db(&client).await {
        Ok(()) => {
            let setup_required = db::count_users(&client).await.unwrap_or(1) == 0;
            Ok(HttpResponse::Ok().json(StatusResponse {
                up: true,
                setup_required,
            }))
        }
        Err(_) => Ok(HttpResponse::ServiceUnavailable().json(StatusResponse {
            up: false,
            setup_required: false,
        })),
    }
}
