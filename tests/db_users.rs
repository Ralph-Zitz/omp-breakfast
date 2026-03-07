//! User CRUD DB function tests.

#[path = "common/db.rs"]
mod db_helpers;

use argon2::password_hash::PasswordVerifier;
use breakfast::{db, models::*};
use chrono::Utc;
use db_helpers::*;
use uuid::Uuid;

// ===========================================================================
// Group 2: User CRUD
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_user_returns_entry_with_correct_fields() {
    let mut client = test_client().await;
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
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_user_by_id_returns_matching_user() {
    let mut client = test_client().await;
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
    db::delete_user(&mut client, created.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_user_by_email_returns_update_user_entry() {
    let mut client = test_client().await;
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
    db::delete_user(&mut client, created.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_users_returns_created_data() {
    let mut client = test_client().await;
    let user = create_test_user(&client).await;
    let (users, total) = db::get_users(&client, 100, 0)
        .await
        .expect("get_users should succeed");
    assert!(total >= 1, "should have at least 1 user, got {}", total);
    assert!(users.iter().any(|u| u.user_id == user.user_id));
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_user_without_password_preserves_hash() {
    let mut client = test_client().await;
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
    db::delete_user(&mut client, created.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_user_with_password_changes_hash() {
    let mut client = test_client().await;
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
    db::delete_user(&mut client, created.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_user_returns_true_then_false() {
    let mut client = test_client().await;

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

    let deleted = db::delete_user(&mut client, user.user_id).await.unwrap();
    assert!(deleted, "first delete should return true");

    let deleted_again = db::delete_user(&mut client, user.user_id).await.unwrap();
    assert!(!deleted_again, "second delete should return false");
}

#[actix_web::test]
#[ignore]
async fn delete_user_by_email_returns_true_then_false() {
    let mut client = test_client().await;
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

    let deleted = db::delete_user_by_email(&mut client, &email).await.unwrap();
    assert!(deleted);

    let deleted_again = db::delete_user_by_email(&mut client, &email).await.unwrap();
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
    let mut client = test_client().await;
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
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup");
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

// ===========================================================================
// Group 13: Timestamp and changed-column behavior
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_user_sets_timestamps() {
    let mut client = test_client().await;
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
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_user_updates_changed_timestamp() {
    let mut client = test_client().await;
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
    db::delete_user(&mut client, created.user_id)
        .await
        .expect("cleanup");
}

/// Deleting a user cleans up memberof rows (handled within the DB function's
/// transaction since memberof FK is ON DELETE RESTRICT).
#[actix_web::test]
#[ignore]
async fn delete_user_cleans_up_membership() {
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
    db::delete_user(&mut client, user.user_id).await.unwrap();

    let (members, _) = db::get_team_users(&client, team.team_id, 100, 0)
        .await
        .unwrap();
    assert!(
        members.is_empty(),
        "membership should be cascade-deleted with user"
    );

    db::delete_team(&client, team.team_id).await.unwrap();
}

// ===========================================================================
// #402 — get_password_hash DB function
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn get_password_hash_returns_argon2_hash() {
    let mut client = test_client().await;
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
    db::delete_user(&mut client, created.user_id)
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
// #663 — count_users DB function test
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn count_users_returns_positive_count() {
    let mut client = test_client().await;

    // Ensure at least one user exists
    let user = create_test_user(&client).await;

    let count = db::count_users(&client)
        .await
        .expect("count_users should succeed");
    assert!(count >= 1, "should have at least one user");

    // Cleanup
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup");
}
