//! Token blacklist DB function tests.

#[path = "common/db.rs"]
mod db_helpers;

use breakfast::db;
use chrono::Utc;
use db_helpers::*;

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
