use crate::api::local_storage;
use leptos::prelude::*;

/// Initialise the theme on app load. Checks localStorage, then OS preference.
pub fn init_theme() {
    let stored = local_storage()
        .and_then(|s| s.get_item("theme").ok())
        .flatten();

    let dark = match stored.as_deref() {
        Some("dark") => true,
        Some("light") => false,
        _ => prefers_dark_scheme(),
    };

    apply_theme(dark);
}

/// Toggle between light and dark mode. Persists choice to localStorage.
pub fn toggle_theme() -> bool {
    let currently_dark = is_dark_mode();
    let new_dark = !currently_dark;
    apply_theme(new_dark);
    if let Some(storage) = local_storage() {
        let _ = storage.set_item("theme", if new_dark { "dark" } else { "light" });
    }
    new_dark
}

/// Check whether dark mode is currently active.
pub fn is_dark_mode() -> bool {
    js_sys::eval("document.documentElement.getAttribute('data-mode') === 'dark'")
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn apply_theme(dark: bool) {
    let mode = if dark { "dark" } else { "light" };
    let _ = js_sys::eval(&format!(
        "document.documentElement.setAttribute('data-mode', '{}')",
        mode
    ));
}

fn prefers_dark_scheme() -> bool {
    js_sys::eval("window.matchMedia('(prefers-color-scheme: dark)').matches")
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

// ── ThemeToggle component ───────────────────────────────────────────────────

#[component]
pub fn ThemeToggle() -> impl IntoView {
    let (dark, set_dark) = signal(is_dark_mode());

    let on_toggle = move |_| {
        let new_val = toggle_theme();
        set_dark.set(new_val);
    };

    view! {
        <div class="theme-toggle">
            <label class="connect-toggle connect-toggle--small connect-toggle--primary">
                <div class="connect-toggle__switch">
                    <input
                        class="connect-toggle__native-input"
                        type="checkbox"
                        role="switch"
                        prop:checked=move || dark.get()
                        on:change=on_toggle
                        aria-label="Toggle dark mode"
                    />
                </div>
                <span class="connect-toggle__indicator-text-wrapper">
                    <span class="theme-toggle__label">
                        {move || if dark.get() { "Dark" } else { "Light" }}
                    </span>
                </span>
            </label>
        </div>
    }
}
