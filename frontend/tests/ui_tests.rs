use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use breakfast_frontend::app;

// ─── Test helpers ───────────────────────────────────────────────────────────

fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

/// Create a container div with the given ID and attach it to the document body.
fn create_test_container(id: &str) -> web_sys::HtmlElement {
    let doc = document();
    let el = doc.create_element("div").unwrap();
    el.set_id(id);
    doc.body().unwrap().append_child(&el).unwrap();
    el.unchecked_into()
}

/// Remove a test container from the DOM.
fn remove_test_container(id: &str) {
    if let Some(el) = document().get_element_by_id(id) {
        el.remove();
    }
}

/// Get the inner HTML of a container by ID.
fn inner_html(id: &str) -> String {
    document()
        .get_element_by_id(id)
        .map(|el| el.inner_html())
        .unwrap_or_default()
}

/// Check whether the container's HTML includes the given text.
fn contains_text(id: &str, text: &str) -> bool {
    inner_html(id).contains(text)
}

/// Check whether the container has a descendant matching `selector`.
fn has_element(id: &str, selector: &str) -> bool {
    document()
        .get_element_by_id(id)
        .and_then(|el| el.query_selector(selector).ok())
        .flatten()
        .is_some()
}

/// Programmatically set an input's value and fire an `input` event so
/// Leptos's `on:input` handler picks up the change.
fn set_input(container_id: &str, selector: &str, value: &str) {
    js_sys::eval(&format!(
        r#"(() => {{
            const el = document.getElementById("{}").querySelector("{}");
            if (!el) throw new Error("set_input: element not found");
            el.value = "{}";
            el.dispatchEvent(new Event("input", {{ bubbles: true }}));
        }})()"#,
        container_id, selector, value
    ))
    .expect("set_input failed");
}

/// Dispatch a `SubmitEvent` on the first `<form>` inside the container.
fn submit_form(container_id: &str) {
    js_sys::eval(&format!(
        r#"(() => {{
            const form = document.getElementById("{}").querySelector("form");
            if (!form) throw new Error("submit_form: no form found");
            form.dispatchEvent(
                new SubmitEvent("submit", {{ cancelable: true, bubbles: true }})
            );
        }})()"#,
        container_id
    ))
    .expect("submit_form failed");
}

/// Click the first element matching `selector` inside the container.
fn click_button(container_id: &str, selector: &str) {
    js_sys::eval(&format!(
        r#"(() => {{
            const btn = document.getElementById("{}").querySelector("{}");
            if (!btn) throw new Error("click_button: not found");
            btn.click();
        }})()"#,
        container_id, selector
    ))
    .expect("click_button failed");
}

/// Yield to the browser event loop for `ms` milliseconds so that
/// `spawn_local` futures and DOM updates can settle.
async fn flush(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let _ = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
}

/// Clear any stored tokens from sessionStorage to ensure a clean test state.
fn clear_tokens() {
    if let Some(storage) = web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
    {
        let _ = storage.remove_item("access_token");
        let _ = storage.remove_item("refresh_token");
    }
}

/// Build a minimal JWT with the given `sub` claim that
/// `decode_jwt_payload` can parse.
fn mock_token(sub: &str) -> String {
    use base64::Engine;
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
    let payload_json = format!(r#"{{"sub":"{}"}}"#, sub);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
    format!("{}.{}.nosig", header, payload)
}

// ─── Fetch mocking helpers ──────────────────────────────────────────────────

/// Replace `window.fetch` with a mock that returns a successful auth
/// response for `POST /auth` and user details for `GET /api/v1.0/users/*`.
fn install_mock_fetch_success() {
    let token = mock_token("12345678-1234-1234-1234-1234567890ab");
    let js = format!(
        r#"(() => {{
            window.__original_fetch = window.fetch;
            window.fetch = function(input) {{
                var url = (typeof input === 'string') ? input : input.url;
                if (url.endsWith('/auth')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{
                            access_token: "{}",
                            refresh_token: "mock_refresh",
                            token_type: "Bearer",
                            expires_in: 900
                        }}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}
                if (url.includes('/api/v1.0/users/')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{
                            user_id: "12345678-1234-1234-1234-1234567890ab",
                            firstname: "John",
                            lastname: "Doe",
                            email: "john@example.com"
                        }}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}
                return Promise.resolve(new Response("Not Found", {{ status: 404 }}));
            }};
        }})()"#,
        token
    );
    js_sys::eval(&js).expect("install_mock_fetch_success failed");
}

