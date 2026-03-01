use crate::{errors::Error, models::*};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use tokio_pg_mapper::FromTokioPostgresRow;
use tracing::warn;
use uuid::Uuid;

pub async fn check_db(client: &Client) -> Result<bool, Error> {
    let statement = client.prepare("select 1").await.map_err(Error::Db)?;

    let result = client.execute(&statement, &[]).await;
    Ok(result.is_ok())
}

pub async fn get_users(client: &Client) -> Result<Vec<UserEntry>, Error> {
    let statement = client
        .prepare("select user_id, firstname, lastname, email, created, changed from users order by firstname asc, lastname asc")
        .await
        .map_err(Error::Db)?;

    let users = client
        .query(&statement, &[])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| match UserEntry::from_row_ref(row) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!(error = %e, "Failed to map user row — skipping");
                None
            }
        })
        .collect();

    Ok(users)
}

pub async fn get_user(client: &Client, user_id: Uuid) -> Result<UserEntry, Error> {
    let statement = client
        .prepare("select user_id, firstname, lastname, email, created, changed from users where user_id = $1 limit 1")
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&user_id])
        .await
        .map(UserEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn get_user_by_email(client: &Client, email: &str) -> Result<UpdateUserEntry, Error> {
    let statement = client
        .prepare("select user_id, firstname, lastname, email, password from users where email = $1 limit 1")
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&email])
        .await
        .map(UpdateUserEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn create_user(client: &Client, user: CreateUserEntry) -> Result<UserEntry, Error> {
    let statement = client
        .prepare(
            r#"
               insert into users (firstname, lastname, email, password)
               values ($1, $2, $3, $4)
               returning user_id, firstname, lastname, email, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(user.password.as_bytes(), &salt)
        .map_err(|err| Error::Argon2(err.to_string()))?
        .to_string();
    client
        .query_one(
            &statement,
            &[&user.firstname, &user.lastname, &user.email, &hash],
        )
        .await
        .map(UserEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn update_user(
    client: &Client,
    uid: Uuid,
    user: UpdateUserRequest,
) -> Result<UserEntry, Error> {
    match &user.password {
        Some(password) => {
            // Password provided — hash and update all fields
            let statement = client
                .prepare(
                    r#"
                       update users set firstname = $1, lastname = $2, email = $3, password = $4
                       where user_id = $5
                       returning user_id, firstname, lastname, email, created, changed
                    "#,
                )
                .await
                .map_err(Error::Db)?;

            let salt = SaltString::generate(&mut OsRng);
            let hash = Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map_err(|err| Error::Argon2(err.to_string()))?
                .to_string();

            client
                .query_one(
                    &statement,
                    &[&user.firstname, &user.lastname, &user.email, &hash, &uid],
                )
                .await
                .map(UserEntry::from_row)?
                .map_err(Error::DbMapper)
        }
        None => {
            // No password — update only profile fields, preserve existing password
            let statement = client
                .prepare(
                    r#"
                       update users set firstname = $1, lastname = $2, email = $3
                       where user_id = $4
                       returning user_id, firstname, lastname, email, created, changed
                    "#,
                )
                .await
                .map_err(Error::Db)?;

            client
                .query_one(
                    &statement,
                    &[&user.firstname, &user.lastname, &user.email, &uid],
                )
                .await
                .map(UserEntry::from_row)?
                .map_err(Error::DbMapper)
        }
    }
}

pub async fn delete_user(client: &Client, uid: Uuid) -> Result<bool, Error> {
    let statement = client
        .prepare("delete from users where user_id = $1")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&uid])
        .await
        .map_err(Error::Db)?;

    Ok(result == 1)
}

pub async fn delete_user_by_email(client: &Client, email: &str) -> Result<bool, Error> {
    let statement = client
        .prepare("delete from users where email = $1")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&email])
        .await
        .map_err(Error::Db)?;

    Ok(result == 1)
}

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
        .map(|row| UserInTeams {
            tname: row.get("tname"),
            title: row.get("title"),
            firstname: row.get("firstname"),
            lastname: row.get("lastname"),
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
        .map(|row| UsersInTeam {
            user_id: row.get("user_id"),
            firstname: row.get("firstname"),
            lastname: row.get("lastname"),
            email: row.get("email"),
            title: row.get("title"),
        })
        .collect::<Vec<UsersInTeam>>();

    Ok(result)
}

pub async fn get_roles(client: &Client) -> Result<Vec<RoleEntry>, Error> {
    let statement = client
        .prepare("select role_id, title, created, changed from roles order by title asc")
        .await
        .map_err(Error::Db)?;

    let roles = client
        .query(&statement, &[])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| match RoleEntry::from_row_ref(row) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!(error = %e, "Failed to map role row — skipping");
                None
            }
        })
        .collect();

    Ok(roles)
}

pub async fn get_role(client: &Client, role_id: Uuid) -> Result<RoleEntry, Error> {
    let statement = client
        .prepare("select role_id, title, created, changed from roles where role_id = $1 limit 1")
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&role_id])
        .await
        .map(RoleEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn create_role(client: &Client, role: CreateRoleEntry) -> Result<RoleEntry, Error> {
    let statement = client
        .prepare("insert into roles (title) values ($1) returning role_id, title, created, changed")
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&role.title])
        .await
        .map(RoleEntry::from_row)?
        .map_err(Error::DbMapper)
}

pub async fn delete_role(client: &Client, rid: Uuid) -> Result<bool, Error> {
    let statement = client
        .prepare("delete from roles where role_id = $1")
        .await
        .map_err(Error::Db)?;

    let result = client
        .execute(&statement, &[&rid])
        .await
        .map_err(Error::Db)?;

    Ok(result == 1)
}

