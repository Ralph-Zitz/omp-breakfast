//! Shared helpers for frontend WASM integration tests.

pub use wasm_bindgen::JsCast;

pub use breakfast_frontend::app;

// ─── Test helpers ───────────────────────────────────────────────────────────

pub fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

/// Create a container div with the given ID and attach it to the document body.
pub fn create_test_container(id: &str) -> web_sys::HtmlElement {
    let doc = document();
    let el = doc.create_element("div").unwrap();
    el.set_id(id);
    doc.body().unwrap().append_child(&el).unwrap();
    el.unchecked_into()
}

/// Remove a test container from the DOM.
pub fn remove_test_container(id: &str) {
    if let Some(el) = document().get_element_by_id(id) {
        el.remove();
    }
}

/// Get the inner HTML of a container by ID.
pub fn inner_html(id: &str) -> String {
    document()
        .get_element_by_id(id)
        .map(|el| el.inner_html())
        .unwrap_or_default()
}

/// Check whether the container's HTML includes the given text.
pub fn contains_text(id: &str, text: &str) -> bool {
    inner_html(id).contains(text)
}

/// Check whether the container has a descendant matching `selector`.
pub fn has_element(id: &str, selector: &str) -> bool {
    document()
        .get_element_by_id(id)
        .and_then(|el| el.query_selector(selector).ok())
        .flatten()
        .is_some()
}

