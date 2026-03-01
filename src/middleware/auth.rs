use crate::{
    db, db::get_user, db::get_user_by_email, errors::Error, handlers::get_client, models::Auth,
    models::CachedUser, models::Claims, models::State,
};
use actix_web::HttpMessage;
use actix_web::{dev::ServiceRequest, web::Data};
use actix_web_httpauth::extractors::{basic::BasicAuth, bearer::BearerAuth};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordVerifier},
};
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::Client;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde_json::json;
use tracing::{error, instrument, warn};
use uuid::Uuid;

// Access token lifetime: 15 minutes
const ACCESS_TOKEN_DURATION_MINUTES: i64 = 15;
// Refresh token lifetime: 7 days
const REFRESH_TOKEN_DURATION_DAYS: i64 = 7;
// Auth cache TTL: 5 minutes
const CACHE_TTL_SECONDS: i64 = 300;
// Auth cache maximum entries
const CACHE_MAX_SIZE: usize = 1000;

#[instrument(skip(jwt_secret), level = "debug")]
fn generate_token(
    user_id: Uuid,
    jwt_secret: &str,
    token_type: &str,
    duration: Duration,
) -> Result<String, jsonwebtoken::errors::Error> {
    let headers = Header::default();
    let encoding_key = EncodingKey::from_secret(jwt_secret.as_ref());
    let now = Utc::now();
    let exp = now + duration;
    let claims = Claims {
        sub: user_id,
        exp: exp.timestamp(),
        iat: now.timestamp(),
        jti: Uuid::now_v7(),
        token_type: token_type.to_string(),
    };
    encode(&headers, &claims, &encoding_key)
}

#[instrument(skip(jwt_secret), level = "debug")]
pub async fn generate_token_pair(user_id: Uuid, jwt_secret: String) -> Result<Auth, Error> {
    let access_token = generate_token(
        user_id,
        &jwt_secret,
        "access",
        Duration::try_minutes(ACCESS_TOKEN_DURATION_MINUTES).expect("valid duration"),
    )
    .map_err(Error::Jwt)?;
    let refresh_token = generate_token(
        user_id,
        &jwt_secret,
        "refresh",
        Duration::try_days(REFRESH_TOKEN_DURATION_DAYS).expect("valid duration"),
    )
    .map_err(Error::Jwt)?;
    Ok(Auth {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: ACCESS_TOKEN_DURATION_MINUTES * 60,
    })
}

#[instrument(skip(jwt_secret), level = "debug")]
pub async fn verify_jwt(token: String, jwt_secret: String) -> Result<TokenData<Claims>, Error> {
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_ref());
    let validation = Validation::default();
    let result = decode::<Claims>(&token, &decoding_key, &validation)?;
    Ok(result)
}

/// Revoke a token by persisting it to the DB blacklist and caching in memory.
/// `expires_at` is the token's original expiry so cleanup can prune stale rows.
pub async fn revoke_token(
    client: &Client,
    state: &Data<State>,
    jti: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), Error> {
    let jti_uuid =
        Uuid::parse_str(jti).map_err(|e| Error::Validation(format!("Invalid jti: {}", e)))?;
    // Persist to DB (source of truth)
    db::revoke_token_db(client, jti_uuid, expires_at).await?;
    // Update in-memory cache for fast-path lookups
    state.token_blacklist.pin().insert(jti.to_string(), true);
    Ok(())
}

/// Check whether a token has been revoked.
/// Uses the in-memory cache first, then falls back to the DB.
pub async fn is_token_revoked(
    client: &Client,
    state: &Data<State>,
    jti: &str,
) -> Result<bool, Error> {
    // Fast path: check in-memory cache
    if state.token_blacklist.pin().contains_key(jti) {
        return Ok(true);
    }
    // Slow path: check DB
    let jti_uuid = match Uuid::parse_str(jti) {
        Ok(u) => u,
        Err(_) => return Ok(false),
    };
    let revoked = db::is_token_revoked_db(client, jti_uuid).await?;
    if revoked {
        // Populate in-memory cache for future fast-path hits
        state.token_blacklist.pin().insert(jti.to_string(), true);
    }
    Ok(revoked)
}

