//! Team membership, RBAC queries, and admin guard DB function tests.

#[path = "common/db.rs"]
mod db_helpers;

use breakfast::{db, models::*};
use db_helpers::*;
use uuid::Uuid;

// ===========================================================================
// Group 8: Memberof / RBAC queries
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn is_admin_returns_true_for_admin() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;

    let result = db::is_admin(&client, admin.user_id)
        .await
        .expect("is_admin should succeed");
    assert!(result, "user with Admin role should be recognized as admin");

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, admin.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_admin_returns_false_for_member() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let result = db::is_admin(&client, member.user_id)
        .await
        .expect("is_admin should succeed");
    assert!(!result, "member should not be admin");

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_admin_returns_false_for_nonexistent_user() {
    let client = test_client().await;
    let result = db::is_admin(&client, Uuid::now_v7())
        .await
        .expect("is_admin should succeed even for unknown user");
    assert!(!result);
}

#[actix_web::test]
#[ignore]
async fn is_admin_or_team_admin_returns_true_for_admin() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;

    let result = db::is_admin_or_team_admin(&client, admin.user_id)
        .await
        .expect("is_admin_or_team_admin should succeed");
    assert!(result);

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, admin.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_admin_or_team_admin_returns_true_for_team_admin() {
    let mut client = test_client().await;
    let (_admin, ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let result = db::is_admin_or_team_admin(&client, ta.user_id)
        .await
        .expect("is_admin_or_team_admin should succeed");
    assert!(result, "Team Admin should return true");

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_admin_or_team_admin_returns_false_for_member() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let result = db::is_admin_or_team_admin(&client, member.user_id)
        .await
        .expect("is_admin_or_team_admin should succeed");
    assert!(!result, "Member should return false");

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_correct_role_for_admin() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;

    let role = db::get_member_role(&client, team.team_id, admin.user_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, Some("Admin".to_string()));

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, admin.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_correct_role_for_team_admin() {
    let mut client = test_client().await;
    let (_admin, ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let role = db::get_member_role(&client, team1.team_id, ta.user_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, Some("Team Admin".to_string()));

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_member_for_regular_user() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let role = db::get_member_role(&client, team1.team_id, member.user_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, Some("Member".to_string()));

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_member_role_returns_none_for_non_member() {
    let mut client = test_client().await;
    let (_admin, _ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    // member is in team1 but not team2
    let role = db::get_member_role(&client, team2.team_id, _member.user_id)
        .await
        .expect("get_member_role should succeed");
    assert_eq!(role, None);

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_admin_of_user_returns_true_for_shared_team() {
    let mut client = test_client().await;
    let (_admin, ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    // ta is Team Admin of team1, member is Member of team1
    let result = db::is_team_admin_of_user(&client, ta.user_id, member.user_id)
        .await
        .expect("is_team_admin_of_user should succeed");
    assert!(
        result,
        "Team Admin should be recognized as team admin of a member in the same team"
    );

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_admin_of_user_returns_false_for_non_shared_team() {
    let mut client = test_client().await;
    let (_admin, ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    // Create a user who is NOT in any of ta's teams
    let outsider = create_test_user(&client).await;

    let result = db::is_team_admin_of_user(&client, ta.user_id, outsider.user_id)
        .await
        .expect("is_team_admin_of_user should succeed");
    assert!(
        !result,
        "Team Admin should not be team admin of a user outside their teams"
    );

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _member.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, outsider.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn is_team_admin_of_user_returns_false_for_regular_member() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    // Create another member in team1
    let member2 = create_test_user(&client).await;
    db::add_team_member(
        &mut client,
        team1.team_id,
        member2.user_id,
        _roles["Member"],
    )
    .await
    .expect("add member2");

    // member is not a Team Admin, so should return false
    let result = db::is_team_admin_of_user(&client, member.user_id, member2.user_id)
        .await
        .expect("is_team_admin_of_user should succeed");
    assert!(
        !result,
        "regular member should not be team admin of another member"
    );

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, member.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, member2.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 9: Team member management
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn add_team_member_returns_users_in_team() {
    let mut client = test_client().await;

    // Create a fresh team and user to avoid conflicts with seed data
    let tname = format!("dbtest-member-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let email = unique_email();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "NewMember".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];

    let result = db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id)
        .await
        .expect("add_team_member should succeed");

    assert_eq!(result.user_id, user.user_id);
    assert_eq!(result.firstname, "NewMember");
    assert_eq!(result.title, "Member");

    // Cleanup (cascade: deleting team removes memberof)
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup user");
}

#[actix_web::test]
#[ignore]
async fn remove_team_member_returns_true_then_false() {
    let mut client = test_client().await;

    let tname = format!("dbtest-rmmember-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let email = unique_email();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "RemoveMe".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];
    db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id)
        .await
        .unwrap();

    let removed = db::remove_team_member(&mut client, team.team_id, user.user_id)
        .await
        .unwrap();
    assert!(removed);

    let removed_again = db::remove_team_member(&mut client, team.team_id, user.user_id)
        .await
        .unwrap();
    assert!(!removed_again);

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup user");
}

#[actix_web::test]
#[ignore]
async fn update_member_role_changes_title() {
    let mut client = test_client().await;

    let tname = format!("dbtest-updrole-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let email = unique_email();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "RoleChange".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];
    let guest_role_id = roles["Guest"];

    db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id)
        .await
        .unwrap();

    let updated = db::update_member_role(&mut client, team.team_id, user.user_id, guest_role_id)
        .await
        .expect("update_member_role should succeed");

    assert_eq!(updated.title, "Guest");
    assert_eq!(updated.user_id, user.user_id);

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup user");
}

#[actix_web::test]
#[ignore]
async fn update_member_role_returns_not_found_for_non_member() {
    let mut client = test_client().await;

    let tname = format!("dbtest-rolefail-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];
    let random_user_id = Uuid::now_v7();

    let result =
        db::update_member_role(&mut client, team.team_id, random_user_id, member_role_id).await;

    assert!(result.is_err(), "should fail for non-member");
    let err_msg = match result {
        Err(e) => e.to_string(),
        Ok(_) => panic!("expected error for non-member"),
    };
    assert!(
        err_msg.contains("member not found"),
        "error should mention member not found, got: {}",
        err_msg
    );

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup team");
}

#[actix_web::test]
#[ignore]
async fn add_duplicate_team_member_returns_error() {
    let mut client = test_client().await;

    let tname = format!("dbtest-dupmember-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let email = unique_email();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "DupMember".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];

    db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id)
        .await
        .unwrap();

    // Adding the same user again should fail (PK violation)
    let result = db::add_team_member(&mut client, team.team_id, user.user_id, member_role_id).await;
    assert!(result.is_err(), "duplicate member should fail");

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup user");
}

#[actix_web::test]
#[ignore]
async fn add_team_member_with_nonexistent_user_returns_error() {
    let mut client = test_client().await;

    let tname = format!("dbtest-baduser-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let roles = ensure_roles(&client).await;
    let member_role_id = roles["Member"];
    let fake_user_id = Uuid::now_v7();

    let result = db::add_team_member(&mut client, team.team_id, fake_user_id, member_role_id).await;
    assert!(
        result.is_err(),
        "adding nonexistent user should fail with FK violation"
    );

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup team");
}

#[actix_web::test]
#[ignore]
async fn add_team_member_with_nonexistent_role_returns_error() {
    let mut client = test_client().await;

    let tname = format!("dbtest-badrole-{}", Uuid::now_v7());
    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let email = unique_email();
    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "BadRole".to_string(),
            lastname: "Test".to_string(),
            email: email.clone(),
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let fake_role_id = Uuid::now_v7();

    let result = db::add_team_member(&mut client, team.team_id, user.user_id, fake_role_id).await;
    assert!(
        result.is_err(),
        "adding member with nonexistent role should fail with FK violation"
    );

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup user");
}

// ===========================================================================
// Group 10: User/Team relationship queries
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn get_user_teams_returns_memberships() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;

    let (teams, _) = db::get_user_teams(&client, admin.user_id, 100, 0)
        .await
        .expect("get_user_teams should succeed");
    assert!(
        !teams.is_empty(),
        "admin should have at least 1 team membership"
    );
    // Verify the structure includes the expected fields
    let first = &teams[0];
    assert!(!first.tname.is_empty());
    assert!(!first.title.is_empty());
    assert!(!first.firstname.is_empty());

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, admin.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_user_teams_returns_empty_for_no_memberships() {
    let mut client = test_client().await;
    let email = unique_email();

    let user = db::create_user(
        &client,
        CreateUserEntry {
            firstname: "NoTeams".to_string(),
            lastname: "User".to_string(),
            email,
            password: "securepassword123".to_string(),
        },
    )
    .await
    .unwrap();

    let (teams, _) = db::get_user_teams(&client, user.user_id, 100, 0)
        .await
        .expect("get_user_teams should succeed for user with no teams");
    assert!(
        teams.is_empty(),
        "user with no memberships should return []"
    );

    // Cleanup
    db::delete_user(&mut client, user.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_team_users_returns_members() {
    let mut client = test_client().await;
    let (_admin, _ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let (users, _) = db::get_team_users(&client, team1.team_id, 100, 0)
        .await
        .expect("get_team_users should succeed");
    assert!(
        users.len() >= 3,
        "team1 should have at least 3 members, got {}",
        users.len()
    );
    // Verify the structure
    let first = &users[0];
    assert!(!first.firstname.is_empty());
    assert!(!first.email.is_empty());
    assert!(!first.title.is_empty());

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_team_users_returns_empty_for_empty_team() {
    let mut client = test_client().await;
    let tname = format!("dbtest-empty-{}", Uuid::now_v7());

    let team = db::create_team(&client, CreateTeamEntry { tname, descr: None })
        .await
        .unwrap();

    let (users, _) = db::get_team_users(&client, team.team_id, 100, 0)
        .await
        .expect("get_team_users should succeed for empty team");
    assert!(users.is_empty(), "empty team should return []");

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn get_user_teams_for_multi_team_user() {
    let mut client = test_client().await;
    // create_rbac_setup puts ta in team1 (Team Admin) and team2 (Member)
    let (_admin, ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let (teams, _) = db::get_user_teams(&client, ta.user_id, 100, 0)
        .await
        .expect("get_user_teams should succeed");
    assert!(
        teams.len() >= 2,
        "team admin should be in at least 2 teams, got {}",
        teams.len()
    );

    // Verify different roles in different teams
    let team_names: Vec<&str> = teams.iter().map(|t| t.tname.as_str()).collect();
    assert!(
        team_names.contains(&team1.tname.as_str()),
        "should include team1"
    );
    assert!(
        team_names.contains(&team2.tname.as_str()),
        "should include team2"
    );

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _member.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// Group 11: check_team_access (#174)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn check_team_access_admin_in_own_team() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;

    let (is_admin, team_role) = db::check_team_access(&client, team.team_id, admin.user_id)
        .await
        .expect("check_team_access should succeed");
    assert!(is_admin, "admin should be recognized as global admin");
    assert_eq!(
        team_role,
        Some("Admin".to_string()),
        "admin should have Admin role in the team"
    );

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, admin.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn check_team_access_regular_member() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let (is_admin, team_role) = db::check_team_access(&client, team1.team_id, member.user_id)
        .await
        .expect("check_team_access should succeed");
    assert!(!is_admin, "regular member should not be global admin");
    assert_eq!(
        team_role,
        Some("Member".to_string()),
        "member should have Member role in the team"
    );

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn check_team_access_non_member() {
    let mut client = test_client().await;
    let (_admin, _ta, member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    // member is in team1 but not team2
    let (is_admin, team_role) = db::check_team_access(&client, team2.team_id, member.user_id)
        .await
        .expect("check_team_access should succeed");
    assert!(!is_admin, "member should not be global admin");
    assert!(team_role.is_none(), "member should have no role in team2");

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, member.user_id)
        .await
        .expect("cleanup");
}

#[actix_web::test]
#[ignore]
async fn check_team_access_admin_in_unrelated_team() {
    let mut client = test_client().await;
    let (admin, team, _roles) = create_admin_setup(&mut client).await;
    // Create a second team where admin is NOT a member
    let other_team = create_test_team(&client).await;

    let (is_admin, team_role) = db::check_team_access(&client, other_team.team_id, admin.user_id)
        .await
        .expect("check_team_access should succeed");
    assert!(is_admin, "admin should still be recognized as global admin");
    assert!(
        team_role.is_none(),
        "admin should have no role in unrelated team (not a member)"
    );

    // Cleanup
    db::delete_team(&mut client, other_team.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, admin.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// check_team_access for Team Admin role (#393)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn check_team_access_team_admin() {
    let mut client = test_client().await;
    let (_admin, ta, _member, team1, team2, _roles) = create_rbac_setup(&mut client).await;

    let (is_admin, team_role) = db::check_team_access(&client, team1.team_id, ta.user_id)
        .await
        .expect("check_team_access should succeed");
    assert!(
        !is_admin,
        "Team Admin should NOT be recognized as global admin"
    );
    assert_eq!(
        team_role,
        Some("Team Admin".to_string()),
        "team admin should have Team Admin role"
    );

    // Cleanup
    db::delete_team(&mut client, team1.team_id)
        .await
        .expect("cleanup");
    db::delete_team(&mut client, team2.team_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _admin.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, ta.user_id)
        .await
        .expect("cleanup");
    db::delete_user(&mut client, _member.user_id)
        .await
        .expect("cleanup");
}

// ===========================================================================
// would_admins_remain_without DB test (#623)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn would_admins_remain_without_two_admins() {
    let mut client = test_client().await;
    let roles = ensure_roles(&client).await;
    let admin1 = create_test_user(&client).await;
    let admin2 = create_test_user(&client).await;
    let team = create_test_team(&client).await;

    db::add_team_member(&mut client, team.team_id, admin1.user_id, roles["Admin"])
        .await
        .expect("add admin1");
    db::add_team_member(&mut client, team.team_id, admin2.user_id, roles["Admin"])
        .await
        .expect("add admin2");

    // With 2 admins, excluding one should still leave admins
    let remains = db::would_admins_remain_without(&client, team.team_id, admin1.user_id)
        .await
        .expect("would_admins_remain_without should succeed");
    assert!(
        remains,
        "should still have admins after excluding one of two"
    );

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&mut client, admin1.user_id)
        .await
        .expect("cleanup admin1");
    db::delete_user(&mut client, admin2.user_id)
        .await
        .expect("cleanup admin2");
}

#[actix_web::test]
#[ignore]
async fn would_admins_remain_without_sole_admin() {
    let mut client = test_client().await;
    let roles = ensure_roles(&client).await;

    // Record how many admins already exist from parallel tests so we can
    // account for them when checking the result.
    let baseline = db::count_admins(&client)
        .await
        .expect("count_admins should succeed");

    let admin = create_test_user(&client).await;
    let team = create_test_team(&client).await;

    db::add_team_member(&mut client, team.team_id, admin.user_id, roles["Admin"])
        .await
        .expect("add admin");

    // Excluding our admin should leave only the baseline admins (from
    // parallel tests). When the baseline is 0, the function should return
    // false (no admins remain). When the baseline is > 0, it should return
    // true because other admins still exist.
    let remains = db::would_admins_remain_without(&client, team.team_id, admin.user_id)
        .await
        .expect("would_admins_remain_without should succeed");
    assert_eq!(
        remains,
        baseline > 0,
        "expected remains={} when baseline admin count is {baseline}",
        baseline > 0
    );

    // Cleanup
    db::delete_team(&mut client, team.team_id)
        .await
        .expect("cleanup team");
    db::delete_user(&mut client, admin.user_id)
        .await
        .expect("cleanup admin");
}
