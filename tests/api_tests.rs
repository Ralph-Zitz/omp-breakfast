//! Integration tests for the breakfast API.
//!
//! These tests require a running PostgreSQL instance initialized via Refinery
//! migrations and seeded with `database_seed.sql`.
//! The easiest way to run them:
//!   make test-integration
//!
//! Or manually:
//!   docker compose up -d postgres && docker compose run --rm postgres-setup
//!   cargo test --test api_tests -- --ignored

use actix_web::{App, test, web::Data};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use breakfast::{models::*, routes::routes};
use serde_json::{Value, json};
use std::net::SocketAddr;
use uuid::Uuid;

/// Fake peer address for test requests (required by actix-governor's PeerIpKeyExtractor).
const PEER: SocketAddr = SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
    12345,
);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the `items` array from a paginated response envelope.
fn paginated_items(body: Value) -> Vec<Value> {
    body["items"]
        .as_array()
        .expect("response should have 'items' array")
        .to_vec()
}

/// Build a `Data<State>` pointing at the local Docker postgres (no TLS).
///
/// Reads `TEST_DB_PORT` from the environment (default: 5432) so that
/// `make test-integration` can point at the isolated test container on 5433.
async fn test_state() -> Data<State> {
    let db_port: u16 = std::env::var("TEST_DB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(5432);

    let mut pg_cfg = deadpool_postgres::Config::new();
    pg_cfg.user = Some("actix".to_string());
    pg_cfg.password = Some("actix".to_string());
    pg_cfg.dbname = Some("actix".to_string());
    pg_cfg.host = Some("localhost".to_string());
    pg_cfg.port = Some(db_port);
    let pool = pg_cfg
        .create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("failed to create test pool");
    Data::new(State {
        pool,
        jwtsecret: "Very Secret".to_string(),
        cache: dashmap::DashMap::new(),
        token_blacklist: dashmap::DashMap::new(),
        login_attempts: dashmap::DashMap::new(),
    })
}

/// Create a test `App` wired with the given state.
macro_rules! test_app {
    ($state:expr) => {
        test::init_service(App::new().app_data($state.clone()).configure(routes)).await
    };
}

/// Helper: authenticate the seed admin user and return the `Auth` response.
async fn login_admin(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
) -> Auth {
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!("Basic {}", STANDARD.encode("admin@admin.com:Very Secret")),
        ))
        .to_request();

    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 200, "admin login should succeed");
    test::read_body_json(resp).await
}

// ---------------------------------------------------------------------------
// Health endpoint
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn health_returns_up() {
    let state = test_state().await;
    let app = test_app!(state);

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["up"], json!(true));
}

// ---------------------------------------------------------------------------
// Auth flow (basic auth → token pair)
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn auth_returns_token_pair() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_admin(&app).await;
    assert!(!auth.access_token.is_empty());
    assert!(!auth.refresh_token.is_empty());
    assert_eq!(auth.token_type, "Bearer");
    assert!(auth.expires_in > 0);
}

#[actix_web::test]
#[ignore]
async fn auth_rejects_wrong_password() {
    let state = test_state().await;
    let app = test_app!(state);

    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode("admin@admin.com:wrong-password")
            ),
        ))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
#[ignore]
async fn auth_rejects_unknown_user() {
    let state = test_state().await;
    let app = test_app!(state);

    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!("Basic {}", STANDARD.encode("unknown@example.com:anything")),
        ))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status() == 401 || resp.status() == 500,
        "unknown user should fail"
    );
}

// ---------------------------------------------------------------------------
// JWT-protected endpoints
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn protected_endpoint_requires_auth() {
    let state = test_state().await;
    let app = test_app!(state);

    let req = test::TestRequest::get().uri("/api/v1.0/users").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "missing bearer should be rejected");
}

#[actix_web::test]
#[ignore]
async fn protected_endpoint_rejects_invalid_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", "Bearer invalid.token.here"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
#[ignore]
async fn get_users_with_valid_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_admin(&app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["items"].as_array().unwrap().len() >= 5,
        "seed data has 5 users"
    );
}

#[actix_web::test]
#[ignore]
async fn get_teams_with_valid_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_admin(&app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["items"].as_array().unwrap().len() >= 2,
        "seed data has 2 teams"
    );
}

#[actix_web::test]
#[ignore]
async fn get_roles_with_valid_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_admin(&app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["items"].as_array().unwrap().len() >= 3,
        "seed data has 3 roles"
    );
}

#[actix_web::test]
#[ignore]
async fn refresh_token_rejects_access_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_admin(&app).await;

    // Try using the access token on /auth/refresh — should be rejected
    let req = test::TestRequest::post()
        .uri("/auth/refresh")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        401,
        "access token should not work as refresh"
    );
}

// ---------------------------------------------------------------------------
// Token refresh flow
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn refresh_token_issues_new_pair() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_admin(&app).await;

    // Use the refresh token to get a new pair
    let req = test::TestRequest::post()
        .uri("/auth/refresh")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.refresh_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let new_auth: Auth = test::read_body_json(resp).await;
    assert!(!new_auth.access_token.is_empty());
    assert!(!new_auth.refresh_token.is_empty());
    assert_ne!(
        new_auth.access_token, auth.access_token,
        "new access token should differ"
    );
}

#[actix_web::test]
#[ignore]
async fn old_refresh_token_is_revoked_after_rotation() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_admin(&app).await;

    // Use refresh token → old one should be revoked
    let req = test::TestRequest::post()
        .uri("/auth/refresh")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.refresh_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Try using the old refresh token again — should fail
    let req2 = test::TestRequest::post()
        .uri("/auth/refresh")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.refresh_token)))
        .to_request();

    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), 401, "old refresh token should be revoked");
}

// ---------------------------------------------------------------------------
// Token revocation flow
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn revoke_endpoint_invalidates_access_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_admin(&app).await;

    // Revoke the access token via /auth/revoke
    let req = test::TestRequest::post()
        .uri("/auth/revoke")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"token": auth.access_token}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Now use the revoked access token — should be rejected
    let req2 = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();

    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), 401, "revoked token should be rejected");
}

#[actix_web::test]
#[ignore]
async fn revoke_endpoint_rejects_invalid_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_admin(&app).await;

    let req = test::TestRequest::post()
        .uri("/auth/revoke")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"token": "not.a.real.token"}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "invalid token should be rejected");
}

