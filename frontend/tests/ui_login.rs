//! Login page rendering, validation, and auth flow tests.

#[path = "ui_helpers.rs"]
mod ui_helpers;

use ui_helpers::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ═══════════════════════════════════════════════════════════════════════════
//  2 · Login-page rendering tests
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_login_page_renders_brand_and_form() {
    let id = "t-login-render";
    clear_tokens();
    install_mock_fetch_health_false();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    assert!(contains_text(id, "Breakfast"), "brand title");
    assert!(contains_text(id, "Sign in to continue"), "subtitle");
    assert!(has_element(id, "input#username"), "username input");
    assert!(has_element(id, "input#password"), "password input");
    assert!(has_element(id, "button[type=\"submit\"]"), "submit button");
    assert!(contains_text(id, "Sign In"), "button label");

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_email_input_attributes() {
    let id = "t-email-attrs";
    clear_tokens();
    install_mock_fetch_health_false();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    let username: web_sys::HtmlInputElement = document()
        .get_element_by_id(id)
        .unwrap()
        .query_selector("input#username")
        .unwrap()
        .unwrap()
        .unchecked_into();

    assert_eq!(username.type_(), "text");
    assert_eq!(username.placeholder(), "you@example.com or username");
    assert_eq!(username.autocomplete(), "username");

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_password_input_attributes() {
    let id = "t-pwd-attrs";
    clear_tokens();
    install_mock_fetch_health_false();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    let pwd: web_sys::HtmlInputElement = document()
        .get_element_by_id(id)
        .unwrap()
        .query_selector("input#password")
        .unwrap()
        .unwrap()
        .unchecked_into();

    assert_eq!(pwd.type_(), "password");
    assert_eq!(pwd.placeholder(), "Enter your password");
    assert_eq!(pwd.autocomplete(), "current-password");

    remove_test_container(id);
    restore_fetch();
}
// ═══════════════════════════════════════════════════════════════════════════
//  3 · Client-side validation tests
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_empty_form_shows_validation_error() {
    let id = "t-empty-form";
    clear_tokens();
    install_mock_fetch_health_false();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    submit_form(id);
    flush(50).await;

    assert!(
        contains_text(id, "Please enter both username and password"),
        "validation error for empty form"
    );

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_email_only_shows_validation_error() {
    let id = "t-email-only";
    clear_tokens();
    install_mock_fetch_health_false();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#username", "user@example.com");
    flush(50).await;
    submit_form(id);
    flush(50).await;

    assert!(
        contains_text(id, "Please enter both username and password"),
        "validation error when only email provided"
    );

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_password_only_shows_validation_error() {
    let id = "t-pwd-only";
    clear_tokens();
    install_mock_fetch_health_false();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#password", "password123");
    flush(50).await;
    submit_form(id);
    flush(50).await;

    assert!(
        contains_text(id, "Please enter both username and password"),
        "validation error when only password provided"
    );

    remove_test_container(id);
    restore_fetch();
}

// ═══════════════════════════════════════════════════════════════════════════
//  4 · Login-flow integration tests (mocked HTTP)
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_successful_login_shows_dashboard() {
    let id = "t-login-ok";
    clear_tokens();
    install_mock_fetch_success();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#username", "john@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;

    submit_form(id);
    flush(500).await;

    let html = inner_html(id);
    assert!(html.contains("Welcome!"), "Welcome heading");
    assert!(
        html.contains("You have successfully signed in"),
        "success message"
    );
    assert!(html.contains("John Doe"), "user full name");
    assert!(html.contains("john@example.com"), "user email");
    assert!(html.contains("JD"), "user initials");
    assert!(html.contains("Sign Out"), "sign-out button");

    // login form should be gone
    assert!(!has_element(id, "input#username"), "username input hidden");
    assert!(!html.contains("Sign In"), "Sign In button hidden");

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_failed_login_shows_error_and_stays_on_login() {
    let id = "t-login-fail";
    clear_tokens();
    install_mock_fetch_failure();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#username", "wrong@example.com");
    set_input(id, "input#password", "badpassword");
    flush(50).await;

    submit_form(id);
    flush(500).await;

    let html = inner_html(id);
    assert!(
        html.contains("Invalid username or password"),
        "auth error message"
    );
    assert!(html.contains("Breakfast"), "still on login page");
    assert!(html.contains("Sign In"), "Sign In still visible");
    assert!(
        has_element(id, "input#username"),
        "username input still there"
    );
    assert!(!html.contains("Welcome!"), "no dashboard");

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_network_error_shows_connection_message() {
    let id = "t-net-err";
    clear_tokens();
    install_mock_fetch_network_error();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#username", "user@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;

    submit_form(id);
    flush(500).await;

    let html = inner_html(id);
    assert!(
        html.contains("Unable to reach the server"),
        "network error message"
    );
    assert!(html.contains("Sign In"), "still on login page");

    remove_test_container(id);
    restore_fetch();
}

// ═══════════════════════════════════════════════════════════════════════════
//  12 · Login error differentiation tests (500/429)
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_rate_limited_login_shows_429_message() {
    let id = "t-login-429";
    clear_tokens();
    install_mock_fetch_rate_limited();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#username", "user@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;

    submit_form(id);
    flush(500).await;

    let html = inner_html(id);
    assert!(
        html.contains("Too many login attempts"),
        "should show rate limit message, got: {}",
        html
    );
    assert!(html.contains("Sign In"), "still on login page");

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_server_error_login_shows_500_message() {
    let id = "t-login-500";
    clear_tokens();
    install_mock_fetch_server_error();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    set_input(id, "input#username", "user@example.com");
    set_input(id, "input#password", "password123");
    flush(50).await;

    submit_form(id);
    flush(500).await;

    let html = inner_html(id);
    assert!(
        html.contains("unexpected server error"),
        "should show server error message, got: {}",
        html
    );
    assert!(html.contains("Sign In"), "still on login page");

    remove_test_container(id);
    restore_fetch();
}

// ═══════════════════════════════════════════════════════════════════════════
// 13 · First-user registration tests (#671)
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_registration_form_renders_when_setup_required() {
    let id = "t-reg-render";
    clear_tokens();
    install_mock_fetch_setup_required();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(500).await;

    let html = inner_html(id);
    assert!(
        html.contains("Create") && html.contains("admin account"),
        "should show registration heading, got: {}",
        html
    );
    assert!(
        has_element(id, "input#firstname"),
        "should have firstname input"
    );
    assert!(
        has_element(id, "input#lastname"),
        "should have lastname input"
    );
    assert!(has_element(id, "input#username"), "should have email input");
    assert!(
        has_element(id, "input#password"),
        "should have password input"
    );

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_registration_short_password_shows_validation_error() {
    let id = "t-reg-short-pw";
    clear_tokens();
    install_mock_fetch_setup_required();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(500).await;

    set_input(id, "input#firstname", "Admin");
    set_input(id, "input#lastname", "User");
    set_input(id, "input#username", "admin@example.com");
    set_input(id, "input#password", "short");
    flush(50).await;
    submit_form(id);
    flush(100).await;

    let html = inner_html(id);
    assert!(
        html.contains("at least 8 characters"),
        "should show password length error, got: {}",
        html
    );

    remove_test_container(id);
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_registration_success_redirects_to_dashboard() {
    let id = "t-reg-success";
    clear_tokens();
    install_mock_fetch_registration_success();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(500).await;

    set_input(id, "input#firstname", "Admin");
    set_input(id, "input#lastname", "User");
    set_input(id, "input#username", "admin@example.com");
    set_input(id, "input#password", "securepassword123");
    flush(50).await;
    submit_form(id);
    flush(800).await;

    let html = inner_html(id);
    assert!(
        html.contains("Welcome") || html.contains("Dashboard"),
        "should redirect to dashboard after registration, got: {}",
        html
    );

    remove_test_container(id);
    restore_fetch();
}
