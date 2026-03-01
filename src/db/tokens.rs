use crate::errors::Error;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use uuid::Uuid;

/// Insert a revoked token into the persistent blacklist.
/// `expires_at` should match the token's original expiry so that cleanup can
/// remove entries that are no longer relevant.
pub async fn revoke_token_db(
    client: &Client,
    jti: Uuid,
    expires_at: DateTime<Utc>,
) -> Result<(), Error> {
    let statement = client
        .prepare(
            r#"
               INSERT INTO token_blacklist (jti, expires_at)
               VALUES ($1, $2)
               ON CONFLICT (jti) DO NOTHING
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .execute(&statement, &[&jti, &expires_at])
        .await
        .map_err(Error::Db)?;

    Ok(())
}

/// Check whether a token (by jti) has been revoked.
pub async fn is_token_revoked_db(client: &Client, jti: Uuid) -> Result<bool, Error> {
    let statement = client
        .prepare(
            r#"
               SELECT EXISTS(
                   SELECT 1 FROM token_blacklist WHERE jti = $1
               ) AS revoked
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&statement, &[&jti])
        .await
        .map_err(Error::Db)?;

    Ok(row.get("revoked"))
}

/// Remove expired entries from the token blacklist.
/// Returns the number of rows deleted.
pub async fn cleanup_expired_tokens(client: &Client) -> Result<u64, Error> {
    let statement = client
        .prepare("DELETE FROM token_blacklist WHERE expires_at < now()")
        .await
        .map_err(Error::Db)?;

    let result = client.execute(&statement, &[]).await.map_err(Error::Db)?;

    Ok(result)
}
