use crate::errors::Error;
use crate::from_row::{FromRow, map_rows};
use crate::models::*;
use deadpool_postgres::Client;
use uuid::Uuid;

/// Lists all available avatars (id + name, no binary data).
pub async fn get_avatars(client: &Client) -> Result<Vec<AvatarListEntry>, Error> {
    let statement = client
        .prepare("select avatar_id, name from avatars order by name asc")
        .await
        .map_err(Error::Db)?;

    let rows = client.query(&statement, &[]).await.map_err(Error::Db)?;

    Ok(map_rows(&rows, "avatar"))
}

/// Fetches a single avatar's binary data and content type by ID.
pub async fn get_avatar(client: &Client, avatar_id: Uuid) -> Result<(Vec<u8>, String), Error> {
    let statement = client
        .prepare("select data, content_type from avatars where avatar_id = $1 limit 1")
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_opt(&statement, &[&avatar_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Avatar not found".to_string()))?;

    let data: Vec<u8> = row.try_get("data").map_err(Error::Db)?;
    let content_type: String = row.try_get("content_type").map_err(Error::Db)?;
    Ok((data, content_type))
}

/// Inserts a new avatar into the database with an explicit UUID.
/// Used by the startup seed task.
pub async fn insert_avatar(
    client: &Client,
    avatar_id: Uuid,
    name: &str,
    data: &[u8],
    content_type: &str,
) -> Result<(), Error> {
    let statement = client
        .prepare(
            "insert into avatars (avatar_id, name, data, content_type) values ($1, $2, $3, $4) on conflict (name) do nothing",
        )
        .await
        .map_err(Error::Db)?;

    client
        .execute(&statement, &[&avatar_id, &name, &data, &content_type])
        .await
        .map_err(Error::Db)?;
    Ok(())
}

/// Returns the number of avatars in the database.
pub async fn count_avatars(client: &Client) -> Result<i64, Error> {
    let row = client
        .query_one("select count(*) as cnt from avatars", &[])
        .await
        .map_err(Error::Db)?;

    row.try_get::<_, i64>("cnt").map_err(Error::Db)
}

/// Sets or clears a user's avatar. Pass `None` to remove the avatar.
pub async fn set_user_avatar(
    client: &Client,
    user_id: Uuid,
    avatar_id: Option<Uuid>,
) -> Result<UserEntry, Error> {
    let statement = client
        .prepare(
            r#"
               update users set avatar_id = $1
               where user_id = $2
               returning user_id, firstname, lastname, email, avatar_id, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(&statement, &[&avatar_id, &user_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("User not found".to_string()))
        .map(UserEntry::from_row)?
        .map_err(Error::DbMapper)
}
