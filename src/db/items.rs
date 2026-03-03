use crate::from_row::FromRow;
use crate::{errors::Error, models::*};
use deadpool_postgres::Client;
use tracing::warn;
use uuid::Uuid;

pub async fn get_items(client: &Client) -> Result<Vec<ItemEntry>, Error> {
    let statement = client
        .prepare("select item_id, descr, price, created, changed from items order by descr asc")
        .await
        .map_err(Error::Db)?;

    let items = client
        .query(&statement, &[])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| match ItemEntry::from_row_ref(row) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!(error = %e, "Failed to map item row — skipping");
                None
            }
        })
        .collect();

    Ok(items)
}

pub async fn get_item(client: &Client, item_id: Uuid) -> Result<ItemEntry, Error> {
    let statement = client
        .prepare(
            "select item_id, descr, price, created, changed from items where item_id = $1 limit 1",
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(&statement, &[&item_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Item not found".to_string()))
        .map(ItemEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn create_item(client: &Client, item: CreateItemEntry) -> Result<ItemEntry, Error> {
    let statement = client
        .prepare(
            r#"
               insert into items (descr, price)
               values ($1, $2)
               returning item_id, descr, price, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&item.descr, &item.price])
        .await
        .map(ItemEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn update_item(
    client: &Client,
    item_id: Uuid,
    item: UpdateItemEntry,
) -> Result<ItemEntry, Error> {
    let statement = client
        .prepare(
            r#"
               update items set descr = $1, price = $2
               where item_id = $3
               returning item_id, descr, price, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(&statement, &[&item.descr, &item.price, &item_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Item not found".to_string()))
        .map(ItemEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn delete_item(client: &Client, item_id: Uuid) -> Result<bool, Error> {
    let statement = client
        .prepare("delete from items where item_id = $1")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&item_id])
        .await
        .map_err(Error::Db)?;

    Ok(result == 1)
}
