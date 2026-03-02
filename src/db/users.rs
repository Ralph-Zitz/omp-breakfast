use crate::errors::Error;
use crate::from_row::FromRow;
use crate::models::*;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use deadpool_postgres::Client;
use tracing::warn;
use uuid::Uuid;

pub async fn get_users(client: &Client) -> Result<Vec<UserEntry>, Error> {
    let statement = client
        .prepare("select user_id, firstname, lastname, email, created, changed from users order by firstname asc, lastname asc")
        .await
        .map_err(Error::Db)?;

    let users = client
        .query(&statement, &[])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| match UserEntry::from_row_ref(row) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!(error = %e, "Failed to map user row — skipping");
                None
            }
        })
        .collect();

    Ok(users)
}

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
    let hash = Argon2::default()
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
            let hash = Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map_err(|err| Error::Argon2(err.to_string()))?
                .to_string();

            client
                .query_one(
                    &statement,
                    &[&user.firstname, &user.lastname, &user.email, &hash, &uid],
                )
                .await
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
                .query_one(
                    &statement,
                    &[&user.firstname, &user.lastname, &user.email, &uid],
                )
                .await
                .map(UserEntry::from_row)?
                .map_err(Error::DbMapper)
        }
    }
}

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
