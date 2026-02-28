use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use flurry::HashMap;
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Display};
use tokio_pg_mapper::PostgresMapper;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub jti: Uuid,
    pub token_type: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Auth {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TokenRequest {
    pub token: String,
}

/// A cached user entry with a timestamp for TTL-based eviction.
#[derive(Clone, Debug)]
pub struct CachedUser {
    pub user: UpdateUserEntry,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct State {
    pub pool: Pool,
    pub secret: String,
    pub jwtsecret: String,
    pub s3_key_id: String,
    pub s3_key_secret: String,
    pub cache: HashMap<String, CachedUser>,
    pub token_blacklist: HashMap<String, bool>,
}

#[derive(Serialize, ToSchema)]
pub struct StatusResponse {
    pub up: bool,
}

#[derive(Serialize, ToSchema)]
pub struct DeletedResponse {
    pub deleted: bool,
}

#[derive(Serialize, PostgresMapper, ToSchema)]
#[pg_mapper(table = "users")]
pub struct UserEntry {
    pub user_id: Uuid,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub created: DateTime<Utc>,
    pub changed: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, PostgresMapper, Clone, Validate, ToSchema, IntoParams)]
#[pg_mapper(table = "users")]
pub struct UpdateUserEntry {
    pub user_id: Uuid,
    #[validate(length(
        min = 2,
        max = 50,
        message = "firstname is required and must be between 2 and 50 characters"
    ))]
    pub firstname: String,
    #[validate(length(
        min = 2,
        max = 50,
        message = "lastname is required and must be between 2 and 50 characters"
    ))]
    pub lastname: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(
        min = 8,
        message = "password is required and must be at least 8 characters"
    ))]
    pub password: String,
}

impl Display for UpdateUserEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.firstname, self.lastname)
    }
}

impl fmt::Debug for UpdateUserEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.firstname, self.lastname)
    }
}

/// API request body for updating a user. Password is optional — if omitted,
/// the existing password is preserved (avoids unnecessary rehashing).
#[derive(Deserialize, Serialize, Clone, Validate, Debug, ToSchema, IntoParams)]
pub struct UpdateUserRequest {
    #[validate(length(
        min = 2,
        max = 50,
        message = "firstname is required and must be between 2 and 50 characters"
    ))]
    pub firstname: String,
    #[validate(length(
        min = 2,
        max = 50,
        message = "lastname is required and must be between 2 and 50 characters"
    ))]
    pub lastname: String,
    #[validate(email)]
    pub email: String,
    #[validate(custom(function = "validate_optional_password"))]
    pub password: Option<String>,
}

// Signature uses `&String` because the `validator` crate passes the inner type of Option<String> by reference.
#[allow(clippy::ptr_arg)]
fn validate_optional_password(password: &String) -> Result<(), validator::ValidationError> {
    if password.len() < 8 {
        let mut err = validator::ValidationError::new("password");
        err.message = Some("password must be at least 8 characters".into());
        return Err(err);
    }
    Ok(())
}

#[derive(Deserialize, Serialize, PostgresMapper, Clone, Validate, Debug, ToSchema, IntoParams)]
#[pg_mapper(table = "users")]
pub struct CreateUserEntry {
    #[validate(length(
        min = 2,
        max = 50,
        message = "firstname is required and must be between 2 and 50 characters"
    ))]
    pub firstname: String,
    #[validate(length(
        min = 2,
        max = 50,
        message = "lastname is required and must be between 2 and 50 characters"
    ))]
    pub lastname: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(
        min = 8,
        message = "password is required and must be at least 8 characters"
    ))]
    pub password: String,
}

#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct UsersInTeam {
    pub user_id: Uuid,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub title: String,
}

#[derive(Serialize, PostgresMapper, ToSchema)]
#[pg_mapper(table = "teams")]
pub struct TeamEntry {
    pub team_id: Uuid,
    pub tname: String,
    pub descr: Option<String>,
}

