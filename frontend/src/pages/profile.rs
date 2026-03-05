use crate::api::{
    HttpMethod, PaginatedResponse, UserContext, UserInTeams, authed_get, authed_request,
};
use crate::components::card::PageHeader;
use crate::components::icons::{Icon, IconKind};
use crate::components::role_tag_class;
use crate::components::toast::{toast_error, toast_success};
use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

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

        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/users/{}", user_id);
            let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    // Refresh user context with updated data
                    if let Some(resp) = authed_get(&format!("/api/v1.0/users/{}", user_id)).await {
                        if resp.ok() {
                            if let Ok(updated) = resp.json::<crate::api::UserEntry>().await {
                                // Fetch fresh teams too
                                let teams_url = format!("/api/v1.0/users/{}/teams", user_id);
                                let teams = if let Some(tr) = authed_get(&teams_url).await {
                                    if tr.ok() {
                                        tr.json::<PaginatedResponse<UserInTeams>>()
                                            .await
                                            .map(|p| p.items)
                                            .unwrap_or_default()
                                    } else {
                                        Vec::new()
                                    }
                                } else {
                                    Vec::new()
                                };

                                let is_admin = teams.iter().any(|t| t.title == "Admin");
                                set_user.set(Some(UserContext {
                                    user_id: updated.user_id.clone(),
                                    firstname: updated.firstname,
                                    lastname: updated.lastname,
                                    email: updated.email,
                                    is_admin,
                                    teams,
                                }));
                            }
                        }
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
                                    <label class="connect-label__text" for="profile-ln">"Last Name"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="profile-ln"
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
                                    <label class="connect-label__text" for="profile-email">"Email"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="profile-email"
                                        type="email"
                                        prop:value=move || email.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return };
                                            set_email.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                        }
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
                                        placeholder="••••••••"
                                        prop:value=move || password.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return };
                                            set_password.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                        }
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
                                                placeholder="••••••••"
                                                required=true
                                                prop:value=move || current_password.get()
                                                on:input=move |ev| {
                                                    let Some(target) = ev.target() else { return };
                                                    set_current_password.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                                }
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
                                    disabled=move || saving.get() || firstname.get().trim().is_empty() || lastname.get().trim().is_empty() || email.get().trim().is_empty() || (!password.get().is_empty() && current_password.get().is_empty())
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
                                <div class="user-avatar user-avatar--large">
                                    {u.initials()}
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
        </div>
    }
}
