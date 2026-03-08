use crate::errors::{json_error_handler, path_error_handler};
use crate::handlers::{avatars::*, items::*, orders::*, roles::*, teams::*, users::*, *};
use crate::middleware::auth::{basic_validator, jwt_validator, refresh_validator};
use crate::middleware::openapi::*;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{
    middleware::{Compat, DefaultHeaders},
    web::JsonConfig,
    web::PathConfig,
    web::PayloadConfig,
    web::ServiceConfig,
    web::delete,
    web::get,
    web::post,
    web::put,
    web::resource,
    web::scope,
};
use actix_web_httpauth::middleware::HttpAuthentication;
use std::env;

pub fn routes(cfg: &mut ServiceConfig) {
    let basic_auth = HttpAuthentication::basic(basic_validator);
    let jwt_auth = HttpAuthentication::bearer(jwt_validator);
    let jwt_auth_revoke = HttpAuthentication::bearer(jwt_validator);
    let refresh_auth = HttpAuthentication::bearer(refresh_validator);

    // Rate limiter for auth endpoints: 10 requests per minute burst, sustained 1 per 6s
    let auth_rate_limit = GovernorConfigBuilder::default()
        .seconds_per_request(6)
        .burst_size(10)
        .finish()
        .expect("valid rate limiter config");

    let swagger_enabled = env::var("ENABLE_SWAGGER")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    cfg
        /* Status Endpoint: Is server running and connected to DB? */
        .route("/health", get().to(get_health));

    // Swagger UI: opt-in via ENABLE_SWAGGER=true
    if swagger_enabled {
        cfg.service(scope("/explorer").service(get_swagger));
    }

    cfg.service(
        resource("/auth")
            .name("auth")
            .wrap(Governor::new(&auth_rate_limit))
            .wrap(Compat::new(basic_auth))
            .route(post().to(auth_user)),
    )
    .service(
        resource("/auth/register")
            .name("auth_register")
            .wrap(Governor::new(&auth_rate_limit))
            .app_data(
                JsonConfig::default()
                    .limit(65_536)
                    .error_handler(json_error_handler),
            )
            .route(post().to(register_first_user)),
    )
    .service(
        resource("/auth/refresh")
            .name("auth_refresh")
            .wrap(Governor::new(&auth_rate_limit))
            .wrap(Compat::new(refresh_auth))
            .app_data(
                JsonConfig::default()
                    .limit(65_536)
                    .error_handler(json_error_handler),
            )
            .route(post().to(refresh_token)),
    )
    .service(
        resource("/auth/revoke")
            .name("auth_revoke")
            .wrap(Governor::new(&auth_rate_limit))
            .wrap(Compat::new(jwt_auth_revoke))
            .app_data(
                JsonConfig::default()
                    .limit(65_536)
                    .error_handler(json_error_handler),
            )
            .route(post().to(revoke_user_token)),
    )
    // Avatar images are served publicly (no JWT) — they are static LEGO minifig
    // thumbnails fetched by <img> tags which cannot carry Authorization headers.
    .service(
        resource("/api/v1.0/avatars/{avatar_id}")
            .name("/public/avatars/avatar_id")
            .route(get().to(get_avatar)),
    )
    .service(
        // All endpoints under /api/v1.0 require JWT authentication.
        //
        // GET endpoints intentionally require only JWT auth (no team-scoped RBAC).
        // Data visibility is open to all authenticated users — there is no multi-tenant
        // data isolation. Team-scoped RBAC is enforced only on mutations (POST/PUT/DELETE)
        // within individual handlers via require_admin, require_team_admin, etc.
        scope("/api/v1.0")
            .wrap(DefaultHeaders::new().add(("Cache-Control", "no-store, private")))
            .wrap(Compat::new(jwt_auth))
            .app_data(
                JsonConfig::default()
                    .limit(65_536)
                    .error_handler(json_error_handler),
            )
            .app_data(PayloadConfig::default().limit(65_536))
            .app_data(PathConfig::default().error_handler(path_error_handler))
            .service(
                resource("/users")
                    .name("/users")
                    .route(get().to(get_users))
                    .route(post().to(create_user)),
            )
            .service(
                scope("/users")
                    .service(
                        resource("/{user_id}")
                            .name("/users/user_id")
                            .route(delete().to(delete_user))
                            .route(get().to(get_user))
                            .route(put().to(update_user)),
                    )
                    .service(
                        resource("/{user_id}/teams")
                            .name("/users/user_id/teams")
                            .route(get().to(user_teams)),
                    )
                    .service(
                        resource("/{user_id}/avatar")
                            .name("/users/user_id/avatar")
                            .route(put().to(set_avatar))
                            .route(delete().to(remove_avatar)),
                    )
                    .service(
                        resource("/email/{email}")
                            .name("/users/email/email")
                            .route(delete().to(delete_user_by_email)),
                    ),
            )
            .service(
                resource("/teams")
                    .name("/teams")
                    .route(get().to(get_teams))
                    .route(post().to(create_team)),
            )
            .service(
                scope("/teams")
                    .service(
                        resource("/{team_id}")
                            .name("/teams/team_id")
                            .route(delete().to(delete_team))
                            .route(get().to(get_team))
                            .route(put().to(update_team)),
                    )
                    .service(
                        resource("/{team_id}/orders")
                            .name("/teams/team_id/orders")
                            .route(delete().to(delete_team_orders))
                            .route(get().to(get_team_orders))
                            .route(post().to(create_team_order)),
                    )
                    .service(
                        resource("/{team_id}/orders/{order_id}")
                            .name("/teams/team_id/order_id")
                            .route(get().to(get_team_order))
                            .route(delete().to(delete_team_order))
                            .route(put().to(update_team_order)),
                    )
                    .service(
                        resource("/{team_id}/orders/{order_id}/reopen")
                            .name("/teams/team_id/order_id/reopen")
                            .route(post().to(reopen_team_order)),
                    )
                    .service(
                        resource("/{team_id}/users")
                            .name("/teams/team_id/users")
                            .route(get().to(team_users))
                            .route(post().to(add_team_member)),
                    )
                    .service(
                        resource("/{team_id}/users/{user_id}")
                            .name("/teams/team_id/users/user_id")
                            .route(delete().to(remove_team_member))
                            .route(put().to(update_member_role)),
                    )
                    .service(
                        resource("/{team_id}/orders/{order_id}/items")
                            .name("/teams/team_id/orders/order_id/items")
                            .route(get().to(get_order_items))
                            .route(post().to(create_order_item)),
                    )
                    .service(
                        resource("/{team_id}/orders/{order_id}/items/{item_id}")
                            .name("/teams/team_id/orders/order_id/items/item_id")
                            .route(get().to(get_order_item))
                            .route(delete().to(delete_order_item))
                            .route(put().to(update_order_item)),
                    ),
            )
            .service(
                resource("/items")
                    .name("/items")
                    .route(get().to(get_items))
                    .route(post().to(create_item)),
            )
            .service(
                scope("/items").service(
                    resource("/{item_id}")
                        .name("/items/item_id")
                        .route(get().to(get_item))
                        .route(delete().to(delete_item))
                        .route(put().to(update_item)),
                ),
            )
            .service(
                resource("/roles")
                    .name("/roles")
                    .route(get().to(get_roles))
                    .route(post().to(create_role)),
            )
            .service(
                scope("/roles").service(
                    resource("/{role_id}")
                        .name("/roles/role_id")
                        .route(get().to(get_role))
                        .route(delete().to(delete_role))
                        .route(put().to(update_role)),
                ),
            )
            .service(
                resource("/avatars")
                    .name("/avatars")
                    .route(get().to(get_avatars)),
            ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::State;
    use actix_web::{App, test, web::Data};
    use dashmap::DashMap;
    use std::net::SocketAddr;

    /// Fake peer address required by actix-governor's PeerIpKeyExtractor.
    const PEER: SocketAddr = SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        12345,
    );

    /// Build a `Data<State>` with a dummy pool that will fail on use.
    /// This is fine because we only test that routes exist (expect 401, not 404).
    fn dummy_state() -> Data<State> {
        let mut pg_cfg = deadpool_postgres::Config::new();
        pg_cfg.user = Some("x".into());
        pg_cfg.password = Some("x".into());
        pg_cfg.dbname = Some("x".into());
        pg_cfg.host = Some("127.0.0.1".into());
        pg_cfg.port = Some(1); // unreachable port
        let pool = pg_cfg
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .expect("pool creation should succeed");
        Data::new(State {
            pool,
            jwtsecret: secrecy::SecretString::from("test".to_string()),
            cache: DashMap::new(),
            token_blacklist: DashMap::new(),
            login_attempts: DashMap::new(),
            avatar_cache: DashMap::new(),
        })
    }

    /// Helper: assert a route is registered by verifying the response is NOT 404.
    /// Protected endpoints should return 401 (auth required) or 500 (DB unavailable),
    /// never 404 (route not found).
    macro_rules! assert_route_exists {
        ($app:expr, $method:ident, $path:expr) => {{
            let req = test::TestRequest::$method()
                .uri($path)
                .peer_addr(PEER)
                .to_request();
            let resp = test::call_service(&$app, req).await;
            assert_ne!(
                resp.status().as_u16(),
                404,
                "Route {} {} should be registered but returned 404",
                stringify!($method),
                $path
            );
        }};
    }

    #[actix_web::test]
    async fn health_endpoint_is_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        assert_route_exists!(app, get, "/health");
    }

    #[actix_web::test]
    async fn auth_endpoint_is_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        // POST /auth requires Basic auth → 401
        assert_route_exists!(app, post, "/auth");
    }

    #[actix_web::test]
    async fn auth_refresh_endpoint_is_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        assert_route_exists!(app, post, "/auth/refresh");
    }

    #[actix_web::test]
    async fn auth_revoke_endpoint_is_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        assert_route_exists!(app, post, "/auth/revoke");
    }

    #[actix_web::test]
    async fn auth_register_endpoint_is_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        assert_route_exists!(app, post, "/auth/register");
    }

    #[actix_web::test]
    async fn users_collection_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        assert_route_exists!(app, get, "/api/v1.0/users");
        assert_route_exists!(app, post, "/api/v1.0/users");
    }

    #[actix_web::test]
    async fn users_item_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        let uid = "00000000-0000-0000-0000-000000000001";
        assert_route_exists!(app, get, &format!("/api/v1.0/users/{}", uid));
        assert_route_exists!(app, put, &format!("/api/v1.0/users/{}", uid));
        assert_route_exists!(app, delete, &format!("/api/v1.0/users/{}", uid));
    }

    #[actix_web::test]
    async fn user_teams_route_is_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        let uid = "00000000-0000-0000-0000-000000000001";
        assert_route_exists!(app, get, &format!("/api/v1.0/users/{}/teams", uid));
    }

    #[actix_web::test]
    async fn delete_user_by_email_route_is_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        assert_route_exists!(app, delete, "/api/v1.0/users/email/test@example.com");
    }

    #[actix_web::test]
    async fn teams_collection_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        assert_route_exists!(app, get, "/api/v1.0/teams");
        assert_route_exists!(app, post, "/api/v1.0/teams");
    }

    #[actix_web::test]
    async fn teams_item_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        let tid = "00000000-0000-0000-0000-000000000001";
        assert_route_exists!(app, get, &format!("/api/v1.0/teams/{}", tid));
        assert_route_exists!(app, put, &format!("/api/v1.0/teams/{}", tid));
        assert_route_exists!(app, delete, &format!("/api/v1.0/teams/{}", tid));
    }

    #[actix_web::test]
    async fn team_orders_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        let tid = "00000000-0000-0000-0000-000000000001";
        let oid = "00000000-0000-0000-0000-000000000002";
        assert_route_exists!(app, get, &format!("/api/v1.0/teams/{}/orders", tid));
        assert_route_exists!(app, post, &format!("/api/v1.0/teams/{}/orders", tid));
        assert_route_exists!(app, delete, &format!("/api/v1.0/teams/{}/orders", tid));
        assert_route_exists!(app, get, &format!("/api/v1.0/teams/{}/orders/{}", tid, oid));
        assert_route_exists!(app, put, &format!("/api/v1.0/teams/{}/orders/{}", tid, oid));
        assert_route_exists!(
            app,
            delete,
            &format!("/api/v1.0/teams/{}/orders/{}", tid, oid)
        );
    }

    #[actix_web::test]
    async fn team_members_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        let tid = "00000000-0000-0000-0000-000000000001";
        let uid = "00000000-0000-0000-0000-000000000002";
        assert_route_exists!(app, get, &format!("/api/v1.0/teams/{}/users", tid));
        assert_route_exists!(app, post, &format!("/api/v1.0/teams/{}/users", tid));
        assert_route_exists!(
            app,
            delete,
            &format!("/api/v1.0/teams/{}/users/{}", tid, uid)
        );
        assert_route_exists!(app, put, &format!("/api/v1.0/teams/{}/users/{}", tid, uid));
    }

    #[actix_web::test]
    async fn order_items_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        let tid = "00000000-0000-0000-0000-000000000001";
        let oid = "00000000-0000-0000-0000-000000000002";
        let iid = "00000000-0000-0000-0000-000000000003";
        assert_route_exists!(
            app,
            get,
            &format!("/api/v1.0/teams/{}/orders/{}/items", tid, oid)
        );
        assert_route_exists!(
            app,
            post,
            &format!("/api/v1.0/teams/{}/orders/{}/items", tid, oid)
        );
        assert_route_exists!(
            app,
            get,
            &format!("/api/v1.0/teams/{}/orders/{}/items/{}", tid, oid, iid)
        );
        assert_route_exists!(
            app,
            put,
            &format!("/api/v1.0/teams/{}/orders/{}/items/{}", tid, oid, iid)
        );
        assert_route_exists!(
            app,
            delete,
            &format!("/api/v1.0/teams/{}/orders/{}/items/{}", tid, oid, iid)
        );
    }

    #[actix_web::test]
    async fn items_collection_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        assert_route_exists!(app, get, "/api/v1.0/items");
        assert_route_exists!(app, post, "/api/v1.0/items");
    }

    #[actix_web::test]
    async fn items_item_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        let iid = "00000000-0000-0000-0000-000000000001";
        assert_route_exists!(app, get, &format!("/api/v1.0/items/{}", iid));
        assert_route_exists!(app, put, &format!("/api/v1.0/items/{}", iid));
        assert_route_exists!(app, delete, &format!("/api/v1.0/items/{}", iid));
    }

    #[actix_web::test]
    async fn roles_collection_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        assert_route_exists!(app, get, "/api/v1.0/roles");
        assert_route_exists!(app, post, "/api/v1.0/roles");
    }

    #[actix_web::test]
    async fn roles_item_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        let rid = "00000000-0000-0000-0000-000000000001";
        assert_route_exists!(app, get, &format!("/api/v1.0/roles/{}", rid));
        assert_route_exists!(app, put, &format!("/api/v1.0/roles/{}", rid));
        assert_route_exists!(app, delete, &format!("/api/v1.0/roles/{}", rid));
    }

    #[actix_web::test]
    async fn avatars_collection_route_is_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        assert_route_exists!(app, get, "/api/v1.0/avatars");
    }

    #[actix_web::test]
    async fn avatars_item_route_is_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        let aid = "00000000-0000-0000-0000-000000000001";
        assert_route_exists!(app, get, &format!("/api/v1.0/avatars/{}", aid));
    }

    #[actix_web::test]
    async fn user_avatar_routes_are_registered() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        let uid = "00000000-0000-0000-0000-000000000001";
        assert_route_exists!(app, put, &format!("/api/v1.0/users/{}/avatar", uid));
        assert_route_exists!(app, delete, &format!("/api/v1.0/users/{}/avatar", uid));
    }

    #[actix_web::test]
    async fn unregistered_route_outside_api_scope_returns_404() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        // A path completely outside any registered scope should return 404
        let req = test::TestRequest::get()
            .uri("/nonexistent/path")
            .peer_addr(PEER)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status().as_u16(),
            404,
            "Unregistered route outside API scope should return 404"
        );
    }

    #[actix_web::test]
    async fn unregistered_route_inside_api_scope_returns_401() {
        let state = dummy_state();
        let app = test::init_service(App::new().app_data(state.clone()).configure(routes)).await;
        // A path inside /api/v1.0 scope is covered by JWT middleware,
        // so it returns 401 (auth required) rather than 404
        let req = test::TestRequest::get()
            .uri("/api/v1.0/nonexistent")
            .peer_addr(PEER)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status().as_u16(),
            401,
            "Unregistered route inside JWT scope should return 401"
        );
    }
}
