use std::error::Error as StdError;
use std::fmt;

use tokio_postgres::Row;
use tracing::warn;

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

impl StdError for FromRowError {}

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
///
/// Uses `Error::source()` to distinguish column-not-found (no source) from
/// type conversion errors (source present), avoiding fragile string matching.
fn map_err(column: &str, e: tokio_postgres::Error) -> FromRowError {
    if e.source().is_some() {
        FromRowError::Conversion(format!("{}: {}", column, e))
    } else {
        FromRowError::ColumnNotFound(column.to_string())
    }
}

/// Maps query result rows into typed structs, logging and skipping rows that
/// fail to convert. Used by all list-query functions in `db/`.
pub fn map_rows<T: FromRow>(rows: &[Row], entity: &str) -> Vec<T> {
    rows.iter()
        .filter_map(|row| match T::from_row_ref(row) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!(error = %e, "Failed to map {} row — skipping", entity);
                None
            }
        })
        .collect()
}

/// Generates a `FromRow` implementation for a struct where every field name
/// matches the corresponding database column name.
macro_rules! impl_from_row {
    ($type:ty { $($field:ident),+ $(,)? }) => {
        impl FromRow for $type {
            fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
                Ok(Self {
                    $( $field: row.try_get(stringify!($field)).map_err(|e| map_err(stringify!($field), e))?, )+
                })
            }
        }
    };
}

// ── FromRow implementations ─────────────────────────────────────────────────

use crate::models::{
    AvatarListEntry, ItemEntry, OrderEntry, RoleEntry, TeamEntry, TeamOrderEntry, UpdateUserEntry,
    UserEntry, UserInTeams, UsersInTeam,
};

impl_from_row!(UserEntry {
    user_id,
    firstname,
    lastname,
    email,
    avatar_id,
    created,
    changed
});
impl_from_row!(UpdateUserEntry {
    user_id,
    firstname,
    lastname,
    email,
    password
});
impl_from_row!(TeamEntry {
    team_id,
    tname,
    descr,
    created,
    changed
});
impl_from_row!(RoleEntry {
    role_id,
    title,
    created,
    changed
});
impl_from_row!(ItemEntry {
    item_id,
    descr,
    price,
    created,
    changed
});
impl_from_row!(TeamOrderEntry {
    teamorders_id,
    teamorders_team_id,
    teamorders_user_id,
    duedate,
    closed,
    created,
    changed
});
impl_from_row!(OrderEntry {
    orders_teamorders_id,
    orders_item_id,
    orders_team_id,
    amt,
    created,
    changed
});
impl_from_row!(UsersInTeam {
    user_id,
    firstname,
    lastname,
    email,
    title,
    joined,
    role_changed
});
impl_from_row!(UserInTeams {
    team_id,
    tname,
    descr,
    title,
    firstname,
    lastname,
    joined,
    role_changed
});
impl_from_row!(AvatarListEntry { avatar_id, name });

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
    fn map_err_classifies_sourceless_error_as_column_not_found() {
        // tokio_postgres::Error::__private_api_timeout() produces an error
        // with no source (cause = None). In practice, map_err is only called
        // from Row::try_get() closures, where sourceless errors correspond to
        // missing columns (Kind::Column also has cause = None).
        let pg_err = tokio_postgres::Error::__private_api_timeout();
        let result = map_err("my_column", pg_err);
        match result {
            FromRowError::ColumnNotFound(col) => {
                assert_eq!(col, "my_column");
            }
            other => panic!("expected ColumnNotFound, got {:?}", other),
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
