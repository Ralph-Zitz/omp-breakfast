//! User CRUD, RBAC ownership, password changes, self-delete, and email-based delete tests.

mod common;

use actix_web::test;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use common::*;
use serde_json::{Value, json};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// RBAC enforcement
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn delete_other_user_returns_forbidden() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a non-admin user (Member with no team = no special role)
    let suffix = Uuid::now_v7();
    let member_email = format!("member-del-{suffix}@test.local");
    let (member_auth, _member_id) = create_and_login_user(
        &app,
        admin_token,
        "Member",
        "Del",
        &member_email,
        "securepassword",
    )
    .await;

    // Get list of users to find another user's ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header((
            "Authorization",
            format!("Bearer {}", member_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);

    // Find a user that is not the member
    let other_user = users
        .iter()
        .find(|u| u["email"].as_str() != Some(member_email.as_str()))
        .unwrap();
    let other_id = other_user["user_id"].as_str().unwrap();

    // Try to delete the other user → should be 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", other_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", member_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "should not be able to delete another user"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", _member_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn update_other_user_returns_forbidden() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a non-admin user
    let suffix = Uuid::now_v7();
    let member_email = format!("member-upd-{suffix}@test.local");
    let (member_auth, member_id) = create_and_login_user(
        &app,
        admin_token,
        "Member",
        "Upd",
        &member_email,
        "securepassword",
    )
    .await;

    // Get list of users to find another user's ID
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header((
            "Authorization",
            format!("Bearer {}", member_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);

    let other_user = users
        .iter()
        .find(|u| u["email"].as_str() != Some(member_email.as_str()))
        .unwrap();
    let other_id = other_user["user_id"].as_str().unwrap();

    // Try to update the other user → should be 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", other_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", member_auth.access_token),
        ))
        .set_json(json!({
            "firstname": "Hacked",
            "lastname": "Name",
            "email": "hacked@example.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "should not be able to update another user"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", member_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Validation rejection
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_user_with_invalid_email_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({
            "firstname": "Test",
            "lastname": "User",
            "email": "not-an-email",
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422, "invalid email should be rejected");
}

// ---------------------------------------------------------------------------
// 404 responses for non-existent resources
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn get_nonexistent_user_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

    let fake_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "non-existent user should return 404");
}

#[actix_web::test]
#[ignore]
async fn create_duplicate_user_returns_409() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Try to create a user with the same email as the admin → 409
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "Duplicate",
            "lastname": "Admin",
            "email": ADMIN_EMAIL,
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 409, "duplicate email should return 409");
}

// ---------------------------------------------------------------------------
// Admin bypass: admin can update/delete other users
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn admin_can_update_other_user() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Create a temporary user to update
    let temp_email = format!("temp-admin-upd-{}@test.local", Uuid::now_v7());
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "Temp",
            "lastname": "User",
            "email": temp_email,
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap();

    // Admin updates the other user → should succeed
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "Updated",
            "lastname": "ByAdmin",
            "email": temp_email
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should be able to update another user"
    );
    let updated: Value = test::read_body_json(resp).await;
    assert_eq!(updated["firstname"], "Updated");

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn admin_can_delete_other_user() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Create a temporary user to delete
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "Temp",
            "lastname": "Delete",
            "email": format!("temp-admin-del-{}@test.local", Uuid::now_v7()),
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap();

    // Admin deletes the other user → should succeed
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should be able to delete another user"
    );
}

