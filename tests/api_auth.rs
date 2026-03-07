//! Authentication, JWT, token refresh, revocation, rate limiting, and lockout tests.

mod common;

use actix_web::test;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use common::*;
use jwt_compact::{
    AlgorithmExt, Claims as JwtClaims, Header as JwtHeader,
    alg::{Hs256, Hs256Key},
};
use serde_json::{Value, json};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Auth flow (basic auth → token pair)
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn auth_returns_token_pair() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = register_admin(&app).await;
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

    // Ensure admin exists
    register_admin(&app).await;

    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:wrong-password", ADMIN_EMAIL))
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

    let auth: Auth = register_admin(&app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        !body["items"].as_array().unwrap().is_empty(),
        "should have at least the admin user"
    );
}

#[actix_web::test]
#[ignore]
async fn get_teams_with_valid_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = register_admin(&app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        !body["items"].as_array().unwrap().is_empty(),
        "should have at least the bootstrap team"
    );
}

#[actix_web::test]
#[ignore]
async fn get_roles_with_valid_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = register_admin(&app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["items"].as_array().unwrap().len() >= 4,
        "registration creates 4 default roles"
    );
}

#[actix_web::test]
#[ignore]
async fn refresh_token_rejects_access_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = register_admin(&app).await;

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

    let auth: Auth = register_admin(&app).await;

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

    let auth: Auth = register_admin(&app).await;

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

    let auth: Auth = register_admin(&app).await;

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

    let auth: Auth = register_admin(&app).await;

    let req = test::TestRequest::post()
        .uri("/auth/revoke")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"token": "not.a.real.token"}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "invalid token should be rejected");
}

// ===========================================================================
// Rate limiting (#179)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn auth_endpoint_rate_limits_after_burst() {
    let state = test_state().await;
    let app = test_app!(state);
    register_admin(&app).await;

    // Use a separate peer address so register_admin's requests don't consume our burst quota
    let rate_peer: SocketAddr = SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        12345,
    );
    let creds = format!(
        "Basic {}",
        STANDARD.encode(format!("{}:{}", ADMIN_EMAIL, ADMIN_PASSWORD))
    );

    // Send requests up to burst size (10)
    for i in 0..10 {
        let req = test::TestRequest::post()
            .uri("/auth")
            .peer_addr(rate_peer)
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
        .peer_addr(rate_peer)
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
// Token revocation ownership check (#207)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_revoke_another_users_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "NonAdmin",
        "Revoke",
        &format!("nonadmin.revoke.{}@test.local", uid),
        "securepassword",
    )
    .await;

    // Non-admin tries to revoke admin's access token → should fail with 403
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

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn admin_can_revoke_another_users_token() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "Revokee",
        "User",
        &format!("revokee.{}@test.local", uid),
        "securepassword",
    )
    .await;

    // Admin revokes user's token → should succeed
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

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// #263 — Revoked refresh token is rejected by /auth/refresh
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn revoked_refresh_token_is_rejected_by_refresh_endpoint() {
    let state = test_state().await;
    let app = test_app!(state);

    let auth: Auth = register_admin(&app).await;

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

// ===========================================================================
// #400 — Account lockout full lifecycle
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn lockout_lifecycle_5_failures_then_429_then_success_clears() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = format!("lockout-{}@test.local", Uuid::now_v7());
    let test_password = "LockoutTest!123";

    // Create a test user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "Lockout",
            "lastname": "Test",
            "email": &test_email,
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
    state.login_attempts.remove(&test_email);

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
// #351 — refresh token rejected by JWT-protected API endpoint
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn jwt_protected_endpoint_rejects_refresh_token() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

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
// Auth flow edge cases (#461, #462, #387)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn revoke_already_revoked_token_is_idempotent() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

    // Revoke once
    let req = test::TestRequest::post()
        .uri("/auth/revoke")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"token": auth.access_token}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "first revoke should succeed");

    // Login again to get a fresh token (the old one is now revoked)
    let auth2: Auth = register_admin(&app).await;

    // Revoke the same (already-revoked) token again
    let req = test::TestRequest::post()
        .uri("/auth/revoke")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth2.access_token)))
        .set_json(json!({"token": auth.access_token}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "second revoke should also succeed (idempotent)"
    );
}

