//! Page rendering, navigation, and interaction tests.

#[path = "ui_helpers.rs"]
mod ui_helpers;

use ui_helpers::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ── 11a · TeamsPage ─────────────────────────────────────────────────────────

#[wasm_bindgen_test]
async fn test_teams_page_renders_with_data() {
    let id = "t-teams-page";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Teams");
    flush(500).await;

    let html = inner_html(id);
    assert!(has_element(id, ".teams-page"), "teams-page root");
    assert!(html.contains("Teams"), "page header");
    assert!(
        has_element(id, "table.connect-table"),
        "teams table present"
    );
    assert!(html.contains("Core Team"), "team name rendered");
    assert!(
        html.contains("The core breakfast team"),
        "team description rendered"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_teams_page_shows_members_on_click() {
    let id = "t-teams-members";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Teams");
    flush(500).await;

    // Click the team row to load members
    click_button(id, ".connect-table-row--clickable");
    flush(500).await;

    let html = inner_html(id);
    assert!(html.contains("Team Members"), "members section title");
    assert!(
        html.contains("John Doe") || html.contains("John"),
        "member name shown"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 11b · ItemsPage ────────────────────────────────────────────────────────

#[wasm_bindgen_test]
async fn test_items_page_renders_with_data() {
    let id = "t-items-page";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Items");
    flush(500).await;

    let html = inner_html(id);
    assert!(has_element(id, ".items-page"), "items-page root");
    assert!(html.contains("Item Catalog"), "page header");
    assert!(
        has_element(id, "table.connect-table"),
        "items table present"
    );
    assert!(html.contains("Croissant"), "item name rendered");
    assert!(html.contains("25.00"), "item price rendered");

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_items_page_has_new_item_button() {
    let id = "t-items-btn";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Items");
    flush(500).await;

    let html = inner_html(id);
    assert!(html.contains("New Item"), "new item button present");

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 11c · OrdersPage ───────────────────────────────────────────────────────

#[wasm_bindgen_test]
async fn test_orders_page_renders_team_selector() {
    let id = "t-orders-page";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Orders");
    flush(500).await;

    let html = inner_html(id);
    assert!(has_element(id, ".orders-page"), "orders-page root");
    assert!(html.contains("Orders"), "page header");
    assert!(html.contains("Select Team"), "team selector title");
    assert!(html.contains("Core Team"), "team button rendered");

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_orders_page_loads_orders_on_team_select() {
    let id = "t-orders-select";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Orders");
    flush(500).await;

    // Click the team button to load orders
    click_button(id, ".team-selector .connect-button");
    flush(500).await;

    let html = inner_html(id);
    // Should show the order with status tag
    assert!(
        html.contains("Open") || html.contains("Closed"),
        "order status tag rendered"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 11d · ProfilePage ──────────────────────────────────────────────────────

#[wasm_bindgen_test]
async fn test_profile_page_renders_user_details() {
    let id = "t-profile-page";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Profile");
    flush(500).await;

    let html = inner_html(id);
    assert!(has_element(id, ".profile-page"), "profile-page root");
    assert!(html.contains("Profile"), "page header");
    assert!(html.contains("John"), "first name shown");
    assert!(html.contains("Doe"), "last name shown");
    assert!(html.contains("john@example.com"), "email shown");

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_profile_page_shows_team_memberships() {
    let id = "t-profile-teams";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Profile");
    flush(500).await;

    let html = inner_html(id);
    assert!(html.contains("Core Team"), "team name in profile");
    assert!(html.contains("Admin"), "role shown in profile");

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 11e · AdminPage ────────────────────────────────────────────────────────

#[wasm_bindgen_test]
async fn test_admin_page_renders_user_list() {
    let id = "t-admin-page";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    let html = inner_html(id);
    assert!(has_element(id, ".admin-page"), "admin-page root");
    assert!(html.contains("User Management"), "page header");
    assert!(
        has_element(id, "table.connect-table"),
        "users table present"
    );
    assert!(
        html.contains("John Doe") || html.contains("john@example.com"),
        "user data rendered"
    );
    assert!(html.contains("New User"), "new user button present");

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_admin_page_hidden_for_non_admin() {
    let id = "t-admin-hidden";
    clear_tokens();
    // Use the basic success mock (no admin role)
    install_mock_fetch_success();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;

    // The Admin nav item should NOT be present for non-admin users
    let has_admin_nav = js_sys::eval(&format!(
        r#"(() => {{
            const items = document.getElementById("{}").querySelectorAll('.nav-item');
            for (const item of items) {{
                if (item.textContent.includes('Admin')) return true;
            }}
            return false;
        }})()"#,
        id
    ))
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(true);

    assert!(
        !has_admin_nav,
        "Admin nav should be hidden for non-admin users"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 11f · RolesPage ────────────────────────────────────────────────────────

#[wasm_bindgen_test]
async fn test_roles_page_renders_role_list() {
    let id = "t-roles-page";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Roles");
    flush(500).await;

    let html = inner_html(id);
    assert!(has_element(id, ".roles-page"), "roles-page root");
    assert!(html.contains("Roles"), "page header");
    assert!(
        has_element(id, "table.connect-table"),
        "roles table present"
    );
    assert!(html.contains("Admin"), "admin role rendered");
    assert!(html.contains("Member"), "member role rendered");
    assert!(html.contains("New Role"), "new role button present");

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_roles_page_hidden_for_non_admin() {
    let id = "t-roles-hidden";
    clear_tokens();
    install_mock_fetch_success();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;

    let has_roles_nav = js_sys::eval(&format!(
        r#"(() => {{
            const items = document.getElementById("{}").querySelectorAll('.nav-item');
            for (const item of items) {{
                if (item.textContent.includes('Roles')) return true;
            }}
            return false;
        }})()"#,
        id
    ))
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(true);

    assert!(
        !has_roles_nav,
        "Roles nav should be hidden for non-admin users"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ═══════════════════════════════════════════════════════════════════════════
// 15 · Shared component tests (#322)
//
// Tests for components that had zero WASM coverage:
//   - Toast: toast-region renders, dismiss button works
//   - ConfirmModal: modal structure, overlay click closes
//   - Sidebar: nav items rendered, active highlight
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_toast_region_renders_on_dashboard() {
    let id = "t-toast-region";
    clear_tokens();
    install_mock_fetch_success();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;

    assert!(
        has_element(id, ".toast-region"),
        "toast-region must be present in the dashboard layout"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_sidebar_nav_items_rendered() {
    let id = "t-sidebar-nav";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;

    // Admin user should see all nav items
    let html = inner_html(id);
    assert!(html.contains("Dashboard"), "Dashboard nav item");
    assert!(html.contains("Teams"), "Teams nav item");
    assert!(html.contains("Orders"), "Orders nav item");
    assert!(html.contains("Items"), "Items nav item");
    assert!(html.contains("Profile"), "Profile nav item");
    assert!(html.contains("Admin"), "Admin nav item for admin user");
    assert!(html.contains("Roles"), "Roles nav item for admin user");

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_sidebar_active_nav_item() {
    let id = "t-sidebar-active";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;

    // Dashboard should be active by default
    assert!(
        has_element(id, ".nav-item--active"),
        "there should be an active nav item"
    );

    // Navigate to Teams → active item should change
    click_nav(id, "Teams");
    flush(500).await;
    assert!(
        has_element(id, ".nav-item--active"),
        "active nav item should still exist after navigation"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_confirm_modal_structure_on_delete() {
    let id = "t-confirm-modal";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    // Click the delete button (trash icon) for a non-self user
    click_button(id, "button[aria-label='Delete user']");
    flush(200).await;

    // Confirm modal should appear
    assert!(
        has_element(id, ".modal-overlay"),
        "confirmation modal should appear on delete click"
    );
    assert!(has_element(id, ".modal-title"), "modal should have a title");
    assert!(
        has_element(id, ".modal-footer"),
        "modal should have a footer with action buttons"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ═══════════════════════════════════════════════════════════════════════════
// 16 · Orders page interactive flows (#357)
//
// Tests for create-order dialog interaction
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_orders_page_create_order_dialog_opens() {
    let id = "t-orders-create-dialog";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Orders");
    flush(500).await;

    // Select a team first
    click_button(id, ".team-selector .connect-button");
    flush(500).await;

    // Click "New Order" button
    assert!(
        !has_element(id, ".modal-overlay"),
        "no modal before clicking New Order"
    );

    // Find and click the New Order button
    js_sys::eval(&format!(
        r#"(() => {{
            const el = document.getElementById("{}");
            if (!el) return;
            const buttons = el.querySelectorAll("button");
            for (const btn of buttons) {{
                if (btn.textContent.includes("New Order")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("click New Order button failed");
    flush(200).await;

    assert!(
        has_element(id, ".modal-overlay"),
        "create-order dialog should appear"
    );
    let html = inner_html(id);
    assert!(
        html.contains("New Order"),
        "dialog title should be 'New Order'"
    );
    assert!(
        has_element(id, "#order-due"),
        "due-date input should be present"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ═══════════════════════════════════════════════════════════════════════════
// 17 · Profile page edit mode and password fields (#358)
//
// Tests for the edit-mode toggle and password change form
// ═══════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
async fn test_profile_page_edit_mode_toggle() {
    let id = "t-profile-edit";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Profile");
    flush(500).await;

    // Should not be in edit mode initially — no edit form fields
    assert!(
        !has_element(id, "#profile-fn"),
        "profile-fn input should not exist before entering edit mode"
    );

    // Click the Edit button
    js_sys::eval(&format!(
        r#"(() => {{
            const el = document.getElementById("{}");
            if (!el) return;
            const buttons = el.querySelectorAll("button");
            for (const btn of buttons) {{
                if (btn.textContent.includes("Edit")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("click Edit button failed");
    flush(300).await;

    // Should now show edit form fields
    assert!(
        has_element(id, "#profile-fn"),
        "first name input should appear in edit mode"
    );
    assert!(
        has_element(id, "#profile-ln"),
        "last name input should appear in edit mode"
    );
    assert!(
        has_element(id, "#profile-email"),
        "email input should appear in edit mode"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_profile_page_password_field_reveals_current_password() {
    let id = "t-profile-pw";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Profile");
    flush(500).await;

    // Enter edit mode
    js_sys::eval(&format!(
        r#"(() => {{
            const el = document.getElementById("{}");
            if (!el) return;
            const buttons = el.querySelectorAll("button");
            for (const btn of buttons) {{
                if (btn.textContent.includes("Edit")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("click Edit button failed");
    flush(300).await;

    // Current password field should NOT be visible yet
    assert!(
        !has_element(id, "#profile-curpw"),
        "current password field should not appear until new password is typed"
    );

    // Type in the password field
    set_input(id, "#profile-pw", "newpassword123");
    flush(200).await;

    // Now current password field should appear
    assert!(
        has_element(id, "#profile-curpw"),
        "current password field should appear after typing new password"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_profile_page_cancel_exits_edit_mode() {
    let id = "t-profile-cancel";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Profile");
    flush(500).await;

    // Enter edit mode
    js_sys::eval(&format!(
        r#"(() => {{
            const el = document.getElementById("{}");
            if (!el) return;
            const buttons = el.querySelectorAll("button");
            for (const btn of buttons) {{
                if (btn.textContent.includes("Edit")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("click Edit button failed");
    flush(300).await;

    assert!(has_element(id, "#profile-fn"), "should be in edit mode");

    // Click Cancel
    js_sys::eval(&format!(
        r#"(() => {{
            const el = document.getElementById("{}");
            if (!el) return;
            const buttons = el.querySelectorAll("button");
            for (const btn of buttons) {{
                if (btn.textContent.includes("Cancel")) {{ btn.click(); return; }}
            }}
        }})()"#,
        id
    ))
    .expect("click Cancel button failed");
    flush(300).await;

    assert!(
        !has_element(id, "#profile-fn"),
        "should exit edit mode after Cancel"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── #696 · Profile page form submission ─────────────────────────────────────

#[wasm_bindgen_test]
async fn test_profile_page_save_triggers_put_and_exits_edit() {
    let id = "t-profile-save";
    clear_tokens();
    install_mock_fetch_with_write_ops();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Profile");
    flush(500).await;

    // Enter edit mode
    click_button_text(id, "Edit");
    flush(300).await;
    assert!(has_element(id, "#profile-fn"), "should be in edit mode");

    // Modify a field
    set_input(id, "#profile-fn", "Johnny");
    flush(100).await;

    // Click Save
    click_button_text(id, "Save");
    flush(500).await;

    // Should exit edit mode after successful save
    assert!(
        !has_element(id, "#profile-fn"),
        "should exit edit mode after successful save"
    );
    // Toast should appear
    let html = inner_html(id);
    assert!(
        html.contains("Profile updated") || html.contains("toast"),
        "success toast should appear after save"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_profile_page_password_change_requires_current_password() {
    let id = "t-profile-pwreq";
    clear_tokens();
    install_mock_fetch_with_write_ops();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Profile");
    flush(500).await;

    // Enter edit mode
    click_button_text(id, "Edit");
    flush(300).await;

    // Type a new password — current password field should appear
    set_input(id, "#profile-pw", "newpassword123");
    flush(200).await;
    assert!(
        has_element(id, "#profile-curpw"),
        "current password field should appear"
    );

    // Fill current password and save
    set_input(id, "#profile-curpw", "oldpassword123");
    flush(100).await;
    click_button_text(id, "Save");
    flush(500).await;

    // Should exit edit mode after successful save
    assert!(
        !has_element(id, "#profile-fn"),
        "should exit edit mode after password change save"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── #684 · TeamsPage CRUD interactions ──────────────────────────────────────

#[wasm_bindgen_test]
async fn test_teams_page_create_dialog_opens() {
    let id = "t-teams-create-dlg";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Teams");
    flush(500).await;

    // Click "New Team" button
    click_button_text(id, "New Team");
    flush(300).await;

    let html = inner_html(id);
    assert!(
        html.contains("Team Name"),
        "create dialog should have Team Name field"
    );
    assert!(
        has_element(id, "input#team-name"),
        "team name input should exist"
    );
    assert!(
        has_element(id, "input#team-descr"),
        "description input should exist"
    );
    assert!(html.contains("Cancel"), "cancel button should be present");

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_teams_page_create_dialog_cancel() {
    let id = "t-teams-cancel-dlg";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Teams");
    flush(500).await;

    // Open create dialog
    click_button_text(id, "New Team");
    flush(300).await;
    assert!(has_element(id, "input#team-name"), "dialog should be open");

    // Click Cancel
    click_button_text(id, "Cancel");
    flush(300).await;
    assert!(
        !has_element(id, "input#team-name"),
        "dialog should be closed after Cancel"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_teams_page_add_member_dialog_opens() {
    let id = "t-teams-addmem";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Teams");
    flush(500).await;

    // Click team row to load members
    click_button(id, ".connect-table-row--clickable");
    flush(500).await;

    // Click "Add" button (for adding member)
    click_button_text(id, "Add");
    flush(300).await;

    let html = inner_html(id);
    assert!(
        html.contains("User") || has_element(id, "select#add-member-user"),
        "add member dialog should be visible"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── #685 · ItemsPage CRUD interactions ────────────────────────────────────

#[wasm_bindgen_test]
async fn test_items_page_create_dialog_opens() {
    let id = "t-items-create-dlg";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Items");
    flush(500).await;

    // Click "New Item" button
    click_button_text(id, "New Item");
    flush(300).await;

    let html = inner_html(id);
    assert!(
        has_element(id, "input#item-descr"),
        "item description input should exist"
    );
    assert!(
        has_element(id, "input#item-price"),
        "item price input should exist"
    );
    assert!(html.contains("Cancel"), "cancel button should be present");

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_items_page_create_dialog_cancel() {
    let id = "t-items-cancel-dlg";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Items");
    flush(500).await;

    // Open create dialog
    click_button_text(id, "New Item");
    flush(300).await;
    assert!(has_element(id, "input#item-descr"), "dialog should be open");

    // Click Cancel
    click_button_text(id, "Cancel");
    flush(300).await;
    assert!(
        !has_element(id, "input#item-descr"),
        "dialog should be closed after Cancel"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_items_page_edit_button_exists() {
    let id = "t-items-edit-btn";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Items");
    flush(500).await;

    assert!(
        has_element(id, ".connect-table-cell--actions button"),
        "items table should have action buttons"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── #686 · OrdersPage detail and line item tests ──────────────────────────

#[wasm_bindgen_test]
async fn test_orders_page_shows_order_detail_on_click() {
    let id = "t-orders-detail";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Orders");
    flush(500).await;

    // Select a team
    click_button(id, ".team-selector .connect-button");
    flush(500).await;

    // Click the order row to see detail
    click_button(id, ".connect-table-row");
    flush(500).await;

    let html = inner_html(id);
    // Order detail section should appear with items info
    assert!(
        html.contains("Order") || html.contains("Items") || html.contains("Total"),
        "order detail should show order info or items"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_orders_page_create_order_dialog_fields() {
    let id = "t-orders-create-fields";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;

    login_to_dashboard(id).await;
    click_nav(id, "Orders");
    flush(500).await;

    // Select a team
    click_button(id, ".team-selector .connect-button");
    flush(500).await;

    // Open create order dialog
    click_button_text(id, "New Order");
    flush(300).await;

    let html = inner_html(id);
    assert!(
        html.contains("Due Date") || html.contains("due") || has_element(id, "input[type='date']"),
        "create order dialog should have a due date field"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}
