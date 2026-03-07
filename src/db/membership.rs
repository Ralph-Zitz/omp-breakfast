use crate::errors::Error;
use crate::from_row::FromRow;
use crate::middleware::auth::{ROLE_ADMIN, ROLE_TEAM_ADMIN};
use crate::models::*;
use deadpool_postgres::Client;
use uuid::Uuid;

/// Count the number of distinct users that hold the "Admin" role in any team.
pub async fn count_admins(client: &Client) -> Result<i64, Error> {
    let statement = client
        .prepare(
            r#"
                SELECT COUNT(DISTINCT m.memberof_user_id) AS admin_count
                FROM memberof m
                JOIN roles r ON r.role_id = m.memberof_role_id
                WHERE r.title = $1
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&statement, &[&ROLE_ADMIN])
        .await
        .map_err(Error::Db)?;

    Ok(row.get("admin_count"))
}

/// Check whether removing or changing a specific membership would leave zero
/// global admins. Returns `true` if the operation is safe (at least one admin
/// would remain), `false` if it would leave zero admins.
///
/// The query counts distinct admin users *excluding* the specified
/// (team_id, user_id) pair, which represents the membership being removed or
/// changed away from Admin.
pub async fn would_admins_remain_without(
    client: &Client,
    team_id: Uuid,
    user_id: Uuid,
) -> Result<bool, Error> {
    let statement = client
        .prepare(
            r#"
                SELECT COUNT(DISTINCT m.memberof_user_id) AS admin_count
                FROM memberof m
                JOIN roles r ON r.role_id = m.memberof_role_id
                WHERE r.title = $1
                  AND NOT (m.memberof_team_id = $2 AND m.memberof_user_id = $3)
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&statement, &[&ROLE_ADMIN, &team_id, &user_id])
        .await
        .map_err(Error::Db)?;

    let remaining: i64 = row.get("admin_count");
    Ok(remaining > 0)
}

/// Check whether the user holds the "Admin" or "Team Admin" role in any team.
pub async fn is_admin_or_team_admin(client: &Client, user_id: Uuid) -> Result<bool, Error> {
    let statement = client
        .prepare(
            r#"
                SELECT EXISTS(
                    SELECT 1
                    FROM memberof m
                    JOIN roles r ON r.role_id = m.memberof_role_id
                    WHERE m.memberof_user_id = $1 AND r.title IN ($2, $3)
                ) AS is_admin_or_team_admin
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&statement, &[&user_id, &ROLE_ADMIN, &ROLE_TEAM_ADMIN])
        .await
        .map_err(Error::Db)?;

    Ok(row.get("is_admin_or_team_admin"))
}

