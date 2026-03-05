use crate::api::{HttpMethod, PaginatedResponse, UserContext, UserEntry, authed_get, authed_request};
use crate::components::card::PageHeader;
use crate::components::icons::{Icon, IconKind};
use crate::components::modal::ConfirmModal;
use crate::components::toast::{toast_error, toast_success};
use crate::components::{LoadingSpinner, PaginationBar};
use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

#[component]
pub fn AdminPage() -> impl IntoView {
    let user = expect_context::<ReadSignal<Option<UserContext>>>();

    let (users, set_users) = signal(Vec::<UserEntry>::new());
    let (loading, set_loading) = signal(true);
    let (show_create, set_show_create) = signal(false);
    let (delete_target, set_delete_target) = signal(Option::<(String, String)>::None);
    let (edit_target, set_edit_target) = signal(Option::<UserEntry>::None);
    let (offset, set_offset) = signal(0usize);
    let (total, set_total) = signal(0usize);
    let limit = 50usize;

    let is_admin = Signal::derive(move || user.get().map(|u| u.is_admin).unwrap_or(false));

    // Fetch all users on mount
    let fetch_users = move |off: usize| {
        set_loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/users?limit={}&offset={}", limit, off);
            if let Some(resp) = authed_get(&url).await {
                if resp.ok() {
                    match resp.json::<PaginatedResponse<UserEntry>>().await {
                        Ok(data) => { set_total.set(data.total as usize); set_users.set(data.items); }
                        Err(e) => web_sys::console::warn_1(&format!("users JSON parse error: {e}").into()),
                    }
                }
            }
            set_loading.set(false);
        });
    };
    fetch_users(0);

    let do_update_user = move |user_id: String, fn_: String, ln: String, em: String| {
        let body = serde_json::json!({
            "firstname": fn_,
            "lastname": ln,
            "email": em,
        });
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/users/{}", user_id);
            let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    match r.json::<UserEntry>().await {
                        Ok(updated) => {
                            set_users.update(|list| {
                                if let Some(u) = list.iter_mut().find(|u| u.user_id == updated.user_id) {
                                    *u = updated;
                                }
                            });
                            toast_success("User updated");
                        }
                        Err(e) => web_sys::console::warn_1(&format!("user update JSON parse error: {e}").into()),
                    }
                }
                _ => toast_error("Failed to update user"),
            }
            set_edit_target.set(None);
        });
    };

    let do_create_user = move |fn_: String, ln: String, em: String, pw: String| {
        let body = serde_json::json!({
            "firstname": fn_,
            "lastname": ln,
            "email": em,
            "password": pw,
        });
        wasm_bindgen_futures::spawn_local(async move {
            let resp = authed_request(HttpMethod::Post, "/api/v1.0/users", Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    match r.json::<UserEntry>().await {
                        Ok(u) => { set_users.update(|list| list.push(u)); toast_success("User created"); }
                        Err(e) => web_sys::console::warn_1(&format!("user create JSON parse error: {e}").into()),
                    }
                }
                _ => toast_error("Failed to create user"),
            }
            set_show_create.set(false);
        });
    };

    let do_delete_user = move |user_id: String| {
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/users/{}", user_id);
            let resp = authed_request(HttpMethod::Delete, &url, None).await;
            match resp {
                Some(r) if r.ok() => {
                    set_users.update(|list| list.retain(|u| u.user_id != user_id));
                    toast_success("User deleted");
                }
                _ => toast_error("Failed to delete user"),
            }
            set_delete_target.set(None);
        });
    };

    view! {
        <div class="admin-page">
            <PageHeader title="User Management">
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
                                    <span class="connect-button__label">"New User"</span>
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

                let user_list = users.get();
                if user_list.is_empty() {
                    return view! {
                        <div class="empty-state">
                            <Icon kind=IconKind::Users size=48 />
                            <p>"No users found."</p>
                        </div>
                    }.into_any();
                }

                view! {
                    <div class="card">
                        <table class="connect-table connect-table--medium">
                            <thead class="connect-table-header">
                                <tr>
                                    <th class="connect-table-header-cell">"Name"</th>
                                    <th class="connect-table-header-cell">"Email"</th>
                                    {move || is_admin.get().then(|| view! {
                                        <th class="connect-table-header-cell connect-table-header-cell--actions">"Actions"</th>
                                    })}
                                </tr>
                            </thead>
                            <tbody class="connect-table-body">
                                {user_list.into_iter().map(|u| {
                                    let uid = u.user_id.clone();
                                    let name = format!("{} {}", u.firstname, u.lastname);
                                    let name_del = name.clone();
                                    let email = u.email.clone();
                                    let uid_for_self = uid.clone();
                                    let is_self = move || user.get().map(|ctx| ctx.user_id == uid_for_self).unwrap_or(false);
                                    let user_for_edit = u.clone();

                                    view! {
                                        <tr class="connect-table-row">
                                            <td class="connect-table-cell">{name}</td>
                                            <td class="connect-table-cell">{email}</td>
                                            {move || (is_admin.get() && !is_self()).then(|| {
                                                let uid = uid.clone();
                                                let name_del = name_del.clone();
                                                let ufe = user_for_edit.clone();
                                                view! {
                                                    <td class="connect-table-cell connect-table-cell--actions">
                                                        <button
                                                            aria-label="Edit user"
                                                            class="connect-button connect-button--neutral connect-button--outline connect-button--small"
                                                            on:click=move |_| set_edit_target.set(Some(ufe.clone()))
                                                        >
                                                            <span class="connect-button__content">
                                                                <span class="connect-button__icon">
                                                                    <Icon kind=IconKind::PenToSquare size=14 />
                                                                </span>
                                                            </span>
                                                        </button>
                                                        <button
                                                            aria-label="Delete user"
                                                            class="connect-button connect-button--negative connect-button--outline connect-button--small"
                                                            on:click=move |_| set_delete_target.set(Some((uid.clone(), name_del.clone())))
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
                            on_prev=move |off| { set_offset.set(off); fetch_users(off); }
                            on_next=move |off| { set_offset.set(off); fetch_users(off); }
                        />
                    </div>
                }.into_any()
            }}

            <CreateUserDialog
                open=show_create.into()
                on_create=do_create_user
                on_cancel=move || set_show_create.set(false)
            />

            // Edit user dialog
            {move || {
                let target = edit_target.get();
                let open = Signal::derive(move || edit_target.get().is_some());
                if let Some(u) = target {
                    view! {
                        <EditUserDialog
                            open=open
                            user=u
                            on_save=do_update_user
                            on_cancel=move || set_edit_target.set(None)
                        />
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }
            }}

            {move || {
                let open = Signal::derive(move || delete_target.get().is_some());
                let (uid, uname) = delete_target.get().unwrap_or_default();
                let uid_clone = uid.clone();
                view! {
                    <ConfirmModal
                        open=open
                        title="Delete User".to_string()
                        message=format!("Are you sure you want to delete \"{}\"? This action cannot be undone.", uname)
                        confirm_label="Delete"
                        destructive=true
                        on_confirm=move || do_delete_user(uid_clone.clone())
                        on_cancel=move || set_delete_target.set(None)
                    />
                }
            }}
        </div>
    }
}

#[component]
fn CreateUserDialog(
    open: Signal<bool>,
    on_create: impl Fn(String, String, String, String) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (firstname, set_firstname) = signal(String::new());
    let (lastname, set_lastname) = signal(String::new());
    let (email, set_email) = signal(String::new());
    let (password, set_password) = signal(String::new());

    let reset = move || {
        set_firstname.set(String::new());
        set_lastname.set(String::new());
        set_email.set(String::new());
        set_password.set(String::new());
    };

    let form_valid = Signal::derive(move || {
        !firstname.get().trim().is_empty()
            && !lastname.get().trim().is_empty()
            && !email.get().trim().is_empty()
            && password.get().len() >= 8
    });

    view! {
        {move || {
            if !open.get() {
                return view! { <div class="modal-hidden" /> }.into_any();
            }

            let on_create = on_create.clone();
            let reset_bd = reset.clone();
            let reset_b = reset.clone();
            let on_cancel_bd = on_cancel.clone();
            let on_cancel_b = on_cancel.clone();

            view! {
                <div class="modal-overlay" on:click=move |_| { reset_bd(); on_cancel_bd(); }>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h2 class="modal-title">"New User"</h2>
                        </div>
                        <div class="modal-body">
                            <div class="connect-text-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="user-fn">"First Name"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="user-fn"
                                        type="text"
                                        prop:value=move || firstname.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return };
                                            set_firstname.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                        }
                                    />
                                </div>
                            </div>
                            <div class="connect-text-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="user-ln">"Last Name"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="user-ln"
                                        type="text"
                                        prop:value=move || lastname.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return };
                                            set_lastname.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                        }
                                    />
                                </div>
                            </div>
                            <div class="connect-text-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="user-email">"Email"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="user-email"
                                        type="email"
                                        prop:value=move || email.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return };
                                            set_email.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                        }
                                    />
                                </div>
                            </div>
                            <div class="connect-text-field">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="user-pw">"Password (min 8 characters)"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="user-pw"
                                        type="password"
                                        prop:value=move || password.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return };
                                            set_password.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
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
                                    let reset = reset_b.clone();
                                    move |_| { reset(); cancel(); }
                                }
                            >
                                <span class="connect-button__content">
                                    <span class="connect-button__label">"Cancel"</span>
                                </span>
                            </button>
                            <button
                                class="connect-button connect-button--accent connect-button--medium"
                                disabled=move || !form_valid.get()
                                on:click={
                                    let create = on_create.clone();
                                    move |_| {
                                        create(firstname.get(), lastname.get(), email.get(), password.get());
                                        set_firstname.set(String::new());
                                        set_lastname.set(String::new());
                                        set_email.set(String::new());
                                        set_password.set(String::new());
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

#[component]
fn EditUserDialog(
    open: Signal<bool>,
    user: UserEntry,
    on_save: impl Fn(String, String, String, String) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (firstname, set_firstname) = signal(user.firstname.clone());
    let (lastname, set_lastname) = signal(user.lastname.clone());
    let (email, set_email) = signal(user.email.clone());
    let user_id = user.user_id.clone();

    let form_valid = Signal::derive(move || {
        let em = email.get();
        !firstname.get().trim().is_empty()
            && !lastname.get().trim().is_empty()
            && em.contains('@')
            && em.split('@').nth(1).map(|d| d.contains('.')).unwrap_or(false)
    });

    view! {
        {move || {
            if !open.get() {
                return view! { <div class="modal-hidden" /> }.into_any();
            }

            let on_save = on_save.clone();
            let on_cancel_bd = on_cancel.clone();
            let on_cancel_b = on_cancel.clone();
            let uid = user_id.clone();

            view! {
                <div class="modal-overlay" on:click=move |_| on_cancel_bd()>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h2 class="modal-title">"Edit User"</h2>
                        </div>
                        <div class="modal-body">
                            <div class="connect-text-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="edit-user-fn">"First Name"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="edit-user-fn"
                                        type="text"
                                        prop:value=move || firstname.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return };
                                            set_firstname.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                        }
                                    />
                                </div>
                            </div>
                            <div class="connect-text-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="edit-user-ln">"Last Name"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="edit-user-ln"
                                        type="text"
                                        prop:value=move || lastname.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return };
                                            set_lastname.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                        }
                                    />
                                </div>
                            </div>
                            <div class="connect-text-field">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="edit-user-email">"Email"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="edit-user-email"
                                        type="email"
                                        prop:value=move || email.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return };
                                            set_email.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
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
                                disabled=move || !form_valid.get()
                                on:click={
                                    let save = on_save.clone();
                                    let uid = uid.clone();
                                    move |_| save(uid.clone(), firstname.get(), lastname.get(), email.get())
                                }
                            >
                                <span class="connect-button__content">
                                    <span class="connect-button__label">"Save"</span>
                                </span>
                            </button>
                        </div>
                    </div>
                </div>
            }.into_any()
        }}
    }
}
