use crate::{config::Settings, db, models::State, routes::routes};
use actix_cors::Cors;
use actix_files::Files;
use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer, middleware::DefaultHeaders, web::Data,
};
use chrono::Utc;
use dashmap::DashMap;
use deadpool_postgres::Runtime;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider};
use opentelemetry_stdout as stdout;
use rustls::ServerConfig;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, pem::PemObject};
use std::{env, path::Path, time::Duration};
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
/// `token_blacklist` table and the in-memory DashMap so neither grows
/// unbounded.
fn spawn_token_cleanup_task(state: Data<State>) {
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(TOKEN_CLEANUP_INTERVAL).await;

            // Evict expired entries from the in-memory blacklist
            let before = state.token_blacklist.len();
            let now = Utc::now();
            state
                .token_blacklist
                .retain(|_, expires_at| *expires_at > now);
            let memory_evicted = before - state.token_blacklist.len();

            // Clean up the persistent DB blacklist
            match state.pool.get().await {
                Ok(client) => match db::cleanup_expired_tokens(&client).await {
                    Ok(db_deleted) => {
                        if db_deleted > 0 || memory_evicted > 0 {
                            info!(
                                db_deleted = db_deleted,
                                memory_evicted = memory_evicted,
                                memory_remaining = state.token_blacklist.len(),
                                "Cleaned up expired entries from token blacklist"
                            );
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to clean up expired tokens from DB");
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

/// Handler that redirects any HTTP request to the equivalent HTTPS URL.
/// Returns `301 Moved Permanently` with a `Location` header pointing to the
/// HTTPS equivalent, preserving the path and query string.
///
/// The hostname used in the redirect is extracted from the `Host` header and
/// validated to prevent open redirect attacks — only alphanumeric characters,
/// hyphens, dots, and colons are allowed.
async fn redirect_to_https(req: HttpRequest, https_port: Data<u16>) -> HttpResponse {
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");

    // Strip any port from the Host header to replace with the HTTPS port
    let hostname = host.split(':').next().unwrap_or(host);

    // Validate hostname to prevent open redirect attacks:
    // Only allow alphanumeric, hyphens, and dots (valid DNS characters).
    if !hostname
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
        return HttpResponse::BadRequest().finish();
    }

    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    let https_port = **https_port;
    let location = if https_port == 443 {
        format!("https://{}{}", hostname, path)
    } else {
        format!("https://{}:{}{}", hostname, https_port, path)
    };

    HttpResponse::MovedPermanently()
        .insert_header(("Location", location))
        .finish()
}

/// Spawn a background HTTP server that redirects all requests to HTTPS.
/// Binds on `host:{http_port}`. If binding fails
/// (e.g., insufficient privileges for port 80), logs a warning and returns
/// without blocking the main HTTPS server.
fn spawn_http_redirect_server(host: String, https_port: u16, http_port: u16) {
    let bind_address = format!("{}:{}", host, http_port);
    actix_web::rt::spawn(async move {
        let https_port_data = Data::new(https_port);
        let server = HttpServer::new(move || {
            App::new()
                .app_data(https_port_data.clone())
                .default_service(actix_web::web::to(redirect_to_https))
        })
        .bind(&bind_address);

        match server {
            Ok(server) => {
                info!(
                    "HTTP→HTTPS redirect server listening on http://{}",
                    bind_address
                );
                if let Err(e) = server.run().await {
                    error!(error = %e, "HTTP redirect server failed");
                }
            }
            Err(e) => {
                warn!(
                    error = %e,
                    address = %bind_address,
                    "Could not bind HTTP redirect server — HTTPS redirect is unavailable. \
                     This is expected in development or when port {} requires elevated privileges.",
                    http_port
                );
            }
        }
    });
}

fn tls_config() -> Result<ServerConfig, crate::errors::Error> {
    let cert_path = "localhost.pem";
    let key_path = "localhost_key.pem";

    let cert_chain: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(cert_path)
        .map_err(|e| {
            error!("Failed to load TLS certificate from '{}': {}", cert_path, e);
            crate::errors::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            error!(
                "Failed to parse TLS certificate from '{}': {}",
                cert_path, e
            );
            crate::errors::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;
    info!("TLS certificate loaded successfully from '{}'", cert_path);

    let key = PrivatePkcs8KeyDer::from_pem_file(key_path).map_err(|e| {
        error!("Failed to load TLS private key from '{}': {}", key_path, e);
        crate::errors::Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            e.to_string(),
        ))
    })?;
    info!("TLS private key loaded successfully from '{}'", key_path);

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, PrivateKeyDer::Pkcs8(key))
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
        let ca_certs: Vec<CertificateDer<'static>> =
            CertificateDer::pem_file_iter(ca_cert_path.as_str())
                .unwrap_or_else(|e| {
                    panic!("Failed to open DB CA certificate '{}': {}", ca_cert_path, e);
                })
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
    if is_production && settings.server.jwtsecret.len() < 32 {
        panic!(
            "FATAL: JWT secret must be at least 32 characters in production. Current length: {}",
            settings.server.jwtsecret.len()
        );
    }
    if !is_production && settings.server.jwtsecret == "Very Secret" {
        warn!("Using default JWT secret — acceptable for development only");
    }

    // Reject default database credentials in production
    let pg_user = settings.pg.user.as_deref().unwrap_or("actix");
    let pg_password = settings.pg.password.as_deref().unwrap_or("actix");
    if is_production && pg_user == "actix" {
        panic!(
            "FATAL: Database user must be changed from the default value in production. Set BREAKFAST_PG_USER environment variable."
        );
    }
    if is_production && pg_password == "actix" {
        panic!(
            "FATAL: Database password must be changed from the default value in production. Set BREAKFAST_PG_PASSWORD environment variable."
        );
    }
    if !is_production && pg_user == "actix" && pg_password == "actix" {
        warn!("Using default database credentials — acceptable for development only");
    }

    // Reject placeholder database hostname in production
    let pg_host = settings.pg.host.as_deref().unwrap_or("localhost");
    if is_production && pg_host == "pick.a.proper.hostname" {
        panic!(
            "FATAL: Database host must be changed from the placeholder value in production. Set BREAKFAST_PG_HOST environment variable."
        );
    }

    // Database pool
    let pool = settings
        .pg
        .create_pool(Some(Runtime::Tokio1), db_tls_connector(&settings))?;

    // Run database migrations before accepting requests
    {
        let mut client = pool.get().await?;
        let report = db::migrate::run_migrations(&mut client)
            .await
            .map_err(|e| {
                error!(error = %e, "Database migration failed — refusing to start");
                std::io::Error::other(e.to_string())
            })?;
        let applied = report.applied_migrations().len();
        if applied > 0 {
            info!(applied = applied, "Applied database migrations");
        } else {
            info!("Database schema is up to date (no pending migrations)");
        }
    }

    // Application state
    let state = Data::new(State {
        pool,
        jwtsecret: settings.server.jwtsecret.clone(),
        cache: DashMap::new(),
        token_blacklist: DashMap::new(),
        login_attempts: DashMap::new(),
    });

    // Swagger UI config (only relevant in non-production environments)
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
    spawn_token_cleanup_task(state.clone());
    info!(
        "Token blacklist cleanup task started (interval: {:?})",
        TOKEN_CLEANUP_INTERVAL
    );

    // Start HTTP→HTTPS redirect server as a background task
    spawn_http_redirect_server(host.clone(), port, settings.server.http_redirect_port);

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
            .wrap(
                DefaultHeaders::new()
                    .add(("Strict-Transport-Security", "max-age=31536000; includeSubDomains"))
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add(("Permissions-Policy", "camera=(), microphone=(), geolocation=(), payment=()")),
            )
            .app_data(state.clone())
            .app_data(swagger_config.clone())
            .configure(routes)
            // Note: /explorer is conditionally registered by routes() based on ENV
            .service(
                actix_web::web::scope("")
                    .wrap(
                        DefaultHeaders::new()
                            .add((
                                "Content-Security-Policy",
                                "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' https://assets.lego.com; connect-src 'self'; frame-ancestors 'none'; form-action 'self'; base-uri 'self'",
                            )),
                    )
                    .service(Files::new("/", FRONTEND_DIR).index_file("index.html")),
            )
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
    fn default_http_redirect_port_is_80() {
        assert_eq!(crate::config::default_http_redirect_port(), 80);
    }

    #[actix_web::test]
    async fn redirect_handler_returns_301_with_location() {
        let app = actix_web::test::init_service(
            App::new()
                .app_data(Data::new(8080u16))
                .default_service(actix_web::web::to(redirect_to_https)),
        )
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/some/path?q=1")
            .insert_header(("Host", "example.com:80"))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        assert_eq!(resp.status(), 301);
        let location = resp
            .headers()
            .get("Location")
            .expect("should have Location header")
            .to_str()
            .unwrap();
        assert_eq!(location, "https://example.com:8080/some/path?q=1");
    }

    #[actix_web::test]
    async fn redirect_handler_omits_port_for_443() {
        let app = actix_web::test::init_service(
            App::new()
                .app_data(Data::new(443u16))
                .default_service(actix_web::web::to(redirect_to_https)),
        )
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/health")
            .insert_header(("Host", "example.com"))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        assert_eq!(resp.status(), 301);
        let location = resp.headers().get("Location").unwrap().to_str().unwrap();
        assert_eq!(location, "https://example.com/health");
    }

    #[actix_web::test]
    async fn redirect_handler_preserves_root_path() {
        let app = actix_web::test::init_service(
            App::new()
                .app_data(Data::new(8080u16))
                .default_service(actix_web::web::to(redirect_to_https)),
        )
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/")
            .insert_header(("Host", "localhost:80"))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        assert_eq!(resp.status(), 301);
        let location = resp.headers().get("Location").unwrap().to_str().unwrap();
        assert_eq!(location, "https://localhost:8080/");
    }

    #[actix_web::test]
    async fn redirect_handler_uses_localhost_when_no_host_header() {
        let app = actix_web::test::init_service(
            App::new()
                .app_data(Data::new(8080u16))
                .default_service(actix_web::web::to(redirect_to_https)),
        )
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1.0/users")
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        assert_eq!(resp.status(), 301);
        let location = resp.headers().get("Location").unwrap().to_str().unwrap();
        assert_eq!(location, "https://localhost:8080/api/v1.0/users");
    }

    // ── CORS configuration tests ────────────────────────────────────────

    /// Build a test app with the same CORS config used in production.
    /// Uses a fixed host/port so assertions are predictable.
    fn cors_test_app() -> App<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        let host = "localhost";
        let port = 8080u16;
        let cors = Cors::default()
            .allowed_origin(&format!("https://{}:{}", host, port))
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::ACCEPT,
            ])
            .max_age(3600);

        App::new().wrap(cors).route(
            "/test",
            actix_web::web::get().to(|| async { HttpResponse::Ok().finish() }),
        )
    }

    #[actix_web::test]
    async fn cors_allows_same_origin() {
        let app = actix_web::test::init_service(cors_test_app()).await;

        let req = actix_web::test::TestRequest::get()
            .uri("/test")
            .insert_header(("Origin", "https://localhost:8080"))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200, "same-origin request should succeed");
        let acao = resp
            .headers()
            .get("access-control-allow-origin")
            .expect("should have ACAO header for allowed origin");
        assert_eq!(acao.to_str().unwrap(), "https://localhost:8080");
    }

    #[actix_web::test]
    async fn cors_rejects_disallowed_origin() {
        let app = actix_web::test::init_service(cors_test_app()).await;

        // Preflight from a disallowed origin
        let req = actix_web::test::TestRequest::default()
            .method(actix_web::http::Method::OPTIONS)
            .uri("/test")
            .insert_header(("Origin", "https://evil.example.com"))
            .insert_header(("Access-Control-Request-Method", "GET"))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        // actix-cors returns 200 for preflight but without ACAO header for
        // disallowed origins, effectively blocking the browser from reading
        // the response.
        let acao = resp.headers().get("access-control-allow-origin");
        assert!(
            acao.is_none(),
            "disallowed origin should NOT receive ACAO header, got: {:?}",
            acao
        );
    }

    #[actix_web::test]
    async fn cors_allows_configured_methods() {
        let app = actix_web::test::init_service(cors_test_app()).await;

        // Preflight for an allowed method (DELETE)
        let req = actix_web::test::TestRequest::default()
            .method(actix_web::http::Method::OPTIONS)
            .uri("/test")
            .insert_header(("Origin", "https://localhost:8080"))
            .insert_header(("Access-Control-Request-Method", "DELETE"))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200, "preflight for DELETE should succeed");
        let methods = resp
            .headers()
            .get("access-control-allow-methods")
            .expect("should have allow-methods header")
            .to_str()
            .unwrap()
            .to_uppercase();
        assert!(
            methods.contains("DELETE"),
            "allowed methods should include DELETE, got: {}",
            methods
        );
    }

    #[actix_web::test]
    async fn cors_rejects_disallowed_method() {
        let app = actix_web::test::init_service(cors_test_app()).await;

        // Preflight for PATCH which is not in the allowed methods list
        let req = actix_web::test::TestRequest::default()
            .method(actix_web::http::Method::OPTIONS)
            .uri("/test")
            .insert_header(("Origin", "https://localhost:8080"))
            .insert_header(("Access-Control-Request-Method", "PATCH"))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        // actix-cors returns 200 for preflight but the allow-methods header
        // should NOT include PATCH
        let methods = resp
            .headers()
            .get("access-control-allow-methods")
            .map(|v| v.to_str().unwrap_or("").to_uppercase());
        if let Some(ref m) = methods {
            assert!(
                !m.contains("PATCH"),
                "PATCH should NOT be in allowed methods, got: {}",
                m
            );
        }
    }

    #[actix_web::test]
    async fn cors_allows_configured_headers() {
        let app = actix_web::test::init_service(cors_test_app()).await;

        // Preflight requesting the Authorization header
        let req = actix_web::test::TestRequest::default()
            .method(actix_web::http::Method::OPTIONS)
            .uri("/test")
            .insert_header(("Origin", "https://localhost:8080"))
            .insert_header(("Access-Control-Request-Method", "GET"))
            .insert_header(("Access-Control-Request-Headers", "authorization"))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);
        let headers = resp
            .headers()
            .get("access-control-allow-headers")
            .expect("should have allow-headers")
            .to_str()
            .unwrap()
            .to_lowercase();
        assert!(
            headers.contains("authorization"),
            "allowed headers should include authorization, got: {}",
            headers
        );
    }

    #[actix_web::test]
    async fn cors_max_age_is_3600() {
        let app = actix_web::test::init_service(cors_test_app()).await;

        let req = actix_web::test::TestRequest::default()
            .method(actix_web::http::Method::OPTIONS)
            .uri("/test")
            .insert_header(("Origin", "https://localhost:8080"))
            .insert_header(("Access-Control-Request-Method", "GET"))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        let max_age = resp
            .headers()
            .get("access-control-max-age")
            .expect("should have max-age header")
            .to_str()
            .unwrap();
        assert_eq!(max_age, "3600", "max-age should be 3600 seconds");
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
                http_redirect_port: 80,
                secret: "secret".to_string(),
                jwtsecret: "jwtsecret".to_string(),
                git_version: "test".to_string(),
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
                http_redirect_port: 80,
                secret: "secret".to_string(),
                jwtsecret: "jwtsecret".to_string(),
                git_version: "test".to_string(),
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
                http_redirect_port: 80,
                secret: "secret".to_string(),
                jwtsecret: "jwtsecret".to_string(),
                git_version: "test".to_string(),
            },
            pg: deadpool_postgres::Config::new(),
            db_ca_cert: Some("/nonexistent/path/ca.pem".to_string()),
        };

        // Should panic because the file does not exist
        let _connector = db_tls_connector(&settings);
    }
}
