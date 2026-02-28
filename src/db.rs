use crate::{errors::Error, models::*};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use deadpool_postgres::Client;
use tokio_pg_mapper::FromTokioPostgresRow;
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
        .filter_map(|row| UserEntry::from_row_ref(row).ok())
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
        .map_err(|err| Error::Argonautica(err.to_string()))?
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
                .map_err(|err| Error::Argonautica(err.to_string()))?
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
                from users, teams, memberof, roles
                where users.user_id = $1
                  and users.user_id = memberof.memberof_user_id
                  and memberof.memberof_team_id = teams.team_id
                  and memberof.memberof_role_id = roles.role_id
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

    if result.is_empty() {
        Err(Error::NotFound("record not found".to_string()))
    } else {
        Ok(result)
    }
}

pub async fn get_teams(client: &Client) -> Result<Vec<TeamEntry>, Error> {
    let statement = client
        .prepare("select team_id, tname, descr from teams order by tname asc")
        .await
        .map_err(Error::Db)?;

    let teams = client
        .query(&statement, &[])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| TeamEntry::from_row_ref(row).ok())
        .collect();

    Ok(teams)
}

pub async fn get_team(client: &Client, team_id: Uuid) -> Result<TeamEntry, Error> {
    let statement = client
        .prepare("select team_id, tname, descr from teams where team_id = $1 limit 1")
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
        .prepare("insert into teams (tname, descr) values ($1, $2) returning team_id, tname, descr")
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
               returning team_id, tname, descr
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
                from users, teams, memberof, roles
                where teams.team_id = $1
                  and teams.team_id = memberof.memberof_team_id
                  and users.user_id = memberof.memberof_user_id
                  and memberof.memberof_role_id = roles.role_id
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

    if result.is_empty() {
        Err(Error::NotFound("record not found".to_string()))
    } else {
        Ok(result)
    }
}

pub async fn get_roles(client: &Client) -> Result<Vec<RoleEntry>, Error> {
    let statement = client
        .prepare("select role_id, title from roles order by title asc")
        .await
        .map_err(Error::Db)?;

    let roles = client
        .query(&statement, &[])
        .await
        .map_err(Error::Db)?
        .iter()
        .filter_map(|row| RoleEntry::from_row_ref(row).ok())
        .collect();

    Ok(roles)
}

pub async fn get_role(client: &Client, role_id: Uuid) -> Result<RoleEntry, Error> {
    let statement = client
        .prepare("select role_id, title from roles where role_id = $1 limit 1")
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
        .prepare("insert into roles (title) values ($1) returning role_id, title")
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
               returning role_id, title
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
        .filter_map(|row| ItemEntry::from_row_ref(row).ok())
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
        .filter_map(|row| TeamOrderEntry::from_row_ref(row).ok())
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

pub async fn add_team_member(
    client: &Client,
    team_id: Uuid,
    user_id: Uuid,
    role_id: Uuid,
) -> Result<UsersInTeam, Error> {
    let statement = client
        .prepare(
            r#"
               insert into memberof (memberof_team_id, memberof_user_id, memberof_role_id)
               values ($1, $2, $3)
            "#,
        )
        .await
        .map_err(Error::Db)?;

    client
        .execute(&statement, &[&team_id, &user_id, &role_id])
        .await
        .map_err(Error::Db)?;

    // Return the joined result
    let query = client
        .prepare(
            r#"
                select user_id, firstname, lastname, email, title
                from users, roles, memberof
                where memberof_team_id = $1
                  and memberof_user_id = $2
                  and users.user_id = memberof_user_id
                  and roles.role_id = memberof_role_id
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&query, &[&team_id, &user_id])
        .await
        .map_err(Error::Db)?;

    Ok(UsersInTeam {
        user_id: row.get("user_id"),
        firstname: row.get("firstname"),
        lastname: row.get("lastname"),
        email: row.get("email"),
        title: row.get("title"),
    })
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
    client: &Client,
    team_id: Uuid,
    user_id: Uuid,
    role_id: Uuid,
) -> Result<UsersInTeam, Error> {
    let statement = client
        .prepare(
            r#"
               update memberof set memberof_role_id = $1
               where memberof_team_id = $2 and memberof_user_id = $3
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let updated = client
        .execute(&statement, &[&role_id, &team_id, &user_id])
        .await
        .map_err(Error::Db)?;

    if updated == 0 {
        return Err(Error::NotFound("member not found in team".to_string()));
    }

    // Return the joined result
    let query = client
        .prepare(
            r#"
                select user_id, firstname, lastname, email, title
                from users, roles, memberof
                where memberof_team_id = $1
                  and memberof_user_id = $2
                  and users.user_id = memberof_user_id
                  and roles.role_id = memberof_role_id
            "#,
        )
        .await
        .map_err(Error::Db)?;

    let row = client
        .query_one(&query, &[&team_id, &user_id])
        .await
        .map_err(Error::Db)?;

    Ok(UsersInTeam {
        user_id: row.get("user_id"),
        firstname: row.get("firstname"),
        lastname: row.get("lastname"),
        email: row.get("email"),
        title: row.get("title"),
    })
}