/// Programmatically set an input's value and fire an `input` event so
/// Leptos's `on:input` handler picks up the change.
pub fn set_input(container_id: &str, selector: &str, value: &str) {
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
pub fn submit_form(container_id: &str) {
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
pub fn click_button(container_id: &str, selector: &str) {
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
pub async fn flush(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let _ = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
}

/// Clear any stored tokens from sessionStorage to ensure a clean test state.
pub fn clear_tokens() {
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
pub fn mock_token(sub: &str) -> String {
    use base64::Engine;
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
    let payload_json = format!(r#"{{"sub":"{}"}}"#, sub);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
    format!("{}.{}.nosig", header, payload)
}

// ─── Fetch mocking helpers ──────────────────────────────────────────────────

/// Replace `window.fetch` with a mock that returns a successful auth
/// response for `POST /auth`, user details for `GET /api/v1.0/users/*`,
/// empty team list for `GET /api/v1.0/users/*/teams`, and accepts
/// `POST /auth/revoke` (fire-and-forget from logout).
pub fn install_mock_fetch_success() {
    let token = mock_token("12345678-1234-1234-1234-1234567890ab");
    let js = format!(
        r#"(() => {{
            window.__original_fetch = window.fetch;
            window.fetch = function(input) {{
                var url = (typeof input === 'string') ? input : input.url;

                // POST /auth (login)
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

                // GET /api/v1.0/users/*/teams — must come BEFORE the general /users/ check
                if (url.includes('/api/v1.0/users/') && url.endsWith('/teams')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[],"total":0,"limit":50,"offset":0}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/users/*
                if (url.includes('/api/v1.0/users/')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{
                            user_id: "12345678-1234-1234-1234-1234567890ab",
                            firstname: "John",
                            lastname: "Doe",
                            email: "john@example.com",
                            created: "2025-01-01T00:00:00Z",
                            changed: "2025-01-01T00:00:00Z"
                        }}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // POST /auth/revoke (fire-and-forget from logout)
                if (url.includes('/auth/revoke')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"up": true}}),
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
pub fn install_mock_fetch_failure() {
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
pub fn install_mock_fetch_network_error() {
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

/// Replace `window.fetch` with a mock that returns 429 for `POST /auth`.
pub fn install_mock_fetch_rate_limited() {
    js_sys::eval(
        r#"(() => {
            window.__original_fetch = window.fetch;
            window.fetch = function(input) {
                var url = (typeof input === 'string') ? input : input.url;
                if (url.endsWith('/auth')) {
                    return Promise.resolve(new Response(
                        JSON.stringify({"error":"Too Many Requests"}),
                        { status: 429 }
                    ));
                }
                return Promise.resolve(new Response("Not Found", { status: 404 }));
            };
        })()"#,
    )
    .expect("install_mock_fetch_rate_limited failed");
}

/// Replace `window.fetch` with a mock that returns 500 for `POST /auth`.
pub fn install_mock_fetch_server_error() {
    js_sys::eval(
        r#"(() => {
            window.__original_fetch = window.fetch;
            window.fetch = function(input) {
                var url = (typeof input === 'string') ? input : input.url;
                if (url.endsWith('/auth')) {
                    return Promise.resolve(new Response(
                        JSON.stringify({"error":"Internal Server Error"}),
                        { status: 500 }
                    ));
                }
                return Promise.resolve(new Response("Not Found", { status: 404 }));
            };
        })()"#,
    )
    .expect("install_mock_fetch_server_error failed");
}

/// Restore the original `window.fetch`.
pub fn restore_fetch() {
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
//  7 · Session persistence tests
// ═══════════════════════════════════════════════════════════════════════════

/// Helper to read a value from sessionStorage.
pub fn get_storage_item(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
        .and_then(|s| s.get_item(key).ok())
        .flatten()
}

// ═══════════════════════════════════════════════════════════════════════════
//  8 · Session restore edge cases
// ═══════════════════════════════════════════════════════════════════════════

/// Install a fetch mock that returns 401 for user fetch (simulates expired/invalid token)
pub fn install_mock_fetch_user_401() {
    js_sys::eval(
        r#"(() => {
            window.__original_fetch = window.fetch;
            window.fetch = function(input) {
                var url = (typeof input === 'string') ? input : input.url;
                if (url.includes('/api/v1.0/users/') && url.endsWith('/teams')) {
                    return Promise.resolve(new Response(
                        JSON.stringify({"items":[],"total":0,"limit":50,"offset":0}),
                        { status: 200, headers: { "Content-Type": "application/json" } }
                    ));
                }
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

// ═══════════════════════════════════════════════════════════════════════════
//  9 · authed_get token refresh retry
// ═══════════════════════════════════════════════════════════════════════════

/// Install a stateful fetch mock that:
/// - POST /auth       → 200 with tokens (initial login)
/// - GET /api/v1.0/users/* → 401 on the FIRST call, 200 on subsequent calls
///   (simulates a server-side revoked token that triggers `authed_get` retry)
/// - POST /auth/refresh → 200 with a new token pair (refresh succeeds)
pub fn install_mock_fetch_refresh_retry() {
    let token = mock_token("12345678-1234-1234-1234-1234567890ab");
    // Build a token with a far-future exp so token_needs_refresh() won't
    // pre-emptively refresh before the GET request is made.
    let far_future_token = {
        use base64::Engine;
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let exp = (js_sys::Date::now() / 1000.0) as u64 + 3600; // 1 hour from now
        let payload_json = format!(
            r#"{{"sub":"12345678-1234-1234-1234-1234567890ab","exp":{}}}"#,
            exp
        );
        let payload =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        format!("{}.{}.nosig", header, payload)
    };
    let js = format!(
        r#"(() => {{
            window.__original_fetch = window.fetch;
            window.__user_fetch_count = 0;
            window.fetch = function(input, init) {{
                var url = (typeof input === 'string') ? input : input.url;
                var method = 'GET';
                if (init && init.method) {{ method = init.method; }}
                else if (typeof input !== 'string' && input.method) {{ method = input.method; }}

                // POST /auth (initial login)
                if (url.endsWith('/auth') && method === 'POST' && !url.includes('/refresh')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{
                            access_token: "{initial_token}",
                            refresh_token: "mock_refresh_initial",
                            token_type: "Bearer",
                            expires_in: 900
                        }}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // POST /auth/refresh
                if (url.includes('/auth/refresh') && method === 'POST') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{
                            access_token: "{refreshed_token}",
                            refresh_token: "mock_refresh_new",
                            token_type: "Bearer",
                            expires_in: 900
                        }}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/users/*/teams — return empty team list
                if (url.includes('/api/v1.0/users/') && url.endsWith('/teams') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[],"total":0,"limit":50,"offset":0}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/users/* — 401 on first call, 200 on subsequent
                if (url.includes('/api/v1.0/users/') && method === 'GET') {{
                    window.__user_fetch_count++;
                    if (window.__user_fetch_count === 1) {{
                        return Promise.resolve(new Response(
                            JSON.stringify({{"error":"Unauthorized"}}),
                            {{ status: 401, headers: {{ "Content-Type": "application/json" }} }}
                        ));
                    }}
                    return Promise.resolve(new Response(
                        JSON.stringify({{
                            user_id: "12345678-1234-1234-1234-1234567890ab",
                            firstname: "Refreshed",
                            lastname: "User",
                            email: "refreshed@example.com",
                            created: "2025-01-01T00:00:00Z",
                            changed: "2025-01-01T00:00:00Z"
                        }}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // POST /auth/revoke (fire-and-forget from logout — accept silently)
                if (url.includes('/auth/revoke')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"up": true}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                return Promise.resolve(new Response("Not Found", {{ status: 404 }}));
            }};
        }})()"#,
        initial_token = token,
        refreshed_token = far_future_token,
    );
    js_sys::eval(&js).expect("install_mock_fetch_refresh_retry failed");
}

// ═══════════════════════════════════════════════════════════════════════════
//  10 · authed_get double-failure (refresh also fails → back to login)
// ═══════════════════════════════════════════════════════════════════════════

/// Install a fetch mock where:
/// - POST /auth       → 200 with tokens (initial login succeeds)
/// - GET /api/v1.0/users/* → always 401 (token revoked server-side)
/// - POST /auth/refresh → 401 (refresh token also invalid/expired)
///
/// This simulates a double-failure: the access token is rejected, and the
/// refresh token is also rejected. `authed_get` should return `None`,
/// tokens should be cleared, and the user should land on the login page.
pub fn install_mock_fetch_double_failure() {
    let _token = mock_token("12345678-1234-1234-1234-1234567890ab");
    let far_future_token = {
        use base64::Engine;
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let exp = (js_sys::Date::now() / 1000.0) as u64 + 3600;
        let payload_json = format!(
            r#"{{"sub":"12345678-1234-1234-1234-1234567890ab","exp":{}}}"#,
            exp
        );
        let payload =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        format!("{}.{}.nosig", header, payload)
    };
    let js = format!(
        r#"(() => {{
            window.__original_fetch = window.fetch;
            window.__double_fail_user_count = 0;
            window.__double_fail_refresh_count = 0;
            window.fetch = function(input, init) {{
                var url = (typeof input === 'string') ? input : input.url;
                var method = 'GET';
                if (init && init.method) {{ method = init.method; }}
                else if (typeof input !== 'string' && input.method) {{ method = input.method; }}

                // POST /auth (initial login — succeeds)
                if (url.endsWith('/auth') && method === 'POST' && !url.includes('/refresh') && !url.includes('/revoke')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{
                            access_token: "{initial_token}",
                            refresh_token: "mock_refresh_will_fail",
                            token_type: "Bearer",
                            expires_in: 900
                        }}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // POST /auth/refresh — ALWAYS 401 (double failure)
                if (url.includes('/auth/refresh') && method === 'POST') {{
                    window.__double_fail_refresh_count++;
                    return Promise.resolve(new Response(
                        JSON.stringify({{"error":"Invalid or expired refresh token"}}),
                        {{ status: 401, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // POST /auth/revoke — accept silently (fire-and-forget)
                if (url.includes('/auth/revoke')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"up": true}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/users/*/teams — return empty team list
                if (url.includes('/api/v1.0/users/') && url.endsWith('/teams') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[],"total":0,"limit":50,"offset":0}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/users/* — ALWAYS 401 (access token invalid)
                if (url.includes('/api/v1.0/users/') && method === 'GET') {{
                    window.__double_fail_user_count++;
                    return Promise.resolve(new Response(
                        JSON.stringify({{"error":"Unauthorized"}}),
                        {{ status: 401, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                return Promise.resolve(new Response("Not Found", {{ status: 404 }}));
            }};
        }})()"#,
        initial_token = far_future_token,
    );
    js_sys::eval(&js).expect("install_mock_fetch_double_failure failed");
}

// ═══════════════════════════════════════════════════════════════════════════
// 11 · Theme toggle tests
// ═══════════════════════════════════════════════════════════════════════════

/// Helper to clear the `theme` key from localStorage so each test starts
/// with a clean slate (falls back to OS preference).
pub fn clear_theme() {
    if let Some(storage) = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
    {
        let _ = storage.remove_item("theme");
    }
    // Also remove data-mode from <html> so init_theme starts fresh
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = el.remove_attribute("data-mode");
    }
}

/// Helper to read the `data-mode` attribute from `<html>`.
pub fn get_data_mode() -> Option<String> {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .and_then(|el| el.get_attribute("data-mode"))
}

/// Helper to read the `theme` key from localStorage.
pub fn get_local_storage_theme() -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .and_then(|s| s.get_item("theme").ok())
        .flatten()
}

// ═══════════════════════════════════════════════════════════════════════════
// 11 · Page rendering tests (6 new pages)
// ═══════════════════════════════════════════════════════════════════════════

/// Click a sidebar navigation item by its label text.
pub fn click_nav(container_id: &str, label: &str) {
    js_sys::eval(&format!(
        r#"(() => {{
            const items = document.getElementById("{}").querySelectorAll('.nav-item');
            for (const item of items) {{
                if (item.textContent.includes('{}')) {{
                    item.click();
                    return;
                }}
            }}
            throw new Error("nav item not found: {}");
        }})()"#,
        container_id, label, label
    ))
    .expect("click_nav failed");
}

/// Install a comprehensive fetch mock that provides data for all pages.
/// The user is set up as an Admin so admin-only pages are visible.
pub fn install_mock_fetch_full() {
    let token = mock_token("12345678-1234-1234-1234-1234567890ab");
    let js = format!(
        r#"(() => {{
            window.__original_fetch = window.fetch;
            window.fetch = function(input, init) {{
                var url = (typeof input === 'string') ? input : input.url;
                var method = 'GET';
                if (init && init.method) {{ method = init.method; }}
                else if (typeof input !== 'string' && input.method) {{ method = input.method; }}

                // POST /auth (login)
                if (url.endsWith('/auth') && method === 'POST' && !url.includes('/refresh')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{
                            access_token: "{token}",
                            refresh_token: "mock_refresh",
                            token_type: "Bearer",
                            expires_in: 900
                        }}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // POST /auth/revoke
                if (url.includes('/auth/revoke')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"up": true}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/users/*/teams — returns admin membership
                if (url.includes('/api/v1.0/users/') && url.endsWith('/teams')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{
                            "team_id": "aaaaaaaa-1234-1234-1234-1234567890ab",
                            "tname": "Core Team",
                            "title": "Admin",
                            "firstname": "John",
                            "lastname": "Doe",
                            "joined": "2025-01-01T00:00:00Z",
                            "role_changed": "2025-01-01T00:00:00Z"
                        }}],"total":1,"limit":50,"offset":0}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/users/* (single user)
                if (url.match(/\/api\/v1\.0\/users\/[^/]+$/) && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{
                            user_id: "12345678-1234-1234-1234-1234567890ab",
                            firstname: "John",
                            lastname: "Doe",
                            email: "john@example.com",
                            created: "2025-01-01T00:00:00Z",
                            changed: "2025-01-01T00:00:00Z"
                        }}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/users (list all)
                if (url.split('?')[0].endsWith('/api/v1.0/users') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{
                            user_id: "12345678-1234-1234-1234-1234567890ab",
                            firstname: "John",
                            lastname: "Doe",
                            email: "john@example.com",
                            created: "2025-01-01T00:00:00Z",
                            changed: "2025-01-01T00:00:00Z"
                        }}],"total":1,"limit":50,"offset":0}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/teams/*/users
                if (url.match(/\/api\/v1\.0\/teams\/[^/]+\/users/) && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{
                            user_id: "12345678-1234-1234-1234-1234567890ab",
                            firstname: "John",
                            lastname: "Doe",
                            email: "john@example.com",
                            title: "Admin",
                            joined: "2025-01-01T00:00:00Z",
                            role_changed: "2025-01-01T00:00:00Z"
                        }}],"total":1,"limit":50,"offset":0}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/teams/*/orders/*/items
                if (url.match(/\/api\/v1\.0\/teams\/[^/]+\/orders\/[^/]+\/items/) && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[],"total":0,"limit":50,"offset":0}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/teams/*/orders
                if (url.match(/\/api\/v1\.0\/teams\/[^/]+\/orders/) && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{
                            teamorders_id: "aaaa1111-0000-0000-0000-000000000001",
                            teamorders_team_id: "bbbb2222-0000-0000-0000-000000000001",
                            teamorders_user_id: "12345678-1234-1234-1234-1234567890ab",
                            pickup_user_id: null,
                            duedate: "2025-08-01",
                            closed: false,
                            created: "2025-01-01T00:00:00Z",
                            changed: "2025-01-01T00:00:00Z"
                        }}],"total":1,"limit":50,"offset":0}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/teams (list all)
                if (url.split('?')[0].endsWith('/api/v1.0/teams') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{
                            team_id: "bbbb2222-0000-0000-0000-000000000001",
                            tname: "Core Team",
                            descr: "The core breakfast team",
                            created: "2025-01-01T00:00:00Z",
                            changed: "2025-01-01T00:00:00Z"
                        }}],"total":1,"limit":50,"offset":0}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/items
                if (url.split('?')[0].endsWith('/api/v1.0/items') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{
                            item_id: "cccc3333-0000-0000-0000-000000000001",
                            descr: "Croissant",
                            price: "25.00",
                            created: "2025-01-01T00:00:00Z",
                            changed: "2025-01-01T00:00:00Z"
                        }}],"total":1,"limit":50,"offset":0}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                // GET /api/v1.0/roles
                if (url.split('?')[0].endsWith('/api/v1.0/roles') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{
                            role_id: "dddd4444-0000-0000-0000-000000000001",
                            title: "Admin",
                            created: "2025-01-01T00:00:00Z",
                            changed: "2025-01-01T00:00:00Z"
                        }}, {{
                            role_id: "dddd4444-0000-0000-0000-000000000002",
                            title: "Member",
                            created: "2025-01-01T00:00:00Z",
                            changed: "2025-01-01T00:00:00Z"
                        }}],"total":2,"limit":50,"offset":0}}),
                        {{ status: 200, headers: {{ "Content-Type": "application/json" }} }}
                    ));
                }}

                return Promise.resolve(new Response("Not Found", {{ status: 404 }}));
            }};
        }})()"#,
        token = token
    );
    js_sys::eval(&js).expect("install_mock_fetch_full failed");
}

/// Helper: log in and wait for dashboard to appear.
pub async fn login_to_dashboard(id: &str) {
    set_input(id, "input#username", "john@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;
    submit_form(id);
    flush(500).await;
}

// ═══════════════════════════════════════════════════════════════════════════
// 13 · Table alignment & spacing guideline tests
//
// These tests enforce the two design rules from CLAUDE.md:
//   - Every <th> must carry .connect-table-header-cell so the global
//     `text-align: left; vertical-align: middle` rules in main.css apply.
//   - Actions columns must carry .connect-table-header-cell--actions /
//     .connect-table-cell--actions so `width: auto` and `gap` are applied,
//     preventing buttons from being clipped or wrapping to a new line.
//   - No inline `width` style ≤ 100 px on actions cells (regression guard
//     against re-introducing the old `width: 80px` that broke row separators).
// ═══════════════════════════════════════════════════════════════════════════

/// Returns `true` when every `<th>` inside any `<table>` in the container
/// carries the `.connect-table-header-cell` class.
pub fn all_th_have_connect_class(container_id: &str) -> bool {
    js_sys::eval(&format!(
        r#"(() => {{
            const ths = document.getElementById("{id}").querySelectorAll("table th");
            if (ths.length === 0) return false;
            for (const th of ths) {{
                if (!th.classList.contains("connect-table-header-cell")) return false;
            }}
            return true;
        }})()"#,
        id = container_id
    ))
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(false)
}

/// Returns `true` when no element matching `selector` carries an inline
/// `width` style with a narrow hard-coded pixel value (≤ 100 px).
pub fn no_narrow_inline_width(container_id: &str, selector: &str) -> bool {
    js_sys::eval(&format!(
        r#"(() => {{
            const els = document.getElementById("{id}").querySelectorAll("{sel}");
            for (const el of els) {{
                const s = el.getAttribute("style") || "";
                const m = s.match(/width\s*:\s*(\d+)px/i);
                if (m && parseInt(m[1]) <= 100) return false;
            }}
            return true;
        }})()"#,
        id = container_id,
        sel = selector
    ))
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(true)
}

/// Returns the number of `<button>` elements inside the first element
/// matching `selector` in the container.
pub fn button_count_in(container_id: &str, selector: &str) -> u32 {
    js_sys::eval(&format!(
        r#"(() => {{
            const cell = document.getElementById("{id}").querySelector("{sel}");
            return cell ? cell.querySelectorAll("button").length : 0;
        }})()"#,
        id = container_id,
        sel = selector
    ))
    .ok()
    .and_then(|v| v.as_f64())
    .map(|n| n as u32)
    .unwrap_or(0)
}

// ═══════════════════════════════════════════════════════════════════════════
// 14 · Admin password reset tests
//
// Tests for the ResetPasswordDialog component and the reset-password workflow
// in AdminPage:
//   - Key-icon button present for non-self rows, absent for the logged-in
//     admin's own row
//   - Clicking the button opens ResetPasswordDialog with the correct title
//     and the target user's name in the body text
//   - New-password and confirm fields are rendered (#reset-pw-new /
//     #reset-pw-confirm)
//   - Save ("Reset Password") is disabled until: both fields are filled,
//     the new password is ≥ 8 characters, and the two values match
//   - "Passwords do not match" error message appears on a mismatch and
//     disappears once they match
//   - Cancel dismisses the dialog without submitting
//   - A successful PUT response triggers a "Password reset successfully" toast
//     and closes the dialog automatically
// ═══════════════════════════════════════════════════════════════════════════

/// Install a fetch mock based on `install_mock_fetch_full` with two changes:
///   1. The user list contains a **second** user ("Jane Smith") in addition to
///      the logged-in admin (John). Without a second user the action buttons
///      are never rendered because `!is_self()` is false for John's own row.
///   2. A PUT handler for `/api/v1.0/users/*` returns HTTP 200 so the
///      password-reset submission path can be tested end-to-end.
pub fn install_mock_fetch_full_with_second_user() {
    let token = mock_token("12345678-1234-1234-1234-1234567890ab");
    let js = format!(
        r#"(() => {{
            window.__original_fetch = window.fetch;
            window.fetch = function(input, init) {{
                var url = (typeof input === 'string') ? input : input.url;
                var method = 'GET';
                if (init && init.method) {{ method = init.method; }}
                else if (typeof input !== 'string' && input.method) {{ method = input.method; }}

                // POST /auth (login)
                if (url.endsWith('/auth') && method === 'POST' && !url.includes('/refresh')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{access_token:"{token}",refresh_token:"mock_refresh",token_type:"Bearer",expires_in:900}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                // POST /auth/revoke
                if (url.includes('/auth/revoke')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                // GET /api/v1.0/users/*/teams
                if (url.includes('/api/v1.0/users/') && url.endsWith('/teams')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{"team_id":"aaaaaaaa-1234-1234-1234-1234567890ab","tname":"Core Team","title":"Admin","firstname":"John","lastname":"Doe","joined":"2025-01-01T00:00:00Z","role_changed":"2025-01-01T00:00:00Z"}}],"total":1,"limit":50,"offset":0}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                // PUT /api/v1.0/users/* — password / profile update (returns 200)
                if (url.match(/\/api\/v1\.0\/users\/[^/]+$/) && method === 'PUT') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"user_id":"eeee5555-0000-0000-0000-000000000001","firstname":"Jane","lastname":"Smith","email":"jane@example.com","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                // GET /api/v1.0/users/* (single user)
                if (url.match(/\/api\/v1\.0\/users\/[^/]+$/) && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"user_id":"12345678-1234-1234-1234-1234567890ab","firstname":"John","lastname":"Doe","email":"john@example.com","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                // GET /api/v1.0/users (list) — John + Jane so Jane's row gets action buttons
                if (url.split('?')[0].endsWith('/api/v1.0/users') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[
                            {{"user_id":"12345678-1234-1234-1234-1234567890ab","firstname":"John","lastname":"Doe","email":"john@example.com","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}},
                            {{"user_id":"eeee5555-0000-0000-0000-000000000001","firstname":"Jane","lastname":"Smith","email":"jane@example.com","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}
                        ],"total":2,"limit":50,"offset":0}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.match(/\/api\/v1\.0\/teams\/[^/]+\/users/) && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[],"total":0,"limit":50,"offset":0}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.split('?')[0].endsWith('/api/v1.0/teams') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{"team_id":"bbbb2222-0000-0000-0000-000000000001","tname":"Core Team","descr":"The core breakfast team","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}],"total":1,"limit":50,"offset":0}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.split('?')[0].endsWith('/api/v1.0/items') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[],"total":0,"limit":50,"offset":0}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.split('?')[0].endsWith('/api/v1.0/roles') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{"role_id":"dddd4444-0000-0000-0000-000000000001","title":"Admin","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}],"total":1,"limit":50,"offset":0}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                return Promise.resolve(new Response("Not Found", {{status:404}}));
            }};
        }})()"#,
        token = token
    );
    js_sys::eval(&js).expect("install_mock_fetch_full_with_second_user failed");
}

