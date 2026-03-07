//! Avatar CRUD, avatar assignment, and avatar removal tests.

mod common;

use actix_web::test;
use common::*;
use serde_json::{Value, json};
use uuid::Uuid;

// ===========================================================================
// Avatar RBAC tests (#613)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn user_sets_own_avatar() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();
    let email = format!("avatar-self-{}@test.local", uid);
    let (user_auth, user_id) =
        create_and_login_user(&app, admin_token, "Ava", "Self", &email, "Very Secret").await;
    let token = &user_auth.access_token;

    // Get available avatars
    let req = test::TestRequest::get()
        .uri("/api/v1.0/avatars")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let avatars: Vec<Value> = test::read_body_json(resp).await;

    if avatars.is_empty() {
        // No avatars seeded — skip the rest (test infra issue, not a code bug)
        return;
    }
    let avatar_id = avatars[0]["avatar_id"].as_str().unwrap();

    // Set own avatar → 200
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}/avatar", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"avatar_id": avatar_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "user should set their own avatar");

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn admin_sets_other_user_avatar() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();
    let email = format!("avatar-admin-{}@test.local", uid);
    let (_user_auth, user_id) =
        create_and_login_user(&app, admin_token, "Ava", "Admin", &email, "Very Secret").await;

    // Get available avatars
    let req = test::TestRequest::get()
        .uri("/api/v1.0/avatars")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let avatars: Vec<Value> = test::read_body_json(resp).await;

    if avatars.is_empty() {
        return;
    }
    let avatar_id = avatars[0]["avatar_id"].as_str().unwrap();

    // Admin sets another user's avatar → 200
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}/avatar", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"avatar_id": avatar_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should set another user's avatar");

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_set_other_user_avatar() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();

    // Create two regular users in a team as Member
    let email_a = format!("avatar-a-{}@test.local", uid);
    let email_b = format!("avatar-b-{}@test.local", uid);
    let (auth_a, user_a_id) =
        create_and_login_user(&app, admin_token, "Ava", "Aa", &email_a, "Very Secret").await;
    let (_auth_b, user_b_id) =
        create_and_login_user(&app, admin_token, "Ava", "Bb", &email_b, "Very Secret").await;

    let team_id = create_test_team(&app, admin_token, &format!("AvaTeam-{}", uid)).await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &user_a_id, &member_role_id).await;
    add_member(&app, admin_token, &team_id, &user_b_id, &member_role_id).await;

    // Get available avatars
    let req = test::TestRequest::get()
        .uri("/api/v1.0/avatars")
        .insert_header(("Authorization", format!("Bearer {}", &auth_a.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let avatars: Vec<Value> = test::read_body_json(resp).await;

    if avatars.is_empty() {
        return;
    }
    let avatar_id = avatars[0]["avatar_id"].as_str().unwrap();

    // User A tries to set User B's avatar → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}/avatar", user_b_id))
        .insert_header(("Authorization", format!("Bearer {}", &auth_a.access_token)))
        .set_json(json!({"avatar_id": avatar_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "member should not set another user's avatar"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_a_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_b_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// Avatar subsystem API tests (#622)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn list_avatars_returns_200() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/avatars")
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "should list avatars");
    let avatars: Vec<Value> = test::read_body_json(resp).await;
    // Avatars are seeded at server startup from minifigs/; the integration test
    // environment does not run the full server init, so the list may be empty.
    if !avatars.is_empty() {
        assert!(
            avatars[0]["avatar_id"].as_str().is_some(),
            "avatar should have avatar_id"
        );
        assert!(
            avatars[0]["name"].as_str().is_some(),
            "avatar should have name"
        );
    }
}

#[actix_web::test]
#[ignore]
async fn get_single_avatar_returns_image_with_cache_headers() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Get list to find an avatar_id
    let req = test::TestRequest::get()
        .uri("/api/v1.0/avatars")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let avatars: Vec<Value> = test::read_body_json(resp).await;

    if avatars.is_empty() {
        return;
    }
    let avatar_id = avatars[0]["avatar_id"].as_str().unwrap();

    // Fetch single avatar
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/avatars/{}", avatar_id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "should serve avatar image");

    // Check Cache-Control header
    let cache_control = resp
        .headers()
        .get("cache-control")
        .expect("should have Cache-Control header")
        .to_str()
        .unwrap();
    assert!(
        cache_control.contains("immutable"),
        "avatar should be served with immutable cache header"
    );

    // Check Content-Type starts with image/
    let content_type = resp
        .headers()
        .get("content-type")
        .expect("should have Content-Type header")
        .to_str()
        .unwrap();
    assert!(
        content_type.starts_with("image/"),
        "avatar content type should be an image type, got: {}",
        content_type
    );
}

// ===========================================================================
// Avatar endpoints
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn get_avatars_returns_list() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/avatars")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Vec<Value> = test::read_body_json(resp).await;
    // Avatars may or may not be seeded depending on whether minifigs/ exists
    // in the test environment, but the endpoint should return a valid JSON array
    assert!(body.is_empty() || body[0].get("avatar_id").is_some());
}

#[actix_web::test]
#[ignore]
async fn get_avatars_requires_auth() {
    let state = test_state().await;
    let app = test_app!(state);
    // Ensure admin exists
    let _ = register_admin(&app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/avatars")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "listing avatars should require auth");
}

#[actix_web::test]
#[ignore]
async fn get_avatar_not_found() {
    let state = test_state().await;
    let app = test_app!(state);
    let _ = register_admin(&app).await;

    let fake_id = Uuid::now_v7();
    // Avatar image endpoint is public (no JWT required)
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/avatars/{}", fake_id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "nonexistent avatar should return 404");
}

#[actix_web::test]
#[ignore]
async fn set_avatar_nonexistent_avatar_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;
    let user_id = admin_user_id_from_token(token);

    let fake_avatar_id = Uuid::now_v7();
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}/avatar", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"avatar_id": fake_avatar_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "setting a nonexistent avatar should return 404"
    );
}

#[actix_web::test]
#[ignore]
async fn set_avatar_requires_self_or_admin() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let admin_user_id = admin_user_id_from_token(admin_token);

    let suffix = Uuid::now_v7();
    let email = format!("avatar-rbac-{suffix}@test.local");
    let (user_auth, _user_id) =
        create_and_login_user(&app, admin_token, "AV", "User", &email, "Very Secret").await;
    let user_token = &user_auth.access_token;

    // Regular user tries to set admin's avatar → 403
    let fake_avatar_id = Uuid::now_v7();
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}/avatar", admin_user_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .set_json(json!({"avatar_id": fake_avatar_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not set another user's avatar"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/users/{}",
            admin_user_id_from_token(&user_auth.access_token)
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn remove_avatar_nonexistent_user_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let fake_user_id = Uuid::now_v7();
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}/avatar", fake_user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    // Admin can bypass RBAC but the user doesn't exist → 404 from set_user_avatar
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
#[ignore]
async fn remove_avatar_succeeds_for_self() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;
    let user_id = admin_user_id_from_token(token);

    // Remove avatar (even if none is set, it should succeed and return user)
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}/avatar", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let user: Value = test::read_body_json(resp).await;
    assert!(
        user["avatar_id"].is_null(),
        "avatar_id should be null after removal"
    );
}
