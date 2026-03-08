use crate::{
    db,
    errors::{Error, ErrorResponse},
    handlers::*,
    middleware::auth::{
        REFRESH_TOKEN_DURATION_DAYS, generate_token_pair, invalidate_cache, is_token_revoked,
        revoke_token, verify_jwt_for_revocation,
    },
    models::*,
    validate::validate,
};
use actix_web::{
    HttpRequest, HttpResponse, Responder, web::Data, web::Json, web::Path, web::Query,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use argon2::password_hash::PasswordVerifier;
use chrono::{DateTime, Duration, Utc};
use secrecy::ExposeSecret;
use tracing::instrument;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v1.0/users",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of Users", body = PaginatedResponse<UserEntry>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_users(
    state: Data<State>,
    pagination: Query<PaginationParams>,
) -> Result<impl Responder, Error> {
    let (limit, offset) = pagination.sanitize();
    let client: Client = get_client(&state.pool).await?;
    let (users, total) = db::get_users(&client, limit, offset).await?;
    Ok(HttpResponse::Ok().json(PaginatedResponse {
        items: users,
        total,
        limit,
        offset,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/users/{user_id}",
    responses(
        (status = 200, description = "User found", body = UserEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
    ),
    params(
        ("user_id", description = "Unique UUID of User")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_user(state: Data<State>, path: Path<Uuid>) -> Result<impl Responder, Error> {
    let client: Client = get_client(&state.pool).await?;
    let user = db::get_user(&client, path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(user))
}

#[utoipa::path(
    post,
    path = "/auth",
    responses(
        (status = 200, description = "Authentication successful", body = Auth),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 429, description = "Too Many Requests - rate limited or account temporarily locked", body = ErrorResponse),
    ),
    security(("basic_auth" = [])),
)]
#[instrument(skip(basic, state), level = "debug")]
pub async fn auth_user(basic: BasicAuth, state: Data<State>) -> Result<impl Responder, Error> {
    // Credentials verified by basic_validator middleware; cache is guaranteed populated.
    let user_id = state
        .cache
        .get(&basic.user_id().to_string())
        .map(|cached| cached.user.user_id)
        .ok_or_else(|| Error::Unauthorized("Unauthorized".to_string()))?;
    let auth = generate_token_pair(user_id, state.jwtsecret.expose_secret())?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(auth))
}

#[utoipa::path(
    post,
    request_body = CreateUserEntry,
    path = "/auth/register",
    responses(
        (status = 201, description = "First admin user created", body = UserEntry),
        (status = 403, description = "Forbidden - registration is closed (users already exist)", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
)]
#[instrument(skip(state, json), level = "debug")]
pub async fn register_first_user(
    state: Data<State>,
    json: Json<CreateUserEntry>,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let mut client: Client = get_client(&state.pool).await?;
    let user = db::bootstrap_first_user(&mut client, json.into_inner()).await?;
    Ok(HttpResponse::Created().json(user))
}

#[utoipa::path(
    post,
    path = "/auth/refresh",
    request_body(content = Option<RefreshRequest>, description = "Optional: include the old access token to revoke it immediately"),
    responses(
        (status = 200, description = "Token refreshed successfully", body = Auth),
        (status = 401, description = "Invalid or expired refresh token", body = ErrorResponse),
        (status = 429, description = "Too Many Requests - rate limited", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req, body), level = "debug")]
pub async fn refresh_token(
    state: Data<State>,
    req: HttpRequest,
    body: Option<Json<RefreshRequest>>,
) -> Result<impl Responder, Error> {
    // Claims are already decoded and validated by the refresh_validator middleware
    // and stored in request extensions — avoid redundant re-decode.
    let claims = req
        .extensions()
        .get::<Claims>()
        .cloned()
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;

    // Defence-in-depth: refresh_validator middleware already rejects non-refresh tokens
    // before this handler is reached, but we check again here as a safety net.
    if claims.token_type != TokenType::Refresh {
        return Err(Error::Unauthorized(
            "Invalid token type, refresh token required".to_string(),
        ));
    }

    // Verify that the user still exists (also need client for revocation checks)
    let client: Client = get_client(&state.pool).await?;

    // Check if revoked
    if is_token_revoked(&client, &state, &claims.jti.to_string()).await? {
        return Err(Error::Unauthorized("Token has been revoked".to_string()));
    }

    db::get_user(&client, claims.sub).await?;

    // Revoke the old refresh token (rotation)
    let expires_at = DateTime::<Utc>::from_timestamp(claims.exp, 0).unwrap_or_else(|| {
        Utc::now() + Duration::try_days(REFRESH_TOKEN_DURATION_DAYS).expect("valid duration")
    });
    revoke_token(&client, &state, &claims.jti.to_string(), expires_at).await?;

    // Also revoke the old access token if the client supplied it, closing the
    // 15-minute window where a leaked access token remains usable.
    if let Some(Json(RefreshRequest {
        access_token: Some(old_access),
    })) = body
        && let Ok(td) = verify_jwt_for_revocation(&old_access, state.jwtsecret.expose_secret())
        && td.sub == claims.sub
        && td.token_type == TokenType::Access
    {
        let at_exp = DateTime::<Utc>::from_timestamp(td.exp, 0).unwrap_or_else(Utc::now);
        let _ = revoke_token(&client, &state, &td.jti.to_string(), at_exp).await;
    }

    // Issue a new token pair
    let auth = generate_token_pair(claims.sub, state.jwtsecret.expose_secret())?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(auth))
}

#[utoipa::path(
    post,
    path = "/auth/revoke",
    request_body = TokenRequest,
    responses(
        (status = 200, description = "Token revoked successfully", body = RevokedResponse),
        (status = 400, description = "Bad request - token is invalid or expired", body = ErrorResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - cannot revoke another user's token", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, json, req), level = "debug")]
pub async fn revoke_user_token(
    state: Data<State>,
    json: Json<TokenRequest>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    // Use lenient verification for revocation: skip expiry check so that
    // legitimately-expired tokens can still be revoked (harmless, and the
    // signature is still validated).
    let token_data = match verify_jwt_for_revocation(
        &json.into_inner().token,
        state.jwtsecret.expose_secret(),
    ) {
        Ok(data) => data,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                error: "Token is invalid or expired".to_string(),
            }));
        }
    };

    // Ownership check: the token being revoked must belong to the requesting user,
    // unless the requester is a global admin.
    let requester_id = requesting_user_id(&req)
        .ok_or_else(|| Error::Unauthorized("Authentication required".to_string()))?;

    let client: Client = get_client(&state.pool).await?;

    if token_data.sub != requester_id && !db::is_admin(&client, requester_id).await? {
        return Err(Error::Forbidden(
            "Cannot revoke another user's token".to_string(),
        ));
    }

    // Revoke by jti — persist to DB
    let expires_at = DateTime::<Utc>::from_timestamp(token_data.exp, 0).unwrap_or_else(|| {
        Utc::now() + Duration::try_days(REFRESH_TOKEN_DURATION_DAYS).expect("valid duration")
    });
    revoke_token(&client, &state, &token_data.jti.to_string(), expires_at).await?;

    // Invalidate the auth cache for the token's owner so any cached credentials
    // are flushed and the next Basic Auth attempt must re-verify against the DB.
    if let Ok(user) = db::get_user(&client, token_data.sub).await {
        let _ = invalidate_cache(state.clone(), &user.email);
    }

    Ok(HttpResponse::Ok().json(RevokedResponse { revoked: true }))
}

