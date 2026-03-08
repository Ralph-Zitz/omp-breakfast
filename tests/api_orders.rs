//! Team order and order item CRUD, pickup user, due date, and reopen tests.

mod common;

use actix_web::test;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use common::*;
use serde_json::{Value, json};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Team orders CRUD (requires team membership)
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_and_list_team_orders() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("OrderTeam-{suffix}")).await;

    // Create a new team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"duedate": "2026-03-15"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // List orders — should include the new one
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let orders = paginated_items(test::read_body_json(resp).await);
    assert!(
        orders
            .iter()
            .any(|o| o["teamorders_id"].as_str() == Some(&order_id))
    );

    // Delete the order
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Cleanup team
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn get_single_team_order_returns_details() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("OrderDetail-{suffix}")).await;

    // Create a new team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"duedate": "2026-03-20"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // GET single order by ID
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "GET single order should return 200");
    let fetched: Value = test::read_body_json(resp).await;
    assert_eq!(
        fetched["teamorders_id"].as_str(),
        Some(order_id.as_str()),
        "returned order ID should match"
    );
    assert_eq!(
        fetched["duedate"].as_str(),
        Some("2026-03-20"),
        "duedate should match"
    );
    assert!(
        fetched["closed"].as_bool() == Some(false),
        "new order should be open"
    );

    // GET nonexistent order should return 404
    let fake_id = uuid::Uuid::now_v7();
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, fake_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "nonexistent order should return 404");

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Team RBAC: non-member cannot mutate team
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_member_cannot_create_team_order() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    // Create a team and a user that is NOT a member of it
    let team_id = create_test_team(&app, admin_token, &format!("NoMemberTeam-{suffix}")).await;
    let outsider_email = format!("outsider-nmo-{suffix}@test.local");
    let (outsider_auth, outsider_id) = create_and_login_user(
        &app,
        admin_token,
        "Out",
        "Sider",
        &outsider_email,
        "securepassword",
    )
    .await;

    // Try to create order → should be 403 (not a member)
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", outsider_auth.access_token),
        ))
        .set_json(json!({"duedate": "2026-03-15"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-member should not create team orders"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", outsider_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Admin bypass on team-scoped operations
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn admin_can_manage_any_team_orders() {
    let state = test_state().await;
    let app = test_app!(state);

    // Admin is NOT a member of the target team — bypass should allow order management
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;
    let suffix = Uuid::now_v7();

    // Create a team the admin is NOT a member of
    let team_id = create_test_team(&app, token, &format!("OrderBypass {}", suffix)).await;

    // Admin creates order on that team (not a member) → should succeed via bypass
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-06-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "admin should create orders on any team via bypass"
    );
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap();

    // Admin deletes the order → should succeed
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should delete orders on any team via bypass"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Order items CRUD (items within a team order)
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_get_update_delete_order_item() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();

    // Create a Team Admin user with a team
    let ta_email = format!("ta-oi-{}@test.local", uid);
    let (ta_auth, ta_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "OrderItems",
        &ta_email,
        "securepassword",
    )
    .await;
    let token = &ta_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("OITeam-{}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_user_id, &ta_role_id).await;

    // Create a catalog item
    let item_id = create_test_item(&app, admin_token, &format!("OIItem-{}", uid), 5.50).await;

    // Create a team order to hold items
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-07-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // --- Create order item ---
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 5}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "should create order item");
    let order_item: Value = test::read_body_json(resp).await;
    assert_eq!(order_item["orders_item_id"].as_str().unwrap(), item_id);
    assert_eq!(order_item["amt"].as_i64().unwrap(), 5);
    assert_eq!(order_item["orders_team_id"].as_str().unwrap(), team_id);

    // --- Get single order item ---
    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "should get order item");
    let fetched: Value = test::read_body_json(resp).await;
    assert_eq!(fetched["amt"].as_i64().unwrap(), 5);

    // --- List order items ---
    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let list = paginated_items(test::read_body_json(resp).await);
    assert!(
        list.iter()
            .any(|i| i["orders_item_id"].as_str() == Some(&item_id)),
        "list should contain the created order item"
    );

    // --- Update order item ---
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"amt": 10}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "should update order item");
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["amt"].as_i64().unwrap(), 10);

    // --- Delete order item ---
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "should delete order item");

    // --- Verify deletion ---
    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "deleted order item should no longer exist"
    );

    // Cleanup: delete order, item, user, team
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;

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

