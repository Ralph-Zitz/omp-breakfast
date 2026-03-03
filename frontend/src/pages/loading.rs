use leptos::prelude::*;

#[component]
pub fn LoadingPage() -> impl IntoView {
    view! {
        <div class="page loading-page">
            <div class="card loading-card">
                <div class="connect-progress-circle connect-progress-circle--indeterminate">
                    <svg class="connect-progress-circle__bar" viewBox="0 0 40 40">
                        <circle class="connect-progress-circle__background" cx="20" cy="20" r="17" />
                        <circle class="connect-progress-circle__indicator" cx="20" cy="20" r="17" />
                    </svg>
                </div>
                <p class="loading-text">"Loading\u{2026}"</p>
            </div>
        </div>
    }
}
