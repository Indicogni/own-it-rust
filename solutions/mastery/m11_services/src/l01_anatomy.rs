//! LESSON 01 — Service anatomy: router, handlers, extractors, state, typed errors
//! Read first: CONCEPTS.md, "Lesson 01". Start after: ecosystem E01, E02 and E06.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml l01_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a handler is a plain `async fn`. Who
//! decides which status code the client sees when the JSON body cannot be parsed,
//! and does your handler body run at all in that case?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Follow one `POST /parts` request from the socket to the response.
//! Name each step (routing, extractors, handler, `IntoResponse`) and say where the
//! shared state is cloned and where it is locked.
//! JUDGMENT: axum is a thin layer over hyper (HTTP) and tower (middleware), and it
//! is maintained by the tokio project, so it fits the rest of the tokio ecosystem.
//! actix-web is the other mature choice and has its own runtime setup and
//! middleware system. For a service with three routes, either is fine; choose the
//! one your team already runs. A `std::sync::Mutex` is the right lock for
//! short, non-async critical sections like the ones here; lesson C01 shows what
//! goes wrong when a lock is held across `.await`.
//!
//! The service is a parts bin for a repair shop. The HTTP contract:
//!
//! | Method and path             | Success                         | Errors                                   |
//! | --------------------------- | ------------------------------- | ---------------------------------------- |
//! | `GET /parts`                | 200, JSON array sorted by `sku` | none                                     |
//! | `GET /parts?min_qty=5`      | 200, only parts with `qty >= 5` | none                                     |
//! | `POST /parts`               | 201, the part, `Location` header| 409 `already_exists`, 422 `invalid`      |
//! | `GET /parts/{sku}`          | 200, the part                   | 404 `not_found`                          |
//! | `POST /parts/{sku}/adjust`  | 200, the part after the change  | 404 `not_found`, 409 `insufficient_stock`|
//! | `DELETE /parts/{sku}`       | 204, empty body                 | 404 `not_found`                          |
//!
//! Every error made by *your* code has the JSON body
//! `{"error": "<code>", "message": "<text for humans>"}`.

use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

/// One kind of part in the bin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub sku: String,
    pub name: String,
    pub qty: u32,
}

/// Body of `POST /parts/{sku}/adjust`: add `delta` (which may be negative) to `qty`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Adjust {
    pub delta: i64,
}

/// Query string of `GET /parts`.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct ListFilter {
    pub min_qty: Option<u32>,
}

/// Shared state. Cloning it clones the `Arc`, not the map, so every request
/// sees the same parts.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub parts: Arc<Mutex<BTreeMap<String, Part>>>,
}

/// Everything that can go wrong in this service, as one type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// No part has this sku.
    NotFound,
    /// A part with this sku exists already.
    AlreadyExists,
    /// The request was well-formed JSON of the right shape, but a value is not
    /// acceptable. The text says which.
    Invalid(String),
    /// The adjustment would make `qty` negative (or larger than `u32::MAX`).
    InsufficientStock,
}

/// A. Map each error to its status code and to the JSON error body described in
/// the header: `NotFound` → 404 `not_found`, `AlreadyExists` → 409 `already_exists`,
/// `Invalid` → 422 `invalid` (its text is the message), `InsufficientStock` → 409
/// `insufficient_stock`. The message of the other variants is yours to write, but
/// it must not be empty.
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "no part has this sku".to_owned(),
            ),
            ApiError::AlreadyExists => (
                StatusCode::CONFLICT,
                "already_exists",
                "a part with this sku exists already".to_owned(),
            ),
            ApiError::Invalid(message) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid", message),
            ApiError::InsufficientStock => (
                StatusCode::CONFLICT,
                "insufficient_stock",
                "the bin does not hold that many".to_owned(),
            ),
        };
        let body = serde_json::json!({ "error": code, "message": message });
        (status, Json(body)).into_response()
    }
}