#[derive(Deserialize, Serialize, PostgresMapper, Clone, Validate, Debug, ToSchema)]
#[pg_mapper(table = "teams")]
pub struct CreateTeamEntry {
    #[validate(length(
        min = 1,
        message = "tname is required and must be at least 1 character"
    ))]
    pub tname: String,
    pub descr: Option<String>,
}

#[derive(Deserialize, Serialize, PostgresMapper, Clone, Validate, Debug, ToSchema)]
#[pg_mapper(table = "teams")]
pub struct UpdateTeamEntry {
    #[validate(length(
        min = 1,
        message = "tname is required and must be at least 1 character"
    ))]
    pub tname: String,
    pub descr: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct UserInTeams {
    pub tname: String,
    pub title: String,
    pub firstname: String,
    pub lastname: String,
}

#[derive(Serialize, PostgresMapper, ToSchema)]
#[pg_mapper(table = "roles")]
pub struct RoleEntry {
    pub role_id: Uuid,
    pub title: String,
}

#[derive(Deserialize, Serialize, PostgresMapper, Validate, Clone, Debug, ToSchema)]
#[pg_mapper(table = "roles")]
pub struct CreateRoleEntry {
    #[validate(length(
        min = 1,
        message = "title is required and must be at least 1 character"
    ))]
    pub title: String,
}

#[derive(Deserialize, Serialize, PostgresMapper, Validate, Clone, Debug, ToSchema)]
#[pg_mapper(table = "roles")]
pub struct UpdateRoleEntry {
    #[validate(length(
        min = 1,
        message = "title is required and must be at least 1 character"
    ))]
    pub title: String,
}

// ── Item models ─────────────────────────────────────────────────────────────

#[derive(Serialize, PostgresMapper, ToSchema)]
#[pg_mapper(table = "items")]
pub struct ItemEntry {
    pub item_id: Uuid,
    pub descr: String,
    pub price: Option<rust_decimal::Decimal>,
    pub created: DateTime<Utc>,
    pub changed: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, PostgresMapper, Validate, Clone, Debug, ToSchema)]
#[pg_mapper(table = "items")]
pub struct CreateItemEntry {
    #[validate(length(
        min = 1,
        message = "descr is required and must be at least 1 character"
    ))]
    pub descr: String,
    pub price: Option<rust_decimal::Decimal>,
}

#[derive(Deserialize, Serialize, PostgresMapper, Validate, Clone, Debug, ToSchema)]
#[pg_mapper(table = "items")]
pub struct UpdateItemEntry {
    #[validate(length(
        min = 1,
        message = "descr is required and must be at least 1 character"
    ))]
    pub descr: String,
    pub price: Option<rust_decimal::Decimal>,
}

// ── Team order models ───────────────────────────────────────────────────────

#[derive(Serialize, PostgresMapper, ToSchema)]
#[pg_mapper(table = "teamorders")]
pub struct TeamOrderEntry {
    pub teamorders_id: Uuid,
    pub teamorders_team_id: Uuid,
    pub teamorders_user_id: Option<Uuid>,
    pub duedate: Option<chrono::NaiveDate>,
    pub closed: Option<bool>,
    pub created: DateTime<Utc>,
    pub changed: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct CreateTeamOrderEntry {
    pub teamorders_user_id: Option<Uuid>,
    pub duedate: Option<chrono::NaiveDate>,
}

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct UpdateTeamOrderEntry {
    pub teamorders_user_id: Option<Uuid>,
    pub duedate: Option<chrono::NaiveDate>,
    pub closed: Option<bool>,
}

// ── Memberof models ─────────────────────────────────────────────────────────

#[derive(Deserialize, Serialize, Clone, Debug, ToSchema)]
pub struct AddMemberEntry {
    pub user_id: Uuid,
    pub role_id: Uuid,
}

#[derive(Deserialize, Serialize, Clone, Debug, ToSchema)]
pub struct UpdateMemberRoleEntry {
    pub role_id: Uuid,
}
