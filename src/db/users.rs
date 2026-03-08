use crate::errors::Error;
use crate::from_row::{FromRow, map_rows};
use crate::middleware::auth::ROLE_ADMIN;
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
/// The lookup is case-insensitive (email is lowercased before querying).
/// Returns `Error::NotFound` if no user exists with the given email.
pub async fn get_user_by_email(client: &Client, email: &str) -> Result<UpdateUserEntry, Error> {
    let email_lower = email.to_lowercase();
    let statement = client
        .prepare("select user_id, firstname, lastname, email, password from users where email = $1 limit 1")
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(&statement, &[&email_lower])
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
    let email_lower = user.email.to_lowercase();
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
            &[&user.firstname, &user.lastname, &email_lower, &hash],
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
    let email_lower = user.email.to_lowercase();
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
                    &[&user.firstname, &user.lastname, &email_lower, &hash, &uid],
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
                    &[&user.firstname, &user.lastname, &email_lower, &uid],
                )
                .await
                .map_err(Error::Db)?
                .ok_or_else(|| Error::NotFound("User not found".to_string()))
                .map(UserEntry::from_row)?
                .map_err(Error::DbMapper)
        }
    }
}

/// Deletes a user by ID within a transaction. Removes all team
/// memberships first (memberof FK is ON DELETE RESTRICT), then
/// deletes the user row. Returns `true` if a row was deleted,
/// `false` if the user did not exist.
pub async fn delete_user(client: &mut Client, uid: Uuid) -> Result<bool, Error> {
    let tx = client.transaction().await.map_err(Error::Db)?;

    let del_memberships = tx
        .prepare("delete from memberof where memberof_user_id = $1")
        .await
        .map_err(Error::Db)?;
    tx.execute(&del_memberships, &[&uid])
        .await
        .map_err(Error::Db)?;

    let del_user = tx
        .prepare("delete from users where user_id = $1")
        .await
        .map_err(Error::Db)?;
    let result = tx.execute(&del_user, &[&uid]).await.map_err(Error::Db)?;

    tx.commit().await.map_err(Error::Db)?;

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

/// Deletes a user by email address within a transaction. Removes all
/// team memberships first (memberof FK is ON DELETE RESTRICT), then
/// deletes the user row. Returns `true` if a row was deleted, `false`
/// if no user matched.
pub async fn delete_user_by_email(client: &mut Client, email: &str) -> Result<bool, Error> {
    let email_lower = email.to_lowercase();
    let tx = client.transaction().await.map_err(Error::Db)?;

    // Look up user_id for membership cleanup
    let lookup = tx
        .prepare("select user_id from users where email = $1")
        .await
        .map_err(Error::Db)?;
    let user_row = tx
        .query_opt(&lookup, &[&email_lower])
        .await
        .map_err(Error::Db)?;

    let Some(row) = user_row else {
        tx.commit().await.map_err(Error::Db)?;
        return Ok(false);
    };
    let uid: Uuid = row.get(0);

    let del_memberships = tx
        .prepare("delete from memberof where memberof_user_id = $1")
        .await
        .map_err(Error::Db)?;
    tx.execute(&del_memberships, &[&uid])
        .await
        .map_err(Error::Db)?;

    let del_user = tx
        .prepare("delete from users where user_id = $1")
        .await
        .map_err(Error::Db)?;
    let result = tx.execute(&del_user, &[&uid]).await.map_err(Error::Db)?;

    tx.commit().await.map_err(Error::Db)?;

    Ok(result == 1)
}

/// Bootstraps the first user in the system within a single transaction.
///
/// 1. Verifies no users exist (returns `Error::Forbidden` if any do)
/// 2. Creates the user with Argon2id-hashed password
/// 3. Seeds the four default roles
/// 4. Creates a "Default" bootstrap team
/// 5. Assigns the new user as Admin in the bootstrap team
///
/// All steps run inside one transaction — a crash mid-sequence rolls back
/// to a clean state, allowing re-registration.
pub async fn bootstrap_first_user(
    client: &mut Client,
    user: CreateUserEntry,
) -> Result<UserEntry, Error> {
    // Hash password outside the transaction (CPU-bound work)
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

    let tx = client.transaction().await.map_err(Error::Db)?;

    // Acquire a transaction-scoped advisory lock to prevent concurrent
    // bootstrap attempts. Released automatically when the transaction ends.
    tx.execute("SELECT pg_advisory_xact_lock(0)", &[])
        .await
        .map_err(Error::Db)?;

    // 1. Check that no users exist
    let count_row = tx
        .query_one("select count(*) as cnt from users", &[])
        .await
        .map_err(Error::Db)?;
    let user_count: i64 = count_row.get("cnt");
    if user_count > 0 {
        return Err(Error::Forbidden(
            "Registration is closed — users already exist".to_string(),
        ));
    }

    // 2. Create the user with the pre-hashed password (email lowercased)
    let email_lower = user.email.to_lowercase();
    let user_row = tx
        .query_one(
            r#"
               insert into users (firstname, lastname, email, password)
               values ($1, $2, $3, $4)
               returning user_id, firstname, lastname, email, avatar_id, created, changed
            "#,
            &[&user.firstname, &user.lastname, &email_lower, &hash],
        )
        .await
        .map_err(Error::Db)?;
    let created_user = UserEntry::from_row(user_row).map_err(Error::DbMapper)?;

    // 3. Seed default roles
    tx.execute(
        r#"
           insert into roles (title)
           values ('Admin'), ('Team Admin'), ('Member'), ('Guest')
           on conflict (title) do nothing
        "#,
        &[],
    )
    .await
    .map_err(Error::Db)?;

    let admin_row = tx
        .query_opt("select role_id from roles where title = $1", &[&ROLE_ADMIN])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Admin role not found after seeding".to_string()))?;
    let admin_role_id: Uuid = admin_row.get(0);

    // 4. Create bootstrap team
    let team_row = tx
        .query_one(
            r#"
               insert into teams (tname, descr)
               values ('Default', 'Bootstrap team')
               returning team_id
            "#,
            &[],
        )
        .await
        .map_err(Error::Db)?;
    let team_id: Uuid = team_row.get(0);

    // 5. Assign user as Admin in the bootstrap team
    tx.execute(
        r#"
           insert into memberof (memberof_team_id, memberof_user_id, memberof_role_id)
           values ($1, $2, $3)
        "#,
        &[&team_id, &created_user.user_id, &admin_role_id],
    )
    .await
    .map_err(Error::Db)?;

    tx.commit().await.map_err(Error::Db)?;

    Ok(created_user)
}