// ---------------------------------------------------------------------------
// RBAC: create_user requires Admin or Team Admin
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_admin_non_team_admin_cannot_create_user() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a regular Member user
    let uid = Uuid::now_v7();
    let email = format!("member-{}@test.local", uid);
    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "Regular",
        "Member",
        &email,
        "securepassword",
    )
    .await;

    // Give them a team with the Member role
    let team_id = create_test_team(&app, admin_token, &format!("MemberTeam-{}", uid)).await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &user_id, &member_role_id).await;

    // Regular Member tries to create a user → should be 403
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .set_json(json!({
            "firstname": "Blocked",
            "lastname": "User",
            "email": format!("blocked-{}@test.local", Uuid::now_v7()),
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "regular member should not be able to create users"
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
async fn team_admin_can_create_user() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a Team Admin user
    let uid = Uuid::now_v7();
    let ta_email = format!("ta-create-{}@test.local", uid);
    let (ta_auth, ta_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TeamAdmin",
        "Creator",
        &ta_email,
        "securepassword",
    )
    .await;
    let ta_token = &ta_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("TACreateTeam-{}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_user_id, &ta_role_id).await;

    // Team Admin creates a user → should succeed
    let new_email = format!("created-by-ta-{}@test.local", Uuid::now_v7());
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "firstname": "Created",
            "lastname": "ByTeamAdmin",
            "email": new_email,
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "team admin should be able to create users"
    );
    let user: Value = test::read_body_json(resp).await;
    let new_user_id = user["user_id"].as_str().unwrap();

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
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

// ---------------------------------------------------------------------------
// RBAC: Team Admin can update/delete users in their team, but not outside
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn team_admin_can_update_user_in_their_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();

    // Create Team Admin user
    let ta_email = format!("ta-upd-{}@test.local", uid);
    let (ta_auth, ta_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "Updater",
        &ta_email,
        "securepassword",
    )
    .await;
    let ta_token = &ta_auth.access_token;

    // Create team and add TA as Team Admin
    let team_id = create_test_team(&app, admin_token, &format!("UpdTeam-{}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_user_id, &ta_role_id).await;

    // Create target user and add as Member of the same team
    let target_email = format!("target-upd-{}@test.local", uid);
    let (_target_auth, target_user_id) = create_and_login_user(
        &app,
        admin_token,
        "Target",
        "User",
        &target_email,
        "securepassword",
    )
    .await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(
        &app,
        admin_token,
        &team_id,
        &target_user_id,
        &member_role_id,
    )
    .await;

    // Team Admin updates target user → should succeed (same team)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", target_user_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "firstname": "Updated",
            "lastname": "User",
            "email": target_email
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "team admin should be able to update a user in their team"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", target_user_id))
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
async fn team_admin_cannot_update_user_outside_their_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();

    // Create Team Admin user
    let ta_email = format!("ta-noext-{}@test.local", uid);
    let (ta_auth, ta_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "NoExt",
        &ta_email,
        "securepassword",
    )
    .await;
    let ta_token = &ta_auth.access_token;

    // Team where TA is Team Admin
    let admin_team_id = create_test_team(&app, admin_token, &format!("TATeam-{}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &admin_team_id, &ta_user_id, &ta_role_id).await;

    // Team where TA is only a Member
    let member_team_id =
        create_test_team(&app, admin_token, &format!("MemberOnlyTeam-{}", uid)).await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(
        &app,
        admin_token,
        &member_team_id,
        &ta_user_id,
        &member_role_id,
    )
    .await;

    // Create a temp user and add only to the member-only team
    let temp_email = format!("outside-{}@test.local", uid);
    let (_temp_auth, temp_user_id) = create_and_login_user(
        &app,
        admin_token,
        "Outside",
        "User",
        &temp_email,
        "securepassword",
    )
    .await;
    add_member(
        &app,
        admin_token,
        &member_team_id,
        &temp_user_id,
        &member_role_id,
    )
    .await;

    // TA tries to update the temp user → should be 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", temp_user_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "firstname": "Hacked",
            "lastname": "Name",
            "email": temp_email
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin should not update a user outside their administered teams"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", temp_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", ta_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", admin_team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", member_team_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn team_admin_can_delete_user_in_their_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();

    // Create Team Admin user
    let ta_email = format!("ta-del-{}@test.local", uid);
    let (ta_auth, ta_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "Deleter",
        &ta_email,
        "securepassword",
    )
    .await;
    let ta_token = &ta_auth.access_token;

    // Create team and add TA as Team Admin
    let team_id = create_test_team(&app, admin_token, &format!("DelTeam-{}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_user_id, &ta_role_id).await;

    // Create a temp user and add as Member to the same team
    let temp_email = format!("deletable-{}@test.local", uid);
    let (_temp_auth, temp_user_id) = create_and_login_user(
        &app,
        admin_token,
        "Deletable",
        "ByTA",
        &temp_email,
        "securepassword",
    )
    .await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &temp_user_id, &member_role_id).await;

    // Team Admin deletes the user → should succeed (user is in their team)
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", temp_user_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "team admin should be able to delete a user in their team"
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
async fn team_admin_cannot_delete_user_outside_their_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let uid = Uuid::now_v7();

    // Create a Team Admin user with a team
    let ta_email = format!("ta-nodelo-{}@test.local", uid);
    let (ta_auth, ta_user_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "NoDelo",
        &ta_email,
        "securepassword",
    )
    .await;
    let ta_token = &ta_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("NoDelTeam-{}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_user_id, &ta_role_id).await;

    // Create a temp user with no team membership at all
    let orphan_email = format!("orphan-{}@test.local", uid);
    let (_orphan_auth, orphan_user_id) = create_and_login_user(
        &app,
        admin_token,
        "Orphan",
        "User",
        &orphan_email,
        "securepassword",
    )
    .await;

    // TA tries to delete the orphan user → should be 403
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", orphan_user_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin should not delete a user not in any of their teams"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", orphan_user_id))
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
async fn user_can_still_update_self() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a regular user
    let uid = Uuid::now_v7();
    let email = format!("selfupd-{}@test.local", uid);
    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "Self",
        "Updater",
        &email,
        "securepassword",
    )
    .await;
    let token = &user_auth.access_token;

    // Self-update → should succeed
    let new_email = format!("selfupd-new-{}@test.local", uid);
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "SelfUpdated",
            "lastname": "Updater",
            "email": new_email
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "user should be able to update their own account"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ---------------------------------------------------------------------------
// Team Admin user scoping: can only modify users in shared teams
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn team_admin_can_update_user_in_shared_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (ta_auth, ta_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "User",
        &format!("ta.shared.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let ta_token = &ta_auth.access_token;

    let u1_email = format!("u1.shared.{}@test.local", uid);
    let (_, u1_id) = create_and_login_user(
        &app,
        admin_token,
        "U1",
        "Target",
        &u1_email,
        "securepassword",
    )
    .await;

    let team_id = create_test_team(&app, admin_token, &format!("SharedTeam {}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &ta_id, &ta_role_id).await;
    add_member(&app, admin_token, &team_id, &u1_id, &member_role_id).await;

    // Team Admin updates U1 → should succeed (shared team)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", u1_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "firstname": "U1",
            "lastname": "Target",
            "email": u1_email
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "team admin should update users in their team"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", u1_id))
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
async fn team_admin_cannot_update_user_outside_shared_team() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    // Create TA user and add to a team as Team Admin
    let (ta_auth, ta_id) = create_and_login_user(
        &app,
        admin_token,
        "TA",
        "Outside",
        &format!("ta.outside.{}@test.local", uid),
        "securepassword",
    )
    .await;
    let ta_token = &ta_auth.access_token;

    let team_id = create_test_team(&app, admin_token, &format!("OutsideTeam {}", uid)).await;
    let ta_role_id = find_role_id(&app, admin_token, "Team Admin").await;
    add_member(&app, admin_token, &team_id, &ta_id, &ta_role_id).await;

    // Create an isolated user NOT in any of TA's teams
    let isolated_email = format!("isolated.{}@test.local", uid);
    let (_, isolated_id) = create_and_login_user(
        &app,
        admin_token,
        "Isolated",
        "User",
        &isolated_email,
        "securepassword",
    )
    .await;

    // TA tries to update isolated user → should be 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", isolated_id))
        .insert_header(("Authorization", format!("Bearer {}", ta_token)))
        .set_json(json!({
            "firstname": "Hacked",
            "lastname": "User",
            "email": isolated_email
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "team admin should NOT update users outside their teams"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", isolated_id))
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
// Member cannot create users (requires admin or team admin)
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn member_cannot_create_user() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "Member",
        "Create",
        &format!("member.create.{}@test.local", uid),
        "securepassword",
    )
    .await;

    let team_id = create_test_team(&app, admin_token, &format!("MemberCreateTeam {}", uid)).await;
    let member_role_id = find_role_id(&app, admin_token, "Member").await;
    add_member(&app, admin_token, &team_id, &user_id, &member_role_id).await;

    // Member tries to create a user → 403
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .set_json(json!({
            "firstname": "Forbidden",
            "lastname": "User",
            "email": format!("forbidden.create.{}@test.local", uid),
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "regular member should not be able to create users"
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
// delete_user_by_email RBAC fallback — non-admin cannot discover whether an
// email exists; admin gets a proper 404 for a nonexistent email.
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn non_admin_delete_by_email_nonexistent_returns_403() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;
    let uid = Uuid::now_v7();

    let (user_auth, user_id) = create_and_login_user(
        &app,
        admin_token,
        "NonAdmin",
        "Delete",
        &format!("nonadmin.del.{}@test.local", uid),
        "securepassword",
    )
    .await;

    let req = test::TestRequest::delete()
        .uri("/api/v1.0/users/email/nonexistent@example.com")
        .insert_header((
            "Authorization",
            format!("Bearer {}", user_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "non-admin should get 403 even when email does not exist (prevents info leakage)"
    );

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let _ = test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn admin_delete_by_email_nonexistent_returns_200() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

    let req = test::TestRequest::delete()
        .uri("/api/v1.0/users/email/nonexistent@example.com")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "admin should get 200 with deleted:false to suppress email oracle"
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["deleted"], json!(false));
}

// ---------------------------------------------------------------------------
// Create-user → authenticate round-trip (validates Argon2 hashing in create)
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_user_then_authenticate_round_trip() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = format!("roundtrip-{}@test.local", Uuid::now_v7());
    let test_password = "RoundTrip!Pass123";

    // 1. Create a new user via the API
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "RoundTrip",
            "lastname": "Test",
            "email": test_email,
            "password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        201,
        "admin should be able to create a new user"
    );
    let user: Value = test::read_body_json(resp).await;
    let new_user_id = user["user_id"].as_str().unwrap();

    // 2. Authenticate the newly created user via Basic Auth
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, test_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "newly created user should authenticate successfully (password must be Argon2 hashed)"
    );
    let new_user_auth: Auth = test::read_body_json(resp).await;
    assert!(
        !new_user_auth.access_token.is_empty(),
        "should receive a non-empty access token"
    );
    assert!(
        !new_user_auth.refresh_token.is_empty(),
        "should receive a non-empty refresh token"
    );

    // 3. Use the new user's token to access a protected endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header((
            "Authorization",
            format!("Bearer {}", new_user_auth.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "new user should access their own profile with the issued token"
    );
    let fetched_user: Value = test::read_body_json(resp).await;
    assert_eq!(fetched_user["email"].as_str().unwrap(), test_email);

    // Clean up: admin deletes the created user
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", new_user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "cleanup: admin should delete temp user");
}

// ---------------------------------------------------------------------------
// Password update → re-login round-trip
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn update_user_password_then_reauthenticate() {
    let state = test_state().await;
    let app = test_app!(state);

    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = format!("pwchange-{}@test.local", Uuid::now_v7());
    let original_password = "OriginalPass!123";
    let new_password = "ChangedPass!456";

    // 1. Create a temp user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "PwChange",
            "lastname": "Test",
            "email": test_email,
            "password": original_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // 2. Authenticate with the original password → should succeed
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, original_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "original password should work before change"
    );

    // 3. Update password via PUT
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "PwChange",
            "lastname": "Test",
            "email": test_email,
            "password": new_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "password update should succeed");

    // 4. Authenticate with the NEW password → should succeed
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, new_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "new password should work after change");

    // 5. Authenticate with the OLD password → should fail
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, original_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        401,
        "old password must not work after change"
    );

    // Clean up: admin deletes the temp user
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "cleanup: admin should delete temp user");
}

// ===========================================================================
// user_teams endpoint (#173)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn user_teams_returns_admin_teams() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;
    let uid = Uuid::now_v7();

    // Create a team and add admin to it
    let team_name = format!("AdminTeams {}", uid);
    let team_id = create_test_team(&app, token, &team_name).await;

    // Get admin user_id
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let users = paginated_items(test::read_body_json(resp).await);
    let admin = users
        .iter()
        .find(|u| u["email"].as_str() == Some(ADMIN_EMAIL))
        .expect("admin user should exist");
    let admin_id = admin["user_id"].as_str().unwrap();

    let admin_role_id = find_role_id(&app, token, "Admin").await;
    add_member(&app, token, &team_id, admin_id, &admin_role_id).await;

    // GET /api/v1.0/users/{admin_id}/teams
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}/teams", admin_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "user_teams should return 200");
    let teams = paginated_items(test::read_body_json(resp).await);
    assert!(
        teams
            .iter()
            .any(|t| t["tname"].as_str() == Some(&*team_name)),
        "admin should be member of the created team"
    );
    // Verify membership timestamps are present (#115)
    let team_entry = teams
        .iter()
        .find(|t| t["tname"].as_str() == Some(&*team_name))
        .unwrap();
    assert!(
        team_entry["joined"].is_string(),
        "joined timestamp should be present"
    );
    assert!(
        team_entry["role_changed"].is_string(),
        "role_changed timestamp should be present"
    );

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/teams/{}", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
#[ignore]
async fn user_teams_returns_empty_for_user_with_no_teams() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let token = &admin_auth.access_token;

    // Create a temp user (not added to any team)
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "NoTeam",
            "lastname": "User",
            "email": format!("noteam-{}@test.local", Uuid::now_v7()),
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap();

    // GET user_teams → should be empty
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}/teams", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let teams = paginated_items(test::read_body_json(resp).await);
    assert!(teams.is_empty(), "new user should have no teams");

    // Clean up
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// Delete user by email success (#206)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn admin_can_delete_user_by_email() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    // Create a temp user
    let email = format!("deleteme-{}@test.local", Uuid::now_v7());
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "DeleteByEmail",
            "lastname": "Test",
            "email": email,
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // Delete by email
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/email/{}", email))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "admin should delete user by email");
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["deleted"], true);
}

// ---------------------------------------------------------------------------
// #294 — Location header on remaining create endpoints
// ---------------------------------------------------------------------------

#[actix_web::test]
#[ignore]
async fn create_user_returns_location_header() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let token = &auth.access_token;

    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "firstname": "LocHdr",
            "lastname": "Test",
            "email": format!("lochdr-{}@test.local", Uuid::now_v7()),
            "password": "securepassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let location = resp
        .headers()
        .get("Location")
        .expect("201 should include Location header");
    assert!(
        location.to_str().unwrap().contains("/api/v1.0/users/"),
        "Location should contain user path"
    );
    let body: Value = test::read_body_json(resp).await;
    let user_id = body["user_id"].as_str().unwrap();

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// #397 — Self-password-change verification: missing, wrong, correct
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn self_password_change_without_current_password_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = format!("selfpw-nocur-{}@test.local", Uuid::now_v7());
    let test_password = "OriginalPass!123";

    // Create user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "NoCurrent",
            "email": &test_email,
            "password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // Login as the user to get their own token
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, test_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let user_auth: Auth = test::read_body_json(resp).await;
    let user_token = &user_auth.access_token;

    // Self-update password WITHOUT current_password → 422
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "NoCurrent",
            "email": test_email,
            "password": "NewPassword!456"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        422,
        "self-password-change without current_password should be 422"
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
async fn self_password_change_with_wrong_current_password_returns_403() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = format!("selfpw-wrong-{}@test.local", Uuid::now_v7());
    let test_password = "OriginalPass!123";

    // Create user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "WrongCurrent",
            "email": &test_email,
            "password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // Login as the user
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, test_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let user_auth: Auth = test::read_body_json(resp).await;
    let user_token = &user_auth.access_token;

    // Self-update password with WRONG current_password → 403
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "WrongCurrent",
            "email": test_email,
            "password": "NewPassword!456",
            "current_password": "TotallyWrongPassword"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "self-password-change with wrong current_password should be 403"
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
async fn self_password_change_with_correct_current_password_succeeds() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = format!("selfpw-ok-{}@test.local", Uuid::now_v7());
    let test_password = "OriginalPass!123";
    let new_password = "ChangedPass!456";

    // Create user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "Correct",
            "email": &test_email,
            "password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // Login as the user
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, test_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let user_auth: Auth = test::read_body_json(resp).await;
    let user_token = &user_auth.access_token;

    // Self-update password with CORRECT current_password → 200
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .set_json(json!({
            "firstname": "SelfPw",
            "lastname": "Correct",
            "email": test_email,
            "password": new_password,
            "current_password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "self-password-change with correct current_password should succeed"
    );

    // Verify new password works
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, new_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "new password should work after change");

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}

// ===========================================================================
// #401 — Self-delete user at API level
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn non_admin_user_can_delete_own_account() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    let test_email = format!("selfdelete-{}@test.local", Uuid::now_v7());
    let test_password = "SelfDelete!123";

    // Create a test user
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SelfDel",
            "lastname": "Test",
            "email": &test_email,
            "password": test_password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // Login as the user
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", test_email, test_password))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let user_auth: Auth = test::read_body_json(resp).await;
    let user_token = &user_auth.access_token;

    // Delete own account
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "non-admin user should be able to delete their own account"
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["deleted"], true);

    // Verify user no longer exists
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "deleted user should not be found");
}

// ===========================================================================
// #399 — Last admin cannot delete themselves
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn last_admin_cannot_delete_own_account() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // The admin is the only admin. Attempting self-delete should fail.
    // First, find the admin's user_id from the token.
    let req = test::TestRequest::get()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let users = paginated_items(body);
    let admin_user = users
        .iter()
        .find(|u| u["email"].as_str() == Some(ADMIN_EMAIL))
        .expect("admin user should exist");
    let admin_id = admin_user["user_id"].as_str().unwrap();

    // Attempt to delete self
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", admin_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "last admin should not be able to delete their own account"
    );
}

