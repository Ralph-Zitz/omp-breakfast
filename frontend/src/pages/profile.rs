use crate::api::{AvatarListEntry, HttpMethod, UserContext, UserEntry, authed_get, authed_request};
use crate::components::card::PageHeader;
use crate::components::icons::{Icon, IconKind};
use crate::components::input_handler;
use crate::components::role_tag_class;
use crate::components::toast::{toast_error, toast_success};
use leptos::prelude::*;

/// Build the avatar image URL for a given avatar_id.
fn avatar_url(id: &str) -> String {
    format!("/api/v1.0/avatars/{}", id)
}

#[component]
pub fn ProfilePage() -> impl IntoView {
    let user = expect_context::<ReadSignal<Option<UserContext>>>();
    let set_user = expect_context::<WriteSignal<Option<UserContext>>>();

    let (editing, set_editing) = signal(false);
    let (firstname, set_firstname) = signal(String::new());
    let (lastname, set_lastname) = signal(String::new());
    let (email, set_email) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (current_password, set_current_password) = signal(String::new());
    let (saving, set_saving) = signal(false);

    // Avatar picker state
    let (picker_open, set_picker_open) = signal(false);
    let (avatars, set_avatars) = signal(Vec::<AvatarListEntry>::new());
    let (avatar_loading, set_avatar_loading) = signal(false);

    // Open avatar picker: fetch available avatars and show dialog
    let open_picker = move || {
        set_avatar_loading.set(true);
        set_picker_open.set(true);
        leptos::task::spawn_local_scoped(async move {
            match authed_get("/api/v1.0/avatars").await {
                Some(resp) if resp.ok() => {
                    if let Ok(list) = resp.json::<Vec<AvatarListEntry>>().await {
                        set_avatars.set(list);
                    }
                }
                _ => toast_error("Failed to load avatars"),
            }
            set_avatar_loading.set(false);
        });
    };

    let select_avatar = move |avatar_id: String| {
        let u = match user.get() {
            Some(u) => u,
            None => return,
        };
        let user_id = u.user_id.clone();
        leptos::task::spawn_local_scoped(async move {
            let body = serde_json::json!({ "avatar_id": avatar_id });
            let url = format!("/api/v1.0/users/{}/avatar", user_id);
            match authed_request(HttpMethod::Put, &url, Some(&body)).await {
                Some(r) if r.ok() => {
                    if let Ok(updated) = r.json::<UserEntry>().await {
                        let teams = crate::api::fetch_user_teams(&user_id).await;
                        set_user.set(Some(UserContext::from_entry(updated, teams)));
                    }
                    toast_success("Avatar updated");
                    set_picker_open.set(false);
                }
                _ => toast_error("Failed to set avatar"),
            }
        });
    };

    let remove_avatar = move || {
        let u = match user.get() {
            Some(u) => u,
            None => return,
        };
        let user_id = u.user_id.clone();
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/users/{}/avatar", user_id);
            match authed_request(HttpMethod::Delete, &url, None).await {
                Some(r) if r.ok() => {
                    if let Ok(updated) = r.json::<UserEntry>().await {
                        let teams = crate::api::fetch_user_teams(&user_id).await;
                        set_user.set(Some(UserContext::from_entry(updated, teams)));
                    }
                    toast_success("Avatar removed");
                    set_picker_open.set(false);
                }
                _ => toast_error("Failed to remove avatar"),
            }
        });
    };

    // Populate form fields from user context
    let populate_fields = move || {
        if let Some(u) = user.get() {
            set_firstname.set(u.firstname.clone());
            set_lastname.set(u.lastname.clone());
            set_email.set(u.email.clone());
            set_password.set(String::new());
            set_current_password.set(String::new());
        }
    };

    let start_edit = move || {
        populate_fields();
        set_editing.set(true);
    };

    let cancel_edit = move || {
        set_editing.set(false);
        set_password.set(String::new());
        set_current_password.set(String::new());
    };

    let save_profile = move || {
        let u = match user.get() {
            Some(u) => u,
            None => return,
        };

        set_saving.set(true);
        let fn_val = firstname.get();
        let ln_val = lastname.get();
        let em_val = email.get();
        let pw_val = password.get();
        let cur_pw_val = current_password.get();
        let user_id = u.user_id.clone();

        let mut body = serde_json::json!({
            "firstname": fn_val,
            "lastname": ln_val,
            "email": em_val,
        });

        // Only include password if user typed one
        if !pw_val.is_empty() {
            body["password"] = serde_json::Value::String(pw_val);
            body["current_password"] = serde_json::Value::String(cur_pw_val);
        }

        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/users/{}", user_id);
            let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    if let Ok(updated) = r.json::<UserEntry>().await {
                        let teams = crate::api::fetch_user_teams(&user_id).await;
                        set_user.set(Some(UserContext::from_entry(updated, teams)));
                    }
                    toast_success("Profile updated");
                    set_editing.set(false);
                }
                _ => toast_error("Failed to update profile"),
            }
            set_saving.set(false);
        });
    };

    view! {
        <div class="profile-page">
            <PageHeader title="Profile">
                {move || {
                    if !editing.get() {
                        view! {
                            <button
                                class="connect-button connect-button--accent connect-button--small"
                                on:click=move |_| start_edit()
                            >
                                <span class="connect-button__content">
                                    <span class="connect-button__icon">
                                        <Icon kind=IconKind::PenToSquare size=16 />
                                    </span>
                                    <span class="connect-button__label">"Edit"</span>
                                </span>
                            </button>
                        }.into_any()
                    } else {
                        view! { <span /> }.into_any()
                    }
                }}
            </PageHeader>

            {move || {
                let u = match user.get() {
                    Some(u) => u,
                    None => return view! {
                        <div class="empty-state">
                            <p>"Loading profile..."</p>
                        </div>
                    }.into_any(),
                };

                if editing.get() {
                    view! {
                        <div class="card" style="max-width: 480px;">
                            <div class="section-title">"Edit Profile"</div>

                            <div class="connect-text-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="profile-fn">"First Name"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="profile-fn"
                                        type="text"
                                        maxlength=50
                                        prop:value=move || firstname.get()
                                        on:input=input_handler(set_firstname)
                                    />
                                </div>
                            </div>

                            <div class="connect-text-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="profile-ln">"Last Name"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="profile-ln"
                                        type="text"
                                        maxlength=50
                                        prop:value=move || lastname.get()
                                        on:input=input_handler(set_lastname)
                                    />
                                </div>
                            </div>

                            <div class="connect-text-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="profile-email">"Email"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="profile-email"
                                        type="email"
                                        maxlength=255
                                        prop:value=move || email.get()
                                        on:input=input_handler(set_email)
                                    />
                                </div>
                            </div>

                            <div class="connect-text-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="profile-pw">"New Password (leave blank to keep current)"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="profile-pw"
                                        type="password"
                                        maxlength=128
                                        autocomplete="new-password"
                                        placeholder="••••••••"
                                        prop:value=move || password.get()
                                        on:input=input_handler(set_password)
                                    />
                                </div>
                            </div>

                            {move || {
                                let pw = password.get();
                                (!pw.is_empty()).then(|| view! {
                                    <div class="connect-text-field" style="margin-bottom: var(--ds-layout-spacing-300, 16px);">
                                        <div class="connect-label">
                                            <label class="connect-label__text" for="profile-curpw">"Current Password"</label>
                                        </div>
                                        <div class="connect-text-field__input-wrapper">
                                            <input
                                                class="connect-text-field__input"
                                                id="profile-curpw"
                                                type="password"
                                                maxlength=128
                                                autocomplete="current-password"
                                                placeholder="••••••••"
                                                required=true
                                                prop:value=move || current_password.get()
                                                on:input=input_handler(set_current_password)
                                            />
                                        </div>
                                    </div>
                                })
                            }}

                            <div style="display: flex; gap: var(--ds-layout-spacing-200, 8px); justify-content: flex-end;">
                                <button
                                    class="connect-button connect-button--neutral connect-button--outline connect-button--medium"
                                    on:click=move |_| cancel_edit()
                                >
                                    <span class="connect-button__content">
                                        <span class="connect-button__label">"Cancel"</span>
                                    </span>
                                </button>
                                <button
                                    class="connect-button connect-button--accent connect-button--medium"
                                    disabled=move || {
                                        let em = email.get();
                                        let email_invalid = em.trim().is_empty()
                                            || !em.contains('@')
                                            || em.split('@').nth(1).map(|d| !d.contains('.')).unwrap_or(true);
                                        saving.get() || firstname.get().trim().is_empty() || lastname.get().trim().is_empty() || email_invalid || (!password.get().is_empty() && current_password.get().is_empty())
                                    }
                                    on:click=move |_| save_profile()
                                >
                                    <span class="connect-button__content">
                                        <span class="connect-button__label">
                                            {move || if saving.get() { "Saving..." } else { "Save" }}
                                        </span>
                                    </span>
                                </button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    // Read-only profile view
                    view! {
                        <div class="card" style="max-width: 480px;">
                            <div class="profile-header" style="display: flex; align-items: center; gap: var(--ds-layout-spacing-300, 16px); margin-bottom: var(--ds-layout-spacing-400, 24px);">
                                <div class="profile-avatar-wrapper" style="position: relative; cursor: pointer;" on:click=move |_| open_picker()>
                                    {match u.avatar_id.as_deref() {
                                        Some(aid) => {
                                            let src = avatar_url(aid);
                                            view! {
                                                <img
                                                    class="user-avatar user-avatar--large"
                                                    src=src
                                                    alt="User avatar"
                                                    style="object-fit: cover;"
                                                />
                                            }.into_any()
                                        }
                                        None => {
                                            view! {
                                                <div class="user-avatar user-avatar--large">
                                                    {u.initials()}
                                                </div>
                                            }.into_any()
                                        }
                                    }}
                                    <div class="avatar-edit-overlay">
                                        <Icon kind=IconKind::PenToSquare size=16 />
                                    </div>
                                </div>
                                <div>
                                    <h2 style="margin: 0; font-size: var(--ds-typo-font-size-200, 18px); font-weight: var(--ds-typo-font-weight-bold, 700);">{u.display_name()}</h2>
                                    <p class="text-muted" style="margin: 0;">{u.email.clone()}</p>
                                </div>
                            </div>

                            <div class="connect-divider" style="margin: var(--ds-layout-spacing-300, 16px) 0;"></div>

                            <div class="profile-details">
                                <div class="profile-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                    <span class="text-muted" style="font-size: var(--ds-typo-font-size-075, 12px);">"First Name"</span>
                                    <p style="margin: 4px 0 0;">{u.firstname.clone()}</p>
                                </div>
                                <div class="profile-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                    <span class="text-muted" style="font-size: var(--ds-typo-font-size-075, 12px);">"Last Name"</span>
                                    <p style="margin: 4px 0 0;">{u.lastname.clone()}</p>
                                </div>
                                <div class="profile-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                    <span class="text-muted" style="font-size: var(--ds-typo-font-size-075, 12px);">"Email"</span>
                                    <p style="margin: 4px 0 0;">{u.email.clone()}</p>
                                </div>
                            </div>

                            {if !u.teams.is_empty() {
                                let teams = u.teams.clone();
                                view! {
                                    <div class="connect-divider" style="margin: var(--ds-layout-spacing-300, 16px) 0;"></div>
                                    <div class="section-title">"Team Memberships"</div>
                                    <table class="connect-table connect-table--small">
                                        <thead class="connect-table-header">
                                            <tr>
                                                <th class="connect-table-header-cell">"Team"</th>
                                                <th class="connect-table-header-cell">"Role"</th>
                                            </tr>
                                        </thead>
                                        <tbody class="connect-table-body">
                                            {teams.into_iter().map(|t| {
                                                let cls = role_tag_class(&t.title);
                                                view! {
                                                    <tr class="connect-table-row">
                                                        <td class="connect-table-cell">{t.tname}</td>
                                                        <td class="connect-table-cell">
                                                            <span class=cls>{t.title}</span>
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                }.into_any()
                            } else {
                                view! {
                                    <p class="text-muted" style="margin-top: var(--ds-layout-spacing-300, 16px);">"Not a member of any team."</p>
                                }.into_any()
                            }}
                        </div>
                    }.into_any()
                }
            }}

            // Avatar picker dialog
            {move || {
                if !picker_open.get() {
                    return view! { <span /> }.into_any();
                }

                let current_avatar_id = user.get().and_then(|u| u.avatar_id.clone());
                let has_avatar = current_avatar_id.is_some();

                view! {
                    <div class="modal-overlay" on:click=move |_| set_picker_open.set(false)>
                        <div class="modal-dialog avatar-picker-dialog" on:click=move |ev| ev.stop_propagation()>
                            <div class="modal-header">
                                <h3 class="modal-title">"Choose Avatar"</h3>
                                <button
                                    class="connect-button connect-button--neutral connect-button--outline connect-button--small"
                                    on:click=move |_| set_picker_open.set(false)
                                >
                                    <span class="connect-button__content">
                                        <span class="connect-button__label">"×"</span>
                                    </span>
                                </button>
                            </div>
                            <div class="modal-body">
                                {move || {
                                    if avatar_loading.get() {
                                        return view! {
                                            <div class="empty-state"><p>"Loading avatars..."</p></div>
                                        }.into_any();
                                    }
                                    let items = avatars.get();
                                    if items.is_empty() {
                                        return view! {
                                            <div class="empty-state"><p>"No avatars available."</p></div>
                                        }.into_any();
                                    }
                                    let current_id = user.get().and_then(|u| u.avatar_id.clone());
                                    view! {
                                        <div class="avatar-grid">
                                            {items.iter().map(|a| {
                                                let aid = a.avatar_id.clone();
                                                let aid2 = a.avatar_id.clone();
                                                let name = a.name.clone();
                                                let src = avatar_url(&a.avatar_id);
                                                let is_selected = current_id.as_deref() == Some(aid.as_str());
                                                let cls = if is_selected {
                                                    "avatar-grid__item avatar-grid__item--selected"
                                                } else {
                                                    "avatar-grid__item"
                                                };
                                                view! {
                                                    <button
                                                        class=cls
                                                        title=name
                                                        on:click=move |_| select_avatar(aid2.clone())
                                                    >
                                                        <img src=src alt="Avatar" class="avatar-grid__img" />
                                                    </button>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                            {if has_avatar {
                                view! {
                                    <div class="modal-footer">
                                        <button
                                            class="connect-button connect-button--negative connect-button--outline connect-button--small"
                                            on:click=move |_| remove_avatar()
                                        >
                                            <span class="connect-button__content">
                                                <span class="connect-button__icon">
                                                    <Icon kind=IconKind::Trash size=14 />
                                                </span>
                                                <span class="connect-button__label">"Remove Avatar"</span>
                                            </span>
                                        </button>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span /> }.into_any()
                            }}
                        </div>
                    </div>
                }.into_any()
            }}
        </div>
    }
}
