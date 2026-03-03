use crate::errors::Error;
use deadpool_postgres::Client;

pub async fn check_db(client: &Client) -> Result<(), Error> {
    let statement = client.prepare("select 1").await.map_err(Error::Db)?;

    client
        .query_one(&statement, &[])
        .await
        .map(|_| ())
        .map_err(Error::Db)
}
