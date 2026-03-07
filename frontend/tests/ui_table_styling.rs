//! Table header classes, actions column modifiers, and inline width tests.

#[path = "ui_helpers.rs"]
mod ui_helpers;

use ui_helpers::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ── 13a · Header-cell class coverage ────────────────────────────────────────

#[wasm_bindgen_test]
async fn test_admin_table_th_have_connect_header_class() {
    let id = "t-align-admin-th";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    assert!(
        has_element(id, "table.connect-table"),
        "admin table must be present"
    );
    assert!(
        all_th_have_connect_class(id),
        "every <th> in the admin table must carry .connect-table-header-cell \
         so the global text-align:left rule is applied"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_items_table_th_have_connect_header_class() {
    let id = "t-align-items-th";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Items");
    flush(500).await;

    assert!(
        has_element(id, "table.connect-table"),
        "items table must be present"
    );
    assert!(
        all_th_have_connect_class(id),
        "every <th> in the items table must carry .connect-table-header-cell"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_roles_table_th_have_connect_header_class() {
    let id = "t-align-roles-th";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Roles");
    flush(500).await;

    assert!(
        has_element(id, "table.connect-table"),
        "roles table must be present"
    );
    assert!(
        all_th_have_connect_class(id),
        "every <th> in the roles table must carry .connect-table-header-cell"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_teams_table_th_have_connect_header_class() {
    let id = "t-align-teams-th";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Teams");
    flush(500).await;

    assert!(
        has_element(id, "table.connect-table"),
        "teams table must be present"
    );
    assert!(
        all_th_have_connect_class(id),
        "every <th> in the teams table must carry .connect-table-header-cell"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 13b · Actions-column modifier class coverage ────────────────────────────

#[wasm_bindgen_test]
async fn test_admin_actions_column_has_actions_modifier() {
    let id = "t-align-admin-actions";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    assert!(
        has_element(id, "th.connect-table-header-cell--actions"),
        "admin actions <th> must carry .connect-table-header-cell--actions \
         so width:auto and gap are applied to prevent button clipping"
    );
    assert!(
        has_element(id, "td.connect-table-cell--actions"),
        "admin actions <td> must carry .connect-table-cell--actions"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_items_actions_column_has_actions_modifier() {
    let id = "t-align-items-actions";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Items");
    flush(500).await;

    assert!(
        has_element(id, "th.connect-table-header-cell--actions"),
        "items actions <th> must carry .connect-table-header-cell--actions"
    );
    assert!(
        has_element(id, "td.connect-table-cell--actions"),
        "items actions <td> must carry .connect-table-cell--actions"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_roles_actions_column_has_actions_modifier() {
    let id = "t-align-roles-actions";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Roles");
    flush(500).await;

    assert!(
        has_element(id, "th.connect-table-header-cell--actions"),
        "roles actions <th> must carry .connect-table-header-cell--actions"
    );
    assert!(
        has_element(id, "td.connect-table-cell--actions"),
        "roles actions <td> must carry .connect-table-cell--actions"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 13c · No narrow inline width on actions cells (regression guard) ─────────

#[wasm_bindgen_test]
async fn test_admin_actions_cell_has_no_narrow_inline_width() {
    let id = "t-align-admin-width";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(500).await;

    assert!(
        no_narrow_inline_width(id, "td.connect-table-cell--actions"),
        "admin actions cells must not carry a narrow inline width (≤ 100 px) \
         — this would clip buttons and break the row-separator line"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_items_actions_cell_has_no_narrow_inline_width() {
    let id = "t-align-items-width";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Items");
    flush(500).await;

    assert!(
        no_narrow_inline_width(id, "td.connect-table-cell--actions"),
        "items actions cells must not carry a narrow inline width (≤ 100 px)"
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

// ── 13d · Actions cells contain multiple sibling buttons ────────────────────

#[wasm_bindgen_test]
async fn test_admin_actions_cell_contains_multiple_buttons() {
    let id = "t-align-admin-btns";
    clear_tokens();
    install_mock_fetch_full_with_second_user();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Admin");
    flush(1000).await;

    // Admin row renders Edit + Reset-password + Delete (≥ 3 buttons in one cell).
    // Having ≥ 2 confirms multiple buttons coexist in the same actions cell,
    // which is the scenario that required the width:auto + gap fix.
    //
    // The first actions cell may be the current user's row (empty / self).
    // Sum buttons across ALL actions cells instead of checking only the first.
    let total_action_buttons: u32 = js_sys::eval(&format!(
        r#"(() => {{
            const cells = document.getElementById("{id}").querySelectorAll("td.connect-table-cell--actions");
            let total = 0;
            for (const cell of cells) {{ total += cell.querySelectorAll("button").length; }}
            return total;
        }})()"#,
        id = id,
    ))
    .ok()
    .and_then(|v| v.as_f64())
    .map(|n| n as u32)
    .unwrap_or(0);
    assert!(
        total_action_buttons >= 2,
        "admin actions cells must contain at least 2 action buttons total, found {}",
        total_action_buttons
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

#[wasm_bindgen_test]
async fn test_items_actions_cell_contains_multiple_buttons() {
    let id = "t-align-items-btns";
    clear_tokens();
    install_mock_fetch_full();
    let container = create_test_container(id);
    let _handle = leptos::mount::mount_to(container.clone(), app::App);
    flush(100).await;
    login_to_dashboard(id).await;
    click_nav(id, "Items");
    flush(500).await;

    let count = button_count_in(id, "td.connect-table-cell--actions");
    assert!(
        count >= 2,
        "items actions cell must contain at least 2 action buttons (edit + delete), found {}",
        count
    );

    remove_test_container(id);
    clear_tokens();
    restore_fetch();
}