/// B. `GET /parts`: every part, sorted by sku, filtered by `min_qty` when given.
pub async fn list_parts(
    State(state): State<AppState>,
    Query(filter): Query<ListFilter>,
) -> Json<Vec<Part>> {
    let parts = state.parts.lock().expect("parts lock");
    let minimum = filter.min_qty.unwrap_or(0);
    // A BTreeMap iterates in key order, so the result is sorted by sku already.
    Json(
        parts
            .values()
            .filter(|part| part.qty >= minimum)
            .cloned()
            .collect(),
    )
}

/// B. `POST /parts`: store a new part. `sku` and `name` must not be empty after
/// trimming (422 `invalid`); a known sku gives 409 `already_exists`. Success is 201
/// with the part as the body and a `Location: /parts/{sku}` header.
pub async fn create_part(
    State(state): State<AppState>,
    Json(part): Json<Part>,
) -> Result<Response, ApiError> {
    if part.sku.trim().is_empty() {
        return Err(ApiError::Invalid("sku must not be empty".to_owned()));
    }
    if part.name.trim().is_empty() {
        return Err(ApiError::Invalid("name must not be empty".to_owned()));
    }
    let mut parts = state.parts.lock().expect("parts lock");
    if parts.contains_key(&part.sku) {
        return Err(ApiError::AlreadyExists);
    }
    parts.insert(part.sku.clone(), part.clone());
    drop(parts);
    let location = format!("/parts/{}", part.sku);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(part),
    )
        .into_response())
}

/// C. Build the router for the whole contract in the header. `list_parts` and
/// `create_part` are given as signatures; write the other three handlers yourself
/// (as functions or closures) with the extractors they need. The adjustment must be
/// atomic: read and write `qty` under one lock acquisition, so that two requests
/// cannot both read the old value.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/parts", get(list_parts).post(create_part))
        .route("/parts/{sku}", get(get_part).delete(delete_part))
        .route("/parts/{sku}/adjust", post(adjust_part))
        .with_state(state)
}

async fn get_part(
    State(state): State<AppState>,
    Path(sku): Path<String>,
) -> Result<Json<Part>, ApiError> {
    let parts = state.parts.lock().expect("parts lock");
    parts.get(&sku).cloned().map(Json).ok_or(ApiError::NotFound)
}

async fn delete_part(
    State(state): State<AppState>,
    Path(sku): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut parts = state.parts.lock().expect("parts lock");
    match parts.remove(&sku) {
        Some(_) => Ok(StatusCode::NO_CONTENT),
        None => Err(ApiError::NotFound),
    }
}

async fn adjust_part(
    State(state): State<AppState>,
    Path(sku): Path<String>,
    Json(change): Json<Adjust>,
) -> Result<Json<Part>, ApiError> {
    // One lock acquisition covers the read and the write.
    let mut parts = state.parts.lock().expect("parts lock");
    let part = parts.get_mut(&sku).ok_or(ApiError::NotFound)?;
    // `delta` comes from the client, so the sum may leave i64 as well as u32.
    // Both cases are the same refusal; neither may panic under the lock.
    part.qty = i64::from(part.qty)
        .checked_add(change.delta)
        .and_then(|updated| u32::try_from(updated).ok())
        .ok_or(ApiError::InsufficientStock)?;
    Ok(Json(part.clone()))
}

