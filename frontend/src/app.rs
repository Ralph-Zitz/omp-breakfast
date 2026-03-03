use base64::Engine;
use gloo_net::http::{Request, RequestBuilder};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use web_sys::wasm_bindgen::JsCast;

// ── API response types ──────────────────────────────────────────────────────

#[derive(Clone, Debug, Deserialize)]
struct AuthResponse {
    access_token: String,
    refresh_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    expires_in: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct JwtPayload {
    pub sub: String,
    #[serde(default)]
    pub exp: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
struct UserEntry {
    #[allow(dead_code)]
    user_id: String,
    firstname: String,
    lastname: String,
    email: String,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

pub fn decode_jwt_payload(token: &str) -> Option<JwtPayload> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    serde_json::from_slice(&payload).ok()
}

fn session_storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
}

/// Returns the current time in seconds since the Unix epoch (via JS Date.now()).
fn now_secs() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

/// Check whether the access token is expired or will expire within the given
/// margin (in seconds). Returns `true` when a refresh is needed.
fn token_needs_refresh(token: &str, margin_secs: u64) -> bool {
    match decode_jwt_payload(token) {
        Some(payload) => match payload.exp {
            Some(exp) => now_secs() + margin_secs >= exp,
            // No `exp` claim — assume it's still valid (server will reject if not)
            None => false,
        },
        // Can't decode — treat as needing refresh
        None => true,
    }
}

/// Attempt to refresh the access token using the stored refresh token.
/// On success, stores the new token pair in sessionStorage and returns
/// the new access token. On failure, clears stored tokens and returns `None`.
async fn try_refresh_token() -> Option<String> {
    let refresh_token = session_storage()
        .and_then(|s| s.get_item("refresh_token").ok())
        .flatten()?;

    if refresh_token.is_empty() {
        return None;
    }

    let resp = Request::post("/auth/refresh")
        .header("Authorization", &format!("Bearer {}", refresh_token))
        .send()
        .await
        .ok()?;

    if !resp.ok() {
        // Refresh token is invalid/expired — clear everything
        if let Some(storage) = session_storage() {
            let _ = storage.remove_item("access_token");
            let _ = storage.remove_item("refresh_token");
        }
        return None;
    }

    let auth: AuthResponse = resp.json().await.ok()?;
    if let Some(storage) = session_storage() {
        let _ = storage.set_item("access_token", &auth.access_token);
        let _ = storage.set_item("refresh_token", &auth.refresh_token);
    }
    Some(auth.access_token)
}

/// Get a valid access token, refreshing it if it's expired or about to expire.
/// Returns `None` if no token is available and refresh fails.
async fn get_valid_token() -> Option<String> {
    let token = session_storage()
        .and_then(|s| s.get_item("access_token").ok())
        .flatten()?;

    if token.is_empty() {
        return None;
    }

    // Refresh if the token will expire within 60 seconds
    if token_needs_refresh(&token, 60) {
        return try_refresh_token().await;
    }

    Some(token)
}

/// HTTP methods supported by [`authed_request`].
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)] // Post, Put, Delete will be used when mutation pages are implemented
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

/// Build a [`RequestBuilder`] for the given method and URL.
fn build_method_request(method: HttpMethod, url: &str) -> RequestBuilder {
    match method {
        HttpMethod::Get => Request::get(url),
        HttpMethod::Post => Request::post(url),
        HttpMethod::Put => Request::put(url),
        HttpMethod::Delete => Request::delete(url),
    }
}

/// Perform an authenticated HTTP request with automatic token refresh.
/// If the initial request returns 401, attempts a token refresh and retries once.
/// Returns `None` if the request fails after retry or if no token is available.
///
/// For GET requests without a body, prefer the [`authed_get`] convenience wrapper.
async fn authed_request(
    method: HttpMethod,
    url: &str,
    body: Option<&serde_json::Value>,
) -> Option<gloo_net::http::Response> {
    let token = match get_valid_token().await {
        Some(t) => t,
        None => return None,
    };

    let send_once = |tok: String, m: HttpMethod, u: String, b: Option<serde_json::Value>| async move {
        let req = build_method_request(m, &u).header("Authorization", &format!("Bearer {}", tok));
        match b.as_ref() {
            Some(v) => req.json(v).ok()?.send().await.ok(),
            None => req.send().await.ok(),
        }
    };

    let body_owned = body.cloned();
    let resp = send_once(token, method, url.to_string(), body_owned.clone()).await?;

    if resp.status() == 401 {
        // Token may have been revoked server-side — try refresh
        if let Some(new_token) = try_refresh_token().await {
            return send_once(new_token, method, url.to_string(), body_owned).await;
        }
        return None;
    }

    Some(resp)
}