// ---------------------------------------------------------------------------
// Full end-to-end: login → use API → refresh → use API → revoke → denied
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn full_lifecycle() {
    let state = test_state().await;
    let app = test_app!(state);

    // 1. Login
    let auth: Auth = login_admin(&app).await;

    // 2. Access a protected resource
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // 3. Refresh tokens
    let req = test::TestRequest::post()
        .uri("/auth/refresh")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.refresh_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let new_auth: Auth = test::read_body_json(resp).await;

    // 4. Use new access token
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", new_auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // 5. Revoke the new access token
    let req = test::TestRequest::post()
        .uri("/auth/revoke")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", new_auth.access_token)))
        .set_json(json!({"token": new_auth.access_token}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // 6. Revoked token should be denied
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", new_auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

// ---------------------------------------------------------------------------
// Helper: authenticate a seed user by email
// ---------------------------------------------------------------------------

async fn login_user(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    email: &str,
) -> Auth {
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:Very Secret", email))
            ),
        ))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 200, "login should succeed for {}", email);
    test::read_body_json(resp).await
}

// ---------------------------------------------------------------------------
// Items CRUD
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn get_items_returns_seed_data() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["items"].as_array().unwrap().len() >= 4,
        "seed data has 4 items"
    );
}

#[actix_web::test]
#[ignore]
async fn create_update_delete_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
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

// ---------------------------------------------------------------------------
// RBAC enforcement
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn delete_other_user_returns_forbidden() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is a Member, not a global Admin
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    // Get list of users to find another user's ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);

    // Find a user that is not U1_F
    let other_user = users
        .iter()
        .find(|u| u["email"].as_str() != Some("U1_F.U1_L@LEGO.com"))
        .unwrap();
    let other_id = other_user["user_id"].as_str().unwrap();

    // Try to delete the other user → should be 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", other_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "should not be able to delete another user"
    );
}

#[actix_web::test]
#[ignore]
async fn update_other_user_returns_forbidden() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is a Member, not a global Admin
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    // Get list of users to find another user's ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);

    let other_user = users
        .iter()
        .find(|u| u["email"].as_str() != Some("U1_F.U1_L@LEGO.com"))
        .unwrap();
    let other_id = other_user["user_id"].as_str().unwrap();

    // Try to update the other user → should be 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", other_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({
            "firstname": "Hacked",
            "lastname": "Name",
            "email": "hacked@example.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "should not be able to update another user"
    );
}

// ---------------------------------------------------------------------------
// Team orders CRUD (requires team membership)
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_and_list_team_orders() {
    let state = test_state().await;
    let app = test_app!(state);

    // Login as U4_F who is Admin of "League of Cool Coders"
    let auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get teams to find the team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get user ID for the logged-in user
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _user_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("U4_F.U4_L@LEGO.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap();

    // Create a new team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-03-15"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // List orders — should include the new one
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
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
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
#[ignore]
async fn get_single_team_order_returns_details() {
    let state = test_state().await;
    let app = test_app!(state);

    // Login as U4_F who is Admin of "League of Cool Coders"
    let auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get teams to find the team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a new team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-03-20"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // GET single order by ID
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
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
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "nonexistent order should return 404");

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

// ---------------------------------------------------------------------------
// Team RBAC: non-member cannot mutate team
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_member_cannot_create_team_order() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is a member of "League of Cool Coders" but NOT "Pixel Bakers"
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get teams to find "Pixel Bakers"
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("Pixel Bakers"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap();

    // Get user ID for the logged-in user
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _user_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("U1_F.U1_L@LEGO.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap();

    // Try to create order → should be 403 (not a member of Pixel Bakers)
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-03-15"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-member should not create team orders"
    );
}

// ---------------------------------------------------------------------------
// Team RBAC: admin-only team CRUD
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_create_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is a Member, not an Admin
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "Forbidden Team", "descr": "Should not be created"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to create teams"
    );
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_delete_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is a Member, not an Admin
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get teams
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams[0]["team_id"].as_str().unwrap();

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to delete teams"
    );
}

#[actix_web::test]
#[ignore]
async fn admin_can_create_and_delete_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // admin@admin.com is an Admin of "League of Cool Coders"
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Create a team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "Temp Admin Team", "descr": "Created by admin"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "admin should be able to create teams");
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap();

    // Delete the team
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should be able to delete teams");
}

// ---------------------------------------------------------------------------
// Validation rejection
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_user_with_invalid_email_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({
            "firstname": "Test",
            "lastname": "User",
            "email": "not-an-email",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422, "invalid email should be rejected");
}

#[actix_web::test]
#[ignore]
async fn create_item_with_empty_descr_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"descr": "", "price": "1.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422, "empty description should be rejected");
}

// ---------------------------------------------------------------------------
// 404 responses for non-existent resources
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn get_nonexistent_user_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    let fake_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "non-existent user should return 404");
}

#[actix_web::test]
#[ignore]
async fn get_nonexistent_team_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    let fake_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "non-existent team should return 404");
}

#[actix_web::test]
#[ignore]
async fn get_nonexistent_item_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    let fake_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/items/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "non-existent item should return 404");
}

#[actix_web::test]
#[ignore]
async fn get_nonexistent_role_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    let fake_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/roles/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "non-existent role should return 404");
}

// ---------------------------------------------------------------------------
// 409 conflict responses for duplicate creation
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_duplicate_item_returns_409() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Create an item
    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"descr": "zzz-duplicate-test-item", "price": "1.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // Try to create a second item with the same description → 409
    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"descr": "zzz-duplicate-test-item", "price": "2.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        409,
        "duplicate item description should return 409"
    );

    // Clean up: delete the item
    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    if let Some(item) = items
        .iter()
        .find(|i| i["descr"].as_str() == Some("zzz-duplicate-test-item"))
    {
        let item_id = item["item_id"].as_str().unwrap();
        let req = test::TestRequest::delete()
            .uri(&format!("/api/v1.0/items/{}", item_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        test::call_service(&app, req).await;
    }
}

#[actix_web::test]
#[ignore]
async fn create_duplicate_user_returns_409() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Try to create a user with the same email as the seed admin → 409
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "Duplicate",
            "lastname": "Admin",
            "email": "admin@admin.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 409, "duplicate email should return 409");
}

// ---------------------------------------------------------------------------
// Admin bypass: admin can update/delete other users
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn admin_can_update_other_user() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Create a temporary user to update
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "Temp",
            "lastname": "User",
            "email": "temp.admin.update@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap();

    // Admin updates the other user → should succeed
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "Updated",
            "lastname": "ByAdmin",
            "email": "temp.admin.update@test.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should be able to update another user"
    );
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["firstname"], "Updated");

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn admin_can_delete_other_user() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Create a temporary user to delete
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "Temp",
            "lastname": "Delete",
            "email": "temp.admin.delete@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap();

    // Admin deletes the other user → should succeed
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should be able to delete another user"
    );
}

// ---------------------------------------------------------------------------
// Items/Roles CUD: non-admin forbidden
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_create_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"descr": "forbidden item", "price": "1.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to create items"
    );
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_update_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let user_auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    // Get an existing item ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    let item_id = items[0]["item_id"].as_str().unwrap();

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
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_delete_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let user_auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    // Get an existing item ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    let item_id = items[0]["item_id"].as_str().unwrap();

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
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_create_role() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"title": "Forbidden Role"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to create roles"
    );
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_delete_role() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let user_auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    // Get an existing role ID (use "Guest" role which is safe to test against)
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let roles = paginated_items(test::read_body_json(resp).await);
    let guest_role = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Guest"))
        .unwrap();
    let role_id = guest_role["role_id"].as_str().unwrap();

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
}