#[utoipa::path(
    post,
    request_body = CreateUserEntry,
    path = "/api/v1.0/users",
    responses(
        (status = 201, description = "User created", body = UserEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin or team admin role required", body = ErrorResponse),
        (status = 409, description = "Conflict - email already exists", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req, json), level = "debug")]
pub async fn create_user(
    state: Data<State>,
    json: Json<CreateUserEntry>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    validate(&json)?;
    let client: Client = get_client(&state.pool).await?;

    // RBAC: admin or team admin
    require_admin_or_team_admin(&client, &req).await?;

    let user = db::create_user(&client, json.into_inner()).await?;
    Ok(created_with_location(
        &req,
        &user,
        "/users/user_id",
        &[user.user_id.to_string()],
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/users/{user_id}",
    responses(
        (status = 200, description = "User deleted successfully", body = DeletedResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - can only delete own account, requires admin, or team admin of a shared team", body = ErrorResponse),
        (status = 404, description = "User not deleted", body = DeletedResponse),
        (status = 409, description = "Conflict - user still owns team orders", body = ErrorResponse),
    ),
    params(
        ("user_id", description = "Unique UUID of the User")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn delete_user(
    state: Data<State>,
    path: Path<Uuid>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let uid = path.into_inner();
    let mut client: Client = get_client(&state.pool).await?;

    // RBAC: self, global admin, or team admin of a shared team
    require_self_or_admin_or_team_admin(&client, &req, uid).await?;

    // Prevent non-admin users from deleting a global Admin's account
    guard_admin_demotion(&client, &req, uid).await?;

    // Prevent the last admin from deleting themselves
    let caller_id = requesting_user_id(&req);
    if caller_id == Some(uid) && db::is_admin(&client, uid).await? {
        let admin_count = db::count_admins(&client).await?;
        if admin_count <= 1 {
            return Err(Error::Forbidden(
                "Cannot delete the last admin account".to_string(),
            ));
        }
    }

    // Fetch user email before deletion so we can invalidate the auth cache after
    let user_email = db::get_user(&client, uid).await.ok().map(|u| u.email);

    // Prevent deletion when the user still owns team orders (FK constraint)
    let order_count = db::count_user_team_orders(&client, uid).await?;
    if order_count > 0 {
        return Err(Error::Conflict(format!(
            "Cannot delete user — they own {order_count} team order(s). Reassign or delete the orders first."
        )));
    }

    let deleted = db::delete_user(&mut client, uid).await?;
    if deleted {
        // Invalidate auth cache after successful deletion
        if let Some(email) = user_email {
            let _ = invalidate_cache(state.clone(), &email);
        }
    }
    Ok(delete_response(deleted))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/users/email/{email}",
    responses(
        (status = 200, description = "User deleted successfully or user not found", body = DeletedResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - can only delete own account, requires admin, or team admin of a shared team", body = ErrorResponse),
        (status = 409, description = "Conflict - user still owns team orders", body = ErrorResponse),
        (status = 422, description = "Validation error - invalid email format", body = ErrorResponse),
    ),
    params(
        ("email", description = "Email of the User")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req), level = "debug")]
pub async fn delete_user_by_email(
    state: Data<State>,
    path: Path<String>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let email = path.into_inner();

    // Basic email format validation — reject obviously invalid paths before DB query
    if email.len() > 255 || !email.contains('@') {
        return Err(Error::Validation("Invalid email format".to_string()));
    }

    let mut client: Client = get_client(&state.pool).await?;

    // RBAC: self, global admin, or team admin of a shared team
    match db::get_user_by_email(&client, &email).await {
        Ok(user) => {
            require_self_or_admin_or_team_admin(&client, &req, user.user_id).await?;

            // Prevent non-admin users from deleting a global Admin's account
            guard_admin_demotion(&client, &req, user.user_id).await?;

            // Prevent the last admin from deleting themselves
            let caller_id = requesting_user_id(&req);
            if caller_id == Some(user.user_id) && db::is_admin(&client, user.user_id).await? {
                let admin_count = db::count_admins(&client).await?;
                if admin_count <= 1 {
                    return Err(Error::Forbidden(
                        "Cannot delete the last admin account".to_string(),
                    ));
                }
            }
        }
        Err(_) => {
            // User not found — still enforce admin check to prevent info leakage.
            // Return 200 (not 404) with deleted:false to avoid email oracle.
            require_admin(&client, &req).await?;
            return Ok(HttpResponse::Ok().json(DeletedResponse { deleted: false }));
        }
    }

    // Prevent deletion when the user still owns team orders (FK constraint)
    if let Ok(user) = db::get_user_by_email(&client, &email).await {
        let order_count = db::count_user_team_orders(&client, user.user_id).await?;
        if order_count > 0 {
            return Err(Error::Conflict(format!(
                "Cannot delete user — they own {order_count} team order(s). Reassign or delete the orders first."
            )));
        }
    }

    let deleted = db::delete_user_by_email(&mut client, &email).await?;
    if deleted {
        // Invalidate the auth cache so the deleted user cannot authenticate
        let _ = invalidate_cache(state.clone(), &email);
    }
    Ok(delete_response(deleted))
}

#[utoipa::path(
    put,
    path = "/api/v1.0/users/{user_id}",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = UserEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - can only update own account (requires admin or team admin of a shared team), or current password is incorrect", body = ErrorResponse),
        (status = 404, description = "User not updated", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    params(
        ("user_id", description = "Unique UUID of the User")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, req, json), level = "debug")]
pub async fn update_user(
    state: Data<State>,
    path: Path<Uuid>,
    json: Json<UpdateUserRequest>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let uid = path.into_inner();
    validate(&json)?;
    let client: Client = get_client(&state.pool).await?;

    // RBAC: self, global admin, or team admin of a shared team
    require_self_or_admin_or_team_admin(&client, &req, uid).await?;

    // Prevent non-admin users from modifying a global Admin's account
    guard_admin_demotion(&client, &req, uid).await?;

    let update_req = json.into_inner();

    // If the user is changing their own password, verify current_password.
    // Admins (and team admins) resetting another user's password skip this check.
    if update_req.password.is_some() {
        let caller_id = requesting_user_id(&req);
        let is_self_update = caller_id == Some(uid);

        if is_self_update {
            let current_pw = update_req.current_password.as_deref().ok_or_else(|| {
                Error::Validation(
                    "current_password is required when changing your own password".to_string(),
                )
            })?;

            let stored_hash = db::get_password_hash(&client, uid).await?;
            let current_pw = current_pw.to_string();
            let verify_ok = tokio::task::spawn_blocking(move || {
                let parsed_hash =
                    argon2::PasswordHash::new(&stored_hash).map_err(|e| e.to_string())?;
                crate::argon2_hasher()
                    .verify_password(current_pw.as_bytes(), &parsed_hash)
                    .map_err(|_| "mismatch".to_string())
            })
            .await
            .map_err(|e| Error::Argon2(e.to_string()))?;

            match verify_ok {
                Err(ref e) if e == "mismatch" => {
                    return Err(Error::Forbidden(
                        "Current password is incorrect".to_string(),
                    ));
                }
                Err(e) => return Err(Error::Argon2(e)),
                Ok(()) => {}
            }
        }
    }

    // Fetch old email before update so we can invalidate the correct cache key
    let old_email = db::get_user(&client, uid)
        .await
        .ok()
        .map(|u| u.email.clone());

    let user = db::update_user(&client, uid, update_req).await?;

    // Invalidate both old and new email cache entries
    if let Some(ref old) = old_email
        && old != &user.email
    {
        let _ = invalidate_cache(state.clone(), old);
    }
    let _ = invalidate_cache(state, &user.email);
    Ok(HttpResponse::Ok().json(user))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/users/{user_id}/teams",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of Teams the User is a member of", body = PaginatedResponse<UserInTeams>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
    ),
    params(
        ("user_id", description = "Unique UUID of the User")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn user_teams(
    state: Data<State>,
    path: Path<Uuid>,
    pagination: Query<PaginationParams>,
) -> Result<impl Responder, Error> {
    let (limit, offset) = pagination.sanitize();
    let client: Client = get_client(&state.pool).await?;
    let (teams, total) = db::get_user_teams(&client, path.into_inner(), limit, offset).await?;
    Ok(HttpResponse::Ok().json(PaginatedResponse {
        items: teams,
        total,
        limit,
        offset,
    }))
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Duration, Utc};

    use crate::middleware::auth::REFRESH_TOKEN_DURATION_DAYS;

    /// Verify that `DateTime::from_timestamp` returns `None` for absurd exp
    /// values, triggering the `unwrap_or_else` fallback in `refresh_token`.
    #[test]
    fn datetime_from_timestamp_fallback_on_extreme_values() {
        // A valid timestamp should parse successfully
        let now = Utc::now().timestamp();
        assert!(
            DateTime::<Utc>::from_timestamp(now, 0).is_some(),
            "current timestamp should parse"
        );

        // i64::MAX should fail — beyond representable range
        assert!(
            DateTime::<Utc>::from_timestamp(i64::MAX, 0).is_none(),
            "i64::MAX should trigger the fallback"
        );

        // Negative extreme should also fail
        assert!(
            DateTime::<Utc>::from_timestamp(i64::MIN, 0).is_none(),
            "i64::MIN should trigger the fallback"
        );

        // Verify the fallback produces a reasonable default (now + 7 days)
        let fallback = DateTime::<Utc>::from_timestamp(i64::MAX, 0).unwrap_or_else(|| {
            Utc::now() + Duration::try_days(REFRESH_TOKEN_DURATION_DAYS).expect("valid duration")
        });
        let expected = Utc::now() + Duration::try_days(7).expect("valid duration");
        assert!(
            (fallback - expected).num_seconds().abs() < 5,
            "fallback should be ~7 days from now"
        );
    }
}