#[instrument(skip(state), level = "debug")]
pub fn invalidate_cache(state: Data<State>, key: &str) -> bool {
    let cache = &state.cache;
    cache.pin().remove(key).is_some()
}

#[instrument(skip(credentials, req), level = "debug")]
pub async fn jwt_validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (actix_web::Error, ServiceRequest)> {
    let state = match req.app_data::<Data<State>>() {
        Some(s) => s,
        None => {
            error!("Application state not configured");
            return Err((
                actix_web::error::ErrorInternalServerError(
                    json!({"error":"Internal server error"}),
                ),
                req,
            ));
        }
    };
    let client = match get_client(state.pool.clone()).await {
        Ok(c) => c,
        Err(_) => {
            return Err((
                actix_web::error::ErrorInternalServerError(
                    json!({"error":"Database connection error"}),
                ),
                req,
            ));
        }
    };
    let jwt_secret = &state.jwtsecret;
    let claims = verify_jwt(credentials.token().to_string(), jwt_secret.clone()).await;
    if let Ok(c) = claims {
        // Only accept access tokens for API endpoints
        if c.claims.token_type != "access" {
            warn!(token_type = %c.claims.token_type, "Invalid token type, access token required");
            return Err((
                actix_web::error::ErrorUnauthorized(
                    json!({"error":"Invalid token type, access token required"}),
                ),
                req,
            ));
        }
        // Check if token has been revoked
        match is_token_revoked(&client, state, &c.claims.jti.to_string()).await {
            Ok(true) => {
                warn!(jti = %c.claims.jti, "Rejected revoked token");
                return Err((
                    actix_web::error::ErrorUnauthorized(json!({"error":"Token has been revoked"})),
                    req,
                ));
            }
            Err(_) => {
                return Err((
                    actix_web::error::ErrorInternalServerError(
                        json!({"error":"Failed to check token revocation"}),
                    ),
                    req,
                ));
            }
            Ok(false) => {}
        }
        match get_user(&client, c.claims.sub).await {
            Ok(_) => {
                req.extensions_mut().insert(c.claims);
                Ok(req)
            }
            Err(e) => Err((actix_web::error::ErrorUnauthorized(e), req)),
        }
    } else {
        warn!("Unauthorized access - invalid or expired JWT");
        Err((
            actix_web::error::ErrorUnauthorized(json!({"error":"Unauthorized access"})),
            req,
        ))
    }
}

#[instrument(skip(credentials, req), level = "debug")]
pub async fn refresh_validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (actix_web::Error, ServiceRequest)> {
    let state = match req.app_data::<Data<State>>() {
        Some(s) => s,
        None => {
            error!("Application state not configured");
            return Err((
                actix_web::error::ErrorInternalServerError(
                    json!({"error":"Internal server error"}),
                ),
                req,
            ));
        }
    };
    let jwt_secret = &state.jwtsecret;
    let claims = verify_jwt(credentials.token().to_string(), jwt_secret.clone()).await;
    if let Ok(c) = claims {
        // Only accept refresh tokens
        if c.claims.token_type != "refresh" {
            warn!(token_type = %c.claims.token_type, "Invalid token type, refresh token required");
            return Err((
                actix_web::error::ErrorUnauthorized(
                    json!({"error":"Invalid token type, refresh token required"}),
                ),
                req,
            ));
        }
        // Check if token has been revoked
        let client = match get_client(state.pool.clone()).await {
            Ok(c) => c,
            Err(_) => {
                return Err((
                    actix_web::error::ErrorInternalServerError(
                        json!({"error":"Database connection error"}),
                    ),
                    req,
                ));
            }
        };
        match is_token_revoked(&client, state, &c.claims.jti.to_string()).await {
            Ok(true) => {
                warn!(jti = %c.claims.jti, "Rejected revoked refresh token");
                return Err((
                    actix_web::error::ErrorUnauthorized(json!({"error":"Token has been revoked"})),
                    req,
                ));
            }
            Err(_) => {
                return Err((
                    actix_web::error::ErrorInternalServerError(
                        json!({"error":"Failed to check token revocation"}),
                    ),
                    req,
                ));
            }
            Ok(false) => {}
        }
        Ok(req)
    } else {
        warn!("Invalid or expired refresh token");
        Err((
            actix_web::error::ErrorUnauthorized(
                json!({"error":"Invalid or expired refresh token"}),
            ),
            req,
        ))
    }
}

