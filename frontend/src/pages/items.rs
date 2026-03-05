use crate::api::{HttpMethod, ItemEntry, PaginatedResponse, UserContext, authed_get, authed_request};
use crate::components::card::PageHeader;
use crate::components::icons::{Icon, IconKind};
use crate::components::modal::ConfirmModal;
use crate::components::toast::{toast_error, toast_success};
use crate::components::LoadingSpinner;
use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

#[component]
pub fn ItemsPage() -> impl IntoView {
    let (items, set_items) = signal(Vec::<ItemEntry>::new());
    let (loading, set_loading) = signal(true);
    let (show_create, set_show_create) = signal(false);
    let (delete_target, set_delete_target) = signal(Option::<(String, String)>::None);

    let user = expect_context::<ReadSignal<Option<UserContext>>>();
    let is_admin = Signal::derive(move || user.get().map(|u| u.is_admin).unwrap_or(false));

    // Fetch items on mount
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(resp) = authed_get("/api/v1.0/items").await {
            if resp.ok() {
                if let Ok(data) = resp.json::<PaginatedResponse<ItemEntry>>().await {
                    set_items.set(data.items);
                }
            }
        }
        set_loading.set(false);
    });

    let do_create_item = move |descr: String, price: String| {
        let body = serde_json::json!({ "descr": descr, "price": price });
        wasm_bindgen_futures::spawn_local(async move {
            let resp = authed_request(HttpMethod::Post, "/api/v1.0/items", Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    if let Ok(item) = r.json::<ItemEntry>().await {
                        set_items.update(|list| list.push(item));
                        toast_success("Item created");
                    }
                }
                _ => toast_error("Failed to create item"),
            }
            set_show_create.set(false);
        });
    };

    let do_delete_item = move |item_id: String| {
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/items/{}", item_id);
            let resp = authed_request(HttpMethod::Delete, &url, None).await;
            match resp {
                Some(r) if r.ok() => {
                    set_items.update(|list| list.retain(|i| i.item_id != item_id));
                    toast_success("Item deleted");
                }
                _ => toast_error("Failed to delete item"),
            }
            set_delete_target.set(None);
        });
    };

    view! {
        <div class="items-page">
            <PageHeader title="Item Catalog">
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
                                    <span class="connect-button__label">"New Item"</span>
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

                let item_list = items.get();
                if item_list.is_empty() {
                    return view! {
                        <div class="empty-state">
                            <Icon kind=IconKind::Tag size=48 />
                            <p>"No items in the catalog."</p>
                        </div>
                    }.into_any();
                }

                view! {
                    <div class="card">
                        <table class="connect-table connect-table--medium">
                            <thead class="connect-table-header">
                                <tr>
                                    <th class="connect-table-header-cell">"Item"</th>
                                    <th class="connect-table-header-cell">"Price"</th>
                                    {move || is_admin.get().then(|| view! {
                                        <th class="connect-table-header-cell connect-table-header-cell--actions">"Actions"</th>
                                    })}
                                </tr>
                            </thead>
                            <tbody class="connect-table-body">
                                {item_list.into_iter().map(|item| {
                                    let iid = item.item_id.clone();
                                    let descr = item.descr.clone();
                                    let descr_del = item.descr.clone();
                                    let price = item.price.clone();

                                    view! {
                                        <tr class="connect-table-row">
                                            <td class="connect-table-cell">{descr}</td>
                                            <td class="connect-table-cell">{format!("{} kr", price)}</td>
                                            {move || is_admin.get().then(|| {
                                                let iid = iid.clone();
                                                let descr_del = descr_del.clone();
                                                view! {
                                                    <td class="connect-table-cell connect-table-cell--actions">
                                                        <button
                                                            aria-label="Delete item"
                                                            class="connect-button connect-button--negative connect-button--outline connect-button--small"
                                                            on:click=move |_| {
                                                                set_delete_target.set(Some((iid.clone(), descr_del.clone())));
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
                }.into_any()
            }}

            // Create item dialog
            <CreateItemDialog
                open=show_create
                on_create=do_create_item
                on_cancel=move || set_show_create.set(false)
            />

            // Delete confirmation
            {move || {
                let target = delete_target.get();
                let (del_open, _) = signal(target.is_some());
                let (iid, iname) = target.unwrap_or_default();
                let iid_clone = iid.clone();
                view! {
                    <ConfirmModal
                        open=del_open
                        title="Delete Item".to_string()
                        message=format!("Are you sure you want to delete \"{}\"?", iname)
                        confirm_label="Delete"
                        destructive=true
                        on_confirm=move || do_delete_item(iid_clone.clone())
                        on_cancel=move || set_delete_target.set(None)
                    />
                }
            }}
        </div>
    }
}

#[component]
fn CreateItemDialog(
    open: ReadSignal<bool>,
    on_create: impl Fn(String, String) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (descr, set_descr) = signal(String::new());
    let (price, set_price) = signal(String::new());

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
                            <h2 class="modal-title">"New Item"</h2>
                        </div>
                        <div class="modal-body">
                            <div class="connect-text-field">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="item-descr">"Description"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="item-descr"
                                        type="text"
                                        placeholder="Item name"
                                        prop:value=move || descr.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return; };
                                            set_descr.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                        }
                                    />
                                </div>
                            </div>
                            <div class="connect-text-field" style="margin-top: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="item-price">"Price"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="item-price"
                                        type="text"
                                        placeholder="0.00"
                                        inputmode="decimal"
                                        prop:value=move || price.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return; };
                                            set_price.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
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
                                disabled=move || descr.get().trim().is_empty() || price.get().trim().is_empty()
                                on:click={
                                    let create = on_create.clone();
                                    move |_| {
                                        create(descr.get(), price.get());
                                        set_descr.set(String::new());
                                        set_price.set(String::new());
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
