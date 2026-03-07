//! Integration tests for `db.rs` — tests every database function directly.
//!
//! These tests require a running PostgreSQL instance initialized via Refinery
//! migrations. Each test creates its own data using unique names/emails to
//! support parallel execution.
//! Run them via:
//!   make test-integration
//!
//! Or manually:
//!   docker compose up -d postgres && docker compose run --rm postgres-setup
//!   TEST_DB_PORT=5433 cargo test --test db_tests -- --ignored

use argon2::password_hash::PasswordVerifier;
use breakfast::{db, models::*};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `deadpool_postgres::Pool` pointing at the local Docker postgres (no TLS).
///
/// Reads `TEST_DB_PORT` from the environment (default: 5432) so that
/// `make test-integration` can point at the isolated test container on 5433.
async fn test_pool() -> deadpool_postgres::Pool {
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

async fn test_client() -> deadpool_postgres::Client {
    let pool = test_pool().await;
    pool.get().await.expect("failed to get test client")
}

/// Generate a unique email that won't collide across parallel tests.
fn unique_email() -> String {
    format!("dbtest-{}@test.local", Uuid::now_v7())
}

/// Generate a unique team name.
fn unique_team_name() -> String {
    format!("Team-{}", Uuid::now_v7())
}

/// Ensure the four default roles exist (Admin, Team Admin, Member, Guest).
/// Returns them as a map of title → role_id.
async fn ensure_roles(
    client: &deadpool_postgres::Client,
) -> std::collections::HashMap<String, Uuid> {
    let roles = db::seed_default_roles(client)
        .await
        .expect("seed_default_roles should succeed");
    roles.into_iter().map(|r| (r.title, r.role_id)).collect()
}

/// Create a test user with a unique email.
async fn create_test_user(client: &deadpool_postgres::Client) -> UserEntry {
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
async fn create_test_team(client: &deadpool_postgres::Client) -> TeamEntry {
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
async fn create_admin_setup(
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
async fn create_rbac_setup(
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

// ===========================================================================
// Group 1: Health / connectivity
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn check_db_succeeds() {
    let client = test_client().await;
    db::check_db(&client)
        .await
        .expect("check_db should succeed on a healthy DB");
}

// ===========================================================================
// Group 2: User CRUD
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_user_returns_entry_with_correct_fields() {
    let client = test_client().await;
    let email = unique_email();

    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "DbTest".to_string(),
            lastname: "User".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .expect("create_user should succeed");

    assert_eq!(user.firstname, "DbTest");
    assert_eq!(user.lastname, "User");
    assert_eq!(user.email, email);
    assert!(!user.user_id.is_nil(), "should have a generated UUID");

    // Cleanup
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_user_by_id_returns_matching_user() {
    let client = test_client().await;
    let email = unique_email();

    let created = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "GetById".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let fetched = db::get_user(&client, created.user_id)
        .await
        .expect("get_user should succeed");
    assert_eq!(fetched.user_id, created.user_id);
    assert_eq!(fetched.email, email);
    assert_eq!(fetched.firstname, "GetById");

    // Cleanup
    db::delete_user(&client, created.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_user_by_email_returns_update_user_entry() {
    let client = test_client().await;
    let email = unique_email();

    let created = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "ByEmail".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let fetched = db::get_user_by_email(&client, &email)
        .await
        .expect("get_user_by_email should succeed");
    assert_eq!(fetched.user_id, created.user_id);
    assert_eq!(fetched.email, email);
    // Password should be an argon2 hash, not the plaintext
    assert!(
        fetched.password.starts_with("$argon2"),
        "password should be hashed"
    );

    // Cleanup
    db::delete_user(&client, created.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_users_returns_created_data() {
    let client = test_client().await;
    let user = create_test_user(&client).await;
    let (users, total) = db::get_users(&client, 100, 0)
        .await
        .expect("get_users should succeed");
    assert!(total >= 1, "should have at least 1 user, got {}", total);
    assert!(users.iter().any(|u| u.user_id == user.user_id));
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_user_without_password_preserves_hash() {
    let client = test_client().await;
    let email = unique_email();

    let created = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "Before".to_string(),
            lastname: "Update".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    // Get the original hash
    let original = db::get_user_by_email(&client, &email).await.unwrap();
    let original_hash = original.password.clone();

    // Update without password
    let updated = db::update_user(
        &client,
        created.user_id,
        UpdateUserRequest {
            firstname: "After".to_string(),
            lastname: "Update".to_string(),
            email: email.clone(),
            password: None,
            current_password: None,
        },
    )
    .await
    .expect("update_user should succeed");

    assert_eq!(updated.firstname, "After");

    // Verify password hash is unchanged
    let refreshed = db::get_user_by_email(&client, &email).await.unwrap();
    assert_eq!(refreshed.password, original_hash);

    // Cleanup
    db::delete_user(&client, created.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_user_with_password_changes_hash() {
    let client = test_client().await;
    let email = unique_email();

    let created = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "PwdChange".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "originalpassword".to_string(),
        },
    )
    .await
    .unwrap();

    let original = db::get_user_by_email(&client, &email).await.unwrap();
    let original_hash = original.password.clone();

    // Update with a new password
    db::update_user(
        &client,
        created.user_id,
        UpdateUserRequest {
            firstname: "PwdChange".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: Some("newpassword456".to_string()),
            current_password: None,
        },
    )
    .await
    .expect("update_user with password should succeed");

    let refreshed = db::get_user_by_email(&client, &email).await.unwrap();
    assert_ne!(refreshed.password, original_hash, "hash should change");
    assert!(
        refreshed.password.starts_with("$argon2"),
        "new password should be hashed"
    );

    // Cleanup
    db::delete_user(&client, created.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_user_returns_true_then_false() {
    let client = test_client().await;

    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "Delete".to_string(),
            lastname: "Me".to_string(),
            email: unique_email(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let deleted = db::delete_user(&client, user.user_id).await.unwrap();
    assert!(deleted, "first delete should return true");

    let deleted_again = db::delete_user(&client, user.user_id).await.unwrap();
    assert!(!deleted_again, "second delete should return false");
}

#[actix_web::test]
#[ignore]
async fn delete_user_by_email_returns_true_then_false() {
    let client = test_client().await;
    let email = unique_email();

    db::create_user(
        &client,
        CreateUserEntry {
            firstname: "DelByEmail".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let deleted = db::delete_user_by_email(&client, &email).await.unwrap();
    assert!(deleted);

    let deleted_again = db::delete_user_by_email(&client, &email).await.unwrap();
    assert!(!deleted_again);
}

#[actix_web::test]
#[ignore]
async fn get_user_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::get_user(&client, Uuid::now_v7()).await;
    assert!(result.is_err(), "nonexistent user should return error");
}

#[actix_web::test]
#[ignore]
async fn create_duplicate_user_returns_error() {
    let client = test_client().await;
    let email = unique_email();

    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "Dup".to_string(),
            lastname: "User".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let result = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "Dup2".to_string(),
            lastname: "User2".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await;

    assert!(result.is_err(), "duplicate email should fail");

    // Cleanup
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 3: Team CRUD
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_team_returns_entry() {
    let client = test_client().await;
    let tname = format!("dbtest-team-{}", Uuid::now_v7());

    let team = db::create_team(
        &client,
        CreateTeamEntry {
            tname: tname.clone(),
            descr: Some("test description".to_string()),
        },
    )
    .await
    .expect("create_team should succeed");

    assert_eq!(team.tname, tname);
    assert_eq!(team.descr, Some("test description".to_string()));
    assert!(!team.team_id.is_nil());

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_team_by_id() {
    let client = test_client().await;
    let tname = format!("dbtest-team-{}", Uuid::now_v7());

    let created = db::create_team(
        &client,
        CreateTeamEntry {
            tname: tname.clone(),
            descr: None,
        },
    )
    .await
    .unwrap();

    let fetched = db::get_team(&client, created.team_id)
        .await
        .expect("get_team should succeed");
    assert_eq!(fetched.team_id, created.team_id);
    assert_eq!(fetched.tname, tname);

    // Cleanup
    db::delete_team(&client, created.team_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_teams_returns_created_data() {
    let client = test_client().await;
    let team = create_test_team(&client).await;
    let (teams, total) = db::get_teams(&client, 100, 0)
        .await
        .expect("get_teams should succeed");
    assert!(total >= 1, "should have at least 1 team, got {}", total);
    assert!(teams.iter().any(|t| t.team_id == team.team_id));
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_team_changes_fields() {
    let client = test_client().await;
    let tname = format!("dbtest-team-{}", Uuid::now_v7());

    let created = db::create_team(
        &client,
        CreateTeamEntry {
            tname: tname.clone(),
            descr: Some("original".to_string()),
        },
    )
    .await
    .unwrap();

    let updated_name = format!("updated-{}", Uuid::now_v7());
    let updated = db::update_team(
        &client,
        created.team_id,
        UpdateTeamEntry {
            tname: updated_name.clone(),
            descr: Some("updated desc".to_string()),
        },
    )
    .await
    .expect("update_team should succeed");

    assert_eq!(updated.tname, updated_name);
    assert_eq!(updated.descr, Some("updated desc".to_string()));

    // Cleanup
    db::delete_team(&client, created.team_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_team_returns_true_then_false() {
    let client = test_client().await;
    let tname = format!("dbtest-team-{}", Uuid::now_v7());

    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let deleted = db::delete_team(&client, team.team_id).await.unwrap();
    assert!(deleted);

    let deleted_again = db::delete_team(&client, team.team_id).await.unwrap();
    assert!(!deleted_again);
}

#[actix_web::test]
#[ignore]
async fn get_team_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::get_team(&client, Uuid::now_v7()).await;
    assert!(result.is_err());
}

// ===========================================================================
// Group 4: Role CRUD
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_role_returns_entry() {
    let client = test_client().await;
    let title = format!("dbtest-role-{}", Uuid::now_v7());

    let role = db::create_role(
        &client,
        CreateRoleEntry {
            title: title.clone(),
        },
    )
    .await
    .expect("create_role should succeed");

    assert_eq!(role.title, title);
    assert!(!role.role_id.is_nil());

    // Cleanup
    db::delete_role(&client, role.role_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_role_by_id() {
    let client = test_client().await;
    let title = format!("dbtest-role-{}", Uuid::now_v7());

    let created = db::create_role(
        &client,
        CreateRoleEntry {
            title: title.clone(),
        },
    )
    .await
    .unwrap();

    let fetched = db::get_role(&client, created.role_id)
        .await
        .expect("get_role should succeed");
    assert_eq!(fetched.role_id, created.role_id);
    assert_eq!(fetched.title, title);

    // Cleanup
    db::delete_role(&client, created.role_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_roles_returns_created_data() {
    let client = test_client().await;
    ensure_roles(&client).await;
    let (_roles, total) = db::get_roles(&client, 100, 0)
        .await
        .expect("get_roles should succeed");
    assert!(
        total >= 4,
        "should have at least 4 roles after seeding, got {}",
        total
    );
}

#[actix_web::test]
#[ignore]
async fn update_role_changes_title() {
    let client = test_client().await;
    let title = format!("dbtest-role-{}", Uuid::now_v7());

    let created = db::create_role(
        &client,
        CreateRoleEntry {
            title: title.clone(),
        },
    )
    .await
    .unwrap();

    let new_title = format!("updated-role-{}", Uuid::now_v7());
    let updated = db::update_role(
        &client,
        created.role_id,
        UpdateRoleEntry {
            title: new_title.clone(),
        },
    )
    .await
    .expect("update_role should succeed");

    assert_eq!(updated.title, new_title);

    // Cleanup
    db::delete_role(&client, created.role_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_role_returns_true_then_false() {
    let client = test_client().await;
    let title = format!("dbtest-role-{}", Uuid::now_v7());

    let role = db::create_role(&client, CreateRoleEntry { title })
        .await
        .unwrap();

    let deleted = db::delete_role(&client, role.role_id).await.unwrap();
    assert!(deleted);

    let deleted_again = db::delete_role(&client, role.role_id).await.unwrap();
    assert!(!deleted_again);
}

#[actix_web::test]
#[ignore]
async fn get_role_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::get_role(&client, Uuid::now_v7()).await;
    assert!(result.is_err());
}

// ===========================================================================
// Group 5: Item CRUD
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_item_returns_entry_with_price() {
    let client = test_client().await;
    let descr = format!("dbtest-item-{}", Uuid::now_v7());

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::from_str("12.50").unwrap(),
        },
    )
    .await
    .expect("create_item should succeed");

    assert_eq!(item.descr, descr);
    assert_eq!(item.price, Decimal::from_str("12.50").unwrap());
    assert!(!item.item_id.is_nil());

    // Cleanup
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn create_item_with_null_price() {
    let client = test_client().await;
    let descr = format!("dbtest-item-nullprice-{}", Uuid::now_v7());

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::ZERO,
        },
    )
    .await
    .expect("create_item with zero price should succeed");

    assert_eq!(item.price, Decimal::ZERO);

    // Cleanup
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_item_by_id() {
    let client = test_client().await;
    let descr = format!("dbtest-item-{}", Uuid::now_v7());

    let created = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::from_str("5.00").unwrap(),
        },
    )
    .await
    .unwrap();

    let fetched = db::get_item(&client, created.item_id)
        .await
        .expect("get_item should succeed");
    assert_eq!(fetched.item_id, created.item_id);
    assert_eq!(fetched.descr, descr);

    // Cleanup
    db::delete_item(&client, created.item_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_items_returns_created_data() {
    let client = test_client().await;
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-item-{}", Uuid::now_v7()),
            price: Decimal::new(900, 2),
        },
    )
    .await
    .expect("create_item should succeed");
    let (items, total) = db::get_items(&client, 100, 0)
        .await
        .expect("get_items should succeed");
    assert!(total >= 1, "should have at least 1 item, got {}", total);
    assert!(items.iter().any(|i| i.item_id == item.item_id));
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_item_changes_fields() {
    let client = test_client().await;
    let descr = format!("dbtest-item-{}", Uuid::now_v7());

    let created = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::from_str("1.00").unwrap(),
        },
    )
    .await
    .unwrap();

    let new_descr = format!("updated-item-{}", Uuid::now_v7());
    let updated = db::update_item(
        &client,
        created.item_id,
        UpdateItemEntry {
            descr: new_descr.clone(),
            price: Decimal::from_str("99.99").unwrap(),
        },
    )
    .await
    .expect("update_item should succeed");

    assert_eq!(updated.descr, new_descr);
    assert_eq!(updated.price, Decimal::from_str("99.99").unwrap());

    // Cleanup
    db::delete_item(&client, created.item_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_item_returns_true_then_false() {
    let client = test_client().await;
    let descr = format!("dbtest-item-{}", Uuid::now_v7());

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr,
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let deleted = db::delete_item(&client, item.item_id).await.unwrap();
    assert!(deleted);

    let deleted_again = db::delete_item(&client, item.item_id).await.unwrap();
    assert!(!deleted_again);
}

#[actix_web::test]
#[ignore]
async fn get_item_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::get_item(&client, Uuid::now_v7()).await;
    assert!(result.is_err());
}

#[actix_web::test]
#[ignore]
async fn create_duplicate_item_returns_error() {
    let client = test_client().await;
    let descr = format!("dbtest-item-dup-{}", Uuid::now_v7());

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let result = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::ZERO,
        },
    )
    .await;

    assert!(result.is_err(), "duplicate item descr should fail");

    // Cleanup
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 6: Team order CRUD
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_team_order_returns_entry() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()),
            pickup_user_id: None,
        },
    )
    .await
    .expect("create_team_order should succeed");

    assert_eq!(order.teamorders_team_id, team.team_id);
    assert_eq!(
        order.duedate,
        Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap())
    );
    assert!(!order.closed);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn create_team_order_with_user_id() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .expect("create_team_order with user should succeed");

    assert_eq!(order.teamorders_user_id, user.user_id);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_team_orders_returns_created_data() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let (orders, total) = db::get_team_orders(&client, team.team_id, 100, 0)
        .await
        .expect("get_team_orders should succeed");
    assert_eq!(total, 1);
    assert_eq!(orders[0].teamorders_id, order.teamorders_id);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_team_order_by_id() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let fetched = db::get_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("get_team_order should succeed");
    assert_eq!(fetched.teamorders_id, order.teamorders_id);
    assert_eq!(fetched.teamorders_team_id, team.team_id);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_team_order_changes_fields() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let updated = db::update_team_order(
        &client,
        team.team_id,
        order.teamorders_id,
        UpdateTeamOrderEntry {
            duedate: Some(Some(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap())),
            closed: Some(true),
            pickup_user_id: None,
        },
    )
    .await
    .expect("update_team_order should succeed");

    assert_eq!(
        updated.duedate,
        Some(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap())
    );
    assert!(updated.closed);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_team_order_returns_true_then_false() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let deleted = db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .unwrap();
    assert!(deleted);

    let deleted_again = db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .unwrap();
    assert!(!deleted_again);

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_team_orders_bulk() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    // Create two orders
    db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let count = db::delete_team_orders(&client, team.team_id)
        .await
        .expect("delete_team_orders should succeed");
    assert_eq!(count, 2, "should delete both orders");

    // Deleting again should return 0
    let count_again = db::delete_team_orders(&client, team.team_id).await.unwrap();
    assert_eq!(count_again, 0);

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 7: Order items CRUD (items within a team order)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_order_item_returns_entry() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    // Create an item and order to work with
    let item_descr = format!("dbtest-oitem-{}", Uuid::now_v7());
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: item_descr,
            price: Decimal::from_str("2.00").unwrap(),
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let order_item = db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 5,
        },
    )
    .await
    .expect("create_order_item should succeed");

    assert_eq!(order_item.orders_teamorders_id, order.teamorders_id);
    assert_eq!(order_item.orders_item_id, item.item_id);
    assert_eq!(order_item.orders_team_id, team.team_id);
    assert_eq!(order_item.amt, 5);

    // Cleanup (cascade: deleting order deletes order items)
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_order_items_returns_list() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let item1 = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-oilist1-{}", Uuid::now_v7()),
            price: Decimal::from_str("1.00").unwrap(),
        },
    )
    .await
    .unwrap();
    let item2 = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-oilist2-{}", Uuid::now_v7()),
            price: Decimal::from_str("2.00").unwrap(),
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item1.item_id,
            amt: 1,
        },
    )
    .await
    .unwrap();
    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item2.item_id,
            amt: 2,
        },
    )
    .await
    .unwrap();

    let (items, total) = db::get_order_items(&client, order.teamorders_id, team.team_id, 100, 0)
        .await
        .expect("get_order_items should succeed");
    assert_eq!(total, 2);
    assert_eq!(items.len(), 2);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_item(&client, item1.item_id)
        .await
        .expect("cleanup");
    db::delete_item(&client, item2.item_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_order_item_by_id() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item_descr = format!("dbtest-getoi-{}", Uuid::now_v7());
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: item_descr,
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 3,
        },
    )
    .await
    .unwrap();

    let fetched = db::get_order_item(&client, order.teamorders_id, item.item_id, team.team_id)
        .await
        .expect("get_order_item should succeed");
    assert_eq!(fetched.orders_item_id, item.item_id);
    assert_eq!(fetched.amt, 3);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_order_item_changes_amt() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item_descr = format!("dbtest-updoi-{}", Uuid::now_v7());
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: item_descr,
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 1,
        },
    )
    .await
    .unwrap();

    let updated = db::update_order_item(
        &mut client,
        order.teamorders_id,
        item.item_id,
        team.team_id,
        UpdateOrderEntry { amt: 42 },
    )
    .await
    .expect("update_order_item should succeed");

    assert_eq!(updated.amt, 42);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_order_item_returns_true_then_false() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item_descr = format!("dbtest-deloi-{}", Uuid::now_v7());
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: item_descr,
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 1,
        },
    )
    .await
    .unwrap();

    let deleted =
        db::delete_order_item(&mut client, order.teamorders_id, item.item_id, team.team_id)
            .await
            .unwrap();
    assert!(deleted);

    let deleted_again =
        db::delete_order_item(&mut client, order.teamorders_id, item.item_id, team.team_id)
            .await
            .unwrap();
    assert!(!deleted_again);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn duplicate_order_item_returns_error() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item_descr = format!("dbtest-dupoi-{}", Uuid::now_v7());
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: item_descr,
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 1,
        },
    )
    .await
    .unwrap();

    // Adding the same item again should fail (PK violation)
    let result = db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 2,
        },
    )
    .await;

    assert!(result.is_err(), "duplicate order item should fail");

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 8: Memberof / RBAC queries
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn is_admin_returns_true_for_admin() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;

    let result = db::is_admin(&client, admin.user_id)
        .await
        .expect("is_admin should succeed");
    assert!(result, "user with Admin role should be recognized as admin");

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, admin.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_admin_returns_false_for_member() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let result = db::is_admin(&client, member.user_id)
        .await
        .expect("is_admin should succeed");
    assert!(!result, "member should not be admin");

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_admin_returns_false_for_nonexistent_user() {
    let client = test_client().await;
    let result = db::is_admin(&client, Uuid::now_v7())
        .await
        .expect("is_admin should succeed even for unknown user");
    assert!(!result);
}

