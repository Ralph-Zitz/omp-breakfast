//! Team CRUD, team membership, member role management, and admin guard tests.

mod common;

use actix_web::test;
use common::*;
use serde_json::{Value, json};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Team RBAC: admin-only team CRUD
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_create_team() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a non-admin user
    let suffix = Uuid::now_v7();
    let email = format!("nonadmin-ct-{suffix}@test.local");
    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "NonAdmin",
        "CT",
        &email,
        "securepassword",
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .set_json(json!({"tname": "Forbidden Team", "descr": "Should not be created"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to create teams"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_delete_team() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a non-admin user
    let suffix = Uuid::now_v7();
    let email = format!("nonadmin-dt-{suffix}@test.local");
    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "NonAdmin",
        "DT",
        &email,
        "securepassword",
    )
    .await;

    // Create a team to try to delete
    let team_id = create_test_team(&app, admin_token, &format!("DelTarget-{suffix}")).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to delete teams"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn admin_can_create_and_delete_team() {
    let state = test_state().await;
    let app = test_app!(state);

    // Admin can create and delete teams
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;
    let suffix = Uuid::now_v7();

    // Create a team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": format!("TempAdminTeam-{suffix}"), "descr": "Created by admin"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "admin should be able to create teams");
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap();

    // Delete the team
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should be able to delete teams");
}

#[actix_web::test]
#[ignore]
async fn get_nonexistent_team_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

    let fake_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "non-existent team should return 404");
}