/// D. Serve `app(state)` on `listener` until the task is cancelled or the listener
/// fails. This is the only place that touches a socket.
pub async fn serve(listener: TcpListener, state: AppState) -> std::io::Result<()> {
    axum::serve(listener, app(state)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{deadline, get, http, json_request, post_json, request, send};
    use axum::body::Body;
    use axum::http::{Method, Request};
    use serde_json::json;

    fn part(sku: &str, name: &str, qty: u32) -> serde_json::Value {
        json!({ "sku": sku, "name": name, "qty": qty })
    }

    async fn seeded() -> Router {
        let app = app(AppState::default());
        for body in [
            part("m4-nut", "M4 hex nut", 120),
            part("b-608", "608 bearing", 8),
            part("fuse-2a", "2 A fuse", 0),
        ] {
            let response = send(&app, post_json("/parts", &body)).await;
            assert_eq!(response.code(), 201, "seeding failed: {}", response.text());
        }
        app
    }

    #[tokio::test]
    async fn a_errors_map_to_status_and_json_body() {
        crate::support::deadline(
            120,
            "the test `a_errors_map_to_status_and_json_body`",
            async {
                for (error, status, code) in [
                    (ApiError::NotFound, 404, "not_found"),
                    (ApiError::AlreadyExists, 409, "already_exists"),
                    (ApiError::Invalid("qty is odd".to_owned()), 422, "invalid"),
                    (ApiError::InsufficientStock, 409, "insufficient_stock"),
                ] {
                    let response = error.clone().into_response();
                    assert_eq!(response.status().as_u16(), status, "status of {error:?}");
                    assert_eq!(
                        response.headers()[header::CONTENT_TYPE],
                        "application/json",
                        "error bodies are JSON, not plain text"
                    );
                    let bytes = axum::body::to_bytes(response.into_body(), 4096)
                        .await
                        .unwrap();
                    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                    assert_eq!(body["error"], code, "error code of {error:?}");
                    let message = body["message"].as_str().unwrap_or_default();
                    assert!(!message.is_empty(), "{error:?} needs a message for humans");
                    if let ApiError::Invalid(text) = &error {
                        assert_eq!(message, text, "Invalid carries its own message");
                    }
                }
            },
        )
        .await
    }

    #[tokio::test]
    async fn b_create_returns_201_with_location() {
        crate::support::deadline(
            120,
            "the test `b_create_returns_201_with_location`",
            async {
                let app = app(AppState::default());
                let body = part("m4-nut", "M4 hex nut", 120);
                let response = send(&app, post_json("/parts", &body)).await;
                assert_eq!(response.code(), 201);
                assert_eq!(response.header("location"), Some("/parts/m4-nut"));
                assert_eq!(response.json(), body);

                let again = send(&app, post_json("/parts", &body)).await;
                assert_eq!(again.code(), 409);
                assert_eq!(again.json()["error"], "already_exists");
            },
        )
        .await
    }

    #[tokio::test]
    async fn b_create_validates_values() {
        crate::support::deadline(120, "the test `b_create_validates_values`", async {
            let app = app(AppState::default());
            for body in [part("", "nameless", 1), part("x-1", "   ", 1)] {
                let response = send(&app, post_json("/parts", &body)).await;
                assert_eq!(response.code(), 422, "{body} must be rejected");
                assert_eq!(response.json()["error"], "invalid");
            }
            let listed = send(&app, get("/parts")).await;
            assert_eq!(
                listed.json(),
                json!([]),
                "a rejected part must not be stored"
            );
        })
        .await
    }

    #[tokio::test]
    async fn b_list_is_sorted_and_filtered() {
        crate::support::deadline(120, "the test `b_list_is_sorted_and_filtered`", async {
            let app = seeded().await;
            let all = send(&app, get("/parts")).await;
            assert_eq!(all.code(), 200);
            assert_eq!(
                all.json(),
                json!([
                    part("b-608", "608 bearing", 8),
                    part("fuse-2a", "2 A fuse", 0),
                    part("m4-nut", "M4 hex nut", 120),
                ])
            );
            let some = send(&app, get("/parts?min_qty=8")).await;
            let body = some.json();
            let skus: Vec<&str> = body
                .as_array()
                .unwrap()
                .iter()
                .map(|part| part["sku"].as_str().unwrap())
                .collect();
            assert_eq!(skus, ["b-608", "m4-nut"], "min_qty=8 keeps qty >= 8");
        })
        .await
    }

    #[tokio::test]
    async fn c_get_and_delete() {
        crate::support::deadline(120, "the test `c_get_and_delete`", async {
            let app = seeded().await;
            let found = send(&app, get("/parts/b-608")).await;
            assert_eq!(found.code(), 200);
            assert_eq!(found.json(), part("b-608", "608 bearing", 8));

            let missing = send(&app, get("/parts/nope")).await;
            assert_eq!(missing.code(), 404);
            assert_eq!(missing.json()["error"], "not_found");

            let deleted = send(&app, request(Method::DELETE, "/parts/b-608")).await;
            assert_eq!(deleted.code(), 204);
            assert!(deleted.body.is_empty(), "204 has no body");
            let again = send(&app, request(Method::DELETE, "/parts/b-608")).await;
            assert_eq!(again.code(), 404);
            assert_eq!(send(&app, get("/parts/b-608")).await.code(), 404);
        })
        .await
    }

    #[tokio::test]
    async fn c_adjust_changes_stock_and_refuses_to_go_negative() {
        crate::support::deadline(
            120,
            "the test `c_adjust_changes_stock_and_refuses_to_go_negative`",
            async {
                let app = seeded().await;
                let taken = send(
                    &app,
                    post_json("/parts/b-608/adjust", &json!({"delta": -3})),
                )
                .await;
                assert_eq!(taken.code(), 200);
                assert_eq!(taken.json(), part("b-608", "608 bearing", 5));

                let too_many = send(
                    &app,
                    post_json("/parts/b-608/adjust", &json!({"delta": -6})),
                )
                .await;
                assert_eq!(too_many.code(), 409);
                assert_eq!(too_many.json()["error"], "insufficient_stock");
                assert_eq!(
                    send(&app, get("/parts/b-608")).await.json()["qty"],
                    5,
                    "a refused adjustment must not change the stock"
                );

                let unknown =
                    send(&app, post_json("/parts/nope/adjust", &json!({"delta": 1}))).await;
                assert_eq!(unknown.code(), 404);

                let overflow = json!({"delta": i64::from(u32::MAX)});
                let response = send(&app, post_json("/parts/b-608/adjust", &overflow)).await;
                assert_eq!(response.code(), 409, "qty must stay inside u32");

                // `delta` is any i64 the client likes. Adding it to the stock
                // must not overflow: in a debug build that would panic inside
                // the handler while the lock is held.
                let extreme = json!({"delta": i64::MAX});
                let response = send(&app, post_json("/parts/b-608/adjust", &extreme)).await;
                assert_eq!(
                    response.code(),
                    409,
                    "a delta close to i64::MAX is refused like any other impossible change"
                );
                assert_eq!(
                    send(&app, get("/parts/b-608")).await.json()["qty"],
                    5,
                    "refused adjustments leave the stock alone"
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn c_state_is_shared_not_copied() {
        crate::support::deadline(120, "the test `c_state_is_shared_not_copied`", async {
            let state = AppState::default();
            let first = app(state.clone());
            let second = app(state.clone());
            let body = part("m4-nut", "M4 hex nut", 1);
            assert_eq!(send(&first, post_json("/parts", &body)).await.code(), 201);
            assert_eq!(
                send(&second, get("/parts/m4-nut")).await.code(),
                200,
                "both routers were built from clones of one AppState, so they must see the same map"
            );
            assert_eq!(
                state.parts.lock().unwrap().len(),
                1,
                "the handler must write to the map inside the AppState it was given"
            );
        })
        .await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn c_adjustments_are_atomic() {
        crate::support::deadline(120, "the test `c_adjustments_are_atomic`", async {
            // Statistical: in each of five rounds, eight tasks start at the same
            // moment on four threads and send 1000 requests each. If the read and
            // the write of `qty` use two lock acquisitions, some updates are lost
            // in almost every round. A pass makes a lost update unlikely, not
            // impossible.
            const ROUNDS: usize = 5;
            const TASKS: usize = 8;
            const REQUESTS: usize = 1000;
            let app = seeded().await;
            for round in 1..=ROUNDS {
                let start = Arc::new(tokio::sync::Barrier::new(TASKS));
                let mut tasks = tokio::task::JoinSet::new();
                for _ in 0..TASKS {
                    let (app, start) = (app.clone(), Arc::clone(&start));
                    tasks.spawn(async move {
                        start.wait().await;
                        for _ in 0..REQUESTS {
                            let request = post_json("/parts/fuse-2a/adjust", &json!({"delta": 1}));
                            assert_eq!(send(&app, request).await.code(), 200);
                        }
                    });
                }
                deadline(60, "one round of concurrent adjustments", tasks.join_all()).await;
                assert_eq!(
                    send(&app, get("/parts/fuse-2a")).await.json()["qty"],
                    round * TASKS * REQUESTS,
                    "every +1 must count: read and write qty under one lock acquisition"
                );
            }
        })
        .await
    }

    #[tokio::test]
    async fn d_serves_over_a_real_socket() {
        crate::support::deadline(120, "the test `d_serves_over_a_real_socket`", async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let state = AppState::default();
            let server = tokio::spawn(serve(listener, state.clone()));

            let create = json_request(
                Method::POST,
                &format!("http://{addr}/parts"),
                &part("m4-nut", "M4 hex nut", 3),
            );
            let created = deadline(10, "POST over TCP", http(create)).await.unwrap();
            assert_eq!(created.code(), 201);
            let fetched = deadline(
                10,
                "GET over TCP",
                http(get(&format!("http://{addr}/parts/m4-nut"))),
            )
            .await
            .unwrap();
            assert_eq!(fetched.json()["qty"], 3);
            assert_eq!(state.parts.lock().unwrap().len(), 1);
            server.abort();
        })
        .await
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// Each answer is the numeric status code that axum itself produces; none of
    /// these requests reaches a handler body.
    #[tokio::test]
    async fn p_predictions() {
        crate::support::deadline(120, "the test `p_predictions`", async {
            let app = seeded().await;
            let raw = |method: Method, uri: &str, content_type: Option<&str>, body: &str| {
                let mut builder = Request::builder().method(method).uri(uri);
                if let Some(content_type) = content_type {
                    builder = builder.header(header::CONTENT_TYPE, content_type);
                }
                builder.body(Body::from(body.to_owned())).unwrap()
            };
            let json_type = Some("application/json");

            // P1. No route matches the path.
            let p1: u16 = 404;
            let response = send(&app, get("/bins")).await;
            assert_eq!(p1, response.code(), "P1: unknown path");

            // P2. The path matches a route, but the route has no handler for PUT.
            let p2: u16 = 405;
            let response = send(&app, raw(Method::PUT, "/parts", json_type, "{}")).await;
            assert_eq!(p2, response.code(), "P2: known path, unsupported method");

            // P3. The body is not valid JSON at all.
            let p3: u16 = 400;
            let response = send(&app, raw(Method::POST, "/parts", json_type, "{sku: m4")).await;
            assert_eq!(p3, response.code(), "P3: JSON syntax error");

            // P4. The body is valid JSON, but `qty` is a string, so it does not fit `Part`.
            let p4: u16 = 422;
            let body = r#"{"sku": "a", "name": "b", "qty": "many"}"#;
            let response = send(&app, raw(Method::POST, "/parts", json_type, body)).await;
            assert_eq!(p4, response.code(), "P4: JSON of the wrong shape");

            // P5. The body is perfect JSON for `Part`, but there is no content-type header.
            let p5: u16 = 415;
            let body = r#"{"sku": "a", "name": "b", "qty": 1}"#;
            let response = send(&app, raw(Method::POST, "/parts", None, body)).await;
            assert_eq!(p5, response.code(), "P5: missing content type");

            // P6. `min_qty` must be a u32, and the query string says `-1`.
            let p6: u16 = 400;
            let response = send(&app, get("/parts?min_qty=-1")).await;
            assert_eq!(p6, response.code(), "P6: query string of the wrong shape");
        })
        .await
    }
}