#[actix_web::test]
#[ignore]
async fn is_admin_or_team_admin_returns_true_for_admin() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;

    let result = db::is_admin_or_team_admin(&client, admin.user_id)
        .await
        .expect("is_admin_or_team_admin should succeed");
    assert!(result);

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, admin.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_admin_or_team_admin_returns_true_for_team_admin() {
    let mut client = test_client().await;
    let (_admin, ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let result = db::is_admin_or_team_admin(&client, ta.user_id)
        .await
        .expect("is_admin_or_team_admin should succeed");
    assert!(result, "Team Admin should return true");

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, ta.user_id).await.expect("cleanup");
    db::delete_user(&client, _member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_admin_or_team_admin_returns_false_for_member() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let result = db::is_admin_or_team_admin(&client, member.user_id)
        .await
        .expect("is_admin_or_team_admin should succeed");
    assert!(!result, "Member should return false");

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_correct_role_for_admin() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;

    let role = db::get_member_role(&client, team.team_id, admin.user_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, Some("Admin".to_string()));

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, admin.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_correct_role_for_team_admin() {
    let mut client = test_client().await;
    let (_admin, ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let role = db::get_member_role(&client, team1.team_id, ta.user_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, Some("Team Admin".to_string()));

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, ta.user_id).await.expect("cleanup");
    db::delete_user(&client, _member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_member_for_regular_user() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let role = db::get_member_role(&client, team1.team_id, member.user_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, Some("Member".to_string()));

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_none_for_non_member() {
    let mut client = test_client().await;
    let (_admin, _ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    // member is in team1 but not team2
    let role = db::get_member_role(&client, team2.team_id, _member.user_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, None);

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_admin_of_user_returns_true_for_shared_team() {
    let mut client = test_client().await;
    let (_admin, ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    // ta is Team Admin of team1, member is Member of team1
    let result = db::is_team_admin_of_user(&client, ta.user_id, member.user_id)
        .await
        .expect("is_team_admin_of_user should succeed");
    assert!(
        result,
        "Team Admin should be recognized as team admin of a member in the same team"
    );

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, ta.user_id).await.expect("cleanup");
    db::delete_user(&client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_admin_of_user_returns_false_for_non_shared_team() {
    let mut client = test_client().await;
    let (_admin, ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    // Create a user who is NOT in any of ta's teams
    let outsider = create_test_user(&client).await;

    let result = db::is_team_admin_of_user(&client, ta.user_id, outsider.user_id)
        .await
        .expect("is_team_admin_of_user should succeed");
    assert!(
        !result,
        "Team Admin should not be team admin of a user outside their teams"
    );

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, ta.user_id).await.expect("cleanup");
    db::delete_user(&client, _member.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, outsider.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_admin_of_user_returns_false_for_regular_member() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    // Create another member in team1
    let member2 = create_test_user(&client).await;
    db::add_team_member(
        &mut client,
        team1.team_id,
        member2.user_id,
        _roles["Member"],
    )
    .await
    .expect("add member2");

    // member is not a Team Admin, so should return false
    let result = db::is_team_admin_of_user(&client, member.user_id, member2.user_id)
        .await
        .expect("is_team_admin_of_user should succeed");
    assert!(
        !result,
        "regular member should not be team admin of another member"
    );

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, member.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, member2.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 9: Team member management
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn add_team_member_returns_users_in_team() {
    let mut client = test_client().await;

    // Create a fresh team and user to avoid conflicts with seed data
    let tname = format!("dbtest-member-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let email = unique_email();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "NewMember".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];

    let result = db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id)
        .await
        .expect("add_team_member should succeed");

    assert_eq!(result.user_id, user.user_id);
    assert_eq!(result.firstname, "NewMember");
    assert_eq!(result.title, "Member");

    // Cleanup (cascade: deleting team removes memberof)
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup user");
}

#[actix_web::test]
#[ignore]
async fn remove_team_member_returns_true_then_false() {
    let mut client = test_client().await;

    let tname = format!("dbtest-rmmember-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let email = unique_email();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "RemoveMe".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];
    db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id)
        .await
        .unwrap();

    let removed = db::remove_team_member(&client, team.team_id, user.user_id)
        .await
        .unwrap();
    assert!(removed);

    let removed_again = db::remove_team_member(&client, team.team_id, user.user_id)
        .await
        .unwrap();
    assert!(!removed_again);

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup user");
}

#[actix_web::test]
#[ignore]
async fn update_member_role_changes_title() {
    let mut client = test_client().await;

    let tname = format!("dbtest-updrole-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let email = unique_email();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "RoleChange".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];
    let guest_role_id = roles["Guest"];

    db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id)
        .await
        .unwrap();

    let updated = db::update_member_role(&mut client, team.team_id, user.user_id, guest_role_id)
        .await
        .expect("update_member_role should succeed");

    assert_eq!(updated.title, "Guest");
    assert_eq!(updated.user_id, user.user_id);

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup user");
}

#[actix_web::test]
#[ignore]
async fn update_member_role_returns_not_found_for_non_member() {
    let mut client = test_client().await;

    let tname = format!("dbtest-rolefail-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];
    let random_user_id = Uuid::now_v7();

    let result =
        db::update_member_role(&mut client, team.team_id, random_user_id, member_role_id).await;

    assert!(result.is_err(), "should fail for non-member");
    let err_msg = match result {
        Err(e) => e.to_string(),
        Ok(_) => panic!("expected error for non-member"),
    };
    assert!(
        err_msg.contains("member not found"),
        "error should mention member not found, got: {}",
        err_msg
    );

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup team");
}

#[actix_web::test]
#[ignore]
async fn add_duplicate_team_member_returns_error() {
    let mut client = test_client().await;

    let tname = format!("dbtest-dupmember-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let email = unique_email();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "DupMember".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];

    db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id)
        .await
        .unwrap();

    // Adding the same user again should fail (PK violation)
    let result = db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id).await;
    assert!(result.is_err(), "duplicate member should fail");

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup user");
}

#[actix_web::test]
#[ignore]
async fn add_team_member_with_nonexistent_user_returns_error() {
    let mut client = test_client().await;

    let tname = format!("dbtest-baduser-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];
    let fake_user_id = Uuid::now_v7();

    let result = db::add_team_member(&mut client, team.team_id, fake_user_id, member_role_id).await;
    assert!(
        result.is_err(),
        "adding nonexistent user should fail with FK violation"
    );

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup team");
}

#[actix_web::test]
#[ignore]
async fn add_team_member_with_nonexistent_role_returns_error() {
    let mut client = test_client().await;

    let tname = format!("dbtest-badrole-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let email = unique_email();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "BadRole".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let fake_role_id = Uuid::now_v7();

    let result = db::add_team_member(&mut client, team.team_id, user.user_id, fake_role_id).await;
    assert!(
        result.is_err(),
        "adding member with nonexistent role should fail with FK violation"
    );

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup user");
}

// ===========================================================================
// Group 10: User/Team relationship queries
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn get_user_teams_returns_memberships() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;

    let (teams, _) = db::get_user_teams(&client, admin.user_id, 100, 0)
        .await
        .expect("get_user_teams should succeed");
    assert!(
        !teams.is_empty(),
        "admin should have at least 1 team membership"
    );
    // Verify the structure includes the expected fields
    let first = &teams[0];
    assert!(!first.tname.is_empty());
    assert!(!first.title.is_empty());
    assert!(!first.firstname.is_empty());

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, admin.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_user_teams_returns_empty_for_no_memberships() {
    let client = test_client().await;
    let email = unique_email();

    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "NoTeams".to_string(),
            lastname: "User".to_string(),
            email,
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let (teams, _) = db::get_user_teams(&client, user.user_id, 100, 0)
        .await
        .expect("get_user_teams should succeed for user with no teams");
    assert!(
        teams.is_empty(),
        "user with no memberships should return []"
    );

    // Cleanup
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_team_users_returns_members() {
    let mut client = test_client().await;
    let (_admin, _ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let (users, _) = db::get_team_users(&client, team1.team_id, 100, 0)
        .await
        .expect("get_team_users should succeed");
    assert!(
        users.len() >= 3,
        "team1 should have at least 3 members, got {}",
        users.len()
    );
    // Verify the structure
    let first = &users[0];
    assert!(!first.firstname.is_empty());
    assert!(!first.email.is_empty());
    assert!(!first.title.is_empty());

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_team_users_returns_empty_for_empty_team() {
    let client = test_client().await;
    let tname = format!("dbtest-empty-{}", Uuid::now_v7());

    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let (users, _) = db::get_team_users(&client, team.team_id, 100, 0)
        .await
        .expect("get_team_users should succeed for empty team");
    assert!(users.is_empty(), "empty team should return []");

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_user_teams_for_multi_team_user() {
    let mut client = test_client().await;
    // create_rbac_setup puts ta in team1 (Team Admin) and team2 (Member)
    let (_admin, ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let (teams, _) = db::get_user_teams(&client, ta.user_id, 100, 0)
        .await
        .expect("get_user_teams should succeed");
    assert!(
        teams.len() >= 2,
        "team admin should be in at least 2 teams, got {}",
        teams.len()
    );

    // Verify different roles in different teams
    let team_names: Vec<&str> = teams.iter().map(|t| t.tname.as_str()).collect();
    assert!(
        team_names.contains(&team1.tname.as_str()),
        "should include team1"
    );
    assert!(
        team_names.contains(&team2.tname.as_str()),
        "should include team2"
    );

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, ta.user_id).await.expect("cleanup");
    db::delete_user(&client, _member.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 11: Token blacklist
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn revoke_token_db_and_check_revoked() {
    let client = test_client().await;
    let jti = Uuid::now_v7();
    let expires_at = Utc::now() + chrono::Duration::try_hours(1).unwrap();

    db::revoke_token_db(&client, jti, expires_at)
        .await
        .expect("revoke_token_db should succeed");

    let revoked = db::is_token_revoked_db(&client, jti)
        .await
        .expect("is_token_revoked_db should succeed");
    assert!(revoked, "token should be marked as revoked");
}

#[actix_web::test]
#[ignore]
async fn is_token_revoked_db_returns_false_for_unknown() {
    let client = test_client().await;
    let revoked = db::is_token_revoked_db(&client, Uuid::now_v7())
        .await
        .expect("is_token_revoked_db should succeed");
    assert!(!revoked, "unknown jti should not be revoked");
}

#[actix_web::test]
#[ignore]
async fn revoke_token_db_is_idempotent() {
    let client = test_client().await;
    let jti = Uuid::now_v7();
    let expires_at = Utc::now() + chrono::Duration::try_hours(1).unwrap();

    // Revoking twice should not error (ON CONFLICT DO NOTHING)
    db::revoke_token_db(&client, jti, expires_at)
        .await
        .expect("first revoke should succeed");
    db::revoke_token_db(&client, jti, expires_at)
        .await
        .expect("second revoke should also succeed (idempotent)");

    let revoked = db::is_token_revoked_db(&client, jti).await.unwrap();
    assert!(revoked);
}

#[actix_web::test]
#[ignore]
async fn cleanup_expired_tokens_removes_old_entries() {
    let client = test_client().await;
    let jti = Uuid::now_v7();
    // Use a well-past expiry to avoid timing edge cases
    let expires_at = Utc::now() - chrono::Duration::try_days(1).unwrap();

    db::revoke_token_db(&client, jti, expires_at)
        .await
        .expect("revoke should succeed");

    // Note: we do NOT assert the token exists before cleanup here because a
    // concurrent test (cleanup_expired_tokens_preserves_valid_entries) also
    // calls cleanup_expired_tokens, which may remove our already-expired
    // entry before we check.  The point of this test is that cleanup removes
    // expired entries — which we verify below.

    // Run cleanup — don't assert on global count since parallel tests may
    // insert/remove expired tokens concurrently
    db::cleanup_expired_tokens(&client)
        .await
        .expect("cleanup should succeed");

    // Verify our specific token was removed
    let revoked_after = db::is_token_revoked_db(&client, jti).await.unwrap();
    assert!(!revoked_after, "should be removed after cleanup");
}

#[actix_web::test]
#[ignore]
async fn cleanup_expired_tokens_preserves_valid_entries() {
    let client = test_client().await;
    let jti = Uuid::now_v7();
    // Set expires_at in the future
    let expires_at = Utc::now() + chrono::Duration::try_hours(24).unwrap();

    db::revoke_token_db(&client, jti, expires_at)
        .await
        .expect("revoke should succeed");

    // Run cleanup — should NOT remove this entry
    db::cleanup_expired_tokens(&client).await.unwrap();

    let revoked = db::is_token_revoked_db(&client, jti).await.unwrap();
    assert!(revoked, "future-expiry entry should survive cleanup");
}

// ===========================================================================
// Group 12: Error handling / edge cases
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn get_user_by_email_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::get_user_by_email(&client, "nonexistent@nobody.test").await;
    assert!(result.is_err(), "nonexistent email should return error");
}

#[actix_web::test]
#[ignore]
async fn update_user_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::update_user(
        &client,
        Uuid::now_v7(),
        UpdateUserRequest {
            firstname: "Ghost".to_string(),
            lastname: "User".to_string(),
            email: "ghost@nobody.test".to_string(),
            password: None,
            current_password: None,
        },
    )
    .await;
    assert!(result.is_err(), "updating nonexistent user should fail");
}

#[actix_web::test]
#[ignore]
async fn update_team_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::update_team(
        &client,
        Uuid::now_v7(),
        UpdateTeamEntry {
            tname: "ghost".to_string(),
            descr: None,
        },
    )
    .await;
    assert!(result.is_err(), "updating nonexistent team should fail");
}

#[actix_web::test]
#[ignore]
async fn update_role_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::update_role(
        &client,
        Uuid::now_v7(),
        UpdateRoleEntry {
            title: "ghost".to_string(),
        },
    )
    .await;
    assert!(result.is_err(), "updating nonexistent role should fail");
}

#[actix_web::test]
#[ignore]
async fn update_item_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::update_item(
        &client,
        Uuid::now_v7(),
        UpdateItemEntry {
            descr: "ghost".to_string(),
            price: Decimal::ZERO,
        },
    )
    .await;
    assert!(result.is_err(), "updating nonexistent item should fail");
}

#[actix_web::test]
#[ignore]
async fn update_team_order_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::update_team_order(
        &client,
        Uuid::now_v7(),
        Uuid::now_v7(),
        UpdateTeamOrderEntry {
            duedate: None,
            closed: None,
            pickup_user_id: None,
        },
    )
    .await;
    assert!(
        result.is_err(),
        "updating nonexistent team order should fail"
    );
}

#[actix_web::test]
#[ignore]
async fn update_order_item_nonexistent_returns_error() {
    let mut client = test_client().await;
    let result = db::update_order_item(
        &mut client,
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        UpdateOrderEntry { amt: 1 },
    )
    .await;
    assert!(
        result.is_err(),
        "updating nonexistent order item should fail"
    );
}

#[actix_web::test]
#[ignore]
async fn get_team_order_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::get_team_order(&client, Uuid::now_v7(), Uuid::now_v7()).await;
    assert!(
        result.is_err(),
        "nonexistent team order should return error"
    );
}

#[actix_web::test]
#[ignore]
async fn get_order_item_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::get_order_item(&client, Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()).await;
    assert!(
        result.is_err(),
        "nonexistent order item should return error"
    );
}

// ===========================================================================
// Group 13: Timestamp and changed-column behavior
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_user_sets_timestamps() {
    let client = test_client().await;
    let email = unique_email();
    let before = Utc::now();

    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "Timestamp".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let after = Utc::now();

    // created and changed should be between before and after
    assert!(
        user.created >= before && user.created <= after,
        "created timestamp should be recent"
    );
    assert!(
        user.changed >= before && user.changed <= after,
        "changed timestamp should be recent"
    );

    // Cleanup
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_user_updates_changed_timestamp() {
    let client = test_client().await;
    let email = unique_email();

    let created = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "Original".to_string(),
            lastname: "Name".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let original_changed = created.changed;

    // Small delay to ensure timestamp difference
    actix_web::rt::time::sleep(std::time::Duration::from_millis(50)).await;

    let updated = db::update_user(
        &client,
        created.user_id,
        UpdateUserRequest {
            firstname: "Updated".to_string(),
            lastname: "Name".to_string(),
            email: email.clone(),
            password: None,
            current_password: None,
        },
    )
    .await
    .unwrap();

    assert!(
        updated.changed >= original_changed,
        "changed timestamp should be updated"
    );
    // created should remain the same
    assert_eq!(updated.created, created.created);

    // Cleanup
    db::delete_user(&client, created.user_id)
        .await
        .expect("cleanup");
}

// ---------------------------------------------------------------------------
// is_team_order_closed
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn is_team_order_closed_returns_false_for_open_order() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    // Create a new order (defaults to closed = false)
    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: Some(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()),
            pickup_user_id: None,
        },
    )
    .await
    .expect("should create team order");

    let closed = db::is_team_order_closed(&client, order.teamorders_id, team.team_id)
        .await
        .expect("should check closed status");
    assert!(!closed, "newly created order should not be closed");

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_order_closed_returns_true_for_closed_order() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    // Create a new order
    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: Some(NaiveDate::from_ymd_opt(2026, 12, 26).unwrap()),
            pickup_user_id: None,
        },
    )
    .await
    .expect("should create team order");

    // Close the order
    db::update_team_order(
        &client,
        team.team_id,
        order.teamorders_id,
        UpdateTeamOrderEntry {
            duedate: Some(Some(NaiveDate::from_ymd_opt(2026, 12, 26).unwrap())),
            closed: Some(true),
            pickup_user_id: None,
        },
    )
    .await
    .expect("should close the order");

    let closed = db::is_team_order_closed(&client, order.teamorders_id, team.team_id)
        .await
        .expect("should check closed status");
    assert!(closed, "updated order should be closed");

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_order_closed_returns_not_found_for_nonexistent_order() {
    let client = test_client().await;
    let team = create_test_team(&client).await;
    let fake_order_id = Uuid::now_v7();

    let result = db::is_team_order_closed(&client, fake_order_id, team.team_id).await;
    assert!(result.is_err(), "nonexistent order should return an error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found"),
        "error should mention 'not found', got: {}",
        err_msg
    );

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 11: check_team_access (#174)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn check_team_access_admin_in_own_team() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;

    let (is_admin, team_role) = db::check_team_access(&client, team.team_id, admin.user_id)
        .await
        .expect("check_team_access should succeed");
    assert!(is_admin, "admin should be recognized as global admin");
    assert_eq!(
        team_role,
        Some("Admin".to_string()),
        "admin should have Admin role in the team"
    );

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, admin.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn check_team_access_regular_member() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let (is_admin, team_role) = db::check_team_access(&client, team1.team_id, member.user_id)
        .await
        .expect("check_team_access should succeed");
    assert!(!is_admin, "regular member should not be global admin");
    assert_eq!(
        team_role,
        Some("Member".to_string()),
        "member should have Member role in the team"
    );

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn check_team_access_non_member() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    // member is in team1 but not team2
    let (is_admin, team_role) = db::check_team_access(&client, team2.team_id, member.user_id)
        .await
        .expect("check_team_access should succeed");
    assert!(!is_admin, "member should not be global admin");
    assert!(team_role.is_none(), "member should have no role in team2");

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn check_team_access_admin_in_unrelated_team() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;
    // Create a second team where admin is NOT a member
    let other_team = create_test_team(&client).await;

    let (is_admin, team_role) = db::check_team_access(&client, other_team.team_id, admin.user_id)
        .await
        .expect("check_team_access should succeed");
    assert!(is_admin, "admin should still be recognized as global admin");
    assert!(
        team_role.is_none(),
        "admin should have no role in unrelated team (not a member)"
    );

    // Cleanup
    db::delete_team(&client, other_team.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, admin.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 9: FK Cascade Behaviour
// ===========================================================================

/// Deleting a team cascades to memberof, teamorders, and orders.
#[actix_web::test]
#[ignore]
async fn delete_team_cascades_membership_and_orders() {
    let mut client = test_client().await;

    // Create isolated team, user, item
    let team = db::create_team(
        &client,
        CreateTeamEntry {
            tname: format!("dbtest-cascade-{}", Uuid::now_v7()),
            descr: None,
        },
    )
    .await
    .unwrap();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "Cascade".to_string(),
            lastname: "Test".to_string(),
            email: unique_email(),
            password: "password123".to_string(),
        },
    )
    .await
    .unwrap();
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-cascade-item-{}", Uuid::now_v7()),
            price: Decimal::from_str("3.00").unwrap(),
        },
    )
    .await
    .unwrap();

    // Add user as member
    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];
    db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id)
        .await
        .unwrap();

    // Create a team order with an order item
    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();
    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 2,
        },
    )
    .await
    .unwrap();

    // Delete the team — should cascade
    let deleted = db::delete_team(&client, team.team_id).await.unwrap();
    assert!(deleted);

    // Membership should be gone
    let (members, _) = db::get_team_users(&client, team.team_id, 100, 0)
        .await
        .unwrap();
    assert!(members.is_empty(), "membership should be cascade-deleted");

    // Team orders should be gone
    let (orders, _) = db::get_team_orders(&client, team.team_id, 100, 0)
        .await
        .unwrap();
    assert!(orders.is_empty(), "team orders should be cascade-deleted");

    // Cleanup: user and item are independent, not cascaded
    db::delete_user(&client, user.user_id).await.unwrap();
    db::delete_item(&client, item.item_id).await.unwrap();
}

/// Deleting a team order cascades to its order items.
#[actix_web::test]
#[ignore]
async fn delete_team_order_cascades_order_items() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-oi-cascade-{}", Uuid::now_v7()),
            price: Decimal::from_str("1.50").unwrap(),
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 3,
        },
    )
    .await
    .unwrap();

    // Delete the order — items should cascade
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .unwrap();

    let (items, _) = db::get_order_items(&client, order.teamorders_id, team.team_id, 100, 0)
        .await
        .unwrap();
    assert!(items.is_empty(), "order items should be cascade-deleted");

    db::delete_item(&client, item.item_id).await.unwrap();
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