// ---------------------------------------------------------------------------
// Team Admin vs Admin distinction
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_create_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is a Team Admin of "League of Cool Coders", not a global Admin
    let auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"tname": "Forbidden Team", "descr": "Should not be created"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin should not be able to create teams (requires global admin)"
    );
}

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_create_item() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is a Team Admin, not a global Admin
    let auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"descr": "forbidden item", "price": "1.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin should not be able to create items (requires global admin)"
    );
}

#[actix_web::test]
#[ignore]
async fn team_admin_can_manage_team_members() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is Team Admin of "League of Cool Coders"
    let auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get teams to find LoCC
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a temporary user to add as member
    let admin_auth: Auth = login_admin(&app).await;
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .set_json(json!({
            "firstname": "TempMember",
            "lastname": "Test",
            "email": "tempmember@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_user: Value = test::read_body_json(resp).await;
    let new_user_id = new_user["user_id"].as_str().unwrap().to_string();

    // Get the "Member" role ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
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

    // Team Admin adds the new user to the team → should succeed
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": member_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "team admin should be able to add members"
    );

    // Team Admin removes the member → should succeed
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_user_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "team admin should be able to remove members"
    );

    // Clean up: delete the temp user
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
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

    // admin@admin.com is Admin of "League of Cool Coders" but NOT a member of "Pixel Bakers"
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Get "Pixel Bakers" team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("Pixel Bakers"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _admin_user_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap();

    // Admin creates order on Pixel Bakers (not a member) → should succeed via bypass
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
}

#[actix_web::test]
#[ignore]
async fn admin_can_manage_any_team_members() {
    let state = test_state().await;
    let app = test_app!(state);

    // admin@admin.com is NOT a member of "Pixel Bakers"
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Get "Pixel Bakers" team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("Pixel Bakers"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a temp user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "TempPB",
            "lastname": "Test",
            "email": "temppb@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_user: Value = test::read_body_json(resp).await;
    let new_user_id = new_user["user_id"].as_str().unwrap().to_string();

    // Get "Member" role ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
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

    // Admin adds member to Pixel Bakers (not a member themselves) → should succeed
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": member_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "admin should add members to any team via bypass"
    );

    // Admin removes the member
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_user_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should remove members from any team via bypass"
    );

    // Clean up: delete the temp user
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// RBAC: create_user requires Admin or Team Admin
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_admin_non_team_admin_cannot_create_user() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is a regular Member (not Admin, not Team Admin)
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({
            "firstname": "Blocked",
            "lastname": "User",
            "email": "blocked@example.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "regular member should not be able to create users"
    );
}

#[actix_web::test]
#[ignore]
async fn team_admin_can_create_user() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is a Team Admin of "League of Cool Coders"
    let auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "Created",
            "lastname": "ByTeamAdmin",
            "email": "created.by.teamadmin@example.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "team admin should be able to create users"
    );
    let user: Value = test::read_body_json(resp).await;
    let new_user_id = user["user_id"].as_str().unwrap();

    // Clean up: admin deletes the created user
    let admin_auth: Auth = login_admin(&app).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "cleanup: admin should delete temp user");
}

// ---------------------------------------------------------------------------
// RBAC: Team Admin can update/delete users in their team, but not outside
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn team_admin_can_update_user_in_their_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is Team Admin of "League of Cool Coders"
    // U1_F is a Member of "League of Cool Coders"
    let ta_auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token = &ta_auth.access_token;

    // Find U1_F's user_id
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let u1 = users
        .iter()
        .find(|u| u["email"].as_str() == Some("U1_F.U1_L@LEGO.com"))
        .unwrap();
    let u1_id = u1["user_id"].as_str().unwrap();

    // Team Admin updates U1_F → should succeed (same team)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", u1_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "U1_F_Updated",
            "lastname": "U1_L",
            "email": "U1_F.U1_L@LEGO.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "team admin should be able to update a user in their team"
    );

    // Restore original name
    let admin_auth: Auth = login_admin(&app).await;
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", u1_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .set_json(json!({
            "firstname": "U1_F",
            "lastname": "U1_L",
            "email": "U1_F.U1_L@LEGO.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "cleanup: admin restores original name");
}

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_update_user_outside_their_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is Team Admin of "League of Cool Coders" and Member of "Pixel Bakers"
    let ta_auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token = &ta_auth.access_token;
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a temp user who will NOT be in any of U4_F's administered teams
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "Outside",
            "lastname": "User",
            "email": "outside.user@example.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_user: Value = test::read_body_json(resp).await;
    let new_user_id = new_user["user_id"].as_str().unwrap().to_string();

    // Find "Pixel Bakers" team ID (U4_F is only a Member here, not Team Admin)
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let pb_team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("Pixel Bakers"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get the "Member" role ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
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

    // Admin adds the temp user to "Pixel Bakers" (where U4_F is only a Member)
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", pb_team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": member_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "admin adds temp user to Pixel Bakers");

    // U4_F tries to update the temp user → should be 403
    // (temp user is only in Pixel Bakers where U4_F is Member, not Team Admin)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "Hacked",
            "lastname": "Name",
            "email": "outside.user@example.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin should not update a user outside their administered teams"
    );

    // Clean up: remove from Pixel Bakers, then delete user
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            pb_team_id, new_user_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "cleanup: admin deletes temp user");
}

#[actix_web::test]
#[ignore]
async fn team_admin_can_delete_user_in_their_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // U4_F is Team Admin of "League of Cool Coders"
    let ta_auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token = &ta_auth.access_token;

    // Create a temp user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "Deletable",
            "lastname": "ByTA",
            "email": "deletable.byta@example.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_user: Value = test::read_body_json(resp).await;
    let new_user_id = new_user["user_id"].as_str().unwrap().to_string();

    // Find "League of Cool Coders" team ID and "Member" role ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let locc_team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
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

    // Team Admin adds user to their team
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", locc_team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": member_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "team admin adds user to their team");

    // Team Admin deletes the user → should succeed (user is in their team)
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "team admin should be able to delete a user in their team"
    );
}

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_delete_user_outside_their_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // U4_F is Team Admin of "League of Cool Coders", Member of "Pixel Bakers"
    let ta_auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token = &ta_auth.access_token;

    // Create a temp user with no team membership at all
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "Orphan",
            "lastname": "User",
            "email": "orphan.user@example.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_user: Value = test::read_body_json(resp).await;
    let new_user_id = new_user["user_id"].as_str().unwrap().to_string();

    // U4_F tries to delete the orphan user → should be 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin should not delete a user not in any of their teams"
    );

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "cleanup: admin deletes orphan user");
}

