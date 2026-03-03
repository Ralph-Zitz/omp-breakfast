use crate::errors::Error;
use deadpool_postgres::Client;

/// Executes a trivial `SELECT 1` query to verify database connectivity.
///
/// Used by the `/health` endpoint to report database availability.
pub async fn check_db(client: &Client) -> Result<(), Error> {
    let statement = client.prepare("select 1").await.map_err(Error::Db)?;

    client
        .query_one(&statement, &[])
        .await
        .map(|_| ())
        .map_err(Error::Db)
}