/// Perform an authenticated GET request with automatic token refresh.
/// Convenience wrapper around [`authed_request`] for body-less GET requests.
async fn authed_get(url: &str) -> Option<gloo_net::http::Response> {
    authed_request(HttpMethod::Get, url, None).await
}

// ── Token revocation ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct TokenRequest {
    token: String,
}

/// Revoke a token server-side via `POST /auth/revoke`. Fire-and-forget: errors
/// are silently ignored so that logout always succeeds from the user's perspective.
async fn revoke_token_server_side(bearer: &str, token_to_revoke: &str) {
    let req = Request::post("/auth/revoke")
        .header("Authorization", &format!("Bearer {}", bearer))
        .header("Content-Type", "application/json")
        .json(&TokenRequest {
            token: token_to_revoke.to_string(),
        });

    if let Ok(req) = req {
        // Intentionally ignoring the response — best-effort revocation.
        let _ = req.send().await;
    }
}

// ── Application state ───────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum Page {
    Loading,
    Login,
    Dashboard { name: String, email: String },
}

// ── Root component ──────────────────────────────────────────────────────────

#[component]
pub fn App() -> impl IntoView {
    let (page, set_page) = signal(Page::Loading);

    // Attempt to restore session from stored JWT on mount
    let set_page_restore = set_page;
    wasm_bindgen_futures::spawn_local(async move {
        let resolved = restore_session().await;
        set_page_restore.set(resolved);
    });

    view! {
        <div class="app">
            {move || {
                match page.get() {
                    Page::Loading => {
                        view! { <LoadingPage /> }.into_any()
                    }
                    Page::Login => {
                        view! { <LoginPage set_page /> }.into_any()
                    }
                    Page::Dashboard { name, email } => {
                        view! {
                            <DashboardPage
                                name=name.clone()
                                email=email.clone()
                                set_page
                            />
                        }
                        .into_any()
                    }
                }
            }}
        </div>
    }
}

/// Attempt to restore a session from a stored JWT in sessionStorage.
/// Returns the appropriate page to navigate to.
async fn restore_session() -> Page {
    let token = match session_storage()
        .and_then(|s| s.get_item("access_token").ok())
        .flatten()
    {
        Some(t) if !t.is_empty() => t,
        _ => return Page::Login,
    };

    let payload = match decode_jwt_payload(&token) {
        Some(p) => p,
        None => {
            // Token is malformed — clear it and show login
            if let Some(storage) = session_storage() {
                let _ = storage.remove_item("access_token");
                let _ = storage.remove_item("refresh_token");
            }
            return Page::Login;
        }
    };

    // If the access token is expired, try to refresh before fetching user details
    let active_token = if token_needs_refresh(&token, 0) {
        match try_refresh_token().await {
            Some(t) => t,
            None => return Page::Login,
        }
    } else {
        token
    };

    // Re-decode payload in case the token changed after refresh
    let active_payload = decode_jwt_payload(&active_token).unwrap_or(payload);

    // Validate token by fetching user details (backend checks expiry)
    let resp = Request::get(&format!("/api/v1.0/users/{}", active_payload.sub))
        .header("Authorization", &format!("Bearer {}", active_token))
        .send()
        .await;

    match resp {
        Ok(r) if r.ok() => match r.json::<UserEntry>().await {
            Ok(user) => Page::Dashboard {
                name: format!("{} {}", user.firstname, user.lastname),
                email: user.email,
            },
            Err(_) => Page::Login,
        },
        _ => {
            // Token expired or invalid — clear stored tokens
            if let Some(storage) = session_storage() {
                let _ = storage.remove_item("access_token");
                let _ = storage.remove_item("refresh_token");
            }
            Page::Login
        }
    }
}

// ── Loading page (shown during session restore) ─────────────────────────────

#[component]
fn LoadingPage() -> impl IntoView {
    view! {
        <div class="page loading-page">
            <div class="card loading-card">
                <div class="connect-progress-circle connect-progress-circle--indeterminate">
                    <svg class="connect-progress-circle__bar" viewBox="0 0 40 40">
                        <circle class="connect-progress-circle__background" cx="20" cy="20" r="17" />
                        <circle class="connect-progress-circle__indicator" cx="20" cy="20" r="17" />
                    </svg>
                </div>
                <p class="loading-text">"Loading\u{2026}"</p>
            </div>
        </div>
    }
}

// ── Login page ──────────────────────────────────────────────────────────────