#[actix_web::test]
#[ignore]
async fn user_can_still_update_self() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is a regular Member — should be able to update their own account
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;
    let token = &auth.access_token;

    // Find U1_F's user_id
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let u1 = users
        .iter()
        .find(|u| u["email"].as_str() == Some("U1_F.U1_L@LEGO.com"))
        .unwrap();
    let u1_id = u1["user_id"].as_str().unwrap();

    // Self-update → should succeed
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", u1_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "U1_F_Self",
            "lastname": "U1_L",
            "email": "U1_F.U1_L@LEGO.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "user should be able to update their own account"
    );

    // Restore original name
    let admin_auth: Auth = login_admin(&app).await;
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", u1_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .set_json(json!({
            "firstname": "U1_F",
            "lastname": "U1_L",
            "email": "U1_F.U1_L@LEGO.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "cleanup: admin restores original name");
}

// ---------------------------------------------------------------------------
// Order items CRUD (items within a team order)
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_get_update_delete_order_item() {
    let state = test_state().await;
    let app = test_app!(state);

    // Login as U4_F who is Team Admin of "League of Cool Coders"
    let auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get "League of Cool Coders" team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get an item ID from the catalog (seed data has items)
    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    assert!(!items.is_empty(), "seed data should have items");
    let item_id = items[0]["item_id"].as_str().unwrap().to_string();

    // Get user ID for the logged-in user
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _user_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("U4_F.U4_L@LEGO.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap();

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

    // Cleanup: delete the team order
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "cleanup: delete team order");
}

#[actix_web::test]
#[ignore]
async fn duplicate_order_item_returns_409() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get team and item IDs
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    let item_id = items[0]["item_id"].as_str().unwrap().to_string();

    // Get user ID for the logged-in user
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _user_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("U4_F.U4_L@LEGO.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap();

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
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn non_member_cannot_create_order_item() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is Team Admin of "League of Cool Coders" and Member of "Pixel Bakers"
    // Create an order on "League of Cool Coders" first
    let auth_u4: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token_u4 = &auth_u4.access_token;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token_u4)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token_u4)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    let item_id = items[0]["item_id"].as_str().unwrap().to_string();

    // Get user ID for U4_F
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token_u4)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _u4_user_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("U4_F.U4_L@LEGO.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap();

    // Create order as U4_F (team admin)
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token_u4)))
        .set_json(json!({"duedate": "2026-09-01"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Now create a user who is NOT a member of this team
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "Outsider",
            "lastname": "User",
            "email": "outsider@test.com",
            "password": "Very Secret"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let outsider: Value = test::read_body_json(resp).await;
    let outsider_id = outsider["user_id"].as_str().unwrap().to_string();

    // Login as the outsider
    let outsider_auth: Auth = login_user(&app, "outsider@test.com").await;
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

    // Cleanup: delete order (cascades order items) and outsider user
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token_u4)))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", outsider_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn admin_can_manage_order_items_on_any_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // Admin is member of "League of Cool Coders" but NOT "Pixel Bakers"
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Get "Pixel Bakers" team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("Pixel Bakers"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get an item
    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    let item_id = items[0]["item_id"].as_str().unwrap().to_string();

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _admin_user_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap();

    // Admin creates order on Pixel Bakers (not a member) → bypass
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

    // Cleanup: delete the team order
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
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
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Find the "League of Cool Coders" team
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _admin_user_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap();

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

    // Get an item ID from the catalog
    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    let item_id = items[0]["item_id"].as_str().unwrap().to_string();

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

    // Cleanup: delete the order
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn closed_order_rejects_update_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Find the "League of Cool Coders" team
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _admin_user_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap();

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

    // Get an item ID from the catalog
    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    let item_id = items[0]["item_id"].as_str().unwrap().to_string();

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

    // Cleanup: reopen and delete
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"closed": false}))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn closed_order_rejects_delete_item() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Find the "League of Cool Coders" team
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _admin_user_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap();

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

    // Get an item ID from the catalog
    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    let item_id = items[0]["item_id"].as_str().unwrap().to_string();

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

    // Cleanup: reopen and delete
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"closed": false}))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn reopened_order_allows_item_mutations() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Find the "League of Cool Coders" team
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _admin_user_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap();

    // Create a team order
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"duedate": "2026-12-28"}))
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
    assert_eq!(resp.status(), 200);

    // Reopen the order
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"closed": false}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Get an item ID from the catalog
    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    let item_id = items[0]["item_id"].as_str().unwrap().to_string();

    // Adding items to the reopened order should succeed
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1.0/teams/{}/orders/{}/items",
            team_id, order_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"orders_item_id": item_id, "amt": 5}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "should add items to a reopened order");

    // Cleanup: delete the order (cascades to order items)
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Team Admin user scoping: can only modify users in shared teams
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn team_admin_can_update_user_in_shared_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is Team Admin of "League of Cool Coders"
    // U1_F is Member of "League of Cool Coders" — shared team
    let ta_auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let ta_token = &ta_auth.access_token;

    // Get U1_F's user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let u1 = users
        .iter()
        .find(|u| u["email"].as_str() == Some("U1_F.U1_L@LEGO.com"))
        .expect("seed user U1_F should exist");
    let u1_id = u1["user_id"].as_str().unwrap();

    // Team Admin updates U1_F → should succeed (shared team)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", u1_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "firstname": "U1_F",
            "lastname": "U1_L",
            "email": "U1_F.U1_L@LEGO.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "team admin should update users in their team"
    );
}

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_update_user_outside_shared_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is Team Admin of "League of Cool Coders" and Member of "Pixel Bakers"
    // Create a user that is NOT in any of U4_F's teams
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "Isolated",
            "lastname": "User",
            "email": "isolated.user@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let isolated_user: Value = test::read_body_json(resp).await;
    let isolated_id = isolated_user["user_id"].as_str().unwrap().to_string();

    // U4_F tries to update isolated user → should be 403
    let ta_auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let ta_token = &ta_auth.access_token;

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", isolated_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "firstname": "Hacked",
            "lastname": "User",
            "email": "isolated.user@test.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin should NOT update users outside their teams"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", isolated_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Member cannot create users (requires admin or team admin)
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn member_cannot_create_user() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is a regular Member
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({
            "firstname": "Forbidden",
            "lastname": "User",
            "email": "forbidden.create@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "regular member should not be able to create users"
    );
}

// ---------------------------------------------------------------------------
// delete_user_by_email RBAC fallback — non-admin cannot discover whether an
// email exists; admin gets a proper 404 for a nonexistent email.
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_admin_delete_by_email_nonexistent_returns_403() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is a regular Member, not an Admin
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    let req = test::TestRequest::delete()
        .uri("/api/v1.0/users/email/nonexistent@example.com")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should get 403 even when email does not exist (prevents info leakage)"
    );
}

