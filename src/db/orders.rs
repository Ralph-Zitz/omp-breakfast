use crate::errors::Error;
use crate::from_row::FromRow;
use crate::models::*;
use deadpool_postgres::Client;
use tracing::warn;
use uuid::Uuid;

/// Fetches all team orders for a team, ordered by creation date descending
/// (newest first).
///
/// Rows that fail to map are logged with `warn!()` and skipped.
pub async fn get_team_orders(client: &Client, team_id: Uuid) -> Result<Vec<TeamOrderEntry>, Error> {
    let statement = client
        .prepare(
            r#"
                select teamorders_id, teamorders_team_id, teamorders_user_id,
                       duedate, closed, created, changed
                from teamorders
                where teamorders_team_id = $1
                order by created desc
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let orders = client
        .query(&statement, &[&team_id])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| match TeamOrderEntry::from_row_ref(row) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!(error = %e, "Failed to map team order row — skipping");
                None
            }
        })
        .collect();

    Ok(orders)
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
                       duedate, closed, created, changed
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
               insert into teamorders (teamorders_team_id, teamorders_user_id, duedate)
               values ($1, $2, $3)
               returning teamorders_id, teamorders_team_id, teamorders_user_id,
                         duedate, closed, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&team_id, &user_id, &order.duedate])
        .await
        .map(TeamOrderEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Updates a team order. `COALESCE` preserves existing values when fields are
/// absent from the request. For `duedate`, the triple-option pattern
/// (`Option<Option<NaiveDate>>`) is used:
///   - `None` → field absent, preserve existing value (CASE WHEN $5)
///   - `Some(None)` → explicitly clear to NULL
///   - `Some(Some(date))` → set to the new date
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

    let statement = client
        .prepare(
            r#"
               update teamorders
               set duedate = CASE WHEN $5::boolean THEN $1 ELSE duedate END,
                   closed = COALESCE($2, closed)
               where teamorders_id = $3 and teamorders_team_id = $4
               returning teamorders_id, teamorders_team_id, teamorders_user_id,
                         duedate, closed, created, changed
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
