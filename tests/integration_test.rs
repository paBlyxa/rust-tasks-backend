use reqwest::Client;
use sqlx::postgres::PgPoolOptions;
use tasks_backend::{config::AppState, routes::create_router};
use uuid::Uuid;

async fn spawn_app() -> String {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "test-secret".into());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database.");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations.");

    let state = AppState { pool, jwt_secret };
    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://127.0.0.1:{}", port)
}

async fn register_and_login(base_url: &str, email: &str, password: &str) -> String {
    let client = Client::new();

    // Register
    client
        .post(format!("{base_url}/auth/register"))
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .await
        .unwrap();

    // Login
    let result = client
        .post(format!("{base_url}/auth/login"))
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    result["token"].as_str().unwrap().to_string()
}

async fn create_task(base_url: &str, token: &str, title: &str) -> serde_json::Value {
    Client::new()
        .post(format!("{base_url}/tasks"))
        .bearer_auth(token)
        .json(&serde_json::json!({ "title": title }))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap()
}

#[tokio::test]
async fn test_register_success() {
    let base_url = spawn_app().await;
    let client = Client::new();

    let email = format!("test_{}@example.com", Uuid::new_v4());
    let response = client
        .post(format!("{base_url}/auth/register"))
        .json(&serde_json::json!({ "email": email, "password": "password123" }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 201);
}

#[tokio::test]
async fn test_register_invalid_email() {
    let base_url = spawn_app().await;
    let client = Client::new();

    let response = client
        .post(format!("{base_url}/auth/register"))
        .json(&serde_json::json!({ "email": "invalid-email", "password": "password123" }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_create_and_get_task() {
    let base_url = spawn_app().await;
    let email = format!("test_{}@example.com", Uuid::new_v4());
    let token = register_and_login(&base_url, &email, "password123").await;
    let client = Client::new();

    let created = client
        .post(format!("{base_url}/tasks"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "title": "Test Task", "description": "This is a test task." }))
        .send()
        .await
        .unwrap();

    assert_eq!(created.status(), 201);

    let task: serde_json::Value = created.json().await.unwrap();
    let task_id = task["id"].as_str().unwrap();

    let fetched = client
        .get(format!("{base_url}/tasks/{task_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(fetched.status(), 200);
    let fetched_task = fetched.json::<serde_json::Value>().await.unwrap();
    assert_eq!(fetched_task["title"], "Test Task");
}

#[tokio::test]
async fn test_update_task() {
    let base_url = spawn_app().await;
    let email = format!("test_{}@example.com", Uuid::new_v4());
    let token = register_and_login(&base_url, &email, "password123").await;
    let client = Client::new();

    let task = create_task(&base_url, &token, "Original title").await;
    let task_id = task["id"].as_str().unwrap();

    let res = client
        .put(format!("{base_url}/tasks/{task_id}"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": "Updated title",
            "completed": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let updated = res.json::<serde_json::Value>().await.unwrap();
    assert_eq!(updated["title"], "Updated title");
    assert_eq!(updated["completed"], true);
}

#[tokio::test]
async fn test_update_another_users_task_returns_404() {
    let base_url = spawn_app().await;
    let client = Client::new();

    // первый пользователь создаёт задачу
    let email1 = format!("test_{}@example.com", Uuid::new_v4());
    let token1 = register_and_login(&base_url, &email1, "password123").await;
    let task = create_task(&base_url, &token1, "User 1 task").await;
    let task_id = task["id"].as_str().unwrap();

    // второй пользователь пытается обновить чужую задачу
    let email2 = format!("test_{}@example.com", Uuid::new_v4());
    let token2 = register_and_login(&base_url, &email2, "password123").await;

    let res = client
        .put(format!("{base_url}/tasks/{task_id}"))
        .bearer_auth(&token2)
        .json(&serde_json::json!({ "title": "Hacked!" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn test_delete_task() {
    let base_url = spawn_app().await;
    let email = format!("test_{}@example.com", Uuid::new_v4());
    let token = register_and_login(&base_url, &email, "password123").await;
    let client = Client::new();

    let task = create_task(&base_url, &token, "Task to delete").await;
    let task_id = task["id"].as_str().unwrap();

    // удаляем
    let res = client
        .delete(format!("{base_url}/tasks/{task_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 204);

    // повторный запрос возвращает 404
    let res = client
        .get(format!("{base_url}/tasks/{task_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn test_tasks_require_auth() {
    let base_url = spawn_app().await;
    let client = Client::new();

    let response = client
        .get(format!("{base_url}/tasks"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_stats() {
    let base_url = spawn_app().await;
    let email = format!("test_{}@example.com", Uuid::new_v4());
    let token = register_and_login(&base_url, &email, "password123").await;
    let client = Client::new();

    for title in &["Task 1", "Task 2", "Task 3"] {
        client
            .post(format!("{base_url}/tasks"))
            .bearer_auth(&token)
            .json(&serde_json::json!({ "title": title, "description": format!("This is {}.", title) }))
            .send()
            .await
            .unwrap();
    }

    let response = client
        .get(format!("{base_url}/tasks/stats"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let stats = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(stats["total"].as_i64().unwrap(), 3);
    assert_eq!(stats["completed"].as_i64().unwrap(), 0);
    assert_eq!(stats["pending"].as_i64().unwrap(), 3);
    assert_eq!(stats["completed_percentage"].as_f64().unwrap(), 0.0);
}

#[tokio::test]
async fn test_list_tasks_with_filter_and_pagination() {
    let base_url = spawn_app().await;
    let email = format!("test_{}@example.com", Uuid::new_v4());
    let token = register_and_login(&base_url, &email, "password123").await;
    let client = Client::new();

    // создаём 3 задачи
    for i in 1..=3 {
        create_task(&base_url, &token, &format!("Task {i}")).await;
    }

    // отмечаем первую как выполненную
    let all = client
        .get(format!("{base_url}/tasks"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    let first_id = all[0]["id"].as_str().unwrap();
    client
        .put(format!("{base_url}/tasks/{first_id}"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "completed": true }))
        .send()
        .await
        .unwrap();

    // фильтр: только выполненные
    let res = client
        .get(format!("{base_url}/tasks?completed=true"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    assert_eq!(res.as_array().unwrap().len(), 1);
    assert_eq!(res[0]["completed"], true);

    // пагинация: 2 задачи на страницу
    let res = client
        .get(format!("{base_url}/tasks?per_page=2&page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    assert_eq!(res.as_array().unwrap().len(), 2);
}
