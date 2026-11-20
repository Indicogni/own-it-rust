//! The HTTP API: health, login, and one authenticated query.

use crate::auth::Authenticator;
use crate::store;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, FromRequestParts, Path, Query, State};
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tower_http::timeout::TimeoutLayer;

const DEFAULT_LIMIT: u32 = 100;
const MAX_LIMIT: u32 = 1000;
/// One password check holds about 19 MiB and a core for its whole duration,
/// and anyone can ask for one. So only this many run at once; further logins
/// wait for a slot, and the request timeout bounds that wait.
const MAX_CONCURRENT_LOGINS: usize = 4;

#[derive(Clone)]
pub struct ApiState {
    pool: SqlitePool,
    auth: Arc<Authenticator>,
    login_slots: Arc<Semaphore>,
}

impl ApiState {
    pub fn new(pool: SqlitePool, auth: Authenticator) -> ApiState {
        ApiState {
            pool,
            auth: Arc::new(auth),
            login_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_LOGINS)),
        }
    }
}

/// Every error the API can answer with. The body is `{"error": "<code>"}`.
#[derive(Debug)]
enum ApiError {
    NotFound,
    MethodNotAllowed,
    BadRequest(&'static str),
    InvalidCredentials,
    Unauthorized,
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "method_not_allowed"),
            ApiError::BadRequest(code) => (StatusCode::BAD_REQUEST, code),
            ApiError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid_credentials"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        };
        let body = Json(json!({ "error": code }));
        if matches!(self, ApiError::Unauthorized) {
            (status, [(header::WWW_AUTHENTICATE, "Bearer")], body).into_response()
        } else {
            (status, body).into_response()
        }
    }
}

fn internal(error: impl std::fmt::Display) -> ApiError {
    tracing::error!(%error, "request failed");
    ApiError::Internal
}

/// The user behind a valid bearer token.
struct AuthUser(#[allow(dead_code)] String);

impl FromRequestParts<ApiState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &ApiState) -> Result<AuthUser, ApiError> {
        parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .and_then(|token| state.auth.verify_token(token))
            .map(AuthUser)
            .ok_or(ApiError::Unauthorized)
    }
}

#[derive(Deserialize)]
struct Credentials {
    user: String,
    password: String,
}

async fn login(
    State(state): State<ApiState>,
    body: Result<Json<Credentials>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(credentials) = body.map_err(|_| ApiError::BadRequest("invalid_body"))?;
    let auth = Arc::clone(&state.auth);
    let user = credentials.user.clone();
    let slot = Arc::clone(&state.login_slots)
        .acquire_owned()
        .await
        .map_err(internal)?;
    // Argon2 takes tens of milliseconds of pure CPU: off the async threads.
    // The slot travels with the job. If this request is cancelled (the timeout
    // layer fires), the hash still runs to its end and still holds its slot.
    let ok = tokio::task::spawn_blocking(move || {
        let _slot = slot;
        auth.check_password(&credentials.user, &credentials.password)
    })
    .await
    .map_err(internal)?;
    if !ok {
        return Err(ApiError::InvalidCredentials);
    }
    Ok(Json(json!({ "token": state.auth.issue_token(&user) })))
}

async fn samples(
    State(state): State<ApiState>,
    _user: AuthUser,
    path: Result<Path<(String, String)>, PathRejection>,
    query: Result<Query<HashMap<String, String>>, QueryRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Taken as `Result`, so that axum's plain-text rejections (for example for
    // a path or query that is not valid UTF-8) become this API's JSON errors.
    let Path((agent, metric)) = path.map_err(|_| ApiError::NotFound)?;
    let Query(query) = query.map_err(|_| ApiError::BadRequest("invalid_query"))?;
    let limit = match query.get("limit") {
        None => DEFAULT_LIMIT,
        Some(text) => match text.parse::<u32>() {
            Ok(limit) if (1..=MAX_LIMIT).contains(&limit) => limit,
            _ => return Err(ApiError::BadRequest("invalid_query")),
        },
    };
    // Timestamps are positive, so -1 means "from the beginning".
    let after = match query.get("after") {
        None => -1,
        Some(text) => text
            .parse::<i64>()
            .map_err(|_| ApiError::BadRequest("invalid_query"))?,
    };
    // One row more than the page tells us whether another page exists.
    let mut rows = store::samples_after(&state.pool, &agent, &metric, after, limit + 1)
        .await
        .map_err(internal)?;
    let has_more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next_after = match (has_more, rows.last()) {
        (true, Some((ts_ms, _))) => Some(*ts_ms),
        _ => None,
    };
    let samples: Vec<_> = rows
        .iter()
        .map(|(ts_ms, value)| json!({ "ts_ms": ts_ms, "value": value }))
        .collect();
    Ok(Json(
        json!({ "samples": samples, "next_after": next_after }),
    ))
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/v1/login", post(login))
        .route("/v1/agents/{agent}/metrics/{metric}/samples", get(samples))
        .fallback(|| async { ApiError::NotFound })
        .method_not_allowed_fallback(|| async { ApiError::MethodNotAllowed })
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            Duration::from_secs(10),
        ))
        .with_state(state)
}