#[actix_web::test]
#[ignore]
async fn duplicate_order_item_returns_409() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();
    let ta_email = format!("ta-dup-oi-{}@test.local", uid);
    let (ta_auth, ta_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "DupOI",
        &ta_email,
        "securepassword",
    )
    .await;
    let token = &ta_auth.access_token;

    let team_name = format!("DupOI-Team-{}", uid);
    let team_id = create_test_team(&app, admin_token, &team_name).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_user_id, &ta_role_id).await;

    let item_name = format!("DupOI-Item-{}", uid);
    let item_id = create_test_item(&app, admin_token, &item_name, 3.50).await;

    // Create a team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-08-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Add item to order
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 2}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // Try adding the same item again → should conflict
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 3}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        409,
        "duplicate order item should return conflict"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn non_member_cannot_create_order_item() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();
    let ta_email = format!("ta-nonmem-oi-{}@test.local", uid);
    let (ta_auth, ta_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "NonMemOI",
        &ta_email,
        "securepassword",
    )
    .await;
    let token_ta = &ta_auth.access_token;

    let team_name = format!("NonMemOI-Team-{}", uid);
    let team_id = create_test_team(&app, admin_token, &team_name).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_user_id, &ta_role_id).await;

    let item_name = format!("NonMemOI-Item-{}", uid);
    let item_id = create_test_item(&app, admin_token, &item_name, 4.00).await;

    // Create order as team admin
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token_ta)))
        .set_json(json!({"duedate": "2026-09-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Create a user who is NOT a member of this team
    let outsider_email = format!("outsider-oi-{}@test.local", uid);
    let (outsider_auth, outsider_id) = create_and_login_user(
        &app,
        admin_token,
        "Outsider",
        "User",
        &outsider_email,
        "securepassword",
    )
    .await;
    let outsider_token = &outsider_auth.access_token;

    // Outsider tries to add an item to the order → should be forbidden
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", outsider_token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 1}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-member should not be able to add order items"
    );

    // Outsider tries to update an order item → should be forbidden
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", outsider_token)))
        .set_json(json!({"amt": 99}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-member should not be able to update order items"
    );

    // Outsider tries to delete an order item → should be forbidden
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", outsider_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-member should not be able to delete order items"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", outsider_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn admin_can_manage_order_items_on_any_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let uid = Uuid::now_v7();

    // Create a team that the admin is NOT a member of
    let team_name = format!("AdminBypass-Team-{}", uid);
    let team_id = create_test_team(&app, token, &team_name).await;

    let item_name = format!("AdminBypass-Item-{}", uid);
    let item_id = create_test_item(&app, token, &item_name, 5.00).await;

    // Admin creates order on a team (not a member) → bypass
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-10-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Admin creates order item → bypass
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 7}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "admin should create order items on any team via bypass"
    );

    // Admin updates order item → bypass
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"amt": 12}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should update order items on any team via bypass"
    );

    // Admin deletes order item → bypass
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should delete order items on any team via bypass"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Closed order enforcement
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn closed_order_rejects_add_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let uid = Uuid::now_v7();
    let team_name = format!("ClosedAdd-Team-{}", uid);
    let team_id = create_test_team(&app, token, &team_name).await;

    let item_name = format!("ClosedAdd-Item-{}", uid);
    let item_id = create_test_item(&app, token, &item_name, 2.50).await;

    // Create a team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-12-25"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Close the order
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"closed": true}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "should close the order");
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["closed"], json!(true));

    // Try to add an item to the closed order → should be 403
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 2}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "adding items to a closed order should return 403"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn closed_order_rejects_update_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let uid = Uuid::now_v7();
    let team_name = format!("ClosedUpd-Team-{}", uid);
    let team_id = create_test_team(&app, token, &team_name).await;

    let item_name = format!("ClosedUpd-Item-{}", uid);
    let item_id = create_test_item(&app, token, &item_name, 2.50).await;

    // Create a team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-12-26"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Add an item while the order is still open
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 3}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "should add item to open order");

    // Close the order
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"closed": true}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Try to update the item on the closed order → should be 403
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"amt": 10}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "updating items on a closed order should return 403"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn closed_order_rejects_delete_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let uid = Uuid::now_v7();
    let team_name = format!("ClosedDel-Team-{}", uid);
    let team_id = create_test_team(&app, token, &team_name).await;

    let item_name = format!("ClosedDel-Item-{}", uid);
    let item_id = create_test_item(&app, token, &item_name, 2.50).await;

    // Create a team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-12-27"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Add an item while the order is still open
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 1}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "should add item to open order");

    // Close the order
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"closed": true}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Try to delete the item from the closed order → should be 403
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "deleting items from a closed order should return 403"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn reopened_order_allows_item_mutations() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let uid = Uuid::now_v7();
    let team_name = format!("Reopen-Team-{}", uid);
    let team_id = create_test_team(&app, token, &team_name).await;

    let item_name = format!("Reopen-Item-{}", uid);
    let item_id = create_test_item(&app, token, &item_name, 2.50).await;

    // Create a team order with an item
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-12-28"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Add an item to the order
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 3}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // Close the order
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"closed": true}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Reopen the order via /reopen endpoint — creates a duplicate
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/reopen",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_order: Value = test::read_body_json(resp).await;
    let new_order_id = new_order["teamorders_id"].as_str().unwrap().to_string();

    // The new order should have a different ID
    assert_ne!(
        new_order_id, order_id,
        "reopened order should have a new ID"
    );
    // The new order should be open with no duedate and no pickup user
    assert_eq!(new_order["closed"].as_bool(), Some(false));
    assert!(new_order["duedate"].is_null());
    assert!(new_order["pickup_user_id"].is_null());

    // The old order should still be closed
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let old_order: Value = test::read_body_json(resp).await;
    assert_eq!(old_order["closed"].as_bool(), Some(true));

    // The new order should have the same items as the old one
    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, new_order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let items: Value = test::read_body_json(resp).await;
    assert_eq!(items["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        items["items"][0]["orders_item_id"].as_str().unwrap(),
        item_id
    );
    assert_eq!(items["items"][0]["amt"].as_i64(), Some(3));

    // Adding items to the reopened order should succeed
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, new_order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 5}))
        .to_request();
    let _resp = test::call_service(&app, req).await;
    // This updates the existing item (same item_id), expect 409 or success depending on unique constraint
    // Let's create a second item to add cleanly
    let item_name2 = format!("Reopen-Item2-{}", uid);
    let item_id2 = create_test_item(&app, token, &item_name2, 3.00).await;
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, new_order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id2, "amt": 2}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "should add items to a reopened order");

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}",
            team_id, new_order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id2))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

