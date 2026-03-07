use chrono::{DateTime, Utc};
use dashmap::DashMap;
use deadpool_postgres::Pool;
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Display};

use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

// ── Pagination ──────────────────────────────────────────────────────────────

/// Default number of items per page.
pub const DEFAULT_PAGE_LIMIT: i64 = 50;
/// Maximum allowed items per page.
pub const MAX_PAGE_LIMIT: i64 = 100;

/// Query parameters for paginated list endpoints.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationParams {
    /// Maximum number of items to return (1–100, default 50).
    pub limit: Option<i64>,
    /// Number of items to skip (default 0).
    pub offset: Option<i64>,
}

impl PaginationParams {
    /// Returns sanitised (limit, offset) values clamped to valid ranges.
    pub fn sanitize(&self) -> (i64, i64) {
        let limit = self
            .limit
            .unwrap_or(DEFAULT_PAGE_LIMIT)
            .clamp(1, MAX_PAGE_LIMIT);
        let offset = self.offset.unwrap_or(0).max(0);
        (limit, offset)
    }
}

/// Paginated response envelope returned by all list endpoints.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T: Serialize> {
    /// The page of items.
    pub items: Vec<T>,
    /// Total number of items matching the query (ignoring limit/offset).
    pub total: i64,
    /// The limit that was applied.
    pub limit: i64,
    /// The offset that was applied.
    pub offset: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub jti: Uuid,
    pub token_type: TokenType,
    pub iss: String,
    pub aud: String,
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

/// Optional body for the refresh endpoint. When the client provides the
/// previous access token, the server revokes it immediately instead of
/// waiting for it to expire naturally (15 min window).
#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    /// The access token that should be revoked as part of the refresh.
    pub access_token: Option<String>,
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
pub struct State {
    pub pool: Pool,
    pub jwtsecret: String,
    pub cache: DashMap<String, CachedUser>,
    /// Revoked token JTIs mapped to their original expiry time.
    /// Entries are evicted by the background cleanup task once expired.
    pub token_blacklist: DashMap<String, DateTime<Utc>>,
    /// Failed login attempt timestamps per email, for account lockout.
    /// Entries older than the lockout window are pruned on each check.
    pub login_attempts: DashMap<String, Vec<DateTime<Utc>>>,
    /// In-memory avatar image cache: avatar_id → (image bytes, content_type).
    /// Loaded on startup from the database; small and static (~2–3 MB total).
    pub avatar_cache: DashMap<Uuid, (Vec<u8>, String)>,
}

#[derive(Serialize, ToSchema)]
pub struct StatusResponse {
    pub up: bool,
    pub setup_required: bool,
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
    pub avatar_id: Option<Uuid>,
    pub created: DateTime<Utc>,
    pub changed: DateTime<Utc>,
}

/// Internal DB row representation of a user (returned by `get_user_by_email`).
/// This struct is NOT used as an API request body — see `UpdateUserRequest` for
/// the API-facing update type. No `Validate` derive: the `password` field contains
/// an Argon2 hash, not plaintext.
#[derive(Deserialize, Clone)]
pub struct UpdateUserEntry {
    pub user_id: Uuid,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
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
///
/// When a non-admin user changes their own password, `current_password` must be
/// provided and will be verified against the stored hash. Admins resetting
/// another user's password may omit `current_password`.
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
    #[validate(
        email,
        length(max = 75, message = "email must not exceed 75 characters")
    )]
    pub email: String,
    #[validate(custom(function = "validate_optional_password"))]
    pub password: Option<String>,
    /// Required when a user changes their own password. Admins resetting
    /// another user's password may omit this field.
    pub current_password: Option<String>,
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
    #[validate(
        email,
        length(max = 75, message = "email must not exceed 75 characters")
    )]
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
    pub joined: DateTime<Utc>,
    pub role_changed: DateTime<Utc>,
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
    #[validate(length(max = 1000, message = "descr must not exceed 1000 characters"))]
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
    #[validate(length(max = 1000, message = "descr must not exceed 1000 characters"))]
    pub descr: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct UserInTeams {
    pub team_id: Uuid,
    pub tname: String,
    pub descr: Option<String>,
    pub title: String,
    pub firstname: String,
    pub lastname: String,
    pub joined: DateTime<Utc>,
    pub role_changed: DateTime<Utc>,
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
    pub teamorders_user_id: Uuid,
    pub pickup_user_id: Option<Uuid>,
    pub duedate: Option<chrono::NaiveDate>,
    pub closed: bool,
    pub created: DateTime<Utc>,
    pub changed: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Clone, Debug, ToSchema)]
pub struct CreateTeamOrderEntry {
    pub duedate: Option<chrono::NaiveDate>,
    /// Optional team member to pick up the order.
    pub pickup_user_id: Option<Uuid>,
}

