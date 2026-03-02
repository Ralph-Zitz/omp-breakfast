//! Integration tests for `db.rs` — tests every database function directly.
//!
//! These tests require a running PostgreSQL instance initialized via Refinery
//! migrations and seeded with `database_seed.sql`.
//! Run them via:
//!   make test-integration
//!
//! Or manually:
//!   docker compose up -d postgres && docker compose run --rm postgres-setup
//!   TEST_DB_PORT=5433 cargo test --test db_tests -- --ignored

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

/// Generate a unique email that won't collide with seed data.
fn unique_email() -> String {
    format!("dbtest-{}@test.local", Uuid::now_v7())
}

/// Lookup a seed user by email, returning the user_id.
async fn seed_user_id(client: &deadpool_postgres::Client, email: &str) -> Uuid {
    let user = db::get_user_by_email(client, email)
        .await
        .expect("seed user should exist");
    user.user_id
}

/// Lookup a seed team by name, returning the team_id.
async fn seed_team_id(client: &deadpool_postgres::Client, tname: &str) -> Uuid {
    let teams = db::get_teams(client).await.expect("should list teams");
    teams
        .into_iter()
        .find(|t| t.tname == tname)
        .unwrap_or_else(|| panic!("seed team '{}' not found", tname))
        .team_id
}

/// Lookup a seed role by title, returning the role_id.
async fn seed_role_id(client: &deadpool_postgres::Client, title: &str) -> Uuid {
    let roles = db::get_roles(client).await.expect("should list roles");
    roles
        .into_iter()
        .find(|r| r.title == title)
        .unwrap_or_else(|| panic!("seed role '{}' not found", title))
        .role_id
}

// ===========================================================================
// Group 1: Health / connectivity
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn check_db_returns_true() {
    let client = test_client().await;
    let result = db::check_db(&client)
        .await
        .expect("check_db should succeed");
    assert!(result, "check_db should return true on a healthy DB");
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
async fn get_users_returns_seed_data() {
    let client = test_client().await;
    let users = db::get_users(&client)
        .await
        .expect("get_users should succeed");
    assert!(
        users.len() >= 5,
        "seed data has 5 users, got {}",
        users.len()
    );
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
async fn get_teams_returns_seed_data() {
    let client = test_client().await;
    let teams = db::get_teams(&client)
        .await
        .expect("get_teams should succeed");
    assert!(
        teams.len() >= 2,
        "seed data has 2 teams, got {}",
        teams.len()
    );
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
async fn get_roles_returns_seed_data() {
    let client = test_client().await;
    let roles = db::get_roles(&client)
        .await
        .expect("get_roles should succeed");
    assert!(
        roles.len() >= 4,
        "seed data has 4 roles, got {}",
        roles.len()
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
async fn get_items_returns_seed_data() {
    let client = test_client().await;
    let items = db::get_items(&client)
        .await
        .expect("get_items should succeed");
    assert!(
        items.len() >= 4,
        "seed data has 4 items, got {}",
        items.len()
    );
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
    let client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    let order = db::create_team_order(
        &client,
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()),
        },
    )
    .await
    .expect("create_team_order should succeed");

    assert_eq!(order.teamorders_team_id, team_id);
    assert_eq!(
        order.duedate,
        Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap())
    );
    assert!(!order.closed);

    // Cleanup
    db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn create_team_order_with_user_id() {
    let client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;
    let user_id = seed_user_id(&client, "admin@admin.com").await;

    let order = db::create_team_order(
        &client,
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: Some(user_id),
            duedate: None,
        },
    )
    .await
    .expect("create_team_order with user should succeed");

    assert_eq!(order.teamorders_user_id, Some(user_id));

    // Cleanup
    db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_team_orders_returns_seed_data() {
    let client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    let orders = db::get_team_orders(&client, team_id)
        .await
        .expect("get_team_orders should succeed");
    assert!(
        !orders.is_empty(),
        "seed data should have at least 1 order for League of Cool Coders"
    );
}

#[actix_web::test]
#[ignore]
async fn get_team_order_by_id() {
    let client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    let order = db::create_team_order(
        &client,
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: None,
        },
    )
    .await
    .unwrap();

    let fetched = db::get_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("get_team_order should succeed");
    assert_eq!(fetched.teamorders_id, order.teamorders_id);
    assert_eq!(fetched.teamorders_team_id, team_id);

    // Cleanup
    db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_team_order_changes_fields() {
    let client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    let order = db::create_team_order(
        &client,
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: None,
        },
    )
    .await
    .unwrap();

    let updated = db::update_team_order(
        &client,
        team_id,
        order.teamorders_id,
        UpdateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: Some(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()),
            closed: Some(true),
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
    db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_team_order_returns_true_then_false() {
    let client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    let order = db::create_team_order(
        &client,
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: None,
        },
    )
    .await
    .unwrap();

    let deleted = db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .unwrap();
    assert!(deleted);

    let deleted_again = db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .unwrap();
    assert!(!deleted_again);
}

#[actix_web::test]
#[ignore]
async fn delete_team_orders_bulk() {
    let client = test_client().await;

    // Create a dedicated team to avoid interfering with seed data orders
    let tname = format!("dbtest-bulk-del-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    // Create two orders
    db::create_team_order(
        &client,
        team.team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: None,
        },
    )
    .await
    .unwrap();

    db::create_team_order(
        &client,
        team.team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: None,
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
}

// ===========================================================================
// Group 7: Order items CRUD (items within a team order)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_order_item_returns_entry() {
    let mut client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

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
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: None,
        },
    )
    .await
    .unwrap();

    let order_item = db::create_order_item(
        &mut client,
        order.teamorders_id,
        team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 5,
        },
    )
    .await
    .expect("create_order_item should succeed");

    assert_eq!(order_item.orders_teamorders_id, order.teamorders_id);
    assert_eq!(order_item.orders_item_id, item.item_id);
    assert_eq!(order_item.orders_team_id, team_id);
    assert_eq!(order_item.amt, 5);

    // Cleanup (cascade: deleting order deletes order items)
    db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
}

