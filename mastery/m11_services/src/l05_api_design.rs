//! LESSON 05 — Validation and API design: boundaries, pagination, idempotency, versions
//! Read first: CONCEPTS.md, "Lesson 05". Start after: lesson 01 and ecosystem E02.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml l05_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a client sends `POST /tickets`, the
//! connection drops before the response arrives, and the client sends the request
//! again. How many tickets exist now? What must the API offer so that the answer
//! is always one?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! From this lesson on, the scaffolding is thinner: you get the contract, the
//! public types and one router function, and you design the state and handlers.
//! EXPLAIN afterward: Where exactly is the boundary in your code, that is, the last
//! line that can see an unvalidated value? Why does keyset pagination survive a
//! delete that offset pagination does not? Why must a reused idempotency key with a
//! different body be an error instead of a replay?
//! JUDGMENT: Validate by converting to a type that cannot hold a bad value; then
//! the rest of the program needs no checks. Crates such as `validator` and `garde`
//! generate field checks from attributes and are worth it when you have dozens of
//! request types; with a few, plain functions like the one here are clearer. Put a
//! version in the path when you make a breaking change, and avoid breaking changes
//! by only adding optional fields. One error shape for the whole API matters more
//! than which shape you choose; RFC 9457 ("problem details") is a standard one.
//!
//! The service stores support tickets in memory. The HTTP contract:
//!
//! | Method and path                   | Success                                   |
//! | --------------------------------- | ----------------------------------------- |
//! | `POST /v1/tickets`                | 201, ticket (v1 form), `Location` header  |
//! | `GET /v1/tickets?limit=&after=`   | 200, `{"items": [...], "next": ...}`      |
//! | `GET /v1/tickets/{id}`            | 200, ticket (v1 form)                     |
//! | `DELETE /v1/tickets/{id}`         | 204                                       |
//! | `GET /v2/tickets/{id}`            | 200, ticket (v2 form)                     |
//!
//! Ticket ids start at 1 and count up. A ticket in v1 form is
//! `{"id": 1, "title": "...", "priority": 3, "tags": ["..."]}`. In v2 form the
//! priority is an object: `"priority": {"level": 3, "label": "normal"}` with the
//! labels `lowest`, `low`, `normal`, `high`, `urgent` for levels 1 to 5.
//!
//! **Every** response with a 4xx status, including the ones axum would write for
//! you, has this JSON body and nothing else:
//!
//! ```json
//! {"error": {"code": "...", "message": "...", "details": [{"field": "...", "problem": "..."}]}}
//! ```
//!
//! `details` is always an array and is empty unless stated. The codes:
//!
//! | Status | Code                     | When                                                  |
//! | ------ | ------------------------ | ----------------------------------------------------- |
//! | 400    | `malformed_json`         | the body is not JSON                                  |
//! | 400    | `invalid_query`          | bad `limit` or `after`; one detail names the field    |
//! | 400    | `invalid_header`         | bad `Idempotency-Key`                                 |
//! | 404    | `not_found`              | unknown path, unknown id, or an id that is not a number |
//! | 405    | `method_not_allowed`     | known path, wrong method                              |
//! | 413    | `payload_too_large`      | body over 16 KiB                                      |
//! | 415    | `unsupported_media_type` | body without `content-type: application/json`         |
//! | 422    | `invalid_body`           | JSON of the wrong shape (missing field, wrong type)   |
//! | 422    | `validation_failed`      | right shape, bad values; one detail per problem       |
//! | 422    | `idempotency_key_reused` | the key was used before with a different body         |

use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, FromRequest, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

/// Largest accepted request body.
pub const MAX_BODY_BYTES: usize = 16 * 1024;
/// Page size when the client gives none, and the largest it may ask for.
pub const DEFAULT_LIMIT: usize = 20;
pub const MAX_LIMIT: usize = 100;

/// One problem with one field of the input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldProblem {
    pub field: &'static str,
    pub problem: String,
}

/// The body of `POST /v1/tickets` exactly as the client sent it: the right JSON
/// shape, but no value has been checked yet.
#[derive(Debug, Clone, Deserialize)]
pub struct TicketInput {
    pub title: String,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A ticket request that has passed validation. The fields are private and there
/// is no public constructor except [`validate`], so holding a `NewTicket` is proof
/// that the rules hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTicket {
    title: String,
    priority: u8,
    tags: Vec<String>,
}