pub async fn update_role(
    client: &Client,
    rid: Uuid,
    role: UpdateRoleEntry,
) -> Result<RoleEntry, Error> {
    let statement = client
        .prepare(
            r#"
               update roles set title = $1
               where role_id = $2
               returning role_id, title, created, changed
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .query_one(&statement, &[&role.title, &rid])
        .await
        .map(RoleEntry::from_row)?
        .map_err(Error::DbMapper)
}

// ── Item CRUD ───────────────────────────────────────────────────────────────

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
        .query_one(&statement, &[&item_id])
        .await
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
        .query_one(&statement, &[&item.descr, &item.price, &item_id])
        .await
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

// ── Team order CRUD ─────────────────────────────────────────────────────────

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

// ── Memberof management ────────────────────────────────────────────────────

/// Check whether the user holds the "Admin" or "Team Admin" role in any team.
pub async fn is_admin_or_team_admin(client: &Client, user_id: Uuid) -> Result<bool, Error> {
    let statement = client
        .prepare(
            r#"
                SELECT EXISTS(
                    SELECT 1
                    FROM memberof m
                    JOIN roles r ON r.role_id = m.memberof_role_id
                    WHERE m.memberof_user_id = $1 AND r.title IN ('Admin', 'Team Admin')
                ) AS is_admin_or_team_admin
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&statement, &[&user_id])
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
                      AND admin_r.title = 'Team Admin'
                      AND target_m.memberof_user_id = $2
                ) AS is_team_admin_of_user
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&statement, &[&requesting_user_id, &target_user_id])
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
                    WHERE m.memberof_user_id = $1 AND r.title = 'Admin'
                ) AS is_admin
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&statement, &[&user_id])
        .await
        .map_err(Error::Db)?;

    Ok(row.get("is_admin"))
}

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

    let rows = client
        .query(&statement, &[&team_id, &user_id])
        .await
        .map_err(Error::Db)?;

    Ok(rows.first().map(|r| r.get("title")))
}

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
               insert into memberof (memberof_team_id, memberof_user_id, memberof_role_id)
               values ($1, $2, $3)
            "#,
        )
        .await
        .map_err(Error::Db)?;

    tx.execute(&statement, &[&team_id, &user_id, &role_id])
        .await
        .map_err(Error::Db)?;

    // Return the joined result
    let query = tx
        .prepare(
            r#"
                select user_id, firstname, lastname, email, title
                from memberof
                join users on users.user_id = memberof.memberof_user_id
                join roles on roles.role_id = memberof.memberof_role_id
                where memberof_team_id = $1
                  and memberof_user_id = $2
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = tx
        .query_one(&query, &[&team_id, &user_id])
        .await
        .map_err(Error::Db)?;

    let result = UsersInTeam {
        user_id: row.get("user_id"),
        firstname: row.get("firstname"),
        lastname: row.get("lastname"),
        email: row.get("email"),
        title: row.get("title"),
    };

    tx.commit().await.map_err(Error::Db)?;

    Ok(result)
}

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
               update memberof set memberof_role_id = $1
               where memberof_team_id = $2 and memberof_user_id = $3
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let updated = tx
        .execute(&statement, &[&role_id, &team_id, &user_id])
        .await
        .map_err(Error::Db)?;

    if updated == 0 {
        return Err(Error::NotFound("member not found in team".to_string()));
    }

    // Return the joined result
    let query = tx
        .prepare(
            r#"
                select user_id, firstname, lastname, email, title
                from memberof
                join users on users.user_id = memberof.memberof_user_id
                join roles on roles.role_id = memberof.memberof_role_id
                where memberof_team_id = $1
                  and memberof_user_id = $2
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = tx
        .query_one(&query, &[&team_id, &user_id])
        .await
        .map_err(Error::Db)?;

    let result = UsersInTeam {
        user_id: row.get("user_id"),
        firstname: row.get("firstname"),
        lastname: row.get("lastname"),
        email: row.get("email"),
        title: row.get("title"),
    };

    tx.commit().await.map_err(Error::Db)?;

    Ok(result)
}

// ── Order CRUD (items within a team order) ──────────────────────────────────

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

// ── Token blacklist (DB-backed) ─────────────────────────────────────────────

/// Insert a revoked token into the persistent blacklist.
/// `expires_at` should match the token's original expiry so that cleanup can
/// remove entries that are no longer relevant.
pub async fn revoke_token_db(
    client: &Client,
    jti: Uuid,
    expires_at: DateTime<Utc>,
) -> Result<(), Error> {
    let statement = client
        .prepare(
            r#"
               INSERT INTO token_blacklist (jti, expires_at)
               VALUES ($1, $2)
               ON CONFLICT (jti) DO NOTHING
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .execute(&statement, &[&jti, &expires_at])
        .await
        .map_err(Error::Db)?;

    Ok(())
}

/// Check whether a token (by jti) has been revoked.
pub async fn is_token_revoked_db(client: &Client, jti: Uuid) -> Result<bool, Error> {
    let statement = client
        .prepare(
            r#"
               SELECT EXISTS(
                   SELECT 1 FROM token_blacklist WHERE jti = $1
               ) AS revoked
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&statement, &[&jti])
        .await
        .map_err(Error::Db)?;

    Ok(row.get("revoked"))
}

/// Remove expired entries from the token blacklist.
/// Returns the number of rows deleted.
pub async fn cleanup_expired_tokens(client: &Client) -> Result<u64, Error> {
    let statement = client
        .prepare("DELETE FROM token_blacklist WHERE expires_at < now()")
        .await
        .map_err(Error::Db)?;

    let result = client.execute(&statement, &[]).await.map_err(Error::Db)?;

    Ok(result)
}
