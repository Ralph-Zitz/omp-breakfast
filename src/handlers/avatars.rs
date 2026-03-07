use crate::{
    db,
    errors::{Error, ErrorResponse},
    handlers::*,
    models::*,
    validate::validate,
};
use actix_web::{
    HttpRequest, HttpResponse, Responder, http::header, web::Data, web::Json, web::Path,
};
use tracing::instrument;

/// GET /api/v1.0/avatars — list available avatars (id + name, no binary data).
///
/// Returns a bare `Vec` instead of `PaginatedResponse` — intentional exception:
/// avatars are a small, static set seeded from `minifigs/` at startup.
#[utoipa::path(
    get,
    path = "/api/v1.0/avatars",
    responses(
        (status = 200, description = "List of available avatars", body = Vec<AvatarListEntry>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_avatars(state: Data<State>) -> Result<impl Responder, Error> {
    let client = get_client(&state.pool).await?;
    let avatars = db::get_avatars(&client).await?;
    Ok(HttpResponse::Ok().json(avatars))
}

/// GET /api/v1.0/avatars/{avatar_id} — serve avatar image bytes.
/// Served from in-memory cache with aggressive caching headers.
#[utoipa::path(
    get,
    path = "/api/v1.0/avatars/{avatar_id}",
    params(
        ("avatar_id" = Uuid, Path, description = "Avatar ID"),
    ),
    responses(
        (status = 200, description = "Avatar image", content_type = "image/png"),
        (status = 404, description = "Avatar not found", body = ErrorResponse),
    ),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_avatar(
    state: Data<State>,
    path: Path<uuid::Uuid>,
) -> Result<impl Responder, Error> {
    let avatar_id = path.into_inner();

    // Try in-memory cache first
    if let Some(entry) = state.avatar_cache.get(&avatar_id) {
        let (data, content_type) = entry.value();
        return Ok(HttpResponse::Ok()
            .insert_header((header::CONTENT_TYPE, content_type.as_str()))
            .insert_header((header::CACHE_CONTROL, "public, max-age=31536000, immutable"))
            .body(data.clone()));
    }

    // Cache miss — fetch from DB and populate cache
    let client = get_client(&state.pool).await?;
    let (data, content_type) = db::get_avatar(&client, avatar_id).await?;
    state
        .avatar_cache
        .insert(avatar_id, (data.clone(), content_type.clone()));

    Ok(HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, content_type.as_str()))
        .insert_header((header::CACHE_CONTROL, "public, max-age=31536000, immutable"))
        .body(data))
}

/// PUT /api/v1.0/users/{user_id}/avatar — set a user's avatar.
/// Requires self, admin, or team admin of a shared team.
#[utoipa::path(
    put,
    path = "/api/v1.0/users/{user_id}/avatar",
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
    ),
    request_body = SetAvatarRequest,
    responses(
        (status = 200, description = "Updated user with new avatar", body = UserEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "User or avatar not found", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn set_avatar(
    state: Data<State>,
    path: Path<uuid::Uuid>,
    json: Json<SetAvatarRequest>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let user_id = path.into_inner();
    let client = get_client(&state.pool).await?;

    // RBAC: self, admin, or team admin of shared team
    require_self_or_admin_or_team_admin(&client, &req, user_id).await?;

    validate(&json)?;

    // Verify the avatar exists
    let _ = db::get_avatar(&client, json.avatar_id).await?;

    let user = db::set_user_avatar(&client, user_id, Some(json.avatar_id)).await?;
    Ok(HttpResponse::Ok().json(user))
}

/// DELETE /api/v1.0/users/{user_id}/avatar — remove a user's avatar (revert to initials).
/// Requires self, admin, or team admin of a shared team.
#[utoipa::path(
    delete,
    path = "/api/v1.0/users/{user_id}/avatar",
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
    ),
    responses(
        (status = 200, description = "Updated user without avatar", body = UserEntry),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[instrument(skip(state), level = "debug")]
pub async fn remove_avatar(
    state: Data<State>,
    path: Path<uuid::Uuid>,
    req: HttpRequest,
) -> Result<impl Responder, Error> {
    let user_id = path.into_inner();
    let client = get_client(&state.pool).await?;

    // RBAC: self, admin, or team admin of shared team
    require_self_or_admin_or_team_admin(&client, &req, user_id).await?;

    let user = db::set_user_avatar(&client, user_id, None).await?;
    Ok(HttpResponse::Ok().json(user))
}