impl NewTicket {
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn priority(&self) -> u8 {
        self.priority
    }
    pub fn tags(&self) -> &[String] {
        &self.tags
    }
}

/// A. Turn raw input into a [`NewTicket`], or report **every** problem (not only
/// the first), in the order title, priority, tags.
///
/// - `title`: surrounding whitespace is removed; then it must have 1 to 80
///   characters (`char`s, not bytes). Field name `"title"`.
/// - `priority`: defaults to 3; must be 1 to 5. Field name `"priority"`.
/// - `tags`: each tag is trimmed and lower-cased; then it must have 1 to 20
///   characters, all of them ASCII letters, digits or `-`. Duplicates are removed
///   and the result is sorted. At most 5 tags remain after that. Field name
///   `"tags"`; report at most one problem for this field.
///
/// The `problem` texts are yours to write; they must not be empty.
pub fn validate(input: TicketInput) -> Result<NewTicket, Vec<FieldProblem>> {
    todo!("A: normalise, check every field, collect all problems")
}

/// The one error type of this API. Supplied; build values of it however you like.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub details: Vec<FieldProblem>,
}

/// B. Write the error as the JSON body from the header, with its status code.
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        todo!("B: status and the standard error body")
    }
}

/// B. Translate axum's JSON rejections into this API's codes: a syntax error is
/// `malformed_json` (400), a body that does not fit the target type is
/// `invalid_body` (422), a missing JSON content type is `unsupported_media_type`
/// (415), and a body over the limit is `payload_too_large` (413).
/// `JsonRejection` is `#[non_exhaustive]`; for any other case keep the status that
/// the rejection reports and choose a code yourself.
impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> ApiError {
        todo!("B: map each rejection to a status and a code")
    }
}

/// B. A JSON extractor that fails with [`ApiError`] instead of axum's plain-text
/// rejection. Handlers use `ApiJson<T>` wherever they would use `Json<T>`.
#[derive(Debug, Clone)]
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<ApiJson<T>, ApiError> {
        todo!("B: delegate to Json<T> and convert the rejection")
    }
}

