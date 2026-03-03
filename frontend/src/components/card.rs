use leptos::prelude::*;

/// Reusable card container using CONNECT Design System elevation tokens.
///
/// Wraps children in a styled card with surface background, subtle border,
/// and elevation shadow. Pass an optional `class` for extra styling.
#[component]
pub fn Card(children: Children, #[prop(default = "")] class: &'static str) -> impl IntoView {
    let class = if class.is_empty() {
        "card".to_string()
    } else {
        format!("card {}", class)
    };

    view! {
        <div class=class>
            {children()}
        </div>
    }
}

/// Page header component — consistent title + optional action area.
#[component]
pub fn PageHeader(
    title: &'static str,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="page-header">
            <h1 class="page-header__title">{title}</h1>
            {children.map(|c| view! { <div class="page-header__actions">{c()}</div> })}
        </div>
    }
}