#[component]
fn LoginPage(set_page: WriteSignal<Page>) -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (loading, set_loading) = signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let username_val = username.get();
        let password_val = password.get();

        if username_val.is_empty() || password_val.is_empty() {
            set_error.set(Some("Please enter both username and password.".into()));
            return;
        }

        set_error.set(None);
        set_loading.set(true);

        wasm_bindgen_futures::spawn_local(async move {
            let credentials = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", username_val, password_val));

            let result = Request::post("/auth")
                .header("Authorization", &format!("Basic {}", credentials))
                .send()
                .await;

            match result {
                Ok(response) if response.ok() => match response.json::<AuthResponse>().await {
                    Ok(auth) => {
                        if let Some(storage) = session_storage() {
                            let _ = storage.set_item("access_token", &auth.access_token);
                            let _ = storage.set_item("refresh_token", &auth.refresh_token);
                        }

                        match fetch_user_details(&auth.access_token).await {
                            Some((name, user_email)) => {
                                set_page.set(Page::Dashboard {
                                    name,
                                    email: user_email,
                                });
                            }
                            None => {
                                // Auth succeeded but user fetch failed (tokens
                                // revoked server-side or double auth failure).
                                // Clear tokens and stay on login.
                                if let Some(storage) = session_storage() {
                                    let _ = storage.remove_item("access_token");
                                    let _ = storage.remove_item("refresh_token");
                                }
                                set_error.set(Some(
                                    "Login succeeded but your session could not be verified. Please try again.".into(),
                                ));
                            }
                        }
                    }
                    Err(_) => {
                        set_error.set(Some("Unexpected server response. Please try again.".into()));
                    }
                },
                Ok(_) => {
                    set_error.set(Some(
                        "Invalid username or password. Please check your credentials and try again."
                            .into(),
                    ));
                }
                Err(_) => {
                    set_error.set(Some(
                        "Unable to reach the server. Please check your connection and try again."
                            .into(),
                    ));
                }
            }

            set_loading.set(false);
        });
    };

    view! {
        <div class="page login-page">
            <div class="card login-card">
                <LoginHeader />
                <LoginForm
                    on_submit
                    error
                    username
                    set_username
                    password
                    set_password
                    loading
                />
            </div>
        </div>
    }
}

#[component]
fn LoginHeader() -> impl IntoView {
    view! {
        <header class="card-header">
            <h1 class="brand">
                "OMP "<span class="brand-accent">"Breakfast"</span>
            </h1>
            <p class="subtitle">"Sign in to continue"</p>
            <div class="brand-bar"></div>
        </header>
    }
}

#[component]
fn LoginForm(
    on_submit: impl Fn(web_sys::SubmitEvent) + 'static,
    error: ReadSignal<Option<String>>,
    username: ReadSignal<String>,
    set_username: WriteSignal<String>,
    password: ReadSignal<String>,
    set_password: WriteSignal<String>,
    loading: ReadSignal<bool>,
) -> impl IntoView {
    view! {
        <form on:submit=on_submit>
            <ErrorAlert error />
            <UsernameField username set_username />
            <PasswordField password set_password />
            <SubmitButton loading />
        </form>
    }
}

#[component]
fn ErrorAlert(error: ReadSignal<Option<String>>) -> impl IntoView {
    move || {
        error.get().map(|msg| {
            view! {
                <div class="connect-inline-alert connect-inline-alert--negative" role="alert">
                    <div class="connect-inline-alert__content-wrapper">
                        <div class="connect-inline-alert__icon-wrapper">
                            <svg class="connect-inline-alert__icon" viewBox="0 0 40 40" fill="currentColor" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
                                <path d="M20.031 36c-5.75 0-11-3-13.875-8-2.875-4.938-2.875-11 0-16 2.875-4.938 8.125-8 13.875-8 5.688 0 10.938 3.063 13.813 8 2.875 5 2.875 11.063 0 16-2.875 5-8.125 8-13.813 8Zm0-24c-.875 0-1.5.688-1.5 1.5v7c0 .875.625 1.5 1.5 1.5.813 0 1.5-.625 1.5-1.5v-7c0-.813-.687-1.5-1.5-1.5Zm-2 14c0 1.125.875 2 2 2 1.063 0 2-.875 2-2 0-1.063-.937-2-2-2-1.125 0-2 .938-2 2Z"/>
                            </svg>
                        </div>
                        <div class="connect-inline-alert__text-wrapper">
                            <p class="connect-inline-alert__message">{msg}</p>
                        </div>
                    </div>
                </div>
            }
        })
    }
}

