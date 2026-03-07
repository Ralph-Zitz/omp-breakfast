use crate::api::sleep_ms;
use crate::components::icons::{Icon, IconKind};
use leptos::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

static TOAST_COUNTER: AtomicU32 = AtomicU32::new(0);

fn next_toast_id() -> u32 {
    TOAST_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ── Toast types ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToastVariant {
    Success,
    Negative,
    Warning,
    Informative,
}

impl ToastVariant {
    fn css_modifier(self) -> &'static str {
        match self {
            Self::Success => "connect-toast--success",
            Self::Negative => "connect-toast--negative",
            Self::Warning => "connect-toast--warning",
            Self::Informative => "connect-toast--informative",
        }
    }

    fn icon_kind(self) -> IconKind {
        match self {
            Self::Success => IconKind::CircleCheck,
            Self::Negative => IconKind::CircleXmark,
            Self::Warning => IconKind::TriangleExclamation,
            Self::Informative => IconKind::CircleInfo,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u32,
    pub title: String,
    pub variant: ToastVariant,
}

// ── Toast context ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct ToastContext {
    toasts: ReadSignal<Vec<Toast>>,
    set_toasts: WriteSignal<Vec<Toast>>,
}

impl Default for ToastContext {
    fn default() -> Self {
        let (toasts, set_toasts) = signal(Vec::<Toast>::new());
        Self { toasts, set_toasts }
    }
}

impl ToastContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a toast and auto-dismiss after 5 seconds.
    pub fn push(&self, title: impl Into<String>, variant: ToastVariant) {
        let id = next_toast_id();
        let toast = Toast {
            id,
            title: title.into(),
            variant,
        };
        self.set_toasts.update(|list| list.push(toast));

        let set_toasts = self.set_toasts;
        wasm_bindgen_futures::spawn_local(async move {
            sleep_ms(5000).await;
            set_toasts.update(|list| list.retain(|t| t.id != id));
        });
    }

    /// Dismiss a toast immediately by ID.
    pub fn dismiss(&self, id: u32) {
        self.set_toasts.update(|list| list.retain(|t| t.id != id));
    }
}

/// Convenience: push a success toast via context.
pub fn toast_success(msg: impl Into<String>) {
    if let Some(ctx) = use_context::<ToastContext>() {
        ctx.push(msg, ToastVariant::Success);
    }
}

/// Convenience: push an error toast via context.
pub fn toast_error(msg: impl Into<String>) {
    if let Some(ctx) = use_context::<ToastContext>() {
        ctx.push(msg, ToastVariant::Negative);
    }
}

// ── Toast region (rendered at app root) ─────────────────────────────────────

#[component]
pub fn ToastRegion() -> impl IntoView {
    let ctx = expect_context::<ToastContext>();

    view! {
        <div class="toast-region">
            {move || {
                ctx.toasts.get().into_iter().map(|toast| {
                    let id = toast.id;
                    let variant_class = toast.variant.css_modifier();
                    let icon_kind = toast.variant.icon_kind();
                    let class = format!("connect-toast {}", variant_class);
                    let title = toast.title.clone();

                    view! {
                        <div class=class role="status" aria-live="polite">
                            <div class="connect-toast__icon">
                                <Icon kind=icon_kind size=20 />
                            </div>
                            <div class="connect-toast__content-wrapper">
                                <div class="connect-toast__content">
                                    <span class="connect-toast-title">{title}</span>
                                </div>
                            </div>
                            <div class="connect-toast__close-button-wrapper">
                                <button
                                    class="toast-dismiss-btn"
                                    aria-label="Dismiss"
                                    on:click=move |_| ctx.dismiss(id)
                                >
                                    <Icon kind=IconKind::CircleXmark size=16 />
                                </button>
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()
            }}
        </div>
    }
}
