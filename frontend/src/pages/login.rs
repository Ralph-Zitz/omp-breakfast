use crate::api::{AuthResponse, UserContext, build_user_context, session_storage};
use crate::app::Page;
use base64::Engine;
use gloo_net::http::Request;
use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

// ── Login page ──────────────────────────────────────────────────────────────

#[component]
pub fn LoginPage() -> impl IntoView {
    let set_page = expect_context::<WriteSignal<Page>>();
    let set_user = expect_context::<WriteSignal<Option<UserContext>>>();

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
                                    "Login succeeded but your session could not be verified. Please try again.".into(),
                                ));
                            }
                        }
                    }
                    Err(_) => {
                        set_error.set(Some("Unexpected server response. Please try again.".into()));
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
            aria-busy=move || if loading.get() { "true" } else { "false" }
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
