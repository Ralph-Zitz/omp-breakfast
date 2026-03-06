use crate::api::UserContext;
use crate::components::card::PageHeader;
use crate::components::role_tag_class;
use leptos::prelude::*;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let user = expect_context::<ReadSignal<Option<UserContext>>>();

    view! {
        <div class="dashboard-page">
            <PageHeader title="Dashboard" />

            {move || {
                user.with(|u| {
                    u.as_ref().map(|u| {
                        let name = u.display_name();
                        let initials = u.initials();
                        let email = u.email.clone();
                        let team_count = u.teams.len();
                        let role_label = if u.is_admin { "Administrator" } else { "Member" };

                    view! {
                        <div class="dashboard-welcome">
                            <div class="card dashboard-card">
                                <SuccessBadge />
                                <h2>"Welcome!"</h2>
                                <p class="success-text">"You have successfully signed in."</p>
                                <UserCard name=name.clone() initials=initials.clone() email=email.clone() />
                            </div>

                            <div class="card dashboard-teams-card">
                                <div class="dashboard-stats">
                                    <div class="stat-card">
                                        <span class="stat-card__value">{team_count}</span>
                                        <span class="stat-card__label">{if team_count == 1 { "Team" } else { "Teams" }}</span>
                                    </div>
                                    <div class="stat-card">
                                        <span class="connect-tag connect-tag--medium connect-tag--primary-default stat-card__tag">
                                            <span class="connect-tag__text-wrapper">
                                                <span class="connect-tag__text">{role_label}</span>
                                            </span>
                                        </span>
                                        <span class="stat-card__label">"Role"</span>
                                    </div>
                                </div>

                                // Team membership overview
                                {if !u.teams.is_empty() {
                                    let teams = u.teams.clone();
                                    view! {
                                        <h3 class="section-title">"Your Teams"</h3>
                                        <table class="connect-table connect-table--medium dashboard-teams-table">
                                            <thead class="connect-table-header">
                                                <tr>
                                                    <th class="connect-table-header-cell">"Team"</th>
                                                    <th class="connect-table-header-cell">"Role"</th>
                                                </tr>
                                            </thead>
                                            <tbody class="connect-table-body">
                                                {teams.into_iter().map(|t| {
                                                    let tname = t.tname.clone();
                                                    let role = t.title.clone();
                                                    let tag_class = role_tag_class(&role);
                                                    view! {
                                                        <tr class="connect-table-row">
                                                            <td class="connect-table-cell">{tname}</td>
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
                                } else {
                                    view! { <div /> }.into_any()
                                }}
                            </div>
                        </div>
                    }
                })
                })
            }}
        </div>
    }
}

#[component]
fn SuccessBadge() -> impl IntoView {
    view! {
        <div class="success-badge">
            <svg class="success-check-icon" viewBox="0 0 40 40" fill="currentColor" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
                <path d="M20.031 36c-5.75 0-11-3-13.875-8-2.875-4.938-2.875-11 0-16 2.875-4.938 8.125-8 13.875-8 5.688 0 10.938 3.063 13.813 8 2.875 5 2.875 11.063 0 16-2.875 5-8.125 8-13.813 8Zm7.063-18.938h-.063c.625-.562.625-1.5 0-2.125a1.471 1.471 0 0 0-2.062 0l-6.938 7L15.094 19c-.625-.625-1.563-.625-2.125 0a1.369 1.369 0 0 0 0 2.063l4 4c.562.625 1.5.625 2.125 0l8-8Z"/>
            </svg>
        </div>
    }
}

#[component]
fn UserCard(name: String, initials: String, email: String) -> impl IntoView {
    view! {
        <div class="user-card">
            <div class="connect-avatar connect-avatar--x-large connect-avatar--initials connect-avatar--bg-yellow">
                <span class="connect-avatar__text">{initials}</span>
            </div>
            <div class="user-details">
                <span class="user-name">{name}</span>
                <span class="user-email">{email}</span>
            </div>
        </div>
    }
}
