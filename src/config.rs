use config::{Config, ConfigError, Environment, File};
use git_version::git_version;
use serde::Deserialize;
use std::env;
use tracing::instrument;

pub fn default_http_redirect_port() -> u16 {
    80
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// HTTP redirect port for the HTTP→HTTPS redirect server (default: 80).
    #[serde(default = "default_http_redirect_port")]
    pub http_redirect_port: u16,
    /// Canary field: not used at runtime, but its production check ensures
    /// that operators have reviewed and customised the config before deploying.
    /// If this is still the default "Very Secret" in production, the server
    /// panics at startup.
    pub secret: String,
    pub jwtsecret: String,
    pub git_version: String,
}

#[derive(Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
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
            .add_source(Environment::with_prefix("BREAKFAST").separator("_"))
            .build()?;
        let mut settings: Settings = cfg.try_deserialize()?;
        settings.server.git_version =
            git_version!(args = ["--tags", "--always"], fallback = "unknown").to_string();
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize config tests because they modify process-wide env vars.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Helper: remove env vars that could leak between tests.
    ///
    /// SAFETY: All config tests are serialized via ENV_LOCK so no other
    /// thread reads these env vars concurrently.
    unsafe fn clean_env() {
        unsafe {
            std::env::remove_var("BREAKFAST_SERVER_PORT");
            std::env::remove_var("BREAKFAST_SERVER_SECRET");
            std::env::remove_var("BREAKFAST_SERVER_JWTSECRET");
            std::env::remove_var("BREAKFAST_SERVER_HOST");
            // Force the "development" environment so the test overlay is loaded
            // only when the file exists (required(false) means it's optional).
            std::env::set_var("ENV", "development");
        }
    }

    #[test]
    fn settings_loads_default_config() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { clean_env() };

        let settings = Settings::new().expect("should load default config");
        assert_eq!(settings.server.host, "0.0.0.0");
        assert_eq!(settings.server.port, 8080);
        assert_eq!(settings.server.secret, "Very Secret");
        assert_eq!(settings.server.jwtsecret, "Very Secret");
    }

    #[test]
    fn settings_env_override_port() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { clean_env() };
        // SAFETY: serialized via ENV_LOCK
        unsafe { std::env::set_var("BREAKFAST_SERVER_PORT", "9090") };

        let settings = Settings::new().expect("should load with port override");
        assert_eq!(settings.server.port, 9090);

        // SAFETY: serialized via ENV_LOCK
        unsafe { std::env::remove_var("BREAKFAST_SERVER_PORT") };
    }

    #[test]
    fn settings_env_override_secret() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { clean_env() };
        // SAFETY: serialized via ENV_LOCK
        unsafe { std::env::set_var("BREAKFAST_SERVER_SECRET", "custom-secret-value") };

        let settings = Settings::new().expect("should load with secret override");
        assert_eq!(settings.server.secret, "custom-secret-value");

        // SAFETY: serialized via ENV_LOCK
        unsafe { std::env::remove_var("BREAKFAST_SERVER_SECRET") };
    }

    #[test]
    fn settings_git_version_is_populated() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { clean_env() };

        let settings = Settings::new().expect("should load config");
        assert!(
            !settings.server.git_version.is_empty(),
            "git_version should not be empty"
        );
        // In a git repo the version should be a tag or commit hash, not the
        // fallback value "unknown".
        assert_ne!(
            settings.server.git_version, "unknown",
            "git_version should resolve to a real value in a git repo"
        );
    }

    #[test]
    fn settings_pg_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { clean_env() };

        let settings = Settings::new().expect("should load config");
        assert_eq!(settings.pg.user, Some("actix".to_string()));
        assert_eq!(settings.pg.dbname, Some("actix".to_string()));
        assert_eq!(settings.pg.port, Some(5432));
        assert_eq!(settings.pg.password, Some("actix".to_string()));
    }

    #[test]
    fn settings_nonexistent_env_file_is_ignored() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { clean_env() };
        // Set ENV to a value with no matching config file — should still work
        // because the environment file source is required(false).
        // SAFETY: serialized via ENV_LOCK
        unsafe { std::env::set_var("ENV", "nonexistent_environment") };

        let settings = Settings::new().expect("should load with missing env file");
        // Should still have defaults from config/default.yml
        assert_eq!(settings.server.host, "0.0.0.0");
        assert_eq!(settings.server.port, 8080);

        // SAFETY: serialized via ENV_LOCK
        unsafe { std::env::set_var("ENV", "development") };
    }
}