/// Replace `window.fetch` with a mock that returns 401 for `POST /auth`.
fn install_mock_fetch_failure() {
    js_sys::eval(
        r#"(() => {
            window.__original_fetch = window.fetch;
            window.fetch = function(input) {
                var url = (typeof input === 'string') ? input : input.url;
                if (url.endsWith('/auth')) {
                    return Promise.resolve(new Response(
                        JSON.stringify({"error":"Unauthorized"}),
                        { status: 401 }
                    ));
                }
                return Promise.resolve(new Response("Not Found", { status: 404 }));
            };
        })()"#,
    )
    .expect("install_mock_fetch_failure failed");
}

/// Replace `window.fetch` with a mock that always rejects (network error).
fn install_mock_fetch_network_error() {
    js_sys::eval(
        r#"(() => {
            window.__original_fetch = window.fetch;
            window.fetch = function() {
                return Promise.reject(new TypeError("Network error"));
            };
        })()"#,
    )
    .expect("install_mock_fetch_network_error failed");
}

/// Restore the original `window.fetch`.
fn restore_fetch() {
    let _ = js_sys::eval(
        r#"(() => {
            if (window.__original_fetch) {
                window.fetch = window.__original_fetch;
                delete window.__original_fetch;
            }
        })()"#,
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  1 · Pure-logic unit tests (JWT decoding)
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_decode_jwt_valid_token() {
    let token = mock_token("my-user-id");
    let result = app::decode_jwt_payload(&token);
    assert!(result.is_some(), "should parse a valid token");
    assert_eq!(result.unwrap().sub, "my-user-id");
}

#[wasm_bindgen_test]
fn test_decode_jwt_missing_segments() {
    assert!(app::decode_jwt_payload("only.two").is_none());
    assert!(app::decode_jwt_payload("single").is_none());
    assert!(app::decode_jwt_payload("").is_none());
}

#[wasm_bindgen_test]
fn test_decode_jwt_invalid_base64() {
    assert!(app::decode_jwt_payload("a.!!!invalid!!!.c").is_none());
}