#[instrument(skip(credentials, req), level = "debug")]
pub async fn basic_validator(
    req: ServiceRequest,
    credentials: BasicAuth,
) -> Result<ServiceRequest, (actix_web::Error, ServiceRequest)> {
    let state = match req.app_data::<Data<State>>() {
        Some(s) => s,
        None => {
            error!("Application state not configured");
            return Err((
                actix_web::error::ErrorInternalServerError(
                    json!({"error":"Internal server error"}),
                ),
                req,
            ));
        }
    };
    let cache = &state.cache;
    let client = match get_client(state.pool.clone()).await {
        Ok(c) => c,
        Err(_) => {
            return Err((
                actix_web::error::ErrorInternalServerError(
                    json!({"error":"Database connection error"}),
                ),
                req,
            ));
        }
    };
    let user = {
        let guard = cache.pin();
        guard
            .get(&credentials.user_id().to_string())
            .filter(|cached| {
                // TTL check: reject entries older than CACHE_TTL_SECONDS
                (Utc::now() - cached.cached_at).num_seconds() < CACHE_TTL_SECONDS
            })
            .map(|cached| cached.user.clone())
    };
    // Evict expired entry if TTL expired
    if user.is_none() {
        let guard = cache.pin();
        if let Some(cached) = guard.get(&credentials.user_id().to_string())
            && (Utc::now() - cached.cached_at).num_seconds() >= CACHE_TTL_SECONDS
        {
            guard.remove(&credentials.user_id().to_string());
        }
    }
    let user = match user {
        Some(u) => u,
        None => {
            // Enforce max cache size before inserting
            if cache.pin().len() >= CACHE_MAX_SIZE {
                // Evict oldest entries
                let guard = cache.pin();
                let mut entries: Vec<(String, i64)> = guard
                    .iter()
                    .map(|(k, v)| (k.clone(), v.cached_at.timestamp()))
                    .collect();
                entries.sort_by_key(|(_, ts)| *ts);
                // Remove oldest 10% to avoid evicting on every request
                let to_remove = (CACHE_MAX_SIZE / 10).max(1);
                for (key, _) in entries.into_iter().take(to_remove) {
                    guard.remove(&key);
                }
            }
            match get_user_by_email(&client, credentials.user_id()).await {
                Ok(u) => {
                    cache.pin().insert(
                        credentials.user_id().to_string(),
                        CachedUser {
                            user: u.clone(),
                            cached_at: Utc::now(),
                        },
                    );
                    u
                }
                Err(_) => {
                    warn!(user = %credentials.user_id(), "Unknown user attempted authentication");
                    return Err((
                        actix_web::error::ErrorUnauthorized(json!({"error":"Unauthorized access"})),
                        req,
                    ));
                }
            }
        }
    };
    if let Some(pswd) = credentials.password() {
        let parsed_hash = match PasswordHash::new(&user.password) {
            Ok(h) => h,
            Err(_) => {
                warn!(user = %credentials.user_id(), "Malformed password hash in database");
                return Err((
                    actix_web::error::ErrorInternalServerError(
                        json!({"error":"Internal server error"}),
                    ),
                    req,
                ));
            }
        };
        match Argon2::default().verify_password(pswd.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(req),
            Err(_) => {
                warn!(user = %credentials.user_id(), "Invalid password for user");
                Err((
                    actix_web::error::ErrorUnauthorized(json!({"error":"Unauthorized access"})),
                    req,
                ))
            }
        }
    } else {
        warn!(user = %credentials.user_id(), "Missing password in basic auth");
        Err((
            actix_web::error::ErrorUnauthorized(json!({"error":"Unauthorized access"})),
            req,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::web::Data;
    use jsonwebtoken::{DecodingKey, Validation, decode};

    const TEST_SECRET: &str = "test-jwt-secret";

    fn test_state() -> Data<State> {
        let mut pg_cfg = deadpool_postgres::Config::new();
        pg_cfg.dbname = Some("dummy".to_string());
        let pool = pg_cfg
            .create_pool(None, tokio_postgres::NoTls)
            .expect("dummy pool");
        Data::new(State {
            pool,
            secret: "secret".to_string(),
            jwtsecret: TEST_SECRET.to_string(),
            s3_key_id: String::new(),
            s3_key_secret: String::new(),
            cache: flurry::HashMap::new(),
            token_blacklist: flurry::HashMap::new(),
        })
    }

    // Helper: directly insert into the in-memory blacklist for unit tests
    // (DB-backed revocation is tested via integration tests)
    fn revoke_token_in_memory(state: &Data<State>, jti: &str) {
        state.token_blacklist.pin().insert(jti.to_string(), true);
    }

    fn is_token_in_memory_blacklist(state: &Data<State>, jti: &str) -> bool {
        state.token_blacklist.pin().contains_key(jti)
    }

    // -- Token generation & verification --

    #[actix_web::test]
    async fn generate_token_pair_returns_two_distinct_tokens() {
        let user_id = Uuid::now_v7();
        let auth = generate_token_pair(user_id, TEST_SECRET.to_string())
            .await
            .unwrap();

        assert!(!auth.access_token.is_empty());
        assert!(!auth.refresh_token.is_empty());
        assert_ne!(auth.access_token, auth.refresh_token);
        assert_eq!(auth.token_type, "Bearer");
        assert_eq!(auth.expires_in, ACCESS_TOKEN_DURATION_MINUTES * 60);
    }

    #[actix_web::test]
    async fn access_token_has_correct_claims() {
        let user_id = Uuid::now_v7();
        let auth = generate_token_pair(user_id, TEST_SECRET.to_string())
            .await
            .unwrap();

        let token_data = decode::<Claims>(
            &auth.access_token,
            &DecodingKey::from_secret(TEST_SECRET.as_ref()),
            &Validation::default(),
        )
        .unwrap();

        assert_eq!(token_data.claims.sub, user_id);
        assert_eq!(token_data.claims.token_type, "access");
        let expected_exp = Utc::now().timestamp() + ACCESS_TOKEN_DURATION_MINUTES * 60;
        assert!(
            (token_data.claims.exp - expected_exp).abs() < 60,
            "access token exp should be ~15 min from now"
        );
    }

    #[actix_web::test]
    async fn refresh_token_has_correct_claims() {
        let user_id = Uuid::now_v7();
        let auth = generate_token_pair(user_id, TEST_SECRET.to_string())
            .await
            .unwrap();

        let token_data = decode::<Claims>(
            &auth.refresh_token,
            &DecodingKey::from_secret(TEST_SECRET.as_ref()),
            &Validation::default(),
        )
        .unwrap();

        assert_eq!(token_data.claims.sub, user_id);
        assert_eq!(token_data.claims.token_type, "refresh");
        let expected_exp = Utc::now().timestamp() + REFRESH_TOKEN_DURATION_DAYS * 86400;
        assert!(
            (token_data.claims.exp - expected_exp).abs() < 60,
            "refresh token exp should be ~7 days from now"
        );
    }

    #[actix_web::test]
    async fn verify_jwt_succeeds_with_valid_token() {
        let user_id = Uuid::now_v7();
        let auth = generate_token_pair(user_id, TEST_SECRET.to_string())
            .await
            .unwrap();

        let result = verify_jwt(auth.access_token, TEST_SECRET.to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().claims.sub, user_id);
    }

    #[actix_web::test]
    async fn verify_jwt_fails_with_wrong_secret() {
        let user_id = Uuid::now_v7();
        let auth = generate_token_pair(user_id, TEST_SECRET.to_string())
            .await
            .unwrap();

        let result = verify_jwt(auth.access_token, "wrong-secret".to_string()).await;
        assert!(result.is_err());
    }

    #[actix_web::test]
    async fn verify_jwt_fails_with_expired_token() {
        let user_id = Uuid::now_v7();
        let token = {
            let encoding_key = EncodingKey::from_secret(TEST_SECRET.as_ref());
            let claims = Claims {
                sub: user_id,
                exp: (Utc::now() - Duration::try_hours(1).unwrap()).timestamp(),
                iat: (Utc::now() - Duration::try_hours(2).unwrap()).timestamp(),
                jti: Uuid::now_v7(),
                token_type: "access".to_string(),
            };
            encode(&Header::default(), &claims, &encoding_key).unwrap()
        };

        let result = verify_jwt(token, TEST_SECRET.to_string()).await;
        assert!(result.is_err());
    }

    #[actix_web::test]
    async fn verify_jwt_fails_with_tampered_token() {
        let user_id = Uuid::now_v7();
        let auth = generate_token_pair(user_id, TEST_SECRET.to_string())
            .await
            .unwrap();

        let mut bytes = auth.access_token.into_bytes();
        let idx = bytes.len() - 2;
        bytes[idx] = if bytes[idx] == b'A' { b'B' } else { b'A' };
        let tampered = String::from_utf8(bytes).unwrap();

        let result = verify_jwt(tampered, TEST_SECRET.to_string()).await;
        assert!(result.is_err());
    }

    #[actix_web::test]
    async fn each_token_has_unique_jti() {
        let user_id = Uuid::now_v7();
        let key = &DecodingKey::from_secret(TEST_SECRET.as_ref());
        let val = &Validation::default();

        let auth1 = generate_token_pair(user_id, TEST_SECRET.to_string())
            .await
            .unwrap();
        let auth2 = generate_token_pair(user_id, TEST_SECRET.to_string())
            .await
            .unwrap();

        let jtis: Vec<Uuid> = [
            &auth1.access_token,
            &auth1.refresh_token,
            &auth2.access_token,
            &auth2.refresh_token,
        ]
        .iter()
        .map(|t| decode::<Claims>(t, key, val).unwrap().claims.jti)
        .collect();

        for i in 0..jtis.len() {
            for j in (i + 1)..jtis.len() {
                assert_ne!(jtis[i], jtis[j], "all jti values must be unique");
            }
        }
    }

    // -- Token blacklist --

    #[actix_web::test]
    async fn revoke_token_adds_to_in_memory_blacklist() {
        let state = test_state();
        let jti = Uuid::now_v7().to_string();

        assert!(!is_token_in_memory_blacklist(&state, &jti));
        revoke_token_in_memory(&state, &jti);
        assert!(is_token_in_memory_blacklist(&state, &jti));
    }

    #[actix_web::test]
    async fn is_token_revoked_returns_false_for_unknown() {
        let state = test_state();
        assert!(!is_token_in_memory_blacklist(&state, "nonexistent-jti"));
    }

    // -- Cache invalidation --

    #[actix_web::test]
    async fn invalidate_cache_removes_existing_entry() {
        let state = test_state();
        let user = crate::models::UpdateUserEntry {
            user_id: Uuid::now_v7(),
            firstname: "Test".to_string(),
            lastname: "User".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        state.cache.pin().insert(
            "test@example.com".to_string(),
            CachedUser {
                user,
                cached_at: Utc::now(),
            },
        );
        assert!(state.cache.pin().contains_key("test@example.com"));

        let removed = invalidate_cache(state.clone(), "test@example.com");
        assert!(removed);
        assert!(!state.cache.pin().contains_key("test@example.com"));
    }

    #[actix_web::test]
    async fn invalidate_cache_returns_false_for_missing_key() {
        let state = test_state();
        let removed = invalidate_cache(state, "nonexistent@example.com");
        assert!(!removed);
    }
}
