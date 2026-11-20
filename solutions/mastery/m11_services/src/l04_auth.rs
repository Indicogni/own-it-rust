//! LESSON 04 — Authentication: slow password hashes, signed tokens, an auth extractor
//! Read first: CONCEPTS.md, "Lesson 04". Start after: lessons 01 and 02.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml l04_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a login handler looks the user up
//! and returns 401 at once when the name is unknown, and only hashes when the name
//! exists. Both cases send the same body. What can an attacker still measure?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! The tests hash with very small Argon2 parameters so that they run in
//! milliseconds. Those parameters are useless for real passwords; CONCEPTS.md
//! explains how to choose real ones.
//! EXPLAIN afterward: Why must a password hash be slow and salted while the token
//! MAC must be fast? Why does `verify_token` check the MAC before it parses the
//! payload? What exactly does the decoy hash protect, and what does it not protect?
//! JUDGMENT: Use Argon2id through the `argon2` crate (pure Rust, RustCrypto) unless
//! a compliance rule names another algorithm; never design your own password
//! scheme. The token here is a minimal HMAC-signed value so that you can see every
//! part. For real systems prefer an established format and library (for example
//! `jsonwebtoken` with one fixed algorithm, or opaque random session ids stored in
//! your database). Signed tokens need no lookup but cannot be revoked before they
//! expire; server-side sessions cost one lookup and can be revoked at once. This
//! lesson does not cover TLS, rate limiting, account lockout or password reset.
//! A real login endpoint needs all four.

use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use argon2::{Algorithm, Argon2, Params, Version};
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// A password hashing failure. The text is for logs only.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("password hashing failed: {0}")]
pub struct HashError(pub String);

/// Argon2 parameters for the tests of this lesson: 64 KiB of memory, one pass,
/// one lane. Fast and far too weak for production.
pub fn test_params() -> Params {
    Params::new(64, 1, 1, None).expect("valid test parameters")
}

/// A. Hash `password` with Argon2id (version 0x13) and the given parameters, with a
/// fresh random salt, and return the PHC string (`$argon2id$v=19$m=...$salt$hash`).
/// The string contains everything needed to verify later, so it is the only thing
/// you store.
pub fn hash_password(params: &Params, password: &str) -> Result<String, HashError> {
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params.clone());
    // `hash_password` asks the operating system for a random salt.
    let hash = argon2
        .hash_password(password.as_bytes())
        .map_err(|err| HashError(err.to_string()))?;
    Ok(hash.to_string())
}

/// A. Check `password` against a stored PHC string. `Ok(false)` means "wrong
/// password"; `Err` means the stored string is not a usable hash, which is a
/// server-side problem. The parameters and the salt come from the stored string.
pub fn verify_password(stored_phc: &str, password: &str) -> Result<bool, HashError> {
    let parsed = PasswordHash::new(stored_phc).map_err(|err| HashError(err.to_string()))?;
    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
        Err(other) => Err(HashError(other.to_string())),
    }
}

/// The secret key that signs tokens. Its `Debug` output never shows the bytes.
#[derive(Clone)]
pub struct TokenKey(Vec<u8>);

impl TokenKey {
    pub fn new(bytes: &[u8]) -> TokenKey {
        TokenKey(bytes.to_vec())
    }
}

impl std::fmt::Debug for TokenKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TokenKey(<redacted>)")
    }
}

/// What a token says: who, and until when (seconds since the Unix epoch).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    pub user: String,
    pub expires_at: u64,
}

/// Why a token was not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    #[error("the token is not in the expected format")]
    Malformed,
    #[error("the token signature does not match")]
    BadSignature,
    #[error("the token has expired")]
    Expired,
}

/// Lower-case hexadecimal text for `bytes`. Supplied.
pub fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// The bytes for lower-case hexadecimal `text`, or `None` if it is not valid.
/// Supplied.
pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2)
        || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

/// B. Make a token: `v1.<payload>.<mac>` where
///
/// - `<payload>` is the hex text of the claims as JSON (`serde_json::to_vec`),
/// - `<mac>` is the hex text of HMAC-SHA-256, keyed with `key`, over the ASCII
///   bytes of `v1.<payload>` (everything before the second dot).
pub fn sign_token(key: &TokenKey, claims: &Claims) -> String {
    let payload = serde_json::to_vec(claims).expect("claims serialise to JSON");
    let signed_part = format!("v1.{}", to_hex(&payload));
    let mut mac = Hmac::<Sha256>::new_from_slice(&key.0).expect("HMAC accepts any key length");
    mac.update(signed_part.as_bytes());
    let tag = mac.finalize().into_bytes();
    format!("{signed_part}.{}", to_hex(&tag))
}

