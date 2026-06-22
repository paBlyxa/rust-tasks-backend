use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
}

#[derive(Deserialize)]
pub struct ListParams {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
    pub completed: Option<bool>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub user_id: Uuid,
    pub completed: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
pub struct TaskStats {
    pub total: i64,
    pub completed: i64,
    pub pending: i64,
    pub completed_percentage: f64,
}