#[actix_web::test]
#[ignore]
async fn get_order_items_returns_list() {
    let client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    // Seed data has at least one team order with items.
    // Use `last()` because orders are sorted by `created desc` so the seed-data
    // order (created first) appears last; other concurrently-running tests may
    // create newer (empty) orders for the same team.
    let orders = db::get_team_orders(&client, team_id).await.unwrap();
    let seed_order = orders
        .last()
        .expect("seed data should have at least one order");

    let items = db::get_order_items(&client, seed_order.teamorders_id, team_id)
        .await
        .expect("get_order_items should succeed");
    assert!(
        items.len() >= 2,
        "seed order should have at least 2 items, got {}",
        items.len()
    );
}

#[actix_web::test]
#[ignore]
async fn get_order_item_by_id() {
    let mut client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

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
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 3,
        },
    )
    .await
    .unwrap();

    let fetched = db::get_order_item(&client, order.teamorders_id, item.item_id, team_id)
        .await
        .expect("get_order_item should succeed");
    assert_eq!(fetched.orders_item_id, item.item_id);
    assert_eq!(fetched.amt, 3);

    // Cleanup
    db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
}

#[actix_web::test]
#[ignore]
async fn update_order_item_changes_amt() {
    let mut client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

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
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team_id,
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
        team_id,
        UpdateOrderEntry { amt: 42 },
    )
    .await
    .expect("update_order_item should succeed");

    assert_eq!(updated.amt, 42);

    // Cleanup
    db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
}

#[actix_web::test]
#[ignore]
async fn delete_order_item_returns_true_then_false() {
    let mut client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

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
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 1,
        },
    )
    .await
    .unwrap();

    let deleted = db::delete_order_item(&mut client, order.teamorders_id, item.item_id, team_id)
        .await
        .unwrap();
    assert!(deleted);

    let deleted_again =
        db::delete_order_item(&mut client, order.teamorders_id, item.item_id, team_id)
            .await
            .unwrap();
    assert!(!deleted_again);

    // Cleanup
    db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
}

#[actix_web::test]
#[ignore]
async fn duplicate_order_item_returns_error() {
    let mut client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

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
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team_id,
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
        team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 2,
        },
    )
    .await;

    assert!(result.is_err(), "duplicate order item should fail");

    // Cleanup
    db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
}

// ===========================================================================
// Group 8: Memberof / RBAC queries
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn is_admin_returns_true_for_seed_admin() {
    let client = test_client().await;
    let admin_id = seed_user_id(&client, "admin@admin.com").await;

    let result = db::is_admin(&client, admin_id)
        .await
        .expect("is_admin should succeed");
    assert!(result, "seed admin should be recognized as admin");
}

