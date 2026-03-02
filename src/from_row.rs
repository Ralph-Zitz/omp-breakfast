use std::fmt;

use tokio_postgres::Row;

/// Error type for row-to-struct mapping, replacing `tokio_pg_mapper::Error`.
#[derive(Debug)]
pub enum FromRowError {
    /// A column was not found in the row.
    ColumnNotFound(String),
    /// A type conversion failed when reading a column value.
    Conversion(String),
}

impl fmt::Display for FromRowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FromRowError::ColumnNotFound(col) => write!(f, "Column not found: {}", col),
            FromRowError::Conversion(msg) => write!(f, "Conversion error: {}", msg),
        }
    }
}

impl std::error::Error for FromRowError {}

/// Trait for converting a `tokio_postgres::Row` into a typed struct.
pub trait FromRow: Sized {
    /// Convert an owned row into the target type.
    /// Default implementation delegates to `from_row_ref`.
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }
    /// Convert a borrowed row into the target type.
    fn from_row_ref(row: &Row) -> Result<Self, FromRowError>;
}

/// Helper to convert a `tokio_postgres` column-get error into a `FromRowError`.
fn map_err(column: &str, e: tokio_postgres::Error) -> FromRowError {
    let msg = e.to_string();
    if msg.contains("column") || msg.contains("not found") {
        FromRowError::ColumnNotFound(column.to_string())
    } else {
        FromRowError::Conversion(format!("{}: {}", column, msg))
    }
}

// ── UserEntry ───────────────────────────────────────────────────────────────

use crate::models::{
    ItemEntry, OrderEntry, RoleEntry, TeamEntry, TeamOrderEntry, UpdateUserEntry, UserEntry,
    UserInTeams, UsersInTeam,
};

impl FromRow for UserEntry {
    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            user_id: row.try_get("user_id").map_err(|e| map_err("user_id", e))?,
            firstname: row
                .try_get("firstname")
                .map_err(|e| map_err("firstname", e))?,
            lastname: row
                .try_get("lastname")
                .map_err(|e| map_err("lastname", e))?,
            email: row.try_get("email").map_err(|e| map_err("email", e))?,
            created: row.try_get("created").map_err(|e| map_err("created", e))?,
            changed: row.try_get("changed").map_err(|e| map_err("changed", e))?,
        })
    }
}

// ── UpdateUserEntry ─────────────────────────────────────────────────────────

impl FromRow for UpdateUserEntry {
    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            user_id: row.try_get("user_id").map_err(|e| map_err("user_id", e))?,
            firstname: row
                .try_get("firstname")
                .map_err(|e| map_err("firstname", e))?,
            lastname: row
                .try_get("lastname")
                .map_err(|e| map_err("lastname", e))?,
            email: row.try_get("email").map_err(|e| map_err("email", e))?,
            password: row
                .try_get("password")
                .map_err(|e| map_err("password", e))?,
        })
    }
}

// ── TeamEntry ───────────────────────────────────────────────────────────────

impl FromRow for TeamEntry {
    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            team_id: row.try_get("team_id").map_err(|e| map_err("team_id", e))?,
            tname: row.try_get("tname").map_err(|e| map_err("tname", e))?,
            descr: row.try_get("descr").map_err(|e| map_err("descr", e))?,
            created: row.try_get("created").map_err(|e| map_err("created", e))?,
            changed: row.try_get("changed").map_err(|e| map_err("changed", e))?,
        })
    }
}

// ── RoleEntry ───────────────────────────────────────────────────────────────

impl FromRow for RoleEntry {
    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            role_id: row.try_get("role_id").map_err(|e| map_err("role_id", e))?,
            title: row.try_get("title").map_err(|e| map_err("title", e))?,
            created: row.try_get("created").map_err(|e| map_err("created", e))?,
            changed: row.try_get("changed").map_err(|e| map_err("changed", e))?,
        })
    }
}

// ── ItemEntry ───────────────────────────────────────────────────────────────

impl FromRow for ItemEntry {
    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            item_id: row.try_get("item_id").map_err(|e| map_err("item_id", e))?,
            descr: row.try_get("descr").map_err(|e| map_err("descr", e))?,
            price: row.try_get("price").map_err(|e| map_err("price", e))?,
            created: row.try_get("created").map_err(|e| map_err("created", e))?,
            changed: row.try_get("changed").map_err(|e| map_err("changed", e))?,
        })
    }
}

// ── TeamOrderEntry ──────────────────────────────────────────────────────────

impl FromRow for TeamOrderEntry {
    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            teamorders_id: row
                .try_get("teamorders_id")
                .map_err(|e| map_err("teamorders_id", e))?,
            teamorders_team_id: row
                .try_get("teamorders_team_id")
                .map_err(|e| map_err("teamorders_team_id", e))?,
            teamorders_user_id: row
                .try_get("teamorders_user_id")
                .map_err(|e| map_err("teamorders_user_id", e))?,
            duedate: row.try_get("duedate").map_err(|e| map_err("duedate", e))?,
            closed: row.try_get("closed").map_err(|e| map_err("closed", e))?,
            created: row.try_get("created").map_err(|e| map_err("created", e))?,
            changed: row.try_get("changed").map_err(|e| map_err("changed", e))?,
        })
    }
}

// ── OrderEntry ──────────────────────────────────────────────────────────────

