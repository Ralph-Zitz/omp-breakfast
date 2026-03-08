use crate::api::{
    HttpMethod, ItemEntry, OrderItemEntry, PaginatedResponse, TeamOrderEntry, UserContext,
    UserInTeams, UsersInTeam, authed_get, authed_request,
};
use crate::components::LoadingSpinner;
use crate::components::card::PageHeader;
use crate::components::icons::{Icon, IconKind};
use crate::components::modal::ConfirmModal;
use crate::components::toast::{toast_error, toast_success};
use leptos::prelude::*;

#[path = "order_components.rs"]
mod order_components;
use order_components::{CreateOrderDialog, OrderDetail};

#[component]
pub fn OrdersPage() -> impl IntoView {
    let user = expect_context::<ReadSignal<Option<UserContext>>>();

    // User's teams for team selector
    let (teams, set_teams) = signal(Vec::<UserInTeams>::new());
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

    // Team members for pickup user selection
    let (team_members, set_team_members) = signal(Vec::<UsersInTeam>::new());

    // Create order dialog
    let (show_create_order, set_show_create_order) = signal(false);

    // Delete confirmation
    let (delete_target, set_delete_target) = signal(Option::<(String, String)>::None); // (order_id, label)

    let _is_admin = crate::api::is_admin_signal(user);

    // Fetch user's teams on mount
    leptos::task::spawn_local_scoped(async move {
        let user_id = user
            .get_untracked()
            .map(|u| u.user_id.clone())
            .unwrap_or_default();
        let teams_url = format!("/api/v1.0/users/{}/teams", user_id);
        if let Some(resp) = authed_get(&teams_url).await
            && resp.ok()
        {
            match resp.json::<PaginatedResponse<UserInTeams>>().await {
                Ok(data) => set_teams.set(data.items),
                Err(e) => web_sys::console::warn_1(&format!("teams JSON parse error: {e}").into()),
            }
        }
        // Also fetch catalog items for the "add item" dropdown
        if let Some(resp) = authed_get("/api/v1.0/items").await
            && resp.ok()
        {
            match resp.json::<PaginatedResponse<ItemEntry>>().await {
                Ok(data) => set_catalog_items.set(data.items),
                Err(e) => {
                    web_sys::console::warn_1(&format!("catalog JSON parse error: {e}").into())
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

        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}/orders", team_id);
            if let Some(resp) = authed_get(&url).await
                && resp.ok()
            {
                match resp.json::<PaginatedResponse<TeamOrderEntry>>().await {
                    Ok(data) => set_orders.set(data.items),
                    Err(e) => {
                        web_sys::console::warn_1(&format!("orders JSON parse error: {e}").into())
                    }
                }
            }
            // Fetch team members for pickup user dropdown
            let members_url = format!("/api/v1.0/teams/{}/users", team_id);
            if let Some(resp) = authed_get(&members_url).await
                && resp.ok()
            {
                match resp.json::<PaginatedResponse<UsersInTeam>>().await {
                    Ok(data) => set_team_members.set(data.items),
                    Err(e) => web_sys::console::warn_1(
                        &format!("team members JSON parse error: {e}").into(),
                    ),
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

        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}/orders/{}/items", team_id, order_id);
            if let Some(resp) = authed_get(&url).await
                && resp.ok()
            {
                match resp.json::<PaginatedResponse<OrderItemEntry>>().await {
                    Ok(data) => set_order_items.set(data.items),
                    Err(e) => web_sys::console::warn_1(
                        &format!("order items JSON parse error: {e}").into(),
                    ),
                }
            }
            set_loading_items.set(false);
        });
    };

    let do_create_order = move |duedate: Option<String>, pickup_user_id: Option<String>| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        let mut body = serde_json::Map::new();
        if let Some(d) = duedate
            && !d.is_empty()
        {
            body.insert("duedate".into(), serde_json::Value::String(d));
        }
        if let Some(pid) = pickup_user_id
            && !pid.is_empty()
        {
            body.insert("pickup_user_id".into(), serde_json::Value::String(pid));
        }
        let body = serde_json::Value::Object(body);
        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}/orders", team_id);
            let resp = authed_request(HttpMethod::Post, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => match r.json::<TeamOrderEntry>().await {
                    Ok(order) => {
                        set_orders.update(|list| list.push(order));
                        toast_success("Order created");
                    }
                    Err(e) => web_sys::console::warn_1(
                        &format!("order create JSON parse error: {e}").into(),
                    ),
                },
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
        if currently_closed {
            // Reopen: duplicate the old order via the /reopen endpoint
            leptos::task::spawn_local_scoped(async move {
                let url = format!("/api/v1.0/teams/{}/orders/{}/reopen", team_id, order_id);
                let resp = authed_request(HttpMethod::Post, &url, None).await;
                match resp {
                    Some(r) if r.ok() => match r.json::<TeamOrderEntry>().await {
                        Ok(new_order) => {
                            set_orders.update(|list| list.insert(0, new_order));
                            toast_success("Order reopened as a new order");
                        }
                        Err(e) => web_sys::console::warn_1(
                            &format!("order reopen JSON parse error: {e}").into(),
                        ),
                    },
                    _ => toast_error("Failed to reopen order"),
                }
            });
        } else {
            // Close the order
            let body = serde_json::json!({ "closed": true });
            leptos::task::spawn_local_scoped(async move {
                let url = format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id);
                let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
                match resp {
                    Some(r) if r.ok() => match r.json::<TeamOrderEntry>().await {
                        Ok(updated) => {
                            set_orders.update(|list| {
                                if let Some(o) = list
                                    .iter_mut()
                                    .find(|o| o.teamorders_id == updated.teamorders_id)
                                {
                                    *o = updated.clone();
                                }
                            });
                            if selected_order
                                .get()
                                .map(|o| o.teamorders_id == updated.teamorders_id)
                                .unwrap_or(false)
                            {
                                set_selected_order.set(Some(updated));
                            }
                            toast_success("Order closed");
                        }
                        Err(e) => web_sys::console::warn_1(
                            &format!("order close JSON parse error: {e}").into(),
                        ),
                    },
                    _ => toast_error("Failed to close order"),
                }
            });
        }
    };

    let do_delete_order = move |order_id: String| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        leptos::task::spawn_local_scoped(async move {
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

        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}/orders/{}/items", team_id, order_id);
            let resp = authed_request(HttpMethod::Post, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => match r.json::<OrderItemEntry>().await {
                    Ok(oi) => {
                        set_order_items.update(|list| list.push(oi));
                        toast_success("Item added to order");
                    }
                    Err(e) => web_sys::console::warn_1(
                        &format!("order item JSON parse error: {e}").into(),
                    ),
                },
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

        leptos::task::spawn_local_scoped(async move {
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

    let do_update_item = move |item_id: String, new_amt: i32| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        let order = match selected_order.get() {
            Some(o) => o,
            None => return,
        };
        let order_id = order.teamorders_id.clone();
        let body = serde_json::json!({ "orders_item_id": item_id, "amt": new_amt });

        leptos::task::spawn_local_scoped(async move {
            let url = format!(
                "/api/v1.0/teams/{}/orders/{}/items/{}",
                team_id, order_id, item_id
            );
            let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => match r.json::<OrderItemEntry>().await {
                    Ok(updated) => {
                        set_order_items.update(|list| {
                            if let Some(oi) = list
                                .iter_mut()
                                .find(|i| i.orders_item_id == updated.orders_item_id)
                            {
                                *oi = updated;
                            }
                        });
                        toast_success("Quantity updated");
                    }
                    Err(e) => web_sys::console::warn_1(
                        &format!("order item update JSON parse error: {e}").into(),
                    ),
                },
                _ => toast_error("Failed to update item"),
            }
        });
    };

    let do_update_duedate = move |duedate: Option<String>| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        let order = match selected_order.get() {
            Some(o) => o,
            None => return,
        };
        let order_id = order.teamorders_id.clone();
        let body = match duedate {
            Some(d) if !d.is_empty() => serde_json::json!({ "duedate": d }),
            _ => serde_json::json!({ "duedate": null }),
        };

        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id);
            let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => match r.json::<TeamOrderEntry>().await {
                    Ok(updated) => {
                        set_orders.update(|list| {
                            if let Some(o) = list
                                .iter_mut()
                                .find(|o| o.teamorders_id == updated.teamorders_id)
                            {
                                *o = updated.clone();
                            }
                        });
                        set_selected_order.set(Some(updated));
                        toast_success("Due date updated");
                    }
                    Err(e) => web_sys::console::warn_1(
                        &format!("duedate update JSON parse error: {e}").into(),
                    ),
                },
                _ => toast_error("Failed to update due date"),
            }
        });
    };

    let do_assign_pickup = move |pickup_user_id: Option<String>| {
        let team_id = match selected_team.get() {
            Some(id) => id,
            None => return,
        };
        let order = match selected_order.get() {
            Some(o) => o,
            None => return,
        };
        let order_id = order.teamorders_id.clone();
        let body = match pickup_user_id {
            Some(pid) if !pid.is_empty() => {
                serde_json::json!({ "pickup_user_id": pid })
            }
            _ => serde_json::json!({ "pickup_user_id": null }),
        };

        leptos::task::spawn_local_scoped(async move {
            let url = format!("/api/v1.0/teams/{}/orders/{}", team_id, order_id);
            let resp = authed_request(HttpMethod::Put, &url, Some(&body)).await;
            match resp {
                Some(r) if r.ok() => match r.json::<TeamOrderEntry>().await {
                    Ok(updated) => {
                        set_orders.update(|list| {
                            if let Some(o) = list
                                .iter_mut()
                                .find(|o| o.teamorders_id == updated.teamorders_id)
                            {
                                *o = updated.clone();
                            }
                        });
                        set_selected_order.set(Some(updated));
                        toast_success("Pickup person updated");
                    }
                    Err(e) => web_sys::console::warn_1(
                        &format!("pickup update JSON parse error: {e}").into(),
                    ),
                },
                _ => toast_error("Failed to update pickup person"),
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
                                let load = load_orders;
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
                                team_members=team_members
                                on_select=load_order_items
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
                                team_members=team_members
                                on_add_item=do_add_item
                                on_update_item=do_update_item
                                on_remove_item=do_remove_item
                                on_assign_pickup=do_assign_pickup
                                on_update_duedate=do_update_duedate
                            />
                        </div>
                    </div>
                }.into_any()
            }}

            <CreateOrderDialog
                open=show_create_order.into()
                team_members=team_members
                on_create=do_create_order
                on_cancel=move || set_show_create_order.set(false)
            />

            {move || {
                let open = Signal::derive(move || delete_target.get().is_some());
                let (oid, label) = delete_target.get().unwrap_or_default();
                let oid_clone = oid.clone();
                view! {
                    <ConfirmModal
                        open=open
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
    team_members: ReadSignal<Vec<UsersInTeam>>,
    on_select: impl Fn(String, TeamOrderEntry) + 'static + Clone + Send,
    on_delete: impl Fn(String, String) + 'static + Clone + Send,
    on_toggle_closed: impl Fn(String, bool) + 'static + Clone + Send,
) -> impl IntoView {
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

            let open_orders: Vec<_> = order_list.iter().filter(|o| !o.closed).cloned().collect();
            let closed_orders: Vec<_> = order_list.iter().filter(|o| o.closed).cloned().collect();

            view! {
                // Active orders section
                <div class="card">
                    <div class="section-title">"Active Orders"</div>
                    {if open_orders.is_empty() {
                        view! {
                            <p class="text-muted">"No active orders."</p>
                        }.into_any()
                    } else {
                        view! {
                            <OrdersTable
                                orders=open_orders
                                team_id=team_id.clone()
                                selected_order=selected_order
                                team_members=team_members
                                on_select=on_select.clone()
                                on_delete=on_delete.clone()
                                on_toggle_closed=on_toggle_closed.clone()
                            />
                        }.into_any()
                    }}
                </div>

                // Previous/closed orders section
                {if !closed_orders.is_empty() {
                    view! {
                        <div class="card" style="margin-top: var(--ds-layout-spacing-300, 16px);">
                            <div class="section-title">"Previous Orders"</div>
                            <OrdersTable
                                orders=closed_orders
                                team_id=team_id.clone()
                                selected_order=selected_order
                                team_members=team_members
                                on_select=on_select.clone()
                                on_delete=on_delete.clone()
                                on_toggle_closed=on_toggle_closed.clone()
                            />
                        </div>
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }}
            }.into_any()
        }}
    }
}

#[component]
fn OrdersTable(
    orders: Vec<TeamOrderEntry>,
    team_id: String,
    selected_order: ReadSignal<Option<TeamOrderEntry>>,
    team_members: ReadSignal<Vec<UsersInTeam>>,
    on_select: impl Fn(String, TeamOrderEntry) + 'static + Clone + Send,
    on_delete: impl Fn(String, String) + 'static + Clone + Send,
    on_toggle_closed: impl Fn(String, bool) + 'static + Clone + Send,
) -> impl IntoView {
    let user = expect_context::<ReadSignal<Option<UserContext>>>();
    view! {
        <table class="connect-table connect-table--medium">
            <thead class="connect-table-header">
                <tr>
                    <th class="connect-table-header-cell">"Due Date"</th>
                    <th class="connect-table-header-cell">"Pickup"</th>
                    <th class="connect-table-header-cell">"Status"</th>
                    <th class="connect-table-header-cell connect-table-header-cell--actions">"Actions"</th>
                </tr>
            </thead>
            <tbody class="connect-table-body">
                {orders.into_iter().map(|order| {
                    let oid = order.teamorders_id.clone();
                    let oid_del = order.teamorders_id.clone();
                    let order_owner_id = order.teamorders_user_id.clone();
                    let order_team_id = order.teamorders_team_id.clone();
                    let due = order.duedate.clone().unwrap_or_else(|| "No date".to_string());
                    let due_label = due.clone();
                    let closed = order.closed;

                    // Resolve pickup user name
                    let pickup_name = {
                        let members = team_members.get();
                        order.pickup_user_id.as_ref().and_then(|pid| {
                            members.iter().find(|m| m.user_id == *pid)
                                .map(|m| format!("{} {}", m.firstname, m.lastname))
                        }).unwrap_or_else(|| "—".to_string())
                    };

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
                            <td class="connect-table-cell">{pickup_name.clone()}</td>
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
    }
}
