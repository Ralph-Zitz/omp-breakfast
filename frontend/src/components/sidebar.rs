use crate::api::{HttpMethod, UserContext, authed_request, session_storage};
use crate::app::Page;
use crate::components::icons::{Icon, IconKind};
use crate::components::theme_toggle::ThemeToggle;
use leptos::prelude::*;

// ── Sidebar component ───────────────────────────────────────────────────────

#[component]
pub fn Sidebar() -> impl IntoView {
    let page = expect_context::<ReadSignal<Page>>();
    let set_page = expect_context::<WriteSignal<Page>>();
    let user = expect_context::<ReadSignal<Option<UserContext>>>();
    let sidebar_open = expect_context::<ReadSignal<bool>>();
    let set_sidebar_open = expect_context::<WriteSignal<bool>>();

    let close_sidebar = move || set_sidebar_open.set(false);

    let nav_to = move |target: Page| {
        set_page.set(target);
        set_sidebar_open.set(false);
    };

    view! {
        // Backdrop overlay (mobile only, visible when sidebar is open)
        <div
            class=move || if sidebar_open.get() { "sidebar-overlay sidebar-overlay--open" } else { "sidebar-overlay" }
            on:click=move |_| close_sidebar()
        />

        <aside class=move || {
            if sidebar_open.get() {
                "sidebar sidebar--open"
            } else {
                "sidebar"
            }
        }>
            // ── Brand header ────────────────────────────────────────────
            <div class="sidebar__header">
                <div class="sidebar__brand">
                    <span class="sidebar__brand-text">
                        "OMP "<span class="brand-accent">"Breakfast"</span>
                    </span>
                </div>
            </div>

            <div class="connect-divider connect-divider--subtle" />

            // ── Navigation ──────────────────────────────────────────────
            <nav class="sidebar__nav">
                <NavItem
                    icon=IconKind::House
                    label="Dashboard"
                    active=Signal::derive(move || page.get() == Page::Dashboard)
                    on_click=move || nav_to(Page::Dashboard)
                />
                <NavItem
                    icon=IconKind::Users
                    label="Teams"
                    active=Signal::derive(move || page.get() == Page::Teams)
                    on_click=move || nav_to(Page::Teams)
                />
                <NavItem
                    icon=IconKind::ClipboardList
                    label="Orders"
                    active=Signal::derive(move || page.get() == Page::Orders)
                    on_click=move || nav_to(Page::Orders)
                />
                <NavItem
                    icon=IconKind::Tag
                    label="Items"
                    active=Signal::derive(move || page.get() == Page::Items)
                    on_click=move || nav_to(Page::Items)
                />

                <div class="connect-divider connect-divider--subtle" />

                <NavItem
                    icon=IconKind::User
                    label="Profile"
                    active=Signal::derive(move || page.get() == Page::Profile)
                    on_click=move || nav_to(Page::Profile)
                />

                // Admin-only items
                {move || {
                    let is_admin = user.get().map(|u| u.is_admin).unwrap_or(false);
                    if is_admin {
                        view! {
                            <NavItem
                                icon=IconKind::ShieldCheck
                                label="Admin"
                                active=Signal::derive(move || page.get() == Page::Admin)
                                on_click=move || nav_to(Page::Admin)
                            />
                            <NavItem
                                icon=IconKind::Gear
                                label="Roles"
                                active=Signal::derive(move || page.get() == Page::Roles)
                                on_click=move || nav_to(Page::Roles)
                            />
                        }.into_any()
                    } else {
                        view! { <div /> }.into_any()
                    }
                }}
            </nav>

            // ── Footer ──────────────────────────────────────────────────
            <div class="sidebar__footer">
                <div class="connect-divider connect-divider--subtle" />

                <ThemeToggle />

                <div class="connect-divider connect-divider--subtle" />

                // User info
                {move || {
                    user.get().map(|u| {
                        let initials = u.initials();
                        let name = u.display_name();
                        let email = u.email.clone();
                        view! {
                            <div class="sidebar__user">
                                <div class="connect-avatar connect-avatar--small connect-avatar--initials connect-avatar--bg-yellow">
                                    <span class="connect-avatar__text">{initials}</span>
                                </div>
                                <div class="sidebar__user-info">
                                    <span class="sidebar__user-name">{name}</span>
                                    <span class="sidebar__user-email">{email}</span>
                                </div>
                            </div>
                        }
                    })
                }}

                <LogoutButton />
            </div>
        </aside>
    }
}

// ── Mobile header (hamburger bar) ───────────────────────────────────────────

#[component]
pub fn MobileHeader() -> impl IntoView {
    let set_sidebar_open = expect_context::<WriteSignal<bool>>();

    view! {
        <header class="mobile-header">
            <button
                class="mobile-header__hamburger"
                aria-label="Open navigation"
                on:click=move |_| set_sidebar_open.set(true)
            >
                <Icon kind=IconKind::Bars size=24 />
            </button>
            <span class="mobile-header__brand">
                "OMP "<span class="brand-accent">"Breakfast"</span>
            </span>
        </header>
    }
}

// ── Navigation item ─────────────────────────────────────────────────────────

#[component]
fn NavItem(
    icon: IconKind,
    label: &'static str,
    active: Signal<bool>,
    on_click: impl Fn() + 'static,
) -> impl IntoView {
    view! {
        <button
            class=move || {
                if active.get() {
                    "connect-menu-item nav-item nav-item--active"
                } else {
                    "connect-menu-item nav-item"
                }
            }
            on:click=move |_| on_click()
        >
            <div class="connect-menu-item-icon">
                <Icon kind=icon size=20 />
            </div>
            <div class="connect-menu-item-text-wrapper">
                <span class="connect-menu-item-label nav-item__label">{label}</span>
            </div>
        </button>
    }
}

// ── Logout button ───────────────────────────────────────────────────────────

#[component]
fn LogoutButton() -> impl IntoView {
    let set_page = expect_context::<WriteSignal<Page>>();
    let set_user = expect_context::<WriteSignal<Option<UserContext>>>();

    let on_logout = move |_| {
        let access = session_storage()
            .and_then(|s| s.get_item("access_token").ok())
            .flatten();
        let refresh = session_storage()
            .and_then(|s| s.get_item("refresh_token").ok())
            .flatten();

        // Clear tokens immediately — before async revocation — to prevent
        // a race window where tokens remain readable in sessionStorage.
        if let Some(storage) = session_storage() {
            let _ = storage.remove_item("access_token");
            let _ = storage.remove_item("refresh_token");
        }

        // Update UI immediately — redirect to login
        set_user.set(None);
        set_page.set(Page::Login);

        // Fire-and-forget: revoke tokens server-side using the saved values.
        leptos::task::spawn_local(async move {
            if let Some(at) = access {
                let body = serde_json::json!({"token": at});
                let _ = authed_request(HttpMethod::Post, "/auth/revoke", Some(&body)).await;
            }
            if let Some(rt) = refresh {
                let body = serde_json::json!({"token": rt});
                let _ = authed_request(HttpMethod::Post, "/auth/revoke", Some(&body)).await;
            }
        });
    };

    view! {
        <button
            class="connect-button connect-button--neutral connect-button--outline connect-button--small sidebar__logout"
            on:click=on_logout
        >
            <span class="connect-button__content">
                <span class="connect-button__icon">
                    <Icon kind=IconKind::ArrowRightFromBracket size=16 />
                </span>
                <span class="connect-button__label">"Sign Out"</span>
            </span>
        </button>
    }
}
