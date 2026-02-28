use crate::errors::{json_error_handler, path_error_handler};
use crate::handlers::{roles::*, teams::*, users::*, *};
use crate::middleware::auth::{basic_validator, jwt_validator, refresh_validator};
use crate::middleware::openapi::*;
use actix_web::{
    middleware::Compat, web::delete, web::get, web::post, web::put, web::resource, web::scope,
    web::JsonConfig, web::PathConfig, web::ServiceConfig,
};
use actix_web_httpauth::middleware::HttpAuthentication;

pub fn routes(cfg: &mut ServiceConfig) {
    let basic_auth = HttpAuthentication::basic(basic_validator);
    let jwt_auth = HttpAuthentication::bearer(jwt_validator);
    let jwt_auth_revoke = HttpAuthentication::bearer(jwt_validator);
    let refresh_auth = HttpAuthentication::bearer(refresh_validator);

    cfg
        /* Status Endpoint: Is server running and connected to DB? */
        .route("/health", get().to(get_health))
        .service(scope("/explorer").service(get_swagger))
        .service(
            resource("/auth")
                .name("auth")
                .wrap(Compat::new(basic_auth))
                .route(post().to(auth_user)),
        )
        .service(
            resource("/auth/refresh")
                .name("auth_refresh")
                .wrap(Compat::new(refresh_auth))
                .route(post().to(refresh_token)),
        )
        .service(
            resource("/auth/revoke")
                .name("auth_revoke")
                .wrap(Compat::new(jwt_auth_revoke))
                .route(post().to(revoke_user_token)),
        )
        .service(
            scope("/api/v1.0")
                .wrap(Compat::new(jwt_auth))
                .app_data(JsonConfig::default().error_handler(json_error_handler))
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
                            resource("/email/{user_id}")
                                .name("/users/email/user_id")
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
                            resource("/{team_id}/users")
                                .name("/teams/team_id/users")
                                .route(get().to(team_users)),
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
                ),
        );
}