/// Returns `true` if the button whose inner text contains `label_text`
/// inside `.modal-footer` has its `disabled` property set to `true`.
pub fn is_modal_button_disabled(container_id: &str, label_text: &str) -> bool {
    js_sys::eval(&format!(
        r#"(() => {{
            const footer = document.getElementById("{id}").querySelector(".modal-footer");
            if (!footer) return true;
            for (const btn of footer.querySelectorAll("button")) {{
                if (btn.textContent.includes("{lbl}")) return btn.disabled;
            }}
            return true;
        }})()"#,
        id = container_id,
        lbl = label_text
    ))
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(true)
}

// ═══════════════════════════════════════════════════════════════════════════
// 18 · CreateUserDialog and EditUserDialog tests (#511)
//
// Tests for the admin page user management dialogs
// ═══════════════════════════════════════════════════════════════════════════

/// Install a fetch mock that extends `install_mock_fetch_full_with_second_user`
/// by also handling POST /api/v1.0/users (user creation).
pub fn install_mock_fetch_with_user_crud() {
    let token = mock_token("12345678-1234-1234-1234-1234567890ab");
    let js = format!(
        r#"(() => {{
            window.__original_fetch = window.fetch;
            window.fetch = function(input, init) {{
                var url = (typeof input === 'string') ? input : input.url;
                var method = 'GET';
                if (init && init.method) {{ method = init.method; }}
                else if (typeof input !== 'string' && input.method) {{ method = input.method; }}

                if (url.endsWith('/auth') && method === 'POST' && !url.includes('/refresh')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{access_token:"{token}",refresh_token:"mock_refresh",token_type:"Bearer",expires_in:900}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.includes('/auth/revoke')) {{
                    return Promise.resolve(new Response(JSON.stringify({{}}),{{status:200,headers:{{"Content-Type":"application/json"}}}}));
                }}
                if (url.includes('/api/v1.0/users/') && url.endsWith('/teams')) {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{"team_id":"aaaaaaaa-1234-1234-1234-1234567890ab","tname":"Core Team","title":"Admin","firstname":"John","lastname":"Doe","joined":"2025-01-01T00:00:00Z","role_changed":"2025-01-01T00:00:00Z"}}],"total":1,"limit":50,"offset":0}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.match(/\/api\/v1\.0\/users\/[^/]+$/) && method === 'POST') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"user_id":"ffff6666-0000-0000-0000-000000000001","firstname":"New","lastname":"User","email":"new@example.com","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}),
                        {{status:201,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.split('?')[0].endsWith('/api/v1.0/users') && method === 'POST') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"user_id":"ffff6666-0000-0000-0000-000000000001","firstname":"New","lastname":"User","email":"new@example.com","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}),
                        {{status:201,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.match(/\/api\/v1\.0\/users\/[^/]+$/) && method === 'PUT') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"user_id":"eeee5555-0000-0000-0000-000000000001","firstname":"Jane","lastname":"Smith","email":"jane@example.com","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.match(/\/api\/v1\.0\/users\/[^/]+$/) && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"user_id":"12345678-1234-1234-1234-1234567890ab","firstname":"John","lastname":"Doe","email":"john@example.com","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.split('?')[0].endsWith('/api/v1.0/users') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[
                            {{"user_id":"12345678-1234-1234-1234-1234567890ab","firstname":"John","lastname":"Doe","email":"john@example.com","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}},
                            {{"user_id":"eeee5555-0000-0000-0000-000000000001","firstname":"Jane","lastname":"Smith","email":"jane@example.com","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}
                        ],"total":2,"limit":50,"offset":0}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.match(/\/api\/v1\.0\/teams\/[^/]+\/users/) && method === 'GET') {{
                    return Promise.resolve(new Response(JSON.stringify({{"items":[],"total":0,"limit":50,"offset":0}}),{{status:200,headers:{{"Content-Type":"application/json"}}}}));
                }}
                if (url.split('?')[0].endsWith('/api/v1.0/teams') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{"team_id":"bbbb2222-0000-0000-0000-000000000001","tname":"Core Team","descr":"The core breakfast team","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}],"total":1,"limit":50,"offset":0}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                if (url.split('?')[0].endsWith('/api/v1.0/items') && method === 'GET') {{
                    return Promise.resolve(new Response(JSON.stringify({{"items":[],"total":0,"limit":50,"offset":0}}),{{status:200,headers:{{"Content-Type":"application/json"}}}}));
                }}
                if (url.split('?')[0].endsWith('/api/v1.0/roles') && method === 'GET') {{
                    return Promise.resolve(new Response(
                        JSON.stringify({{"items":[{{"role_id":"dddd4444-0000-0000-0000-000000000001","title":"Admin","created":"2025-01-01T00:00:00Z","changed":"2025-01-01T00:00:00Z"}}],"total":1,"limit":50,"offset":0}}),
                        {{status:200,headers:{{"Content-Type":"application/json"}}}}
                    ));
                }}
                return Promise.resolve(new Response("Not Found", {{status:404}}));
            }};
        }})()"#,
        token = token
    );
    js_sys::eval(&js).expect("install_mock_fetch_with_user_crud failed");
}