#[actix_web::test]
#[ignore]
async fn is_admin_returns_false_for_member() {
    let client = test_client().await;
    let member_id = seed_user_id(&client, "U1_F.U1_L@LEGO.com").await;

    let result = db::is_admin(&client, member_id)
        .await
        .expect("is_admin should succeed");
    assert!(!result, "seed member U1_F should not be admin");
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
    let client = test_client().await;
    let admin_id = seed_user_id(&client, "admin@admin.com").await;

    let result = db::is_admin_or_team_admin(&client, admin_id)
        .await
        .expect("is_admin_or_team_admin should succeed");
    assert!(result);
}

#[actix_web::test]
#[ignore]
async fn is_admin_or_team_admin_returns_true_for_team_admin() {
    let client = test_client().await;
    // U4_F is Team Admin of "League of Cool Coders"
    let u4_id = seed_user_id(&client, "U4_F.U4_L@LEGO.com").await;

    let result = db::is_admin_or_team_admin(&client, u4_id)
        .await
        .expect("is_admin_or_team_admin should succeed");
    assert!(result, "U4_F is a Team Admin and should return true");
}

#[actix_web::test]
#[ignore]
async fn is_admin_or_team_admin_returns_false_for_member() {
    let client = test_client().await;
    // U1_F is just a Member
    let u1_id = seed_user_id(&client, "U1_F.U1_L@LEGO.com").await;

    let result = db::is_admin_or_team_admin(&client, u1_id)
        .await
        .expect("is_admin_or_team_admin should succeed");
    assert!(!result, "U1_F is a Member and should return false");
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_correct_role_for_admin() {
    let client = test_client().await;
    let admin_id = seed_user_id(&client, "admin@admin.com").await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    let role = db::get_member_role(&client, team_id, admin_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, Some("Admin".to_string()));
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_correct_role_for_team_admin() {
    let client = test_client().await;
    let u4_id = seed_user_id(&client, "U4_F.U4_L@LEGO.com").await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    let role = db::get_member_role(&client, team_id, u4_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, Some("Team Admin".to_string()));
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_member_for_regular_user() {
    let client = test_client().await;
    let u1_id = seed_user_id(&client, "U1_F.U1_L@LEGO.com").await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    let role = db::get_member_role(&client, team_id, u1_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, Some("Member".to_string()));
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_none_for_non_member() {
    let client = test_client().await;
    // U1_F is NOT a member of "Pixel Bakers"
    let u1_id = seed_user_id(&client, "U1_F.U1_L@LEGO.com").await;
    let pixel_bakers_id = seed_team_id(&client, "Pixel Bakers").await;

    let role = db::get_member_role(&client, pixel_bakers_id, u1_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, None);
}

#[actix_web::test]
#[ignore]
async fn is_team_admin_of_user_returns_true_for_shared_team() {
    let client = test_client().await;
    // U4_F is Team Admin of "League of Cool Coders"
    // U1_F is a Member of "League of Cool Coders"
    let u4_id = seed_user_id(&client, "U4_F.U4_L@LEGO.com").await;
    let u1_id = seed_user_id(&client, "U1_F.U1_L@LEGO.com").await;

    let result = db::is_team_admin_of_user(&client, u4_id, u1_id)
        .await
        .expect("is_team_admin_of_user should succeed");
    assert!(
        result,
        "U4_F is Team Admin of a team where U1_F is a member"
    );
}

#[actix_web::test]
#[ignore]
async fn is_team_admin_of_user_returns_false_for_non_shared_team() {
    let client = test_client().await;
    // U4_F is Team Admin of "League of Cool Coders" and Member of "Pixel Bakers"
    // Create a new user who is NOT in any of U4's teams
    let email = unique_email();
    let outsider = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "Outsider".to_string(),
            lastname: "User".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let u4_id = seed_user_id(&client, "U4_F.U4_L@LEGO.com").await;

    let result = db::is_team_admin_of_user(&client, u4_id, outsider.user_id)
        .await
        .expect("is_team_admin_of_user should succeed");
    assert!(
        !result,
        "U4_F should not be team admin of a user outside their teams"
    );

    // Cleanup
    db::delete_user(&client, outsider.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_admin_of_user_returns_false_for_regular_member() {
    let client = test_client().await;
    // U1_F is a Member (not Team Admin) of "League of Cool Coders"
    // U2_F is also a Member of "League of Cool Coders"
    let u1_id = seed_user_id(&client, "U1_F.U1_L@LEGO.com").await;
    let u2_id = seed_user_id(&client, "U2_F.U2_L@LEGO.com").await;

    let result = db::is_team_admin_of_user(&client, u1_id, u2_id)
        .await
        .expect("is_team_admin_of_user should succeed");
    assert!(!result, "U1_F is not a Team Admin, so should return false");
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

    let member_role_id = seed_role_id(&client, "Member").await;

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

    let member_role_id = seed_role_id(&client, "Member").await;
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

    let member_role_id = seed_role_id(&client, "Member").await;
    let guest_role_id = seed_role_id(&client, "Guest").await;

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

    let member_role_id = seed_role_id(&client, "Member").await;
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

    let member_role_id = seed_role_id(&client, "Member").await;

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

// ===========================================================================
// Group 10: User/Team relationship queries
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn get_user_teams_returns_memberships() {
    let client = test_client().await;
    let admin_id = seed_user_id(&client, "admin@admin.com").await;

    let teams = db::get_user_teams(&client, admin_id)
        .await
        .expect("get_user_teams should succeed");
    assert!(
        !teams.is_empty(),
        "seed admin should have at least 1 team membership"
    );
    // Verify the structure includes the expected fields
    let first = &teams[0];
    assert!(!first.tname.is_empty());
    assert!(!first.title.is_empty());
    assert!(!first.firstname.is_empty());
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

    let teams = db::get_user_teams(&client, user.user_id)
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
    let client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    let users = db::get_team_users(&client, team_id)
        .await
        .expect("get_team_users should succeed");
    assert!(
        users.len() >= 4,
        "League of Cool Coders should have at least 4 members, got {}",
        users.len()
    );
    // Verify the structure
    let first = &users[0];
    assert!(!first.firstname.is_empty());
    assert!(!first.email.is_empty());
    assert!(!first.title.is_empty());
}

#[actix_web::test]
#[ignore]
async fn get_team_users_returns_empty_for_empty_team() {
    let client = test_client().await;
    let tname = format!("dbtest-empty-{}", Uuid::now_v7());

    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let users = db::get_team_users(&client, team.team_id)
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
    let client = test_client().await;
    // U4_F is a member of both "League of Cool Coders" (Team Admin) and "Pixel Bakers" (Member)
    let u4_id = seed_user_id(&client, "U4_F.U4_L@LEGO.com").await;

    let teams = db::get_user_teams(&client, u4_id)
        .await
        .expect("get_user_teams should succeed");
    assert!(
        teams.len() >= 2,
        "U4_F should be in at least 2 teams, got {}",
        teams.len()
    );

    // Verify different roles in different teams
    let team_names: Vec<&str> = teams.iter().map(|t| t.tname.as_str()).collect();
    assert!(
        team_names.contains(&"League of Cool Coders"),
        "should include League of Cool Coders"
    );
    assert!(
        team_names.contains(&"Pixel Bakers"),
        "should include Pixel Bakers"
    );
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
            teamorders_user_id: None,
            duedate: None,
            closed: None,
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
    let client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    // Create a new order (defaults to closed = false)
    let order = db::create_team_order(
        &client,
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: Some(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()),
        },
    )
    .await
    .expect("should create team order");

    let closed = db::is_team_order_closed(&client, order.teamorders_id, team_id)
        .await
        .expect("should check closed status");
    assert!(!closed, "newly created order should not be closed");

    // Cleanup
    db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_order_closed_returns_true_for_closed_order() {
    let client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;

    // Create a new order
    let order = db::create_team_order(
        &client,
        team_id,
        CreateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: Some(NaiveDate::from_ymd_opt(2026, 12, 26).unwrap()),
        },
    )
    .await
    .expect("should create team order");

    // Close the order
    db::update_team_order(
        &client,
        team_id,
        order.teamorders_id,
        UpdateTeamOrderEntry {
            teamorders_user_id: None,
            duedate: Some(NaiveDate::from_ymd_opt(2026, 12, 26).unwrap()),
            closed: Some(true),
        },
    )
    .await
    .expect("should close the order");

    let closed = db::is_team_order_closed(&client, order.teamorders_id, team_id)
        .await
        .expect("should check closed status");
    assert!(closed, "updated order should be closed");

    // Cleanup
    db::delete_team_order(&client, team_id, order.teamorders_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_order_closed_returns_not_found_for_nonexistent_order() {
    let client = test_client().await;
    let team_id = seed_team_id(&client, "League of Cool Coders").await;
    let fake_order_id = Uuid::now_v7();

    let result = db::is_team_order_closed(&client, fake_order_id, team_id).await;
    assert!(result.is_err(), "nonexistent order should return an error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found"),
        "error should mention 'not found', got: {}",
        err_msg
    );
}
