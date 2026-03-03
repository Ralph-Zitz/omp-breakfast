use crate::api::{UserContext, build_user_context, session_storage, try_refresh_token};
use crate::components::sidebar::{MobileHeader, Sidebar};
use crate::components::theme_toggle::init_theme;
use crate::components::toast::{ToastContext, ToastRegion};
use crate::pages::{
    admin::AdminPage, dashboard::DashboardPage, items::ItemsPage, loading::LoadingPage,
    login::LoginPage, orders::OrdersPage, profile::ProfilePage, roles::RolesPage, teams::TeamsPage,
};
use leptos::prelude::*;

// ── Re-exports for backward compatibility with tests ────────────────────────
pub use crate::api::JwtPayload;
pub use crate::api::decode_jwt_payload;

// ── Application page state ──────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum Page {
    Loading,
    Login,
    Dashboard,
    Teams,
    Orders,
    Items,
    Profile,
    Admin,
    Roles,
}

// ── Root component ──────────────────────────────────────────────────────────

#[component]
pub fn App() -> impl IntoView {
    // Initialize theme (light/dark) from localStorage or OS preference
    init_theme();

    // Core application state
    let (page, set_page) = signal(Page::Loading);
    let (user, set_user) = signal(Option::<UserContext>::None);
    let (sidebar_open, set_sidebar_open) = signal(false);

    // Provide context for all child components
    provide_context(page);
    provide_context(set_page);
    provide_context(user);
    provide_context(set_user);
    provide_context(sidebar_open);
    provide_context(set_sidebar_open);

    // Toast notification context
    let toast_ctx = ToastContext::new();
    provide_context(toast_ctx);

    // Attempt to restore session from stored JWT on mount
    wasm_bindgen_futures::spawn_local(async move {
        restore_session(set_page, set_user).await;
    });

    view! {
        <div class="app">
            {move || {
                match page.get() {
                    Page::Loading => {
                        view! { <LoadingPage /> }.into_any()
                    }
                    Page::Login => {
                        view! { <LoginPage /> }.into_any()
                    }
                    _ => {
                        // All authenticated pages get the app shell
                        view! { <AppShell page=page /> }.into_any()
                    }
                }
            }}
            <ToastRegion />
        </div>
    }
}

// ── App shell (sidebar + main content) ──────────────────────────────────────

#[component]
fn AppShell(page: ReadSignal<Page>) -> impl IntoView {
    view! {
        <MobileHeader />
        <div class="app-shell">
            <Sidebar />
            <main class="main-content">
                {move || match page.get() {
                    Page::Dashboard => view! { <DashboardPage /> }.into_any(),
                    Page::Teams => view! { <TeamsPage /> }.into_any(),
                    Page::Orders => view! { <OrdersPage /> }.into_any(),
                    Page::Items => view! { <ItemsPage /> }.into_any(),
                    Page::Profile => view! { <ProfilePage /> }.into_any(),
                    Page::Admin => view! { <AdminPage /> }.into_any(),
                    Page::Roles => view! { <RolesPage /> }.into_any(),
                    // Loading/Login are handled by the parent — shouldn't reach here
                    _ => view! { <div /> }.into_any(),
                }}
            </main>
        </div>
    }
}

// ── Session restore ─────────────────────────────────────────────────────────

/// Attempt to restore a session from a stored JWT in sessionStorage.
async fn restore_session(set_page: WriteSignal<Page>, set_user: WriteSignal<Option<UserContext>>) {
    let token = match session_storage()
        .and_then(|s| s.get_item("access_token").ok())
        .flatten()
    {
        Some(t) if !t.is_empty() => t,
        _ => {
            set_page.set(Page::Login);
            return;
        }
    };

    let payload = match decode_jwt_payload(&token) {
        Some(p) => p,
        None => {
            // Token is malformed — clear it and show login
            if let Some(storage) = session_storage() {
                let _ = storage.remove_item("access_token");
                let _ = storage.remove_item("refresh_token");
            }
            set_page.set(Page::Login);
            return;
        }
    };

    // If the access token is expired, try to refresh before fetching user details
    let active_token = if crate::api::token_needs_refresh(&token, 0) {
        match try_refresh_token().await {
            Some(t) => t,
            None => {
                set_page.set(Page::Login);
                return;
            }
        }
    } else {
        token
    };

    // Re-decode payload in case the token changed after refresh
    let _active_payload = decode_jwt_payload(&active_token).unwrap_or(payload);

    // Build full user context (fetches user details + team memberships)
    match build_user_context(&active_token).await {
        Some(ctx) => {
            set_user.set(Some(ctx));
            set_page.set(Page::Dashboard);
        }
        None => {
            // Token expired or invalid — clear stored tokens
            if let Some(storage) = session_storage() {
                let _ = storage.remove_item("access_token");
                let _ = storage.remove_item("refresh_token");
            }
            set_page.set(Page::Login);
        }
    }
}