#[actix_web::test]
#[ignore]
async fn admin_delete_by_email_nonexistent_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    let req = test::TestRequest::delete()
        .uri("/api/v1.0/users/email/nonexistent@example.com")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "admin should get 404 for a nonexistent email"
    );
}

// ---------------------------------------------------------------------------
// Create-user → authenticate round-trip (validates Argon2 hashing in create)
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_user_then_authenticate_round_trip() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = "roundtrip.test@example.com";
    let test_password = "RoundTrip!Pass123";

    // 1. Create a new user via the API
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "RoundTrip",
            "lastname": "Test",
            "email": test_email,
            "password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "admin should be able to create a new user"
    );
    let user: Value = test::read_body_json(resp).await;
    let new_user_id = user["user_id"].as_str().unwrap();

    // 2. Authenticate the newly created user via Basic Auth
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, test_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "newly created user should authenticate successfully (password must be Argon2 hashed)"
    );
    let new_user_auth: Auth = test::read_body_json(resp).await;
    assert!(
        !new_user_auth.access_token.is_empty(),
        "should receive a non-empty access token"
    );
    assert!(
        !new_user_auth.refresh_token.is_empty(),
        "should receive a non-empty refresh token"
    );

    // 3. Use the new user's token to access a protected endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", new_user_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "new user should access their own profile with the issued token"
    );
    let fetched_user: Value = test::read_body_json(resp).await;
    assert_eq!(fetched_user["email"].as_str().unwrap(), test_email);

    // Clean up: admin deletes the created user
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "cleanup: admin should delete temp user");
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_update_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is a Member, not an Admin
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get teams
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams[0]["team_id"].as_str().unwrap();

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "Forbidden Update", "descr": "Should not work"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to update teams"
    );
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_update_role() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let user_auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    // Get an existing role ID (use "Guest" role which is safe to test against)
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let roles = paginated_items(test::read_body_json(resp).await);
    let guest_role = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Guest"))
        .unwrap();
    let role_id = guest_role["role_id"].as_str().unwrap();

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
}

// ---------------------------------------------------------------------------
// Escalation guard: Team Admin cannot assign global Admin role
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_assign_admin_role_via_add_member() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is Team Admin of "League of Cool Coders"
    let ta_auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let ta_token = &ta_auth.access_token;

    // Get the LoCC team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get the "Admin" role ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let roles = paginated_items(test::read_body_json(resp).await);
    let admin_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Admin"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a temp user to use as the target
    let admin_auth: Auth = login_admin(&app).await;
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .set_json(json!({
            "firstname": "EscGuard",
            "lastname": "Test",
            "email": "escguard.add@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_user: Value = test::read_body_json(resp).await;
    let new_user_id = new_user["user_id"].as_str().unwrap().to_string();

    // Team Admin tries to add user with Admin role → should be 403
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": admin_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin must not assign global Admin role via add_team_member"
    );

    // Clean up: delete the temp user
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_assign_admin_role_via_update_role() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is Team Admin of "League of Cool Coders"
    let ta_auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let ta_token = &ta_auth.access_token;

    // Get the LoCC team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get role IDs for "Member" and "Admin"
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
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
    let admin_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Admin"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a temp user and add as Member first
    let admin_auth: Auth = login_admin(&app).await;
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .set_json(json!({
            "firstname": "EscGuard",
            "lastname": "Update",
            "email": "escguard.update@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_user: Value = test::read_body_json(resp).await;
    let new_user_id = new_user["user_id"].as_str().unwrap().to_string();

    // Add user as Member (Team Admin can do this)
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": member_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "team admin should add user as Member");

    // Team Admin tries to update user's role to Admin → should be 403
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_user_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "role_id": admin_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin must not escalate a member to global Admin via update_member_role"
    );

    // Clean up: remove member then delete user
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_user_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Password update → re-login round-trip
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn update_user_password_then_reauthenticate() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = "pwchange.test@example.com";
    let original_password = "OriginalPass!123";
    let new_password = "ChangedPass!456";

    // 1. Create a temp user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "PwChange",
            "lastname": "Test",
            "email": test_email,
            "password": original_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // 2. Authenticate with the original password → should succeed
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, original_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "original password should work before change"
    );

    // 3. Update password via PUT
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "PwChange",
            "lastname": "Test",
            "email": test_email,
            "password": new_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "password update should succeed");

    // 4. Authenticate with the NEW password → should succeed
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, new_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "new password should work after change");

    // 5. Authenticate with the OLD password → should fail
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, original_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        401,
        "old password must not work after change"
    );

    // Clean up: admin deletes the temp user
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "cleanup: admin should delete temp user");
}

// ===========================================================================
// user_teams endpoint (#173)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn user_teams_returns_teams_for_seed_admin() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Get admin user_id
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let admin = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .expect("admin user should exist");
    let admin_id = admin["user_id"].as_str().unwrap();

    // GET /api/v1.0/users/{admin_id}/teams
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}/teams", admin_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "user_teams should return 200");
    let teams = paginated_items(test::read_body_json(resp).await);
    assert!(
        teams.iter().any(|t| t["tname"] == "League of Cool Coders"),
        "admin should be member of League of Cool Coders"
    );
    // Verify membership timestamps are present (#115)
    let team = &teams[0];
    assert!(
        team["joined"].is_string(),
        "joined timestamp should be present"
    );
    assert!(
        team["role_changed"].is_string(),
        "role_changed timestamp should be present"
    );
}

#[actix_web::test]
#[ignore]
async fn user_teams_returns_empty_for_user_with_no_teams() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let token = &admin_auth.access_token;

    // Create a temp user (not added to any team)
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "NoTeam",
            "lastname": "User",
            "email": "noteam@test.local",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap();

    // GET user_teams → should be empty
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}/teams", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let teams = paginated_items(test::read_body_json(resp).await);
    assert!(teams.is_empty(), "new user should have no teams");

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// Malformed path parameter (#175)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn malformed_uuid_path_returns_400() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/users/not-a-valid-uuid")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        400,
        "malformed UUID should return 400 Bad Request"
    );
}

// ===========================================================================
// JSON error handler (#176)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn wrong_content_type_returns_415() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    // POST with text/plain instead of application/json
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .insert_header(("Content-Type", "text/plain"))
        .set_payload("this is not json")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        415,
        "wrong content type should return 415 Unsupported Media Type"
    );
}

#[actix_web::test]
#[ignore]
async fn invalid_json_body_returns_error() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    // POST with Content-Type: application/json but invalid JSON
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .insert_header(("Content-Type", "application/json"))
        .set_payload("{invalid json}")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 422,
        "invalid JSON should return 400 or 422, got {}",
        status
    );
}

