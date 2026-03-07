#![allow(dead_code)]
//! Shared helpers for DB integration tests.

use breakfast::{db, models::*};
pub use uuid::Uuid;

// ---------------------------------------------------------------------------

/// Build a `deadpool_postgres::Pool` pointing at the local Docker postgres (no TLS).
///
/// Reads `TEST_DB_PORT` from the environment (default: 5432) so that
/// `make test-integration` can point at the isolated test container on 5433.
pub async fn test_pool() -> deadpool_postgres::Pool {
    let db_port: u16 = std::env::var("TEST_DB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(5432);

    let mut cfg = deadpool_postgres::Config::new();
    cfg.user = Some("actix".to_string());
    cfg.password = Some("actix".to_string());
    cfg.dbname = Some("actix".to_string());
    cfg.host = Some("localhost".to_string());
    cfg.port = Some(db_port);
    cfg.create_pool(
        Some(deadpool_postgres::Runtime::Tokio1),
        tokio_postgres::NoTls,
    )
    .expect("failed to create test pool")
}

pub async fn test_client() -> deadpool_postgres::Client {
    let pool = test_pool().await;
    pool.get().await.expect("failed to get test client")
}

/// Generate a unique email that won't collide across parallel tests.
pub fn unique_email() -> String {
    format!("dbtest-{}@test.local", Uuid::now_v7())
}

/// Generate a unique team name.
pub fn unique_team_name() -> String {
    format!("Team-{}", Uuid::now_v7())
}

/// Ensure the four default roles exist (Admin, Team Admin, Member, Guest).
/// Returns them as a map of title → role_id.
pub async fn ensure_roles(
    client: &deadpool_postgres::Client,
) -> std::collections::HashMap<String, Uuid> {
    let roles = db::seed_default_roles(client)
        .await
        .expect("seed_default_roles should succeed");
    roles.into_iter().map(|r| (r.title, r.role_id)).collect()
}

/// Create a test user with a unique email.
pub async fn create_test_user(client: &deadpool_postgres::Client) -> UserEntry {
    db::create_user(
        client,
        CreateUserEntry {
            firstname: "Test".to_string(),
            lastname: "User".to_string(),
            email: unique_email(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .expect("create_test_user should succeed")
}

/// Create a test team with a unique name.
pub async fn create_test_team(client: &deadpool_postgres::Client) -> TeamEntry {
    db::create_team(
        client,
        CreateTeamEntry {
            tname: unique_team_name(),
            descr: Some("Test team".to_string()),
        },
    )
    .await
    .expect("create_test_team should succeed")
}

/// Create a full admin setup: roles + user + team + Admin membership.
/// Returns (user, team, roles_map).
pub async fn create_admin_setup(
    client: &mut deadpool_postgres::Client,
) -> (
    UserEntry,
    TeamEntry,
    std::collections::HashMap<String, Uuid>,
) {
    let roles = ensure_roles(client).await;
    let user = create_test_user(client).await;
    let team = create_test_team(client).await;
    let admin_role_id = roles["Admin"];
    db::add_team_member(client, team.team_id, user.user_id, admin_role_id)
        .await
        .expect("add admin membership should succeed");
    (user, team, roles)
}

/// Create a membership graph for RBAC testing: admin user (Admin role),
/// team-admin user (Team Admin role), member user (Member role), and
/// optionally a second team with the team-admin as Member.
/// Returns (admin_user, team_admin_user, member_user, team1, Option<team2>, roles_map).
#[allow(clippy::type_complexity)]
pub async fn create_rbac_setup(
    client: &mut deadpool_postgres::Client,
) -> (
    UserEntry,
    UserEntry,
    UserEntry,
    TeamEntry,
    TeamEntry,
    std::collections::HashMap<String, Uuid>,
) {
    let roles = ensure_roles(client).await;
    let admin = create_test_user(client).await;
    let team_admin = create_test_user(client).await;
    let member = create_test_user(client).await;
    let team1 = create_test_team(client).await;
    let team2 = create_test_team(client).await;

    db::add_team_member(client, team1.team_id, admin.user_id, roles["Admin"])
        .await
        .expect("add admin");
    db::add_team_member(
        client,
        team1.team_id,
        team_admin.user_id,
        roles["Team Admin"],
    )
    .await
    .expect("add team admin");
    db::add_team_member(client, team1.team_id, member.user_id, roles["Member"])
        .await
        .expect("add member");
    // team_admin is also a Member of team2
    db::add_team_member(client, team2.team_id, team_admin.user_id, roles["Member"])
        .await
        .expect("add team admin to team2 as member");

    (admin, team_admin, member, team1, team2, roles)
}
