use crate::{
    db,
    errors::{Error, ErrorResponse},
    handlers::*,
    middleware::auth::{
        generate_token_pair, invalidate_cache, is_token_revoked, revoke_token, verify_jwt,
    },
    models::*,
    validate::validate,
};
use actix_web::{
    HttpRequest, HttpResponse, Responder, http::header, web::Data, web::Json, web::Path,
};
use actix_web_httpauth::extractors::{basic::BasicAuth, bearer::BearerAuth};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use tracing::instrument;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v1.0/users",
    responses(
        (status = 200, description = "List of Users", body = [UserEntry]),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_users(state: Data<State>) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    let users = db::get_users(&client).await?;
    Ok(HttpResponse::Ok().json(users))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/users/{id}",
    responses(
        (status = 200, description = "User found", body = UserEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
    ),
    params(
        ("id", description = "Unique UUID of User")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_user(state: Data<State>, path: Path<Uuid>) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    let user = db::get_user(&client, path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(user))
}

#[utoipa::path(
    post,
    path = "/auth",
    responses(
        (status = 200, description = "Authentication successful", body = Auth),
        (status = 401, description = "Unauthorized"),
    ),
    security(("basic_auth" = [])),
)]
#[instrument(skip(basic, state), level = "debug")]
pub async fn auth_user(basic: BasicAuth, state: Data<State>) -> Result<impl Responder, Error> {
    let cache = &state.cache;
    if let Some(cached) = cache.get(&basic.user_id().to_string()) {
        let auth = generate_token_pair(cached.user.user_id, state.jwtsecret.clone()).await?;
        Ok(HttpResponse::Ok().json(auth))
    } else {
        return Err(Error::Unauthorized("Unauthorized".to_string()));
    }
}

#[utoipa::path(
    post,
    path = "/auth/refresh",
    responses(
        (status = 200, description = "Token refreshed successfully", body = Auth),
        (status = 401, description = "Invalid or expired refresh token", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(credentials, state), level = "debug")]
pub async fn refresh_token(
    credentials: BearerAuth,
    state: Data<State>,
) -> Result<impl Responder, Error> {
    let jwt_secret = state.jwtsecret.clone();
    let claims = verify_jwt(credentials.token().to_string(), jwt_secret.clone()).await?;

    // Defence-in-depth: refresh_validator middleware already rejects non-refresh tokens
    // before this handler is reached, but we check again here as a safety net.
    if claims.claims.token_type != "refresh" {
        return Err(Error::Unauthorized(
            "Invalid token type, refresh token required".to_string(),
        ));
    }

    // Verify that the user still exists (also need client for revocation checks)
    let client: Client = get_client(state.pool.clone()).await?;

    // Check if revoked
    if is_token_revoked(&client, &state, &claims.claims.jti.to_string()).await? {
        return Err(Error::Unauthorized("Token has been revoked".to_string()));
    }

    db::get_user(&client, claims.claims.sub).await?;

    // Revoke the old refresh token (rotation)
    let expires_at = DateTime::<Utc>::from_timestamp(claims.claims.exp, 0).unwrap_or_else(Utc::now);
    revoke_token(&client, &state, &claims.claims.jti.to_string(), expires_at).await?;

    // Issue a new token pair
    let auth = generate_token_pair(claims.claims.sub, jwt_secret).await?;
    Ok(HttpResponse::Ok().json(auth))
}

#[utoipa::path(
    post,
    path = "/auth/revoke",
    request_body = TokenRequest,
    responses(
        (status = 200, description = "Token revoked successfully", body = StatusResponse),
        (status = 400, description = "Invalid token", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state, json), level = "debug")]
pub async fn revoke_user_token(
    state: Data<State>,
    json: Json<TokenRequest>,
) -> Result<impl Responder, Error> {
    let jwt_secret = state.jwtsecret.clone();
    let token_data = verify_jwt(json.into_inner().token, jwt_secret).await?;

    // Revoke by jti — persist to DB
    let client: Client = get_client(state.pool.clone()).await?;
    let expires_at =
        DateTime::<Utc>::from_timestamp(token_data.claims.exp, 0).unwrap_or_else(Utc::now);
    revoke_token(
        &client,
        &state,
        &token_data.claims.jti.to_string(),
        expires_at,
    )
    .await?;

    Ok(HttpResponse::Ok().json(StatusResponse { up: true }))
}

