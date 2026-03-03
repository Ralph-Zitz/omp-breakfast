pub mod config;
pub mod db;
pub mod errors;
pub mod from_row;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod server;
pub mod validate;

use argon2::{Algorithm, Argon2, Params, Version};

/// Explicit Argon2id hasher — pins algorithm, version, and parameters so that
/// a future `argon2` crate update cannot silently weaken hashing defaults.
///
/// Used by both password hashing (`db::users`) and password verification
/// (`middleware::auth`). A single source of truth prevents parameter drift.
pub fn argon2_hasher() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
}
