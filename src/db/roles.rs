use crate::errors::Error;
use crate::from_row::FromRow;
use crate::models::*;
use deadpool_postgres::Client;
use tracing::warn;
use uuid::Uuid;

pub async fn get_roles(client: &Client) -> Result<Vec<RoleEntry>, Error> {
    let statement = client
        .prepare("select role_id, title, created, changed from roles order by title asc")
        .await
        .map_err(Error::Db)?;

    let roles = client
        .query(&statement, &[])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| match RoleEntry::from_row_ref(row) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!(error = %e, "Failed to map role row — skipping");
                None
            }
        })
        .collect();

    Ok(roles)
}

pub async fn get_role(client: &Client, role_id: Uuid) -> Result<RoleEntry, Error> {
    let statement = client
        .prepare("select role_id, title, created, changed from roles where role_id = $1 limit 1")
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&role_id])
        .await
        .map(RoleEntry::from_row)?
        .map_err(Error::DbMapper)
}

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
        .query_one(&statement, &[&role.title, &rid])
        .await
        .map(RoleEntry::from_row)?
        .map_err(Error::DbMapper)
}
