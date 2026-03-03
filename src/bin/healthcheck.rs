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

/// Check whether a raw HTTP response indicates a healthy service.
/// Returns `true` if the response starts with an HTTP 200 status line.
fn is_healthy_response(response: &str) -> bool {
    response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200")
}

/// Extract the first line of an HTTP response for error reporting.
/// Returns `"(empty)"` when the response is empty.
fn status_line(response: &str) -> &str {
    response.lines().next().unwrap_or("(empty)")
}

fn main() -> ExitCode {
    let host = "localhost";
    let port: u16 = std::env::var("HEALTH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

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
    if is_healthy_response(&response_str) {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "healthcheck: unhealthy response: {}",
            status_line(&response_str)
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

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::client::danger::ServerCertVerifier;
    use rustls_pki_types::{CertificateDer, ServerName, UnixTime};

    // ── is_healthy_response ─────────────────────────────────────────────

    #[test]
    fn healthy_response_http_1_1_200_ok() {
        assert!(is_healthy_response("HTTP/1.1 200 OK\r\n\r\n{\"up\":true}"));
    }

    #[test]
    fn healthy_response_http_1_0_200_ok() {
        assert!(is_healthy_response("HTTP/1.0 200 OK\r\n\r\n{\"up\":true}"));
    }

    #[test]
    fn healthy_response_200_with_extra_headers() {
        let resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"up\":true}";
        assert!(is_healthy_response(resp));
    }

    #[test]
    fn unhealthy_response_503() {
        assert!(!is_healthy_response(
            "HTTP/1.1 503 Service Unavailable\r\n\r\n{\"up\":false}"
        ));
    }

    #[test]
    fn unhealthy_response_500() {
        assert!(!is_healthy_response(
            "HTTP/1.1 500 Internal Server Error\r\n\r\n"
        ));
    }

    #[test]
    fn unhealthy_response_404() {
        assert!(!is_healthy_response("HTTP/1.1 404 Not Found\r\n\r\n"));
    }

    #[test]
    fn unhealthy_response_empty() {
        assert!(!is_healthy_response(""));
    }

    #[test]
    fn unhealthy_response_garbage() {
        assert!(!is_healthy_response("not an HTTP response at all"));
    }

    #[test]
    fn unhealthy_response_partial_status() {
        // Missing the "200" part
        assert!(!is_healthy_response("HTTP/1.1 "));
    }

    #[test]
    fn healthy_response_201_is_not_healthy() {
        // Only 200 is considered healthy, not other 2xx codes
        assert!(!is_healthy_response("HTTP/1.1 201 Created\r\n\r\n"));
    }

    // ── status_line ─────────────────────────────────────────────────────

    #[test]
    fn status_line_extracts_first_line() {
        let resp = "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain\r\n\r\nbody";
        assert_eq!(status_line(resp), "HTTP/1.1 503 Service Unavailable");
    }

    #[test]
    fn status_line_returns_empty_marker_for_empty_string() {
        assert_eq!(status_line(""), "(empty)");
    }

    #[test]
    fn status_line_single_line_response() {
        assert_eq!(status_line("HTTP/1.1 200 OK"), "HTTP/1.1 200 OK");
    }

    // ── NoVerifier ──────────────────────────────────────────────────────

    #[test]
    fn no_verifier_accepts_any_server_cert() {
        let verifier = NoVerifier;
        let cert = CertificateDer::from(vec![0u8; 32]);
        let server_name = ServerName::try_from("localhost").unwrap();
        let now = UnixTime::now();

        let result = verifier.verify_server_cert(&cert, &[], &server_name, &[], now);
        assert!(result.is_ok(), "NoVerifier should accept any certificate");
    }

    #[test]
    fn no_verifier_supported_schemes_includes_rsa() {
        let schemes = NoVerifier.supported_verify_schemes();
        assert!(
            schemes.contains(&rustls::SignatureScheme::RSA_PKCS1_SHA256),
            "should support RSA_PKCS1_SHA256"
        );
        assert!(
            schemes.contains(&rustls::SignatureScheme::RSA_PKCS1_SHA384),
            "should support RSA_PKCS1_SHA384"
        );
        assert!(
            schemes.contains(&rustls::SignatureScheme::RSA_PKCS1_SHA512),
            "should support RSA_PKCS1_SHA512"
        );
        assert!(
            schemes.contains(&rustls::SignatureScheme::RSA_PSS_SHA256),
            "should support RSA_PSS_SHA256"
        );
        assert!(
            schemes.contains(&rustls::SignatureScheme::RSA_PSS_SHA384),
            "should support RSA_PSS_SHA384"
        );
        assert!(
            schemes.contains(&rustls::SignatureScheme::RSA_PSS_SHA512),
            "should support RSA_PSS_SHA512"
        );
    }

    #[test]
    fn no_verifier_supported_schemes_includes_ecdsa() {
        let schemes = NoVerifier.supported_verify_schemes();
        assert!(
            schemes.contains(&rustls::SignatureScheme::ECDSA_NISTP256_SHA256),
            "should support ECDSA_NISTP256_SHA256"
        );
        assert!(
            schemes.contains(&rustls::SignatureScheme::ECDSA_NISTP384_SHA384),
            "should support ECDSA_NISTP384_SHA384"
        );
        assert!(
            schemes.contains(&rustls::SignatureScheme::ECDSA_NISTP521_SHA512),
            "should support ECDSA_NISTP521_SHA512"
        );
    }

    #[test]
    fn no_verifier_supported_schemes_includes_eddsa() {
        let schemes = NoVerifier.supported_verify_schemes();
        assert!(
            schemes.contains(&rustls::SignatureScheme::ED25519),
            "should support ED25519"
        );
        assert!(
            schemes.contains(&rustls::SignatureScheme::ED448),
            "should support ED448"
        );
    }

    #[test]
    fn no_verifier_supported_schemes_has_expected_count() {
        let schemes = NoVerifier.supported_verify_schemes();
        assert_eq!(
            schemes.len(),
            11,
            "should support exactly 11 signature schemes, got {}",
            schemes.len()
        );
    }

    #[test]
    fn no_verifier_is_debug() {
        // Verify the Debug derive works (required by rustls trait bounds)
        let debug_str = format!("{:?}", NoVerifier);
        assert_eq!(debug_str, "NoVerifier");
    }

    // ── TLS config construction ─────────────────────────────────────────

    #[test]
    fn tls_client_config_builds_successfully() {
        // Verify the TLS configuration used in main() can be constructed
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth();

        // with_no_client_auth means no client certs are configured
        assert!(
            !config.client_auth_cert_resolver.has_certs(),
            "should have no client auth certs"
        );
    }

    #[test]
    fn localhost_is_valid_server_name() {
        // Verify "localhost" can be converted to a valid ServerName
        let result: Result<ServerName<'_>, _> = "localhost".try_into();
        assert!(
            result.is_ok(),
            "\"localhost\" should be a valid DNS server name"
        );
    }

    #[test]
    fn tls_client_connection_creates_successfully() {
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth();

        let server_name: ServerName<'_> = "localhost".try_into().unwrap();
        let result = rustls::ClientConnection::new(Arc::new(config), server_name);
        assert!(
            result.is_ok(),
            "should create a ClientConnection for localhost"
        );
    }
}
