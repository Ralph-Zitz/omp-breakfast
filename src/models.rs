use chrono::{DateTime, Utc};
use dashmap::DashMap;
use deadpool_postgres::Pool;
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Display};

use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub jti: Uuid,
    pub token_type: TokenType,
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

/// Cached authentication data. Intentionally omits `Serialize` to prevent
/// accidental serialization of password hashes.
#[derive(Clone, Debug)]
pub struct AuthCacheEntry {
    pub user_id: Uuid,
    pub password_hash: String,
}

/// A cached user entry with a timestamp for TTL-based eviction.
#[derive(Clone, Debug)]
pub struct CachedUser {
    pub user: AuthCacheEntry,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct State {
    pub pool: Pool,
    pub jwtsecret: String,
    pub s3_key_id: String,
    pub s3_key_secret: String,
    pub cache: DashMap<String, CachedUser>,
    /// Revoked token JTIs mapped to their original expiry time.
    /// Entries are evicted by the background cleanup task once expired.
    pub token_blacklist: DashMap<String, DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct StatusResponse {
    pub up: bool,
}

#[derive(Serialize, ToSchema)]
pub struct RevokedResponse {
    pub revoked: bool,
}

#[derive(Serialize, ToSchema)]
pub struct DeletedResponse {
    pub deleted: bool,
}

#[derive(Serialize, ToSchema)]
pub struct UserEntry {
    pub user_id: Uuid,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub created: DateTime<Utc>,
    pub changed: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Clone, Validate)]
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
        max = 128,
        message = "password is required and must be between 8 and 128 characters"
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
#[derive(Deserialize, Serialize, Clone, Validate, Debug, ToSchema)]
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
    if password.len() > 128 {
        let mut err = validator::ValidationError::new("password");
        err.message = Some("password must not exceed 128 characters".into());
        return Err(err);
    }
    Ok(())
}

#[derive(Deserialize, Serialize, Clone, Validate, Debug, ToSchema)]
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
        max = 128,
        message = "password is required and must be between 8 and 128 characters"
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

#[derive(Serialize, ToSchema)]
pub struct TeamEntry {
    pub team_id: Uuid,
    pub tname: String,
    pub descr: Option<String>,
    pub created: DateTime<Utc>,
    pub changed: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct CreateTeamEntry {
    #[validate(length(
        min = 1,
        max = 255,
        message = "tname is required and must be between 1 and 255 characters"
    ))]
    pub tname: String,
    #[validate(length(
        max = 1000,
        message = "descr must not exceed 1000 characters"
    ))]
    pub descr: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Validate, Debug, ToSchema)]
pub struct UpdateTeamEntry {
    #[validate(length(
        min = 1,
        max = 255,
        message = "tname is required and must be between 1 and 255 characters"
    ))]
    pub tname: String,
    #[validate(length(
        max = 1000,
        message = "descr must not exceed 1000 characters"
    ))]
    pub descr: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct UserInTeams {
    pub tname: String,
    pub title: String,
    pub firstname: String,
    pub lastname: String,
}

#[derive(Serialize, ToSchema)]
pub struct RoleEntry {
    pub role_id: Uuid,
    pub title: String,
    pub created: DateTime<Utc>,
    pub changed: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct CreateRoleEntry {
    #[validate(length(
        min = 1,
        max = 255,
        message = "title is required and must be between 1 and 255 characters"
    ))]
    pub title: String,
}

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct UpdateRoleEntry {
    #[validate(length(
        min = 1,
        max = 255,
        message = "title is required and must be between 1 and 255 characters"
    ))]
    pub title: String,
}

// ── Item models ─────────────────────────────────────────────────────────────

#[derive(Serialize, ToSchema)]
pub struct ItemEntry {
    pub item_id: Uuid,
    pub descr: String,
    pub price: rust_decimal::Decimal,
    pub created: DateTime<Utc>,
    pub changed: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct CreateItemEntry {
    #[validate(length(
        min = 1,
        max = 255,
        message = "descr is required and must be between 1 and 255 characters"
    ))]
    pub descr: String,
    #[validate(custom(function = "validate_non_negative_price"))]
    pub price: rust_decimal::Decimal,
}

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct UpdateItemEntry {
    #[validate(length(
        min = 1,
        max = 255,
        message = "descr is required and must be between 1 and 255 characters"
    ))]
    pub descr: String,
    #[validate(custom(function = "validate_non_negative_price"))]
    pub price: rust_decimal::Decimal,
}

fn validate_non_negative_price(
    price: &rust_decimal::Decimal,
) -> Result<(), validator::ValidationError> {
    if *price < rust_decimal::Decimal::ZERO {
        let mut err = validator::ValidationError::new("price");
        err.message = Some("price must be zero or positive".into());
        return Err(err);
    }
    Ok(())
}

// ── Team order models ───────────────────────────────────────────────────────

#[derive(Serialize, ToSchema)]
pub struct TeamOrderEntry {
    pub teamorders_id: Uuid,
    pub teamorders_team_id: Uuid,
    pub teamorders_user_id: Option<Uuid>,
    pub duedate: Option<chrono::NaiveDate>,
    pub closed: bool,
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

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct AddMemberEntry {
    pub user_id: Uuid,
    pub role_id: Uuid,
}

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct UpdateMemberRoleEntry {
    pub role_id: Uuid,
}

// ── Order models (individual items within a team order) ─────────────────────

#[derive(Serialize, ToSchema)]
pub struct OrderEntry {
    pub orders_teamorders_id: Uuid,
    pub orders_item_id: Uuid,
    pub orders_team_id: Uuid,
    pub amt: i32,
    pub created: DateTime<Utc>,
    pub changed: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct CreateOrderEntry {
    pub orders_item_id: Uuid,
    #[validate(range(min = 1, message = "quantity must be at least 1"))]
    pub amt: i32,
}

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct UpdateOrderEntry {
    #[validate(range(min = 1, message = "quantity must be at least 1"))]
    pub amt: i32,
}
