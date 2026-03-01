use crate::{config::Settings, db, models::State, routes::routes};
use actix_cors::Cors;
use actix_files::Files;
use actix_web::{App, HttpServer, web::Data};
use deadpool_postgres::{Pool, Runtime};
use flurry::HashMap;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider};
use opentelemetry_stdout as stdout;
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use rustls_pki_types::PrivateKeyDer;
use std::{env, fs::File, io::BufReader, path::Path, time::Duration};
use tokio_postgres_rustls::MakeRustlsConnect;
use tracing::{error, info, warn};
use tracing_actix_web::TracingLogger;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt};
use utoipa_swagger_ui::Config as SwaggerConfig;

/// Interval between token blacklist cleanup runs (1 hour).
const TOKEN_CLEANUP_INTERVAL: Duration = Duration::from_secs(3600);

/// Background task that periodically removes expired entries from the
/// `token_blacklist` table so it doesn't grow unbounded.
fn spawn_token_cleanup_task(pool: Pool) {
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(TOKEN_CLEANUP_INTERVAL).await;
            match pool.get().await {
                Ok(client) => match db::cleanup_expired_tokens(&client).await {
                    Ok(count) if count > 0 => {
                        info!(
                            deleted = count,
                            "Cleaned up expired entries from token blacklist"
                        );
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!(error = %e, "Failed to clean up expired tokens");
                    }
                },
                Err(e) => {
                    error!(error = %e, "Failed to acquire DB client for token cleanup");
                }
            }
        }
    });
}

const FRONTEND_DIR: &str = "frontend/dist";

