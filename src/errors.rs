use actix_web::{
    Error as AError, HttpRequest, HttpResponse,
    error::{JsonPayloadError, PathError, ResponseError},
};
use color_eyre::eyre::Error as EError;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};
use utoipa::ToSchema;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Eyre(#[from] EError),
    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error(transparent)]
    ActixAuth(#[from] AError),
    #[error(transparent)]
    ActixJson(#[from] JsonPayloadError),
    #[error(transparent)]
    ActixPath(#[from] PathError),
    #[error(transparent)]
    Pool(#[from] deadpool_postgres::PoolError),
    #[error(transparent)]
    Db(#[from] tokio_postgres::error::Error),
    #[error(transparent)]
    DbMapper(#[from] crate::from_row::FromRowError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error(transparent)]
    Conversion(#[from] serde_json::Error),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Argon2(String),
    #[error("{0}")]
    Utoipa(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Unauthorized(String),
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

impl std::fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self)
                .unwrap_or_else(|_| format!(r#"{{"error":"{}"}}"#, self.error))
        )
    }
}

pub fn json_error_handler(err: JsonPayloadError, _req: &HttpRequest) -> AError {
    Error::ActixJson(err).into()
}

pub fn path_error_handler(err: PathError, _req: &HttpRequest) -> AError {
    Error::ActixPath(err).into()
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::Jwt(e) => {
                error!(error = %e, "JWT error");
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Internal server error".to_string(),
                })
            }
            Error::Eyre(e) => {
                error!(error = %e, "Internal error");
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Internal server error".to_string(),
                })
            }
            Error::Io(e) => {
                error!(error = %e, "IO error");
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Internal server error".to_string(),
                })
            }
            // TODO: Fine grained query errors, or revert to general
            Error::Db(e) => {
                match e.code() {
                    // Unique constraint violation
                    Some(st) if st.code() == "23505" => {
                        warn!(error = %e, code = %st.code(), "DB unique constraint violation");
                        HttpResponse::Conflict().json(ErrorResponse {
                            error: "A record with that value already exists".to_string(),
                        })
                    }
                    // Foreign key constraint violation
                    Some(st) if st.code() == "23503" => {
                        warn!(error = %e, code = %st.code(), "DB foreign key constraint violation");
                        HttpResponse::Conflict().json(ErrorResponse {
                            error: "Operation conflicts with an existing relationship".to_string(),
                        })
                    }
                    Some(st) => {
                        error!(error = %e, code = %st.code(), "DB error");
                        HttpResponse::InternalServerError().json(ErrorResponse {
                            error: "Internal server error".to_string(),
                        })
                    }
                    None => {
                        error!(error = %e, "DB error");
                        HttpResponse::InternalServerError().json(ErrorResponse {
                            error: "Internal server error".to_string(),
                        })
                    }
                }
            }
            Error::DbMapper(e) => {
                // Keep these for a rainy day - i.e. for fine grained error handling
                match e {
                    crate::from_row::FromRowError::ColumnNotFound(col) => {
                        error!(error = %e, column = %col, "DB mapper column not found");
                        HttpResponse::InternalServerError().json(ErrorResponse {
                            error: "Internal server error".to_string(),
                        })
                    }
                    crate::from_row::FromRowError::Conversion(msg) => {
                        error!(error = %msg, "DB mapper conversion error");
                        HttpResponse::InternalServerError().json(ErrorResponse {
                            error: "Internal server error".to_string(),
                        })
                    }
                }
            }
            Error::ActixAuth(e) => {
                warn!(error = %e, "Authentication failure");
                HttpResponse::Unauthorized().json(ErrorResponse {
                    error: e.to_string(),
                })
            }
            Error::Config(e) => {
                error!(error = %e, "Configuration error");
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Internal server error".to_string(),
                })
            }
            Error::Pool(e) => {
                error!(error = %e, "Connection pool error");
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Internal server error".to_string(),
                })
            }
            Error::Conversion(e) => {
                error!(error = %e, "Serialization/conversion error");
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Internal server error".to_string(),
                })
            }
            Error::Validation(e) => {
                warn!(error = %e, "Validation error");
                HttpResponse::UnprocessableEntity().json(ErrorResponse {
                    error: e.to_string(),
                })
            }
            Error::Utoipa(e) => {
                warn!(error = %e, "OpenAPI schema error");
                HttpResponse::UnprocessableEntity().json(ErrorResponse {
                    error: e.to_string(),
                })
            }
            Error::Argon2(e) => {
                error!(error = %e, "Password hashing error");
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Internal server error".to_string(),
                })
            }
            Error::ActixJson(e) => match &e {
                JsonPayloadError::ContentType => {
                    warn!(error = %e, "Unsupported media type");
                    HttpResponse::UnsupportedMediaType().json(ErrorResponse {
                        error: e.to_string(),
                    })
                }
                JsonPayloadError::Deserialize(json_err) if json_err.is_data() => {
                    warn!(error = %json_err, "JSON deserialization error");
                    HttpResponse::UnprocessableEntity().json(ErrorResponse {
                        error: json_err.to_string(),
                    })
                }
                _ => {
                    warn!(error = %e, "Bad JSON request");
                    HttpResponse::BadRequest().json(ErrorResponse {
                        error: e.to_string(),
                    })
                }
            },
            Error::ActixPath(e) => {
                warn!(error = %e, "Bad path parameter");
                HttpResponse::BadRequest().json(ErrorResponse {
                    error: e.to_string(),
                })
            }
            Error::NotFound(e) => {
                warn!(error = %e, "Resource not found");
                HttpResponse::NotFound().json(ErrorResponse {
                    error: e.to_string(),
                })
            }
            Error::Forbidden(e) => {
                warn!(error = %e, "Forbidden");
                HttpResponse::Forbidden().json(ErrorResponse {
                    error: e.to_string(),
                })
            }
            Error::Unauthorized(e) => {
                warn!(error = %e, "Unauthorized");
                HttpResponse::Unauthorized().json(ErrorResponse {
                    error: e.to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;
    use serde::de::Error as DeError;

    #[test]
    fn validation_error_returns_422() {
        let err = Error::Validation("bad input".into());
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn not_found_error_returns_404() {
        let err = Error::NotFound("missing".into());
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn argon2_error_returns_500() {
        let err = Error::Argon2("hash failure".into());
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn utoipa_error_returns_422() {
        let err = Error::Utoipa("schema error".into());
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn io_error_returns_500() {
        let err = Error::Io(std::io::Error::other("disk"));
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn config_error_returns_500() {
        let err = Error::Config(config::ConfigError::NotFound("key".into()));
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn conversion_error_returns_500() {
        let json_err = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let err = Error::Conversion(json_err);
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn actix_path_error_returns_400() {
        // PathError wraps a Display-able error
        let inner = serde::de::value::Error::custom("bad path");
        let path_err = PathError::Deserialize(inner);
        let err = Error::ActixPath(path_err);
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn jwt_error_returns_500() {
        // Create a JWT error by trying to decode an invalid token
        let jwt_err = jsonwebtoken::decode::<serde_json::Value>(
            "invalid",
            &jsonwebtoken::DecodingKey::from_secret(b"secret"),
            &jsonwebtoken::Validation::default(),
        )
        .unwrap_err();
        let err = Error::Jwt(jwt_err);
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn pool_error_returns_500() {
        // PoolError can be constructed via a timeout
        let pool_err =
            deadpool_postgres::PoolError::Backend(tokio_postgres::Error::__private_api_timeout());
        let err = Error::Pool(pool_err);
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn db_mapper_column_not_found_returns_500() {
        let err = Error::DbMapper(crate::from_row::FromRowError::ColumnNotFound("test".into()));
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn eyre_error_returns_500() {
        let eyre_err = color_eyre::eyre::eyre!("something went wrong");
        let err = Error::Eyre(eyre_err);
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn forbidden_error_returns_403() {
        let err = Error::Forbidden("not allowed".into());
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn unauthorized_error_returns_401() {
        let err = Error::Unauthorized("bad credentials".into());
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn error_responses_do_not_leak_internal_details() {
        // 5xx errors should return generic "Internal server error" to clients
        let err = Error::Argon2("secret hash details".into());
        let resp = err.error_response();
        // We can't easily read the body in a sync test, but we verify the status
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // ── Error response body shape (#181) ─────────────────────────────────────

    /// Verify that error responses have the JSON shape `{"error": "..."}`.
    #[actix_web::test]
    async fn error_response_body_is_json_object_with_error_key() {
        use actix_web::body::to_bytes;

        // 4xx — message is passed through
        let err = Error::NotFound("resource missing".into());
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.is_object(), "body must be a JSON object");
        assert_eq!(json["error"], "resource missing");

        // 5xx — message is sanitized to "Internal server error"
        let err = Error::Argon2("secret details".into());
        let resp = err.error_response();
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Internal server error");
    }
}
