//! Item (breakfast item) CRUD and RBAC tests.

mod common;

use actix_web::test;
use common::*;
use serde_json::{Value, json};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Items CRUD
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn get_items_returns_created_data() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Create a test item so there is at least one
    let suffix = Uuid::now_v7();
    let item_id = create_test_item(&app, token, &format!("TestItem-{}", suffix), 2.50).await;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        !body["items"].as_array().unwrap().is_empty(),
        "should have at least one item"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn create_update_delete_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Create
    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"descr": "test croissant", "price": "3.50"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let item: Value = test::read_body_json(resp).await;
    let item_id = item["item_id"].as_str().unwrap();

    // Update
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"descr": "updated croissant", "price": "4.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["descr"], "updated croissant");

    // Get
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Delete
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
#[ignore]
async fn create_item_with_empty_descr_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"descr": "", "price": "1.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422, "empty description should be rejected");
}

#[actix_web::test]
#[ignore]
async fn get_nonexistent_item_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

    let fake_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/items/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "non-existent item should return 404");
}

// ---------------------------------------------------------------------------
// 409 conflict responses for duplicate creation
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_duplicate_item_returns_409() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;
    let dup_item_name = format!("dup-item-{}", Uuid::now_v7());

    // Create an item
    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"descr": dup_item_name, "price": "1.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let created: Value = test::read_body_json(resp).await;
    let item_id = created["item_id"].as_str().unwrap();

    // Try to create a second item with the same description → 409
    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"descr": dup_item_name, "price": "2.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        409,
        "duplicate item description should return 409"
    );

    // Clean up: delete the item
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Items/Roles CUD: non-admin forbidden
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_create_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let email = format!("nonadmin-ci-{suffix}@test.local");
    let (user_auth, user_id) =
        create_and_login_user(&app, admin_token, "NA", "CI", &email, "securepassword").await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .set_json(json!({"descr": "forbidden item", "price": "1.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to create items"
    );

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_update_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let email = format!("nonadmin-ui-{suffix}@test.local");
    let (user_auth, user_id) =
        create_and_login_user(&app, admin_token, "NA", "UI", &email, "securepassword").await;

    // Create an item to test against
    let item_id =
        create_test_item(&app, admin_token, &format!("NAUpdateItem-{suffix}"), 1.00).await;

    // Non-admin tries to update → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .set_json(json!({"descr": "hacked item", "price": "0.01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to update items"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_delete_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let email = format!("nonadmin-di-{suffix}@test.local");
    let (user_auth, user_id) =
        create_and_login_user(&app, admin_token, "NA", "DI", &email, "securepassword").await;

    // Create an item to test against
    let item_id =
        create_test_item(&app, admin_token, &format!("NADeleteItem-{suffix}"), 1.00).await;

    // Non-admin tries to delete → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to delete items"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_create_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin = register_admin(&app).await;
    let admin_token = &admin.access_token;

    // Create a user and make them Team Admin
    let suffix = Uuid::now_v7();
    let (ta_auth, ta_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "NoItem",
        &format!("ta-noitem-{}@test.local", suffix),
        "password123",
    )
    .await;
    let team_id = create_test_team(&app, admin_token, &format!("TAItemTeam-{}", suffix)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_user_id, &ta_role_id).await;

    // Team Admin tries to create an item → 403
    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", ta_auth.access_token)))
        .set_json(json!({"descr": "forbidden item", "price": "1.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin should not be able to create items (requires global admin)"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_user_id))
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
// Location header in create responses (#178)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_item_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"descr": format!("loc-item-{}", Uuid::now_v7()), "price": "1.50"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    let location = resp
        .headers()
        .get("Location")
        .expect("201 response should include Location header");
    let loc_str = location.to_str().unwrap();
    assert!(
        loc_str.contains("/api/v1.0/items/"),
        "Location header should contain the item path, got: {}",
        loc_str
    );

    // Clean up: get item_id from body
    let body: Value = test::read_body_json(resp).await;
    let item_id = body["item_id"].as_str().unwrap();
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// #433 — Creating an item with a negative price returns 422
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_item_with_negative_price_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "descr": "negative price item",
            "price": "-1.00"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        422,
        "negative price should return 422 Unprocessable Entity"
    );
}

// ===========================================================================
// DELETE nonexistent resources → 404 (#296)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn delete_nonexistent_item_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let fake_id = Uuid::now_v7();

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "DELETE nonexistent item should be 404");
}

#[actix_web::test]
#[ignore]
async fn update_nonexistent_item_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let fake_id = Uuid::now_v7();

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/items/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"descr": "Ghost Item", "price": "1.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "PUT nonexistent item should be 404");
}
