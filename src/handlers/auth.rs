use crate::config::AppState;
use crate::errors::AppError;
use crate::middleware::auth::create_token;
use crate::models::user::User;
use axum::extract::{Json, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use validator::ValidateEmail;

#[derive(Deserialize)]
pub struct RegisterRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    id: uuid::Uuid,
    email: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), AppError> {
    if !payload.email.validate_email() {
        return Err(AppError::BadRequest("Invalid email format".into()));
    }
    if payload.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters long".into(),
        ));
    }

    let password_hash = hash_password(&payload.password)?;

    let user = sqlx::query_as!(
        User,
        r#"INSERT INTO users (email, password_hash)
           VALUES ($1, $2)
           RETURNING id, email, password_hash"#,
        payload.email,
        password_hash
    )
    .fetch_one(&state.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            id: user.id,
            email: user.email,
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let user = sqlx::query_as!(
        User,
        r#"SELECT id, email, password_hash
           FROM users
           WHERE email = $1"#,
        payload.email
    )
    .fetch_optional(&state.pool)
    .await?;

    let user = user.ok_or(AppError::Unauthorized)?;

    if !verify_password(&payload.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let token = create_token(&user.id.to_string(), &state.jwt_secret);

    Ok(Json(LoginResponse { token }))
}

fn hash_password(password: &str) -> Result<String, AppError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|_| AppError::Internal("Password hashing failed".into()))
}

fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    bcrypt::verify(password, password_hash)
        .map_err(|_| AppError::Internal("Password verification failed".into()))
}