// ===========================================================================
// Update team / update role success paths (#177)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn admin_can_update_team() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Create a temp team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "UpdateMe Team", "descr": "Original"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap();

    // Update the team
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "Updated Team", "descr": "Changed"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should be able to update teams");
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["tname"], "Updated Team");
    assert_eq!(updated["descr"], "Changed");

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn admin_can_update_role() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
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

// ===========================================================================
// Location header in create responses (#178)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_item_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"descr": "location-test-item", "price": "1.50"}))
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
// Rate limiting (#179)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn auth_endpoint_rate_limits_after_burst() {
    let state = test_state().await;
    let app = test_app!(state);

    let creds = format!("Basic {}", STANDARD.encode("admin@admin.com:Very Secret"));

    // Send requests up to burst size (10)
    for i in 0..10 {
        let req = test::TestRequest::post()
            .uri("/auth")
            .peer_addr(PEER)
            .insert_header(("Authorization", creds.clone()))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_ne!(
            resp.status().as_u16(),
            429,
            "request {} should not be rate limited within burst",
            i + 1
        );
    }

    // 11th request should be rate limited
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header(("Authorization", creds.clone()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status().as_u16(),
        429,
        "request after burst should be rate limited"
    );
}

// ===========================================================================
// Bulk delete team orders (#204)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn admin_can_bulk_delete_team_orders() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Get admin user_id
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let _admin_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a temp team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "BulkDeleteOrders Team", "descr": "temp"}))
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

// ===========================================================================
// Update member role (#205)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn admin_can_update_member_role() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Get seed IDs
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
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
    let guest_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Guest"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a temp user and add them to the team as Member
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "RoleChange",
            "lastname": "Test",
            "email": "rolechange@test.local",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"user_id": user_id, "role_id": member_role_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // Update their role to Guest
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"role_id": guest_role_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should update member role");
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["title"], "Guest", "role should be updated to Guest");

    // Clean up: remove member, delete user
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// Delete user by email success (#206)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn admin_can_delete_user_by_email() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Create a temp user
    let email = "deleteme.byemail@test.local";
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "DeleteByEmail",
            "lastname": "Test",
            "email": email,
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // Delete by email
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/email/{}", email))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should delete user by email");
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["deleted"], true);
}

// ===========================================================================
// Token revocation ownership check (#207)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_revoke_another_users_token() {
    let state = test_state().await;
    let app = test_app!(state);

    // Login as two different users
    let admin_auth: Auth = login_admin(&app).await;
    let user_auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    // U1_F tries to revoke admin's access token → should fail with 403
    let req = test::TestRequest::post()
        .uri("/auth/revoke")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .set_json(json!({"token": admin_auth.access_token}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not revoke another user's token"
    );
}

#[actix_web::test]
#[ignore]
async fn admin_can_revoke_another_users_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = login_admin(&app).await;
    let user_auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;

    // Admin revokes U1_F's token → should succeed
    let req = test::TestRequest::post()
        .uri("/auth/revoke")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .set_json(json!({"token": user_auth.access_token}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should be able to revoke another user's token"
    );

    // Verify the revoked token is now invalid
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "revoked token should be rejected");
}

// ===========================================================================
// Team users endpoint (#208)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn team_users_returns_members_of_seed_team() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Find "League of Cool Coders" team_id
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap();

    // GET /api/v1.0/teams/{team_id}/users
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let users = paginated_items(test::read_body_json(resp).await);

    // Seed data has 5 members in LoCC: admin, U1_F, U2_F, U3_F, U4_F
    assert_eq!(users.len(), 5, "LoCC should have 5 seed members");

    // Check that membership timestamps are present (#115)
    let first = &users[0];
    assert!(
        first["joined"].is_string(),
        "joined timestamp should be present"
    );
    assert!(
        first["role_changed"].is_string(),
        "role_changed timestamp should be present"
    );
}

#[actix_web::test]
#[ignore]
async fn team_users_returns_empty_for_team_with_no_members() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Create a fresh team with no members
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "EmptyTeamUsersTest", "descr": "no members"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let users = paginated_items(test::read_body_json(resp).await);
    assert!(users.is_empty(), "new team should have no members");

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// #263 — Revoked refresh token is rejected by /auth/refresh
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn revoked_refresh_token_is_rejected_by_refresh_endpoint() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_admin(&app).await;

    // Explicitly revoke the refresh token
    let req = test::TestRequest::post()
        .uri("/auth/revoke")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"token": auth.refresh_token}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "revoke should succeed");

    // Try to use the revoked refresh token — should fail
    let req = test::TestRequest::post()
        .uri("/auth/refresh")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.refresh_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        401,
        "revoked refresh token should be rejected"
    );
}

