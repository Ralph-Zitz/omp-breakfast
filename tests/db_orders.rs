//! Team order and order item CRUD, closed-order logic, and order total DB function tests.

#[path = "common/db.rs"]
mod db_helpers;

use breakfast::{db, models::*};
use chrono::NaiveDate;
use db_helpers::*;
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

// ===========================================================================
// Group 6: Team order CRUD
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_team_order_returns_entry() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()),
            pickup_user_id: None,
        },
    )
    .await
    .expect("create_team_order should succeed");

    assert_eq!(order.teamorders_team_id, team.team_id);
    assert_eq!(
        order.duedate,
        Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap())
    );
    assert!(!order.closed);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn create_team_order_with_user_id() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .expect("create_team_order with user should succeed");

    assert_eq!(order.teamorders_user_id, user.user_id);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_team_orders_returns_created_data() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let (orders, total) = db::get_team_orders(&client, team.team_id, 100, 0)
        .await
        .expect("get_team_orders should succeed");
    assert_eq!(total, 1);
    assert_eq!(orders[0].teamorders_id, order.teamorders_id);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_team_order_by_id() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let fetched = db::get_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("get_team_order should succeed");
    assert_eq!(fetched.teamorders_id, order.teamorders_id);
    assert_eq!(fetched.teamorders_team_id, team.team_id);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_team_order_changes_fields() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let updated = db::update_team_order(
        &client,
        team.team_id,
        order.teamorders_id,
        UpdateTeamOrderEntry {
            duedate: Some(Some(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap())),
            closed: Some(true),
            pickup_user_id: None,
        },
    )
    .await
    .expect("update_team_order should succeed");

    assert_eq!(
        updated.duedate,
        Some(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap())
    );
    assert!(updated.closed);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_team_order_returns_true_then_false() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let deleted = db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .unwrap();
    assert!(deleted);

    let deleted_again = db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .unwrap();
    assert!(!deleted_again);

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_team_orders_bulk() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    // Create two orders
    db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let count = db::delete_team_orders(&client, team.team_id)
        .await
        .expect("delete_team_orders should succeed");
    assert_eq!(count, 2, "should delete both orders");

    // Deleting again should return 0
    let count_again = db::delete_team_orders(&client, team.team_id).await.unwrap();
    assert_eq!(count_again, 0);

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 7: Order items CRUD (items within a team order)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_order_item_returns_entry() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    // Create an item and order to work with
    let item_descr = format!("dbtest-oitem-{}", Uuid::now_v7());
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: item_descr,
            price: Decimal::from_str("2.00").unwrap(),
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let order_item = db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 5,
        },
    )
    .await
    .expect("create_order_item should succeed");

    assert_eq!(order_item.orders_teamorders_id, order.teamorders_id);
    assert_eq!(order_item.orders_item_id, item.item_id);
    assert_eq!(order_item.orders_team_id, team.team_id);
    assert_eq!(order_item.amt, 5);

    // Cleanup (cascade: deleting order deletes order items)
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_order_items_returns_list() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let item1 = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-oilist1-{}", Uuid::now_v7()),
            price: Decimal::from_str("1.00").unwrap(),
        },
    )
    .await
    .unwrap();
    let item2 = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-oilist2-{}", Uuid::now_v7()),
            price: Decimal::from_str("2.00").unwrap(),
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item1.item_id,
            amt: 1,
        },
    )
    .await
    .unwrap();
    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item2.item_id,
            amt: 2,
        },
    )
    .await
    .unwrap();

    let (items, total) = db::get_order_items(&client, order.teamorders_id, team.team_id, 100, 0)
        .await
        .expect("get_order_items should succeed");
    assert_eq!(total, 2);
    assert_eq!(items.len(), 2);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_item(&client, item1.item_id)
        .await
        .expect("cleanup");
    db::delete_item(&client, item2.item_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_order_item_by_id() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item_descr = format!("dbtest-getoi-{}", Uuid::now_v7());
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: item_descr,
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 3,
        },
    )
    .await
    .unwrap();

    let fetched = db::get_order_item(&client, order.teamorders_id, item.item_id, team.team_id)
        .await
        .expect("get_order_item should succeed");
    assert_eq!(fetched.orders_item_id, item.item_id);
    assert_eq!(fetched.amt, 3);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_order_item_changes_amt() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item_descr = format!("dbtest-updoi-{}", Uuid::now_v7());
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: item_descr,
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 1,
        },
    )
    .await
    .unwrap();

    let updated = db::update_order_item(
        &mut client,
        order.teamorders_id,
        item.item_id,
        team.team_id,
        UpdateOrderEntry { amt: 42 },
    )
    .await
    .expect("update_order_item should succeed");

    assert_eq!(updated.amt, 42);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_order_item_returns_true_then_false() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item_descr = format!("dbtest-deloi-{}", Uuid::now_v7());
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: item_descr,
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 1,
        },
    )
    .await
    .unwrap();

    let deleted =
        db::delete_order_item(&mut client, order.teamorders_id, item.item_id, team.team_id)
            .await
            .unwrap();
    assert!(deleted);

    let deleted_again =
        db::delete_order_item(&mut client, order.teamorders_id, item.item_id, team.team_id)
            .await
            .unwrap();
    assert!(!deleted_again);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn duplicate_order_item_returns_error() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item_descr = format!("dbtest-dupoi-{}", Uuid::now_v7());
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: item_descr,
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 1,
        },
    )
    .await
    .unwrap();

    // Adding the same item again should fail (PK violation)
    let result = db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 2,
        },
    )
    .await;

    assert!(result.is_err(), "duplicate order item should fail");

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup order");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup item");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_team_order_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::update_team_order(
        &client,
        Uuid::now_v7(),
        Uuid::now_v7(),
        UpdateTeamOrderEntry {
            duedate: None,
            closed: None,
            pickup_user_id: None,
        },
    )
    .await;
    assert!(
        result.is_err(),
        "updating nonexistent team order should fail"
    );
}

