use crate::config::AppState;
use crate::handlers;
use axum::routing::get;
use axum::{Router, middleware};

pub fn tasks_router(jwt_secret: String) -> Router<AppState> {
    Router::new()
        .route(
            "/tasks",
            get(handlers::tasks::list_tasks).post(handlers::tasks::create_task),
        )
        .route("/tasks/stats", get(handlers::tasks::get_stats))
        .route(
            "/tasks/{id}",
            get(handlers::tasks::get_task)
                .put(handlers::tasks::update_task)
                .delete(handlers::tasks::delete_task),
        )
        .layer(middleware::from_fn_with_state(
            jwt_secret,
            crate::middleware::auth::auth_middleware,
        ))
}
