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
    fn from_row(row: Row) -> Result<Self, FromRowError>;
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
    CreateItemEntry, CreateRoleEntry, CreateTeamEntry, CreateUserEntry, ItemEntry, OrderEntry,
    RoleEntry, TeamEntry, TeamOrderEntry, UpdateItemEntry, UpdateRoleEntry, UpdateTeamEntry,
    UpdateUserEntry, UserEntry,
};

impl FromRow for UserEntry {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

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
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

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

// ── CreateUserEntry ─────────────────────────────────────────────────────────

impl FromRow for CreateUserEntry {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
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
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

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

// ── CreateTeamEntry ─────────────────────────────────────────────────────────

impl FromRow for CreateTeamEntry {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            tname: row.try_get("tname").map_err(|e| map_err("tname", e))?,
            descr: row.try_get("descr").map_err(|e| map_err("descr", e))?,
        })
    }
}

// ── UpdateTeamEntry ─────────────────────────────────────────────────────────

impl FromRow for UpdateTeamEntry {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            tname: row.try_get("tname").map_err(|e| map_err("tname", e))?,
            descr: row.try_get("descr").map_err(|e| map_err("descr", e))?,
        })
    }
}

// ── RoleEntry ───────────────────────────────────────────────────────────────

impl FromRow for RoleEntry {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            role_id: row.try_get("role_id").map_err(|e| map_err("role_id", e))?,
            title: row.try_get("title").map_err(|e| map_err("title", e))?,
            created: row.try_get("created").map_err(|e| map_err("created", e))?,
            changed: row.try_get("changed").map_err(|e| map_err("changed", e))?,
        })
    }
}

// ── CreateRoleEntry ─────────────────────────────────────────────────────────

impl FromRow for CreateRoleEntry {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            title: row.try_get("title").map_err(|e| map_err("title", e))?,
        })
    }
}

// ── UpdateRoleEntry ─────────────────────────────────────────────────────────

impl FromRow for UpdateRoleEntry {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            title: row.try_get("title").map_err(|e| map_err("title", e))?,
        })
    }
}

// ── ItemEntry ───────────────────────────────────────────────────────────────

impl FromRow for ItemEntry {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

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

// ── CreateItemEntry ─────────────────────────────────────────────────────────

impl FromRow for CreateItemEntry {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            descr: row.try_get("descr").map_err(|e| map_err("descr", e))?,
            price: row.try_get("price").map_err(|e| map_err("price", e))?,
        })
    }
}

// ── UpdateItemEntry ─────────────────────────────────────────────────────────

impl FromRow for UpdateItemEntry {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

    fn from_row_ref(row: &Row) -> Result<Self, FromRowError> {
        Ok(Self {
            descr: row.try_get("descr").map_err(|e| map_err("descr", e))?,
            price: row.try_get("price").map_err(|e| map_err("price", e))?,
        })
    }
}

// ── TeamOrderEntry ──────────────────────────────────────────────────────────

impl FromRow for TeamOrderEntry {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

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
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        Self::from_row_ref(&row)
    }

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
        })
    }
}
