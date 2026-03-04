use crate::api::{HttpMethod, RoleEntry, UserContext, authed_get, authed_request};
use crate::components::card::PageHeader;
use crate::components::icons::{Icon, IconKind};
use crate::components::modal::ConfirmModal;
use crate::components::toast::{toast_error, toast_success};
use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

#[component]
pub fn RolesPage() -> impl IntoView {
    let user = expect_context::<ReadSignal<Option<UserContext>>>();

    let (roles, set_roles) = signal(Vec::<RoleEntry>::new());
    let (loading, set_loading) = signal(true);
    let (show_create, set_show_create) = signal(false);
    let (delete_target, set_delete_target) = signal(Option::<(String, String)>::None);

    let is_admin = Signal::derive(move || user.get().map(|u| u.is_admin).unwrap_or(false));

    // Fetch all roles on mount
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(resp) = authed_get("/api/v1.0/roles").await {
            if resp.ok() {
                if let Ok(data) = resp.json::<Vec<RoleEntry>>().await {
                    set_roles.set(data);
                }
            }
        }
        set_loading.set(false);
    });

    let do_create_role = move |title: String| {
        let body = serde_json::json!({ "title": title });
        wasm_bindgen_futures::spawn_local(async move {
            let resp = authed_request(HttpMethod::Post, "/api/v1.0/roles", Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    if let Ok(role) = r.json::<RoleEntry>().await {
                        set_roles.update(|list| list.push(role));
                        toast_success("Role created");
                    }
                }
                _ => toast_error("Failed to create role"),
            }
            set_show_create.set(false);
        });
    };

    let do_delete_role = move |role_id: String| {
        wasm_bindgen_futures::spawn_local(async move {
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

                                    view! {
                                        <tr class="connect-table-row">
                                            <td class="connect-table-cell">
                                                <span class=format!("connect-tag connect-tag--small {}", cls)>{title}</span>
                                            </td>
                                            {move || is_admin.get().then(|| {
                                                let rid = rid.clone();
                                                let title_del = title_del.clone();
                                                view! {
                                                    <td class="connect-table-cell connect-table-cell--actions">
                                                        <button
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
                    </div>
                }.into_any()
            }}

            <CreateRoleDialog
                open=show_create
                on_create=do_create_role
                on_cancel=move || set_show_create.set(false)
            />

            {move || {
                let target = delete_target.get();
                let (del_open, _) = signal(target.is_some());
                let (rid, rname) = target.unwrap_or_default();
                let rid_clone = rid.clone();
                view! {
                    <ConfirmModal
                        open=del_open
                        title=format!("Delete Role")
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
    open: ReadSignal<bool>,
    on_create: impl Fn(String) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (title, set_title) = signal(String::new());

    view! {
        {move || {
            if !open.get() {
                return view! { <div class="modal-hidden" /> }.into_any();
            }

            let on_create = on_create.clone();
            let on_cancel_bd = on_cancel.clone();
            let on_cancel_b = on_cancel.clone();

            view! {
                <div class="modal-overlay" on:click=move |_| on_cancel_bd()>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h2 class="modal-title">"New Role"</h2>
                        </div>
                        <div class="modal-body">
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
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return };
                                            set_title.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                        }
                                    />
                                </div>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button
                                class="connect-button connect-button--neutral connect-button--outline connect-button--medium"
                                on:click={
                                    let cancel = on_cancel_b.clone();
                                    move |_| cancel()
                                }
                            >
                                <span class="connect-button__content">
                                    <span class="connect-button__label">"Cancel"</span>
                                </span>
                            </button>
                            <button
                                class="connect-button connect-button--accent connect-button--medium"
                                disabled=move || title.get().trim().is_empty()
                                on:click={
                                    let create = on_create.clone();
                                    move |_| {
                                        create(title.get());
                                        set_title.set(String::new());
                                    }
                                }
                            >
                                <span class="connect-button__content">
                                    <span class="connect-button__label">"Create"</span>
                                </span>
                            </button>
                        </div>
                    </div>
                </div>
            }.into_any()
        }}
    }
}

fn role_tag_class(role: &str) -> &'static str {
    match role {
        "Admin" => "connect-tag--negative-emphasis",
        "Team Admin" => "connect-tag--warning-default",
        "Member" => "connect-tag--primary-default",
        _ => "connect-tag--neutral-default",
    }
}

#[component]
fn LoadingSpinner() -> impl IntoView {
    view! {
        <div class="loading-spinner">
            <div class="connect-progress-circle connect-progress-circle--indeterminate">
                <svg class="connect-progress-circle__bar" viewBox="0 0 40 40">
                    <circle class="connect-progress-circle__background" cx="20" cy="20" r="17" />
                    <circle class="connect-progress-circle__indicator" cx="20" cy="20" r="17" />
                </svg>
            </div>
        </div>
    }
}
