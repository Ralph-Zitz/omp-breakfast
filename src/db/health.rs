use crate::errors::Error;
use deadpool_postgres::Client;

pub async fn check_db(client: &Client) -> Result<bool, Error> {
    let statement = client.prepare("select 1").await.map_err(Error::Db)?;

    let result = client.execute(&statement, &[]).await;
    Ok(result.is_ok())
}
