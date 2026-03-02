use crate::errors::Error;
use crate::models::*;
use deadpool_postgres::Client;
use uuid::Uuid;

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
        user_id: row.try_get("user_id").map_err(Error::Db)?,
        firstname: row.try_get("firstname").map_err(Error::Db)?,
        lastname: row.try_get("lastname").map_err(Error::Db)?,
        email: row.try_get("email").map_err(Error::Db)?,
        title: row.try_get("title").map_err(Error::Db)?,
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
        user_id: row.try_get("user_id").map_err(Error::Db)?,
        firstname: row.try_get("firstname").map_err(Error::Db)?,
        lastname: row.try_get("lastname").map_err(Error::Db)?,
        email: row.try_get("email").map_err(Error::Db)?,
        title: row.try_get("title").map_err(Error::Db)?,
    };

    tx.commit().await.map_err(Error::Db)?;

    Ok(result)
}