#[derive(Deserialize, Serialize, Clone, Debug, ToSchema)]
pub struct UpdateTeamOrderEntry {
    /// `None` = field absent (preserve existing value).
    /// `Some(None)` = explicitly set to null (clear the due date).
    /// `Some(Some(date))` = update to the given date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duedate: Option<Option<chrono::NaiveDate>>,
    pub closed: Option<bool>,
    /// `None` = field absent (preserve existing value).
    /// `Some(None)` = explicitly clear the pickup user.
    /// `Some(Some(id))` = assign a new pickup user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pickup_user_id: Option<Option<Uuid>>,
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
    #[validate(range(min = 1, max = 10000, message = "quantity must be between 1 and 10000"))]
    pub amt: i32,
}

#[derive(Deserialize, Serialize, Validate, Clone, Debug, ToSchema)]
pub struct UpdateOrderEntry {
    #[validate(range(min = 1, max = 10000, message = "quantity must be between 1 and 10000"))]
    pub amt: i32,
}

// ── Avatars ─────────────────────────────────────────────────────────────────

/// Avatar list entry (returned by GET /avatars — no binary data).
#[derive(Serialize, ToSchema)]
pub struct AvatarListEntry {
    pub avatar_id: Uuid,
    pub name: String,
}

/// Request body for setting a user's avatar.
#[derive(Deserialize, Serialize, Clone, Debug, Validate, ToSchema)]
pub struct SetAvatarRequest {
    pub avatar_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    // ── validate_optional_password (#172) ────────────────────────────────────

    #[test]
    fn validate_optional_password_rejects_too_short() {
        let err = validate_optional_password(&"short".to_string()).unwrap_err();
        assert_eq!(err.code, "password");
        assert!(
            err.message
                .as_ref()
                .unwrap()
                .contains("at least 8 characters")
        );
    }

    #[test]
    fn validate_optional_password_rejects_too_long() {
        let long = "a".repeat(129);
        let err = validate_optional_password(&long).unwrap_err();
        assert_eq!(err.code, "password");
        assert!(
            err.message
                .as_ref()
                .unwrap()
                .contains("must not exceed 128")
        );
    }

    #[test]
    fn validate_optional_password_accepts_valid() {
        assert!(validate_optional_password(&"validpass".to_string()).is_ok());
    }

    #[test]
    fn validate_optional_password_boundary_min() {
        // Exactly 8 characters — should be accepted
        assert!(validate_optional_password(&"12345678".to_string()).is_ok());
        // 7 characters — rejected
        assert!(validate_optional_password(&"1234567".to_string()).is_err());
    }

    #[test]
    fn validate_optional_password_boundary_max() {
        // Exactly 128 characters — should be accepted
        let max = "a".repeat(128);
        assert!(validate_optional_password(&max).is_ok());
        // 129 characters — rejected
        let over = "a".repeat(129);
        assert!(validate_optional_password(&over).is_err());
    }

    // ── Order model validation (#180) ────────────────────────────────────────