#[component]
fn UsernameField(username: ReadSignal<String>, set_username: WriteSignal<String>) -> impl IntoView {
    let (focused, set_focused) = signal(false);

    let wrapper_class = move || {
        if focused.get() {
            "connect-text-field__input-wrapper connect-text-field__input-wrapper--is-focused"
        } else {
            "connect-text-field__input-wrapper"
        }
    };

    view! {
        <div class="connect-text-field">
            <div class="connect-label">
                <label class="connect-label__text" for="username">"Username"</label>
            </div>
            <div class=wrapper_class>
                <div class="connect-text-field__enhancer">
                    <svg class="connect-text-field__spot-icon" viewBox="0 0 40 40" fill="currentColor" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" style="width: 20px; height: 20px;">
                        <path d="M20 4.5a7 7 0 1 0 0 14 7 7 0 0 0 0-14ZM14.257 22c-3.464 0-6.32 2.24-6.492 5.7l-.21 4.175a2.5 2.5 0 0 0 2.497 2.625h19.897a2.5 2.5 0 0 0 2.497-2.625l-.21-4.175c-.173-3.46-3.028-5.7-6.492-5.7H14.257Z"/>
                    </svg>
                </div>
                <input
                    class="connect-text-field__input"
                    id="username"
                    type="text"
                    placeholder="you@example.com or username"
                    autocomplete="username"
                    required=true
                    prop:value=move || username.get()
                    on:input=move |ev| {
                        let Some(target) = ev.target() else { return; };
                        let target = target
                            .unchecked_into::<web_sys::HtmlInputElement>();
                        set_username.set(target.value());
                    }
                    on:focus=move |_| set_focused.set(true)
                    on:blur=move |_| set_focused.set(false)
                />
            </div>
        </div>
    }
}

#[component]
fn PasswordField(password: ReadSignal<String>, set_password: WriteSignal<String>) -> impl IntoView {
    let (focused, set_focused) = signal(false);

    let wrapper_class = move || {
        if focused.get() {
            "connect-text-field__input-wrapper connect-text-field__input-wrapper--is-focused"
        } else {
            "connect-text-field__input-wrapper"
        }
    };

    view! {
        <div class="connect-text-field">
            <div class="connect-label">
                <label class="connect-label__text" for="password">"Password"</label>
            </div>
            <div class=wrapper_class>
                <div class="connect-text-field__enhancer">
                    <svg class="connect-text-field__spot-icon" viewBox="0 0 40 40" fill="currentColor" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" style="width: 20px; height: 20px;">
                        <path d="M15 12v3h10v-3c0-2.75-2.25-5-5-5-2.813 0-5 2.25-5 5Zm-4 3v-3c0-4.938 4-9 9-9 4.938 0 9 4.063 9 9v3h1c2.188 0 4 1.813 4 4v12c0 2.25-1.813 4-4 4H10c-2.25 0-4-1.75-4-4V19c0-2.188 1.75-4 4-4h1Z"/>
                    </svg>
                </div>
                <input
                    class="connect-text-field__input"
                    id="password"
                    type="password"
                    placeholder="Enter your password"
                    autocomplete="current-password"
                    required=true
                    prop:value=move || password.get()
                    on:input=move |ev| {
                        let Some(target) = ev.target() else { return; };
                        let target = target
                            .unchecked_into::<web_sys::HtmlInputElement>();
                        set_password.set(target.value());
                    }
                    on:focus=move |_| set_focused.set(true)
                    on:blur=move |_| set_focused.set(false)
                />
            </div>
        </div>
    }
}

#[component]
fn SubmitButton(loading: ReadSignal<bool>) -> impl IntoView {
    view! {
        <button
            type="submit"
            class="connect-button connect-button--accent connect-button--large connect-button--full-width"
            aria-disabled=move || if loading.get() { "true" } else { "false" }
            disabled=move || loading.get()
        >
            <span class="connect-button__content">
                {move || {
                    if loading.get() {
                        view! {
                            <span class="connect-button__icon">
                                <svg class="connect-progress-circle connect-progress-circle--indeterminate" viewBox="0 0 40 40" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" style="width: 20px; height: 20px;">
                                    <circle cx="20" cy="20" r="16" fill="none" stroke="currentColor" stroke-width="4" stroke-dasharray="75 25" stroke-linecap="round"/>
                                </svg>
                            </span>
                            <span class="connect-button__label">"Signing in\u{2026}"</span>
                        }.into_any()
                    } else {
                        view! {
                            <span class="connect-button__label">"Sign In"</span>
                        }.into_any()
                    }
                }}
            </span>
        </button>
    }
}