// ---------------------------------------------------------------------------
// Team Admin vs Admin distinction
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_create_team() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin = register_admin(&app).await;
    let admin_token = &admin.access_token;

    // Create a user and make them Team Admin of a team
    let suffix = Uuid::now_v7();
    let (ta_auth, ta_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "NoCreate",
        &format!("ta-nocreate-{}@test.local", suffix),
        "password123",
    )
    .await;
    let team_id = create_test_team(&app, admin_token, &format!("TATeam-{}", suffix)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_user_id, &ta_role_id).await;

    // Team Admin tries to create a team → 403
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", ta_auth.access_token)))
        .set_json(json!({"tname": "Forbidden Team", "descr": "Should not be created"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin should not be able to create teams (requires global admin)"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn team_admin_can_manage_team_members() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin = register_admin(&app).await;
    let admin_token = &admin.access_token;

    let suffix = Uuid::now_v7();

    // Create a Team Admin user and a team
    let (ta_auth, ta_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "Manager",
        &format!("ta-mgr-{}@test.local", suffix),
        "password123",
    )
    .await;
    let ta_token = &ta_auth.access_token;
    let team_id = create_test_team(&app, admin_token, &format!("TAMgrTeam-{}", suffix)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_user_id, &ta_role_id).await;

    // Create a temporary user to add as member
    let (_member_auth, member_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TempMember",
        "Test",
        &format!("tempmember-{}@test.local", suffix),
        "password123",
    )
    .await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;

    // Team Admin adds the new user to the team → should succeed
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "user_id": member_user_id,
            "role_id": member_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "team admin should be able to add members"
    );

    // Team Admin removes the member → should succeed
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, member_user_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "team admin should be able to remove members"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", member_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn admin_can_manage_any_team_members() {
    let state = test_state().await;
    let app = test_app!(state);

    // Admin is NOT a member of the target team — bypass should allow member management
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;
    let suffix = Uuid::now_v7();

    // Create a team the admin is NOT a member of
    let team_id = create_test_team(&app, token, &format!("MemberBypass {}", suffix)).await;

    // Create a temp user
    let email = format!("tempmb-{}@test.local", suffix);
    let (_, new_user_id) =
        create_and_login_user(&app, token, "TempMB", "Test", &email, "securepassword").await;

    // Get "Member" role ID
    let member_role_id = find_role_id(&app, token, "Member").await;

    // Admin adds member to team (not a member themselves) → should succeed
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": member_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "admin should add members to any team via bypass"
    );

    // Admin removes the member
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_user_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should remove members from any team via bypass"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn non_admin_cannot_update_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "NonAdmin",
        "Team",
        &format!("nonadmin.team.{}@test.local", uid),
        "securepassword",
    )
    .await;

    let team_id = create_test_team(&app, admin_token, &format!("NoUpdateTeam {}", uid)).await;

    // Non-admin tries to update the team → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .set_json(json!({"tname": "Forbidden Update", "descr": "Should not work"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should not be able to update teams"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Escalation guard: Team Admin cannot assign global Admin role
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_assign_admin_role_via_add_member() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    // Create TA user and team
    let (ta_auth, ta_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "EscAdd",
        &format!("ta.esc.add.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let ta_token = &ta_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("EscAddTeam {}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_id, &ta_role_id).await;

    // Create target user
    let (_, target_id) = create_and_login_user(
        &app,
        admin_token,
        "EscGuard",
        "Add",
        &format!("escguard.add.{}@test.local", uid),
        "securepassword",
    )
    .await;

    let admin_role_id = find_role_id(&app, admin_token, "Admin").await;

    // Team Admin tries to add user with Admin role → should be 403
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "user_id": target_id,
            "role_id": admin_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin must not assign global Admin role via add_team_member"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", target_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_assign_admin_role_via_update_role() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    // Create TA user and team
    let (ta_auth, ta_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "EscUpd",
        &format!("ta.esc.upd.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let ta_token = &ta_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("EscUpdTeam {}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    let admin_role_id = find_role_id(&app, admin_token, "Admin").await;
    add_member(&app, admin_token, &team_id, &ta_id, &ta_role_id).await;

    // Create target user and add as Member
    let (_, target_id) = create_and_login_user(
        &app,
        admin_token,
        "EscGuard",
        "Update",
        &format!("escguard.upd.{}@test.local", uid),
        "securepassword",
    )
    .await;
    add_member(&app, ta_token, &team_id, &target_id, &member_role_id).await;

    // Team Admin tries to update user's role to Admin → should be 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, target_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "role_id": admin_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin must not escalate a member to global Admin via update_member_role"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, target_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", target_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

// ===========================================================================
// Update team / update role success paths (#177)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn admin_can_update_team() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Create a temp team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": format!("UpdateMe-{}", Uuid::now_v7()), "descr": "Original"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap();

    // Update the team
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": "Updated Team", "descr": "Changed"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should be able to update teams");
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["tname"], "Updated Team");
    assert_eq!(updated["descr"], "Changed");

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// Update member role (#205)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn admin_can_update_member_role() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;
    let uid = Uuid::now_v7();

    // Create a team and look up roles
    let team_id = create_test_team(&app, token, &format!("RoleChangeTeam {}", uid)).await;
    let member_role_id = find_role_id(&app, token, "Member").await;
    let guest_role_id = find_role_id(&app, token, "Guest").await;

    // Create a temp user and add them to the team as Member
    let (_, user_id) = create_and_login_user(
        &app,
        token,
        "RoleChange",
        "Test",
        &format!("rolechange.{}@test.local", uid),
        "securepassword",
    )
    .await;
    add_member(&app, token, &team_id, &user_id, &member_role_id).await;

    // Update their role to Guest
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"role_id": guest_role_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should update member role");
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["title"], "Guest", "role should be updated to Guest");

    // Clean up: remove member, delete user, delete team
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// Team users endpoint (#208)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn team_users_returns_members_of_team() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;
    let uid = Uuid::now_v7();

    // Create a team and add 3 members
    let team_id = create_test_team(&app, token, &format!("TeamUsersTest {}", uid)).await;
    let member_role_id = find_role_id(&app, token, "Member").await;

    let mut user_ids = Vec::new();
    let mut expected_emails = Vec::new();
    for i in 1..=3 {
        let email = format!("teamuser{}.{}@test.local", i, uid);
        let (_, user_id) = create_and_login_user(
            &app,
            token,
            &format!("TU{}", i),
            "Test",
            &email,
            "securepassword",
        )
        .await;
        add_member(&app, token, &team_id, &user_id, &member_role_id).await;
        user_ids.push(user_id);
        expected_emails.push(email);
    }

    // GET /api/v1.0/teams/{team_id}/users
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let users = paginated_items(test::read_body_json(resp).await);

    assert_eq!(
        users.len(),
        3,
        "team should have exactly 3 members, got {}",
        users.len()
    );

    // Verify all 3 members are present
    let emails: Vec<&str> = users.iter().filter_map(|u| u["email"].as_str()).collect();
    for expected in &expected_emails {
        assert!(
            emails.contains(&expected.as_str()),
            "member {} should be in team",
            expected
        );
    }

    // Check that membership timestamps are present (#115)
    let first = &users[0];
    assert!(
        first["joined"].is_string(),
        "joined timestamp should be present"
    );
    assert!(
        first["role_changed"].is_string(),
        "role_changed timestamp should be present"
    );

    // Clean up: remove members, delete users, delete team
    for user_id in &user_ids {
        let req = test::TestRequest::delete()
            .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, user_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        test::call_service(&app, req).await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/v1.0/users/{}", user_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        test::call_service(&app, req).await;
    }

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn team_users_returns_empty_for_team_with_no_members() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Create a fresh team with no members
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": format!("EmptyTeam-{}", Uuid::now_v7()), "descr": "no members"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let users = paginated_items(test::read_body_json(resp).await);
    assert!(users.is_empty(), "new team should have no members");

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// #265 — add_team_member with non-existent role_id returns 404
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn add_team_member_with_nonexistent_role_id_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    // Create a Team Admin user and team
    let (ta_auth, ta_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "FakeRole",
        &format!("ta.fakerole.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let ta_token = &ta_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("FakeRoleTeam {}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_id, &ta_role_id).await;

    // Create a temp user to add
    let (_, new_user_id) = create_and_login_user(
        &app,
        admin_token,
        "RoleTest",
        "User",
        &format!("roletest.nonexistent.{}@test.local", uid),
        "securepassword",
    )
    .await;

    // Try to add user with a non-existent role_id
    let fake_role_id = Uuid::now_v7().to_string();
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": fake_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "non-existent role_id should return 404 from guard_admin_role_assignment"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// #289 — Member cannot manage team members
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn member_cannot_add_team_member() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (member_auth, member_id) = create_and_login_user(
        &app,
        admin_token,
        "Member",
        "Add",
        &format!("member.add.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let member_token = &member_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("MemberAddTeam {}", uid)).await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &member_id, &member_role_id).await;

    // Create a target user
    let (_, target_id) = create_and_login_user(
        &app,
        admin_token,
        "Target",
        "Add",
        &format!("target.add.{}@test.local", uid),
        "securepassword",
    )
    .await;

    // Member tries to add a team member → 403
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", member_token)))
        .set_json(json!({
            "user_id": target_id,
            "role_id": member_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "Member should not be able to add team members"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", target_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", member_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn member_cannot_remove_team_member() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (member_auth, member_id) = create_and_login_user(
        &app,
        admin_token,
        "Member",
        "Remove",
        &format!("member.remove.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let member_token = &member_auth.access_token;

    let (_, target_id) = create_and_login_user(
        &app,
        admin_token,
        "Target",
        "Remove",
        &format!("target.remove.{}@test.local", uid),
        "securepassword",
    )
    .await;

    let team_id = create_test_team(&app, admin_token, &format!("MemberRemoveTeam {}", uid)).await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &member_id, &member_role_id).await;
    add_member(&app, admin_token, &team_id, &target_id, &member_role_id).await;

    // Member tries to remove another member → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, target_id))
        .insert_header(("Authorization", format!("Bearer {}", member_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "Member should not be able to remove team members"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", target_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", member_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn member_cannot_update_member_role() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (member_auth, member_id) = create_and_login_user(
        &app,
        admin_token,
        "Member",
        "UpdRole",
        &format!("member.updrole.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let member_token = &member_auth.access_token;

    let (_, target_id) = create_and_login_user(
        &app,
        admin_token,
        "Target",
        "UpdRole",
        &format!("target.updrole.{}@test.local", uid),
        "securepassword",
    )
    .await;

    let team_id = create_test_team(&app, admin_token, &format!("MemberUpdRoleTeam {}", uid)).await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &member_id, &member_role_id).await;
    add_member(&app, admin_token, &team_id, &target_id, &member_role_id).await;

    // Member tries to update another member's role → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, target_id))
        .insert_header(("Authorization", format!("Bearer {}", member_token)))
        .set_json(json!({ "role_id": member_role_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "Member should not be able to update member roles"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", target_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", member_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn create_team_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": format!("LocHdrTeam-{}", Uuid::now_v7()), "descr": "temp"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let location = resp
        .headers()
        .get("Location")
        .expect("201 should include Location header");
    assert!(
        location.to_str().unwrap().contains("/api/v1.0/teams/"),
        "Location should contain team path"
    );
    let body: Value = test::read_body_json(resp).await;
    let team_id = body["team_id"].as_str().unwrap();

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn add_team_member_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Create temp team
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": format!("LocHdrMember-{}", Uuid::now_v7()), "descr": "temp"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap().to_string();

    // Create temp user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "LocMember",
            "lastname": "Test",
            "email": format!("locmember-{}@test.local", Uuid::now_v7()),
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_user: Value = test::read_body_json(resp).await;
    let new_user_id = new_user["user_id"].as_str().unwrap().to_string();

    // Get Member role_id
    let req = test::TestRequest::get()
        .uri("/api/v1.0/roles")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let roles = paginated_items(test::read_body_json(resp).await);
    let member_role_id = roles
        .iter()
        .find(|r| r["title"].as_str() == Some("Member"))
        .unwrap()["role_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Add member
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": new_user_id,
            "role_id": member_role_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let location = resp
        .headers()
        .get("Location")
        .expect("201 should include Location header");
    let loc_str = location.to_str().unwrap();
    assert!(
        loc_str.contains(&format!("/api/v1.0/teams/{}/users/", team_id)),
        "Location should contain team member path, got: {}",
        loc_str
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// #432 — Creating a team with a duplicate name returns 409
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn create_team_with_duplicate_name_returns_409() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Create first team
    let dup_team_name = format!("DupTeam-{}", Uuid::now_v7());
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": dup_team_name, "descr": "first"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "first team should be created");
    let team: Value = test::read_body_json(resp).await;
    let team_id = team["team_id"].as_str().unwrap().to_string();

    // Attempt to create second team with the same name → 409
    let req = test::TestRequest::post()
        .uri("/api/v1.0/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"tname": dup_team_name, "descr": "duplicate"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        409,
        "duplicate team name should return 409 Conflict"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// guard_admin_demotion — protect global Admins from Team Admin actions
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_demote_global_admin() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let tag = Uuid::now_v7();
    let ta_email = format!("ta-demote-{}@test.local", tag);
    let (ta_auth, ta_id) =
        create_and_login_user(&app, admin_token, "TA", "Demote", &ta_email, "password123").await;
    let ta_token = &ta_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("DemoteGuard-{}", tag)).await;
    let admin_role_id = find_role_id(&app, admin_token, "Admin").await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users?limit=100")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let admin_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some(ADMIN_EMAIL))
        .expect("admin user not found")["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    add_member(&app, admin_token, &team_id, &admin_id, &admin_role_id).await;
    add_member(&app, admin_token, &team_id, &ta_id, &ta_role_id).await;

    // Team Admin tries to demote global Admin to Member → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, admin_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({ "role_id": member_role_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin must not demote a global Admin"
    );
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["error"].as_str().unwrap().contains("global Admin"),
        "error message should mention global Admin: {:?}",
        body
    );

    // Ensure admin still has Admin role (unchanged)
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let members = paginated_items(test::read_body_json(resp).await);
    let admin_member = members
        .iter()
        .find(|m| m["email"].as_str() == Some(ADMIN_EMAIL))
        .expect("admin should still be in team");
    assert_eq!(
        admin_member["title"].as_str().unwrap(),
        "Admin",
        "admin's role should be unchanged"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, ta_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn team_admin_cannot_remove_global_admin_from_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let tag = Uuid::now_v7();
    let ta_email = format!("ta-remove-{}@test.local", tag);
    let (ta_auth, ta_id) =
        create_and_login_user(&app, admin_token, "TA", "Remove", &ta_email, "password123").await;
    let ta_token = &ta_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("RemoveGuard-{}", tag)).await;
    let admin_role_id = find_role_id(&app, admin_token, "Admin").await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users?limit=100")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let admin_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some(ADMIN_EMAIL))
        .expect("admin user not found")["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    add_member(&app, admin_token, &team_id, &admin_id, &admin_role_id).await;
    add_member(&app, admin_token, &team_id, &ta_id, &ta_role_id).await;

    // Team Admin tries to remove global Admin from team → 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, admin_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin must not remove a global Admin from a team"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, ta_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn global_admin_can_demote_another_global_admin() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let tag = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("AdminDemote-{}", tag)).await;
    let admin_role_id = find_role_id(&app, admin_token, "Admin").await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;

    // Create a second admin: create user, add as Admin in the team
    let email2 = format!("second-admin-demotion-{}@test.local", tag);
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SecondAdmin",
            "lastname": "Test",
            "email": email2,
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_admin: Value = test::read_body_json(resp).await;
    let new_admin_id = new_admin["user_id"].as_str().unwrap().to_string();

    add_member(&app, admin_token, &team_id, &new_admin_id, &admin_role_id).await;

    // First admin demotes second admin to Member → should succeed
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_admin_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({ "role_id": member_role_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "global admin should be able to demote another global admin"
    );

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_admin_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// guard_last_admin_membership — prevent zero-admin state
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn last_admin_cannot_demote_self() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users?limit=100")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let admin_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some(ADMIN_EMAIL))
        .expect("admin user not found")["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get the bootstrap team where admin has Admin role
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}/teams", admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let user_teams = paginated_items(test::read_body_json(resp).await);
    let admin_team = user_teams
        .iter()
        .find(|t| t["title"].as_str() == Some("Admin"))
        .expect("admin should be Admin in at least one team");
    let team_id = admin_team["team_id"].as_str().unwrap().to_string();

    let member_role_id = find_role_id(&app, admin_token, "Member").await;

    // Admin tries to demote self to Member → 403 (last admin)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({ "role_id": member_role_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "last admin must not be able to demote themselves"
    );
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("last global Admin"),
        "error should mention last admin: {:?}",
        body
    );
}

#[actix_web::test]
#[ignore]
async fn last_admin_cannot_remove_self_from_admin_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Get admin user ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users?limit=100")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let admin_id = users
        .iter()
        .find(|u| u["email"].as_str() == Some(ADMIN_EMAIL))
        .expect("admin user not found")["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get the bootstrap team where admin has Admin role
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}/teams", admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let user_teams = paginated_items(test::read_body_json(resp).await);
    let admin_team = user_teams
        .iter()
        .find(|t| t["title"].as_str() == Some("Admin"))
        .expect("admin should be Admin in at least one team");
    let team_id = admin_team["team_id"].as_str().unwrap().to_string();

    // Admin tries to remove self from team → 403 (last admin)
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "last admin must not be able to remove themselves from their admin team"
    );
}

#[actix_web::test]
#[ignore]
async fn demoting_admin_allowed_when_another_admin_exists() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let tag = Uuid::now_v7();
    let team_id = create_test_team(&app, admin_token, &format!("LastGuard-{}", tag)).await;
    let admin_role_id = find_role_id(&app, admin_token, "Admin").await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;

    // Create a second admin
    let email2 = format!("second-admin-lastguard-{}@test.local", tag);
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SecondAdmin",
            "lastname": "LastGuard",
            "email": email2,
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let new_admin: Value = test::read_body_json(resp).await;
    let new_admin_id = new_admin["user_id"].as_str().unwrap().to_string();

    // Add second user as Admin in the team
    add_member(&app, admin_token, &team_id, &new_admin_id, &admin_role_id).await;

    // Now demoting the second admin to Member should succeed (first admin still exists)
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_admin_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({ "role_id": member_role_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "demoting an admin should succeed when another admin exists"
    );

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, new_admin_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn remove_nonexistent_team_member_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let team_id = create_test_team(&app, token, &format!("RemMem404-{}", Uuid::now_v7())).await;
    let fake_user_id = Uuid::now_v7();

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1.0/teams/{}/users/{}",
            team_id, fake_user_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "removing nonexistent member should be 404"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn update_nonexistent_team_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let fake_id = Uuid::now_v7();

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/teams/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"tname": "Ghost Team", "descr": null}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "PUT nonexistent team should be 404");
}

#[actix_web::test]
#[ignore]
async fn admin_can_assign_admin_role_via_add_member() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a temp user and a team
    let uid = Uuid::now_v7();
    let test_email = format!("admin-assign-{}@test.local", uid);
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "Temp",
            "lastname": "Admin",
            "email": test_email,
            "password": "ValidPassword123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    let team_id = create_test_team(&app, admin_token, &format!("AdminAssign-{}", uid)).await;
    let admin_role_id = find_role_id(&app, admin_token, "Admin").await;

    // Admin adds user as Admin role → should succeed
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1.0/teams/{}/users", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({"user_id": user_id, "role_id": admin_role_id}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "admin should be able to assign Admin role"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}/users/{}", team_id, user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn get_team_users_for_nonexistent_team_returns_empty() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let fake_team_id = Uuid::now_v7();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/teams/{}/users", fake_team_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "GET /teams/{{nonexistent}}/users should return 200"
    );
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body.as_array().is_none_or(|a| a.is_empty()),
        "should return empty list for nonexistent team"
    );
}