/// B. Check a token and return its claims. `now` is the current time in seconds
/// since the Unix epoch.
///
/// 1. The token must have exactly three dot-separated parts, the first must be
///    `v1`, and the other two must be hex text. Otherwise: `Malformed`.
/// 2. The MAC must match, compared in constant time. The `hmac` crate has a
///    method for that; use it, and do not compare bytes or hex text with `==`.
///    (Timing cannot be tested reliably, so one test reads your source text
///    for this.) Otherwise: `BadSignature`.
/// 3. Only then parse the payload. Not valid claims JSON: `Malformed`.
/// 4. A token is valid strictly before `expires_at`. Otherwise: `Expired`.
///
/// The order matters: never interpret data that has not been authenticated.
pub fn verify_token(key: &TokenKey, token: &str, now: u64) -> Result<Claims, TokenError> {
    let mut parts = token.split('.');
    let (Some("v1"), Some(payload_hex), Some(mac_hex), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(TokenError::Malformed);
    };
    let payload = from_hex(payload_hex).ok_or(TokenError::Malformed)?;
    let tag = from_hex(mac_hex).ok_or(TokenError::Malformed)?;

    let mut mac = Hmac::<Sha256>::new_from_slice(&key.0).expect("HMAC accepts any key length");
    mac.update(b"v1.");
    mac.update(payload_hex.as_bytes());
    // `verify_slice` compares in constant time and rejects a tag of the wrong length.
    mac.verify_slice(&tag)
        .map_err(|_| TokenError::BadSignature)?;

    let claims: Claims = serde_json::from_slice(&payload).map_err(|_| TokenError::Malformed)?;
    if now >= claims.expires_at {
        return Err(TokenError::Expired);
    }
    Ok(claims)
}

/// How the login handler checks a password. The service uses [`Argon2Check`];
/// tests substitute a version that also counts calls.
pub trait PasswordCheck: Send + Sync + 'static {
    /// `true` if `password` matches `stored_phc`. A broken stored hash counts as
    /// "no match".
    fn matches(&self, stored_phc: &str, password: &str) -> bool;
}

/// The real check: [`verify_password`] from task A.
#[derive(Debug, Clone, Copy, Default)]
pub struct Argon2Check;

impl PasswordCheck for Argon2Check {
    fn matches(&self, stored_phc: &str, password: &str) -> bool {
        verify_password(stored_phc, password).unwrap_or(false)
    }
}

/// A clock that tests can set. Supplied.
#[derive(Debug, Clone)]
pub struct Clock(Arc<AtomicU64>);

impl Clock {
    /// A clock that shows `now` until someone calls [`Clock::set`].
    pub fn fixed(now: u64) -> Clock {
        Clock(Arc::new(AtomicU64::new(now)))
    }
    pub fn set(&self, now: u64) {
        self.0.store(now, Ordering::SeqCst);
    }
    /// Seconds since the Unix epoch.
    pub fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

/// Shared state of the login service. Supplied; do not add a second copy of any
/// of this in globals.
#[derive(Clone)]
pub struct AuthState {
    /// User name → stored PHC string.
    pub users: Arc<HashMap<String, String>>,
    /// A valid PHC string that belongs to no user, made with the same parameters
    /// as the real ones. See task C.
    pub decoy_hash: Arc<str>,
    pub key: TokenKey,
    pub checker: Arc<dyn PasswordCheck>,
    pub clock: Clock,
    /// How long a new token stays valid, in seconds.
    pub token_ttl: u64,
}

/// Body of `POST /login`. `Debug` hides the password so that it cannot reach a
/// log line by accident.
#[derive(Clone, Deserialize)]
pub struct Credentials {
    pub user: String,
    pub password: String,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("user", &self.user)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// The authenticated user, taken from a valid `Authorization: Bearer <token>`
/// header. A handler that lists `AuthUser` as a parameter cannot run without one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthUser(pub String);

/// The response for every failed authentication: status 401, the header
/// `www-authenticate: Bearer`, and the JSON body `{"error": "unauthorized"}`.
/// It deliberately does not say *why* (missing, malformed, bad signature, expired).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unauthorized;

impl IntoResponse for Unauthorized {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            Json(serde_json::json!({ "error": "unauthorized" })),
        )
            .into_response()
    }
}

