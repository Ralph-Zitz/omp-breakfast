use crate::api::{
    HttpMethod, ItemEntry, OrderItemEntry, PaginatedResponse, TeamEntry, TeamOrderEntry,
    UserContext, authed_get, authed_request,
};
use crate::components::card::PageHeader;
use crate::components::icons::{Icon, IconKind};
use crate::components::modal::ConfirmModal;
use crate::components::toast::{toast_error, toast_success};
use crate::components::LoadingSpinner;
use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

#[component]
pub fn OrdersPage() -> impl IntoView {
    let user = expect_context::<ReadSignal<Option<UserContext>>>();

    // User's teams for team selector
    let (teams, set_teams) = signal(Vec::<TeamEntry>::new());
    let (selected_team, set_selected_team) = signal(Option::<String>::None);
    let (loading_teams, set_loading_teams) = signal(true);

    // Orders for selected team
    let (orders, set_orders) = signal(Vec::<TeamOrderEntry>::new());
    let (loading_orders, set_loading_orders) = signal(false);

    // Selected order detail
    let (selected_order, set_selected_order) = signal(Option::<TeamOrderEntry>::None);
    let (order_items, set_order_items) = signal(Vec::<OrderItemEntry>::new());
    let (loading_items, set_loading_items) = signal(false);

    // Available items catalog for adding to orders
    let (catalog_items, set_catalog_items) = signal(Vec::<ItemEntry>::new());

    // Create order dialog
    let (show_create_order, set_show_create_order) = signal(false);

    // Delete confirmation
    let (delete_target, set_delete_target) = signal(Option::<(String, String)>::None); // (order_id, label)

    let is_admin = Signal::derive(move || user.get().map(|u| u.is_admin).unwrap_or(false));

    // Fetch user's teams on mount
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(resp) = authed_get("/api/v1.0/teams").await {
            if resp.ok() {
                if let Ok(data) = resp.json::<PaginatedResponse<TeamEntry>>().await {
                    set_teams.set(data.items);
                }
            }
        }
        // Also fetch catalog items for the "add item" dropdown
        if let Some(resp) = authed_get("/api/v1.0/items").await {
            if resp.ok() {
                if let Ok(data) = resp.json::<PaginatedResponse<ItemEntry>>().await {
                    set_catalog_items.set(data.items);
                }
            }
        }
        set_loading_teams.set(false);
    });

    // Load orders when team is selected
    let load_orders = move |team_id: String| {
        set_selected_team.set(Some(team_id.clone()));
        set_selected_order.set(None);
        set_order_items.set(Vec::new());
        set_loading_orders.set(true);

        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/teams/{}/orders", team_id);
            if let Some(resp) = authed_get(&url).await {
                if resp.ok() {
                    if let Ok(data) = resp.json::<PaginatedResponse<TeamOrderEntry>>().await {
                        set_orders.set(data.items);
                    }
                }
            }
            set_loading_orders.set(false);
        });
    };

    // Load order items when an order is selected
    let load_order_items = move |team_id: String, order: TeamOrderEntry| {
        let order_id = order.teamorders_id.clone();
        set_selected_order.set(Some(order.clone()));
        set_loading_items.set(true);

        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/teams/{}/orders/{}/items", team_id, order_id);
            if let Some(resp) = authed_get(&url).await {
                if resp.ok() {
                    if let Ok(data) = resp.json::<PaginatedResponse<OrderItemEntry>>().await {
                        set_order_items.set(data.items);
                    }
                }
            }
            set_loading_items.set(false);
        });
    };

    let do_create_order = move |duedate: Option<String>| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        let body = match duedate {
            Some(d) if !d.is_empty() => serde_json::json!({ "duedate": d }),
            _ => serde_json::json!({}),
        };
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/teams/{}/orders", team_id);
            let resp = authed_request(HttpMethod::Post, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    if let Ok(order) = r.json::<TeamOrderEntry>().await {
                        set_orders.update(|list| list.push(order));
                        toast_success("Order created");
                    }
                }
                _ => toast_error("Failed to create order"),
            }
            set_show_create_order.set(false);
        });
    };

    let do_toggle_order_closed = move |order_id: String, currently_closed: bool| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        let body = serde_json::json!({ "closed": !currently_closed });
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id);
            let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    if let Ok(updated) = r.json::<TeamOrderEntry>().await {
                        set_orders.update(|list| {
                            if let Some(o) = list.iter_mut().find(|o| o.teamorders_id == updated.teamorders_id) {
                                *o = updated.clone();
                            }
                        });
                        if selected_order.get().map(|o| o.teamorders_id == updated.teamorders_id).unwrap_or(false) {
                            set_selected_order.set(Some(updated));
                        }
                        let msg = if currently_closed { "Order reopened" } else { "Order closed" };
                        toast_success(msg);
                    }
                }
                _ => toast_error("Failed to update order"),
            }
        });
    };

    let do_delete_order = move |order_id: String| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id);
            let resp = authed_request(HttpMethod::Delete, &url, None).await;
            match resp {
                Some(r) if r.ok() => {
                    set_orders.update(|list| list.retain(|o| o.teamorders_id != order_id));
                    // Clear detail panel if we deleted the selected order
                    if selected_order
                        .get()
                        .map(|o| o.teamorders_id == order_id)
                        .unwrap_or(false)
                    {
                        set_selected_order.set(None);
                        set_order_items.set(Vec::new());
                    }
                    toast_success("Order deleted");
                }
                _ => toast_error("Failed to delete order"),
            }
            set_delete_target.set(None);
        });
    };

    let do_add_item = move |item_id: String, amt: i32| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        let order = match selected_order.get() {
            Some(o) => o,
            None => return,
        };
        let order_id = order.teamorders_id.clone();
        let body = serde_json::json!({ "orders_item_id": item_id, "amt": amt });

        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/v1.0/teams/{}/orders/{}/items", team_id, order_id);
            let resp = authed_request(HttpMethod::Post, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => {
                    if let Ok(oi) = r.json::<OrderItemEntry>().await {
                        set_order_items.update(|list| list.push(oi));
                        toast_success("Item added to order");
                    }
                }
                _ => toast_error("Failed to add item"),
            }
        });
    };

    let do_remove_item = move |item_id: String| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        let order = match selected_order.get() {
            Some(o) => o,
            None => return,
        };
        let order_id = order.teamorders_id.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let url = format!(
                "/api/v1.0/teams/{}/orders/{}/items/{}",
                team_id, order_id, item_id
            );
            let resp = authed_request(HttpMethod::Delete, &url, None).await;
            match resp {
                Some(r) if r.ok() => {
                    set_order_items.update(|list| list.retain(|i| i.orders_item_id != item_id));
                    toast_success("Item removed");
                }
                _ => toast_error("Failed to remove item"),
            }
        });
    };

    view! {
        <div class="orders-page">
            <PageHeader title="Orders">
                {move || {
                    if selected_team.get().is_some() {
                        view! {
                            <button
                                class="connect-button connect-button--accent connect-button--small"
                                on:click=move |_| set_show_create_order.set(true)
                            >
                                <span class="connect-button__content">
                                    <span class="connect-button__icon">
                                        <Icon kind=IconKind::CirclePlus size=16 />
                                    </span>
                                    <span class="connect-button__label">"New Order"</span>
                                </span>
                            </button>
                        }.into_any()
                    } else {
                        view! { <span /> }.into_any()
                    }
                }}
            </PageHeader>

            {move || {
                if loading_teams.get() {
                    return view! { <LoadingSpinner /> }.into_any();
                }

                let team_list = teams.get();
                if team_list.is_empty() {
                    return view! {
                        <div class="empty-state">
                            <Icon kind=IconKind::ClipboardList size=48 />
                            <p>"No teams available. Join a team to start ordering."</p>
                        </div>
                    }.into_any();
                }

                view! {
                    // Team selector
                    <div class="card" style="margin-bottom: var(--ds-layout-spacing-300, 16px);">
                        <div class="section-title">"Select Team"</div>
                        <div class="team-selector">
                            {team_list.into_iter().map(|team| {
                                let tid = team.team_id.clone();
                                let tid2 = team.team_id.clone();
                                let name = team.tname.clone();
                                let load = load_orders.clone();
                                view! {
                                    <button
                                        class=move || {
                                            let base = "connect-button connect-button--medium";
                                            if selected_team.get().as_ref() == Some(&tid2) {
                                                format!("{} connect-button--accent", base)
                                            } else {
                                                format!("{} connect-button--neutral connect-button--outline", base)
                                            }
                                        }
                                        on:click=move |_| load(tid.clone())
                                    >
                                        <span class="connect-button__content">
                                            <span class="connect-button__label">{name}</span>
                                        </span>
                                    </button>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>

                    // Orders list + detail split
                    <div class="content-split">
                        <div class="content-split__primary">
                            <OrdersList
                                orders=orders
                                loading=loading_orders
                                selected_order=selected_order
                                selected_team=selected_team
                                on_select=load_order_items.clone()
                                on_delete=move |oid: String, label: String| set_delete_target.set(Some((oid, label)))
                                on_toggle_closed=do_toggle_order_closed
                            />
                        </div>
                        <div class="content-split__secondary">
                            <OrderDetail
                                order=selected_order
                                items=order_items
                                catalog=catalog_items
                                loading=loading_items
                                is_admin=is_admin
                                on_add_item=do_add_item
                                on_remove_item=do_remove_item
                            />
                        </div>
                    </div>
                }.into_any()
            }}

            <CreateOrderDialog
                open=show_create_order
                on_create=do_create_order
                on_cancel=move || set_show_create_order.set(false)
            />

            {move || {
                let target = delete_target.get();
                let (del_open, _) = signal(target.is_some());
                let (oid, label) = target.unwrap_or_default();
                let oid_clone = oid.clone();
                view! {
                    <ConfirmModal
                        open=del_open
                        title=format!("Delete Order")
                        message=format!("Are you sure you want to delete order {}?", label)
                        confirm_label="Delete"
                        destructive=true
                        on_confirm=move || do_delete_order(oid_clone.clone())
                        on_cancel=move || set_delete_target.set(None)
                    />
                }
            }}
        </div>
    }
}

#[component]
fn OrdersList(
    orders: ReadSignal<Vec<TeamOrderEntry>>,
    loading: ReadSignal<bool>,
    selected_order: ReadSignal<Option<TeamOrderEntry>>,
    selected_team: ReadSignal<Option<String>>,
    on_select: impl Fn(String, TeamOrderEntry) + 'static + Clone + Send,
    on_delete: impl Fn(String, String) + 'static + Clone + Send,
    on_toggle_closed: impl Fn(String, bool) + 'static + Clone + Send,
) -> impl IntoView {
    let user = expect_context::<ReadSignal<Option<UserContext>>>();
    view! {
        {move || {
            if loading.get() {
                return view! { <LoadingSpinner /> }.into_any();
            }

            let team_id = match selected_team.get() {
                Some(id) => id,
                None => return view! {
                    <div class="empty-state">
                        <Icon kind=IconKind::ClipboardList size=40 />
                        <p>"Select a team to view orders."</p>
                    </div>
                }.into_any(),
            };

            let order_list = orders.get();
            if order_list.is_empty() {
                return view! {
                    <div class="empty-state">
                        <Icon kind=IconKind::ClipboardList size=40 />
                        <p>"No orders yet for this team."</p>
                    </div>
                }.into_any();
            }

            view! {
                <div class="card">
                    <table class="connect-table connect-table--medium">
                        <thead class="connect-table-header">
                            <tr>
                                <th class="connect-table-header-cell">"Due Date"</th>
                                <th class="connect-table-header-cell">"Status"</th>
                                <th class="connect-table-header-cell connect-table-header-cell--actions">"Actions"</th>
                            </tr>
                        </thead>
                        <tbody class="connect-table-body">
                            {order_list.into_iter().map(|order| {
                                let oid = order.teamorders_id.clone();
                                let oid_del = order.teamorders_id.clone();
                                let order_owner_id = order.teamorders_user_id.clone();
                                let order_team_id = order.teamorders_team_id.clone();
                                let due = order.duedate.clone().unwrap_or_else(|| "No date".to_string());
                                let due_label = due.clone();
                                let closed = order.closed;
                                let is_selected = move || {
                                    selected_order.get().as_ref().map(|o| o.teamorders_id.as_str()) == Some(oid.as_str())
                                };
                                let can_delete = move || {
                                    let ctx = user.get();
                                    match ctx {
                                        Some(ref u) if u.is_admin => true,
                                        Some(ref u) if u.user_id == order_owner_id => true,
                                        Some(ref u) => u.teams.iter().any(|t| t.team_id == order_team_id && t.title == "Team Admin"),
                                        None => false,
                                    }
                                };
                                let team_id_click = team_id.clone();
                                let order_click = order.clone();
                                let on_select = on_select.clone();
                                let on_delete = on_delete.clone();
                                let on_toggle = on_toggle_closed.clone();

                                view! {
                                    <tr
                                        class=move || if is_selected() { "connect-table-row connect-table-row--selected" } else { "connect-table-row" }
                                        style="cursor: pointer;"
                                        on:click={
                                            let order_click = order_click.clone();
                                            let team_id_click = team_id_click.clone();
                                            let on_select = on_select.clone();
                                            move |_| on_select(team_id_click.clone(), order_click.clone())
                                        }
                                    >
                                        <td class="connect-table-cell">{due.clone()}</td>
                                        <td class="connect-table-cell">
                                            {if closed {
                                                view! {
                                                    <span class="connect-tag connect-tag--small connect-tag--neutral-default">"Closed"</span>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <span class="connect-tag connect-tag--small connect-tag--positive-default">"Open"</span>
                                                }.into_any()
                                            }}
                                        </td>
                                        <td class="connect-table-cell connect-table-cell--actions">
                                            {move || can_delete().then(|| {
                                                let oid_toggle = oid_del.clone();
                                                let oid_del2 = oid_del.clone();
                                                let due_label = due_label.clone();
                                                let on_delete = on_delete.clone();
                                                let on_toggle = on_toggle.clone();
                                                view! {
                                                    <button
                                                        aria-label=if closed { "Reopen order" } else { "Close order" }
                                                        class="connect-button connect-button--neutral connect-button--outline connect-button--small"
                                                        on:click=move |ev| {
                                                            ev.stop_propagation();
                                                            on_toggle(oid_toggle.clone(), closed);
                                                        }
                                                    >
                                                        <span class="connect-button__content">
                                                            <span class="connect-button__label">
                                                                {if closed { "Reopen" } else { "Close" }}
                                                            </span>
                                                        </span>
                                                    </button>
                                                    <button
                                                        aria-label="Delete order"
                                                        class="connect-button connect-button--negative connect-button--outline connect-button--small"
                                                        on:click=move |ev| {
                                                            ev.stop_propagation();
                                                            on_delete(oid_del2.clone(), due_label.clone());
                                                        }
                                                    >
                                                        <span class="connect-button__content">
                                                            <span class="connect-button__icon">
                                                                <Icon kind=IconKind::Trash size=14 />
                                                            </span>
                                                        </span>
                                                    </button>
                                                }
                                            })}
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()}
                        </tbody>
                    </table>
                </div>
            }.into_any()
        }}
    }
}

#[component]
fn OrderDetail(
    order: ReadSignal<Option<TeamOrderEntry>>,
    items: ReadSignal<Vec<OrderItemEntry>>,
    catalog: ReadSignal<Vec<ItemEntry>>,
    loading: ReadSignal<bool>,
    #[allow(unused)] is_admin: Signal<bool>,
    on_add_item: impl Fn(String, i32) + 'static + Clone + Send,
    on_remove_item: impl Fn(String) + 'static + Clone + Send,
) -> impl IntoView {
    let (add_item_id, set_add_item_id) = signal(String::new());
    let (add_qty, set_add_qty) = signal("1".to_string());

    view! {
        {move || {
            let ord = match order.get() {
                Some(o) => o,
                None => return view! {
                    <div class="empty-state">
                        <p class="text-muted">"Select an order to see its items."</p>
                    </div>
                }.into_any(),
            };

            if loading.get() {
                return view! { <LoadingSpinner /> }.into_any();
            }

            let closed = ord.closed;
            let item_list = items.get();
            let cat = catalog.get();

            // Resolve item names from catalog
            let resolve_name = move |item_id: &str| -> String {
                cat.iter()
                    .find(|i| i.item_id == item_id)
                    .map(|i| i.descr.clone())
                    .unwrap_or_else(|| item_id.to_string())
            };

            view! {
                <div class="card">
                    <div class="section-title">
                        "Order Items"
                        {if closed {
                            view! {
                                <span class="connect-tag connect-tag--small connect-tag--neutral-default" style="margin-left: 8px;">"Closed"</span>
                            }.into_any()
                        } else {
                            view! { <span /> }.into_any()
                        }}
                    </div>

                    {if item_list.is_empty() {
                        view! {
                            <p class="text-muted">"No items in this order yet."</p>
                        }.into_any()
                    } else {
                        view! {
                            <table class="connect-table connect-table--small">
                                <thead class="connect-table-header">
                                    <tr>
                                        <th class="connect-table-header-cell">"Item"</th>
                                        <th class="connect-table-header-cell">"Qty"</th>
                                        {(!closed).then(|| view! {
                                            <th class="connect-table-header-cell connect-table-header-cell--actions">"Remove"</th>
                                        })}
                                    </tr>
                                </thead>
                                <tbody class="connect-table-body">
                                    {item_list.into_iter().map(|oi| {
                                        let name = resolve_name(&oi.orders_item_id);
                                        let iid = oi.orders_item_id.clone();
                                        let on_remove_item = on_remove_item.clone();
                                        view! {
                                            <tr class="connect-table-row">
                                                <td class="connect-table-cell">{name}</td>
                                                <td class="connect-table-cell">{oi.amt}</td>
                                                {(!closed).then(|| {
                                                    let iid = iid.clone();
                                                    let on_remove_item = on_remove_item.clone();
                                                    view! {
                                                        <td class="connect-table-cell connect-table-cell--actions">
                                                            <button
                                                                aria-label="Remove item from order"
                                                                class="connect-button connect-button--negative connect-button--outline connect-button--small"
                                                                on:click=move |_| on_remove_item(iid.clone())
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
                        }.into_any()
                    }}

                    // Add item form (only if order is open)
                    {if !closed {
                        let cat_for_select = catalog.get();
                        let on_add_item = on_add_item.clone();
                        view! {
                            <div class="add-item-form" style="margin-top: var(--ds-layout-spacing-300, 16px); display: flex; gap: var(--ds-layout-spacing-200, 8px); align-items: flex-end;">
                                <div class="connect-text-field" style="flex: 1;">
                                    <div class="connect-label">
                                        <label class="connect-label__text" for="add-item-select">"Item"</label>
                                    </div>
                                    <select
                                        id="add-item-select"
                                        class="connect-text-field__input"
                                        prop:value=move || add_item_id.get()
                                        on:change=move |ev| {
                                            let Some(target) = ev.target() else { return; };
                                            set_add_item_id.set(target.unchecked_into::<web_sys::HtmlSelectElement>().value());
                                        }
                                    >
                                        <option value="">"Select an item..."</option>
                                        {cat_for_select.into_iter().map(|item| {
                                            let iid = item.item_id.clone();
                                            view! {
                                                <option value=iid>{format!("{} ({} kr)", item.descr, item.price)}</option>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </select>
                                </div>
                                <div class="connect-text-field" style="width: 80px;">
                                    <div class="connect-label">
                                        <label class="connect-label__text" for="add-item-qty">"Qty"</label>
                                    </div>
                                    <div class="connect-text-field__input-wrapper">
                                        <input
                                            class="connect-text-field__input"
                                            id="add-item-qty"
                                            type="number"
                                            min="1"
                                            prop:value=move || add_qty.get()
                                            on:input=move |ev| {
                                                let Some(target) = ev.target() else { return; };
                                                set_add_qty.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
                                            }
                                        />
                                    </div>
                                </div>
                                <button
                                    class="connect-button connect-button--accent connect-button--small"
                                    disabled=move || add_item_id.get().is_empty()
                                    on:click={
                                        let on_add_item = on_add_item.clone();
                                        move |_| {
                                            let qty: i32 = add_qty.get().parse().unwrap_or(1);
                                            on_add_item(add_item_id.get(), qty);
                                            set_add_item_id.set(String::new());
                                            set_add_qty.set("1".to_string());
                                        }
                                    }
                                >
                                    <span class="connect-button__content">
                                        <span class="connect-button__icon">
                                            <Icon kind=IconKind::CirclePlus size=14 />
                                        </span>
                                        <span class="connect-button__label">"Add"</span>
                                    </span>
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! { <span /> }.into_any()
                    }}
                </div>
            }.into_any()
        }}
    }
}

#[component]
fn CreateOrderDialog(
    open: ReadSignal<bool>,
    on_create: impl Fn(Option<String>) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (duedate, set_duedate) = signal(String::new());

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
                            <h2 class="modal-title">"New Order"</h2>
                        </div>
                        <div class="modal-body">
                            <div class="connect-text-field">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="order-due">"Due Date (optional)"</label>
                                </div>
                                <div class="connect-text-field__input-wrapper">
                                    <input
                                        class="connect-text-field__input"
                                        id="order-due"
                                        type="date"
                                        prop:value=move || duedate.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return; };
                                            set_duedate.set(target.unchecked_into::<web_sys::HtmlInputElement>().value());
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
                                on:click={
                                    let create = on_create.clone();
                                    move |_| {
                                        let d = duedate.get();
                                        create(if d.is_empty() { None } else { Some(d) });
                                        set_duedate.set(String::new());
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
