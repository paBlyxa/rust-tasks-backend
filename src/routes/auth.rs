use crate::config::AppState;
use crate::handlers;
use axum::routing::post;
use axum::Router;

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/login", post(handlers::auth::login))
}
