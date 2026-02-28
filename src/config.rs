use config::{Config, ConfigError, Environment, File};
use git_version::git_describe;
use serde::Deserialize;
use std::env;
use tracing::instrument;

#[derive(Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub secret: String,
    pub jwtsecret: String,
    pub s3_key_id: String,
    pub s3_key_secret: String,
    pub git_version: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct Database {
    pub url: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: Database,
    pub pg: deadpool_postgres::Config,
    pub db_ca_cert: Option<String>,
}

impl Settings {
    #[instrument]
    pub fn new() -> Result<Self, ConfigError> {
        let env = env::var("ENV").unwrap_or("development".into());
        let cfg = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", env)).required(false))
            .add_source(Environment::default().separator("_"))
            .build()?;
        let mut settings: Settings = cfg.try_deserialize()?;
        settings.server.git_version = git_describe!("--tags").to_string();
        Ok(settings)
    }
}
