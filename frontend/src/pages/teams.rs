use crate::api::{HttpMethod, TeamEntry, UserContext, UsersInTeam, authed_get, authed_request};
use crate::components::card::PageHeader;
use crate::components::icons::{Icon, IconKind};
use crate::components::modal::ConfirmModal;
use crate::components::toast::{toast_error, toast_success};
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

    let user = expect_context::<ReadSignal<Option<UserContext>>>();
    let is_admin = Signal::derive(move || user.get().map(|u| u.is_admin).unwrap_or(false));

    // Fetch teams on mount
    let set_teams_load = set_teams;
    let set_loading_load = set_loading;
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(resp) = authed_get("/api/v1.0/teams").await {
            if resp.ok() {
                if let Ok(data) = resp.json::<Vec<TeamEntry>>().await {
                    set_teams_load.set(data);
                }
            }
        }
        set_loading_load.set(false);
    });

    let load_members = move |team_id: String| {
        set_selected_team.set(Some(team_id.clone()));
        set_members_loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/teams/{}/users", team_id);
            if let Some(resp) = authed_get(&url).await {
                if resp.ok() {
                    if let Ok(data) = resp.json::<Vec<UsersInTeam>>().await {
                        set_team_members.set(data);
                    }
                }
            }
            set_members_loading.set(false);
        });
    };

    let do_create_team = move |name: String, descr: Option<String>| {
        let body = serde_json::json!({ "tname": name, "descr": descr });
        wasm_bindgen_futures::spawn_local(async move {
            let resp = authed_request(HttpMethod::Post, "/api/v1.0/teams", Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    if let Ok(team) = r.json::<TeamEntry>().await {
                        set_teams.update(|list| list.push(team));
                        toast_success("Team created");
                    }
                }
                _ => toast_error("Failed to create team"),
            }
            set_show_create.set(false);
        });
    };

    let do_delete_team = move |team_id: String| {
        wasm_bindgen_futures::spawn_local(async move {
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
                                        let is_selected = {
                                            let tid = tid.clone();
                                            move || selected_team.get().as_deref() == Some(&tid)
                                        };
                                        let load = load_members.clone();

                                        view! {
                                            <tr
                                                class=move || if is_selected() {
                                                    "connect-table-row connect-table-row--selected connect-table-row--clickable"
                                                } else {
                                                    "connect-table-row connect-table-row--clickable"
                                                }
                                                on:click={
                                                    let tid = tid.clone();
                                                    let load = load.clone();
                                                    move |_| load(tid.clone())
                                                }
                                            >
                                                <td class="connect-table-cell">{tname.clone()}</td>
                                                <td class="connect-table-cell">{descr.clone()}</td>
                                                {move || is_admin.get().then(|| {
                                                    let tid_del = tid_del.clone();
                                                    let tname_del = tname_del.clone();
                                                    view! {
                                                        <td class="connect-table-cell connect-table-cell--actions">
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
                        </div>

                        // Team members panel
                        {move || {
                            selected_team.get().map(|_tid| {
                                if members_loading.get() {
                                    return view! { <div class="card"><LoadingSpinner /></div> }.into_any();
                                }
                                let members = team_members.get();
                                view! {
                                    <div class="card">
                                        <h3 class="section-title">"Team Members"</h3>
                                        {if members.is_empty() {
                                            view! { <p class="text-muted">"No members in this team."</p> }.into_any()
                                        } else {
                                            view! {
                                                <table class="connect-table connect-table--small">
                                                    <thead class="connect-table-header">
                                                        <tr>
                                                            <th class="connect-table-header-cell">"Name"</th>
                                                            <th class="connect-table-header-cell">"Email"</th>
                                                            <th class="connect-table-header-cell">"Role"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody class="connect-table-body">
                                                        {members.into_iter().map(|m| {
                                                            let name = format!("{} {}", m.firstname, m.lastname);
                                                            let email = m.email.clone();
                                                            let role = m.title.clone();
                                                            let tag_class = role_tag_class(&role);
                                                            view! {
                                                                <tr class="connect-table-row">
                                                                    <td class="connect-table-cell">{name}</td>
                                                                    <td class="connect-table-cell">{email}</td>
                                                                    <td class="connect-table-cell">
                                                                        <span class=tag_class>
                                                                            <span class="connect-tag__text-wrapper">
                                                                                <span class="connect-tag__text">{role}</span>
                                                                            </span>
                                                                        </span>
                                                                    </td>
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
                open=show_create
                on_create=do_create_team
                on_cancel=move || set_show_create.set(false)
            />

            // Delete confirmation modal
            {move || {
                let target = delete_target.get();
                let (del_open, _set_del_open) = signal(target.is_some());
                let (tid, tname) = target.unwrap_or_default();
                let tid_clone = tid.clone();
                view! {
                    <ConfirmModal
                        open=del_open
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

fn role_tag_class(role: &str) -> String {
    let color = match role {
        "Admin" => "negative-emphasis",
        "Team Admin" => "warning-default",
        "Member" => "primary-default",
        _ => "neutral-default",
    };
    format!("connect-tag connect-tag--small connect-tag--{}", color)
}

#[component]
fn CreateTeamDialog(
    open: ReadSignal<bool>,
    on_create: impl Fn(String, Option<String>) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (descr, set_descr) = signal(String::new());

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