#[wasm_bindgen_test]
fn test_decode_jwt_invalid_json() {
    use base64::Engine;
    let not_json = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"this is not json");
    let token = format!("header.{}.sig", not_json);
    assert!(app::decode_jwt_payload(&token).is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
//  2 · Login-page rendering tests
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_login_page_renders_brand_and_form() {
    let id = "t-login-render";
    clear_tokens();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    assert!(contains_text(id, "OMP Breakfast"), "brand title");
    assert!(contains_text(id, "Sign in to continue"), "subtitle");
    assert!(has_element(id, "input#username"), "username input");
    assert!(has_element(id, "input#password"), "password input");
    assert!(has_element(id, "button[type=\"submit\"]"), "submit button");
    assert!(contains_text(id, "Sign In"), "button label");

    remove_test_container(id);
}

#[wasm_bindgen_test]
async fn test_email_input_attributes() {
    let id = "t-email-attrs";
    clear_tokens();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    let username: web_sys::HtmlInputElement = document()
        .get_element_by_id(id)
        .unwrap()
        .query_selector("input#username")
        .unwrap()
        .unwrap()
        .unchecked_into();

    assert_eq!(username.type_(), "text");
    assert_eq!(username.placeholder(), "you@example.com or username");
    assert_eq!(username.autocomplete(), "username");

    remove_test_container(id);
}

#[wasm_bindgen_test]
async fn test_password_input_attributes() {
    let id = "t-pwd-attrs";
    clear_tokens();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    let pwd: web_sys::HtmlInputElement = document()
        .get_element_by_id(id)
        .unwrap()
        .query_selector("input#password")
        .unwrap()
        .unwrap()
        .unchecked_into();

    assert_eq!(pwd.type_(), "password");
    assert_eq!(pwd.placeholder(), "Enter your password");
    assert_eq!(pwd.autocomplete(), "current-password");

    remove_test_container(id);
}

// ═══════════════════════════════════════════════════════════════════════════
//  3 · Client-side validation tests
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_empty_form_shows_validation_error() {
    let id = "t-empty-form";
    clear_tokens();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    submit_form(id);
    flush(50).await;

    assert!(
        contains_text(id, "Please enter both username and password"),
        "validation error for empty form"
    );

    remove_test_container(id);
}

#[wasm_bindgen_test]
async fn test_email_only_shows_validation_error() {
    let id = "t-email-only";
    clear_tokens();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#username", "user@example.com");
    flush(50).await;
    submit_form(id);
    flush(50).await;

    assert!(
        contains_text(id, "Please enter both username and password"),
        "validation error when only email provided"
    );

    remove_test_container(id);
}

#[wasm_bindgen_test]
async fn test_password_only_shows_validation_error() {
    let id = "t-pwd-only";
    clear_tokens();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#password", "password123");
    flush(50).await;
    submit_form(id);
    flush(50).await;

    assert!(
        contains_text(id, "Please enter both username and password"),
        "validation error when only password provided"
    );

    remove_test_container(id);
}

// ═══════════════════════════════════════════════════════════════════════════
//  4 · Login-flow integration tests (mocked HTTP)
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_successful_login_shows_dashboard() {
    let id = "t-login-ok";
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

    let html = inner_html(id);
    assert!(html.contains("Welcome!"), "Welcome heading");
    assert!(
        html.contains("You have successfully signed in"),
        "success message"
    );
    assert!(html.contains("John Doe"), "user full name");
    assert!(html.contains("john@example.com"), "user email");
    assert!(html.contains("JD"), "user initials");
    assert!(html.contains("Sign Out"), "sign-out button");

    // login form should be gone
    assert!(!has_element(id, "input#username"), "username input hidden");
    assert!(!html.contains("Sign In"), "Sign In button hidden");

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_failed_login_shows_error_and_stays_on_login() {
    let id = "t-login-fail";
    clear_tokens();
    install_mock_fetch_failure();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#username", "wrong@example.com");
    set_input(id, "input#password", "badpassword");
    flush(50).await;

    submit_form(id);
    flush(500).await;

    let html = inner_html(id);
    assert!(
        html.contains("Invalid username or password"),
        "auth error message"
    );
    assert!(html.contains("OMP Breakfast"), "still on login page");
    assert!(html.contains("Sign In"), "Sign In still visible");
    assert!(
        has_element(id, "input#username"),
        "username input still there"
    );
    assert!(!html.contains("Welcome!"), "no dashboard");

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_network_error_shows_connection_message() {
    let id = "t-net-err";
    clear_tokens();
    install_mock_fetch_network_error();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#username", "user@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;

    submit_form(id);
    flush(500).await;

    let html = inner_html(id);
    assert!(
        html.contains("Unable to reach the server"),
        "network error message"
    );
    assert!(html.contains("Sign In"), "still on login page");

    remove_test_container(id);
    restore_fetch();
}

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
    assert!(has_element(id, ".avatar"), "avatar element");
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
    click_button(id, ".btn-outline");
    flush(100).await;

    let html = inner_html(id);
    assert!(html.contains("OMP Breakfast"), "brand restored");
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
    assert!(contains_text(id, "OMP Breakfast"), "step 1: brand");
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
    click_button(id, ".btn-outline");
    flush(100).await;

    assert!(
        contains_text(id, "OMP Breakfast"),
        "step 4: brand after logout"
    );
    assert!(contains_text(id, "Sign In"), "step 4: Sign In after logout");
    assert!(!contains_text(id, "Welcome!"), "step 4: dashboard gone");

    remove_test_container(id);
    restore_fetch();
}

// ═══════════════════════════════════════════════════════════════════════════
//  7 · Session persistence tests
// ═══════════════════════════════════════════════════════════════════════════

/// Helper to read a value from sessionStorage.
fn get_storage_item(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
        .and_then(|s| s.get_item(key).ok())
        .flatten()
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
    click_button(id, ".btn-outline");
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
    assert!(html.contains("OMP Breakfast"), "phase 3: brand shown");
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

// ═══════════════════════════════════════════════════════════════════════════
//  8 · Session restore edge cases
// ═══════════════════════════════════════════════════════════════════════════

/// Install a fetch mock that returns 401 for user fetch (simulates expired/invalid token)
fn install_mock_fetch_user_401() {
    js_sys::eval(
        r#"(() => {
            window.__original_fetch = window.fetch;
            window.fetch = function(input) {
                var url = (typeof input === 'string') ? input : input.url;
                if (url.includes('/api/v1.0/users/')) {
                    return Promise.resolve(new Response(
                        JSON.stringify({"error":"Unauthorized"}),
                        { status: 401, headers: { "Content-Type": "application/json" } }
                    ));
                }
                return Promise.resolve(new Response("Not Found", { status: 404 }));
            };
        })()"#,
    )
    .expect("install_mock_fetch_user_401 failed");
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
    let js = format!(
        r#"(() => {{
            window.__original_fetch = window.fetch;
            window.fetch = function(input) {{
                var url = (typeof input === 'string') ? input : input.url;
                if (url.includes('/api/v1.0/users/')) {{
                    return new Promise(function(resolve) {{
                        setTimeout(function() {{
                            resolve(new Response(
                                JSON.stringify({{
                                    user_id: "12345678-1234-1234-1234-1234567890ab",
                                    firstname: "John",
                                    lastname: "Doe",
                                    email: "john@example.com"
                                }}),
                                {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                            ));
                        }}, 2000);
                    }});
                }}
                return Promise.resolve(new Response("Not Found", {{ status: 404 }}));
            }};
        }})()"#,
    );
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
