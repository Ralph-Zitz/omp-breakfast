use crate::errors::Error;
use crate::from_row::FromRow;
use crate::models::*;
use deadpool_postgres::Client;
use tracing::warn;
use uuid::Uuid;

pub async fn get_user_teams(client: &Client, uid: Uuid) -> Result<Vec<UserInTeams>, Error> {
    let statement = client
        .prepare(
            r#"
                select tname, title, firstname, lastname
                from memberof
                join users on users.user_id = memberof.memberof_user_id
                join teams on teams.team_id = memberof.memberof_team_id
                join roles on roles.role_id = memberof.memberof_role_id
                where users.user_id = $1
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let result = client
        .query(&statement, &[&uid])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| match UserInTeams::from_row_ref(row) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!(error = %e, "Failed to map user-in-teams row — skipping");
                None
            }
        })
        .collect::<Vec<UserInTeams>>();

    Ok(result)
}

pub async fn get_teams(client: &Client) -> Result<Vec<TeamEntry>, Error> {
    let statement = client
        .prepare("select team_id, tname, descr, created, changed from teams order by tname asc")
        .await
        .map_err(Error::Db)?;

    let teams = client
        .query(&statement, &[])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| match TeamEntry::from_row_ref(row) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!(error = %e, "Failed to map team row — skipping");
                None
            }
        })
        .collect();

    Ok(teams)
}

pub async fn get_team(client: &Client, team_id: Uuid) -> Result<TeamEntry, Error> {
    let statement = client
        .prepare(
            "select team_id, tname, descr, created, changed from teams where team_id = $1 limit 1",
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&team_id])
        .await
        .map(TeamEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn create_team(client: &Client, team: CreateTeamEntry) -> Result<TeamEntry, Error> {
    let statement = client
        .prepare("insert into teams (tname, descr) values ($1, $2) returning team_id, tname, descr, created, changed")
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&team.tname, &team.descr])
        .await
        .map(TeamEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn delete_team(client: &Client, tid: Uuid) -> Result<bool, Error> {
    let statement = client
        .prepare("delete from teams where team_id = $1")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&tid])
        .await
        .map_err(Error::Db)?;

    Ok(result == 1)
}

pub async fn update_team(
    client: &Client,
    tid: Uuid,
    team: UpdateTeamEntry,
) -> Result<TeamEntry, Error> {
    let statement = client
        .prepare(
            r#"
               update teams set tname = $1, descr = $2
               where team_id = $3
               returning team_id, tname, descr, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&team.tname, &team.descr, &tid])
        .await
        .map(TeamEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn get_team_users(client: &Client, tid: Uuid) -> Result<Vec<UsersInTeam>, Error> {
    let statement = client
        .prepare(
            r#"
                select user_id, firstname, lastname, email, title
                from memberof
                join users on users.user_id = memberof.memberof_user_id
                join teams on teams.team_id = memberof.memberof_team_id
                join roles on roles.role_id = memberof.memberof_role_id
                where teams.team_id = $1
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let result = client
        .query(&statement, &[&tid])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| match UsersInTeam::from_row_ref(row) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!(error = %e, "Failed to map users-in-team row — skipping");
                None
            }
        })
        .collect::<Vec<UsersInTeam>>();

    Ok(result)
}
