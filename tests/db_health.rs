//! Database health check tests.

#[path = "common/db.rs"]
mod db_helpers;

use breakfast::db;
use db_helpers::*;

// ===========================================================================
// Group 1: Health / connectivity
// ===========================================================================

#[actix_web::test]
#[ignore]
async fn check_db_succeeds() {
    let client = test_client().await;
    db::check_db(&client)
        .await
        .expect("check_db should succeed on a healthy DB");
}
