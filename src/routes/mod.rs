use crate::config::AppState;
use axum::Router;
pub mod auth;
pub mod tasks;

pub fn create_router(state: AppState) -> Router {
    let jwt_secret = state.jwt_secret.clone();
    Router::new()
        .merge(auth::auth_router())
        .merge(tasks::tasks_router(jwt_secret))
        .with_state(state)
}
