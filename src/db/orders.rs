use crate::errors::Error;
use crate::from_row::{FromRow, map_rows};
use crate::models::*;
use deadpool_postgres::Client;
use uuid::Uuid;

/// Fetches team orders with pagination for a team, ordered by creation date
/// descending (newest first).
///
/// Rows that fail to map are logged with `warn!()` and skipped.
pub async fn get_team_orders(
    client: &Client,
    team_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<(Vec<TeamOrderEntry>, i64), Error> {
    let statement = client
        .prepare(
            r#"
                select teamorders_id, teamorders_team_id, teamorders_user_id,
                       pickup_user_id, duedate, closed, created, changed,
                       count(*) over() as total_count
                from teamorders
                where teamorders_team_id = $1
                order by created desc
                limit $2 offset $3
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let rows = client
        .query(&statement, &[&team_id, &limit, &offset])
        .await
        .map_err(Error::Db)?;

    let total: i64 = rows.first().map(|r| r.get("total_count")).unwrap_or(0);
    Ok((map_rows(&rows, "team order"), total))
}

/// Fetches a single team order by order ID and team ID.
///
/// Returns `Error::NotFound` if no matching order exists.
pub async fn get_team_order(
    client: &Client,
    team_id: Uuid,
    order_id: Uuid,
) -> Result<TeamOrderEntry, Error> {
    let statement = client
        .prepare(
            r#"
                select teamorders_id, teamorders_team_id, teamorders_user_id,
                       pickup_user_id, duedate, closed, created, changed
                from teamorders
                where teamorders_id = $1 and teamorders_team_id = $2
                limit 1
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(&statement, &[&order_id, &team_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Team order not found".to_string()))
        .map(TeamOrderEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Creates a new team order with a due date. The creating user's ID is passed
/// separately (extracted from JWT claims by the handler) to prevent attribution
/// spoofing.
pub async fn create_team_order(
    client: &Client,
    team_id: Uuid,
    user_id: Uuid,
    order: CreateTeamOrderEntry,
) -> Result<TeamOrderEntry, Error> {
    let statement = client
        .prepare(
            r#"
               insert into teamorders (teamorders_team_id, teamorders_user_id, duedate, pickup_user_id)
               values ($1, $2, $3, $4)
               returning teamorders_id, teamorders_team_id, teamorders_user_id,
                         pickup_user_id, duedate, closed, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_one(
            &statement,
            &[&team_id, &user_id, &order.duedate, &order.pickup_user_id],
        )
        .await
        .map(TeamOrderEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Updates a team order. `COALESCE` preserves existing values when fields are
/// absent from the request. For `duedate` and `pickup_user_id`, the
/// triple-option pattern (`Option<Option<T>>`) is used:
///   - `None` → field absent, preserve existing value (CASE WHEN $N)
///   - `Some(None)` → explicitly clear to NULL
///   - `Some(Some(val))` → set to the new value
///
/// `teamorders_user_id` is intentionally excluded — order ownership cannot be
/// reassigned after creation.
///
/// Uses `query_opt` + 404 to avoid returning 500 for missing orders.
pub async fn update_team_order(
    client: &Client,
    team_id: Uuid,
    order_id: Uuid,
    order: UpdateTeamOrderEntry,
) -> Result<TeamOrderEntry, Error> {
    // Decompose the triple-option: None → preserve, Some(x) → update to x (including NULL)
    let (duedate_val, update_duedate) = match order.duedate {
        Some(d) => (d, true),
        None => (None, false),
    };
    let (pickup_val, update_pickup) = match order.pickup_user_id {
        Some(p) => (p, true),
        None => (None, false),
    };

    let statement = client
        .prepare(
            r#"
               update teamorders
               set duedate = CASE WHEN $5::boolean THEN $1 ELSE duedate END,
                   closed = COALESCE($2, closed),
                   pickup_user_id = CASE WHEN $6::boolean THEN $7 ELSE pickup_user_id END
               where teamorders_id = $3 and teamorders_team_id = $4
               returning teamorders_id, teamorders_team_id, teamorders_user_id,
                         pickup_user_id, duedate, closed, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(
            &statement,
            &[
                &duedate_val,
                &order.closed,
                &order_id,
                &team_id,
                &update_duedate,
                &update_pickup,
                &pickup_val,
            ],
        )
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Team order not found".to_string()))
        .map(TeamOrderEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Deletes a single team order by order ID and team ID. Returns `true` if
/// a row was deleted, `false` if no matching order existed.
pub async fn delete_team_order(
    client: &Client,
    team_id: Uuid,
    order_id: Uuid,
) -> Result<bool, Error> {
    let statement = client
        .prepare("delete from teamorders where teamorders_id = $1 and teamorders_team_id = $2")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&order_id, &team_id])
        .await
        .map_err(Error::Db)?;

    Ok(result == 1)
}

/// Counts the number of orders for a team.
pub async fn count_team_orders(client: &Client, team_id: Uuid) -> Result<i64, Error> {
    let statement = client
        .prepare("select count(*) as cnt from teamorders where teamorders_team_id = $1")
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&statement, &[&team_id])
        .await
        .map_err(Error::Db)?;

    Ok(row.get("cnt"))
}

/// Deletes all team orders for a team. Returns the number of rows deleted.
pub async fn delete_team_orders(client: &Client, team_id: Uuid) -> Result<u64, Error> {
    let statement = client
        .prepare("delete from teamorders where teamorders_team_id = $1")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&team_id])
        .await
        .map_err(Error::Db)?;

    Ok(result)
}

/// Reopens a closed team order by duplicating it: creates a new open order
/// (with no due date and no pickup user) and copies all line items from the
/// original order into the new one. The original order remains unchanged.
///
/// Returns `Error::NotFound` if the source order does not exist, and
/// `Error::Validation` if the source order is not closed.
pub async fn reopen_team_order(
    client: &mut Client,
    team_id: Uuid,
    order_id: Uuid,
    user_id: Uuid,
) -> Result<TeamOrderEntry, Error> {
    let tx = client.transaction().await.map_err(Error::Db)?;

    // Verify the source order exists and is closed
    let check = tx
        .prepare(
            "select closed from teamorders where teamorders_id = $1 and teamorders_team_id = $2 for update",
        )
        .await
        .map_err(Error::Db)?;

    let row = tx
        .query_opt(&check, &[&order_id, &team_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Team order not found".to_string()))?;

    if !row.get::<_, bool>("closed") {
        return Err(Error::Validation(
            "Only closed orders can be reopened".to_string(),
        ));
    }

    // Create the new order with no due date and no pickup user
    let insert = tx
        .prepare(
            r#"
               insert into teamorders (teamorders_team_id, teamorders_user_id)
               values ($1, $2)
               returning teamorders_id, teamorders_team_id, teamorders_user_id,
                         pickup_user_id, duedate, closed, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let new_row = tx
        .query_one(&insert, &[&team_id, &user_id])
        .await
        .map_err(Error::Db)?;

    let new_order = TeamOrderEntry::from_row(new_row).map_err(Error::DbMapper)?;

    // Copy all line items from the old order to the new one
    let copy = tx
        .prepare(
            r#"
               insert into orders (orders_teamorders_id, orders_item_id, orders_team_id, amt)
               select $1, orders_item_id, orders_team_id, amt
               from orders
               where orders_teamorders_id = $2 and orders_team_id = $3
            "#,
        )
        .await
        .map_err(Error::Db)?;

    tx.execute(&copy, &[&new_order.teamorders_id, &order_id, &team_id])
        .await
        .map_err(Error::Db)?;

    tx.commit().await.map_err(Error::Db)?;

    Ok(new_order)
}
