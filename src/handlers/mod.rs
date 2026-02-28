pub mod items;
pub mod roles;
pub mod teams;
pub mod users;

use crate::{db, errors::Error, models::*};
use actix_web::{web::Data, HttpResponse, Responder};
use deadpool_postgres::{Client, Pool};
use tracing::{error, instrument};

/* Utility Functions */
pub async fn get_client(pool: Pool) -> Result<Client, Error> {
    pool.get().await.map_err(|err| {
        error!(error = %err, "Failed to acquire DB client from pool");
        err.into()
    })
}

// API Health endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health check", body = StatusResponse)
    ),
)]
#[instrument(skip(state), level = "debug")]
pub async fn get_health(state: Data<State>) -> Result<impl Responder, Error> {
    //    let client: Client = get_client(state.pool.clone()).await?;
    let Ok(client) = get_client(state.pool.clone()).await else {
        return Ok(HttpResponse::Ok().json(StatusResponse { up: false }));
    };
    let result = db::check_db(&client).await;

    result.map(|ok| HttpResponse::Ok().json(StatusResponse { up: ok }))
}
