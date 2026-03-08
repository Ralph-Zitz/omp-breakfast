#![allow(dead_code)]
//! Shared helpers for API integration tests.
//!
//! This module is imported by each `api_*.rs` test file via `mod common;`.

use actix_web::{test, web::Data};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Serialize;
use serde_json::{Value, json};

// Re-export items that split test files need via `use common::*`.
pub use actix_web;
pub use breakfast::models::*;
pub use std::net::SocketAddr;

/// Custom claims for forging test tokens (mirrors the server's internal struct).
#[derive(Debug, Serialize, serde::Deserialize)]
pub struct ForgeClaims {
    pub sub: uuid::Uuid,
    pub jti: uuid::Uuid,
    pub token_type: TokenType,
    pub iss: String,
    pub aud: String,
}

/// Fake peer address for test requests (required by actix-governor's PeerIpKeyExtractor).
pub const PEER: SocketAddr = SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
    12345,
);

/// Fixed admin credentials used by all integration tests.
pub const ADMIN_EMAIL: &str = "admin@test.local";
pub const ADMIN_PASSWORD: &str = "Very Secret";

/// Extract the `items` array from a paginated response envelope.
pub fn paginated_items(body: Value) -> Vec<Value> {
    body["items"]
        .as_array()
        .expect("response should have 'items' array")
        .to_vec()
}

/// Decode the `sub` (user_id) claim from a JWT access token.
pub fn admin_user_id_from_token(token: &str) -> String {
    let parts: Vec<&str> = token.split('.').collect();
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .expect("base64-decode JWT payload");
    let claims: Value = serde_json::from_slice(&payload).expect("parse JWT payload");
    claims["sub"].as_str().expect("sub claim").to_string()
}

/// Build a `Data<State>` pointing at the local Docker postgres (no TLS).
///
/// Reads `TEST_DB_PORT` from the environment (default: 5432) so that
/// `make test-integration` can point at the isolated test container on 5433.
pub async fn test_state() -> Data<State> {
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
        jwtsecret: secrecy::SecretString::from("Very Secret".to_string()),
        cache: dashmap::DashMap::new(),
        token_blacklist: dashmap::DashMap::new(),
        login_attempts: dashmap::DashMap::new(),
        avatar_cache: dashmap::DashMap::new(),
    })
}

/// Create a test `App` wired with the given state.
macro_rules! test_app {
    ($state:expr) => {
        actix_web::test::init_service(
            actix_web::App::new()
                .app_data($state.clone())
                .configure(breakfast::routes::routes),
        )
        .await
    };
}
pub(crate) use test_app;

/// Generic login helper: authenticate with email + password via Basic Auth.
pub async fn login_as(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    email: &str,
    password: &str,
) -> Auth {
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", email, password))
            ),
        ))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 200, "login should succeed for {}", email);
    test::read_body_json(resp).await
}

/// Bootstrap the admin user via `POST /auth/register` (idempotent).
///
/// On first call (empty DB) the registration endpoint creates the user,
/// default roles, and a bootstrap team. Subsequent calls get 403
/// ("users already exist") and fall back to Basic Auth login.
pub async fn register_admin(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
) -> Auth {
    let req = test::TestRequest::post()
        .uri("/auth/register")
        .peer_addr(PEER)
        .set_json(json!({
            "firstname": "Test",
            "lastname": "Admin",
            "email": ADMIN_EMAIL,
            "password": ADMIN_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    if resp.status() == 201 {
        return login_as(app, ADMIN_EMAIL, ADMIN_PASSWORD).await;
    }
    login_as(app, ADMIN_EMAIL, ADMIN_PASSWORD).await
}

/// Create a user via the admin API and return their auth tokens + user_id.
pub async fn create_and_login_user(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    admin_token: &str,
    firstname: &str,
    lastname: &str,
    email: &str,
    password: &str,
) -> (Auth, String) {
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": firstname,
            "lastname": lastname,
            "email": email,
            "password": password,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 201, "creating user {} should succeed", email);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    let auth = login_as(app, email, password).await;
    (auth, user_id)
}

/// Look up a role ID by title (e.g. "Admin", "Member", "Team Admin", "Guest").
pub async fn find_role_id(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    token: &str,
    title: &str,
) -> String {
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(app, req).await;
    let roles = paginated_items(test::read_body_json(resp).await);
    roles
        .iter()
        .find(|r| r["title"].as_str() == Some(title))
        .unwrap_or_else(|| panic!("role '{}' not found", title))["role_id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Create a team and return its ID.
pub async fn create_test_team(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    admin_token: &str,
    name: &str,
) -> String {
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"tname": name, "descr": "test team"}))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "creating team '{}' should succeed",
        name
    );
    let team: Value = test::read_body_json(resp).await;
    team["team_id"].as_str().unwrap().to_string()
}

/// Add a user to a team with a given role.
pub async fn add_member(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    token: &str,
    team_id: &str,
    user_id: &str,
    role_id: &str,
) {
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"user_id": user_id, "role_id": role_id}))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 201, "adding member to team should succeed");
}

/// Create a test item (breakfast item) and return its ID.
pub async fn create_test_item(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    admin_token: &str,
    name: &str,
    price: f64,
) -> String {
    let req = test::TestRequest::post()
        .uri("/api/v1.0/items")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "descr": name,
            "price": price,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "creating item '{}' should succeed",
        name
    );
    let item: Value = test::read_body_json(resp).await;
    item["item_id"].as_str().unwrap().to_string()
}
