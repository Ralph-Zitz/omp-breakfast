use crate::errors::Error;
use crate::from_row::FromRow;
use crate::models::*;
use deadpool_postgres::Client;
use tracing::warn;
use uuid::Uuid;

/// Check whether a team order is closed. Returns `true` if the order exists
/// and has `closed = true`. Returns `false` if `closed` is `NULL` or `false`.
/// Returns `Error::NotFound` if the order doesn't exist for the given team.
pub async fn is_team_order_closed(
    client: &Client,
    teamorder_id: Uuid,
    team_id: Uuid,
) -> Result<bool, Error> {
    let statement = client
        .prepare(
            "select closed from teamorders where teamorders_id = $1 and teamorders_team_id = $2 limit 1",
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_opt(&statement, &[&teamorder_id, &team_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Team order not found".to_string()))?;

    Ok(row.get::<_, Option<bool>>("closed").unwrap_or(false))
}

pub async fn get_order_items(
    client: &Client,
    teamorder_id: Uuid,
    team_id: Uuid,
) -> Result<Vec<OrderEntry>, Error> {
    let statement = client
        .prepare(
            "select orders_teamorders_id, orders_item_id, orders_team_id, amt from orders where orders_teamorders_id = $1 and orders_team_id = $2 order by orders_item_id",
        )
        .await
        .map_err(Error::Db)?;

    let items = client
        .query(&statement, &[&teamorder_id, &team_id])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| match OrderEntry::from_row_ref(row) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!(error = %e, "Failed to map order item row — skipping");
                None
            }
        })
        .collect();

    Ok(items)
}

pub async fn get_order_item(
    client: &Client,
    teamorder_id: Uuid,
    item_id: Uuid,
    team_id: Uuid,
) -> Result<OrderEntry, Error> {
    let statement = client
        .prepare(
            "select orders_teamorders_id, orders_item_id, orders_team_id, amt from orders where orders_teamorders_id = $1 and orders_item_id = $2 and orders_team_id = $3 limit 1",
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&teamorder_id, &item_id, &team_id])
        .await
        .map(OrderEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn create_order_item(
    client: &Client,
    teamorder_id: Uuid,
    team_id: Uuid,
    order: CreateOrderEntry,
) -> Result<OrderEntry, Error> {
    let statement = client
        .prepare(
            r#"
               insert into orders (orders_teamorders_id, orders_item_id, orders_team_id, amt)
               values ($1, $2, $3, $4)
               returning orders_teamorders_id, orders_item_id, orders_team_id, amt
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_one(
            &statement,
            &[&teamorder_id, &order.orders_item_id, &team_id, &order.amt],
        )
        .await
        .map(OrderEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn update_order_item(
    client: &Client,
    teamorder_id: Uuid,
    item_id: Uuid,
    team_id: Uuid,
    order: UpdateOrderEntry,
) -> Result<OrderEntry, Error> {
    let statement = client
        .prepare(
            r#"
               update orders set amt = $1
               where orders_teamorders_id = $2 and orders_item_id = $3 and orders_team_id = $4
               returning orders_teamorders_id, orders_item_id, orders_team_id, amt
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&order.amt, &teamorder_id, &item_id, &team_id])
        .await
        .map(OrderEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn delete_order_item(
    client: &Client,
    teamorder_id: Uuid,
    item_id: Uuid,
    team_id: Uuid,
) -> Result<bool, Error> {
    let statement = client
        .prepare("delete from orders where orders_teamorders_id = $1 and orders_item_id = $2 and orders_team_id = $3")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&teamorder_id, &item_id, &team_id])
        .await
        .map_err(Error::Db)?;

    Ok(result == 1)
}
