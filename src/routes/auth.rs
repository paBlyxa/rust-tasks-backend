use crate::config::AppState;
use crate::handlers;
use axum::Router;
use axum::routing::post;

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/login", post(handlers::auth::login))
}
