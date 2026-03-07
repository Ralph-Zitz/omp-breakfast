use crate::api::{
    AuthResponse, UserContext, build_user_context, check_setup_required, session_storage,
};
use crate::app::Page;
use base64::Engine;
use gloo_net::http::Request;
use crate::components::input_handler;
use leptos::prelude::*;
use serde::Serialize;

// ── Registration request body ───────────────────────────────────────────────

#[derive(Serialize)]
struct RegisterRequest {
    firstname: String,
    lastname: String,
    email: String,
    password: String,
}

// ── Login page ──────────────────────────────────────────────────────────────

#[component]
pub fn LoginPage() -> impl IntoView {
    let set_page = expect_context::<WriteSignal<Page>>();
    let set_user = expect_context::<WriteSignal<Option<UserContext>>>();

    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (firstname, set_firstname) = signal(String::new());
    let (lastname, set_lastname) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (loading, set_loading) = signal(false);
    let (setup_required, set_setup_required) = signal(false);
    let (checking, set_checking) = signal(true);

    // Check if first-user registration is needed
    leptos::task::spawn_local_scoped(async move {
        let required = check_setup_required().await;
        set_setup_required.set(required);
        set_checking.set(false);
    });

    // ── Shared post-auth handler ────────────────────────────────────────────
    async fn finish_login(
        auth: AuthResponse,
        set_page: WriteSignal<Page>,
        set_user: WriteSignal<Option<UserContext>>,
        set_error: WriteSignal<Option<String>>,
        set_loading: WriteSignal<bool>,
    ) {
        if let Some(storage) = session_storage() {
            let _ = storage.set_item("access_token", &auth.access_token);
            let _ = storage.set_item("refresh_token", &auth.refresh_token);
        }
        match build_user_context(&auth.access_token).await {
            Some(ctx) => {
                set_user.set(Some(ctx));
                set_page.set(Page::Dashboard);
            }
            None => {
                if let Some(storage) = session_storage() {
                    let _ = storage.remove_item("access_token");
                    let _ = storage.remove_item("refresh_token");
                }
                set_error.set(Some(
                    "Login succeeded but your session could not be verified. Please try again."
                        .into(),
                ));
            }
        }
        set_loading.set(false);
    }

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let username_val = username.get();
        let password_val = password.get();
        let is_setup = setup_required.get();

        // Validate common fields
        if username_val.is_empty() || password_val.is_empty() {
            set_error.set(Some(if is_setup {
                "Please fill in all fields.".into()
            } else {
                "Please enter both username and password.".into()
            }));
            return;
        }

        // Validate registration-specific fields
        if is_setup {
            let first = firstname.get();
            let last = lastname.get();
            if first.is_empty() || last.is_empty() {
                set_error.set(Some("Please fill in all fields.".into()));
                return;
            }
            if first.len() < 2 || first.len() > 50 {
                set_error.set(Some(
                    "First name must be between 2 and 50 characters.".into(),
                ));
                return;
            }
            if last.len() < 2 || last.len() > 50 {
                set_error.set(Some(
                    "Last name must be between 2 and 50 characters.".into(),
                ));
                return;
            }
            if password_val.len() < 8 {
                set_error.set(Some("Password must be at least 8 characters.".into()));
                return;
            }
        }

        set_error.set(None);
        set_loading.set(true);

        if is_setup {
            let first = firstname.get();
            let last = lastname.get();
            leptos::task::spawn_local_scoped(async move {
                let body = RegisterRequest {
                    firstname: first,
                    lastname: last,
                    email: username_val.clone(),
                    password: password_val.clone(),
                };

                let result = Request::post("/auth/register")
                    .json(&body)
                    .map(|r| r.send());

                match result {
                    Ok(fut) => match fut.await {
                        Ok(response) if response.status() == 201 => {
                            // Registration succeeded — now login
                            let credentials = base64::engine::general_purpose::STANDARD
                                .encode(format!("{}:{}", username_val, password_val));

                            match Request::post("/auth")
                                .header("Authorization", &format!("Basic {}", credentials))
                                .send()
                                .await
                            {
                                Ok(login_resp) if login_resp.ok() => {
                                    match login_resp.json::<AuthResponse>().await {
                                        Ok(auth) => {
                                            finish_login(
                                                auth,
                                                set_page,
                                                set_user,
                                                set_error,
                                                set_loading,
                                            )
                                            .await;
                                        }
                                        Err(_) => {
                                            set_error.set(Some(
                                                "Account created but login failed. Please sign in manually.".into(),
                                            ));
                                            set_setup_required.set(false);
                                            set_loading.set(false);
                                        }
                                    }
                                }
                                _ => {
                                    set_error.set(Some(
                                        "Account created but login failed. Please sign in manually."
                                            .into(),
                                    ));
                                    set_setup_required.set(false);
                                    set_loading.set(false);
                                }
                            }
                        }
                        Ok(response) => {
                            let status = response.status();
                            let msg = match status {
                                400 => "Invalid registration data. Please check all fields.".to_string(),
                                403 => "Registration is no longer available. An admin account already exists.".to_string(),
                                422 => "Validation failed. Please check your input (email format, password length, name length).".to_string(),
                                429 => "Too many requests. Please wait a moment and try again.".to_string(),
                                _ => format!("Registration failed (HTTP {}). Please try again.", status),
                            };
                            if status == 403 {
                                set_setup_required.set(false);
                            }
                            set_error.set(Some(msg));
                            set_loading.set(false);
                        }
                        Err(_) => {
                            set_error.set(Some(
                                "Unable to reach the server. Please check your connection and try again.".into(),
                            ));
                            set_loading.set(false);
                        }
                    },
                    Err(_) => {
                        set_error.set(Some("Failed to build request.".into()));
                        set_loading.set(false);
                    }
                }
            });
        } else {
            leptos::task::spawn_local_scoped(async move {
                let credentials = base64::engine::general_purpose::STANDARD
                    .encode(format!("{}:{}", username_val, password_val));

                let result = Request::post("/auth")
                    .header("Authorization", &format!("Basic {}", credentials))
                    .send()
                    .await;

                match result {
                    Ok(response) if response.ok() => match response.json::<AuthResponse>().await {
                        Ok(auth) => {
                            finish_login(auth, set_page, set_user, set_error, set_loading).await;
                        }
                        Err(_) => {
                            set_error
                                .set(Some("Unexpected server response. Please try again.".into()));
                            set_loading.set(false);
                        }
                    },
                    Ok(response) => {
                        let status = response.status();
                        let msg = match status {
                            401 => "Invalid username or password. Please check your credentials and try again.".to_string(),
                            429 => "Too many login attempts. Please wait a few minutes and try again.".to_string(),
                            500 => "An unexpected server error occurred. Please try again later.".to_string(),
                            503 => "The service is temporarily unavailable. Please try again later.".to_string(),
                            _ => format!("Login failed (HTTP {}). Please try again.", status),
                        };
                        set_error.set(Some(msg));
                        set_loading.set(false);
                    }
                    Err(_) => {
                        set_error.set(Some(
                            "Unable to reach the server. Please check your connection and try again."
                                .into(),
                        ));
                        set_loading.set(false);
                    }
                }
            });
        }
    };

    view! {
        <div class="page login-page">
            <div class="card login-card">
                <LoginHeader setup_required />
                {move || {
                    if checking.get() {
                        view! {
                            <div class="login-checking">
                                <span class="connect-progress-circle connect-progress-circle--indeterminate" style="display: inline-block; width: 24px; height: 24px;">
                                    <svg viewBox="0 0 40 40" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
                                        <circle cx="20" cy="20" r="16" fill="none" stroke="currentColor" stroke-width="4" stroke-dasharray="75 25" stroke-linecap="round"/>
                                    </svg>
                                </span>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <LoginForm
                                on_submit
                                error
                                username
                                set_username
                                password
                                set_password
                                loading
                                setup_required
                                firstname
                                set_firstname
                                lastname
                                set_lastname
                            />
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn LoginHeader(setup_required: ReadSignal<bool>) -> impl IntoView {
    view! {
        <header class="card-header">
            <img src="lego-logo.svg" alt="LEGO" class="login-logo" />
            <h1 class="brand">
                "OMP "<span class="brand-accent">"Breakfast"</span>
            </h1>
            <p class="subtitle">
                {move || {
                    if setup_required.get() {
                        "Create the first admin account"
                    } else {
                        "Sign in to continue"
                    }
                }}
            </p>
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
    setup_required: ReadSignal<bool>,
    firstname: ReadSignal<String>,
    set_firstname: WriteSignal<String>,
    lastname: ReadSignal<String>,
    set_lastname: WriteSignal<String>,
) -> impl IntoView {
    let form_ref = NodeRef::<leptos::html::Form>::new();

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" && let Some(form) = form_ref.get() {
            let _ = form.request_submit();
        }
    };

    view! {
        <form node_ref=form_ref on:submit=on_submit on:keydown=on_keydown>
            <ErrorAlert error />
            {move || {
                if setup_required.get() {
                    view! {
                        <NameField
                            id="firstname"
                            label="First Name"
                            placeholder="Enter your first name"
                            value=firstname
                            set_value=set_firstname
                        />
                        <NameField
                            id="lastname"
                            label="Last Name"
                            placeholder="Enter your last name"
                            value=lastname
                            set_value=set_lastname
                        />
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}
            <UsernameField username set_username />
            <PasswordField password set_password />
            <SubmitButton loading setup_required />
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
                    maxlength=255
                    placeholder="you@example.com or username"
                    autocomplete="username"
                    prop:value=move || username.get()
                    on:input=input_handler(set_username)
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
                    maxlength=128
                    placeholder="Enter your password"
                    autocomplete="current-password"
                    prop:value=move || password.get()
                    on:input=input_handler(set_password)
                    on:focus=move |_| set_focused.set(true)
                    on:blur=move |_| set_focused.set(false)
                />
            </div>
        </div>
    }
}

#[component]
fn NameField(
    id: &'static str,
    label: &'static str,
    placeholder: &'static str,
    value: ReadSignal<String>,
    set_value: WriteSignal<String>,
) -> impl IntoView {
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
                <label class="connect-label__text" for=id>{label}</label>
            </div>
            <div class=wrapper_class>
                <input
                    class="connect-text-field__input"
                    id=id
                    type="text"
                    maxlength=50
                    placeholder=placeholder
                    autocomplete="off"
                    prop:value=move || value.get()
                    on:input=input_handler(set_value)
                    on:focus=move |_| set_focused.set(true)
                    on:blur=move |_| set_focused.set(false)
                />
            </div>
        </div>
    }
}

#[component]
fn SubmitButton(loading: ReadSignal<bool>, setup_required: ReadSignal<bool>) -> impl IntoView {
    view! {
        <button
            type="submit"
            class="connect-button connect-button--accent connect-button--large connect-button--full-width"
            aria-disabled=move || if loading.get() { "true" } else { "false" }
            aria-busy=move || if loading.get() { "true" } else { "false" }
            disabled=move || loading.get()
        >
            <span class="connect-button__content">
                {move || {
                    if loading.get() {
                        let text = if setup_required.get() { "Creating account\u{2026}" } else { "Signing in\u{2026}" };
                        view! {
                            <span class="connect-button__icon">
                                <svg class="connect-progress-circle connect-progress-circle--indeterminate" viewBox="0 0 40 40" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" style="width: 20px; height: 20px;">
                                    <circle cx="20" cy="20" r="16" fill="none" stroke="currentColor" stroke-width="4" stroke-dasharray="75 25" stroke-linecap="round"/>
                                </svg>
                            </span>
                            <span class="connect-button__label">{text}</span>
                        }.into_any()
                    } else {
                        let text = if setup_required.get() { "Create Account" } else { "Sign In" };
                        view! {
                            <span class="connect-button__label">{text}</span>
                        }.into_any()
                    }
                }}
            </span>
        </button>
    }
}