/// D. The extractor. Read the `authorization` header, require the scheme `Bearer`
/// followed by one space and the token, verify the token with the state's key and
/// clock, and return the user. Any failure is [`Unauthorized`].
impl FromRequestParts<AuthState> for AuthUser {
    type Rejection = Unauthorized;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AuthState,
    ) -> Result<AuthUser, Unauthorized> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(Unauthorized)?;
        let claims =
            verify_token(&state.key, token, state.clock.now()).map_err(|_| Unauthorized)?;
        Ok(AuthUser(claims.user))
    }
}

/// C. `POST /login` with a JSON [`Credentials`] body.
///
/// - Correct: 200 and `{"token": "<token>", "expires_at": <seconds>}` where the
///   token expires `state.token_ttl` seconds after `state.clock.now()`.
/// - Unknown user **or** wrong password: 401 with exactly the body
///   `{"error": "invalid_credentials"}`. The two cases must be indistinguishable:
///   same status, same headers, same body, and the same amount of hashing work.
///   For an unknown user, check the password against `state.decoy_hash` and
///   ignore the result. Call `state.checker.matches` exactly once per request.
/// - Hashing blocks a thread for a noticeable time. Run it with
///   `tokio::task::spawn_blocking`, never directly in the handler.
pub async fn login(
    State(state): State<AuthState>,
    Json(credentials): Json<Credentials>,
) -> Response {
    // Pick the hash to check before any await, so that both paths below are the same.
    let (stored, user_exists) = match state.users.get(&credentials.user) {
        Some(phc) => (Arc::<str>::from(phc.as_str()), true),
        None => (Arc::clone(&state.decoy_hash), false),
    };
    let checker = Arc::clone(&state.checker);
    let password = credentials.password;
    let matched = tokio::task::spawn_blocking(move || checker.matches(&stored, &password))
        .await
        .unwrap_or(false);

    if !(user_exists && matched) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "invalid_credentials" })),
        )
            .into_response();
    }
    let claims = Claims {
        user: credentials.user,
        expires_at: state.clock.now() + state.token_ttl,
    };
    let token = sign_token(&state.key, &claims);
    Json(serde_json::json!({ "token": token, "expires_at": claims.expires_at })).into_response()
}

