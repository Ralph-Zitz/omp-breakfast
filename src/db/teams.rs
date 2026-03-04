use crate::errors::Error;
use crate::from_row::{FromRow, map_rows};
use crate::models::*;
use deadpool_postgres::Client;
use uuid::Uuid;

/// Returns teams a user belongs to (paginated), with the role title and
/// membership timestamps (`joined`, `role_changed`).
///
/// Returns an empty `Vec` (not 404) when the user has no memberships.
pub async fn get_user_teams(client: &Client, uid: Uuid, limit: i64, offset: i64) -> Result<(Vec<UserInTeams>, i64), Error> {
    let count: i64 = client
        .query_one(
            "select count(*) from memberof where memberof_user_id = $1",
            &[&uid],
        )
        .await
        .map_err(Error::Db)?
        .get(0);

    let statement = client
        .prepare(
            r#"
                select teams.team_id, tname, teams.descr, title, firstname, lastname, memberof.joined, memberof.changed as role_changed
                from memberof
                join users on users.user_id = memberof.memberof_user_id
                join teams on teams.team_id = memberof.memberof_team_id
                join roles on roles.role_id = memberof.memberof_role_id
                where users.user_id = $1
                order by tname asc
                limit $2 offset $3
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let rows = client
        .query(&statement, &[&uid, &limit, &offset])
        .await
        .map_err(Error::Db)?;

    Ok((map_rows(&rows, "user-in-teams"), count))
}

/// Fetches teams with pagination, ordered alphabetically by team name.
///
/// Rows that fail to map are logged with `warn!()` and skipped.
pub async fn get_teams(client: &Client, limit: i64, offset: i64) -> Result<(Vec<TeamEntry>, i64), Error> {
    let count: i64 = client
        .query_one("select count(*) from teams", &[])
        .await
        .map_err(Error::Db)?
        .get(0);

    let statement = client
        .prepare("select team_id, tname, descr, created, changed from teams order by tname asc limit $1 offset $2")
        .await
        .map_err(Error::Db)?;

    let rows = client
        .query(&statement, &[&limit, &offset])
        .await
        .map_err(Error::Db)?;

    Ok((map_rows(&rows, "team"), count))
}

/// Fetches a single team by ID.
///
/// Returns `Error::NotFound` if no team exists with the given ID.
pub async fn get_team(client: &Client, team_id: Uuid) -> Result<TeamEntry, Error> {
    let statement = client
        .prepare(
            "select team_id, tname, descr, created, changed from teams where team_id = $1 limit 1",
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_opt(&statement, &[&team_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Team not found".to_string()))
        .map(TeamEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Creates a new team and returns the created entry.
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

/// Deletes a team by ID. Returns `true` if a row was deleted, `false` if
/// the team did not exist.
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

/// Updates a team's name and description.
///
/// Uses `query_opt` + 404 to avoid returning 500 for missing teams.
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
        .query_opt(&statement, &[&team.tname, &team.descr, &tid])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("Team not found".to_string()))
        .map(TeamEntry::from_row)?
        .map_err(Error::DbMapper)
}

/// Returns users who are members of a team (paginated), with their role titles
/// and membership timestamps (`joined`, `role_changed`).
///
/// Returns an empty `Vec` (not 404) when the team has no members.
pub async fn get_team_users(client: &Client, tid: Uuid, limit: i64, offset: i64) -> Result<(Vec<UsersInTeam>, i64), Error> {
    let count: i64 = client
        .query_one(
            "select count(*) from memberof where memberof_team_id = $1",
            &[&tid],
        )
        .await
        .map_err(Error::Db)?
        .get(0);

    let statement = client
        .prepare(
            r#"
                select user_id, firstname, lastname, email, title, memberof.joined, memberof.changed as role_changed
                from memberof
                join users on users.user_id = memberof.memberof_user_id
                join roles on roles.role_id = memberof.memberof_role_id
                where memberof.memberof_team_id = $1
                order by lastname asc, firstname asc
                limit $2 offset $3
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let rows = client
        .query(&statement, &[&tid, &limit, &offset])
        .await
        .map_err(Error::Db)?;

    Ok((map_rows(&rows, "users-in-team"), count))
}
