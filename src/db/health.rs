use crate::errors::Error;
use deadpool_postgres::Client;

pub async fn check_db(client: &Client) -> Result<bool, Error> {
    let statement = client.prepare("select 1").await.map_err(Error::Db)?;

    client
        .query_one(&statement, &[])
        .await
        .map(|_| true)
        .map_err(Error::Db)
}