/// Deleting a user cascades to memberof but is restricted by teamorders FK.
#[actix_web::test]
#[ignore]
async fn delete_user_cascades_membership() {
    let mut client = test_client().await;

    let team = db::create_team(
        &client,
        CreateTeamEntry {
            tname: format!("dbtest-ucascade-{}", Uuid::now_v7()),
            descr: None,
        },
    )
    .await
    .unwrap();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "UserCascade".to_string(),
            lastname: "Test".to_string(),
            email: unique_email(),
            password: "password123".to_string(),
        },
    )
    .await
    .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];
    db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id)
        .await
        .unwrap();

    // Verify membership exists
    let (members, _) = db::get_team_users(&client, team.team_id, 100, 0)
        .await
        .unwrap();
    assert!(!members.is_empty());

    // Delete user — membership should cascade
    db::delete_user(&client, user.user_id).await.unwrap();

    let (members, _) = db::get_team_users(&client, team.team_id, 100, 0)
        .await
        .unwrap();
    assert!(
        members.is_empty(),
        "membership should be cascade-deleted with user"
    );

    db::delete_team(&client, team.team_id).await.unwrap();
}

/// Deleting an item referenced by an order is blocked by FK RESTRICT (V3 migration).
#[actix_web::test]
#[ignore]
async fn delete_item_with_order_reference_is_restricted() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-restrict-{}", Uuid::now_v7()),
            price: Decimal::from_str("5.00").unwrap(),
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 1,
        },
    )
    .await
    .unwrap();

    // Attempt to delete item — should fail due to FK RESTRICT
    let result = db::delete_item(&client, item.item_id).await;
    assert!(
        result.is_err(),
        "deleting an item referenced by an order should fail (FK RESTRICT)"
    );

    // Cleanup: delete order first (cascades order items), then item
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .unwrap();
    db::delete_item(&client, item.item_id).await.unwrap();
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 10: Partial updates and FK violation tests
// ===========================================================================