// ---------------------------------------------------------------------------
// #264 — Empty order items list returns 200 with empty items array
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn empty_order_items_returns_200_with_empty_list() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get "League of Cool Coders" team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a fresh order (no items)
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
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
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let items = paginated_items(test::read_body_json(resp).await);
    assert!(items.is_empty(), "new order should have zero items");

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// #265 — add_team_member with non-existent role_id returns 404
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn add_team_member_with_nonexistent_role_id_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is Team Admin of "League of Cool Coders"
    let ta_auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let ta_token = &ta_auth.access_token;

    // Get the team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a temp user
    let admin_auth: Auth = login_admin(&app).await;
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .set_json(json!({
            "firstname": "RoleTest",
            "lastname": "User",
            "email": "roletest.nonexistent@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_user: Value = test::read_body_json(resp).await;
    let new_user_id = new_user["user_id"].as_str().unwrap().to_string();

    // Try to add user with a non-existent role_id
    let fake_role_id = Uuid::now_v7().to_string();
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": fake_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "non-existent role_id should return 404 from guard_admin_role_assignment"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// #289 — Member cannot manage team members
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn member_cannot_add_team_member() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is "Member" in "League of Cool Coders"
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get team ID for "League of Cool Coders"
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get a role ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
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

    // Create a temp user as admin
    let admin_auth: Auth = login_admin(&app).await;
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .set_json(json!({
            "firstname": "TempMember",
            "lastname": "Test",
            "email": "tempmember.test289@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_user: Value = test::read_body_json(resp).await;
    let new_user_id = new_user["user_id"].as_str().unwrap().to_string();

    // Member tries to add a team member → 403
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": member_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "Member should not be able to add team members"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn member_cannot_remove_team_member() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is "Member" in "League of Cool Coders"
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get team and users
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get U2_F's user_id (another member)
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let u2_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("U2_F.U2_L@LEGO.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Member tries to remove another member → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, u2_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "Member should not be able to remove team members"
    );
}

#[actix_web::test]
#[ignore]
async fn member_cannot_update_member_role() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is "Member" in "League of Cool Coders"
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get team
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get U2_F and Member role ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let u2_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("U2_F.U2_L@LEGO.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
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

    // Member tries to update another member's role → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, u2_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "role_id": member_role_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "Member should not be able to update member roles"
    );
}

// ---------------------------------------------------------------------------
// #290 — Member cannot bulk-delete team orders
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn member_cannot_bulk_delete_team_orders() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is "Member" in "League of Cool Coders"
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;
    let token = &auth.access_token;

    // Get team
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Member tries to bulk-delete all team orders → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "Member should not be able to bulk-delete team orders"
    );
}

// ---------------------------------------------------------------------------
// #291 — Non-member cannot PUT/DELETE single team order
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_member_cannot_update_team_order() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is NOT a member of "Pixel Bakers"
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;
    let token = &auth.access_token;
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Get team ID for "Pixel Bakers"
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("Pixel Bakers"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a real order via admin so the lookup succeeds
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
        .insert_header(("Authorization", format!("Bearer {}", token)))
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
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn non_member_cannot_delete_team_order() {
    let state = test_state().await;
    let app = test_app!(state);

    // U1_F is NOT a member of "Pixel Bakers"
    let auth: Auth = login_user(&app, "U1_F.U1_L@LEGO.com").await;
    let token = &auth.access_token;
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Get team ID for "Pixel Bakers"
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("Pixel Bakers"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a real order via admin
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
        .insert_header(("Authorization", format!("Bearer {}", token)))
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
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// #294 — Location header on remaining create endpoints
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_user_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "LocHdr",
            "lastname": "Test",
            "email": "lochdr.test294@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let location = resp
        .headers()
        .get("Location")
        .expect("201 should include Location header");
    assert!(
        location.to_str().unwrap().contains("/api/v1.0/users/"),
        "Location should contain user path"
    );
    let body: Value = test::read_body_json(resp).await;
    let user_id = body["user_id"].as_str().unwrap();

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn create_team_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "LocHdr Team 294", "descr": "temp"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let location = resp
        .headers()
        .get("Location")
        .expect("201 should include Location header");
    assert!(
        location.to_str().unwrap().contains("/api/v1.0/teams/"),
        "Location should contain team path"
    );
    let body: Value = test::read_body_json(resp).await;
    let team_id = body["team_id"].as_str().unwrap();

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn create_role_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"title": "LocHdrRole294", "descr": "temp"}))
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
async fn create_team_order_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Create temp team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "LocHdr Order Team 294", "descr": "temp"}))
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
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Create temp team + order
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "LocHdr OItem Team 294", "descr": "temp"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/orders", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let order: Value = test::read_body_json(resp).await;
    let order_id = order["teamorders_id"].as_str().unwrap().to_string();

    // Get a seed item
    let req = test::TestRequest::get()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items = paginated_items(test::read_body_json(resp).await);
    let item_id = items[0]["item_id"].as_str().unwrap().to_string();

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
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn add_team_member_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Create temp team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "LocHdr Member Team 294", "descr": "temp"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap().to_string();

    // Create temp user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "LocMember",
            "lastname": "Test",
            "email": "locmember.test294@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_user: Value = test::read_body_json(resp).await;
    let new_user_id = new_user["user_id"].as_str().unwrap().to_string();

    // Get Member role_id
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
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

    // Add member
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": member_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let location = resp
        .headers()
        .get("Location")
        .expect("201 should include Location header");
    let loc_str = location.to_str().unwrap();
    assert!(
        loc_str.contains(&format!("/api/v1.0/teams/{}/users/", team_id)),
        "Location should contain team member path, got: {}",
        loc_str
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
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
    let auth: Auth = login_admin(&app).await;
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
// #397 — Self-password-change verification: missing, wrong, correct
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn self_password_change_without_current_password_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = "selfpw-no-current@example.com";
    let test_password = "OriginalPass!123";

    // Create user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "NoCurrent",
            "email": test_email,
            "password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // Login as the user to get their own token
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, test_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let user_auth: Auth = test::read_body_json(resp).await;
    let user_token = &user_auth.access_token;

    // Self-update password WITHOUT current_password → 422
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "NoCurrent",
            "email": test_email,
            "password": "NewPassword!456"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        422,
        "self-password-change without current_password should be 422"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn self_password_change_with_wrong_current_password_returns_403() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = "selfpw-wrong@example.com";
    let test_password = "OriginalPass!123";

    // Create user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "WrongCurrent",
            "email": test_email,
            "password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // Login as the user
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, test_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let user_auth: Auth = test::read_body_json(resp).await;
    let user_token = &user_auth.access_token;

    // Self-update password with WRONG current_password → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "WrongCurrent",
            "email": test_email,
            "password": "NewPassword!456",
            "current_password": "TotallyWrongPassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "self-password-change with wrong current_password should be 403"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn self_password_change_with_correct_current_password_succeeds() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = "selfpw-correct@example.com";
    let test_password = "OriginalPass!123";
    let new_password = "ChangedPass!456";

    // Create user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "Correct",
            "email": test_email,
            "password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // Login as the user
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, test_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let user_auth: Auth = test::read_body_json(resp).await;
    let user_token = &user_auth.access_token;

    // Self-update password with CORRECT current_password → 200
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "Correct",
            "email": test_email,
            "password": new_password,
            "current_password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "self-password-change with correct current_password should succeed"
    );

    // Verify new password works
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, new_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "new password should work after change");

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// #400 — Account lockout full lifecycle
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn lockout_lifecycle_5_failures_then_429_then_success_clears() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = "lockout-lifecycle@example.com";
    let test_password = "LockoutTest!123";

    // Create a test user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "Lockout",
            "lastname": "Test",
            "email": test_email,
            "password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // 1. Send 5 wrong password attempts
    for i in 1..=5 {
        let req = test::TestRequest::post()
            .uri("/auth")
            .peer_addr(PEER)
            .insert_header((
                "Authorization",
                format!(
                    "Basic {}",
                    STANDARD.encode(format!("{}:wrong-password-{}", test_email, i))
                ),
            ))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            401,
            "attempt {} should be rejected with 401",
            i
        );
    }

    // 2. Next attempt should be locked out → 429
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:any-password", test_email))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        429,
        "after 5 failures, account should be locked (429)"
    );

    // 3. Clear lockout by directly manipulating state (simulates window expiry)
    state.login_attempts.remove(test_email);

    // 4. Correct password should now succeed
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, test_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "correct password should work after lockout cleared"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// #401 — Self-delete user at API level
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn non_admin_user_can_delete_own_account() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = "self-delete@example.com";
    let test_password = "SelfDelete!123";

    // Create a test user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SelfDel",
            "lastname": "Test",
            "email": test_email,
            "password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // Login as the user
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, test_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let user_auth: Auth = test::read_body_json(resp).await;
    let user_token = &user_auth.access_token;

    // Delete own account
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "non-admin user should be able to delete their own account"
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["deleted"], true);

    // Verify user no longer exists
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "deleted user should not be found");
}

