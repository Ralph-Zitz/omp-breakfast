use crate::errors::Error;
use crate::from_row::FromRow;
use crate::models::*;
use deadpool_postgres::Client;
use tokio_postgres::Transaction;
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

/// Lock the team order row with `FOR UPDATE` inside a transaction and return
/// whether it is closed. This prevents TOCTOU races: the row lock is held
/// until the transaction commits or rolls back.
async fn guard_open_order(
    tx: &Transaction<'_>,
    teamorder_id: Uuid,
    team_id: Uuid,
    action: &str,
) -> Result<(), Error> {
    let statement = tx
        .prepare(
            "select closed from teamorders where teamorders_id = $1 and teamorders_team_id = $2 for update",
        )
        .await
        .map_err(Error::Db)?;

    let row = tx
        .query_opt(&statement, &[&teamorder_id, &team_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Team order not found".to_string()))?;

    if row.get::<_, Option<bool>>("closed").unwrap_or(false) {
        return Err(Error::Forbidden(format!(
            "Cannot {} items in a closed order",
            action,
        )));
    }

    Ok(())
}

/// Fetches all line items for a team order, ordered by item ID.
///
/// Rows that fail to map are logged with `warn!()` and skipped.
pub async fn get_order_items(
    client: &Client,
    teamorder_id: Uuid,
    team_id: Uuid,
) -> Result<Vec<OrderEntry>, Error> {
    let statement = client
        .prepare(
            "select orders_teamorders_id, orders_item_id, orders_team_id, amt, created, changed from orders where orders_teamorders_id = $1 and orders_team_id = $2 order by orders_item_id",
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

/// Fetches a single line item by team order ID, item ID, and team ID.
///
/// Returns `Error::NotFound` if no matching line item exists.
pub async fn get_order_item(
    client: &Client,
    teamorder_id: Uuid,
    item_id: Uuid,
    team_id: Uuid,
) -> Result<OrderEntry, Error> {
    let statement = client
        .prepare(
            "select orders_teamorders_id, orders_item_id, orders_team_id, amt, created, changed from orders where orders_teamorders_id = $1 and orders_item_id = $2 and orders_team_id = $3 limit 1",
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(&statement, &[&teamorder_id, &item_id, &team_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Order item not found".to_string()))
        .map(OrderEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Adds a line item to a team order within a transaction. The order is
/// locked with `SELECT ... FOR UPDATE` via [`guard_open_order`] to prevent
/// modifications to closed orders.
pub async fn create_order_item(
    client: &mut Client,
    teamorder_id: Uuid,
    team_id: Uuid,
    order: CreateOrderEntry,
) -> Result<OrderEntry, Error> {
    let tx = client.transaction().await.map_err(Error::Db)?;

    guard_open_order(&tx, teamorder_id, team_id, "add").await?;

    let statement = tx
        .prepare(
            r#"
               insert into orders (orders_teamorders_id, orders_item_id, orders_team_id, amt)
               values ($1, $2, $3, $4)
               returning orders_teamorders_id, orders_item_id, orders_team_id, amt, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = tx
        .query_one(
            &statement,
            &[&teamorder_id, &order.orders_item_id, &team_id, &order.amt],
        )
        .await
        .map_err(Error::Db)?;

    let result = OrderEntry::from_row(row).map_err(Error::DbMapper)?;

    tx.commit().await.map_err(Error::Db)?;

    Ok(result)
}

/// Updates the quantity (`amt`) of a line item within a transaction.
/// The order is locked via [`guard_open_order`] to prevent modifications
/// to closed orders.
///
/// Returns `Error::NotFound` if the line item does not exist.
pub async fn update_order_item(
    client: &mut Client,
    teamorder_id: Uuid,
    item_id: Uuid,
    team_id: Uuid,
    order: UpdateOrderEntry,
) -> Result<OrderEntry, Error> {
    let tx = client.transaction().await.map_err(Error::Db)?;

    guard_open_order(&tx, teamorder_id, team_id, "modify").await?;

    let statement = tx
        .prepare(
            r#"
               update orders set amt = $1
               where orders_teamorders_id = $2 and orders_item_id = $3 and orders_team_id = $4
               returning orders_teamorders_id, orders_item_id, orders_team_id, amt, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = tx
        .query_opt(&statement, &[&order.amt, &teamorder_id, &item_id, &team_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Order item not found".to_string()))?;

    let result = OrderEntry::from_row(row).map_err(Error::DbMapper)?;

    tx.commit().await.map_err(Error::Db)?;

    Ok(result)
}

/// Deletes a line item from a team order within a transaction.
/// The order is locked via [`guard_open_order`] to prevent modifications
/// to closed orders.
///
/// Returns `true` if a row was deleted, `false` otherwise.
pub async fn delete_order_item(
    client: &mut Client,
    teamorder_id: Uuid,
    item_id: Uuid,
    team_id: Uuid,
) -> Result<bool, Error> {
    let tx = client.transaction().await.map_err(Error::Db)?;

    guard_open_order(&tx, teamorder_id, team_id, "remove").await?;

    let statement = tx
        .prepare("delete from orders where orders_teamorders_id = $1 and orders_item_id = $2 and orders_team_id = $3")
        .await
        .map_err(Error::Db)?;

    let result = tx
        .execute(&statement, &[&teamorder_id, &item_id, &team_id])
        .await
        .map_err(Error::Db)?;

    tx.commit().await.map_err(Error::Db)?;

    Ok(result == 1)
}
