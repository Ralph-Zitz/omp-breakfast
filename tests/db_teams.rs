//! Team CRUD DB function tests.

#[path = "common/db.rs"]
mod db_helpers;

use breakfast::{db, models::*};
use db_helpers::*;
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

// ===========================================================================
// Group 3: Team CRUD
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_team_returns_entry() {
    let client = test_client().await;
    let tname = format!("dbtest-team-{}", Uuid::now_v7());

    let team = db::create_team(
        &client,
        CreateTeamEntry {
            tname: tname.clone(),
            descr: Some("test description".to_string()),
        },
    )
    .await
    .expect("create_team should succeed");

    assert_eq!(team.tname, tname);
    assert_eq!(team.descr, Some("test description".to_string()));
    assert!(!team.team_id.is_nil());

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_team_by_id() {
    let client = test_client().await;
    let tname = format!("dbtest-team-{}", Uuid::now_v7());

    let created = db::create_team(
        &client,
        CreateTeamEntry {
            tname: tname.clone(),
            descr: None,
        },
    )
    .await
    .unwrap();

    let fetched = db::get_team(&client, created.team_id)
        .await
        .expect("get_team should succeed");
    assert_eq!(fetched.team_id, created.team_id);
    assert_eq!(fetched.tname, tname);

    // Cleanup
    db::delete_team(&client, created.team_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_teams_returns_created_data() {
    let client = test_client().await;
    let team = create_test_team(&client).await;
    let (teams, total) = db::get_teams(&client, 100, 0)
        .await
        .expect("get_teams should succeed");
    assert!(total >= 1, "should have at least 1 team, got {}", total);
    assert!(teams.iter().any(|t| t.team_id == team.team_id));
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn update_team_changes_fields() {
    let client = test_client().await;
    let tname = format!("dbtest-team-{}", Uuid::now_v7());

    let created = db::create_team(
        &client,
        CreateTeamEntry {
            tname: tname.clone(),
            descr: Some("original".to_string()),
        },
    )
    .await
    .unwrap();

    let updated_name = format!("updated-{}", Uuid::now_v7());
    let updated = db::update_team(
        &client,
        created.team_id,
        UpdateTeamEntry {
            tname: updated_name.clone(),
            descr: Some("updated desc".to_string()),
        },
    )
    .await
    .expect("update_team should succeed");

    assert_eq!(updated.tname, updated_name);
    assert_eq!(updated.descr, Some("updated desc".to_string()));

    // Cleanup
    db::delete_team(&client, created.team_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn delete_team_returns_true_then_false() {
    let client = test_client().await;
    let tname = format!("dbtest-team-{}", Uuid::now_v7());

    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let deleted = db::delete_team(&client, team.team_id).await.unwrap();
    assert!(deleted);

    let deleted_again = db::delete_team(&client, team.team_id).await.unwrap();
    assert!(!deleted_again);
}

#[actix_web::test]
#[ignore]
async fn get_team_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::get_team(&client, Uuid::now_v7()).await;
    assert!(result.is_err());
}

#[actix_web::test]
#[ignore]
async fn update_team_nonexistent_returns_error() {
    let client = test_client().await;
    let result = db::update_team(
        &client,
        Uuid::now_v7(),
        UpdateTeamEntry {
            tname: "ghost".to_string(),
            descr: None,
        },
    )
    .await;
    assert!(result.is_err(), "updating nonexistent team should fail");
}

// ===========================================================================
// Group 9: FK Cascade Behaviour
// ===========================================================================

/// Deleting a team cascades to memberof, teamorders, and orders.
#[actix_web::test]
#[ignore]
async fn delete_team_cascades_membership_and_orders() {
    let mut client = test_client().await;

    // Create isolated team, user, item
    let team = db::create_team(
        &client,
        CreateTeamEntry {
            tname: format!("dbtest-cascade-{}", Uuid::now_v7()),
            descr: None,
        },
    )
    .await
    .unwrap();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "Cascade".to_string(),
            lastname: "Test".to_string(),
            email: unique_email(),
            password: "password123".to_string(),
        },
    )
    .await
    .unwrap();
    let item = db::create_item(
        &client,
        CreateItemEntry {
            descr: format!("dbtest-cascade-item-{}", Uuid::now_v7()),
            price: Decimal::from_str("3.00").unwrap(),
        },
    )
    .await
    .unwrap();

    // Add user as member
    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];
    db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id)
        .await
        .unwrap();

    // Create a team order with an order item
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
            amt: 2,
        },
    )
    .await
    .unwrap();

    // Delete the team — should cascade
    let deleted = db::delete_team(&client, team.team_id).await.unwrap();
    assert!(deleted);

    // Membership should be gone
    let (members, _) = db::get_team_users(&client, team.team_id, 100, 0)
        .await
        .unwrap();
    assert!(members.is_empty(), "membership should be cascade-deleted");

    // Team orders should be gone
    let (orders, _) = db::get_team_orders(&client, team.team_id, 100, 0)
        .await
        .unwrap();
    assert!(orders.is_empty(), "team orders should be cascade-deleted");

    // Cleanup: user and item are independent, not cascaded
    db::delete_user(&client, user.user_id).await.unwrap();
    db::delete_item(&client, item.item_id).await.unwrap();
}

// ===========================================================================
// #436 — create_team with a duplicate name fails with a DB error
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_team_with_duplicate_name_fails() {
    let client = test_client().await;
    let unique_name = format!("DuplicateTeam-{}", Uuid::now_v7());

    let entry = CreateTeamEntry {
        tname: unique_name.clone(),
        descr: Some("first".to_string()),
    };

    // First insert should succeed
    let team = db::create_team(&client, entry)
        .await
        .expect("first create_team should succeed");

    // Second insert with the same name should fail (unique constraint violation)
    let duplicate = CreateTeamEntry {
        tname: unique_name,
        descr: Some("duplicate".to_string()),
    };
    let result = db::create_team(&client, duplicate).await;
    assert!(
        result.is_err(),
        "create_team with a duplicate name should return an error"
    );

    // Cleanup
    db::delete_team(&client, team.team_id)
        .await
        .expect("cleanup: delete_team should succeed");
}