// ===========================================================================
// #399 — Last admin cannot delete themselves
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn last_admin_cannot_delete_own_account() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // The seed admin is the only admin. Attempting self-delete should fail.
    // First, find the admin's user_id from the token.
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let users = paginated_items(body);
    let admin_user = users
        .iter()
        .find(|u| u["email"] == "admin@admin.com")
        .expect("seed admin should exist");
    let admin_id = admin_user["user_id"].as_str().unwrap();

    // Attempt to delete self
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "last admin should not be able to delete their own account"
    );
}

// ===========================================================================
// #351 — refresh token rejected by JWT-protected API endpoint
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn jwt_protected_endpoint_rejects_refresh_token() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;

    // Use the refresh token (token_type = Refresh) against a JWT-gated endpoint.
    // jwt_validator checks token_type == Access and rejects anything else.
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.refresh_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        401,
        "refresh token should be rejected by JWT-protected endpoints"
    );
}

// ===========================================================================
// #429 — Team Admin can bulk-delete their own team's orders
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn team_admin_can_bulk_delete_own_team_orders() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
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
    let admin_auth: Auth = login_admin(&app).await;
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
    let admin_auth: Auth = login_admin(&app).await;
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

// ===========================================================================
// #432 — Creating a team with a duplicate name returns 409
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_team_with_duplicate_name_returns_409() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // Create first team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "DuplicateTeamName432", "descr": "first"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "first team should be created");
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap().to_string();

    // Attempt to create second team with the same name → 409
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "DuplicateTeamName432", "descr": "duplicate"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        409,
        "duplicate team name should return 409 Conflict"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
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
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "iname": "negative price item",
            "descr": "should fail",
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
// #434 — Pagination: limit is clamped to max 100, negative offset treated as 0
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn pagination_clamps_limit_and_treats_negative_offset_as_zero() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = login_admin(&app).await;
    let token = &auth.access_token;

    // limit=200 should be clamped to 100
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users?limit=200")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["limit"].as_i64(),
        Some(100),
        "limit should be clamped to 100"
    );
    assert!(
        body["items"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(usize::MAX)
            <= 100,
        "items array should not exceed 100 entries"
    );

    // offset=-5 should be treated as 0
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users?offset=-5")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["offset"].as_i64(),
        Some(0),
        "negative offset should be treated as 0"
    );
}

// ===========================================================================
// #435 — Non-admin user can delete their own account by email
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn user_can_delete_own_account_by_email() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a regular (non-admin) user
    let email = format!("selfdelete435-{}@test.local", Uuid::now_v7());
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SelfDelete",
            "lastname": "ByEmail",
            "email": email,
            "password": "selfdeletepass435"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "admin should create the user");
    let created: Value = test::read_body_json(resp).await;
    let user_id = created["user_id"].as_str().unwrap().to_string();

    // User logs in
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:selfdeletepass435", email))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "user should be able to login");
    let user_auth: Auth = test::read_body_json(resp).await;
    let user_token = &user_auth.access_token;

    // User deletes own account by email
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/email/{}", email))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "user should be able to delete own account by email"
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["deleted"], true);

    // Verify the user no longer exists
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "deleted user should no longer be found");
}

// ---------------------------------------------------------------------------
// guard_admin_demotion — protect global Admins from Team Admin actions
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_demote_global_admin() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is Team Admin of "League of Cool Coders"
    let ta_auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let ta_token = &ta_auth.access_token;

    // Get admin user ID (global Admin in LoCC)
    let admin_auth: Auth = login_admin(&app).await;
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let admin_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get team ID and Member role ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
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

    // Team Admin tries to demote global Admin to Member → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, admin_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({ "role_id": member_role_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin must not demote a global Admin"
    );
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["error"].as_str().unwrap().contains("global Admin"),
        "error message should mention global Admin: {:?}",
        body
    );

    // Ensure admin still has Admin role (unchanged)
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", admin_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let members = paginated_items(test::read_body_json(resp).await);
    let admin_member = members
        .iter()
        .find(|m| m["email"].as_str() == Some("admin@admin.com"))
        .expect("admin should still be in team");
    assert_eq!(
        admin_member["title"].as_str().unwrap(),
        "Admin",
        "admin's role should be unchanged"
    );
}

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_remove_global_admin_from_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // U4_F is Team Admin of "League of Cool Coders"
    let ta_auth: Auth = login_user(&app, "U4_F.U4_L@LEGO.com").await;
    let ta_token = &ta_auth.access_token;

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let admin_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Team Admin tries to remove global Admin from team → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, admin_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin must not remove a global Admin from a team"
    );
}

#[actix_web::test]
#[ignore]
async fn global_admin_can_demote_another_global_admin() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Get team and role IDs
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let roles = paginated_items(test::read_body_json(resp).await);
    let admin_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Admin"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();
    let member_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Member"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a second admin: create user, add as Admin in the team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SecondAdmin",
            "lastname": "Test",
            "email": "second.admin.demotion@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_admin: Value = test::read_body_json(resp).await;
    let new_admin_id = new_admin["user_id"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "user_id": new_admin_id,
            "role_id": admin_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "admin should add second admin");

    // First admin demotes second admin to Member → should succeed
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_admin_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({ "role_id": member_role_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "global admin should be able to demote another global admin"
    );

    // Clean up: remove from team and delete user
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_admin_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// guard_last_admin_membership — prevent zero-admin state
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn last_admin_cannot_demote_self() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let admin_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get team ID and Member role ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

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

    // Admin tries to demote self to Member → 403 (last admin)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({ "role_id": member_role_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "last admin must not be able to demote themselves"
    );
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("last global Admin"),
        "error should mention last admin: {:?}",
        body
    );
}

#[actix_web::test]
#[ignore]
async fn last_admin_cannot_remove_self_from_admin_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Get admin user ID and team ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let admin_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some("admin@admin.com"))
        .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Admin tries to remove self from team → 403 (last admin)
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "last admin must not be able to remove themselves from their admin team"
    );
}

#[actix_web::test]
#[ignore]
async fn demoting_admin_allowed_when_another_admin_exists() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = login_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Get team and role IDs
    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let teams = paginated_items(test::read_body_json(resp).await);
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("League of Cool Coders"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let roles = paginated_items(test::read_body_json(resp).await);
    let admin_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Admin"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();
    let member_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Member"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a second admin
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SecondAdmin",
            "lastname": "LastGuard",
            "email": "second.admin.lastguard@test.com",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_admin: Value = test::read_body_json(resp).await;
    let new_admin_id = new_admin["user_id"].as_str().unwrap().to_string();

    // Add second user as Admin in the team
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "user_id": new_admin_id,
            "role_id": admin_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // Now demoting the second admin to Member should succeed (first admin still exists)
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_admin_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({ "role_id": member_role_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "demoting an admin should succeed when another admin exists"
    );

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_admin_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}