// ===========================================================================
// Bulk delete team orders (#204)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn admin_can_bulk_delete_team_orders() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;
    let uid = Uuid::now_v7();

    // Create a temp team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": format!("BulkDeleteOrders {}", uid), "descr": "temp"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap().to_string();

    // Create two orders on the team
    for _ in 0..2 {
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
    }

    // Bulk delete all orders
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should bulk-delete team orders");
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["deleted"], true);

    // Verify no orders remain
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let orders = paginated_items(test::read_body_json(resp).await);
    assert!(orders.is_empty(), "all orders should be deleted");

    // Clean up team
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// #264 — Empty order items list returns 200 with empty items array
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn empty_order_items_returns_200_with_empty_list() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (ta_auth, ta_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "EmptyOI",
        &format!("ta.emptyoi.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let ta_token = &ta_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("EmptyOITeam {}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_id, &ta_role_id).await;

    // Create a fresh order (no items)
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({"duedate": "2026-09-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // List items for this empty order
    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let items = paginated_items(test::read_body_json(resp).await);
    assert!(items.is_empty(), "new order should have zero items");

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// #290 — Member cannot bulk-delete team orders
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn member_cannot_bulk_delete_team_orders() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (member_auth, member_id) = create_and_login_user(
        &app,
        admin_token,
        "Member",
        "BulkDel",
        &format!("member.bulkdel.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let member_token = &member_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("MemberBulkDelTeam {}", uid)).await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &member_id, &member_role_id).await;

    // Member tries to bulk-delete all team orders → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", member_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "Member should not be able to bulk-delete team orders"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", member_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// #291 — Non-member cannot PUT/DELETE single team order
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_member_cannot_update_team_order() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    // Create a non-member user
    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "NonMember",
        "Order",
        &format!("nonmember.order.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let user_token = &user_auth.access_token;

    // Create a team the user is NOT a member of
    let team_id = create_test_team(&app, admin_token, &format!("NonMemberOrderTeam {}", uid)).await;

    // Create an order via admin
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Non-member tries PUT → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .set_json(json!({"closed": true}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-member should not update team orders"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn non_member_cannot_delete_team_order() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    // Create a non-member user
    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "NonMember",
        "DelOrder",
        &format!("nonmember.delorder.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let user_token = &user_auth.access_token;

    // Create a team the user is NOT a member of
    let team_id =
        create_test_team(&app, admin_token, &format!("NonMemberDelOrderTeam {}", uid)).await;

    // Create an order via admin
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Non-member tries DELETE → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-member should not delete team orders"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn create_team_order_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Create temp team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": format!("LocHdrOrder-{}", Uuid::now_v7()), "descr": "temp"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap().to_string();

    // Create order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let location = resp
        .headers()
        .get("Location")
        .expect("201 should include Location header");
    let loc_str = location.to_str().unwrap();
    assert!(
        loc_str.contains(&format!("/api/v1.0/teams/{}/orders/", team_id)),
        "Location should contain team order path, got: {}",
        loc_str
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn create_order_item_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;
    let suffix = Uuid::now_v7();

    // Create temp team + order
    let team_id = create_test_team(&app, token, &format!("LocHdrTeam-{suffix}")).await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Create a test item
    let item_id = create_test_item(&app, token, &format!("LocHdrItem-{}", team_id), 2.50).await;

    // Create order item
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 2}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let location = resp
        .headers()
        .get("Location")
        .expect("201 should include Location header");
    let loc_str = location.to_str().unwrap();
    assert!(
        loc_str.contains(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/",
            team_id, order_id
        )),
        "Location should contain order item path, got: {}",
        loc_str
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// #295 — GET orders for nonexistent team returns 200 with empty list
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn get_orders_for_nonexistent_team_returns_empty_list() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let fake_team_id = Uuid::now_v7();
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/orders", fake_team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "should return 200 for nonexistent team");
    let body: Value = test::read_body_json(resp).await;
    let orders = body["items"].as_array().expect("should have items array");
    assert!(orders.is_empty(), "should return empty list");
    assert_eq!(body["total"], 0);
}

// ===========================================================================
// #429 — Team Admin can bulk-delete their own team's orders
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn team_admin_can_bulk_delete_own_team_orders() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7().to_string();

    // Create a fresh isolated team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"tname": format!("TeamAdminBulkDelete429-{}", suffix), "descr": "temp"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap().to_string();

    // Create a temp user with known password
    let ta_email = format!("ta_bulk_delete_429-{}@test.local", suffix);
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "TeamAdmin",
            "lastname": "BulkDelete",
            "email": ta_email,
            "password": "securepassword429"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let ta_user: Value = test::read_body_json(resp).await;
    let ta_user_id = ta_user["user_id"].as_str().unwrap().to_string();

    // Get the "Team Admin" role_id
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let roles = paginated_items(test::read_body_json(resp).await);
    let team_admin_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Team Admin"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Add temp user to team as Team Admin
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"user_id": ta_user_id, "role_id": team_admin_role_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "admin should add team admin to team");

    // Admin creates 2 orders on the team
    for _ in 0..2 {
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
            .insert_header(("Authorization", format!("Bearer {}", admin_token)))
            .set_json(json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
    }

    // Login as the Team Admin user
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:securepassword429", ta_email))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "team admin should be able to login");
    let ta_auth: Auth = test::read_body_json(resp).await;
    let ta_token = &ta_auth.access_token;

    // Team Admin bulk-deletes all orders on their team → should succeed
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "team admin should be able to bulk-delete own team orders"
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["deleted"], true);

    // Verify no orders remain
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let orders = paginated_items(test::read_body_json(resp).await);
    assert!(orders.is_empty(), "all orders should have been deleted");

    // Cleanup: delete user and team
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
// #430 — Team Admin can update an order created by another member
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn team_admin_can_update_order_by_another_member() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7().to_string();
    let m430_email = format!("m430-{}@test.local", suffix);
    let ta430_email = format!("ta430-{}@test.local", suffix);

    // Create isolated team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"tname": format!("TeamAdminUpdateOrder430-{}", suffix), "descr": "temp"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap().to_string();

    // Create a member and a team admin
    for (email, password) in &[
        (m430_email.as_str(), "memberpass430"),
        (ta430_email.as_str(), "teamadminpass430"),
    ] {
        let req = test::TestRequest::post()
            .uri("/api/v1.0/users")
            .insert_header(("Authorization", format!("Bearer {}", admin_token)))
            .set_json(json!({
                "firstname": "Test",
                "lastname": "User",
                "email": email,
                "password": password
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
    }

    // Get user IDs
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let member_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some(m430_email.as_str()))
        .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_string();
    let ta_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some(ta430_email.as_str()))
        .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get role IDs
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let roles = paginated_items(test::read_body_json(resp).await);
    let member_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Member"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();
    let team_admin_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Team Admin"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Add both users to the team
    for (user_id, role_id) in &[
        (member_id.as_str(), member_role_id.as_str()),
        (ta_id.as_str(), team_admin_role_id.as_str()),
    ] {
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1.0/teams/{}/users", team_id))
            .insert_header(("Authorization", format!("Bearer {}", admin_token)))
            .set_json(json!({"user_id": user_id, "role_id": role_id}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
    }

    // Member creates an order
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:memberpass430", m430_email))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let member_auth: Auth = test::read_body_json(resp).await;
    let member_token = &member_auth.access_token;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", member_token)))
        .set_json(json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Team Admin updates the order created by the member
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:teamadminpass430", ta430_email))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let ta_auth: Auth = test::read_body_json(resp).await;
    let ta_token = &ta_auth.access_token;

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({"closed": false}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "team admin should be able to update a member's order"
    );

    // Cleanup
    for user_id in &[member_id.as_str(), ta_id.as_str()] {
        let req = test::TestRequest::delete()
            .uri(&format!("/api/v1.0/users/{}", user_id))
            .insert_header(("Authorization", format!("Bearer {}", admin_token)))
            .to_request();
        test::call_service(&app, req).await;
    }
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// #431 — Regular member can update and delete their own order
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn member_can_update_and_delete_own_order() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7().to_string();
    let m431_email = format!("m431-{}@test.local", suffix);

    // Create isolated team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"tname": format!("MemberOwnOrder431-{}", suffix), "descr": "temp"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap().to_string();

    // Create a member user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "Member",
            "lastname": "OwnOrder",
            "email": m431_email,
            "password": "memberpass431"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let member_user: Value = test::read_body_json(resp).await;
    let member_id = member_user["user_id"].as_str().unwrap().to_string();

    // Get "Member" role_id
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let roles = paginated_items(test::read_body_json(resp).await);
    let member_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Member"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Add member to team
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"user_id": member_id, "role_id": member_role_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // Login as member
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:memberpass431", m431_email))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "member should be able to login");
    let member_auth: Auth = test::read_body_json(resp).await;
    let member_token = &member_auth.access_token;

    // Member creates own order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", member_token)))
        .set_json(json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "member should be able to create an order"
    );
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Member updates own order
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", member_token)))
        .set_json(json!({"closed": false}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "member should be able to update own order"
    );

    // Member deletes own order
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", member_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "member should be able to delete own order"
    );

    // Cleanup: delete user and team
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", member_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn delete_nonexistent_team_order_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let team_id = create_test_team(&app, token, &format!("DelOrder404-{}", Uuid::now_v7())).await;
    let fake_order_id = Uuid::now_v7();

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}",
            team_id, fake_order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "DELETE nonexistent team order should be 404"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn delete_nonexistent_order_item_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let team_id = create_test_team(&app, token, &format!("DelOrdItem404-{}", Uuid::now_v7())).await;

    // Create a team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-08-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    let fake_item_id = Uuid::now_v7();
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, fake_item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "DELETE nonexistent order item should be 404"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn update_nonexistent_team_order_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let team_id = create_test_team(&app, token, &format!("UpdOrder404-{}", Uuid::now_v7())).await;
    let fake_order_id = Uuid::now_v7();

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}",
            team_id, fake_order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"closed": true}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "PUT nonexistent team order should be 404"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn update_nonexistent_order_item_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let team_id = create_test_team(&app, token, &format!("UpdOrdItem404-{}", Uuid::now_v7())).await;

    // Create a team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-08-15"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    let fake_item_id = Uuid::now_v7();
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, fake_item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"amt": 5}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "PUT nonexistent order item should be 404"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// RBAC edge cases (#323, #355, #388)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn member_cannot_update_another_members_order_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create team, item, and two members
    let uid = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("OrdItemRBAC-{}", uid)).await;
    let item_id = create_test_item(&app, admin_token, &format!("OrdItem-{}", uid), 3.50).await;
    let member_role = find_role_id(&app, admin_token, "Member").await;

    let (u1_auth, u1_id) = create_and_login_user(
        &app,
        admin_token,
        "U1",
        "OrdItem",
        &format!("u1-orditem-{}@test.local", uid),
        "Very Secret",
    )
    .await;
    let u1_token = &u1_auth.access_token;
    add_member(&app, admin_token, &team_id, &u1_id, &member_role).await;

    let (u2_auth, u2_id) = create_and_login_user(
        &app,
        admin_token,
        "U2",
        "OrdItem",
        &format!("u2-orditem-{}@test.local", uid),
        "Very Secret",
    )
    .await;
    let u2_token = &u2_auth.access_token;
    add_member(&app, admin_token, &team_id, &u2_id, &member_role).await;

    // U1 creates a team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", u1_token)))
        .set_json(json!({"duedate": "2026-09-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // U1 creates an order item
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", u1_token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 2}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // U2 (another member) tries to update U1's order item → 403
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", u2_token)))
        .set_json(json!({"amt": 99}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "member should not update another member's order item"
    );

    // U2 tries to delete U1's order item → 403
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", u2_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "member should not delete another member's order item"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, u1_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, u2_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", u1_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", u2_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn member_cannot_update_or_delete_team_order_they_did_not_create() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create team and two members
    let uid = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("OrdRBAC-{}", uid)).await;
    let member_role = find_role_id(&app, admin_token, "Member").await;

    let (u1_auth, u1_id) = create_and_login_user(
        &app,
        admin_token,
        "U1",
        "OrdOwner",
        &format!("u1-ordrbac-{}@test.local", uid),
        "Very Secret",
    )
    .await;
    let u1_token = &u1_auth.access_token;
    add_member(&app, admin_token, &team_id, &u1_id, &member_role).await;

    let (u2_auth, u2_id) = create_and_login_user(
        &app,
        admin_token,
        "U2",
        "OrdOwner",
        &format!("u2-ordrbac-{}@test.local", uid),
        "Very Secret",
    )
    .await;
    let u2_token = &u2_auth.access_token;
    add_member(&app, admin_token, &team_id, &u2_id, &member_role).await;

    // U1 creates a team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", u1_token)))
        .set_json(json!({"duedate": "2026-09-15"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // U2 tries to update U1's order → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", u2_token)))
        .set_json(json!({"closed": true}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "member should not update another member's order"
    );

    // U2 tries to delete U1's order → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", u2_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "member should not delete another member's order"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, u1_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, u2_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", u1_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", u2_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// Order item RBAC — member cannot delete another's order item (#615)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn member_cannot_delete_other_members_order_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();

    // Create a Team Admin who owns the order
    let ta_email = format!("oi-ta-{}@test.local", uid);
    let (ta_auth, ta_user_id) =
        create_and_login_user(&app, admin_token, "OI", "TA", &ta_email, "Very Secret").await;
    let ta_token = &ta_auth.access_token;

    // Create a regular member
    let mem_email = format!("oi-mem-{}@test.local", uid);
    let (mem_auth, mem_user_id) =
        create_and_login_user(&app, admin_token, "OI", "Mem", &mem_email, "Very Secret").await;
    let mem_token = &mem_auth.access_token;

    // Set up team + membership
    let team_id = create_test_team(&app, admin_token, &format!("OIRbac-{}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &ta_user_id, &ta_role_id).await;
    add_member(&app, admin_token, &team_id, &mem_user_id, &member_role_id).await;

    // Create a catalog item
    let item_id = create_test_item(&app, admin_token, &format!("OIRbacItem-{}", uid), 3.00).await;

    // Team Admin creates an order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({"duedate": "2026-07-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Member adds an item (allowed for any team member)
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", mem_token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 2}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "member should add item to order");

    // Regular member tries to delete the order item → should be 403
    // (member is not the order owner and not a team admin)
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", mem_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "member should not delete order items on order they don't own"
    );

    // Cleanup: TA deletes order item, order, then admin cleans up users/team/item
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items/{}",
            team_id, order_id, item_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", mem_user_id))
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
// Pickup user — create order with pickup_user_id
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_order_with_pickup_user() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("PickupTeam-{suffix}")).await;

    // Extract admin user_id and add admin to the new team
    let admin_user_id = admin_user_id_from_token(admin_token);
    let admin_role_id = find_role_id(&app, admin_token, "Admin").await;
    add_member(&app, admin_token, &team_id, &admin_user_id, &admin_role_id).await;

    // Create order with pickup_user_id set to admin
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"duedate": "2026-06-01", "pickup_user_id": admin_user_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "should create order with pickup user");
    let order: Value = test::read_body_json(resp).await;
    assert_eq!(
        order["pickup_user_id"].as_str(),
        Some(admin_user_id.as_str()),
        "pickup_user_id should be set"
    );
    let order_id = order["teamorders_id"].as_str().unwrap();

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn create_order_with_non_member_pickup_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("PickupNM-{suffix}")).await;

    // Create a user who is NOT a member of this team
    let email = format!("pickup-nm-{suffix}@test.local");
    let (_, outsider_id) =
        create_and_login_user(&app, admin_token, "Out", "Sider", &email, "Very Secret").await;

    // Try to create order with outsider as pickup → 422
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"duedate": "2026-06-01", "pickup_user_id": outsider_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422, "pickup user must be a team member");

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", outsider_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn member_cannot_change_assigned_pickup_user() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("PickupRBAC-{suffix}")).await;
    let admin_user_id = admin_user_id_from_token(admin_token);

    // Add admin to the new team so they can be a valid pickup user
    let admin_role_id = find_role_id(&app, admin_token, "Admin").await;
    add_member(&app, admin_token, &team_id, &admin_user_id, &admin_role_id).await;

    // Create a member user
    let m_email = format!("pickup-m-{suffix}@test.local");
    let (m_auth, m_id) =
        create_and_login_user(&app, admin_token, "Pick", "Member", &m_email, "Very Secret").await;
    let m_token = &m_auth.access_token;

    // Add member to team
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &m_id, &member_role_id).await;

    // Member creates order with pickup assigned to admin
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", m_token)))
        .set_json(json!({"pickup_user_id": admin_user_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Member tries to change pickup user → 403 (requires admin/team admin)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", m_token)))
        .set_json(json!({"pickup_user_id": m_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "member should not be able to change an assigned pickup user"
    );

    // Admin CAN change pickup user
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"pickup_user_id": m_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should be able to change pickup user"
    );
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["pickup_user_id"].as_str(), Some(m_id.as_str()));

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", m_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn member_can_set_pickup_when_unassigned() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("PickupUnset-{suffix}")).await;

    // Create a member user
    let m_email = format!("pickup-u-{suffix}@test.local");
    let (m_auth, m_id) =
        create_and_login_user(&app, admin_token, "Pick", "Unset", &m_email, "Very Secret").await;
    let m_token = &m_auth.access_token;

    // Add member to team
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &m_id, &member_role_id).await;

    // Member creates order WITHOUT pickup
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", m_token)))
        .set_json(json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();
    assert!(
        order["pickup_user_id"].is_null(),
        "pickup should be null initially"
    );

    // Member sets pickup (first assignment — no RBAC restriction)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", m_token)))
        .set_json(json!({"pickup_user_id": m_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "member should set pickup when unassigned"
    );
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["pickup_user_id"].as_str(), Some(m_id.as_str()));

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", m_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Admin clears assigned pickup user (#694)
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn admin_can_clear_assigned_pickup_user() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let suffix = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("PickupClr-{suffix}")).await;
    let admin_user_id = admin_user_id_from_token(admin_token);

    // Add admin to the team
    let admin_role_id = find_role_id(&app, admin_token, "Admin").await;
    add_member(&app, admin_token, &team_id, &admin_user_id, &admin_role_id).await;

    // Create order with pickup_user_id set
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"pickup_user_id": admin_user_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();
    assert!(
        order["pickup_user_id"].as_str().is_some(),
        "pickup should be set"
    );

    // Admin clears pickup by sending null
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"pickup_user_id": null}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should be able to clear pickup");
    let updated: Value = test::read_body_json(resp).await;
    assert!(
        updated["pickup_user_id"].is_null(),
        "pickup_user_id should be null after clearing"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Due date cannot be in the past
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_order_rejects_past_due_date() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let team_id = create_test_team(&app, token, &format!("PastDate-{}", Uuid::now_v7())).await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2020-01-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        422,
        "creating an order with a past due date should be rejected"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn update_order_rejects_past_due_date() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let team_id = create_test_team(&app, token, &format!("PastDateUp-{}", Uuid::now_v7())).await;

    // Create order with a future date
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2027-06-15"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap();

    // Attempt to update the due date to a past date
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2020-01-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        422,
        "updating an order with a past due date should be rejected"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Reopen endpoint edge cases
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn reopen_open_order_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let team_id = create_test_team(&app, token, &format!("ReopenOpen-{}", Uuid::now_v7())).await;

    // Create an open order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2027-06-15"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap();

    // Attempt to reopen an open order
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/reopen",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        422,
        "reopening an already-open order should be rejected"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn reopen_nonexistent_order_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let team_id = create_test_team(&app, token, &format!("ReopenNone-{}", Uuid::now_v7())).await;

    let fake_order_id = Uuid::now_v7();
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/reopen",
            team_id, fake_order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

/// Non-member attempting to reopen an order should get 403.
#[actix_web::test]
#[ignore]
async fn non_member_cannot_reopen_order() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("NonMemReopen-{}", uid)).await;

    // Create and close an order as admin
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"duedate": "2027-06-15"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap();

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"closed": true}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Create a non-member user
    let outsider_email = format!("outsider-reopen-{}@test.local", uid);
    let (outsider_auth, outsider_id) = create_and_login_user(
        &app,
        admin_token,
        "Out",
        "Sider",
        &outsider_email,
        "securepassword",
    )
    .await;

    // Non-member tries to reopen → 403
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/reopen",
            team_id, order_id
        ))
        .insert_header((
            "Authorization",
            format!("Bearer {}", outsider_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-member should not be able to reopen an order"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", outsider_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

/// Team Admin can delete a single order created by another member.
#[actix_web::test]
#[ignore]
async fn team_admin_can_delete_order_by_another_member() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("TADelOther-{}", uid)).await;
    let member_role = find_role_id(&app, admin_token, "Member").await;
    let ta_role = find_role_id(&app, admin_token, "Team Admin").await;

    // Create a member and a team admin
    let (member_auth, member_id) = create_and_login_user(
        &app,
        admin_token,
        "Mem",
        "Ber",
        &format!("mem-tadel-{}@test.local", uid),
        "securepassword",
    )
    .await;
    add_member(&app, admin_token, &team_id, &member_id, &member_role).await;

    let (ta_auth, ta_id) = create_and_login_user(
        &app,
        admin_token,
        "Team",
        "Admin",
        &format!("ta-tadel-{}@test.local", uid),
        "securepassword",
    )
    .await;
    add_member(&app, admin_token, &team_id, &ta_id, &ta_role).await;

    // Member creates an order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", member_auth.access_token),
        ))
        .set_json(json!({"duedate": "2027-01-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Team admin deletes the member's order → should succeed
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "team admin should be able to delete another member's order"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", member_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}