/// #261 — Partial `update_team_order`: pass `None` for both fields and
/// verify existing values are preserved (COALESCE behaviour).
#[actix_web::test]
#[ignore]
async fn update_team_order_partial_preserves_existing_values() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    // Create with a duedate
    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    // Close the order first so we have a non-default value for `closed`
    let _ = db::update_team_order(
        &client,
        team.team_id,
        order.teamorders_id,
        UpdateTeamOrderEntry {
            duedate: None,
            closed: Some(true),
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    // Now update with None for both fields — values should be preserved
    let updated = db::update_team_order(
        &client,
        team.team_id,
        order.teamorders_id,
        UpdateTeamOrderEntry {
            duedate: None,
            closed: None,
            pickup_user_id: None,
        },
    )
    .await
    .expect("partial update should succeed");

    assert_eq!(
        updated.duedate,
        Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
        "duedate should be preserved when None is passed"
    );
    assert!(
        updated.closed,
        "closed should be preserved when None is passed"
    );

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

/// #262 — `create_team_order` with a non-existent `team_id` should fail
/// with a foreign key violation.
#[actix_web::test]
#[ignore]
async fn create_team_order_with_nonexistent_team_id_fails() {
    let client = test_client().await;
    let user = create_test_user(&client).await;
    let fake_team_id = Uuid::now_v7();

    let result = db::create_team_order(
        &client,
        fake_team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await;

    assert!(
        result.is_err(),
        "creating a team order with non-existent team_id should fail (FK violation)"
    );

    // Cleanup
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// #402 — get_password_hash DB function
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn get_password_hash_returns_argon2_hash() {
    let client = test_client().await;
    let email = unique_email();

    let created = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "HashTest".to_string(),
            lastname: "User".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let hash = db::get_password_hash(&client, created.user_id)
        .await
        .expect("get_password_hash should succeed");

    assert!(
        hash.starts_with("$argon2"),
        "hash should be an Argon2 hash, got: {}",
        hash
    );

    // Verify the hash matches the original password
    let parsed = argon2::PasswordHash::new(&hash).expect("should parse");
    argon2::Argon2::default()
        .verify_password(b"securepassword123", &parsed)
        .expect("hash should verify against original password");

    // Cleanup
    db::delete_user(&client, created.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_password_hash_returns_not_found_for_nonexistent_user() {
    let client = test_client().await;
    let fake_id = Uuid::now_v7();

    let result = db::get_password_hash(&client, fake_id).await;
    assert!(result.is_err(), "should return error for nonexistent user");
}

// ===========================================================================
// #399 — count_admins DB function
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn count_admins_returns_at_least_one() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;

    let count = db::count_admins(&client)
        .await
        .expect("count_admins should succeed");
    assert!(count >= 1, "should have at least one admin after setup");

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, admin.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// #436 — create_team with a duplicate name fails with a DB error
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_team_with_duplicate_name_fails() {
    let client = test_client().await;
    let unique_name = format!("DuplicateTeam-{}", Uuid::now_v7());

    let entry = CreateTeamEntry {
        tname: unique_name.clone(),
        descr: Some("first".to_string()),
    };

    // First insert should succeed
    let team = db::create_team(&client, entry)
        .await
        .expect("first create_team should succeed");

    // Second insert with the same name should fail (unique constraint violation)
    let duplicate = CreateTeamEntry {
        tname: unique_name,
        descr: Some("duplicate".to_string()),
    };
    let result = db::create_team(&client, duplicate).await;
    assert!(
        result.is_err(),
        "create_team with a duplicate name should return an error"
    );

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup: delete_team should succeed");
}

// ===========================================================================
// #437 — create_role with a duplicate title fails with a DB error
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_role_with_duplicate_title_fails() {
    let client = test_client().await;
    let unique_title = format!("DuplicateRole-{}", Uuid::now_v7());

    let entry = CreateRoleEntry {
        title: unique_title.clone(),
    };

    // First insert should succeed
    let role = db::create_role(&client, entry)
        .await
        .expect("first create_role should succeed");

    // Second insert with the same title should fail (unique constraint violation)
    let duplicate = CreateRoleEntry {
        title: unique_title,
    };
    let result = db::create_role(&client, duplicate).await;
    assert!(
        result.is_err(),
        "create_role with a duplicate title should return an error"
    );

    // Cleanup
    db::delete_role(&client, role.role_id)
        .await
        .expect("cleanup: delete_role should succeed");
}

// ===========================================================================
// check_team_access for Team Admin role (#393)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn check_team_access_team_admin() {
    let mut client = test_client().await;
    let (_admin, ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let (is_admin, team_role) = db::check_team_access(&client, team1.team_id, ta.user_id)
        .await
        .expect("check_team_access should succeed");
    assert!(
        !is_admin,
        "Team Admin should NOT be recognized as global admin"
    );
    assert_eq!(
        team_role,
        Some("Team Admin".to_string()),
        "team admin should have Team Admin role"
    );

    // Cleanup
    db::delete_team(&client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, ta.user_id).await.expect("cleanup");
    db::delete_user(&client, _member.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Avatar subsystem DB tests (#622)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn insert_and_get_avatar() {
    let client = test_client().await;
    let avatar_id = Uuid::now_v7();
    let name = format!("test-avatar-{}", avatar_id);
    let data = b"\x89PNG\r\n\x1a\n fake png";
    let content_type = "image/png";

    db::insert_avatar(&client, avatar_id, &name, data, content_type)
        .await
        .expect("insert_avatar should succeed");

    // Fetch it back
    let (fetched_data, fetched_ct) = db::get_avatar(&client, avatar_id)
        .await
        .expect("get_avatar should succeed");
    assert_eq!(fetched_data, data);
    assert_eq!(fetched_ct, content_type);

    // It should appear in the list
    let list = db::get_avatars(&client)
        .await
        .expect("get_avatars should succeed");
    assert!(
        list.iter().any(|a| a.avatar_id == avatar_id),
        "inserted avatar should appear in get_avatars list"
    );

    // Cleanup: delete directly (no db function for deletion, use raw SQL)
    client
        .execute("DELETE FROM avatars WHERE avatar_id = $1", &[&avatar_id])
        .await
        .expect("cleanup avatar");
}

#[actix_web::test]
#[ignore]
async fn count_avatars_matches_list() {
    let client = test_client().await;

    let count = db::count_avatars(&client)
        .await
        .expect("count_avatars should succeed");
    let list = db::get_avatars(&client)
        .await
        .expect("get_avatars should succeed");

    assert_eq!(
        count as usize,
        list.len(),
        "count_avatars should match get_avatars length"
    );
}

#[actix_web::test]
#[ignore]
async fn set_user_avatar_and_clear() {
    let client = test_client().await;
    let user = create_test_user(&client).await;

    // Insert a test avatar
    let avatar_id = Uuid::now_v7();
    let name = format!("set-avatar-{}", avatar_id);
    db::insert_avatar(&client, avatar_id, &name, b"fake", "image/png")
        .await
        .expect("insert_avatar");

    // Set the avatar on the user
    let updated = db::set_user_avatar(&client, user.user_id, Some(avatar_id))
        .await
        .expect("set_user_avatar should succeed");
    assert_eq!(updated.avatar_id, Some(avatar_id));

    // Clear the avatar
    let cleared = db::set_user_avatar(&client, user.user_id, None)
        .await
        .expect("set_user_avatar(None) should succeed");
    assert_eq!(cleared.avatar_id, None);

    // Cleanup
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup user");
    client
        .execute("DELETE FROM avatars WHERE avatar_id = $1", &[&avatar_id])
        .await
        .expect("cleanup avatar");
}

// ===========================================================================
// would_admins_remain_without DB test (#623)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn would_admins_remain_without_two_admins() {
    let mut client = test_client().await;
    let roles = ensure_roles(&client).await;
    let admin1 = create_test_user(&client).await;
    let admin2 = create_test_user(&client).await;
    let team = create_test_team(&client).await;

    db::add_team_member(&mut client, team.team_id, admin1.user_id, roles["Admin"])
        .await
        .expect("add admin1");
    db::add_team_member(&mut client, team.team_id, admin2.user_id, roles["Admin"])
        .await
        .expect("add admin2");

    // With 2 admins, excluding one should still leave admins
    let remains = db::would_admins_remain_without(&client, team.team_id, admin1.user_id)
        .await
        .expect("would_admins_remain_without should succeed");
    assert!(
        remains,
        "should still have admins after excluding one of two"
    );

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&client, admin1.user_id)
        .await
        .expect("cleanup admin1");
    db::delete_user(&client, admin2.user_id)
        .await
        .expect("cleanup admin2");
}

#[actix_web::test]
#[ignore]
async fn would_admins_remain_without_sole_admin() {
    let mut client = test_client().await;
    let roles = ensure_roles(&client).await;

    // Record how many admins already exist from parallel tests so we can
    // account for them when checking the result.
    let baseline = db::count_admins(&client)
        .await
        .expect("count_admins should succeed");

    let admin = create_test_user(&client).await;
    let team = create_test_team(&client).await;

    db::add_team_member(&mut client, team.team_id, admin.user_id, roles["Admin"])
        .await
        .expect("add admin");

    // Excluding our admin should leave only the baseline admins (from
    // parallel tests). When the baseline is 0, the function should return
    // false (no admins remain). When the baseline is > 0, it should return
    // true because other admins still exist.
    let remains = db::would_admins_remain_without(&client, team.team_id, admin.user_id)
        .await
        .expect("would_admins_remain_without should succeed");
    assert_eq!(
        remains,
        baseline > 0,
        "expected remains={} when baseline admin count is {baseline}",
        baseline > 0
    );

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&client, admin.user_id)
        .await
        .expect("cleanup admin");
}
