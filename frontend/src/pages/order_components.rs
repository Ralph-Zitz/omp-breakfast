use crate::api::{ItemEntry, OrderItemEntry, TeamOrderEntry, UsersInTeam};
use crate::components::LoadingSpinner;
use crate::components::icons::{Icon, IconKind};
use crate::components::input_handler;
use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

#[component]
pub fn OrderDetail(
    order: ReadSignal<Option<TeamOrderEntry>>,
    items: ReadSignal<Vec<OrderItemEntry>>,
    catalog: ReadSignal<Vec<ItemEntry>>,
    loading: ReadSignal<bool>,
    team_members: ReadSignal<Vec<UsersInTeam>>,
    on_add_item: impl Fn(String, i32) + 'static + Clone + Send,
    on_update_item: impl Fn(String, i32) + 'static + Clone + Send,
    on_remove_item: impl Fn(String) + 'static + Clone + Send,
    on_assign_pickup: impl Fn(Option<String>) + 'static + Clone + Send,
    on_update_duedate: impl Fn(Option<String>) + 'static + Clone + Send,
) -> impl IntoView {
    let (add_item_id, set_add_item_id) = signal(String::new());
    let (add_qty, set_add_qty) = signal("1".to_string());

    // Reset add-item form whenever the selected order changes
    Effect::new(move |_| {
        let _ = order.get();
        set_add_item_id.set(String::new());
        set_add_qty.set("1".to_string());
    });

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
            let cat_for_price = cat.clone();
            let resolve_name = move |item_id: &str| -> String {
                cat.iter()
                    .find(|i| i.item_id == item_id)
                    .map(|i| i.descr.clone())
                    .unwrap_or_else(|| item_id.to_string())
            };

            // Resolve item price from catalog as integer cents to avoid f64 rounding
            let resolve_price_cents = move |item_id: &str| -> i64 {
                cat_for_price
                    .iter()
                    .find(|i| i.item_id == item_id)
                    .and_then(|i| {
                        // Parse "12.50" → 1250 cents using fixed-point string parsing
                        let s = i.price.as_str();
                        let (whole, frac) = match s.find('.') {
                            Some(dot) => (&s[..dot], &s[dot + 1..]),
                            None => (s, ""),
                        };
                        let w: i64 = whole.parse().ok()?;
                        let f: i64 = match frac.len() {
                            0 => 0,
                            1 => frac.parse::<i64>().ok()? * 10,
                            _ => frac[..2].parse().ok()?,
                        };
                        Some(w * 100 + f)
                    })
                    .unwrap_or(0)
            };

            // Compute grand total in cents, then format
            let grand_total_cents: i64 = item_list.iter()
                .map(|oi| resolve_price_cents(&oi.orders_item_id) * oi.amt as i64)
                .sum();

            view! {
                <div class="card">
                    <div class="section-title">
                        "Order Items"
                        {if closed {
                            view! {
                                <span class="connect-tag connect-tag--small connect-tag--neutral-default order-closed-tag">"Closed"</span>
                            }.into_any()
                        } else {
                            view! { <span /> }.into_any()
                        }}
                    </div>

                    // Due date
                    <div class="order-field-group">
                        <div class="connect-text-field">
                            <div class="connect-label">
                                <label class="connect-label__text" for="detail-duedate">"Due Date"</label>
                            </div>
                            {if closed {
                                let date_display = ord.duedate.clone().unwrap_or_else(|| "No date".to_string());
                                view! { <p class="text-muted">{date_display}</p> }.into_any()
                            } else {
                                let on_update = on_update_duedate.clone();
                                let current_date = ord.duedate.clone().unwrap_or_default();
                                let today = {
                                    let d = js_sys::Date::new_0();
                                    let y = d.get_full_year();
                                    let m = d.get_month() + 1;
                                    let day = d.get_date();
                                    format!("{y:04}-{m:02}-{day:02}")
                                };
                                view! {
                                    <div class="connect-text-field__input-wrapper">
                                        <input
                                            class="connect-text-field__input"
                                            id="detail-duedate"
                                            type="date"
                                            min=today
                                            prop:value=current_date
                                            on:change=move |ev| {
                                                let Some(target) = ev.target() else { return; };
                                                let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() else { return; };
                                                let val = input.value();
                                                on_update(if val.is_empty() { None } else { Some(val) });
                                            }
                                        />
                                    </div>
                                }.into_any()
                            }}
                        </div>
                    </div>

                    // Pickup user assignment
                    <div class="order-field-group">
                        <div class="connect-text-field">
                            <div class="connect-label">
                                <label class="connect-label__text" for="detail-pickup">"Pickup Person"</label>
                            </div>
                            {if closed {
                                let members = team_members.get();
                                let pickup_name = ord.pickup_user_id.as_ref().and_then(|pid| {
                                    members.iter().find(|m| m.user_id == *pid)
                                        .map(|m| format!("{} {}", m.firstname, m.lastname))
                                }).unwrap_or_else(|| "None".to_string());
                                view! { <p class="text-muted">{pickup_name}</p> }.into_any()
                            } else {
                                let on_assign = on_assign_pickup.clone();
                                let current_pickup = ord.pickup_user_id.clone().unwrap_or_default();
                                let members = team_members.get();
                                view! {
                                    <select
                                        id="detail-pickup"
                                        class="connect-text-field__input"
                                        prop:value=current_pickup.clone()
                                        on:change=move |ev| {
                                            let Some(target) = ev.target() else { return; };
                                            let Some(select) = target.dyn_ref::<web_sys::HtmlSelectElement>() else { return; };
                                            let val = select.value();
                                            on_assign(if val.is_empty() { None } else { Some(val) });
                                        }
                                    >
                                        <option value="">"None"</option>
                                        {members.into_iter().map(|m| {
                                            let uid = m.user_id.clone();
                                            let selected = uid == current_pickup;
                                            let label = format!("{} {}", m.firstname, m.lastname);
                                            view! { <option value=uid selected=selected>{label}</option> }
                                        }).collect::<Vec<_>>()}
                                    </select>
                                }.into_any()
                            }}
                        </div>
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
                                        <th class="connect-table-header-cell cell-align-right">"Total"</th>
                                        {(!closed).then(|| view! {
                                            <th class="connect-table-header-cell connect-table-header-cell--actions">"Remove"</th>
                                        })}
                                    </tr>
                                </thead>
                                <tbody class="connect-table-body">
                                    {item_list.into_iter().map(|oi| {
                                        let name = resolve_name(&oi.orders_item_id);
                                        let line_total_cents = resolve_price_cents(&oi.orders_item_id) * oi.amt as i64;
                                        let iid = oi.orders_item_id.clone();
                                        let on_remove_item = on_remove_item.clone();
                                        let on_update_item = on_update_item.clone();
                                        let current_amt = oi.amt;
                                        view! {
                                            <tr class="connect-table-row">
                                                <td class="connect-table-cell">{name}</td>
                                                <td class="connect-table-cell">
                                                    {if !closed {
                                                        let iid_upd = iid.clone();
                                                        view! {
                                                            <input
                                                                class="connect-text-field__input order-qty-input"
                                                                type="number"
                                                                min="1"
                                                                value=current_amt.to_string()
                                                                on:change=move |ev| {
                                                                    let Some(target) = ev.target() else { return; };
                                                                    let val: i32 = target.dyn_ref::<web_sys::HtmlInputElement>().map(|el| el.value()).unwrap_or_default().parse().unwrap_or(current_amt);
                                                                    if val != current_amt && val >= 1 {
                                                                        on_update_item(iid_upd.clone(), val);
                                                                    }
                                                                }
                                                            />
                                                        }.into_any()
                                                    } else {
                                                        view! { <span>{current_amt}</span> }.into_any()
                                                    }}
                                                </td>
                                                <td class="connect-table-cell cell-align-right">
                                                    {format!("{}.{:02} kr", line_total_cents / 100, line_total_cents % 100)}
                                                </td>
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
                            <div class="order-total-row">
                                <strong>{format!("Order Total: {}.{:02} kr", grand_total_cents / 100, grand_total_cents % 100)}</strong>
                            </div>
                        }.into_any()
                    }}

                    // Add item form (only if order is open)
                    {if !closed {
                        let cat_for_select = catalog.get();
                        let on_add_item = on_add_item.clone();
                        view! {
                            <div class="add-item-form">
                                <div class="connect-text-field field-flex-grow">
                                    <div class="connect-label">
                                        <label class="connect-label__text" for="add-item-select">"Item"</label>
                                    </div>
                                    <select
                                        id="add-item-select"
                                        class="connect-text-field__input"
                                        prop:value=move || add_item_id.get()
                                        on:change=move |ev| {
                                            let Some(target) = ev.target() else { return; };
                                            let Some(select) = target.dyn_ref::<web_sys::HtmlSelectElement>() else { return; };
                                            set_add_item_id.set(select.value());
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
                                <div class="connect-text-field field-narrow">
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
                                            on:input=input_handler(set_add_qty)
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
pub fn CreateOrderDialog(
    open: Signal<bool>,
    team_members: ReadSignal<Vec<UsersInTeam>>,
    on_create: impl Fn(Option<String>, Option<String>) + 'static + Clone + Send,
    on_cancel: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let (duedate, set_duedate) = signal(String::new());
    let (pickup_user, set_pickup_user) = signal(String::new());
    let (date_error, set_date_error) = signal(Option::<String>::None);

    let today = move || {
        let d = js_sys::Date::new_0();
        let y = d.get_full_year();
        let m = d.get_month() + 1;
        let day = d.get_date();
        format!("{y:04}-{m:02}-{day:02}")
    };

    let reset = move || {
        set_duedate.set(String::new());
        set_pickup_user.set(String::new());
        set_date_error.set(None);
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
                                        min=today()
                                        prop:value=move || duedate.get()
                                        on:input=move |ev| {
                                            let Some(target) = ev.target() else { return; };
                                            let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() else { return; };
                                            let val = input.value();
                                            if !val.is_empty() && val < today() {
                                                set_date_error.set(Some("Due date cannot be in the past".to_string()));
                                            } else {
                                                set_date_error.set(None);
                                            }
                                            set_duedate.set(val);
                                        }
                                    />
                                </div>
                                {move || date_error.get().map(|msg| view! {
                                    <p class="connect-text-field__error-text">{msg}</p>
                                })}
                            </div>
                            <div class="connect-text-field order-field-group-top">
                                <div class="connect-label">
                                    <label class="connect-label__text" for="order-pickup">"Pickup Person (optional)"</label>
                                </div>
                                <select
                                    id="order-pickup"
                                    class="connect-text-field__input"
                                    prop:value=move || pickup_user.get()
                                    on:change=move |ev| {
                                        let Some(target) = ev.target() else { return; };
                                        let Some(select) = target.dyn_ref::<web_sys::HtmlSelectElement>() else { return; };
                                        set_pickup_user.set(select.value());
                                    }
                                >
                                    <option value="">"None"</option>
                                    {team_members.get().into_iter().map(|m| {
                                        let uid = m.user_id.clone();
                                        let label = format!("{} {}", m.firstname, m.lastname);
                                        view! { <option value=uid>{label}</option> }
                                    }).collect::<Vec<_>>()}
                                </select>
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
                                disabled=move || date_error.get().is_some()
                                on:click={
                                    let create = on_create.clone();
                                    move |_| {
                                        if date_error.get_untracked().is_some() {
                                            return;
                                        }
                                        let d = duedate.get();
                                        let p = pickup_user.get();
                                        create(
                                            if d.is_empty() { None } else { Some(d) },
                                            if p.is_empty() { None } else { Some(p) },
                                        );
                                        set_duedate.set(String::new());
                                        set_pickup_user.set(String::new());
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