#[actix_web::test]
#[ignore]
async fn auth_response_has_cache_control_no_store() {
    let state = test_state().await;
    let app = test_app!(state);
    let _admin_auth: Auth = register_admin(&app).await;

    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", ADMIN_EMAIL, ADMIN_PASSWORD))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let cache_control = resp
        .headers()
        .get("Cache-Control")
        .expect("auth response should have Cache-Control header")
        .to_str()
        .unwrap();
    assert_eq!(
        cache_control, "no-store",
        "auth response must have Cache-Control: no-store"
    );
}

#[actix_web::test]
#[ignore]
async fn refresh_response_has_cache_control_no_store() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

    let req = test::TestRequest::post()
        .uri("/auth/refresh")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", auth.refresh_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let cache_control = resp
        .headers()
        .get("Cache-Control")
        .expect("refresh response should have Cache-Control header")
        .to_str()
        .unwrap();
    assert_eq!(
        cache_control, "no-store",
        "refresh response must have Cache-Control: no-store"
    );
}

#[actix_web::test]
#[ignore]
async fn refresh_token_after_user_deleted_returns_error() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a temporary user and login
    let test_email = format!("refresh-deleted-{}@test.local", Uuid::now_v7());
    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "Temp",
        "User",
        &test_email,
        "Very Secret",
    )
    .await;

    // Delete the user via admin
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Try to refresh the deleted user's token → should fail
    let req = test::TestRequest::post()
        .uri("/auth/refresh")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.refresh_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_client_error(),
        "refresh after user deletion should fail, got {}",
        resp.status()
    );
}

// ===========================================================================
// Expired token revocation (#299)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn revoke_expired_token_succeeds() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let admin_token = &auth.access_token;

    // Get admin's user_id from the JWT
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let admin_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some(ADMIN_EMAIL))
        .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_string();
    let admin_uuid: Uuid = admin_id.parse().unwrap();

    // Forge an expired JWT signed with the correct secret
    let now = Utc::now();
    let custom = ForgeClaims {
        sub: admin_uuid,
        jti: Uuid::now_v7(),
        token_type: TokenType::Access,
        iss: "omp-breakfast".to_string(),
        aud: "omp-breakfast".to_string(),
    };
    let mut jwt_claims = JwtClaims::new(custom);
    jwt_claims.expiration = Some(now - chrono::Duration::try_hours(1).unwrap());
    jwt_claims.issued_at = Some(now - chrono::Duration::try_hours(2).unwrap());
    let key = Hs256Key::new(b"Very Secret");
    let expired_token = Hs256
        .token(&JwtHeader::empty(), &jwt_claims, &key)
        .expect("encoding should succeed");

    // Submit the expired token for revocation (using a valid token as bearer)
    let req = test::TestRequest::post()
        .uri("/auth/revoke")
        .peer_addr(PEER)
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"token": expired_token}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "revoking an expired-but-validly-signed token should succeed"
    );
}

// ===========================================================================
// Register returns 403 when users already exist (#614)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn register_when_users_exist_returns_403() {
    let state = test_state().await;
    let app = test_app!(state);

    // First registration succeeds (admin bootstrap)
    let _admin_auth: Auth = register_admin(&app).await;

    // Second registration must fail with 403
    let uid = Uuid::now_v7();
    let req = test::TestRequest::post()
        .uri("/auth/register")
        .peer_addr(PEER)
        .set_json(json!({
            "firstname": "Second",
            "lastname": "User",
            "email": format!("second-{}@test.local", uid),
            "password": "Very Secret"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "registration should be forbidden when users already exist"
    );
}

// ===========================================================================
// #683 — register_first_user validation errors
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn register_with_short_password_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);

    let req = test::TestRequest::post()
        .uri("/auth/register")
        .peer_addr(PEER)
        .set_json(json!({
            "firstname": "Test",
            "lastname": "User",
            "email": format!("valtest-{}@test.local", Uuid::now_v7()),
            "password": "short"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        422,
        "short password should return 422 validation error"
    );
}

#[actix_web::test]
#[ignore]
async fn register_with_missing_firstname_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);

    let req = test::TestRequest::post()
        .uri("/auth/register")
        .peer_addr(PEER)
        .set_json(json!({
            "firstname": "A",
            "lastname": "User",
            "email": format!("valtest-{}@test.local", Uuid::now_v7()),
            "password": "Very Secret Password"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        422,
        "too-short firstname should return 422 validation error"
    );
}
