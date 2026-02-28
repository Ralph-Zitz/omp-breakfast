use crate::{config::Settings, models::State, routes::routes};
use actix_web::{web::Data, App, HttpServer};
use deadpool_postgres::Runtime;
use flurry::HashMap;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider};
use opentelemetry_stdout as stdout;
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use rustls_pki_types::PrivateKeyDer;
use std::{env, fs::File, io::BufReader};
use tokio_postgres_rustls::MakeRustlsConnect;
use tracing::info;
use tracing_actix_web::TracingLogger;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};
use utoipa_swagger_ui::Config as SwaggerConfig;

fn tls_config() -> ServerConfig {
    let cert_file = &mut BufReader::new(File::open("localhost.pem").unwrap());
    let key_file = &mut BufReader::new(File::open("localhost_key.pem").unwrap());
    let cert_chain = certs(cert_file).map(|f| f.unwrap()).collect();
    let keys = pkcs8_private_keys(key_file).next().unwrap().unwrap();
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, PrivateKeyDer::Pkcs8(keys))
        .unwrap();
    config.alpn_protocols.push(b"http/1.1".to_vec());
    config.alpn_protocols.push(b"h2".to_vec());
    config
}

fn db_tls_connector(settings: &Settings) -> MakeRustlsConnect {
    let mut root_store = rustls::RootCertStore::empty();
    if let Some(ca_cert_path) = &settings.db_ca_cert {
        let ca_file =
            &mut BufReader::new(File::open(ca_cert_path).expect("CA cert file not found"));
        let ca_certs: Vec<_> = certs(ca_file)
            .map(|c| c.expect("invalid CA cert"))
            .collect();
        for cert in ca_certs {
            root_store.add(cert).expect("failed to add CA cert");
        }
    } else {
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
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

    if is_production {
        // Production: structured JSON output for log aggregators (no color)
        let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stdout());
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
    let ssl_config = tls_config();

    info!("Starting server at https://{}:{}", host, port);

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(state.clone())
            .app_data(swagger_config.clone())
            .configure(routes)
    })
    .bind_rustls_0_23(format!("{}:{}", host, port), ssl_config)?
    .run()
    .await?;

    Ok(())
}
