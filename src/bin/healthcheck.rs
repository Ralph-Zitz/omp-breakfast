//! Minimal healthcheck binary for use in Docker containers (including distroless).
//!
//! Connects to `https://localhost:8080/health` using rustls (accepts self-signed
//! certs via a custom verifier) and exits with code 0 on success, 1 on failure.
//!
//! This avoids the need for `curl` or `wget` in the container image.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;
use std::sync::Arc;

fn main() -> ExitCode {
    let host = "localhost";
    let port = 8080;

    // Build a TLS config that accepts any certificate (local/self-signed)
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerifier))
        .with_no_client_auth();

    let server_name = host.try_into().expect("valid DNS name");
    let mut conn = match rustls::ClientConnection::new(Arc::new(config), server_name) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("healthcheck: TLS setup failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut sock = match TcpStream::connect(format!("{host}:{port}")) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("healthcheck: connection failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut tls = rustls::Stream::new(&mut conn, &mut sock);

    let request = format!("GET /health HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");

    if let Err(e) = tls.write_all(request.as_bytes()) {
        eprintln!("healthcheck: write failed: {e}");
        return ExitCode::FAILURE;
    }

    let mut response = Vec::new();
    // Read response (ignore errors after we have data — server closes connection)
    let _ = tls.read_to_end(&mut response);

    let response_str = String::from_utf8_lossy(&response);

    // Check for HTTP 200 status line
    if response_str.starts_with("HTTP/1.1 200") || response_str.starts_with("HTTP/1.0 200") {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "healthcheck: unhealthy response: {}",
            response_str.lines().next().unwrap_or("(empty)")
        );
        ExitCode::FAILURE
    }
}

/// A certificate verifier that accepts any server certificate.
/// This is safe here because we are only connecting to localhost for a health check.
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls_pki_types::CertificateDer<'_>,
        _intermediates: &[rustls_pki_types::CertificateDer<'_>],
        _server_name: &rustls_pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls_pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}
