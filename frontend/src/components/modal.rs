use crate::components::icons::{Icon, IconKind};
use leptos::prelude::*;

/// Confirmation modal for destructive actions.
///
/// Uses CONNECT overlay CSS for the backdrop and a custom dialog panel.
/// Controlled via the `open` signal — set to `true` to show, `false` to hide.
#[component]
pub fn ConfirmModal(
    open: ReadSignal<bool>,
    title: String,
    message: String,
    #[prop(default = "Delete")] confirm_label: &'static str,
    #[prop(default = false)] destructive: bool,
    on_confirm: impl Fn() + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let confirm_btn_class = if destructive {
        "connect-button connect-button--negative connect-button--medium"
    } else {
        "connect-button connect-button--accent connect-button--medium"
    };

    let on_cancel_backdrop = on_cancel.clone();
    let on_cancel_btn = on_cancel;
    let on_confirm_btn = on_confirm;

    view! {
        {move || {
            if !open.get() {
                return view! { <div class="modal-hidden" /> }.into_any();
            }

            let title = title.clone();
            let message = message.clone();
            let on_cancel_bd = on_cancel_backdrop.clone();
            let on_cancel_b = on_cancel_btn.clone();
            let on_confirm_b = on_confirm_btn.clone();

            view! {
                <div class="modal-overlay" on:click=move |_| on_cancel_bd()>
                    <div class="modal-dialog" on:click=move |ev| {
                        // Prevent backdrop click from closing when clicking dialog
                        ev.stop_propagation();
                    }>
                        <div class="modal-header">
                            <h2 class="modal-title">{title}</h2>
                            <button
                                class="modal-close-btn"
                                aria-label="Close"
                                on:click={
                                    let cancel = on_cancel_b.clone();
                                    move |_| cancel()
                                }
                            >
                                <Icon kind=IconKind::CircleXmark size=20 />
                            </button>
                        </div>
                        <div class="modal-body">
                            <p>{message}</p>
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
                                class=confirm_btn_class
                                on:click=move |_| on_confirm_b()
                            >
                                <span class="connect-button__content">
                                    <span class="connect-button__label">{confirm_label}</span>
                                </span>
                            </button>
                        </div>
                    </div>
                </div>
            }.into_any()
        }}
    }
}
