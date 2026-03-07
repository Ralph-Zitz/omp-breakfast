use crate::api::{HttpMethod, PaginatedResponse, RoleEntry, UserContext, authed_get, authed_request};
use crate::components::card::PageHeader;
use crate::components::icons::{Icon, IconKind};
use crate::components::modal::{ConfirmModal, FormDialog};
use crate::components::toast::{toast_error, toast_success};
use crate::components::{LoadingSpinner, PaginationBar, input_handler, role_tag_class};
use leptos::prelude::*;

#[component]
pub fn RolesPage() -> impl IntoView {
    let user = expect_context::<ReadSignal<Option<UserContext>>>();

    let (roles, set_roles) = signal(Vec::<RoleEntry>::new());
    let (loading, set_loading) = signal(true);
    let (show_create, set_show_create) = signal(false);
    let (delete_target, set_delete_target) = signal(Option::<(String, String)>::None);
    let (edit_target, set_edit_target) = signal(Option::<RoleEntry>::None);
    let (offset, set_offset) = signal(0usize);
    let (total, set_total) = signal(0usize);
    let limit = 50usize;

    let is_admin = Signal::derive(move || user.get().map(|u| u.is_admin).unwrap_or(false));

    // Fetch all roles on mount
    let fetch_roles = move |off: usize| {
        set_loading.set(true);
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/roles?limit={}&offset={}", limit, off);
            if let Some(resp) = authed_get(&url).await && resp.ok() {
                    match resp.json::<PaginatedResponse<RoleEntry>>().await {
                        Ok(data) => { set_total.set(data.total as usize); set_roles.set(data.items); }
                        Err(e) => web_sys::console::warn_1(&format!("roles JSON parse error: {e}").into()),
                    }
            }
            set_loading.set(false);
        });
    };
    fetch_roles(0);

    let do_create_role = move |title: String| {
        let body = serde_json::json!({ "title": title });
        leptos::task::spawn_local_scoped(async move {
            let resp = authed_request(HttpMethod::Post, "/api/v1.0/roles", Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    match r.json::<RoleEntry>().await {
                        Ok(role) => { set_roles.update(|list| list.push(role)); toast_success("Role created"); }
                        Err(e) => web_sys::console::warn_1(&format!("role create JSON parse error: {e}").into()),
                    }
                }
                _ => toast_error("Failed to create role"),
            }
            set_show_create.set(false);
        });
    };

    let do_update_role = move |role_id: String, title: String| {
        let body = serde_json::json!({ "title": title });
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/roles/{}", role_id);
            let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    match r.json::<RoleEntry>().await {
                        Ok(updated) => {
                            set_roles.update(|list| {
                                if let Some(r) = list.iter_mut().find(|r| r.role_id == updated.role_id) {
                                    *r = updated;
                                }
                            });
                            toast_success("Role updated");
                        }
                        Err(e) => web_sys::console::warn_1(&format!("role update JSON parse error: {e}").into()),
                    }
                }
                _ => toast_error("Failed to update role"),
            }
            set_edit_target.set(None);
        });
    };

    let do_delete_role = move |role_id: String| {
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/roles/{}", role_id);
            let resp = authed_request(HttpMethod::Delete, &url, None).await;
            match resp {
                Some(r) if r.ok() => {
                    set_roles.update(|list| list.retain(|r| r.role_id != role_id));
                    toast_success("Role deleted");
                }
                _ => toast_error("Failed to delete role"),
            }
            set_delete_target.set(None);
        });
    };

    view! {
        <div class="roles-page">
            <PageHeader title="Roles">
                {move || {
                    if is_admin.get() {
                        view! {
                            <button
                                class="connect-button connect-button--accent connect-button--small"
                                on:click=move |_| set_show_create.set(true)
                            >
                                <span class="connect-button__content">
                                    <span class="connect-button__icon">
                                        <Icon kind=IconKind::CirclePlus size=16 />
                                    </span>
                                    <span class="connect-button__label">"New Role"</span>
                                </span>
                            </button>
                        }.into_any()
                    } else {
                        view! { <span /> }.into_any()
                    }
                }}
            </PageHeader>

            {move || {
                if loading.get() {
                    return view! { <LoadingSpinner /> }.into_any();
                }

                let role_list = roles.get();
                if role_list.is_empty() {
                    return view! {
                        <div class="empty-state">
                            <Icon kind=IconKind::ShieldCheck size=48 />
                            <p>"No roles defined."</p>
                        </div>
                    }.into_any();
                }

                view! {
                    <div class="card">
                        <table class="connect-table connect-table--medium">
                            <thead class="connect-table-header">
                                <tr>
                                    <th class="connect-table-header-cell">"Role"</th>
                                    {move || is_admin.get().then(|| view! {
                                        <th class="connect-table-header-cell connect-table-header-cell--actions">"Actions"</th>
                                    })}
                                </tr>
                            </thead>
                            <tbody class="connect-table-body">
                                {role_list.into_iter().map(|role| {
                                    let rid = role.role_id.clone();
                                    let title = role.title.clone();
                                    let title_del = role.title.clone();
                                    let cls = role_tag_class(&title);
                                    let role = role.clone();

                                    view! {
                                        <tr class="connect-table-row">
                                            <td class="connect-table-cell">
                                                <span class=cls>{title}</span>
                                            </td>
                                            {move || is_admin.get().then(|| {
                                                let rid = rid.clone();
                                                let title_del = title_del.clone();
                                                let role_for_edit = role.clone();
                                                view! {
                                                    <td class="connect-table-cell connect-table-cell--actions">
                                                        <button
                                                            aria-label="Edit role"
                                                            class="connect-button connect-button--neutral connect-button--outline connect-button--small"
                                                            on:click=move |_| set_edit_target.set(Some(role_for_edit.clone()))
                                                        >
                                                            <span class="connect-button__content">
                                                                <span class="connect-button__icon">
                                                                    <Icon kind=IconKind::PenToSquare size=14 />
                                                                </span>
                                                            </span>
                                                        </button>
                                                        <button
                                                            aria-label="Delete role"
                                                            class="connect-button connect-button--negative connect-button--outline connect-button--small"
                                                            on:click=move |_| set_delete_target.set(Some((rid.clone(), title_del.clone())))
                                                        >
                                                            <span class="connect-button__content">
                                                                <span class="connect-button__icon">
                                                                    <Icon kind=IconKind::Trash size=14 />
                                                                </span>
                                                            </span>
                                                        </button>
                                                    </td>
                                                }
                                            })}
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                        <PaginationBar
                            offset=offset
                            limit=limit
                            total=total
                            on_prev=move |off| { set_offset.set(off); fetch_roles(off); }
                            on_next=move |off| { set_offset.set(off); fetch_roles(off); }
                        />
                    </div>
                }.into_any()
            }}

            <CreateRoleDialog
                open=show_create.into()
                on_create=do_create_role
                on_cancel=move || set_show_create.set(false)
            />

            // Edit role dialog
            {move || {
                let target = edit_target.get();
                let open = Signal::derive(move || edit_target.get().is_some());
                if let Some(role) = target {
                    view! {
                        <EditRoleDialog
                            open=open
                            role=role
                            on_save=do_update_role
                            on_cancel=move || set_edit_target.set(None)
                        />
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }
            }}

            {move || {
                let open = Signal::derive(move || delete_target.get().is_some());
                let (rid, rname) = delete_target.get().unwrap_or_default();
                let rid_clone = rid.clone();
                view! {
                    <ConfirmModal
                        open=open
                        title="Delete Role".to_string()
                        message=format!("Are you sure you want to delete the \"{}\" role?", rname)
                        confirm_label="Delete"
                        destructive=true
                        on_confirm=move || do_delete_role(rid_clone.clone())
                        on_cancel=move || set_delete_target.set(None)
                    />
                }
            }}
        </div>
    }
}

#[component]
fn CreateRoleDialog(
    open: Signal<bool>,
    on_create: impl Fn(String) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (title, set_title) = signal(String::new());

    let disabled = Signal::derive(move || title.get().trim().is_empty());
    let on_submit = {
        let on_create = on_create.clone();
        move || {
            on_create(title.get());
            set_title.set(String::new());
        }
    };
    let on_cancel_reset = move || {
        set_title.set(String::new());
        on_cancel();
    };

    view! {
        <FormDialog
            open=open
            title="New Role"
            disabled=disabled
            on_submit=on_submit
            on_cancel=on_cancel_reset
        >
            <div class="connect-text-field">
                <div class="connect-label">
                    <label class="connect-label__text" for="role-title">"Role Title"</label>
                </div>
                <div class="connect-text-field__input-wrapper">
                    <input
                        class="connect-text-field__input"
                        id="role-title"
                        type="text"
                        placeholder="e.g., Coordinator"
                        prop:value=move || title.get()
                        on:input=input_handler(set_title)
                    />
                </div>
            </div>
        </FormDialog>
    }
}

#[component]
fn EditRoleDialog(
    open: Signal<bool>,
    role: RoleEntry,
    on_save: impl Fn(String, String) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (title, set_title) = signal(role.title.clone());
    let role_id = role.role_id.clone();

    let disabled = Signal::derive(move || title.get().trim().is_empty());
    let on_submit = {
        let on_save = on_save.clone();
        let rid = role_id.clone();
        move || on_save(rid.clone(), title.get())
    };

    view! {
        <FormDialog
            open=open
            title="Edit Role"
            submit_label="Save"
            disabled=disabled
            on_submit=on_submit
            on_cancel=on_cancel
        >
            <div class="connect-text-field">
                <div class="connect-label">
                    <label class="connect-label__text" for="edit-role-title">"Role Title"</label>
                </div>
                <div class="connect-text-field__input-wrapper">
                    <input
                        class="connect-text-field__input"
                        id="edit-role-title"
                        type="text"
                        prop:value=move || title.get()
                        on:input=input_handler(set_title)
                    />
                </div>
            </div>
        </FormDialog>
    }
}