    #[test]
    fn create_order_entry_rejects_zero_quantity() {
        let entry = CreateOrderEntry {
            orders_item_id: Uuid::nil(),
            amt: 0,
        };
        let err = entry.validate().unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("quantity must be between 1 and 10000"));
    }

    #[test]
    fn create_order_entry_rejects_negative_quantity() {
        let entry = CreateOrderEntry {
            orders_item_id: Uuid::nil(),
            amt: -1,
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn create_order_entry_accepts_positive_quantity() {
        let entry = CreateOrderEntry {
            orders_item_id: Uuid::nil(),
            amt: 1,
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn create_order_entry_rejects_exceeding_max_quantity() {
        let entry = CreateOrderEntry {
            orders_item_id: Uuid::nil(),
            amt: 10001,
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn create_order_entry_accepts_max_quantity() {
        let entry = CreateOrderEntry {
            orders_item_id: Uuid::nil(),
            amt: 10000,
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn update_order_entry_rejects_zero_quantity() {
        let entry = UpdateOrderEntry { amt: 0 };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn update_order_entry_accepts_positive_quantity() {
        let entry = UpdateOrderEntry { amt: 3 };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn update_order_entry_rejects_exceeding_max_quantity() {
        let entry = UpdateOrderEntry { amt: 10001 };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn update_order_entry_accepts_max_quantity() {
        let entry = UpdateOrderEntry { amt: 10000 };
        assert!(entry.validate().is_ok());
    }

    // ── validate_non_negative_price (#352) ───────────────────────────────────

    #[test]
    fn validate_non_negative_price_rejects_negative() {
        let price = rust_decimal::Decimal::new(-100, 2); // -1.00
        let err = validate_non_negative_price(&price).unwrap_err();
        assert_eq!(err.code, "price");
        assert!(err.message.as_ref().unwrap().contains("zero or positive"));
    }

    #[test]
    fn validate_non_negative_price_accepts_zero() {
        let price = rust_decimal::Decimal::ZERO;
        assert!(validate_non_negative_price(&price).is_ok());
    }

    #[test]
    fn validate_non_negative_price_accepts_positive() {
        let price = rust_decimal::Decimal::new(999, 2); // 9.99
        assert!(validate_non_negative_price(&price).is_ok());
    }

    // ── CreateUserEntry boundary tests (#353) ────────────────────────────────

    #[test]
    fn create_user_entry_firstname_at_max_50_is_valid() {
        let entry = CreateUserEntry {
            firstname: "a".repeat(50),
            lastname: "Valid".to_string(),
            email: "test@example.com".to_string(),
            password: "validpass123".to_string(),
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn create_user_entry_firstname_at_51_is_rejected() {
        let entry = CreateUserEntry {
            firstname: "a".repeat(51),
            lastname: "Valid".to_string(),
            email: "test@example.com".to_string(),
            password: "validpass123".to_string(),
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn create_user_entry_lastname_at_max_50_is_valid() {
        let entry = CreateUserEntry {
            firstname: "Valid".to_string(),
            lastname: "b".repeat(50),
            email: "test@example.com".to_string(),
            password: "validpass123".to_string(),
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn create_user_entry_lastname_at_51_is_rejected() {
        let entry = CreateUserEntry {
            firstname: "Valid".to_string(),
            lastname: "b".repeat(51),
            email: "test@example.com".to_string(),
            password: "validpass123".to_string(),
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn create_user_entry_firstname_at_min_2_is_valid() {
        let entry = CreateUserEntry {
            firstname: "ab".to_string(),
            lastname: "Valid".to_string(),
            email: "test@example.com".to_string(),
            password: "validpass123".to_string(),
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn create_user_entry_firstname_at_1_is_rejected() {
        let entry = CreateUserEntry {
            firstname: "a".to_string(),
            lastname: "Valid".to_string(),
            email: "test@example.com".to_string(),
            password: "validpass123".to_string(),
        };
        assert!(entry.validate().is_err());
    }

    // ── Team/Role/Item field length boundary tests (#354) ────────────────────

    #[test]
    fn create_team_entry_tname_at_max_255_is_valid() {
        let entry = CreateTeamEntry {
            tname: "t".repeat(255),
            descr: None,
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn create_team_entry_tname_at_256_is_rejected() {
        let entry = CreateTeamEntry {
            tname: "t".repeat(256),
            descr: None,
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn create_team_entry_descr_at_max_1000_is_valid() {
        let entry = CreateTeamEntry {
            tname: "Valid Team".to_string(),
            descr: Some("d".repeat(1000)),
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn create_team_entry_descr_at_1001_is_rejected() {
        let entry = CreateTeamEntry {
            tname: "Valid Team".to_string(),
            descr: Some("d".repeat(1001)),
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn create_team_entry_empty_tname_is_rejected() {
        let entry = CreateTeamEntry {
            tname: String::new(),
            descr: None,
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn update_team_entry_tname_at_max_255_is_valid() {
        let entry = UpdateTeamEntry {
            tname: "t".repeat(255),
            descr: None,
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn update_team_entry_tname_at_256_is_rejected() {
        let entry = UpdateTeamEntry {
            tname: "t".repeat(256),
            descr: None,
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn create_role_entry_title_at_max_255_is_valid() {
        let entry = CreateRoleEntry {
            title: "r".repeat(255),
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn create_role_entry_title_at_256_is_rejected() {
        let entry = CreateRoleEntry {
            title: "r".repeat(256),
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn create_role_entry_empty_title_is_rejected() {
        let entry = CreateRoleEntry {
            title: String::new(),
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn create_item_entry_descr_at_max_255_is_valid() {
        let entry = CreateItemEntry {
            descr: "i".repeat(255),
            price: rust_decimal::Decimal::new(100, 2),
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn create_item_entry_descr_at_256_is_rejected() {
        let entry = CreateItemEntry {
            descr: "i".repeat(256),
            price: rust_decimal::Decimal::new(100, 2),
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn create_item_entry_empty_descr_is_rejected() {
        let entry = CreateItemEntry {
            descr: String::new(),
            price: rust_decimal::Decimal::new(100, 2),
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn update_item_entry_descr_at_max_255_is_valid() {
        let entry = UpdateItemEntry {
            descr: "i".repeat(255),
            price: rust_decimal::Decimal::new(100, 2),
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn update_item_entry_descr_at_256_is_rejected() {
        let entry = UpdateItemEntry {
            descr: "i".repeat(256),
            price: rust_decimal::Decimal::new(100, 2),
        };
        assert!(entry.validate().is_err());
    }

    // ── Order entry amt boundary tests (#396) ────────────────────────────────

    #[test]
    fn create_order_entry_boundary_min_1_is_valid() {
        let entry = CreateOrderEntry {
            orders_item_id: Uuid::nil(),
            amt: 1,
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn update_order_entry_boundary_min_1_is_valid() {
        let entry = UpdateOrderEntry { amt: 1 };
        assert!(entry.validate().is_ok());
    }
}
