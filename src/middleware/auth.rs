use crate::{
    db::get_user, db::get_user_by_email, errors::Error, handlers::get_client, models::Auth,
    models::Claims, models::State,
};
use actix_web::{dev::ServiceRequest, web::block, web::Data};
use actix_web_httpauth::extractors::{basic::BasicAuth, bearer::BearerAuth};
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde_json::json;
use tracing::{instrument, warn};
use uuid::Uuid;

// Access token lifetime: 15 minutes
const ACCESS_TOKEN_DURATION_MINUTES: i64 = 15;
// Refresh token lifetime: 7 days
const REFRESH_TOKEN_DURATION_DAYS: i64 = 7;

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
    let auth = block(move || {
        let access_token = generate_token(
            user_id,
            &jwt_secret,
            "access",
            Duration::try_minutes(ACCESS_TOKEN_DURATION_MINUTES).expect("valid duration"),
        )?;
        let refresh_token = generate_token(
            user_id,
            &jwt_secret,
            "refresh",
            Duration::try_days(REFRESH_TOKEN_DURATION_DAYS).expect("valid duration"),
        )?;
        Ok::<Auth, jsonwebtoken::errors::Error>(Auth {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: ACCESS_TOKEN_DURATION_MINUTES * 60,
        })
    })
    .await
    .map_err(|e| Error::Argonautica(format!("blocking error: {e}")))?;
    Ok(auth?)
}

#[instrument(skip(jwt_secret), level = "debug")]
pub async fn verify_jwt(token: String, jwt_secret: String) -> Result<TokenData<Claims>, Error> {
    let result = block(move || {
        let decoding_key = DecodingKey::from_secret(jwt_secret.as_ref());
        let validation = Validation::default();
        decode::<Claims>(&token, &decoding_key, &validation)
    })
    .await
    .map_err(|e| Error::Argonautica(format!("blocking error: {e}")))?;
    Ok(result?)
}

pub fn revoke_token(state: &Data<State>, jti: &str) {
    state.token_blacklist.pin().insert(jti.to_string(), true);
}

pub fn is_token_revoked(state: &Data<State>, jti: &str) -> bool {
    state.token_blacklist.pin().contains_key(jti)
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
    let state = req.app_data::<Data<State>>().unwrap();
    let client = get_client(state.pool.clone())
        .await
        .map_err(|err| (err, &req))
        .unwrap();
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
        if is_token_revoked(state, &c.claims.jti.to_string()) {
            warn!(jti = %c.claims.jti, "Rejected revoked token");
            return Err((
                actix_web::error::ErrorUnauthorized(json!({"error":"Token has been revoked"})),
                req,
            ));
        }
        match get_user(&client, c.claims.sub).await {
            Ok(_) => Ok(req),
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
    let state = req.app_data::<Data<State>>().unwrap();
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
        if is_token_revoked(state, &c.claims.jti.to_string()) {
            warn!(jti = %c.claims.jti, "Rejected revoked refresh token");
            return Err((
                actix_web::error::ErrorUnauthorized(json!({"error":"Token has been revoked"})),
                req,
            ));
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
    let state = req.app_data::<Data<State>>().unwrap();
    let cache = &state.cache;
    let client = get_client(state.pool.clone()).await.unwrap();
    let user = match cache.pin().get(&credentials.user_id().to_string()) {
        Some(u) => u.to_owned(),
        None => {
            let u = get_user_by_email(&client, credentials.user_id())
                .await
                .unwrap();
            cache
                .pin()
                .insert(credentials.user_id().to_string(), u.clone());
            u
        }
    };
    if let Some(pswd) = credentials.password() {
        let parsed_hash = PasswordHash::new(&user.password).unwrap();
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
    use jsonwebtoken::{decode, DecodingKey, Validation};

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
    async fn revoke_token_adds_to_blacklist() {
        let state = test_state();
        let jti = Uuid::now_v7().to_string();

        assert!(!is_token_revoked(&state, &jti));
        revoke_token(&state, &jti);
        assert!(is_token_revoked(&state, &jti));
    }

    #[actix_web::test]
    async fn is_token_revoked_returns_false_for_unknown() {
        let state = test_state();
        assert!(!is_token_revoked(&state, "nonexistent-jti"));
    }
}
