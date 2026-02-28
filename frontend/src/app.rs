use base64::Engine;
use gloo_net::http::Request;
use leptos::prelude::*;
use serde::Deserialize;
use web_sys::wasm_bindgen::JsCast;

// ── API response types ──────────────────────────────────────────────────────

#[derive(Clone, Debug, Deserialize)]
struct AuthResponse {
    access_token: String,
    #[allow(dead_code)]
    refresh_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    expires_in: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct JwtPayload {
    pub sub: String,
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

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
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

/// Attempt to restore a session from a stored JWT in localStorage.
/// Returns the appropriate page to navigate to.
async fn restore_session() -> Page {
    let token = match local_storage()
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
            if let Some(storage) = local_storage() {
                let _ = storage.remove_item("access_token");
                let _ = storage.remove_item("refresh_token");
            }
            return Page::Login;
        }
    };

    // Validate token by fetching user details (backend checks expiry)
    let resp = Request::get(&format!("/api/v1.0/users/{}", payload.sub))
        .header("Authorization", &format!("Bearer {}", token))
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
            if let Some(storage) = local_storage() {
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
                <div class="loading-spinner"></div>
                <p class="loading-text">"Loading…"</p>
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
                        if let Some(storage) = local_storage() {
                            let _ = storage.set_item("access_token", &auth.access_token);
                            let _ = storage.set_item("refresh_token", &auth.refresh_token);
                        }

                        let (name, user_email) =
                            fetch_user_details(&auth.access_token, &username_val).await;

                        set_page.set(Page::Dashboard {
                            name,
                            email: user_email,
                        });
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
            <h1 class="brand">"OMP Breakfast"</h1>
            <p class="subtitle">"Sign in to continue"</p>
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
                <div class="alert alert-error" role="alert">
                    <span class="alert-icon">{"\u{26A0}"}</span>
                    <span>{msg}</span>
                </div>
            }
        })
    }
}

#[component]
fn UsernameField(username: ReadSignal<String>, set_username: WriteSignal<String>) -> impl IntoView {
    view! {
        <div class="form-group">
            <label for="username">"Username"</label>
            <input
                id="username"
                type="text"
                placeholder="you@example.com or username"
                autocomplete="username"
                required=true
                prop:value=move || username.get()
                on:input=move |ev| {
                    let target = ev
                        .target()
                        .unwrap()
                        .unchecked_into::<web_sys::HtmlInputElement>();
                    set_username.set(target.value());
                }
            />
        </div>
    }
}

#[component]
fn PasswordField(password: ReadSignal<String>, set_password: WriteSignal<String>) -> impl IntoView {
    view! {
        <div class="form-group">
            <label for="password">"Password"</label>
            <input
                id="password"
                type="password"
                placeholder="Enter your password"
                autocomplete="current-password"
                required=true
                prop:value=move || password.get()
                on:input=move |ev| {
                    let target = ev
                        .target()
                        .unwrap()
                        .unchecked_into::<web_sys::HtmlInputElement>();
                    set_password.set(target.value());
                }
            />
        </div>
    }
}

#[component]
fn SubmitButton(loading: ReadSignal<bool>) -> impl IntoView {
    view! {
        <button type="submit" class="btn btn-primary" disabled=move || loading.get()>
            {move || {
                if loading.get() {
                    "Signing in\u{2026}"
                } else {
                    "Sign In"
                }
            }}
        </button>
    }
}

/// Fetch user details from the API after authentication.
async fn fetch_user_details(access_token: &str, fallback_email: &str) -> (String, String) {
    let payload = decode_jwt_payload(access_token);
    let user_id = match &payload {
        Some(p) => &p.sub,
        None => return ("User".into(), fallback_email.into()),
    };

    let resp = Request::get(&format!("/api/v1.0/users/{}", user_id))
        .header("Authorization", &format!("Bearer {}", access_token))
        .send()
        .await;

    match resp {
        Ok(r) if r.ok() => match r.json::<UserEntry>().await {
            Ok(user) => (format!("{} {}", user.firstname, user.lastname), user.email),
            Err(_) => ("User".into(), fallback_email.into()),
        },
        _ => ("User".into(), fallback_email.into()),
    }
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
        if let Some(storage) = local_storage() {
            let _ = storage.remove_item("access_token");
            let _ = storage.remove_item("refresh_token");
        }
        set_page.set(Page::Login);
    };

    view! {
        <div class="page dashboard-page">
            <div class="card dashboard-card">
                <SuccessBadge />
                <h1>"Welcome!"</h1>
                <p class="success-text">"You have successfully signed in."</p>
                <UserCard name initials email />
                <button class="btn btn-outline" on:click=on_logout>
                    "Sign Out"
                </button>
            </div>
        </div>
    }
}

#[component]
fn SuccessBadge() -> impl IntoView {
    view! {
        <div class="success-badge">
            <span class="success-check">{"\u{2713}"}</span>
        </div>
    }
}

#[component]
fn UserCard(name: String, initials: String, email: String) -> impl IntoView {
    view! {
        <div class="user-card">
            <div class="avatar">{initials}</div>
            <div class="user-details">
                <span class="user-name">{name}</span>
                <span class="user-email">{email}</span>
            </div>
        </div>
    }
}
