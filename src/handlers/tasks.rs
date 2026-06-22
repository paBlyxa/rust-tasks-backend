use crate::errors::AppError;
use crate::middleware::auth::Claims;
use crate::models::task::TaskStats;
use crate::{
    config::AppState,
    models::task::{CreateTaskRequest, ListParams, Task, UpdateTaskRequest},
};
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::QueryBuilder;
use uuid::Uuid;

pub async fn list_tasks(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<Task>>, AppError> {
    let user_id = claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| AppError::Internal("Invalid user ID".into()))?;

    let sort_col = match params.sort.as_deref() {
        Some("title") => "title",
        Some("updated_at") => "updated_at",
        _ => "created_at",
    };

    let order = match params.order.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    let per_page = params.per_page.unwrap_or(20).min(100) as i64;
    let page = params.page.unwrap_or(1).max(1) as i64;
    let offset = (page - 1) * per_page;

    let mut query_builder = QueryBuilder::new(
        r#"SELECT id, title, description, user_id, completed, created_at, updated_at
           FROM tasks
           WHERE user_id = "#,
    );
    query_builder.push_bind(user_id);

    if let Some(completed) = params.completed {
        query_builder.push(" AND completed = ");
        query_builder.push_bind(completed);
    };

    query_builder.push(format!(" ORDER BY {sort_col} {order} LIMIT "));
    query_builder.push_bind(per_page);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let tasks = query_builder
        .build_query_as::<Task>()
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(tasks))
}

pub async fn create_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<Task>), AppError> {
    let title = payload.title.trim();
    if title.is_empty() || title.chars().count() > 255 {
        return Err(AppError::BadRequest(
            "Title must be between 1 and 255 characters".into(),
        ));
    }

    let user_id = claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| AppError::Internal("Invalid user ID".into()))?;

    let task = sqlx::query_as!(
        Task,
        r#"INSERT INTO tasks (title, description, user_id)
           VALUES ($1, $2, $3)
           RETURNING id, title, description, user_id, completed, created_at, updated_at"#,
        title,
        payload.description,
        user_id
    )
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(task)))
}

pub async fn get_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<Task>, AppError> {
    let user_id = claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| AppError::Internal("Invalid user ID".into()))?;

    let task = sqlx::query_as!(
        Task,
        r#"SELECT id, title, description, user_id, completed, created_at, updated_at
           FROM tasks
           WHERE id = $1 AND user_id = $2"#,
        task_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    match task {
        Some(task) => Ok(Json(task)),
        None => Err(AppError::NotFound("Task not found".into())),
    }
}

pub async fn update_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(task_id): Path<Uuid>,
    Json(payload): Json<UpdateTaskRequest>,
) -> Result<Json<Task>, AppError> {
    let user_id = claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| AppError::Internal("Invalid user ID".into()))?;

    if let Some(title) = &payload.title {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 255 {
            return Err(AppError::BadRequest(
                "Title must be between 1 and 255 characters".into(),
            ));
        }
    }

    let task = sqlx::query_as!(
        Task,
        r#"UPDATE tasks
           SET title = COALESCE($1, title),
               description = COALESCE($2, description),
               completed = COALESCE($3, completed),
               updated_at = NOW()
           WHERE id = $4 AND user_id = $5
           RETURNING id, title, description, user_id, completed, created_at, updated_at"#,
        payload.title.map(|t| t.trim().to_string()),
        payload.description,
        payload.completed,
        task_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    match task {
        Some(task) => Ok(Json(task)),
        None => Err(AppError::NotFound("Task not found".into())),
    }
}

pub async fn delete_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(task_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let user_id = claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| AppError::Internal("Invalid user ID".into()))?;

    let result = sqlx::query!(
        r#"DELETE FROM tasks
           WHERE id = $1 AND user_id = $2"#,
        task_id,
        user_id
    )
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Task not found".into()));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_stats(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<TaskStats>, AppError> {
    let user_id = claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| AppError::Internal("Invalid user ID".into()))?;

    let stats = sqlx::query!(
        r#"SELECT
            COUNT(*) AS "total!",
            COUNT(*) FILTER (WHERE completed) AS "completed!",
            COUNT(*) FILTER (WHERE NOT completed) AS "pending!"
           FROM tasks
           WHERE user_id = $1"#,
        user_id
    )
    .fetch_one(&state.pool)
    .await?;

    let percentage = if stats.total > 0 {
        (stats.completed as f64 / stats.total as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(TaskStats {
        total: stats.total,
        completed: stats.completed,
        pending: stats.pending,
        completed_percentage: percentage,
    }))
}