/// E. The router: `POST /login` (task C) and `GET /me`, which needs a valid token
/// and answers 200 with `{"user": "<name>"}`. Write the `/me` handler so that it
/// takes an [`AuthUser`] parameter and contains no authentication code itself.
pub fn app(state: AuthState) -> Router {
    async fn me(AuthUser(user): AuthUser) -> Json<serde_json::Value> {
        Json(serde_json::json!({ "user": user }))
    }
    Router::new()
        .route("/login", post(login))
        .route("/me", get(me))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{get, post_json, send, with_header};
    use serde_json::json;
    use std::sync::Mutex;
    use std::thread::ThreadId;

    const NOW: u64 = 1_800_000_000;
    const TTL: u64 = 900;

    fn key() -> TokenKey {
        TokenKey::new(b"a test key; real keys are 32 random bytes")
    }

    fn claims(user: &str, expires_at: u64) -> Claims {
        Claims {
            user: user.to_owned(),
            expires_at,
        }
    }

    /// Delegates to the real check and records each call.
    #[derive(Default)]
    struct Recording {
        calls: Mutex<Vec<(String, ThreadId)>>,
    }

    impl PasswordCheck for Recording {
        fn matches(&self, stored_phc: &str, password: &str) -> bool {
            self.calls
                .lock()
                .unwrap()
                .push((stored_phc.to_owned(), std::thread::current().id()));
            Argon2Check.matches(stored_phc, password)
        }
    }

    fn state_with(checker: Arc<dyn PasswordCheck>) -> AuthState {
        let params = test_params();
        let users = HashMap::from([
            (
                "ada".to_owned(),
                hash_password(&params, "correct horse").unwrap(),
            ),
            (
                "lin".to_owned(),
                hash_password(&params, "battery staple").unwrap(),
            ),
        ]);
        AuthState {
            users: Arc::new(users),
            decoy_hash: hash_password(&params, "nobody has this password")
                .unwrap()
                .into(),
            key: key(),
            checker,
            clock: Clock::fixed(NOW),
            token_ttl: TTL,
        }
    }

    fn login_request(user: &str, password: &str) -> axum::http::Request<axum::body::Body> {
        post_json("/login", &json!({ "user": user, "password": password }))
    }

    #[test]
    fn a_hashes_are_phc_strings_with_fresh_salts() {
        let params = test_params();
        let first = hash_password(&params, "correct horse").unwrap();
        let second = hash_password(&params, "correct horse").unwrap();
        assert!(
            first.starts_with("$argon2id$v=19$m=64,t=1,p=1$"),
            "expected an Argon2id PHC string with the given parameters, got {first}"
        );
        assert_eq!(
            first.split('$').count(),
            6,
            "PHC: $id$version$params$salt$hash"
        );
        assert_ne!(
            first, second,
            "every hash needs its own random salt, so equal passwords give different strings"
        );
        assert!(!first.contains("correct horse"));
    }

    #[test]
    fn a_verify_accepts_only_the_right_password() {
        let stored = hash_password(&test_params(), "correct horse").unwrap();
        assert_eq!(verify_password(&stored, "correct horse"), Ok(true));
        assert_eq!(verify_password(&stored, "correct horsf"), Ok(false));
        assert_eq!(verify_password(&stored, ""), Ok(false));
        assert!(
            verify_password("plaintext-from-an-old-system", "x").is_err(),
            "a stored value that is not a PHC string is an error, not a wrong password"
        );
    }

    #[test]
    fn b_tokens_round_trip_until_they_expire() {
        let token = sign_token(&key(), &claims("ada", NOW + TTL));
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "v1.<payload>.<mac>, got {token}");
        assert_eq!(parts[0], "v1");
        assert_eq!(
            from_hex(parts[1]).as_deref(),
            Some(
                serde_json::to_vec(&claims("ada", NOW + TTL))
                    .unwrap()
                    .as_slice()
            ),
            "the payload is the claims JSON in hex"
        );

        assert_eq!(
            verify_token(&key(), &token, NOW),
            Ok(claims("ada", NOW + TTL))
        );
        assert_eq!(
            verify_token(&key(), &token, NOW + TTL - 1),
            Ok(claims("ada", NOW + TTL))
        );
        assert_eq!(
            verify_token(&key(), &token, NOW + TTL),
            Err(TokenError::Expired),
            "a token is valid strictly before expires_at"
        );
    }

    #[test]
    fn b_forged_tokens_are_rejected() {
        let token = sign_token(&key(), &claims("ada", NOW + TTL));
        let parts: Vec<&str> = token.split('.').collect();

        // Someone edits the payload and keeps the old MAC.
        let forged_payload = to_hex(&serde_json::to_vec(&claims("root", NOW + TTL)).unwrap());
        let forged = format!("v1.{forged_payload}.{}", parts[2]);
        assert_eq!(
            verify_token(&key(), &forged, NOW),
            Err(TokenError::BadSignature)
        );

        // A token signed with a different key.
        let other = sign_token(&TokenKey::new(b"another key"), &claims("ada", NOW + TTL));
        assert_eq!(
            verify_token(&key(), &other, NOW),
            Err(TokenError::BadSignature)
        );

        // Only a prefix of the right MAC.
        let truncated = format!("v1.{}.{}", parts[1], &parts[2][..16]);
        assert_eq!(
            verify_token(&key(), &truncated, NOW),
            Err(TokenError::BadSignature),
            "a shortened MAC must not be accepted"
        );

        // One flipped bit in the MAC.
        let mut mac = from_hex(parts[2]).unwrap();
        mac[31] ^= 1;
        let flipped = format!("v1.{}.{}", parts[1], to_hex(&mac));
        assert_eq!(
            verify_token(&key(), &flipped, NOW),
            Err(TokenError::BadSignature)
        );
    }

    #[test]
    fn b_the_mac_is_checked_before_the_payload_is_read() {
        // Not JSON, and not signed by us: the answer must be about the signature.
        let junk_payload = to_hex(b"not json");
        let unsigned = format!("v1.{junk_payload}.{}", "00".repeat(32));
        assert_eq!(
            verify_token(&key(), &unsigned, NOW),
            Err(TokenError::BadSignature),
            "verify the MAC before parsing: unauthenticated bytes must not reach the JSON parser"
        );

        // An expired token with a wrong MAC is a forgery, not an expired token.
        let expired = sign_token(&TokenKey::new(b"another key"), &claims("ada", NOW - 1));
        assert_eq!(
            verify_token(&key(), &expired, NOW),
            Err(TokenError::BadSignature)
        );

        for malformed in [
            "",
            "v1",
            "v1.00",
            "v1.00.00.00",
            "v2.00.00",
            "v1.zz.00",
            "v1.00.0",
            "v1.AB.00",
        ] {
            assert_eq!(
                verify_token(&key(), malformed, NOW),
                Err(TokenError::Malformed),
                "{malformed:?}"
            );
        }
    }

    #[test]
    fn b_the_mac_is_compared_in_constant_time() {
        // First the behaviour, so that this test also starts at `todo!()`.
        let token = sign_token(&key(), &claims("ada", NOW + TTL));
        assert_eq!(
            verify_token(&key(), &token, NOW),
            Ok(claims("ada", NOW + TTL))
        );

        // No test can measure reliably how long a comparison takes. So this one
        // reads the source text of this file: everything above the test module,
        // without comments. It passes when that code lets the `hmac` crate
        // compare the MAC (`verify_slice`, `verify`, or two `CtOutput` values).
        // It cannot see whether you use the result; the other tests do that.
        let source = include_str!("l04_auth.rs");
        let code: String = source
            .split("#[cfg(test)]")
            .next()
            .unwrap_or_default()
            .lines()
            .map(|line| line.split("//").next().unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");
        let constant_time = ["verify_slice(", ".verify(", "CtOutput"]
            .iter()
            .any(|call| code.contains(call));
        assert!(
            constant_time,
            "verify_token does not use the `hmac` crate's constant-time check. `==` on \
             bytes or on hex text stops at the first difference, so its duration tells \
             an attacker how many leading bytes of a forged MAC were right. Use \
             `Mac::verify_slice`"
        );
    }

    #[tokio::test]
    async fn c_login_issues_a_token_that_expires_after_the_ttl() {
        crate::support::deadline(
            120,
            "the test `c_login_issues_a_token_that_expires_after_the_ttl`",
            async {
                let state = state_with(Arc::new(Argon2Check));
                let app = app(state.clone());
                let response = send(&app, login_request("ada", "correct horse")).await;
                assert_eq!(response.code(), 200, "{}", response.text());
                let body = response.json();
                assert_eq!(body["expires_at"], NOW + TTL);
                let token = body["token"].as_str().expect("a token string");
                assert_eq!(
                    verify_token(&state.key, token, NOW),
                    Ok(claims("ada", NOW + TTL))
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn c_failures_look_the_same_for_unknown_user_and_wrong_password() {
        crate::support::deadline(
            120,
            "the test `c_failures_look_the_same_for_unknown_user_and_wrong_password`",
            async {
                let app = app(state_with(Arc::new(Argon2Check)));
                let wrong_password = send(&app, login_request("ada", "wrong")).await;
                let unknown_user = send(&app, login_request("mallory", "wrong")).await;
                let other_users_password = send(&app, login_request("ada", "battery staple")).await;

                assert_eq!(wrong_password.code(), 401);
                assert_eq!(
                    wrong_password.json(),
                    json!({ "error": "invalid_credentials" })
                );
                for (name, response) in [
                    ("unknown user", &unknown_user),
                    ("another user's password", &other_users_password),
                ] {
                    assert_eq!(response.code(), 401, "{name}");
                    assert_eq!(
                        response.body, wrong_password.body,
                        "{name}: the body must not reveal which part was wrong"
                    );
                    assert_eq!(
                        response.headers, wrong_password.headers,
                        "{name}: the headers must not differ either"
                    );
                }
            },
        )
        .await
    }

    #[tokio::test]
    async fn c_every_attempt_costs_exactly_one_hash_off_the_async_thread() {
        crate::support::deadline(
            120,
            "the test `c_every_attempt_costs_exactly_one_hash_off_the_async_thread`",
            async {
                let recording = Arc::new(Recording::default());
                let state = state_with(recording.clone());
                let app = app(state.clone());
                let runtime_thread = std::thread::current().id();

                for (user, password, expected_status) in [
                    ("ada", "correct horse", 200),
                    ("ada", "wrong", 401),
                    ("mallory", "correct horse", 401),
                ] {
                    let response = send(&app, login_request(user, password)).await;
                    assert_eq!(response.code(), expected_status, "{user}/{password}");
                }

                let calls = recording.calls.lock().unwrap();
                assert_eq!(
                    calls.len(),
                    3,
                    "each attempt must run exactly one password check, also for an unknown \
             user; otherwise the response time shows which user names exist"
                );
                assert_eq!(calls[0].0, state.users["ada"]);
                assert_eq!(
                    &*calls[2].0, &*state.decoy_hash,
                    "an unknown user is checked against the decoy hash"
                );
                for (_, thread) in calls.iter() {
                    assert_ne!(
                        *thread, runtime_thread,
                        "the hash ran on the async runtime thread; move it to spawn_blocking"
                    );
                }
            },
        )
        .await
    }

    #[tokio::test]
    async fn d_me_needs_a_valid_bearer_token() {
        crate::support::deadline(120, "the test `d_me_needs_a_valid_bearer_token`", async {
            let state = state_with(Arc::new(Argon2Check));
            let app = app(state.clone());
            let token = sign_token(&state.key, &claims("lin", NOW + TTL));

            let ok = send(
                &app,
                with_header(get("/me"), "authorization", &format!("Bearer {token}")),
            )
            .await;
            assert_eq!(ok.code(), 200);
            assert_eq!(ok.json(), json!({ "user": "lin" }));

            let stolen_key = sign_token(&TokenKey::new(b"guess"), &claims("lin", NOW + TTL));
            let rejected = [
                get("/me"),
                with_header(get("/me"), "authorization", &token),
                with_header(get("/me"), "authorization", &format!("Basic {token}")),
                with_header(get("/me"), "authorization", &format!("Bearer {stolen_key}")),
                with_header(get("/me"), "authorization", "Bearer "),
            ];
            let mut bodies = Vec::new();
            for request in rejected {
                let label = format!("{:?}", request.headers().get("authorization"));
                let response = send(&app, request).await;
                assert_eq!(response.code(), 401, "{label}");
                assert_eq!(
                    response.header("www-authenticate"),
                    Some("Bearer"),
                    "{label}"
                );
                assert_eq!(
                    response.json(),
                    json!({ "error": "unauthorized" }),
                    "{label}"
                );
                bodies.push(response.body);
            }

            state.clock.set(NOW + TTL);
            let expired = send(
                &app,
                with_header(get("/me"), "authorization", &format!("Bearer {token}")),
            )
            .await;
            assert_eq!(
                expired.code(),
                401,
                "the token expired when the clock moved"
            );
            assert_eq!(
                expired.body, bodies[0],
                "an expired token gets the same answer as a missing one"
            );
        })
        .await
    }

    #[tokio::test]
    async fn e_login_then_me_end_to_end() {
        crate::support::deadline(120, "the test `e_login_then_me_end_to_end`", async {
            let app = app(state_with(Arc::new(Argon2Check)));
            let login = send(&app, login_request("lin", "battery staple")).await;
            let token = login.json()["token"].as_str().unwrap().to_owned();
            let me = send(
                &app,
                with_header(get("/me"), "authorization", &format!("Bearer {token}")),
            )
            .await;
            assert_eq!(me.json(), json!({ "user": "lin" }));
        })
        .await
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        let params = test_params();
        let stored = hash_password(&params, "pw").unwrap();

        // P1. How many '$'-separated fields of the PHC string change when the same
        // password is hashed again with the same parameters? (`split('$')` gives
        // six fields; the first is empty.)
        let again = hash_password(&params, "pw").unwrap();
        let changed = stored
            .split('$')
            .zip(again.split('$'))
            .filter(|(a, b)| a != b)
            .count();
        let p1: usize = 2;
        assert_eq!(p1, changed, "P1: which PHC fields depend on the salt");

        // P2. The hash was made with m=64. We now verify it through an `Argon2`
        // built with much larger parameters. Does the right password still verify?
        let big = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(1024, 3, 1, None).unwrap(),
        );
        let parsed = PasswordHash::new(&stored).unwrap();
        let p2: bool = true;
        assert_eq!(
            p2,
            big.verify_password(b"pw", &parsed).is_ok(),
            "P2: verification uses the parameters stored in the PHC string"
        );

        // P3. How many characters has the MAC part of a token (the text after the
        // last dot)?
        let token = sign_token(&key(), &claims("ada", NOW));
        let p3: usize = 64;
        assert_eq!(
            p3,
            token.rsplit('.').next().unwrap().len(),
            "P3: HMAC-SHA-256 output in hex"
        );

        // P4. What does `{:?}` print for these credentials?
        let credentials = Credentials {
            user: "ada".to_owned(),
            password: "hunter2".to_owned(),
        };
        let p4: bool = false;
        assert_eq!(
            p4,
            format!("{credentials:?}").contains("hunter2"),
            "P4: does Debug output contain the password"
        );
    }
}
