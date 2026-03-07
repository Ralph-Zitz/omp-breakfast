//! Role CRUD and admin-count DB function tests.

#[path = "common/db.rs"]
mod db_helpers;

use breakfast::{db, models::*};
use db_helpers::*;
use uuid::Uuid;

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
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, admin.user_id)
        .await
        .expect("cleanup");
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
// #663 — seed_default_roles DB function test
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn seed_default_roles_creates_four_default_roles() {
    let client = test_client().await;

    let roles = db::seed_default_roles(&client)
        .await
        .expect("seed_default_roles should succeed");

    let titles: Vec<&str> = roles.iter().map(|r| r.title.as_str()).collect();
    assert!(titles.contains(&"Admin"), "should contain Admin");
    assert!(titles.contains(&"Team Admin"), "should contain Team Admin");
    assert!(titles.contains(&"Member"), "should contain Member");
    assert!(titles.contains(&"Guest"), "should contain Guest");
}

#[actix_web::test]
#[ignore]
async fn seed_default_roles_is_idempotent() {
    let client = test_client().await;

    let first = db::seed_default_roles(&client).await.unwrap();
    let second = db::seed_default_roles(&client).await.unwrap();

    // Compare only the 4 default roles (other tests may create roles concurrently)
    let defaults = ["Admin", "Team Admin", "Member", "Guest"];
    let first_ids: Vec<_> = first
        .iter()
        .filter(|r| defaults.contains(&r.title.as_str()))
        .map(|r| r.role_id)
        .collect();
    let second_ids: Vec<_> = second
        .iter()
        .filter(|r| defaults.contains(&r.title.as_str()))
        .map(|r| r.role_id)
        .collect();
    assert_eq!(
        first_ids, second_ids,
        "seed_default_roles should be idempotent for default roles"
    );
    assert_eq!(first_ids.len(), 4, "should have exactly 4 default roles");
}
