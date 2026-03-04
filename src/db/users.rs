use crate::errors::Error;
use crate::from_row::{FromRow, map_rows};
use crate::models::*;
use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
use deadpool_postgres::Client;
use uuid::Uuid;

/// Fetches all users, ordered by first name then last name.
///
/// Rows that fail to map are logged with `warn!()` and skipped.
pub async fn get_users(client: &Client) -> Result<Vec<UserEntry>, Error> {
    let statement = client
        .prepare("select user_id, firstname, lastname, email, created, changed from users order by firstname asc, lastname asc")
        .await
        .map_err(Error::Db)?;

    let rows = client
        .query(&statement, &[])
        .await
        .map_err(Error::Db)?;

    Ok(map_rows(&rows, "user"))
}

/// Fetches a single user by ID.
///
/// Returns `Error::NotFound` if no user exists with the given ID.
pub async fn get_user(client: &Client, user_id: Uuid) -> Result<UserEntry, Error> {
    let statement = client
        .prepare("select user_id, firstname, lastname, email, created, changed from users where user_id = $1 limit 1")
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
               returning user_id, firstname, lastname, email, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let salt = SaltString::generate(&mut OsRng);
    let hash = crate::argon2_hasher()
        .hash_password(user.password.as_bytes(), &salt)
        .map_err(|err| Error::Argon2(err.to_string()))?
        .to_string();
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
                       returning user_id, firstname, lastname, email, created, changed
                    "#,
                )
                .await
                .map_err(Error::Db)?;

            let salt = SaltString::generate(&mut OsRng);
            let hash = crate::argon2_hasher()
                .hash_password(password.as_bytes(), &salt)
                .map_err(|err| Error::Argon2(err.to_string()))?
                .to_string();

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
                       returning user_id, firstname, lastname, email, created, changed
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
