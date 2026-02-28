//! Integration tests for the breakfast API.
//!
//! These tests require a running PostgreSQL instance seeded with `database.sql`.
//! The easiest way to run them:
//!   make test-integration
//!
//! Or manually:
//!   docker compose up -d postgres && docker compose run --rm postgres-setup
//!   cargo test --test api_tests -- --ignored

use actix_web::{test, web::Data, App};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use breakfast::{models::*, routes::routes};
use serde_json::{json, Value};
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
        secret: "Very Secret".to_string(),
        jwtsecret: "Very Secret".to_string(),
        s3_key_id: String::new(),
        s3_key_secret: String::new(),
        cache: flurry::HashMap::new(),
        token_blacklist: flurry::HashMap::new(),
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
    assert!(body.as_array().unwrap().len() >= 5, "seed data has 5 users");
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
    assert!(body.as_array().unwrap().len() >= 2, "seed data has 2 teams");
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
    assert!(body.as_array().unwrap().len() >= 3, "seed data has 3 roles");
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
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"token": "not.a.real.token"}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 500, "invalid token should be rejected");
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
    assert!(body.as_array().unwrap().len() >= 4, "seed data has 4 items");
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
    let users: Vec<Value> = test::read_body_json(resp).await;

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
    let users: Vec<Value> = test::read_body_json(resp).await;

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
    let teams: Vec<Value> = test::read_body_json(resp).await;
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
    let orders: Vec<Value> = test::read_body_json(resp).await;
    assert!(orders
        .iter()
        .any(|o| o["teamorders_id"].as_str() == Some(&order_id)));

    // Delete the order
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
    let teams: Vec<Value> = test::read_body_json(resp).await;
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("Pixel Bakers"))
        .unwrap()["team_id"]
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
    let teams: Vec<Value> = test::read_body_json(resp).await;
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
        .set_json(json!({"descr": "duplicate-test-item", "price": "1.00"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // Try to create a second item with the same description → 409
    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"descr": "duplicate-test-item", "price": "2.00"}))
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
    let items: Vec<Value> = test::read_body_json(resp).await;
    if let Some(item) = items
        .iter()
        .find(|i| i["descr"].as_str() == Some("duplicate-test-item"))
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
    assert_eq!(
        resp.status(),
        409,
        "duplicate email should return 409"
    );
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
        .insert_header(("Authorization", format!("Bearer {}", admin_auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items: Vec<Value> = test::read_body_json(resp).await;
    let item_id = items[0]["item_id"].as_str().unwrap();

    // Non-admin tries to update → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", user_auth.access_token)))
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
        .insert_header(("Authorization", format!("Bearer {}", admin_auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let items: Vec<Value> = test::read_body_json(resp).await;
    let item_id = items[0]["item_id"].as_str().unwrap();

    // Non-admin tries to delete → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/items/{}", item_id))
        .insert_header(("Authorization", format!("Bearer {}", user_auth.access_token)))
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
        .insert_header(("Authorization", format!("Bearer {}", admin_auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let roles: Vec<Value> = test::read_body_json(resp).await;
    let guest_role = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Guest"))
        .unwrap();
    let role_id = guest_role["role_id"].as_str().unwrap();

    // Non-admin tries to delete → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/roles/{}", role_id))
        .insert_header(("Authorization", format!("Bearer {}", user_auth.access_token)))
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
    let teams: Vec<Value> = test::read_body_json(resp).await;
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
        .insert_header(("Authorization", format!("Bearer {}", admin_auth.access_token)))
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
    let roles: Vec<Value> = test::read_body_json(resp).await;
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
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, new_user_id))
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
        .insert_header(("Authorization", format!("Bearer {}", admin_auth.access_token)))
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
    let teams: Vec<Value> = test::read_body_json(resp).await;
    let team_id = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some("Pixel Bakers"))
        .unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

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
    let teams: Vec<Value> = test::read_body_json(resp).await;
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
    let roles: Vec<Value> = test::read_body_json(resp).await;
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
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, new_user_id))
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