//! Theme toggle (dark/light mode) tests.

#[path = "ui_helpers.rs"]
mod ui_helpers;

use ui_helpers::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_theme_toggle_renders_on_dashboard() {
    let id = "t-theme-render";
    clear_tokens();
    clear_theme();
    install_mock_fetch_success();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    // Log in to reach dashboard (toggle is in sidebar)
    set_input(id, "input#username", "john@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;
    submit_form(id);
    flush(500).await;

    // Toggle should be present
    assert!(
        has_element(id, "[data-testid=\"theme-toggle\"]"),
        "theme toggle should be present in sidebar"
    );
    assert!(
        has_element(id, ".theme-toggle__button"),
        "toggle button element should exist"
    );
    assert!(
        has_element(id, ".theme-toggle__track"),
        "toggle track should exist"
    );
    assert!(
        has_element(id, ".theme-toggle__thumb"),
        "toggle thumb should exist"
    );

    // Label should show either "Light" or "Dark"
    let html = inner_html(id);
    assert!(
        html.contains("Light") || html.contains("Dark"),
        "toggle label should show Light or Dark"
    );

    remove_test_container(id);
    clear_theme();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_theme_toggle_switches_mode() {
    let id = "t-theme-switch";
    clear_tokens();
    clear_theme();
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

    // data-mode should be set after init_theme runs
    let initial_mode = get_data_mode();
    assert!(
        initial_mode.is_some(),
        "data-mode should be set on <html> after app init"
    );
    let initial_mode = initial_mode.unwrap();
    assert!(
        initial_mode == "light" || initial_mode == "dark",
        "initial data-mode should be 'light' or 'dark', got: {}",
        initial_mode
    );

    // Click the toggle
    click_button(id, ".theme-toggle__button");
    flush(100).await;

    // Mode should have flipped
    let new_mode = get_data_mode().expect("data-mode should still be set after toggle");
    assert_ne!(
        initial_mode, new_mode,
        "data-mode should change after clicking toggle (was '{}', still '{}')",
        initial_mode, new_mode
    );

    // localStorage should reflect the new mode
    let stored = get_local_storage_theme();
    assert_eq!(
        stored.as_deref(),
        Some(new_mode.as_str()),
        "localStorage 'theme' should match the new data-mode"
    );

    // Toggle label should update
    let html = inner_html(id);
    if new_mode == "dark" {
        assert!(
            html.contains("Dark"),
            "label should say 'Dark' after switching to dark"
        );
    } else {
        assert!(
            html.contains("Light"),
            "label should say 'Light' after switching to light"
        );
    }

    remove_test_container(id);
    clear_theme();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_theme_toggle_round_trip() {
    let id = "t-theme-roundtrip";
    clear_tokens();
    clear_theme();
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

    let initial_mode = get_data_mode().expect("data-mode should be set");

    // Click once — mode flips
    click_button(id, ".theme-toggle__button");
    flush(100).await;
    let after_first = get_data_mode().expect("data-mode should be set after first click");
    assert_ne!(initial_mode, after_first, "first click should flip mode");

    // Click again — mode returns to initial
    click_button(id, ".theme-toggle__button");
    flush(100).await;
    let after_second = get_data_mode().expect("data-mode should be set after second click");
    assert_eq!(
        initial_mode, after_second,
        "second click should return to initial mode (expected '{}', got '{}')",
        initial_mode, after_second
    );

    // localStorage should also match the original mode
    let stored = get_local_storage_theme();
    assert_eq!(
        stored.as_deref(),
        Some(initial_mode.as_str()),
        "localStorage should match initial mode after round-trip toggle"
    );

    remove_test_container(id);
    clear_theme();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_theme_toggle_aria_attributes() {
    let id = "t-theme-aria";
    clear_tokens();
    clear_theme();
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

    // Get the toggle button
    let btn = document()
        .get_element_by_id(id)
        .unwrap()
        .query_selector(".theme-toggle__button")
        .unwrap()
        .expect("toggle button should exist");

    // Check role and aria-label
    assert_eq!(
        btn.get_attribute("role").as_deref(),
        Some("switch"),
        "toggle button should have role='switch'"
    );
    assert_eq!(
        btn.get_attribute("aria-label").as_deref(),
        Some("Toggle dark mode"),
        "toggle button should have aria-label"
    );

    // Check initial aria-checked matches the mode
    let initial_mode = get_data_mode().unwrap();
    let expected_checked = if initial_mode == "dark" {
        "true"
    } else {
        "false"
    };
    assert_eq!(
        btn.get_attribute("aria-checked").as_deref(),
        Some(expected_checked),
        "aria-checked should match dark mode state"
    );

    // Click and verify aria-checked flips
    click_button(id, ".theme-toggle__button");
    flush(100).await;

    let new_mode = get_data_mode().unwrap();
    let new_expected = if new_mode == "dark" { "true" } else { "false" };
    // Re-query the button since Leptos may have re-rendered
    let btn = document()
        .get_element_by_id(id)
        .unwrap()
        .query_selector(".theme-toggle__button")
        .unwrap()
        .expect("toggle button should still exist after click");
    assert_eq!(
        btn.get_attribute("aria-checked").as_deref(),
        Some(new_expected),
        "aria-checked should flip after toggle click"
    );

    remove_test_container(id);
    clear_theme();
    restore_fetch();
}