#[utoipa::path(
    post,
    request_body = CreateUserEntry,
    path = "/api/v1.0/users",
    params(
        CreateUserEntry
    ),
    responses(
        (status = 201, description = "User created", body = UserEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin or team admin role required", body = ErrorResponse),
        (status = 404, description = "User not created", body = ErrorResponse),
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
    let client: Client = get_client(state.pool.clone()).await?;

    // RBAC: admin or team admin
    require_admin_or_team_admin(&client, &req).await?;

    let user = db::create_user(&client, json.into_inner()).await?;
    let mut response = HttpResponse::Created();
    if let Ok(url) = req.url_for("/users/user_id", [user.user_id.to_string()]) {
        response.append_header((header::LOCATION, url.as_str().to_owned()));
    }
    Ok(response.json(user))
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/users/{id}",
    responses(
        (status = 200, description = "User deleted successfully", body = DeletedResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - can only delete own account, requires admin, or team admin of a shared team", body = ErrorResponse),
        (status = 404, description = "User not deleted", body = DeletedResponse),
    ),
    params(
        ("id", description = "Unique UUID of the User")
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
    let client: Client = get_client(state.pool.clone()).await?;

    // RBAC: self, global admin, or team admin of a shared team
    require_self_or_admin_or_team_admin(&client, &req, uid).await?;

    // Fetch user email before deletion to invalidate the auth cache
    if let Ok(user) = db::get_user(&client, uid).await {
        invalidate_cache(state.clone(), &user.email);
    }

    let deleted = db::delete_user(&client, uid).await?;
    if deleted {
        Ok(HttpResponse::Ok().json(DeletedResponse { deleted }))
    } else {
        Ok(HttpResponse::NotFound().json(DeletedResponse { deleted }))
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1.0/users/email/{email}",
    responses(
        (status = 200, description = "User deleted successfully", body = DeletedResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - can only delete own account, requires admin, or team admin of a shared team", body = ErrorResponse),
        (status = 404, description = "User not deleted", body = DeletedResponse),
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
    let client: Client = get_client(state.pool.clone()).await?;

    // RBAC: self, global admin, or team admin of a shared team
    match db::get_user_by_email(&client, &email).await {
        Ok(user) => {
            require_self_or_admin_or_team_admin(&client, &req, user.user_id).await?;
        }
        Err(_) => {
            // User not found — still enforce admin check to prevent info leakage
            require_admin(&client, &req).await?;
            return Ok(HttpResponse::NotFound().json(DeletedResponse { deleted: false }));
        }
    }

    let deleted = db::delete_user_by_email(&client, &email).await?;
    if deleted {
        // Invalidate the auth cache so the deleted user cannot authenticate
        invalidate_cache(state.clone(), &email);
        Ok(HttpResponse::Ok().json(DeletedResponse { deleted }))
    } else {
        Ok(HttpResponse::NotFound().json(DeletedResponse { deleted }))
    }
}

#[utoipa::path(
    put,
    path = "/api/v1.0/users/{id}",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = UserEntry),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
        (status = 403, description = "Forbidden - can only update own account, requires admin, or team admin of a shared team", body = ErrorResponse),
        (status = 404, description = "User not updated", body = ErrorResponse),
    ),
    params(
        ("id", description = "Unique UUID of the User"),
        UpdateUserRequest
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
    let client: Client = get_client(state.pool.clone()).await?;

    // RBAC: self, global admin, or team admin of a shared team
    require_self_or_admin_or_team_admin(&client, &req, uid).await?;

    validate(&json)?;
    let user = db::update_user(&client, uid, json.into_inner()).await?;
    invalidate_cache(state, &user.email);
    Ok(HttpResponse::Ok().json(user))
}

#[utoipa::path(
    get,
    path = "/api/v1.0/users/{id}/teams",
    responses(
        (status = 200, description = "List of Teams the User is a member of", body = [UserInTeams]),
        (status = 401, description = "Unauthorized - invalid or missing JWT token", body = ErrorResponse),
    ),
    params(
        ("id", description = "Unique UUID of the User")
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn user_teams(state: Data<State>, path: Path<Uuid>) -> Result<impl Responder, Error> {
    let client: Client = get_client(state.pool.clone()).await?;
    let teams = db::get_user_teams(&client, path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(teams))
}
