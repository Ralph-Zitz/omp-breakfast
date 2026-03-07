//! Item CRUD DB function tests.

#[path = "common/db.rs"]
mod db_helpers;

use breakfast::{db, models::*};
use db_helpers::*;
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

// ===========================================================================
// Group 5: Item CRUD
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_item_returns_entry_with_price() {
    let client = test_client().await;
    let descr = format!("dbtest-item-{}", Uuid::now_v7());

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::from_str("12.50").unwrap(),
        },
    )
    .await
    .expect("create_item should succeed");

    assert_eq!(item.descr, descr);
    assert_eq!(item.price, Decimal::from_str("12.50").unwrap());
    assert!(!item.item_id.is_nil());

    // Cleanup
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn create_item_with_null_price() {
    let client = test_client().await;
    let descr = format!("dbtest-item-nullprice-{}", Uuid::now_v7());

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::ZERO,
        },
    )
    .await
    .expect("create_item with zero price should succeed");

    assert_eq!(item.price, Decimal::ZERO);

    // Cleanup
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_item_by_id() {
    let client = test_client().await;
    let descr = format!("dbtest-item-{}", Uuid::now_v7());

    let created = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::from_str("5.00").unwrap(),
        },
    )
    .await
    .unwrap();

    let fetched = db::get_item(&client, created.item_id)
        .await
        .expect("get_item should succeed");
    assert_eq!(fetched.item_id, created.item_id);
    assert_eq!(fetched.descr, descr);

    // Cleanup
    db::delete_item(&client, created.item_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_items_returns_created_data() {
    let client = test_client().await;
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-item-{}", Uuid::now_v7()),
            price: Decimal::new(900, 2),
        },
    )
    .await
    .expect("create_item should succeed");
    let (items, total) = db::get_items(&client, 100, 0)
        .await
        .expect("get_items should succeed");
    assert!(total >= 1, "should have at least 1 item, got {}", total);
    assert!(items.iter().any(|i| i.item_id == item.item_id));
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_item_changes_fields() {
    let client = test_client().await;
    let descr = format!("dbtest-item-{}", Uuid::now_v7());

    let created = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::from_str("1.00").unwrap(),
        },
    )
    .await
    .unwrap();

    let new_descr = format!("updated-item-{}", Uuid::now_v7());
    let updated = db::update_item(
        &client,
        created.item_id,
        UpdateItemEntry {
            descr: new_descr.clone(),
            price: Decimal::from_str("99.99").unwrap(),
        },
    )
    .await
    .expect("update_item should succeed");

    assert_eq!(updated.descr, new_descr);
    assert_eq!(updated.price, Decimal::from_str("99.99").unwrap());

    // Cleanup
    db::delete_item(&client, created.item_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_item_returns_true_then_false() {
    let client = test_client().await;
    let descr = format!("dbtest-item-{}", Uuid::now_v7());

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr,
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let deleted = db::delete_item(&client, item.item_id).await.unwrap();
    assert!(deleted);

    let deleted_again = db::delete_item(&client, item.item_id).await.unwrap();
    assert!(!deleted_again);
}

#[actix_web::test]
#[ignore]
async fn get_item_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::get_item(&client, Uuid::now_v7()).await;
    assert!(result.is_err());
}

#[actix_web::test]
#[ignore]
async fn create_duplicate_item_returns_error() {
    let client = test_client().await;
    let descr = format!("dbtest-item-dup-{}", Uuid::now_v7());

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::ZERO,
        },
    )
    .await
    .unwrap();

    let result = db::create_item(
        &client,
        CreateItemEntry {
            descr: descr.clone(),
            price: Decimal::ZERO,
        },
    )
    .await;

    assert!(result.is_err(), "duplicate item descr should fail");

    // Cleanup
    db::delete_item(&client, item.item_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_item_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::update_item(
        &client,
        Uuid::now_v7(),
        UpdateItemEntry {
            descr: "ghost".to_string(),
            price: Decimal::ZERO,
        },
    )
    .await;
    assert!(result.is_err(), "updating nonexistent item should fail");
}

/// Deleting an item referenced by an order is blocked by FK RESTRICT (V3 migration).
#[actix_web::test]
#[ignore]
async fn delete_item_with_order_reference_is_restricted() {
    let mut client = test_client().await;
    let (user, team, _roles) = create_admin_setup(&mut client).await;

    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-restrict-{}", Uuid::now_v7()),
            price: Decimal::from_str("5.00").unwrap(),
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

    // Attempt to delete item — should fail due to FK RESTRICT
    let result = db::delete_item(&client, item.item_id).await;
    assert!(
        result.is_err(),
        "deleting an item referenced by an order should fail (FK RESTRICT)"
    );

    // Cleanup: delete order first (cascades order items), then item
    db::delete_team_order(&client, team.team_id, order.teamorders_id)
        .await
        .unwrap();
    db::delete_item(&client, item.item_id).await.unwrap();
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup");
}