#[actix_web::test]
#[ignore]
async fn update_order_item_nonexistent_returns_error() {
    let mut client = test_client().await;
    let result = db::update_order_item(
        &mut client,
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        UpdateOrderEntry { amt: 1 },
    )
    .await;
    assert!(
        result.is_err(),
        "updating nonexistent order item should fail"
    );
}

#[actix_web::test]
#[ignore]
async fn get_team_order_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::get_team_order(&client, Uuid::now_v7(), Uuid::now_v7()).await;
    assert!(
        result.is_err(),
        "nonexistent team order should return error"
    );
}

#[actix_web::test]
#[ignore]
async fn get_order_item_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::get_order_item(&client, Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()).await;
    assert!(
        result.is_err(),
        "nonexistent order item should return error"
    );
}

// ---------------------------------------------------------------------------
// is_team_order_closed
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn is_team_order_closed_returns_false_for_open_order() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    // Create a new order (defaults to closed = false)
    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: Some(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()),
            pickup_user_id: None,
        },
    )
    .await
    .expect("should create team order");

    let closed = db::is_team_order_closed(&client, order.teamorders_id, team.team_id)
        .await
        .expect("should check closed status");
    assert!(!closed, "newly created order should not be closed");

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_order_closed_returns_true_for_closed_order() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    // Create a new order
    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: Some(NaiveDate::from_ymd_opt(2026, 12, 26).unwrap()),
            pickup_user_id: None,
        },
    )
    .await
    .expect("should create team order");

    // Close the order
    db::update_team_order(
        &client,
        team.team_id,
        order.teamorders_id,
        UpdateTeamOrderEntry {
            duedate: Some(Some(NaiveDate::from_ymd_opt(2026, 12, 26).unwrap())),
            closed: Some(true),
            pickup_user_id: None,
        },
    )
    .await
    .expect("should close the order");

    let closed = db::is_team_order_closed(&client, order.teamorders_id, team.team_id)
        .await
        .expect("should check closed status");
    assert!(closed, "updated order should be closed");

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_order_closed_returns_not_found_for_nonexistent_order() {
    let client = test_client().await;
    let team = create_test_team(&client).await;
    let fake_order_id = Uuid::now_v7();

    let result = db::is_team_order_closed(&client, fake_order_id, team.team_id).await;
    assert!(result.is_err(), "nonexistent order should return an error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found"),
        "error should mention 'not found', got: {}",
        err_msg
    );

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
}

/// Deleting a team order cascades to its order items.
#[actix_web::test]
#[ignore]
async fn delete_team_order_cascades_order_items() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-oi-cascade-{}", Uuid::now_v7()),
            price: Decimal::from_str("1.50").unwrap(),
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 3,
        },
    )
    .await
    .unwrap();

    // Delete the order — items should cascade
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .unwrap();

    let (items, _) = db::get_order_items(&client, order.teamorders_id, team.team_id, 100, 0)
        .await
        .unwrap();
    assert!(items.is_empty(), "order items should be cascade-deleted");

    db::delete_item(&client, item.item_id).await.unwrap();
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 10: Partial updates and FK violation tests
// ===========================================================================

