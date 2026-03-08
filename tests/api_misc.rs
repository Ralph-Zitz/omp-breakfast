//! Health endpoint, validation, 404/409 responses, pagination, and misc edge-case tests.

mod common;

use actix_web::test;
use actix_web::web::Data;
use common::*;
use serde_json::{Value, json};

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

#[actix_web::test]
#[ignore]
async fn health_returns_503_when_db_unreachable() {
    // Create a state with a pool pointing to an unreachable port
    let mut pg_cfg = deadpool_postgres::Config::new();
    pg_cfg.user = Some("actix".to_string());
    pg_cfg.password = Some("actix".to_string());
    pg_cfg.dbname = Some("actix".to_string());
    pg_cfg.host = Some("127.0.0.1".to_string());
    pg_cfg.port = Some(1); // unreachable port
    // Short connect timeout to avoid slow test
    pg_cfg.connect_timeout = Some(std::time::Duration::from_millis(200));
    let pool = pg_cfg
        .create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("pool creation should succeed");
    let state = Data::new(State {
        pool,
        jwtsecret: secrecy::SecretString::from("test".to_string()),
        cache: dashmap::DashMap::new(),
        token_blacklist: dashmap::DashMap::new(),
        login_attempts: dashmap::DashMap::new(),
        avatar_cache: dashmap::DashMap::new(),
    });
    let app = test_app!(state);

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 503);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["up"], json!(false));
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
    let auth: Auth = register_admin(&app).await;

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

// ===========================================================================
// Malformed path parameter (#175)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn malformed_uuid_path_returns_400() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

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
    let auth: Auth = register_admin(&app).await;

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
    let auth: Auth = register_admin(&app).await;

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
// #434 — Pagination: limit is clamped to max 100, negative offset treated as 0
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn pagination_clamps_limit_and_treats_negative_offset_as_zero() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
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
