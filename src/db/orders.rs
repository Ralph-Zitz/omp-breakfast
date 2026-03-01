use crate::errors::Error;
use crate::from_row::FromRow;
use crate::models::*;
use deadpool_postgres::Client;
use tracing::warn;
use uuid::Uuid;

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
        .query_one(&statement, &[&order_id, &team_id])
        .await
        .map(TeamOrderEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn create_team_order(
    client: &Client,
    team_id: Uuid,
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
        .query_one(
            &statement,
            &[&team_id, &order.teamorders_user_id, &order.duedate],
        )
        .await
        .map(TeamOrderEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn update_team_order(
    client: &Client,
    team_id: Uuid,
    order_id: Uuid,
    order: UpdateTeamOrderEntry,
) -> Result<TeamOrderEntry, Error> {
    let statement = client
        .prepare(
            r#"
               update teamorders
               set teamorders_user_id = $1, duedate = $2, closed = $3
               where teamorders_id = $4 and teamorders_team_id = $5
               returning teamorders_id, teamorders_team_id, teamorders_user_id,
                         duedate, closed, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_one(
            &statement,
            &[
                &order.teamorders_user_id,
                &order.duedate,
                &order.closed,
                &order_id,
                &team_id,
            ],
        )
        .await
        .map(TeamOrderEntry::from_row)?
        .map_err(Error::DbMapper)
}

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