/// Check whether the requesting user holds the "Team Admin" role in any team
/// where the target user is also a member.
pub async fn is_team_admin_of_user(
    client: &Client,
    requesting_user_id: Uuid,
    target_user_id: Uuid,
) -> Result<bool, Error> {
    let statement = client
        .prepare(
            r#"
                SELECT EXISTS(
                    SELECT 1
                    FROM memberof admin_m
                    JOIN roles admin_r ON admin_r.role_id = admin_m.memberof_role_id
                    JOIN memberof target_m ON target_m.memberof_team_id = admin_m.memberof_team_id
                    WHERE admin_m.memberof_user_id = $1
                      AND admin_r.title = $3
                      AND target_m.memberof_user_id = $2
                ) AS is_team_admin_of_user
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(
            &statement,
            &[&requesting_user_id, &target_user_id, &ROLE_TEAM_ADMIN],
        )
        .await
        .map_err(Error::Db)?;

    Ok(row.get("is_team_admin_of_user"))
}

/// Check whether the user holds the "Admin" role in any team.
pub async fn is_admin(client: &Client, user_id: Uuid) -> Result<bool, Error> {
    let statement = client
        .prepare(
            r#"
                SELECT EXISTS(
                    SELECT 1
                    FROM memberof m
                    JOIN roles r ON r.role_id = m.memberof_role_id
                    WHERE m.memberof_user_id = $1 AND r.title = $2
                ) AS is_admin
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&statement, &[&user_id, &ROLE_ADMIN])
        .await
        .map_err(Error::Db)?;

    Ok(row.get("is_admin"))
}

/// Returns the role title for a user in a specific team, or `None` if the
/// user is not a member of that team.
pub async fn get_member_role(
    client: &Client,
    team_id: Uuid,
    user_id: Uuid,
) -> Result<Option<String>, Error> {
    let statement = client
        .prepare(
            r#"
                SELECT r.title
                FROM memberof m
                JOIN roles r ON r.role_id = m.memberof_role_id
                WHERE m.memberof_team_id = $1 AND m.memberof_user_id = $2
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_opt(&statement, &[&team_id, &user_id])
        .await
        .map_err(Error::Db)?;

    Ok(row.map(|r| r.get("title")))
}

/// Combined RBAC check: returns whether the user is a global admin and their
/// role in the specified team (if any), in a single database round-trip.
/// Used by `require_team_member` and `require_team_admin` handlers.
pub async fn check_team_access(
    client: &Client,
    team_id: Uuid,
    user_id: Uuid,
) -> Result<(bool, Option<String>), Error> {
    let statement = client
        .prepare(
            r#"
                SELECT
                    EXISTS(
                        SELECT 1
                        FROM memberof m
                        JOIN roles r ON r.role_id = m.memberof_role_id
                        WHERE m.memberof_user_id = $1 AND r.title = $3
                    ) AS is_admin,
                    (
                        SELECT r.title
                        FROM memberof m
                        JOIN roles r ON r.role_id = m.memberof_role_id
                        WHERE m.memberof_team_id = $2 AND m.memberof_user_id = $1
                        LIMIT 1
                    ) AS team_role
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&statement, &[&user_id, &team_id, &ROLE_ADMIN])
        .await
        .map_err(Error::Db)?;

    let is_admin: bool = row.get("is_admin");
    let team_role: Option<String> = row.get("team_role");
    Ok((is_admin, team_role))
}

/// Adds a user to a team with the specified role inside a transaction.
///
/// Returns the new membership as a [`UsersInTeam`] with joined user/role details.
pub async fn add_team_member(
    client: &mut Client,
    team_id: Uuid,
    user_id: Uuid,
    role_id: Uuid,
) -> Result<UsersInTeam, Error> {
    let tx = client.transaction().await.map_err(Error::Db)?;

    let statement = tx
        .prepare(
            r#"
               with ins as (
                   insert into memberof (memberof_team_id, memberof_user_id, memberof_role_id)
                   values ($1, $2, $3)
                   returning memberof_user_id, memberof_role_id, joined, changed
               )
               select u.user_id, u.firstname, u.lastname, u.email,
                      r.title, ins.joined, ins.changed as role_changed
               from ins
               join users u on u.user_id = ins.memberof_user_id
               join roles r on r.role_id = ins.memberof_role_id
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = tx
        .query_one(&statement, &[&team_id, &user_id, &role_id])
        .await
        .map_err(Error::Db)?;

    let result = UsersInTeam::from_row_ref(&row)?;

    tx.commit().await.map_err(Error::Db)?;

    Ok(result)
}

/// Removes a user from a team. Returns `true` if the membership was
/// deleted, `false` if the user was not a member.
pub async fn remove_team_member(
    client: &Client,
    team_id: Uuid,
    user_id: Uuid,
) -> Result<bool, Error> {
    let statement = client
        .prepare("delete from memberof where memberof_team_id = $1 and memberof_user_id = $2")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&team_id, &user_id])
        .await
        .map_err(Error::Db)?;

    Ok(result == 1)
}

/// Changes a member's role in a team inside a transaction.
///
/// Returns `Error::NotFound` if the user is not a member of the team.
/// Returns the updated membership as a [`UsersInTeam`].
pub async fn update_member_role(
    client: &mut Client,
    team_id: Uuid,
    user_id: Uuid,
    role_id: Uuid,
) -> Result<UsersInTeam, Error> {
    let tx = client.transaction().await.map_err(Error::Db)?;

    let statement = tx
        .prepare(
            r#"
               with upd as (
                   update memberof set memberof_role_id = $1
                   where memberof_team_id = $2 and memberof_user_id = $3
                   returning memberof_user_id, memberof_role_id, joined, changed
               )
               select u.user_id, u.firstname, u.lastname, u.email,
                      r.title, upd.joined, upd.changed as role_changed
               from upd
               join users u on u.user_id = upd.memberof_user_id
               join roles r on r.role_id = upd.memberof_role_id
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = tx
        .query_opt(&statement, &[&role_id, &team_id, &user_id])
        .await
        .map_err(Error::Db)?
        .ok_or_else(|| Error::NotFound("member not found in team".to_string()))?;

    let result = UsersInTeam::from_row_ref(&row)?;

    tx.commit().await.map_err(Error::Db)?;

    Ok(result)
}
