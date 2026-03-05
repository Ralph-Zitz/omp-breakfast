pub mod card;
pub mod icons;
pub mod modal;
pub mod sidebar;
pub mod theme_toggle;
pub mod toast;

use leptos::prelude::*;

/// Returns the full CSS class string for a role tag badge.
/// Used across dashboard, teams, profile, and roles pages.
pub fn role_tag_class(role: &str) -> &'static str {
    match role {
        "Admin" => "connect-tag connect-tag--small connect-tag--negative-emphasis",
        "Team Admin" => "connect-tag connect-tag--small connect-tag--warning-default",
        "Member" => "connect-tag connect-tag--small connect-tag--primary-default",
        _ => "connect-tag connect-tag--small connect-tag--neutral-default",
    }
}

/// Shared indeterminate progress circle used while data is loading.
#[component]
pub fn LoadingSpinner() -> impl IntoView {
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