/// #261 — Partial `update_team_order`: pass `None` for both fields and
/// verify existing values are preserved (COALESCE behaviour).
#[actix_web::test]
#[ignore]
async fn update_team_order_partial_preserves_existing_values() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    // Create with a duedate
    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    // Close the order first so we have a non-default value for `closed`
    let _ = db::update_team_order(
        &client,
        team.team_id,
        order.teamorders_id,
        UpdateTeamOrderEntry {
            duedate: None,
            closed: Some(true),
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    // Now update with None for both fields — values should be preserved
    let updated = db::update_team_order(
        &client,
        team.team_id,
        order.teamorders_id,
        UpdateTeamOrderEntry {
            duedate: None,
            closed: None,
            pickup_user_id: None,
        },
    )
    .await
    .expect("partial update should succeed");

    assert_eq!(
        updated.duedate,
        Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
        "duedate should be preserved when None is passed"
    );
    assert!(
        updated.closed,
        "closed should be preserved when None is passed"
    );

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

/// #262 — `create_team_order` with a non-existent `team_id` should fail
/// with a foreign key violation.
#[actix_web::test]
#[ignore]
async fn create_team_order_with_nonexistent_team_id_fails() {
    let client = test_client().await;
    let user = create_test_user(&client).await;
    let fake_team_id = Uuid::now_v7();

    let result = db::create_team_order(
        &client,
        fake_team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await;

    assert!(
        result.is_err(),
        "creating a team order with non-existent team_id should fail (FK violation)"
    );

    // Cleanup
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// #663 — Missing DB-level tests for count_team_orders, reopen_team_order,
//        and get_order_total
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn count_team_orders_returns_correct_count() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let count_before = db::count_team_orders(&client, team.team_id)
        .await
        .expect("count_team_orders should succeed");
    assert_eq!(count_before, 0);

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let count_after = db::count_team_orders(&client, team.team_id)
        .await
        .expect("count_team_orders should succeed");
    assert_eq!(count_after, 1);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn reopen_team_order_creates_copy_of_closed_order() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-reopen-{}", Uuid::now_v7()),
            price: Decimal::from_str("3.50").unwrap(),
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: Some(NaiveDate::from_ymd_opt(2026, 7, 1).unwrap()),
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item.item_id,
            amt: 2,
        },
    )
    .await
    .unwrap();

    // Close the order
    db::update_team_order(
        &client,
        team.team_id,
        order.teamorders_id,
        UpdateTeamOrderEntry {
            duedate: None,
            closed: Some(true),
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    // Reopen
    let new_order =
        db::reopen_team_order(&mut client, team.team_id, order.teamorders_id, user.user_id)
            .await
            .expect("reopen_team_order should succeed");

    assert!(!new_order.closed, "new order should be open");
    assert_ne!(
        new_order.teamorders_id, order.teamorders_id,
        "new order should have a different ID"
    );
    assert!(
        new_order.duedate.is_none(),
        "new order should have no due date"
    );

    // Verify items were copied
    let (new_items, _) =
        db::get_order_items(&client, new_order.teamorders_id, team.team_id, 100, 0)
            .await
            .unwrap();
    assert_eq!(new_items.len(), 1, "items should be copied to new order");
    assert_eq!(new_items[0].orders_item_id, item.item_id);
    assert_eq!(new_items[0].amt, 2);

    // Original order should still exist and be closed
    let original = db::get_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .unwrap();
    assert!(original.closed, "original order should remain closed");

    // Cleanup
    db::delete_team_order(&client, team.team_id, new_order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn reopen_team_order_rejects_open_order() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let result =
        db::reopen_team_order(&mut client, team.team_id, order.teamorders_id, user.user_id).await;
    assert!(result.is_err(), "should reject reopening an open order");

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_order_total_returns_correct_sum() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item1 = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-total1-{}", Uuid::now_v7()),
            price: Decimal::from_str("2.50").unwrap(),
        },
    )
    .await
    .unwrap();

    let item2 = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-total2-{}", Uuid::now_v7()),
            price: Decimal::from_str("4.00").unwrap(),
        },
    )
    .await
    .unwrap();

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    // Add 3x item1 (3 * 2.50 = 7.50) and 2x item2 (2 * 4.00 = 8.00) = 15.50
    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item1.item_id,
            amt: 3,
        },
    )
    .await
    .unwrap();

    db::create_order_item(
        &mut client,
        order.teamorders_id,
        team.team_id,
        CreateOrderEntry {
            orders_item_id: item2.item_id,
            amt: 2,
        },
    )
    .await
    .unwrap();

    let total = db::get_order_total(&client, order.teamorders_id, team.team_id)
        .await
        .expect("get_order_total should succeed");
    assert_eq!(total, Decimal::from_str("15.50").unwrap());

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_item(&client, item1.item_id)
        .await
        .expect("cleanup");
    db::delete_item(&client, item2.item_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_order_total_returns_zero_for_empty_order() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let order = db::create_team_order(
        &client,
        team.team_id,
        user.user_id,
        CreateTeamOrderEntry {
            duedate: None,
            pickup_user_id: None,
        },
    )
    .await
    .unwrap();

    let total = db::get_order_total(&client, order.teamorders_id, team.team_id)
        .await
        .expect("get_order_total should succeed for empty order");
    assert_eq!(total, Decimal::ZERO);

    // Cleanup
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .expect("cleanup");
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&client, user.user_id)
        .await
        .expect("cleanup");
}