/// Fetch user details using an authenticated GET request.
///
/// Returns `Some((name, email))` on success, or `None` when the request
/// fails due to an authentication/authorization error (e.g. both the access
/// token and the refresh token are rejected). Callers should treat `None`
/// as a signal that the session is invalid and redirect to the login page.
async fn fetch_user_details(access_token: &str) -> Option<(String, String)> {
    let payload = decode_jwt_payload(access_token)?;
    let url = format!("/api/v1.0/users/{}", payload.sub);
    let resp = authed_get(&url).await?;
    if !resp.ok() {
        return None;
    }
    let user: UserEntry = resp.json().await.ok()?;
    Some((format!("{} {}", user.firstname, user.lastname), user.email))
}

// ── Dashboard page (post-login) ─────────────────────────────────────────────

#[component]
fn DashboardPage(name: String, email: String, set_page: WriteSignal<Page>) -> impl IntoView {
    let initials: String = name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();

    let on_logout = move |_| {
        // Grab tokens before clearing storage so we can revoke them server-side
        let access = session_storage()
            .and_then(|s| s.get_item("access_token").ok())
            .flatten();
        let refresh = session_storage()
            .and_then(|s| s.get_item("refresh_token").ok())
            .flatten();

        // Clear storage immediately so the user is logged out even if revocation fails
        if let Some(storage) = session_storage() {
            let _ = storage.remove_item("access_token");
            let _ = storage.remove_item("refresh_token");
        }
        set_page.set(Page::Login);

        // Fire-and-forget: revoke both tokens server-side
        if let Some(bearer) = &access {
            let bearer = bearer.clone();
            let access_clone = access.clone();
            let refresh_clone = refresh.clone();
            leptos::task::spawn_local(async move {
                if let Some(at) = access_clone {
                    revoke_token_server_side(&bearer, &at).await;
                }
                if let Some(rt) = refresh_clone {
                    revoke_token_server_side(&bearer, &rt).await;
                }
            });
        }
    };

    view! {
        <div class="page dashboard-page">
            <div class="card dashboard-card">
                <SuccessBadge />
                <h1>"Welcome!"</h1>
                <p class="success-text">"You have successfully signed in."</p>
                <UserCard name initials email />
                <button
                    class="connect-button connect-button--neutral connect-button--outline connect-button--medium"
                    on:click=on_logout
                >
                    <span class="connect-button__content">
                        <span class="connect-button__icon">
                            <svg viewBox="0 0 40 40" fill="currentColor" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" style="width: 20px; height: 20px;">
                                <path d="m35.383 21.438-8 8a1.927 1.927 0 0 1-2.813 0 1.927 1.927 0 0 1 0-2.813L29.133 22H16.008c-1.125 0-2-.875-2-2 0-1.063.875-2 2-2h13.125l-4.563-4.563a1.927 1.927 0 0 1 0-2.812 1.927 1.927 0 0 1 2.813 0l8 8c.812.75.812 2.063 0 2.813ZM14.008 10h-4c-1.125 0-2 .938-2 2v16c0 1.125.875 2 2 2h4c1.062 0 2 .938 2 2 0 1.125-.938 2-2 2h-4c-3.313 0-6-2.688-6-6V12a6 6 0 0 1 6-6h4c1.062 0 2 .938 2 2 0 1.125-.938 2-2 2Z"/>
                            </svg>
                        </span>
                        <span class="connect-button__label">"Sign Out"</span>
                    </span>
                </button>
            </div>
        </div>
    }
}

#[component]
fn SuccessBadge() -> impl IntoView {
    view! {
        <div class="success-badge">
            <svg class="success-check-icon" viewBox="0 0 40 40" fill="currentColor" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
                <path d="M20.031 36c-5.75 0-11-3-13.875-8-2.875-4.938-2.875-11 0-16 2.875-4.938 8.125-8 13.875-8 5.688 0 10.938 3.063 13.813 8 2.875 5 2.875 11.063 0 16-2.875 5-8.125 8-13.813 8Zm7.063-18.938h-.063c.625-.562.625-1.5 0-2.125a1.471 1.471 0 0 0-2.062 0l-6.938 7L15.094 19c-.625-.625-1.563-.625-2.125 0a1.369 1.369 0 0 0 0 2.063l4 4c.562.625 1.5.625 2.125 0l8-8Z"/>
            </svg>
        </div>
    }
}

#[component]
fn UserCard(name: String, initials: String, email: String) -> impl IntoView {
    view! {
        <div class="user-card">
            <div class="connect-avatar connect-avatar--x-large connect-avatar--initials connect-avatar--bg-yellow">
                <span class="connect-avatar__text">{initials}</span>
            </div>
            <div class="user-details">
                <span class="user-name">{name}</span>
                <span class="user-email">{email}</span>
            </div>
        </div>
    }
}
