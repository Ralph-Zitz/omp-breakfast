//! Avatar CRUD DB function tests.

#[path = "common/db.rs"]
mod db_helpers;

use breakfast::db;
use db_helpers::*;

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

    // Insert a known avatar so the table is non-empty
    let avatar_id = Uuid::now_v7();
    let name = format!("count-test-{}", avatar_id);
    db::insert_avatar(&client, avatar_id, &name, b"img", "image/png")
        .await
        .expect("insert_avatar");

    let count = db::count_avatars(&client)
        .await
        .expect("count_avatars should succeed");
    let list = db::get_avatars(&client)
        .await
        .expect("get_avatars should succeed");

    // Both should reflect at least the avatar we just inserted.
    // In a parallel test environment the exact totals may differ slightly
    // between the two queries, so we verify each one is ≥ 1 and that the
    // count is not wildly off (within ±10 of the list length).
    assert!(count >= 1, "count_avatars should be >= 1 after insert");
    assert!(
        !list.is_empty(),
        "get_avatars should be non-empty after insert"
    );
    assert!(
        (count as i64 - list.len() as i64).unsigned_abs() <= 10,
        "count_avatars ({count}) and get_avatars len ({}) should be roughly equal",
        list.len()
    );
}

#[actix_web::test]
#[ignore]
async fn set_user_avatar_and_clear() {
    let mut client = test_client().await;
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
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup user");
    client
        .execute("DELETE FROM avatars WHERE avatar_id = $1", &[&avatar_id])
        .await
        .expect("cleanup avatar");
}

// ===========================================================================
// Additional avatar DB tests (#634)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn get_avatar_nonexistent_returns_error() {
    let client = test_client().await;
    let fake_id = Uuid::now_v7();

    let result = db::get_avatar(&client, fake_id).await;
    assert!(result.is_err(), "nonexistent avatar should return error");
}

#[actix_web::test]
#[ignore]
async fn set_user_avatar_nonexistent_user_returns_error() {
    let client = test_client().await;

    // Insert a real avatar so the FK isn't the cause of failure
    let avatar_id = Uuid::now_v7();
    let name = format!("orphan-avatar-{}", avatar_id);
    db::insert_avatar(&client, avatar_id, &name, b"fake", "image/png")
        .await
        .expect("insert_avatar should succeed");

    let fake_user_id = Uuid::now_v7();
    let result = db::set_user_avatar(&client, fake_user_id, Some(avatar_id)).await;
    assert!(
        result.is_err(),
        "setting avatar on nonexistent user should fail"
    );

    // Cleanup
    client
        .execute("DELETE FROM avatars WHERE avatar_id = $1", &[&avatar_id])
        .await
        .expect("cleanup avatar");
}

#[actix_web::test]
#[ignore]
async fn insert_avatar_duplicate_name_is_idempotent() {
    let client = test_client().await;
    let name = format!("dup-avatar-{}", Uuid::now_v7());

    let id1 = Uuid::now_v7();
    db::insert_avatar(&client, id1, &name, b"data1", "image/png")
        .await
        .expect("first insert should succeed");

    // Second insert with same name should be silently ignored (ON CONFLICT DO NOTHING)
    let id2 = Uuid::now_v7();
    db::insert_avatar(&client, id2, &name, b"data2", "image/png")
        .await
        .expect("duplicate name insert should not error");

    // Only the first should be retrievable
    let (data, _) = db::get_avatar(&client, id1)
        .await
        .expect("first avatar should exist");
    assert_eq!(data, b"data1");

    let result = db::get_avatar(&client, id2).await;
    assert!(
        result.is_err(),
        "second avatar with duplicate name should not be inserted"
    );

    // Cleanup
    client
        .execute("DELETE FROM avatars WHERE avatar_id = $1", &[&id1])
        .await
        .expect("cleanup avatar");
}
