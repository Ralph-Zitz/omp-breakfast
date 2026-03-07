//! Role CRUD and RBAC tests.

mod common;

use actix_web::test;
use common::*;
use serde_json::{Value, json};
use uuid::Uuid;

#[actix_web::test]
#[ignore]
async fn get_nonexistent_role_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

    let fake_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/roles/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "non-existent role should return 404");
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_create_role() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let email = format!("nonadmin-cr-{suffix}@test.local");
    let (user_auth, user_id) =
        create_and_login_user(&app, admin_token, "NA", "CR", &email, "securepassword").await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/roles")
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .set_json(json!({"title": "Forbidden Role"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to create roles"
    );

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_delete_role() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let email = format!("nonadmin-dr-{suffix}@test.local");
    let (user_auth, user_id) =
        create_and_login_user(&app, admin_token, "NA", "DR", &email, "securepassword").await;

    // Get the "Guest" role ID (safe to test against)
    let role_id = find_role_id(&app, admin_token, "Guest").await;

    // Non-admin tries to delete → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/roles/{}", role_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to delete roles"
    );

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_update_role() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "NonAdmin",
        "Role",
        &format!("nonadmin.role.{}@test.local", uid),
        "securepassword",
    )
    .await;

    let role_id = find_role_id(&app, admin_token, "Guest").await;

    // Non-admin tries to update → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/roles/{}", role_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .set_json(json!({"title": "Forbidden Update"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to update roles"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn admin_can_update_role() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Create a temp role
    let req = test::TestRequest::post()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"title": "TempUpdateRole"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let role: Value = test::read_body_json(resp).await;
    let role_id = role["role_id"].as_str().unwrap();

    // Update the role
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/roles/{}", role_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"title": "UpdatedTempRole"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should be able to update roles");
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["title"], "UpdatedTempRole");

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/roles/{}", role_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn create_role_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"title": format!("LocHdrRole-{}", Uuid::now_v7()), "descr": "temp"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let location = resp
        .headers()
        .get("Location")
        .expect("201 should include Location header");
    assert!(
        location.to_str().unwrap().contains("/api/v1.0/roles/"),
        "Location should contain role path"
    );
    let body: Value = test::read_body_json(resp).await;
    let role_id = body["role_id"].as_str().unwrap();

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/roles/{}", role_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn delete_nonexistent_role_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let fake_id = Uuid::now_v7();

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/roles/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "DELETE nonexistent role should be 404");
}

#[actix_web::test]
#[ignore]
async fn update_nonexistent_role_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let fake_id = Uuid::now_v7();

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/roles/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"title": "Ghost Role"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "PUT nonexistent role should be 404");
}