impl FromRow for OrderEntry {
    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            orders_teamorders_id: row
                .try_get("orders_teamorders_id")
                .map_err(|e| map_err("orders_teamorders_id", e))?,
            orders_item_id: row
                .try_get("orders_item_id")
                .map_err(|e| map_err("orders_item_id", e))?,
            orders_team_id: row
                .try_get("orders_team_id")
                .map_err(|e| map_err("orders_team_id", e))?,
            amt: row.try_get("amt").map_err(|e| map_err("amt", e))?,
            created: row.try_get("created").map_err(|e| map_err("created", e))?,
            changed: row.try_get("changed").map_err(|e| map_err("changed", e))?,
        })
    }
}

// ── UsersInTeam ─────────────────────────────────────────────────────────────

impl FromRow for UsersInTeam {
    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            user_id: row.try_get("user_id").map_err(|e| map_err("user_id", e))?,
            firstname: row
                .try_get("firstname")
                .map_err(|e| map_err("firstname", e))?,
            lastname: row
                .try_get("lastname")
                .map_err(|e| map_err("lastname", e))?,
            email: row.try_get("email").map_err(|e| map_err("email", e))?,
            title: row.try_get("title").map_err(|e| map_err("title", e))?,
        })
    }
}

// ── UserInTeams ─────────────────────────────────────────────────────────────

impl FromRow for UserInTeams {
    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            tname: row.try_get("tname").map_err(|e| map_err("tname", e))?,
            title: row.try_get("title").map_err(|e| map_err("title", e))?,
            firstname: row
                .try_get("firstname")
                .map_err(|e| map_err("firstname", e))?,
            lastname: row
                .try_get("lastname")
                .map_err(|e| map_err("lastname", e))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FromRowError Display ────────────────────────────────────────────

    #[test]
    fn column_not_found_error_displays_column_name() {
        let err = FromRowError::ColumnNotFound("email".to_string());
        assert_eq!(format!("{}", err), "Column not found: email");
    }

    #[test]
    fn conversion_error_displays_message() {
        let err = FromRowError::Conversion("user_id: invalid type".to_string());
        assert_eq!(
            format!("{}", err),
            "Conversion error: user_id: invalid type"
        );
    }

    #[test]
    fn from_row_error_implements_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(FromRowError::ColumnNotFound("x".to_string()));
        // Should be usable as a trait object
        assert!(err.to_string().contains("Column not found"));
    }

    #[test]
    fn from_row_error_debug_format() {
        let err = FromRowError::ColumnNotFound("team_id".to_string());
        let debug = format!("{:?}", err);
        assert!(
            debug.contains("ColumnNotFound"),
            "debug format should contain variant name"
        );
        assert!(
            debug.contains("team_id"),
            "debug format should contain column name"
        );
    }

    // ── map_err helper ──────────────────────────────────────────────────

    #[test]
    fn map_err_returns_conversion_for_non_column_errors() {
        // tokio_postgres::Error::__private_api_timeout() produces an error
        // whose message does NOT contain "column" or "not found", so map_err
        // should classify it as a Conversion error.
        let pg_err = tokio_postgres::Error::__private_api_timeout();
        let result = map_err("my_column", pg_err);
        match result {
            FromRowError::Conversion(msg) => {
                assert!(
                    msg.starts_with("my_column:"),
                    "Conversion message should start with the column name, got: {}",
                    msg
                );
            }
            other => panic!("expected Conversion, got {:?}", other),
        }
    }

    #[test]
    fn map_err_column_not_found_variant_is_constructable() {
        // We can't easily create a tokio_postgres error whose message
        // contains "column" or "not found" without a live Row, so we
        // verify the ColumnNotFound variant directly.
        let err = FromRowError::ColumnNotFound("missing_col".to_string());
        assert_eq!(format!("{}", err), "Column not found: missing_col");
    }

    #[test]
    fn map_err_conversion_variant_includes_column_and_detail() {
        let err = FromRowError::Conversion("price: expected numeric, got text".to_string());
        let display = format!("{}", err);
        assert!(display.contains("price"));
        assert!(display.contains("numeric"));
    }

    // ── FromRow trait: from_row delegates to from_row_ref ───────────────

    // We can't easily construct tokio_postgres::Row in unit tests (it requires
    // internal binary protocol state), so the FromRow implementations are
    // validated via DB integration tests. Here we verify the error types and
    // the public trait contract at the type level.

    #[test]
    fn from_row_trait_is_object_safe_for_user_entry() {
        // Verify that FromRow is implemented for UserEntry by checking
        // the associated function signatures exist at compile time.
        fn _assert_from_row<T: FromRow>() {}
        _assert_from_row::<UserEntry>();
        _assert_from_row::<UpdateUserEntry>();
    }

    #[test]
    fn from_row_trait_is_implemented_for_all_entry_types() {
        fn _assert_from_row<T: FromRow>() {}
        _assert_from_row::<TeamEntry>();
        _assert_from_row::<RoleEntry>();
        _assert_from_row::<ItemEntry>();
        _assert_from_row::<TeamOrderEntry>();
        _assert_from_row::<OrderEntry>();
        _assert_from_row::<UsersInTeam>();
        _assert_from_row::<UserInTeams>();
    }

    // ── FromRowError variants are distinct ──────────────────────────────

    #[test]
    fn column_not_found_and_conversion_are_distinct_variants() {
        let err1 = FromRowError::ColumnNotFound("col".to_string());
        let err2 = FromRowError::Conversion("col".to_string());
        // They should produce different Display output
        assert_ne!(format!("{}", err1), format!("{}", err2));
    }
}