fn tls_config() -> Result<ServerConfig, crate::errors::Error> {
    let cert_path = "localhost.pem";
    let key_path = "localhost_key.pem";

    let cert_file = &mut BufReader::new(File::open(cert_path).map_err(|e| {
        error!("Failed to open TLS certificate file '{}': {}", cert_path, e);
        crate::errors::Error::Io(e)
    })?);
    let key_file = &mut BufReader::new(File::open(key_path).map_err(|e| {
        error!("Failed to open TLS private key file '{}': {}", key_path, e);
        crate::errors::Error::Io(e)
    })?);

    let cert_chain: Vec<_> = certs(cert_file)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            error!("Failed to parse TLS certificate from '{}': {}", cert_path, e);
            crate::errors::Error::Io(e)
        })?;
    info!("TLS certificate loaded successfully from '{}'", cert_path);

    let keys = pkcs8_private_keys(key_file)
        .next()
        .ok_or_else(|| {
            error!("No PKCS8 private key found in '{}'", key_path);
            crate::errors::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("No PKCS8 private key found in '{}'", key_path),
            ))
        })?
        .map_err(|e| {
            error!("Failed to parse TLS private key from '{}': {}", key_path, e);
            crate::errors::Error::Io(e)
        })?;
    info!("TLS private key loaded successfully from '{}'", key_path);

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, PrivateKeyDer::Pkcs8(keys))
        .map_err(|e| {
            error!("Failed to build TLS configuration: {}", e);
            crate::errors::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;
    config.alpn_protocols.push(b"http/1.1".to_vec());
    config.alpn_protocols.push(b"h2".to_vec());

    info!("TLS configuration initialized successfully");
    Ok(config)
}

fn db_tls_connector(settings: &Settings) -> MakeRustlsConnect {
    let mut root_store = rustls::RootCertStore::empty();
    if let Some(ca_cert_path) = &settings.db_ca_cert {
        let ca_file = &mut BufReader::new(File::open(ca_cert_path).unwrap_or_else(|e| {
            panic!("Failed to open DB CA certificate '{}': {}", ca_cert_path, e);
        }));
        let ca_certs: Vec<_> = certs(ca_file)
            .map(|c| c.expect("invalid CA cert"))
            .collect();
        for cert in ca_certs {
            root_store.add(cert).expect("failed to add CA cert");
        }
        info!(
            "Database CA certificate loaded successfully from '{}'",
            ca_cert_path
        );
    } else {
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        info!("Using default webpki root certificates for database TLS");
    }
    let tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    MakeRustlsConnect::new(tls_config)
}

pub async fn server() -> Result<(), Box<dyn std::error::Error>> {
    // Install color-eyre for colorized panic reports and error context
    color_eyre::install()?;

    // Logging
    LogTracer::init().expect("Unable to set up log tracer!");
    global::set_text_map_propagator(TraceContextPropagator::new());
    let app_name = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION")).to_string();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(stdout::SpanExporter::default())
        .build();
    let tracer = provider.tracer(app_name.clone());

    // Create a tracing layer with the configured tracer
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("debug,actix_web=debug"));

    let is_production = env::var("ENV").unwrap_or_default() == "production";

    // Hold the non-blocking writer guard at function scope so it lives for the
    // entire server lifetime. Dropping it early would lose buffered log writes.
    let _non_blocking_guard;

    if is_production {
        // Production: structured JSON output for log aggregators (no color)
        let (non_blocking_writer, guard) = tracing_appender::non_blocking(std::io::stdout());
        _non_blocking_guard = Some(guard);
        let bunyan_formatting_layer = BunyanFormattingLayer::new(app_name, non_blocking_writer);
        let subscriber = Registry::default()
            .with(env_filter)
            .with(telemetry)
            .with(JsonStorageLayer)
            .with(bunyan_formatting_layer)
            .with(ErrorLayer::default());
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to install `tracing` subscriber.");
    } else {
        _non_blocking_guard = None;
        // Development: colorized human-readable output with severity colors
        // ERROR = red, WARN = yellow, INFO = green, DEBUG = blue, TRACE = purple
        let subscriber = Registry::default()
            .with(env_filter)
            .with(telemetry)
            .with(
                fmt::layer()
                    .with_ansi(true)
                    .with_target(true)
                    .with_level(true)
                    .with_thread_ids(false)
                    .pretty(),
            )
            .with(ErrorLayer::default());
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to install `tracing` subscriber.");
    }

    // Load configuration
    let settings = Settings::new()?;
    let host = settings.server.host.clone();
    let port = settings.server.port;

    // Reject default secrets in production
    if is_production && settings.server.secret == "Very Secret" {
        panic!(
            "FATAL: Server secret must be changed from the default value in production. Set BREAKFAST_SERVER_SECRET environment variable."
        );
    }
    if !is_production && settings.server.secret == "Very Secret" {
        warn!("Using default server secret — acceptable for development only");
    }
    if is_production && settings.server.jwtsecret == "Very Secret" {
        panic!(
            "FATAL: JWT secret must be changed from the default value in production. Set BREAKFAST_SERVER_JWTSECRET environment variable."
        );
    }
    if !is_production && settings.server.jwtsecret == "Very Secret" {
        warn!("Using default JWT secret — acceptable for development only");
    }

    // Database pool
    let pool = settings
        .pg
        .create_pool(Some(Runtime::Tokio1), db_tls_connector(&settings))?;

    // Application state
    let state = Data::new(State {
        pool,
        secret: settings.server.secret.clone(),
        jwtsecret: settings.server.jwtsecret.clone(),
        s3_key_id: settings.server.s3_key_id.clone(),
        s3_key_secret: settings.server.s3_key_secret.clone(),
        cache: HashMap::new(),
        token_blacklist: HashMap::new(),
    });

    // Swagger UI config
    let swagger_config = Data::new(SwaggerConfig::from("/explorer/swagger.json"));

    // TLS
    let ssl_config = tls_config()?;

    // Verify frontend assets
    let frontend_path = Path::new(FRONTEND_DIR);
    if frontend_path.is_dir() {
        let index_path = frontend_path.join("index.html");
        if index_path.exists() {
            let file_count = std::fs::read_dir(frontend_path)
                .map(|entries| entries.count())
                .unwrap_or(0);
            info!(
                "Frontend assets loaded successfully from '{}' ({} files)",
                FRONTEND_DIR, file_count
            );
        } else {
            warn!(
                "Frontend directory '{}' exists but index.html is missing",
                FRONTEND_DIR
            );
        }
    } else {
        warn!(
            "Frontend directory '{}' not found — UI will be unavailable",
            FRONTEND_DIR
        );
    }

    // Start background task: periodic token blacklist cleanup
    spawn_token_cleanup_task(state.pool.clone());
    info!(
        "Token blacklist cleanup task started (interval: {:?})",
        TOKEN_CLEANUP_INTERVAL
    );

    info!("Starting server at https://{}:{}", host, port);

    let bind_address = format!("{}:{}", host, port);
    HttpServer::new(move || {
        // CORS: restrict to same-origin by default.
        // In production the frontend is served from the same origin so
        // `allowed_origin` matches the server's own address. For local
        // Trunk dev-server proxying, the proxy forwards requests to the
        // backend so no extra origin is needed.
        let cors = Cors::default()
            .allowed_origin(&format!("https://{}:{}", host, port))
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::ACCEPT,
            ])
            .max_age(3600);

        App::new()
            .wrap(TracingLogger::default())
            .wrap(cors)
            .app_data(state.clone())
            .app_data(swagger_config.clone())
            .configure(routes)
            .service(Files::new("/", FRONTEND_DIR).index_file("index.html"))
    })
    .bind_rustls_0_23(&bind_address, ssl_config)?
    .run()
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontend_dir_constant_is_correct() {
        assert_eq!(FRONTEND_DIR, "frontend/dist");
    }

    #[test]
    fn token_cleanup_interval_is_one_hour() {
        assert_eq!(TOKEN_CLEANUP_INTERVAL, Duration::from_secs(3600));
    }

    #[test]
    fn tls_config_loads_certs() {
        // Skip if cert files are not present (e.g. CI environments)
        if !std::path::Path::new("localhost.pem").exists()
            || !std::path::Path::new("localhost_key.pem").exists()
        {
            eprintln!("SKIP: TLS cert files not found — skipping tls_config test");
            return;
        }

        let config = tls_config().expect("tls_config() should succeed when cert files are present");

        // Verify ALPN protocols are configured for HTTP/1.1 and h2
        assert!(
            config.alpn_protocols.contains(&b"http/1.1".to_vec()),
            "ALPN should include http/1.1"
        );
        assert!(
            config.alpn_protocols.contains(&b"h2".to_vec()),
            "ALPN should include h2"
        );
        assert_eq!(
            config.alpn_protocols.len(),
            2,
            "should have exactly 2 ALPN protocols"
        );
    }

    #[test]
    fn db_tls_connector_with_webpki_roots() {
        // When db_ca_cert is None, the connector should use webpki root certs
        let settings = Settings {
            server: crate::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                secret: "secret".to_string(),
                jwtsecret: "jwtsecret".to_string(),
                s3_key_id: String::new(),
                s3_key_secret: String::new(),
                git_version: "test".to_string(),
            },
            database: crate::config::Database {
                url: "postgres://localhost/test".to_string(),
            },
            pg: deadpool_postgres::Config::new(),
            db_ca_cert: None,
        };

        // Should not panic — returns a valid MakeRustlsConnect
        let _connector = db_tls_connector(&settings);
    }

    #[test]
    fn db_tls_connector_with_custom_ca() {
        // Skip if the local CA cert is not present
        let ca_path = "localhost_ca.pem";
        if !std::path::Path::new(ca_path).exists() {
            eprintln!("SKIP: {} not found — skipping custom CA test", ca_path);
            return;
        }

        let settings = Settings {
            server: crate::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                secret: "secret".to_string(),
                jwtsecret: "jwtsecret".to_string(),
                s3_key_id: String::new(),
                s3_key_secret: String::new(),
                git_version: "test".to_string(),
            },
            database: crate::config::Database {
                url: "postgres://localhost/test".to_string(),
            },
            pg: deadpool_postgres::Config::new(),
            db_ca_cert: Some(ca_path.to_string()),
        };

        // Should not panic — loads the CA cert and returns a valid connector
        let _connector = db_tls_connector(&settings);
    }

    #[test]
    #[should_panic(expected = "Failed to open DB CA certificate")]
    fn db_tls_connector_panics_on_missing_ca_file() {
        let settings = Settings {
            server: crate::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                secret: "secret".to_string(),
                jwtsecret: "jwtsecret".to_string(),
                s3_key_id: String::new(),
                s3_key_secret: String::new(),
                git_version: "test".to_string(),
            },
            database: crate::config::Database {
                url: "postgres://localhost/test".to_string(),
            },
            pg: deadpool_postgres::Config::new(),
            db_ca_cert: Some("/nonexistent/path/ca.pem".to_string()),
        };

        // Should panic because the file does not exist
        let _connector = db_tls_connector(&settings);
    }
}
