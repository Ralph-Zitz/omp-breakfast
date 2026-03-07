//! Admin password reset, create user, and edit user dialog tests.

#[path = "ui_helpers.rs"]
mod ui_helpers;

use ui_helpers::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ── 14a · Button presence ────────────────────────────────────────────────────

#[wasm_bindgen_test]
async fn test_reset_password_button_present_for_other_users() {
    let id = "t-rpw-btn-present";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    assert!(
        has_element(id, "button[aria-label='Reset password']"),
        "a 'Reset password' button must be present for non-self rows"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_reset_password_button_absent_for_self() {
    let id = "t-rpw-btn-self";
    clear_tokens();
    // install_mock_fetch_full returns only John (the logged-in admin) in the
    // user list, so is_self() is true — no action buttons should be rendered.
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    assert!(
        !has_element(id, "button[aria-label='Reset password']"),
        "the 'Reset password' button must NOT appear for the logged-in admin's own row"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 14b · Dialog opens with correct structure ────────────────────────────────

#[wasm_bindgen_test]
async fn test_reset_password_dialog_opens_on_click() {
    let id = "t-rpw-dialog-open";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    assert!(
        !has_element(id, ".modal-overlay"),
        "no modal should be open before clicking the reset button"
    );

    click_button(id, "button[aria-label='Reset password']");
    flush(200).await;

    assert!(
        has_element(id, ".modal-overlay"),
        "the reset-password dialog must appear after clicking the key button"
    );
    let html = inner_html(id);
    assert!(
        html.contains("Reset Password"),
        "dialog title must contain 'Reset Password', got: {}",
        html
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_reset_password_dialog_shows_target_user_name() {
    let id = "t-rpw-dialog-name";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;
    click_button(id, "button[aria-label='Reset password']");
    flush(200).await;

    let html = inner_html(id);
    // The dialog body contains "Set a new password for <strong>{name}</strong>"
    assert!(
        html.contains("Jane") || html.contains("Smith"),
        "the dialog body must mention the target user's name, got: {}",
        html
    );
    assert!(
        has_element(id, "#reset-pw-new"),
        "new-password input (#reset-pw-new) must be present"
    );
    assert!(
        has_element(id, "#reset-pw-confirm"),
        "confirm-password input (#reset-pw-confirm) must be present"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 14c · Save-button disabled states ───────────────────────────────────────

#[wasm_bindgen_test]
async fn test_reset_password_save_disabled_when_fields_empty() {
    let id = "t-rpw-disabled-empty";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;
    click_button(id, "button[aria-label='Reset password']");
    flush(200).await;

    assert!(
        is_modal_button_disabled(id, "Reset Password"),
        "Save button must be disabled when both password fields are empty"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_reset_password_save_disabled_when_password_too_short() {
    let id = "t-rpw-disabled-short";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;
    click_button(id, "button[aria-label='Reset password']");
    flush(200).await;

    // "short" is only 5 characters — below the 8-character minimum
    set_input(id, "#reset-pw-new", "short");
    set_input(id, "#reset-pw-confirm", "short");
    flush(100).await;

    assert!(
        is_modal_button_disabled(id, "Reset Password"),
        "Save button must be disabled when the password is shorter than 8 characters"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_reset_password_save_disabled_when_passwords_mismatch() {
    let id = "t-rpw-disabled-mismatch";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;
    click_button(id, "button[aria-label='Reset password']");
    flush(200).await;

    set_input(id, "#reset-pw-new", "password123");
    set_input(id, "#reset-pw-confirm", "different999");
    flush(100).await;

    assert!(
        is_modal_button_disabled(id, "Reset Password"),
        "Save button must be disabled when new and confirm passwords do not match"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_reset_password_save_enabled_when_valid() {
    let id = "t-rpw-enabled";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;
    click_button(id, "button[aria-label='Reset password']");
    flush(200).await;

    set_input(id, "#reset-pw-new", "newpassword1");
    set_input(id, "#reset-pw-confirm", "newpassword1");
    flush(100).await;

    assert!(
        !is_modal_button_disabled(id, "Reset Password"),
        "Save button must be enabled when both fields match and are ≥ 8 characters"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 14d · Mismatch error message ────────────────────────────────────────────

#[wasm_bindgen_test]
async fn test_reset_password_mismatch_error_shown() {
    let id = "t-rpw-mismatch-msg";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;
    click_button(id, "button[aria-label='Reset password']");
    flush(200).await;

    set_input(id, "#reset-pw-new", "password123");
    set_input(id, "#reset-pw-confirm", "wrongpass1");
    flush(100).await;

    let html = inner_html(id);
    assert!(
        html.contains("Passwords do not match"),
        "mismatch error message must appear when confirm ≠ new password, got: {}",
        html
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_reset_password_mismatch_error_hidden_when_matching() {
    let id = "t-rpw-no-mismatch";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;
    click_button(id, "button[aria-label='Reset password']");
    flush(200).await;

    set_input(id, "#reset-pw-new", "matchingpw1");
    set_input(id, "#reset-pw-confirm", "matchingpw1");
    flush(100).await;

    let html = inner_html(id);
    assert!(
        !html.contains("Passwords do not match"),
        "mismatch error must be hidden when both passwords are equal"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 14e · Cancel dismisses the dialog ───────────────────────────────────────

#[wasm_bindgen_test]
async fn test_reset_password_cancel_closes_dialog() {
    let id = "t-rpw-cancel";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;
    click_button(id, "button[aria-label='Reset password']");
    flush(200).await;

    assert!(
        has_element(id, ".modal-overlay"),
        "dialog should be open before cancel"
    );

    js_sys::eval(&format!(
        r#"(() => {{
            const footer = document.getElementById("{}").querySelector(".modal-footer");
            if (!footer) return;
            for (const btn of footer.querySelectorAll("button")) {{
                if (btn.textContent.includes("Cancel")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("cancel click failed");
    flush(200).await;

    assert!(
        !has_element(id, ".modal-overlay"),
        "the reset-password dialog must be closed after clicking Cancel"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 14f · Successful submission ──────────────────────────────────────────────

#[wasm_bindgen_test]
async fn test_reset_password_success_shows_toast_and_closes_dialog() {
    let id = "t-rpw-success";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    click_button(id, "button[aria-label='Reset password']");
    flush(200).await;

    set_input(id, "#reset-pw-new", "newpassword1");
    set_input(id, "#reset-pw-confirm", "newpassword1");
    flush(100).await;

    // Click the enabled "Reset Password" save button
    js_sys::eval(&format!(
        r#"(() => {{
            const footer = document.getElementById("{}").querySelector(".modal-footer");
            if (!footer) return;
            for (const btn of footer.querySelectorAll("button")) {{
                if (btn.textContent.includes("Reset Password") && !btn.disabled) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("save click failed");
    flush(500).await;

    let html = inner_html(id);
    assert!(
        html.contains("Password reset successfully"),
        "a success toast must appear after the PUT succeeds, got: {}",
        html
    );
    assert!(
        !has_element(id, ".modal-overlay"),
        "the dialog must be closed automatically after a successful reset"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 18a · CreateUserDialog opens ─────────────────────────────────────────────

#[wasm_bindgen_test]
async fn test_create_user_dialog_opens() {
    let id = "t-create-user-open";
    clear_tokens();
    install_mock_fetch_with_user_crud();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    assert!(
        !has_element(id, ".modal-overlay"),
        "no modal before clicking New User"
    );

    // Click "New User" button
    js_sys::eval(&format!(
        r#"(() => {{
            const el = document.getElementById("{}");
            if (!el) return;
            const buttons = el.querySelectorAll("button");
            for (const btn of buttons) {{
                if (btn.textContent.includes("New User")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("click New User failed");
    flush(200).await;

    assert!(
        has_element(id, ".modal-overlay"),
        "create-user dialog should open"
    );
    let html = inner_html(id);
    assert!(
        html.contains("New User"),
        "dialog title should be 'New User'"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 18b · CreateUserDialog has form fields ───────────────────────────────────

#[wasm_bindgen_test]
async fn test_create_user_dialog_has_form_fields() {
    let id = "t-create-user-fields";
    clear_tokens();
    install_mock_fetch_with_user_crud();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    js_sys::eval(&format!(
        r#"(() => {{
            const el = document.getElementById("{}");
            if (!el) return;
            const buttons = el.querySelectorAll("button");
            for (const btn of buttons) {{
                if (btn.textContent.includes("New User")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("click New User failed");
    flush(200).await;

    assert!(has_element(id, "#user-fn"), "First Name input (#user-fn)");
    assert!(has_element(id, "#user-ln"), "Last Name input (#user-ln)");
    assert!(has_element(id, "#user-email"), "Email input (#user-email)");
    assert!(has_element(id, "#user-pw"), "Password input (#user-pw)");

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 18c · CreateUserDialog Create button disabled when empty ─────────────────

#[wasm_bindgen_test]
async fn test_create_user_dialog_create_disabled_when_empty() {
    let id = "t-create-user-disabled";
    clear_tokens();
    install_mock_fetch_with_user_crud();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    js_sys::eval(&format!(
        r#"(() => {{
            const el = document.getElementById("{}");
            if (!el) return;
            const buttons = el.querySelectorAll("button");
            for (const btn of buttons) {{
                if (btn.textContent.includes("New User")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("click New User failed");
    flush(200).await;

    assert!(
        is_modal_button_disabled(id, "Create"),
        "Create button should be disabled when fields are empty"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 18d · CreateUserDialog cancel closes dialog ──────────────────────────────

#[wasm_bindgen_test]
async fn test_create_user_dialog_cancel_closes() {
    let id = "t-create-user-cancel";
    clear_tokens();
    install_mock_fetch_with_user_crud();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    js_sys::eval(&format!(
        r#"(() => {{
            const el = document.getElementById("{}");
            if (!el) return;
            const buttons = el.querySelectorAll("button");
            for (const btn of buttons) {{
                if (btn.textContent.includes("New User")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("click New User failed");
    flush(200).await;

    assert!(has_element(id, ".modal-overlay"), "dialog should be open");

    // Click Cancel
    js_sys::eval(&format!(
        r#"(() => {{
            const footer = document.getElementById("{}").querySelector(".modal-footer");
            if (!footer) return;
            for (const btn of footer.querySelectorAll("button")) {{
                if (btn.textContent.includes("Cancel")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("click Cancel failed");
    flush(200).await;

    assert!(
        !has_element(id, ".modal-overlay"),
        "dialog should be closed after Cancel"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 18e · EditUserDialog opens with correct data ─────────────────────────────

#[wasm_bindgen_test]
async fn test_edit_user_dialog_opens() {
    let id = "t-edit-user-open";
    clear_tokens();
    install_mock_fetch_with_user_crud();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    // Click the edit button for the non-self user (Jane)
    click_button(id, "button[aria-label='Edit user']");
    flush(200).await;

    assert!(
        has_element(id, ".modal-overlay"),
        "edit-user dialog should open"
    );
    let html = inner_html(id);
    assert!(
        html.contains("Edit User"),
        "dialog title should be 'Edit User'"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 18f · EditUserDialog has form fields ─────────────────────────────────────

#[wasm_bindgen_test]
async fn test_edit_user_dialog_has_form_fields() {
    let id = "t-edit-user-fields";
    clear_tokens();
    install_mock_fetch_with_user_crud();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    click_button(id, "button[aria-label='Edit user']");
    flush(200).await;

    assert!(
        has_element(id, "#edit-user-fn"),
        "First Name input (#edit-user-fn)"
    );
    assert!(
        has_element(id, "#edit-user-ln"),
        "Last Name input (#edit-user-ln)"
    );
    assert!(
        has_element(id, "#edit-user-email"),
        "Email input (#edit-user-email)"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 18g · EditUserDialog cancel closes ───────────────────────────────────────

#[wasm_bindgen_test]
async fn test_edit_user_dialog_cancel_closes() {
    let id = "t-edit-user-cancel";
    clear_tokens();
    install_mock_fetch_with_user_crud();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    click_button(id, "button[aria-label='Edit user']");
    flush(200).await;

    assert!(has_element(id, ".modal-overlay"), "dialog should be open");

    js_sys::eval(&format!(
        r#"(() => {{
            const footer = document.getElementById("{}").querySelector(".modal-footer");
            if (!footer) return;
            for (const btn of footer.querySelectorAll("button")) {{
                if (btn.textContent.includes("Cancel")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("click Cancel failed");
    flush(200).await;

    assert!(
        !has_element(id, ".modal-overlay"),
        "dialog should be closed after Cancel"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

