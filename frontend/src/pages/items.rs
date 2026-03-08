use crate::api::{
    HttpMethod, ItemEntry, PaginatedResponse, UserContext, authed_get, authed_request,
    is_admin_signal,
};
use crate::components::card::PageHeader;
use crate::components::icons::{Icon, IconKind};
use crate::components::modal::ConfirmModal;
use crate::components::toast::{toast_error, toast_success};
use crate::components::{LoadingSpinner, PaginationBar, input_handler};
use leptos::prelude::*;

/// Returns true if the price string is a valid positive decimal number
/// (digits with an optional decimal point, up to 2 decimal places).
fn is_valid_price(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Match digits with an optional single decimal point and up to 2 fractional digits.
    // Rejects scientific notation, negative signs, and other non-decimal formats.
    trimmed.len() <= 13
        && trimmed.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && trimmed.matches('.').count() <= 1
        && trimmed
            .find('.')
            .is_none_or(|dot| trimmed.len() - dot - 1 <= 2 && dot > 0)
}

#[component]
pub fn ItemsPage() -> impl IntoView {
    let (items, set_items) = signal(Vec::<ItemEntry>::new());
    let (loading, set_loading) = signal(true);
    let (show_create, set_show_create) = signal(false);
    let (delete_target, set_delete_target) = signal(Option::<(String, String)>::None);
    let (edit_target, set_edit_target) = signal(Option::<ItemEntry>::None);
    let (offset, set_offset) = signal(0usize);
    let (total, set_total) = signal(0usize);
    let limit = 50usize;

    let user = expect_context::<ReadSignal<Option<UserContext>>>();
    let is_admin = is_admin_signal(user);

    // Fetch items on mount
    let fetch_items = move |off: usize| {
        set_loading.set(true);
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/items?limit={}&offset={}", limit, off);
            if let Some(resp) = authed_get(&url).await
                && resp.ok()
            {
                match resp.json::<PaginatedResponse<ItemEntry>>().await {
                    Ok(data) => {
                        set_total.set(data.total as usize);
                        set_items.set(data.items);
                    }
                    Err(e) => {
                        web_sys::console::warn_1(&format!("items JSON parse error: {e}").into())
                    }
                }
            }
            set_loading.set(false);
        });
    };
    fetch_items(0);

    let do_create_item = move |descr: String, price: String| {
        let body = serde_json::json!({ "descr": descr, "price": price });
        leptos::task::spawn_local_scoped(async move {
            let resp = authed_request(HttpMethod::Post, "/api/v1.0/items", Some(&body)).await;
            match resp {
                Some(r) if r.ok() => match r.json::<ItemEntry>().await {
                    Ok(item) => {
                        set_items.update(|list| list.push(item));
                        toast_success("Item created");
                    }
                    Err(e) => web_sys::console::warn_1(
                        &format!("item create JSON parse error: {e}").into(),
                    ),
                },
                _ => toast_error("Failed to create item"),
            }
            set_show_create.set(false);
        });
    };

    let do_update_item = move |item_id: String, descr: String, price: String| {
        let body = serde_json::json!({ "descr": descr, "price": price });
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/items/{}", item_id);
            let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => match r.json::<ItemEntry>().await {
                    Ok(updated) => {
                        set_items.update(|list| {
                            if let Some(i) = list.iter_mut().find(|i| i.item_id == updated.item_id)
                            {
                                *i = updated;
                            }
                        });
                        toast_success("Item updated");
                    }
                    Err(e) => web_sys::console::warn_1(
                        &format!("item update JSON parse error: {e}").into(),
                    ),
                },
                _ => toast_error("Failed to update item"),
            }
            set_edit_target.set(None);
        });
    };

    let do_delete_item = move |item_id: String| {
        leptos::task::spawn_local_scoped(async move {
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
                                    let item = item.clone();

                                    view! {
                                        <tr class="connect-table-row">
                                            <td class="connect-table-cell">{descr}</td>
                                            <td class="connect-table-cell">{format!("{} kr", price)}</td>
                                            {move || is_admin.get().then(|| {
                                                let iid = iid.clone();
                                                let descr_del = descr_del.clone();
                                                let item_for_edit = item.clone();
                                                view! {
                                                    <td class="connect-table-cell connect-table-cell--actions">
                                                        <button
                                                            aria-label="Edit item"
                                                            class="connect-button connect-button--neutral connect-button--outline connect-button--small"
                                                            on:click=move |_| set_edit_target.set(Some(item_for_edit.clone()))
                                                        >
                                                            <span class="connect-button__content">
                                                                <span class="connect-button__icon">
                                                                    <Icon kind=IconKind::PenToSquare size=14 />
                                                                </span>
                                                            </span>
                                                        </button>
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
                        <PaginationBar
                            offset=offset
                            limit=limit
                            total=total
                            on_prev=move |off| { set_offset.set(off); fetch_items(off); }
                            on_next=move |off| { set_offset.set(off); fetch_items(off); }
                        />
                    </div>
                }.into_any()
            }}

            // Create item dialog
            <CreateItemDialog
                open=show_create.into()
                on_create=do_create_item
                on_cancel=move || set_show_create.set(false)
            />

            // Edit item dialog
            {move || {
                let target = edit_target.get();
                let open = Signal::derive(move || edit_target.get().is_some());
                if let Some(item) = target {
                    view! {
                        <EditItemDialog
                            open=open
                            item=item
                            on_save=do_update_item
                            on_cancel=move || set_edit_target.set(None)
                        />
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }
            }}

            // Delete confirmation
            {move || {
                let open = Signal::derive(move || delete_target.get().is_some());
                let (iid, iname) = delete_target.get().unwrap_or_default();
                let iid_clone = iid.clone();
                view! {
                    <ConfirmModal
                        open=open
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
    open: Signal<bool>,
    on_create: impl Fn(String, String) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (descr, set_descr) = signal(String::new());
    let (price, set_price) = signal(String::new());

    let reset = move || {
        set_descr.set(String::new());
        set_price.set(String::new());
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
                                        on:input=input_handler(set_descr)
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
                                        on:input=input_handler(set_price)
                                    />
                                </div>
                                {move || {
                                    let p = price.get();
                                    if !p.trim().is_empty() && !is_valid_price(&p) {
                                        view! { <div class="field-error">"Enter a valid price (e.g. 12.50)"</div> }.into_any()
                                    } else {
                                        view! { <span /> }.into_any()
                                    }
                                }}
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
                                disabled=move || descr.get().trim().is_empty() || !is_valid_price(&price.get())
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

#[component]
fn EditItemDialog(
    open: Signal<bool>,
    item: ItemEntry,
    on_save: impl Fn(String, String, String) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (descr, set_descr) = signal(item.descr.clone());
    let (price, set_price) = signal(item.price.clone());
    let item_id = item.item_id.clone();

    view! {
        {move || {
            if !open.get() {
                return view! { <div class="modal-hidden" /> }.into_any();
            }

            let on_save = on_save.clone();
            let on_cancel_bd = on_cancel.clone();
            let on_cancel_b = on_cancel.clone();
            let iid = item_id.clone();

            view! {
                <div class="modal-overlay" on:click=move |_| on_cancel_bd()>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h2 class="modal-title">"Edit Item"</h2>
                        </div>
                        <div class="modal-body">
                            <div class="connect-text-field">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="edit-item-descr">"Description"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="edit-item-descr"
                                        type="text"
                                        prop:value=move || descr.get()
                                        on:input=input_handler(set_descr)
                                    />
                                </div>
                            </div>
                            <div class="connect-text-field" style="margin-top: var(--ds-layout-spacing-200, 12px);">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="edit-item-price">"Price"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="edit-item-price"
                                        type="text"
                                        inputmode="decimal"
                                        prop:value=move || price.get()
                                        on:input=input_handler(set_price)
                                    />
                                </div>
                                {move || {
                                    let p = price.get();
                                    if !p.trim().is_empty() && !is_valid_price(&p) {
                                        view! { <div class="field-error">"Enter a valid price (e.g. 12.50)"</div> }.into_any()
                                    } else {
                                        view! { <span /> }.into_any()
                                    }
                                }}
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
                                disabled=move || descr.get().trim().is_empty() || !is_valid_price(&price.get())
                                on:click={
                                    let save = on_save.clone();
                                    let iid = iid.clone();
                                    move |_| save(iid.clone(), descr.get(), price.get())
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