/// C, D, E. Build the whole service from the contract in the header, with fresh,
/// empty state. The tests only use HTTP, so the state type, the handlers and the
/// helper types are yours to design. Requirements beyond the tables:
///
/// - **C. Pagination.** Tickets are listed in id order. `limit` defaults to
///   [`DEFAULT_LIMIT`] and must be 1 to [`MAX_LIMIT`]. `next` is `null` on the last
///   page; otherwise it is a string that the client sends back unchanged as
///   `after` to get the following page. A page boundary must not move when
///   earlier tickets are deleted: no ticket may be skipped or shown twice.
///   `after` is client input like any other: text that is not one of your
///   cursors is `invalid_query`, and no value may make the handler panic.
/// - **D. Idempotency.** `POST /v1/tickets` accepts an optional `Idempotency-Key`
///   header of 1 to 64 characters. The first request with a key creates the
///   ticket. A later request with the same key and an equal validated body creates
///   nothing and returns the first response again (201, same body, same
///   `Location`) plus the header `idempotent-replay: true`. The same key with a
///   different body is `idempotency_key_reused`. Requests without the header are
///   never deduplicated. Concurrent requests with one key create one ticket.
/// - **E. Versions.** Both versions read the same stored tickets; only the
///   representation differs.
pub fn app() -> Router {
    todo!("C, D, E: state, routes, fallbacks, body limit")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{
        TestResponse, deadline, get as http_get, post_json, request, send, with_header,
    };
    use axum::body::Body;
    use axum::http::Method;
    use serde_json::{Value, json};

    fn input(title: &str, priority: Option<i64>, tags: &[&str]) -> TicketInput {
        TicketInput {
            title: title.to_owned(),
            priority,
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        }
    }

    fn fields(result: Result<NewTicket, Vec<FieldProblem>>) -> Vec<&'static str> {
        let problems = result.expect_err("expected validation problems");
        assert!(
            problems.iter().all(|p| !p.problem.trim().is_empty()),
            "every problem needs a text"
        );
        problems.iter().map(|p| p.field).collect()
    }

    /// Asserts the standard error shape and returns the `error` object.
    #[track_caller]
    fn error_of(response: &TestResponse, status: u16, code: &str) -> Value {
        assert_eq!(response.code(), status, "body: {}", response.text());
        assert_eq!(
            response.header("content-type"),
            Some("application/json"),
            "error bodies are JSON; body: {}",
            response.text()
        );
        let body = response.json();
        let object = body.as_object().expect("the body is a JSON object");
        assert_eq!(object.len(), 1, "the body has only the key `error`: {body}");
        let error = &body["error"];
        assert_eq!(error["code"], code, "{body}");
        assert!(
            error["message"].as_str().is_some_and(|m| !m.is_empty()),
            "`message` is a non-empty string: {body}"
        );
        assert!(
            error["details"].is_array(),
            "`details` is always an array: {body}"
        );
        error.clone()
    }

    async fn create(app: &Router, title: &str) -> Value {
        let response = send(app, post_json("/v1/tickets", &json!({ "title": title }))).await;
        assert_eq!(response.code(), 201, "{}", response.text());
        response.json()
    }

    async fn page(app: &Router, uri: &str) -> (Vec<u64>, Value) {
        let response = send(app, http_get(uri)).await;
        assert_eq!(response.code(), 200, "{uri}: {}", response.text());
        let body = response.json();
        let ids = body["items"]
            .as_array()
            .expect("`items` is an array")
            .iter()
            .map(|item| item["id"].as_u64().expect("numeric id"))
            .collect();
        (ids, body["next"].clone())
    }

    #[test]
    fn a_valid_input_is_normalised() {
        let ticket = validate(input(
            "  Printer on fire ",
            None,
            &[" Hardware", "urgent", "hardware", "floor-3"],
        ))
        .expect("valid input");
        assert_eq!(ticket.title(), "Printer on fire");
        assert_eq!(ticket.priority(), 3, "priority defaults to 3");
        assert_eq!(
            ticket.tags(),
            ["floor-3", "hardware", "urgent"],
            "tags are trimmed, lower-cased, deduplicated and sorted"
        );
        let eighty = "é".repeat(80);
        assert_eq!(
            validate(input(&eighty, Some(5), &[])).unwrap().title(),
            eighty,
            "the limit is 80 characters, not 80 bytes"
        );
    }

    #[test]
    fn a_every_problem_is_reported() {
        assert_eq!(fields(validate(input("   ", None, &[]))), ["title"]);
        assert_eq!(
            fields(validate(input(&"x".repeat(81), None, &[]))),
            ["title"]
        );
        assert_eq!(fields(validate(input("ok", Some(0), &[]))), ["priority"]);
        assert_eq!(fields(validate(input("ok", Some(6), &[]))), ["priority"]);
        assert_eq!(fields(validate(input("ok", Some(-1), &[]))), ["priority"]);
        assert_eq!(
            fields(validate(input("ok", None, &["has space"]))),
            ["tags"]
        );
        assert_eq!(fields(validate(input("ok", None, &["  "]))), ["tags"]);
        assert_eq!(
            fields(validate(input("ok", None, &[&"t".repeat(21)]))),
            ["tags"]
        );
        assert_eq!(
            fields(validate(input("ok", None, &["a", "b", "c", "d", "e", "f"]))),
            ["tags"]
        );
        assert!(
            validate(input("ok", None, &["a", "b", "c", "d", "e", "E", "a "])).is_ok(),
            "the limit of 5 applies after duplicates are removed"
        );
        assert_eq!(
            fields(validate(input("", Some(9), &["BAD TAG", "ünïcode"]))),
            ["title", "priority", "tags"],
            "report all problems at once, in field order, one per field"
        );
    }

    #[tokio::test]
    async fn b_api_error_has_the_standard_shape() {
        crate::support::deadline(
            120,
            "the test `b_api_error_has_the_standard_shape`",
            async {
                let error = ApiError {
                    status: StatusCode::IM_A_TEAPOT,
                    code: "teapot",
                    message: "short and stout".to_owned(),
                    details: vec![FieldProblem {
                        field: "spout",
                        problem: "is blocked".to_owned(),
                    }],
                };
                let response = error.into_response();
                assert_eq!(response.status(), StatusCode::IM_A_TEAPOT);
                let bytes = axum::body::to_bytes(response.into_body(), 4096)
                    .await
                    .unwrap();
                let body: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(
                    body,
                    json!({"error": {
                        "code": "teapot",
                        "message": "short and stout",
                        "details": [{"field": "spout", "problem": "is blocked"}],
                    }})
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn b_bad_bodies_use_the_standard_error() {
        crate::support::deadline(
            120,
            "the test `b_bad_bodies_use_the_standard_error`",
            async {
                let app = app();
                let raw = |content_type: Option<&str>, body: Vec<u8>| {
                    let mut builder = axum::http::Request::builder()
                        .method(Method::POST)
                        .uri("/v1/tickets");
                    if let Some(content_type) = content_type {
                        builder = builder.header(header::CONTENT_TYPE, content_type);
                    }
                    builder.body(Body::from(body)).unwrap()
                };
                let json_type = Some("application/json");

                let response = send(&app, raw(json_type, b"{\"title\": ".to_vec())).await;
                error_of(&response, 400, "malformed_json");

                let response = send(&app, raw(json_type, b"{\"priority\": 2}".to_vec())).await;
                error_of(&response, 422, "invalid_body");
                let response = send(&app, raw(json_type, b"{\"title\": 7}".to_vec())).await;
                error_of(&response, 422, "invalid_body");

                let response = send(&app, raw(None, b"{\"title\": \"x\"}".to_vec())).await;
                error_of(&response, 415, "unsupported_media_type");

                let mut big = b"{\"title\": \"".to_vec();
                big.resize(MAX_BODY_BYTES + 1, b'x');
                let response = send(&app, raw(json_type, big)).await;
                error_of(&response, 413, "payload_too_large");

                let (ids, _) = page(&app, "/v1/tickets").await;
                assert!(ids.is_empty(), "no rejected request may create a ticket");
            },
        )
        .await
    }

    #[tokio::test]
    async fn b_validation_failures_list_every_field() {
        crate::support::deadline(
            120,
            "the test `b_validation_failures_list_every_field`",
            async {
                let app = app();
                let body = json!({ "title": " ", "priority": 0, "tags": ["no good"] });
                let response = send(&app, post_json("/v1/tickets", &body)).await;
                let error = error_of(&response, 422, "validation_failed");
                let named: Vec<&str> = error["details"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|detail| detail["field"].as_str().unwrap())
                    .collect();
                assert_eq!(named, ["title", "priority", "tags"]);
            },
        )
        .await
    }

    #[tokio::test]
    async fn b_unknown_paths_and_methods_use_the_standard_error() {
        crate::support::deadline(
            120,
            "the test `b_unknown_paths_and_methods_use_the_standard_error`",
            async {
                let app = app();
                create(&app, "one").await;
                for uri in ["/", "/v1", "/v1/nothing", "/v3/tickets/1", "/tickets"] {
                    error_of(&send(&app, http_get(uri)).await, 404, "not_found");
                }
                for uri in [
                    "/v1/tickets/2",
                    "/v1/tickets/abc",
                    "/v1/tickets/-1",
                    "/v2/tickets/9",
                ] {
                    error_of(&send(&app, http_get(uri)).await, 404, "not_found");
                }
                error_of(
                    &send(&app, request(Method::DELETE, "/v1/tickets/2")).await,
                    404,
                    "not_found",
                );
                error_of(
                    &send(&app, request(Method::PUT, "/v1/tickets")).await,
                    405,
                    "method_not_allowed",
                );
                error_of(
                    &send(&app, request(Method::DELETE, "/v2/tickets/1")).await,
                    405,
                    "method_not_allowed",
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn c_create_then_get_and_delete() {
        crate::support::deadline(120, "the test `c_create_then_get_and_delete`", async {
            let app = app();
            let body = json!({ "title": " Door squeaks ", "priority": 2, "tags": ["Facilities"] });
            let response = send(&app, post_json("/v1/tickets", &body)).await;
            assert_eq!(response.code(), 201, "{}", response.text());
            assert_eq!(response.header("location"), Some("/v1/tickets/1"));
            let expected = json!({
                "id": 1, "title": "Door squeaks", "priority": 2, "tags": ["facilities"]
            });
            assert_eq!(response.json(), expected);
            assert_eq!(send(&app, http_get("/v1/tickets/1")).await.json(), expected);

            let deleted = send(&app, request(Method::DELETE, "/v1/tickets/1")).await;
            assert_eq!(deleted.code(), 204);
            error_of(
                &send(&app, http_get("/v1/tickets/1")).await,
                404,
                "not_found",
            );
        })
        .await
    }

    #[tokio::test]
    async fn c_pages_follow_the_cursor() {
        crate::support::deadline(120, "the test `c_pages_follow_the_cursor`", async {
            let app = app();
            for number in 1..=7 {
                create(&app, &format!("ticket {number}")).await;
            }
            let (ids, next) = page(&app, "/v1/tickets?limit=3").await;
            assert_eq!(ids, [1, 2, 3]);
            let cursor = next
                .as_str()
                .expect("`next` is a string while pages remain");
            let (ids, next) = page(&app, &format!("/v1/tickets?limit=3&after={cursor}")).await;
            assert_eq!(ids, [4, 5, 6]);
            let cursor = next.as_str().expect("one more page");
            let (ids, next) = page(&app, &format!("/v1/tickets?limit=3&after={cursor}")).await;
            assert_eq!(ids, [7]);
            assert_eq!(next, Value::Null, "`next` is null on the last page");

            let (ids, next) = page(&app, "/v1/tickets?limit=7").await;
            assert_eq!(ids.len(), 7);
            assert_eq!(
                next,
                Value::Null,
                "a full last page has no next page: do not promise a page that is empty"
            );
            let (ids, next) = page(&app, "/v1/tickets").await;
            assert_eq!((ids.len(), next), (7, Value::Null), "default limit is 20");
        })
        .await
    }

    #[tokio::test]
    async fn c_pages_do_not_shift_when_earlier_tickets_are_deleted() {
        crate::support::deadline(
            120,
            "the test `c_pages_do_not_shift_when_earlier_tickets_are_deleted`",
            async {
                let app = app();
                for number in 1..=7 {
                    create(&app, &format!("ticket {number}")).await;
                }
                let (ids, next) = page(&app, "/v1/tickets?limit=3").await;
                assert_eq!(ids, [1, 2, 3]);
                let cursor = next.as_str().unwrap().to_owned();

                // Between two page requests, someone deletes a ticket the client has seen.
                let deleted = send(&app, request(Method::DELETE, "/v1/tickets/2")).await;
                assert_eq!(deleted.code(), 204);

                let (ids, _) = page(&app, &format!("/v1/tickets?limit=3&after={cursor}")).await;
                assert_eq!(
                    ids,
                    [4, 5, 6],
                    "ticket 4 was skipped: the cursor must name a position in the data \
             (\"after id 3\"), not a row count (\"skip 3 rows\")"
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn c_bad_page_parameters_are_rejected() {
        crate::support::deadline(
            120,
            "the test `c_bad_page_parameters_are_rejected`",
            async {
                let app = app();
                for uri in [
                    "/v1/tickets?limit=0",
                    "/v1/tickets?limit=101",
                    "/v1/tickets?limit=ten",
                    "/v1/tickets?limit=-5",
                ] {
                    let error = error_of(&send(&app, http_get(uri)).await, 400, "invalid_query");
                    assert_eq!(error["details"][0]["field"], "limit", "{uri}");
                }
                let response = send(&app, http_get("/v1/tickets?after=%00garbage")).await;
                let error = error_of(&response, 400, "invalid_query");
                assert_eq!(error["details"][0]["field"], "after");
                // Whatever your cursor looks like inside, a client can send any
                // text as `after`. A cursor that points behind every possible
                // ticket is either refused or followed by nothing; arithmetic on
                // it must not overflow and panic while the lock is held.
                create(&app, "the only ticket").await;
                for uri in [
                    "/v1/tickets?after=18446744073709551615",
                    "/v1/tickets?after=9223372036854775807",
                ] {
                    let response = send(&app, http_get(uri)).await;
                    match response.code() {
                        200 => assert_eq!(
                            response.json()["items"],
                            json!([]),
                            "{uri}: no ticket comes after the largest possible id"
                        ),
                        400 => {
                            error_of(&response, 400, "invalid_query");
                        }
                        other => panic!("{uri}: expected 200 or 400, got {other}"),
                    }
                }
                let (ids, _) = page(&app, "/v1/tickets").await;
                assert_eq!(ids, [1], "the service still answers after hostile cursors");
                let (ids, _) = page(&app, "/v1/tickets?limit=100").await;
                assert_eq!(ids, [1], "limit=100 is allowed");
            },
        )
        .await
    }

    fn keyed(key: &str, body: &Value) -> axum::http::Request<Body> {
        with_header(post_json("/v1/tickets", body), "idempotency-key", key)
    }

    #[tokio::test]
    async fn d_a_repeated_key_replays_the_first_response() {
        crate::support::deadline(
            120,
            "the test `d_a_repeated_key_replays_the_first_response`",
            async {
                let app = app();
                let body = json!({ "title": "Coffee machine leaks", "tags": ["kitchen"] });
                let first = send(&app, keyed("k-1", &body)).await;
                assert_eq!(first.code(), 201);
                assert_eq!(first.header("idempotent-replay"), None);

                // The same request again, as a client would send it after a lost response.
                // Formatting differences in the JSON text do not make it a different request.
                let respelled = json!({ "tags": [" Kitchen "], "title": "Coffee machine leaks  " });
                for retry in [&body, &respelled] {
                    let again = send(&app, keyed("k-1", retry)).await;
                    assert_eq!(again.code(), 201, "{}", again.text());
                    assert_eq!(again.body, first.body, "the replay has the first body");
                    assert_eq!(again.header("location"), first.header("location"));
                    assert_eq!(again.header("idempotent-replay"), Some("true"));
                }
                let (ids, _) = page(&app, "/v1/tickets").await;
                assert_eq!(ids, [1], "three requests with one key create one ticket");

                let other_key = send(&app, keyed("k-2", &body)).await;
                assert_eq!(other_key.json()["id"], 2, "a new key is a new request");
                let no_key = send(&app, post_json("/v1/tickets", &body)).await;
                assert_eq!(
                    no_key.json()["id"],
                    3,
                    "without a key nothing is deduplicated"
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn d_a_key_with_a_different_body_is_an_error() {
        crate::support::deadline(
            120,
            "the test `d_a_key_with_a_different_body_is_an_error`",
            async {
                let app = app();
                let first = send(&app, keyed("k-1", &json!({ "title": "A" }))).await;
                assert_eq!(first.code(), 201);
                let conflict = send(&app, keyed("k-1", &json!({ "title": "B" }))).await;
                error_of(&conflict, 422, "idempotency_key_reused");
                let (ids, _) = page(&app, "/v1/tickets").await;
                assert_eq!(ids, [1]);

                let invalid = send(&app, keyed("k-9", &json!({ "title": "" }))).await;
                error_of(&invalid, 422, "validation_failed");
                let retry_fixed = send(&app, keyed("k-9", &json!({ "title": "fixed" }))).await;
                assert_eq!(
                    retry_fixed.code(),
                    201,
                    "a request that failed validation created nothing, so its key is still free"
                );

                let too_long = "k".repeat(65);
                for key in ["", too_long.as_str()] {
                    let response = send(&app, keyed(key, &json!({ "title": "A" }))).await;
                    error_of(&response, 400, "invalid_header");
                }
            },
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn d_concurrent_retries_create_one_ticket() {
        crate::support::deadline(120, "the test `d_concurrent_retries_create_one_ticket`", async {
        // The race window between "is the key known?" and "store the key" can
        // be a few nanoseconds wide, so this test is statistical: it tries
        // 5000 times and lets all requests of a round start at the same moment.
        // A failure proves the race; a pass makes it unlikely, not impossible.
        for round in 0..5000 {
            let app = app();
            let start = Arc::new(tokio::sync::Barrier::new(8));
            let mut tasks = tokio::task::JoinSet::new();
            for _ in 0..8 {
                let (app, start) = (app.clone(), Arc::clone(&start));
                tasks.spawn(async move {
                    start.wait().await;
                    send(&app, keyed("same-key", &json!({ "title": "Only once" }))).await
                });
            }
            let responses = deadline(30, "8 concurrent retries", tasks.join_all()).await;
            assert!(responses.iter().all(|r| r.code() == 201));
            let fresh = responses
                .iter()
                .filter(|r| r.header("idempotent-replay").is_none())
                .count();
            assert_eq!(fresh, 1, "round {round}: exactly one request is the original");
            let (ids, _) = page(&app, "/v1/tickets").await;
            assert_eq!(
                ids,
                [1],
                "round {round}: check the key and create the ticket under one lock acquisition"
            );
        }
        })
        .await
    }

    #[tokio::test]
    async fn e_versions_differ_only_in_representation() {
        crate::support::deadline(
            120,
            "the test `e_versions_differ_only_in_representation`",
            async {
                let app = app();
                let body = json!({ "title": "Server room too warm", "priority": 5 });
                assert_eq!(
                    send(&app, post_json("/v1/tickets", &body)).await.code(),
                    201
                );
                create(&app, "Default priority").await;

                let v1 = send(&app, http_get("/v1/tickets/1")).await.json();
                assert_eq!(v1["priority"], 5);
                let v2 = send(&app, http_get("/v2/tickets/1")).await.json();
                assert_eq!(
                    v2,
                    json!({
                        "id": 1,
                        "title": "Server room too warm",
                        "priority": { "level": 5, "label": "urgent" },
                        "tags": [],
                    })
                );
                let v2 = send(&app, http_get("/v2/tickets/2")).await.json();
                assert_eq!(v2["priority"], json!({ "level": 3, "label": "normal" }));

                let labels: Vec<String> = {
                    let mut labels = Vec::new();
                    for level in 1..=5 {
                        let body = json!({ "title": "t", "priority": level });
                        let id =
                            send(&app, post_json("/v1/tickets", &body)).await.json()["id"].clone();
                        let v2 = send(&app, http_get(&format!("/v2/tickets/{id}")))
                            .await
                            .json();
                        labels.push(v2["priority"]["label"].as_str().unwrap().to_owned());
                    }
                    labels
                };
                assert_eq!(labels, ["lowest", "low", "normal", "high", "urgent"]);
            },
        )
        .await
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// These use a tiny router of their own, so they describe axum and serde, not
    /// your code.
    #[tokio::test]
    async fn p_predictions() {
        crate::support::deadline(120, "the test `p_predictions`", async {
            #[derive(Deserialize)]
            struct Probe {
                #[allow(dead_code)]
                name: String,
                #[serde(default)]
                #[allow(dead_code)]
                count: Option<u8>,
            }
            let probe = Router::new()
                .route("/body", post(|Json(_): Json<Probe>| async { "ok" }))
                .route("/n/{n}", get(|Path(_): Path<u64>| async { "ok" }));
            let post_raw = |content_type: &str, body: &str| {
                axum::http::Request::builder()
                    .method(Method::POST)
                    .uri("/body")
                    .header(header::CONTENT_TYPE, content_type)
                    .body(Body::from(body.to_owned()))
                    .unwrap()
            };
            let json_type = "application/json";

            // P1. The body has a field that `Probe` does not know. Status code?
            let p1: u16 = crate::guess();
            let response = send(
                &probe,
                post_raw(json_type, r#"{"name": "a", "colour": "red"}"#),
            )
            .await;
            assert_eq!(
                p1,
                response.code(),
                "P1: unknown fields are ignored by default"
            );

            // P2. `count` is an `Option<u8>` and the body says 300. Status code?
            let p2: u16 = crate::guess();
            let response = send(
                &probe,
                post_raw(json_type, r#"{"name": "a", "count": 300}"#),
            )
            .await;
            assert_eq!(
                p2,
                response.code(),
                "P2: a number that does not fit the type"
            );

            // P3. `count` is explicitly `null`. Status code?
            let p3: u16 = crate::guess();
            let response = send(
                &probe,
                post_raw(json_type, r#"{"name": "a", "count": null}"#),
            )
            .await;
            assert_eq!(p3, response.code(), "P3: null for an Option field");

            // P4. The content type carries a charset parameter. Status code?
            let p4: u16 = crate::guess();
            let response = send(
                &probe,
                post_raw("application/json; charset=utf-8", r#"{"name": "a"}"#),
            )
            .await;
            assert_eq!(p4, response.code(), "P4: content type with a parameter");

            // P5. `Path<u64>` and the path segment is "abc". Status code?
            let p5: u16 = crate::guess();
            let response = send(&probe, http_get("/n/abc")).await;
            assert_eq!(p5, response.code(), "P5: axum's own path rejection");
        })
        .await
    }
}
