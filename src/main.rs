use sqlx::postgres::PgPoolOptions;
use tasks_backend::{config, routes};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = config::Config::from_env();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database.");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations.");

    let state = config::AppState {
        pool,
        jwt_secret: config.jwt_secret,
    };

    let app = routes::create_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Server running on http://0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}
