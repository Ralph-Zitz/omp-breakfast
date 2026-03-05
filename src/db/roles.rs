use crate::errors::Error;
use crate::from_row::{FromRow, map_rows};
use crate::models::*;
use deadpool_postgres::Client;
use uuid::Uuid;

/// Fetches roles with pagination, ordered alphabetically by title.
///
/// Rows that fail to map are logged with `warn!()` and skipped.
pub async fn get_roles(client: &Client, limit: i64, offset: i64) -> Result<(Vec<RoleEntry>, i64), Error> {
    let statement = client
        .prepare("select role_id, title, created, changed, count(*) over() as total_count from roles order by title asc limit $1 offset $2")
        .await
        .map_err(Error::Db)?;

    let rows = client
        .query(&statement, &[&limit, &offset])
        .await
        .map_err(Error::Db)?;

    let total: i64 = rows.first().map(|r| r.get("total_count")).unwrap_or(0);
    Ok((map_rows(&rows, "role"), total))
}

/// Fetches a single role by ID.
///
/// Returns `Error::NotFound` if no role exists with the given ID.
pub async fn get_role(client: &Client, role_id: Uuid) -> Result<RoleEntry, Error> {
    let statement = client
        .prepare("select role_id, title, created, changed from roles where role_id = $1 limit 1")
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(&statement, &[&role_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Role not found".to_string()))
        .map(RoleEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Creates a new role and returns the created entry.
pub async fn create_role(client: &Client, role: CreateRoleEntry) -> Result<RoleEntry, Error> {
    let statement = client
        .prepare("insert into roles (title) values ($1) returning role_id, title, created, changed")
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&role.title])
        .await
        .map(RoleEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Deletes a role by ID. Returns `true` if a row was deleted, `false` if
/// the role did not exist.
pub async fn delete_role(client: &Client, rid: Uuid) -> Result<bool, Error> {
    let statement = client
        .prepare("delete from roles where role_id = $1")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&rid])
        .await
        .map_err(Error::Db)?;

    Ok(result == 1)
}

/// Updates a role's title.
///
/// Uses `query_opt` + 404 to avoid returning 500 for missing roles.
pub async fn update_role(
    client: &Client,
    rid: Uuid,
    role: UpdateRoleEntry,
) -> Result<RoleEntry, Error> {
    let statement = client
        .prepare(
            r#"
               update roles set title = $1
               where role_id = $2
               returning role_id, title, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(&statement, &[&role.title, &rid])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Role not found".to_string()))
        .map(RoleEntry::from_row)?
        .map_err(Error::DbMapper)
}
