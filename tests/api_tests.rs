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
        .insert_header(("Authorization", format!("Bearer {}", auth.refresh_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Try using the old refresh token again — should fail
    let req2 = test::TestRequest::post()
        .uri("/auth/refresh")
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

    let req = test::TestRequest::post()
        .uri("/auth/revoke")
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
