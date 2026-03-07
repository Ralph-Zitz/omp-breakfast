//! Session persistence, restore, token refresh, dashboard, and logout tests.

#[path = "ui_helpers.rs"]
mod ui_helpers;

use ui_helpers::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ═══════════════════════════════════════════════════════════════════════════
//  5 · Dashboard & logout tests
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_dashboard_user_card_structure() {
    let id = "t-user-card";
    clear_tokens();
    install_mock_fetch_success();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#username", "john@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;
    submit_form(id);
    flush(500).await;

    assert!(has_element(id, ".user-card"), "user card element");
    assert!(has_element(id, ".connect-avatar"), "avatar element");
    assert!(has_element(id, ".user-name"), "user-name element");
    assert!(has_element(id, ".user-email"), "user-email element");
    assert!(has_element(id, ".success-badge"), "success badge");

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_logout_returns_to_login_page() {
    let id = "t-logout";
    clear_tokens();
    install_mock_fetch_success();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    // Log in
    set_input(id, "input#username", "john@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;
    submit_form(id);
    flush(500).await;

    assert!(contains_text(id, "Welcome!"), "on dashboard after login");

    // Log out
    click_button(id, ".connect-button--outline");
    flush(100).await;

    let html = inner_html(id);
    assert!(html.contains("Breakfast"), "brand restored");
    assert!(html.contains("Sign In"), "Sign In restored");
    assert!(!html.contains("Welcome!"), "dashboard gone");

    remove_test_container(id);
    restore_fetch();
}

// ═══════════════════════════════════════════════════════════════════════════
//  6 · Full end-to-end cycle
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_full_login_validate_logout_cycle() {
    let id = "t-full-cycle";
    clear_tokens();
    install_mock_fetch_success();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    // 1. Verify initial login page
    assert!(contains_text(id, "Breakfast"), "step 1: brand");
    assert!(contains_text(id, "Sign in to continue"), "step 1: subtitle");

    // 2. Empty submit → validation error
    submit_form(id);
    flush(50).await;
    assert!(
        contains_text(id, "Please enter both username and password"),
        "step 2: validation error"
    );

    // 3. Fill credentials and submit → dashboard
    set_input(id, "input#username", "john@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;
    submit_form(id);
    flush(500).await;

    assert!(contains_text(id, "Welcome!"), "step 3: welcome");
    assert!(contains_text(id, "John Doe"), "step 3: name");
    assert!(
        contains_text(id, "john@example.com"),
        "step 3: email on dashboard"
    );
    assert!(
        contains_text(id, "You have successfully signed in"),
        "step 3: success text"
    );

    // 4. Sign out → back to login
    click_button(id, ".connect-button--outline");
    flush(100).await;

    assert!(contains_text(id, "Breakfast"), "step 4: brand after logout");
    assert!(contains_text(id, "Sign In"), "step 4: Sign In after logout");
    assert!(!contains_text(id, "Welcome!"), "step 4: dashboard gone");

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_session_persists_across_page_refresh() {
    // Phase 1: Log in and verify tokens are stored
    let id = "t-session-persist";
    clear_tokens();
    install_mock_fetch_success();
    let container = create_test_container(id);
    let handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    // Log in
    set_input(id, "input#username", "john@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;
    submit_form(id);
    flush(500).await;

    // Verify we're on the dashboard
    assert!(contains_text(id, "Welcome!"), "phase 1: on dashboard");
    assert!(contains_text(id, "John Doe"), "phase 1: user name shown");

    // Verify token was stored in sessionStorage
    let stored_token = get_storage_item("access_token");
    assert!(stored_token.is_some(), "phase 1: access_token stored");
    assert!(
        !stored_token.as_ref().unwrap().is_empty(),
        "phase 1: token not empty"
    );

    // Phase 2: Simulate page refresh by unmounting and re-mounting
    drop(handle);
    remove_test_container(id);

    let container2 = create_test_container(id);
    let _handle2 = leptos::mount::mount_to(container2.clone(), app::App);
    flush(500).await;

    // Should restore directly to dashboard without showing login
    let html = inner_html(id);
    assert!(
        html.contains("Welcome!"),
        "phase 2: session restored to dashboard"
    );
    assert!(html.contains("John Doe"), "phase 2: user name restored");
    assert!(html.contains("john@example.com"), "phase 2: email restored");
    assert!(!html.contains("Sign In"), "phase 2: login form not shown");
    assert!(
        !has_element(id, "input#username"),
        "phase 2: no username input"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_logout_clears_tokens_and_prevents_session_restore() {
    // Phase 1: Log in
    let id = "t-logout-tokens";
    clear_tokens();
    install_mock_fetch_success();
    let container = create_test_container(id);
    let handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#username", "john@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;
    submit_form(id);
    flush(500).await;

    assert!(contains_text(id, "Welcome!"), "phase 1: on dashboard");

    // Verify tokens exist
    assert!(
        get_storage_item("access_token").is_some(),
        "phase 1: access_token exists"
    );
    assert!(
        get_storage_item("refresh_token").is_some(),
        "phase 1: refresh_token exists"
    );

    // Phase 2: Log out
    click_button(id, ".connect-button--outline");
    flush(100).await;

    // Verify login page is shown
    assert!(contains_text(id, "Sign In"), "phase 2: back to login");
    assert!(!contains_text(id, "Welcome!"), "phase 2: dashboard gone");

    // Verify tokens are cleared from sessionStorage
    assert!(
        get_storage_item("access_token").is_none(),
        "phase 2: access_token cleared"
    );
    assert!(
        get_storage_item("refresh_token").is_none(),
        "phase 2: refresh_token cleared"
    );

    // Phase 3: Simulate page refresh after logout - should NOT restore session
    drop(handle);
    remove_test_container(id);

    let container2 = create_test_container(id);
    let _handle2 = leptos::mount::mount_to(container2.clone(), app::App);
    flush(500).await;

    // Should show login page, not dashboard
    let html = inner_html(id);
    assert!(html.contains("Breakfast"), "phase 3: brand shown");
    assert!(html.contains("Sign In"), "phase 3: login page shown");
    assert!(
        !html.contains("Welcome!"),
        "phase 3: no dashboard after logout+refresh"
    );
    assert!(
        has_element(id, "input#username"),
        "phase 3: username input present"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_session_restore_with_malformed_token_falls_back_to_login() {
    let id = "t-malformed-restore";
    clear_tokens();

    // Store a malformed token (not a valid JWT structure)
    if let Some(storage) = web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
    {
        let _ = storage.set_item("access_token", "not-a-valid-jwt");
    }

    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(500).await;

    // Should fall back to login page since the token can't be decoded
    let html = inner_html(id);
    assert!(
        html.contains("Sign In"),
        "malformed token: should show login page"
    );
    assert!(
        !html.contains("Welcome!"),
        "malformed token: should not show dashboard"
    );
    assert!(
        has_element(id, "input#username"),
        "malformed token: username input present"
    );

    remove_test_container(id);
    clear_tokens();
}

#[wasm_bindgen_test]
async fn test_session_restore_with_expired_token_falls_back_to_login() {
    let id = "t-expired-restore";
    clear_tokens();
    install_mock_fetch_user_401();

    // Store a structurally valid token (so it decodes) but the fetch will return 401
    let token = mock_token("12345678-1234-1234-1234-1234567890ab");
    if let Some(storage) = web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
    {
        let _ = storage.set_item("access_token", &token);
    }

    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(500).await;

    // Should fall back to login page since the user fetch returns 401
    let html = inner_html(id);
    assert!(
        html.contains("Sign In"),
        "expired token: should show login page"
    );
    assert!(
        !html.contains("Welcome!"),
        "expired token: should not show dashboard"
    );
    assert!(
        has_element(id, "input#username"),
        "expired token: username input present"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_loading_page_shown_during_session_restore() {
    let id = "t-loading-page";
    clear_tokens();

    // Install a slow-responding fetch mock to catch the loading state
    let token = mock_token("12345678-1234-1234-1234-1234567890ab");
    let js = r#"(() => {
            window.__original_fetch = window.fetch;
            window.fetch = function(input) {
                var url = (typeof input === 'string') ? input : input.url;
                if (url.includes('/api/v1.0/users/') && url.endsWith('/teams')) {
                    return new Promise(function(resolve) {
                        setTimeout(function() {
                            resolve(new Response(
                                JSON.stringify({"items":[],"total":0,"limit":50,"offset":0}),
                                { status: 200, headers: { "Content-Type": "application/json" } }
                            ));
                        }, 2000);
                    });
                }
                if (url.includes('/api/v1.0/users/')) {
                    return new Promise(function(resolve) {
                        setTimeout(function() {
                            resolve(new Response(
                                JSON.stringify({
                                    user_id: "12345678-1234-1234-1234-1234567890ab",
                                    firstname: "John",
                                    lastname: "Doe",
                                    email: "john@example.com",
                                    created: "2025-01-01T00:00:00Z",
                                    changed: "2025-01-01T00:00:00Z"
                                }),
                                { status: 200, headers: { "Content-Type": "application/json" } }
                            ));
                        }, 2000);
                    });
                }
                return Promise.resolve(new Response("Not Found", { status: 404 }));
            };
        })()"#
        .to_string();
    js_sys::eval(&js).expect("install slow mock failed");

    // Store a valid token so session restore triggers
    if let Some(storage) = web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
    {
        let _ = storage.set_item("access_token", &token);
    }

    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    // Check quickly, before the slow fetch resolves
    flush(100).await;

    let html = inner_html(id);
    // During loading, should not show login or dashboard
    assert!(
        !html.contains("Sign In"),
        "loading: should not show login form"
    );
    // The loading page should show some loading indicator (spinner or text)
    assert!(
        html.contains("loading-page") || html.contains("spinner") || html.contains("Loading"),
        "loading: should show loading indicator"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_authed_get_retries_after_401_with_token_refresh() {
    let id = "t-authed-get-retry";
    clear_tokens();
    install_mock_fetch_refresh_retry();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    // Fill in login form and submit
    set_input(id, "input#username", "test@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;
    submit_form(id);

    // Wait for: login → fetch_user_details → authed_get (401) → refresh → retry (200)
    flush(1000).await;

    let html = inner_html(id);

    // The dashboard should render with the refreshed user details
    assert!(
        html.contains("Welcome!"),
        "should reach dashboard after token refresh retry"
    );
    assert!(
        html.contains("Refreshed User"),
        "should show user name from the retried (refreshed) request"
    );
    assert!(
        html.contains("refreshed@example.com"),
        "should show email from the retried request"
    );

    // Verify that sessionStorage was updated with the new (refreshed) token
    let new_token = get_storage_item("access_token");
    assert!(
        new_token.is_some(),
        "access_token should be stored after refresh"
    );
    let new_token_val = new_token.unwrap();
    // The refreshed token should be the far-future one, not the initial mock
    assert!(
        new_token_val.contains('.'),
        "stored token should look like a JWT"
    );

    let new_refresh = get_storage_item("refresh_token");
    assert_eq!(
        new_refresh.as_deref(),
        Some("mock_refresh_new"),
        "refresh_token should be updated to the new one from the refresh response"
    );

    // Verify the mock was called the expected number of times
    let count = js_sys::eval("window.__user_fetch_count")
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u32;
    assert_eq!(
        count, 2,
        "user endpoint should have been called twice (initial 401 + retry 200)"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
    // Clean up the counter
    let _ = js_sys::eval("delete window.__user_fetch_count");
}

#[wasm_bindgen_test]
async fn test_authed_get_double_failure_falls_back_to_login() {
    let id = "t-double-fail";
    clear_tokens();
    install_mock_fetch_double_failure();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    // Fill in login form and submit
    set_input(id, "input#username", "test@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;
    submit_form(id);

    // Wait for: login → fetch_user_details → authed_get (401) → refresh (401) → fallback
    flush(1500).await;

    let html = inner_html(id);

    // Should NOT show the dashboard — double failure means no user data
    assert!(
        !html.contains("Welcome!"),
        "dashboard should NOT render after double failure"
    );

    // Should be back on the login page
    assert!(
        html.contains("Sign In") || has_element(id, "input#username"),
        "should fall back to login page after double failure"
    );

    // Tokens should have been cleared from sessionStorage by try_refresh_token
    let access = get_storage_item("access_token");
    let refresh = get_storage_item("refresh_token");
    assert!(
        access.is_none() || access.as_deref() == Some(""),
        "access_token should be cleared after refresh failure, got: {:?}",
        access
    );
    assert!(
        refresh.is_none() || refresh.as_deref() == Some(""),
        "refresh_token should be cleared after refresh failure, got: {:?}",
        refresh
    );

    // Verify the refresh endpoint was actually called (proving the retry path was exercised)
    let refresh_count = js_sys::eval("window.__double_fail_refresh_count")
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u32;
    assert!(
        refresh_count >= 1,
        "refresh endpoint should have been called at least once, got: {}",
        refresh_count
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
    let _ = js_sys::eval("delete window.__double_fail_user_count");
    let _ = js_sys::eval("delete window.__double_fail_refresh_count");
}

