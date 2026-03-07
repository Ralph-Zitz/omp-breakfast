use crate::api::{
    HttpMethod, PaginatedResponse, RoleEntry, TeamEntry, UserContext, UserEntry, UsersInTeam,
    authed_get, authed_request,
};
use crate::components::card::PageHeader;
use crate::components::icons::{Icon, IconKind};
use crate::components::modal::ConfirmModal;
use crate::components::toast::{toast_error, toast_success};
use crate::components::{LoadingSpinner, PaginationBar, role_tag_class};
use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

#[component]
pub fn TeamsPage() -> impl IntoView {
    let (teams, set_teams) = signal(Vec::<TeamEntry>::new());
    let (loading, set_loading) = signal(true);
    let (selected_team, set_selected_team) = signal(Option::<String>::None);
    let (team_members, set_team_members) = signal(Vec::<UsersInTeam>::new());
    let (members_loading, set_members_loading) = signal(false);
    let (show_create, set_show_create) = signal(false);
    let (delete_target, set_delete_target) = signal(Option::<(String, String)>::None);
    let (edit_target, set_edit_target) = signal(Option::<TeamEntry>::None);
    let (show_add_member, set_show_add_member) = signal(false);
    let (available_users, set_available_users) = signal(Vec::<UserEntry>::new());
    let (available_roles, set_available_roles) = signal(Vec::<RoleEntry>::new());
    let (remove_member_target, set_remove_member_target) = signal(Option::<(String, String)>::None); // (user_id, name)
    let (offset, set_offset) = signal(0usize);
    let (total, set_total) = signal(0usize);
    let limit = 50usize;

    let user = expect_context::<ReadSignal<Option<UserContext>>>();
    let is_admin = Signal::derive(move || user.get().map(|u| u.is_admin).unwrap_or(false));

    // Fetch teams on mount
    let fetch_teams = move |off: usize| {
        set_loading.set(true);
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams?limit={}&offset={}", limit, off);
            if let Some(resp) = authed_get(&url).await
                && resp.ok()
            {
                match resp.json::<PaginatedResponse<TeamEntry>>().await {
                    Ok(data) => {
                        set_total.set(data.total as usize);
                        set_teams.set(data.items);
                    }
                    Err(e) => {
                        web_sys::console::warn_1(&format!("teams JSON parse error: {e}").into())
                    }
                }
            }
            set_loading.set(false);
        });
    };
    fetch_teams(0);

    let load_members = move |team_id: String| {
        set_selected_team.set(Some(team_id.clone()));
        set_members_loading.set(true);
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}/users", team_id);
            if let Some(resp) = authed_get(&url).await
                && resp.ok()
            {
                match resp.json::<PaginatedResponse<UsersInTeam>>().await {
                    Ok(data) => set_team_members.set(data.items),
                    Err(e) => web_sys::console::warn_1(
                        &format!("team members JSON parse error: {e}").into(),
                    ),
                }
            }
            set_members_loading.set(false);
        });
    };

    let do_update_team = move |team_id: String, name: String, descr: Option<String>| {
        let body = serde_json::json!({ "tname": name, "descr": descr });
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}", team_id);
            let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => match r.json::<TeamEntry>().await {
                    Ok(updated) => {
                        set_teams.update(|list| {
                            if let Some(t) = list.iter_mut().find(|t| t.team_id == updated.team_id)
                            {
                                *t = updated;
                            }
                        });
                        toast_success("Team updated");
                    }
                    Err(e) => web_sys::console::warn_1(
                        &format!("team update JSON parse error: {e}").into(),
                    ),
                },
                _ => toast_error("Failed to update team"),
            }
            set_edit_target.set(None);
        });
    };

    let open_add_member = move |_team_id: String| {
        leptos::task::spawn_local_scoped(async move {
            if let Some(r) = authed_get("/api/v1.0/users?limit=100").await
                && r.ok()
            {
                match r.json::<PaginatedResponse<UserEntry>>().await {
                    Ok(data) => set_available_users.set(data.items),
                    Err(e) => {
                        web_sys::console::warn_1(&format!("users JSON parse error: {e}").into())
                    }
                }
            }
            if let Some(r) = authed_get("/api/v1.0/roles?limit=100").await
                && r.ok()
            {
                match r.json::<PaginatedResponse<RoleEntry>>().await {
                    Ok(data) => set_available_roles.set(data.items),
                    Err(e) => {
                        web_sys::console::warn_1(&format!("roles JSON parse error: {e}").into())
                    }
                }
            }
            set_show_add_member.set(true);
        });
    };

    let do_add_member = move |user_id: String, role_id: String| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        let body = serde_json::json!({ "user_id": user_id, "role_id": role_id });
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}/users", team_id);
            let resp = authed_request(HttpMethod::Post, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    // Reload members
                    let members_url = format!("/api/v1.0/teams/{}/users", team_id);
                    if let Some(mr) = authed_get(&members_url).await
                        && mr.ok()
                    {
                        match mr.json::<PaginatedResponse<UsersInTeam>>().await {
                            Ok(data) => set_team_members.set(data.items),
                            Err(e) => web_sys::console::warn_1(
                                &format!("team members JSON parse error: {e}").into(),
                            ),
                        }
                    }
                    toast_success("Member added");
                }
                Some(r) if r.status() == 409 => {
                    toast_error("User is already a member of this team")
                }
                _ => toast_error("Failed to add member"),
            }
            set_show_add_member.set(false);
        });
    };

    let do_remove_member = move |user_id: String| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}/users/{}", team_id, user_id);
            let resp = authed_request(HttpMethod::Delete, &url, None).await;
            match resp {
                Some(r) if r.ok() => {
                    set_team_members.update(|list| list.retain(|m| m.user_id != user_id));
                    toast_success("Member removed");
                }
                _ => toast_error("Failed to remove member"),
            }
            set_remove_member_target.set(None);
        });
    };

    let do_update_member_role = move |user_id: String, role_id: String| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        let body = serde_json::json!({ "role_id": role_id });
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}/users/{}", team_id, user_id);
            let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    // Reload members to get fresh role titles
                    let members_url = format!("/api/v1.0/teams/{}/users", team_id);
                    if let Some(mr) = authed_get(&members_url).await
                        && mr.ok()
                    {
                        match mr.json::<PaginatedResponse<UsersInTeam>>().await {
                            Ok(data) => set_team_members.set(data.items),
                            Err(e) => web_sys::console::warn_1(
                                &format!("team members JSON parse error: {e}").into(),
                            ),
                        }
                    }
                    toast_success("Role updated");
                }
                _ => toast_error("Failed to update role"),
            }
        });
    };

    let do_create_team = move |name: String, descr: Option<String>| {
        let body = serde_json::json!({ "tname": name, "descr": descr });
        leptos::task::spawn_local_scoped(async move {
            let resp = authed_request(HttpMethod::Post, "/api/v1.0/teams", Some(&body)).await;
            match resp {
                Some(r) if r.ok() => match r.json::<TeamEntry>().await {
                    Ok(team) => {
                        set_teams.update(|list| list.push(team));
                        toast_success("Team created");
                    }
                    Err(e) => web_sys::console::warn_1(
                        &format!("team create JSON parse error: {e}").into(),
                    ),
                },
                _ => toast_error("Failed to create team"),
            }
            set_show_create.set(false);
        });
    };

    let do_delete_team = move |team_id: String| {
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}", team_id);
            let resp = authed_request(HttpMethod::Delete, &url, None).await;
            match resp {
                Some(r) if r.ok() => {
                    set_teams.update(|list| list.retain(|t| t.team_id != team_id));
                    if selected_team.get().as_deref() == Some(&team_id) {
                        set_selected_team.set(None);
                        set_team_members.set(Vec::new());
                    }
                    toast_success("Team deleted");
                }
                _ => toast_error("Failed to delete team"),
            }
            set_delete_target.set(None);
        });
    };

    view! {
        <div class="teams-page">
            <PageHeader title="Teams">
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
                                    <span class="connect-button__label">"New Team"</span>
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

                let team_list = teams.get();
                if team_list.is_empty() {
                    return view! {
                        <div class="empty-state">
                            <Icon kind=IconKind::Users size=48 />
                            <p>"No teams found."</p>
                        </div>
                    }.into_any();
                }

                view! {
                    <div class="content-split">
                        <div class="card">
                            <table class="connect-table connect-table--medium">
                                <thead class="connect-table-header">
                                    <tr>
                                        <th class="connect-table-header-cell">"Name"</th>
                                        <th class="connect-table-header-cell">"Description"</th>
                                        {move || is_admin.get().then(|| view! {
                                            <th class="connect-table-header-cell connect-table-header-cell--actions">"Actions"</th>
                                        })}
                                    </tr>
                                </thead>
                                <tbody class="connect-table-body">
                                    {team_list.into_iter().map(|team| {
                                        let tid = team.team_id.clone();
                                        let tid_del = team.team_id.clone();
                                        let tname = team.tname.clone();
                                        let tname_del = team.tname.clone();
                                        let descr = team.descr.clone().unwrap_or_default();
                                        let team_entry = team.clone();
                                        let is_selected = {
                                            let tid = tid.clone();
                                            move || selected_team.get().as_deref() == Some(&tid)
                                        };
                                        let load = load_members;

                                        view! {
                                            <tr
                                                class=move || if is_selected() {
                                                    "connect-table-row connect-table-row--selected connect-table-row--clickable"
                                                } else {
                                                    "connect-table-row connect-table-row--clickable"
                                                }
                                                on:click={
                                                    let tid = tid.clone();
                                                    move |_| load(tid.clone())
                                                }
                                            >
                                                <td class="connect-table-cell">{tname.clone()}</td>
                                                <td class="connect-table-cell">{descr.clone()}</td>
                                                {move || is_admin.get().then(|| {
                                                    let tid_del = tid_del.clone();
                                                    let tname_del = tname_del.clone();
                                                    let team_for_edit = team_entry.clone();
                                                    view! {
                                                        <td class="connect-table-cell connect-table-cell--actions">
                                                            <button
                                                                aria-label="Edit team"
                                                                class="connect-button connect-button--neutral connect-button--outline connect-button--small"
                                                                on:click=move |ev| {
                                                                    ev.stop_propagation();
                                                                    set_edit_target.set(Some(team_for_edit.clone()));
                                                                }
                                                            >
                                                                <span class="connect-button__content">
                                                                    <span class="connect-button__icon">
                                                                        <Icon kind=IconKind::PenToSquare size=14 />
                                                                    </span>
                                                                </span>
                                                            </button>
                                                            <button
                                                                aria-label="Delete team"
                                                                class="connect-button connect-button--negative connect-button--outline connect-button--small"
                                                                on:click=move |ev| {
                                                                    ev.stop_propagation();
                                                                    set_delete_target.set(Some((tid_del.clone(), tname_del.clone())));
                                                                }
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
                                on_prev=move |off| { set_offset.set(off); fetch_teams(off); }
                                on_next=move |off| { set_offset.set(off); fetch_teams(off); }
                            />
                        </div>

                        // Team members panel
                        {move || {
                            selected_team.get().map(|tid| {
                                if members_loading.get() {
                                    return view! { <div class="card"><LoadingSpinner /></div> }.into_any();
                                }
                                let members = team_members.get();
                                let roles_for_panel = available_roles.get();
                                let can_manage = is_admin.get()
                                    || user.get().map(|u| u.teams.iter().any(|t| t.team_id == tid && t.title == "Team Admin")).unwrap_or(false);
                                view! {
                                    <div class="card">
                                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                            <h3 class="section-title" style="margin: 0;">"Team Members"</h3>
                                            {can_manage.then(|| {
                                                let oam = open_add_member;
                                                let tid2 = tid.clone();
                                                view! {
                                                    <button
                                                        class="connect-button connect-button--accent connect-button--small"
                                                        on:click=move |_| oam(tid2.clone())
                                                    >
                                                        <span class="connect-button__content">
                                                            <span class="connect-button__icon">
                                                                <Icon kind=IconKind::CirclePlus size=14 />
                                                            </span>
                                                            <span class="connect-button__label">"Add"</span>
                                                        </span>
                                                    </button>
                                                }
                                            })}
                                        </div>
                                        {if members.is_empty() {
                                            view! { <p class="text-muted">"No members in this team."</p> }.into_any()
                                        } else {
                                            let roles_for_rows = roles_for_panel.clone();
                                            view! {
                                                <table class="connect-table connect-table--small">
                                                    <thead class="connect-table-header">
                                                        <tr>
                                                            <th class="connect-table-header-cell">"Name"</th>
                                                            <th class="connect-table-header-cell">"Email"</th>
                                                            <th class="connect-table-header-cell">"Role"</th>
                                                            {can_manage.then(|| view! {
                                                                <th class="connect-table-header-cell connect-table-header-cell--actions">"Actions"</th>
                                                            })}
                                                        </tr>
                                                    </thead>
                                                    <tbody class="connect-table-body">
                                                        {members.into_iter().map(|m| {
                                                            let name = format!("{} {}", m.firstname, m.lastname);
                                                            let email = m.email.clone();
                                                            let uid = m.user_id.clone();
                                                            let uid_del = m.user_id.clone();
                                                            let name_del = name.clone();
                                                            let tag_class = role_tag_class(&m.title);
                                                            let current_role_title = m.title.clone();
                                                            let roles_for_select = roles_for_rows.clone();
                                                            let _tid_update = tid.clone();
                                                            view! {
                                                                <tr class="connect-table-row">
                                                                    <td class="connect-table-cell">{name}</td>
                                                                    <td class="connect-table-cell">{email}</td>
                                                                    <td class="connect-table-cell">
                                                                        {if can_manage && !roles_for_select.is_empty() {
                                                                            let dam = do_update_member_role;
                                                                            let uid2 = uid.clone();
                                                                            view! {
                                                                                <select
                                                                                    class="connect-text-field__input"
                                                                                    style="width: auto; min-width: 120px;"
                                                                                    prop:value=current_role_title.clone()
                                                                                    on:change=move |ev| {
                                                                                        let Some(target) = ev.target() else { return; };
                                                                                        let new_role_id = target.unchecked_into::<web_sys::HtmlSelectElement>().value();
                                                                                        dam(uid2.clone(), new_role_id);
                                                                                    }
                                                                                >
                                                                                    {roles_for_select.into_iter().map(|r| {
                                                                                        let rid = r.role_id.clone();
                                                                                        let rtitle = r.title.clone();
                                                                                        let selected = rtitle == current_role_title;
                                                                                        view! {
                                                                                            <option value=rid selected=selected>{rtitle}</option>
                                                                                        }
                                                                                    }).collect::<Vec<_>>()}
                                                                                </select>
                                                                            }.into_any()
                                                                        } else {
                                                                            view! {
                                                                                <span class=tag_class>
                                                                                    <span class="connect-tag__text-wrapper">
                                                                                        <span class="connect-tag__text">{current_role_title}</span>
                                                                                    </span>
                                                                                </span>
                                                                            }.into_any()
                                                                        }}
                                                                    </td>
                                                                    {can_manage.then(|| {
                                                                        let uid_del = uid_del.clone();
                                                                        let name_del = name_del.clone();
                                                                        view! {
                                                                            <td class="connect-table-cell connect-table-cell--actions">
                                                                                <button
                                                                                    aria-label="Remove member"
                                                                                    class="connect-button connect-button--negative connect-button--outline connect-button--small"
                                                                                    on:click=move |_| set_remove_member_target.set(Some((uid_del.clone(), name_del.clone())))
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
                                            }.into_any()
                                        }}
                                    </div>
                                }.into_any()
                            })
                        }}
                    </div>
                }.into_any()
            }}

            // Create team dialog
            <CreateTeamDialog
                open=show_create.into()
                on_create=do_create_team
                on_cancel=move || set_show_create.set(false)
            />

            // Edit team dialog
            {move || {
                let target = edit_target.get();
                let open = Signal::derive(move || edit_target.get().is_some());
                if let Some(team) = target {
                    view! {
                        <EditTeamDialog
                            open=open
                            team=team
                            on_save=do_update_team
                            on_cancel=move || set_edit_target.set(None)
                        />
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }
            }}

            // Add member dialog
            {move || {
                let open = Signal::derive(move || show_add_member.get());
                view! {
                    <AddMemberDialog
                        open=open
                        users=available_users
                        roles=available_roles
                        on_add=do_add_member
                        on_cancel=move || set_show_add_member.set(false)
                    />
                }
            }}

            // Remove member confirmation
            {move || {
                let open = Signal::derive(move || remove_member_target.get().is_some());
                let (uid, uname) = remove_member_target.get().unwrap_or_default();
                let uid_clone = uid.clone();
                view! {
                    <ConfirmModal
                        open=open
                        title="Remove Member".to_string()
                        message=format!("Remove {} from this team?", uname)
                        confirm_label="Remove"
                        destructive=true
                        on_confirm=move || do_remove_member(uid_clone.clone())
                        on_cancel=move || set_remove_member_target.set(None)
                    />
                }
            }}

            // Delete confirmation modal
            {move || {
                let open = Signal::derive(move || delete_target.get().is_some());
                let (tid, tname) = delete_target.get().unwrap_or_default();
                let tid_clone = tid.clone();
                view! {
                    <ConfirmModal
                        open=open
                        title=format!("Delete Team")
                        message=format!("Are you sure you want to delete \"{}\"? This action cannot be undone.", tname)
                        confirm_label="Delete"
                        destructive=true
                        on_confirm=move || do_delete_team(tid_clone.clone())
                        on_cancel=move || set_delete_target.set(None)
                    />
                }
            }}
        </div>
    }
}

#[component]
fn CreateTeamDialog(
    open: Signal<bool>,
    on_create: impl Fn(String, Option<String>) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (descr, set_descr) = signal(String::new());

    let reset = move || {
        set_name.set(String::new());
        set_descr.set(String::new());
    };

    view! {
        {move || {
            if !open.get() {
                return view! { <div class="modal-hidden" /> }.into_any();
            }

            let on_create = on_create.clone();
            let reset_bd = reset;
            let reset_b = reset;
            let on_cancel_bd = on_cancel.clone();
            let on_cancel_b = on_cancel.clone();

            view! {
                <div class="modal-overlay" on:click=move |_| { reset_bd(); on_cancel_bd(); }>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h2 class="modal-title">"New Team"</h2>
                        </div>
                        <div class="modal-body">
                            <div class="connect-text-field">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="team-name">"Team Name"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="team-name"
                                        type="text"
                                        placeholder="Enter team name"
                                        prop:value=move || name.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return; };
                                            set_name.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                        }
                                    />
                                </div>
                            </div>
                            <div class="connect-text-field" style="margin-top: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="team-descr">"Description (optional)"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="team-descr"
                                        type="text"
                                        placeholder="Team description"
                                        prop:value=move || descr.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return; };
                                            set_descr.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
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
                                    let reset = reset_b;
                                    move |_| { reset(); cancel(); }
                                }
                            >
                                <span class="connect-button__content">
                                    <span class="connect-button__label">"Cancel"</span>
                                </span>
                            </button>
                            <button
                                class="connect-button connect-button--accent connect-button--medium"
                                disabled=move || name.get().trim().is_empty()
                                on:click={
                                    let create = on_create.clone();
                                    move |_| {
                                        let n = name.get();
                                        let d = descr.get();
                                        let d = if d.trim().is_empty() { None } else { Some(d) };
                                        create(n, d);
                                        set_name.set(String::new());
                                        set_descr.set(String::new());
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
fn EditTeamDialog(
    open: Signal<bool>,
    team: TeamEntry,
    on_save: impl Fn(String, String, Option<String>) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (name, set_name) = signal(team.tname.clone());
    let (descr, set_descr) = signal(team.descr.clone().unwrap_or_default());
    let team_id = team.team_id.clone();

    view! {
        {move || {
            if !open.get() {
                return view! { <div class="modal-hidden" /> }.into_any();
            }

            let on_save = on_save.clone();
            let on_cancel_bd = on_cancel.clone();
            let on_cancel_b = on_cancel.clone();
            let tid = team_id.clone();

            view! {
                <div class="modal-overlay" on:click=move |_| on_cancel_bd()>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h2 class="modal-title">"Edit Team"</h2>
                        </div>
                        <div class="modal-body">
                            <div class="connect-text-field">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="edit-team-name">"Team Name"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="edit-team-name"
                                        type="text"
                                        prop:value=move || name.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return; };
                                            set_name.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                        }
                                    />
                                </div>
                            </div>
                            <div class="connect-text-field" style="margin-top: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="edit-team-descr">"Description (optional)"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="edit-team-descr"
                                        type="text"
                                        prop:value=move || descr.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return; };
                                            set_descr.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
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
                                disabled=move || name.get().trim().is_empty()
                                on:click={
                                    let save = on_save.clone();
                                    let tid = tid.clone();
                                    move |_| {
                                        let d = descr.get();
                                        let d = if d.trim().is_empty() { None } else { Some(d) };
                                        save(tid.clone(), name.get(), d);
                                    }
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

#[component]
fn AddMemberDialog(
    open: Signal<bool>,
    users: ReadSignal<Vec<UserEntry>>,
    roles: ReadSignal<Vec<RoleEntry>>,
    on_add: impl Fn(String, String) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (sel_user, set_sel_user) = signal(String::new());
    let (sel_role, set_sel_role) = signal(String::new());

    view! {
        {move || {
            if !open.get() {
                return view! { <div class="modal-hidden" /> }.into_any();
            }

            let on_add = on_add.clone();
            let on_cancel_bd = on_cancel.clone();
            let on_cancel_b = on_cancel.clone();
            let user_list = users.get();
            let role_list = roles.get();

            // Pre-select first role if none selected
            if sel_role.get().is_empty() && let Some(r) = role_list.first() {
                    set_sel_role.set(r.role_id.clone());
            }

            view! {
                <div class="modal-overlay" on:click=move |_| on_cancel_bd()>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h2 class="modal-title">"Add Team Member"</h2>
                        </div>
                        <div class="modal-body">
                            <div class="connect-text-field" style="margin-bottom: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="add-member-user">"User"</label>
                                </div>
                                <select
                                    id="add-member-user"
                                    class="connect-text-field__input"
                                    prop:value=move || sel_user.get()
                                    on:change=move |ev| {
                                        let Some(target) = ev.target() else { return; };
                                        set_sel_user.set(target.unchecked_into::<web_sys::HtmlSelectElement>().value());
                                    }
                                >
                                    <option value="">"Select user..."</option>
                                    {user_list.into_iter().map(|u| {
                                        let uid = u.user_id.clone();
                                        let label = format!("{} {} ({})", u.firstname, u.lastname, u.email);
                                        view! { <option value=uid>{label}</option> }
                                    }).collect::<Vec<_>>()}
                                </select>
                            </div>
                            <div class="connect-text-field">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="add-member-role">"Role"</label>
                                </div>
                                <select
                                    id="add-member-role"
                                    class="connect-text-field__input"
                                    prop:value=move || sel_role.get()
                                    on:change=move |ev| {
                                        let Some(target) = ev.target() else { return; };
                                        set_sel_role.set(target.unchecked_into::<web_sys::HtmlSelectElement>().value());
                                    }
                                >
                                    {role_list.into_iter().map(|r| {
                                        let rid = r.role_id.clone();
                                        let rtitle = r.title.clone();
                                        view! { <option value=rid>{rtitle}</option> }
                                    }).collect::<Vec<_>>()}
                                </select>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button
                                class="connect-button connect-button--neutral connect-button--outline connect-button--medium"
                                on:click={
                                    let cancel = on_cancel_b.clone();
                                    move |_| {
                                        set_sel_user.set(String::new());
                                        set_sel_role.set(String::new());
                                        cancel();
                                    }
                                }
                            >
                                <span class="connect-button__content">
                                    <span class="connect-button__label">"Cancel"</span>
                                </span>
                            </button>
                            <button
                                class="connect-button connect-button--accent connect-button--medium"
                                disabled=move || sel_user.get().is_empty() || sel_role.get().is_empty()
                                on:click={
                                    let add = on_add.clone();
                                    move |_| {
                                        add(sel_user.get(), sel_role.get());
                                        set_sel_user.set(String::new());
                                        set_sel_role.set(String::new());
                                    }
                                }
                            >
                                <span class="connect-button__content">
                                    <span class="connect-button__label">"Add Member"</span>
                                </span>
                            </button>
                        </div>
                    </div>
                </div>
            }.into_any()
        }}
    }
}