// ===========================================================================
// #435 — Non-admin user can delete their own account by email
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn user_can_delete_own_account_by_email() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a regular (non-admin) user
    let email = format!("selfdelete435-{}@test.local", Uuid::now_v7());
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "SelfDelete",
            "lastname": "ByEmail",
            "email": email,
            "password": "selfdeletepass435"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "admin should create the user");
    let created: Value = test::read_body_json(resp).await;
    let user_id = created["user_id"].as_str().unwrap().to_string();

    // User logs in
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:selfdeletepass435", email))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "user should be able to login");
    let user_auth: Auth = test::read_body_json(resp).await;
    let user_token = &user_auth.access_token;

    // User deletes own account by email
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/email/{}", email))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "user should be able to delete own account by email"
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["deleted"], true);

    // Verify the user no longer exists
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "deleted user should no longer be found");
}

// ===========================================================================
// UPDATE nonexistent resources → 404 (#300)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn update_nonexistent_user_returns_404() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;
    let fake_id = Uuid::now_v7();

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .set_json(json!({"firstname": "Ghost", "lastname": "User", "email": "ghost@test.local"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "PUT nonexistent user should be 404");
}

// ===========================================================================
// Misc edge cases (#390, #392)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn delete_user_by_email_invalid_format_returns_422() {
    let state = test_state().await;
    let app = test_app!(state);
    let auth: Auth = register_admin(&app).await;

    let req = test::TestRequest::delete()
        .uri("/api/v1.0/users/email/not-an-email")
        .insert_header(("Authorization", format!("Bearer {}", auth.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422, "malformed email should return 422");
}

// ===========================================================================
// Email change dual cache invalidation (#391)
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn update_user_email_invalidates_both_cache_keys() {
    let state = test_state().await;
    let app = test_app!(state);
    let admin_auth: Auth = register_admin(&app).await;
    let admin_token = &admin_auth.access_token;

    // Create a temp user
    let uid = Uuid::now_v7();
    let original_email = format!("cache-orig-{}@test.local", uid);
    let new_email = format!("cache-new-{}@test.local", uid);
    let req = test::TestRequest::post()
        .uri("/api/v1.0/users")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "Cache",
            "lastname": "Test",
            "email": original_email,
            "password": "Very Secret"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let user: Value = test::read_body_json(resp).await;
    let user_id = user["user_id"].as_str().unwrap().to_string();

    // Login with original email to populate cache
    let _user_auth: Auth = login_as(&app, &original_email, "Very Secret").await;

    // Update the user's email
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "firstname": "Cache",
            "lastname": "Test",
            "email": new_email
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Login with old email should fail (404 user not found → 401)
    let req = test::TestRequest::post()
        .uri("/auth")
        .peer_addr(PEER)
        .insert_header((
            "Authorization",
            format!(
                "Basic {}",
                STANDARD.encode(format!("{}:Very Secret", original_email))
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        401,
        "login with old email should fail after email change"
    );

    // Login with new email should succeed
    let _new_auth: Auth = login_as(&app, &new_email, "Very Secret").await;

    // Cleanup
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1.0/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    test::call_service(&app, req).await;
}
