use crate::api::local_storage;
use leptos::prelude::*;

/// Initialise the theme on app load. Checks localStorage, then OS preference.
/// Returns `true` if dark mode is active.
pub fn init_theme() -> bool {
    let stored = local_storage()
        .and_then(|s| s.get_item("theme").ok())
        .flatten();

    let dark = match stored.as_deref() {
        Some("dark") => true,
        Some("light") => false,
        _ => prefers_dark_scheme(),
    };

    apply_theme(dark);
    dark
}

/// Check whether dark mode is currently active by reading the DOM attribute.
pub fn is_dark_mode() -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .and_then(|el| el.get_attribute("data-mode"))
        .map(|v| v == "dark")
        .unwrap_or(false)
}

fn apply_theme(dark: bool) {
    let mode = if dark { "dark" } else { "light" };
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = el.set_attribute("data-mode", mode);
    }
}

fn prefers_dark_scheme() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok())
        .flatten()
        .map(|mq| mq.matches())
        .unwrap_or(false)
}

// ── ThemeToggle component ───────────────────────────────────────────────────

#[component]
pub fn ThemeToggle() -> impl IntoView {
    // Read initial dark-mode state from the `data-mode` attribute that
    // `init_theme()` has already set on `<html>`. This avoids the race
    // condition where `is_dark_mode()` would return `false` before
    // `init_theme()` applies the attribute.
    let initial = is_dark_mode();
    let (dark, set_dark) = signal(initial);

    let on_click = move |_| {
        let new_dark = !dark.get();
        apply_theme(new_dark);
        if let Some(storage) = local_storage() {
            let _ = storage.set_item("theme", if new_dark { "dark" } else { "light" });
        }
        set_dark.set(new_dark);
    };

    view! {
        <div class="theme-toggle" data-testid="theme-toggle">
            <button
                type="button"
                class="connect-toggle connect-toggle--small connect-toggle--primary theme-toggle__button"
                role="switch"
                aria-checked=move || if dark.get() { "true" } else { "false" }
                aria-label="Toggle dark mode"
                on:click=on_click
                data-active=move || if dark.get() { "true" } else { "false" }
            >
                <span class="theme-toggle__track">
                    <span class="theme-toggle__thumb"
                        style=move || if dark.get() {
                            "transform: translateX(20px)"
                        } else {
                            "transform: translateX(0)"
                        }
                    />
                </span>
                <span class="theme-toggle__label">
                    {move || if dark.get() { "Dark" } else { "Light" }}
                </span>
            </button>
        </div>
    }
}
