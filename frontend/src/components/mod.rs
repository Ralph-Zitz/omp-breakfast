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

/// Pagination bar: shows "Showing X-Y of N" + prev/next buttons.
/// `offset` and `limit` are the current slice parameters.
/// `total` is the total item count from the API response.
/// `on_prev` / `on_next` receive the new offset value.
#[component]
pub fn PaginationBar(
    offset: ReadSignal<usize>,
    limit: usize,
    total: ReadSignal<usize>,
    on_prev: impl Fn(usize) + 'static + Clone + Send,
    on_next: impl Fn(usize) + 'static + Clone + Send,
) -> impl IntoView {
    view! {
        {move || {
            let off = offset.get();
            let tot = total.get();
            if tot <= limit {
                return view! { <span /> }.into_any();
            }
            let start = off + 1;
            let end = (off + limit).min(tot);
            let has_prev = off > 0;
            let has_next = off + limit < tot;
            let on_prev2 = on_prev.clone();
            let on_next2 = on_next.clone();
            view! {
                <div class="pagination-bar" style="display: flex; align-items: center; gap: var(--ds-layout-spacing-200, 8px); margin-top: var(--ds-layout-spacing-200, 12px);">
                    <button
                        class="connect-button connect-button--neutral connect-button--outline connect-button--small"
                        disabled=!has_prev
                        on:click=move |_| on_prev2(off.saturating_sub(limit))
                    >
                        <span class="connect-button__content">
                            <span class="connect-button__label">"← Prev"</span>
                        </span>
                    </button>
                    <span class="text-muted" style="font-size: var(--ds-typo-font-size-075, 12px);">
                        {format!("{start}–{end} of {tot}")}
                    </span>
                    <button
                        class="connect-button connect-button--neutral connect-button--outline connect-button--small"
                        disabled=!has_next
                        on:click=move |_| on_next2(off + limit)
                    >
                        <span class="connect-button__content">
                            <span class="connect-button__label">"Next →"</span>
                        </span>
                    </button>
                </div>
            }.into_any()
        }}
    }
}
