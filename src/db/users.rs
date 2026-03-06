use crate::errors::Error;
use crate::from_row::{FromRow, map_rows};
use crate::models::*;
use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
use deadpool_postgres::Client;
use uuid::Uuid;

/// Fetches users with pagination, ordered by first name then last name.
///
/// Returns the page of results and the total count (for pagination metadata).
/// Rows that fail to map are logged with `warn!()` and skipped.
pub async fn get_users(
    client: &Client,
    limit: i64,
    offset: i64,
) -> Result<(Vec<UserEntry>, i64), Error> {
    let statement = client
        .prepare("select user_id, firstname, lastname, email, avatar_id, created, changed, count(*) over() as total_count from users order by firstname asc, lastname asc limit $1 offset $2")
        .await
        .map_err(Error::Db)?;

    let rows = client
        .query(&statement, &[&limit, &offset])
        .await
        .map_err(Error::Db)?;

    let total: i64 = rows.first().map(|r| r.get("total_count")).unwrap_or(0);
    Ok((map_rows(&rows, "user"), total))
}

/// Fetches a single user by ID.
///
/// Returns `Error::NotFound` if no user exists with the given ID.
pub async fn get_user(client: &Client, user_id: Uuid) -> Result<UserEntry, Error> {
    let statement = client
        .prepare("select user_id, firstname, lastname, email, avatar_id, created, changed from users where user_id = $1 limit 1")
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(&statement, &[&user_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("User not found".to_string()))
        .map(UserEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Looks up a user by email address, returning an [`UpdateUserEntry`] that
/// includes the password hash (for auth cache verification).
///
/// Returns `Error::NotFound` if no user exists with the given email.
pub async fn get_user_by_email(client: &Client, email: &str) -> Result<UpdateUserEntry, Error> {
    let statement = client
        .prepare("select user_id, firstname, lastname, email, password from users where email = $1 limit 1")
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(&statement, &[&email])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("User not found".to_string()))
        .map(UpdateUserEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Fetches the password hash for a user by ID.
///
/// Returns `Error::NotFound` if no user exists with the given ID.
pub async fn get_password_hash(client: &Client, user_id: Uuid) -> Result<String, Error> {
    let statement = client
        .prepare("select password from users where user_id = $1 limit 1")
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_opt(&statement, &[&user_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("User not found".to_string()))?;

    row.try_get::<_, String>(0).map_err(Error::Db)
}

/// Creates a new user, hashing the plaintext password with Argon2id before
/// storing it.
///
/// Returns the created user (without password).
pub async fn create_user(client: &Client, user: CreateUserEntry) -> Result<UserEntry, Error> {
    let statement = client
        .prepare(
            r#"
               insert into users (firstname, lastname, email, password)
               values ($1, $2, $3, $4)
               returning user_id, firstname, lastname, email, avatar_id, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let password = user.password.clone();
    let hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        crate::argon2_hasher()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await
    .map_err(|e| Error::Argon2(e.to_string()))?
    .map_err(|err| Error::Argon2(err.to_string()))?;
    client
        .query_one(
            &statement,
            &[&user.firstname, &user.lastname, &user.email, &hash],
        )
        .await
        .map(UserEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Updates a user's profile fields. If `password` is `Some`, the new password
/// is hashed with Argon2id before storing; otherwise the existing hash is
/// preserved.
///
/// Uses `query_opt` + 404 to avoid returning 500 for missing users.
pub async fn update_user(
    client: &Client,
    uid: Uuid,
    user: UpdateUserRequest,
) -> Result<UserEntry, Error> {
    match &user.password {
        Some(password) => {
            // Password provided — hash and update all fields
            let statement = client
                .prepare(
                    r#"
                       update users set firstname = $1, lastname = $2, email = $3, password = $4
                       where user_id = $5
                       returning user_id, firstname, lastname, email, avatar_id, created, changed
                    "#,
                )
                .await
                .map_err(Error::Db)?;

            let pw = password.clone();
            let hash = tokio::task::spawn_blocking(move || {
                let salt = SaltString::generate(&mut OsRng);
                crate::argon2_hasher()
                    .hash_password(pw.as_bytes(), &salt)
                    .map(|h| h.to_string())
            })
            .await
            .map_err(|e| Error::Argon2(e.to_string()))?
            .map_err(|err| Error::Argon2(err.to_string()))?;

            client
                .query_opt(
                    &statement,
                    &[&user.firstname, &user.lastname, &user.email, &hash, &uid],
                )
                .await
                .map_err(Error::Db)?
                .ok_or_else(|| Error::NotFound("User not found".to_string()))
                .map(UserEntry::from_row)?
                .map_err(Error::DbMapper)
        }
        None => {
            // No password — update only profile fields, preserve existing password
            let statement = client
                .prepare(
                    r#"
                       update users set firstname = $1, lastname = $2, email = $3
                       where user_id = $4
                       returning user_id, firstname, lastname, email, avatar_id, created, changed
                    "#,
                )
                .await
                .map_err(Error::Db)?;

            client
                .query_opt(
                    &statement,
                    &[&user.firstname, &user.lastname, &user.email, &uid],
                )
                .await
                .map_err(Error::Db)?
                .ok_or_else(|| Error::NotFound("User not found".to_string()))
                .map(UserEntry::from_row)?
                .map_err(Error::DbMapper)
        }
    }
}

/// Deletes a user by ID. Returns `true` if a row was deleted, `false` if
/// the user did not exist.
pub async fn delete_user(client: &Client, uid: Uuid) -> Result<bool, Error> {
    let statement = client
        .prepare("delete from users where user_id = $1")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&uid])
        .await
        .map_err(Error::Db)?;

    Ok(result == 1)
}

/// Returns the total number of users in the database.
pub async fn count_users(client: &Client) -> Result<i64, Error> {
    let statement = client
        .prepare("select count(*) as cnt from users")
        .await
        .map_err(Error::Db)?;

    let row = client.query_one(&statement, &[]).await.map_err(Error::Db)?;

    Ok(row.get("cnt"))
}

/// Deletes a user by email address. Returns `true` if a row was deleted,
/// `false` if no user matched.
pub async fn delete_user_by_email(client: &Client, email: &str) -> Result<bool, Error> {
    let statement = client
        .prepare("delete from users where email = $1")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&email])
        .await
        .map_err(Error::Db)?;

    Ok(result == 1)
}
