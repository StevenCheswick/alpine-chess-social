//! Integration tests for auth endpoints.
//!
//! Requires the server to be running on localhost:8000.

mod common;

use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Register a test user and return the full response body.
async fn register_user(
    client: &reqwest::Client,
    username: &str,
    email: &str,
    password: &str,
) -> reqwest::Response {
    client
        .post(common::url("/api/auth/register"))
        .json(&json!({
            "username": username,
            "email": email,
            "password": password,
        }))
        .send()
        .await
        .expect("Failed to send register request")
}

/// Login and return the response.
async fn login_user(
    client: &reqwest::Client,
    email: &str,
    password: &str,
) -> reqwest::Response {
    client
        .post(common::url("/api/auth/login"))
        .json(&json!({
            "email": email,
            "password": password,
        }))
        .send()
        .await
        .expect("Failed to send login request")
}

/// Call GET /api/auth/me with a bearer token.
async fn get_me(client: &reqwest::Client, token: &str) -> reqwest::Response {
    client
        .get(common::url("/api/auth/me"))
        .bearer_auth(token)
        .send()
        .await
        .expect("Failed to send me request")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Full auth flow: register → login → me.
#[tokio::test]
async fn register_login_and_me() {
    let client = common::client();
    let suffix = common::unique_suffix();
    let username = format!("testuser_{suffix}");
    let email = format!("test_{suffix}@alpine.dev");
    let password = "testpass123";

    // ── Register ────────────────────────────────────────────────────
    let resp = register_user(&client, &username, &email, password).await;
    assert_eq!(resp.status(), 200, "Register should succeed");

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["user"]["username"], username);
    assert_eq!(body["user"]["email"], email);
    assert!(body["token"].is_string(), "Should return a JWT token");

    let token = body["token"].as_str().unwrap();

    // ── Login ───────────────────────────────────────────────────────
    let resp = login_user(&client, &email, password).await;
    assert_eq!(resp.status(), 200, "Login should succeed");

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["user"]["username"], username);
    assert!(body["token"].is_string(), "Login should return a JWT token");

    // ── Me ──────────────────────────────────────────────────────────
    let resp = get_me(&client, token).await;
    assert_eq!(resp.status(), 200, "GET /me should succeed with valid token");

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["username"], username);
    assert_eq!(body["email"], email);
}

/// Registering the same email twice should fail.
#[tokio::test]
async fn register_duplicate_email_fails() {
    let client = common::client();
    let suffix = common::unique_suffix();
    let email = format!("dup_{suffix}@alpine.dev");
    let password = "testpass123";

    // First registration
    let resp = register_user(&client, &format!("dup_a_{suffix}"), &email, password).await;
    assert_eq!(resp.status(), 200);

    // Second registration with same email, different username
    let resp = register_user(&client, &format!("dup_b_{suffix}"), &email, password).await;
    assert_eq!(resp.status(), 400, "Duplicate email should be rejected");

    let body: Value = resp.json().await.unwrap();
    assert!(
        body["detail"].as_str().unwrap().contains("Email"),
        "Error should mention email: got {:?}",
        body["detail"]
    );
}

/// Registering the same username twice should fail.
#[tokio::test]
async fn register_duplicate_username_fails() {
    let client = common::client();
    let suffix = common::unique_suffix();
    let username = format!("dupuser_{suffix}");
    let password = "testpass123";

    let resp = register_user(
        &client,
        &username,
        &format!("first_{suffix}@alpine.dev"),
        password,
    )
    .await;
    assert_eq!(resp.status(), 200);

    let resp = register_user(
        &client,
        &username,
        &format!("second_{suffix}@alpine.dev"),
        password,
    )
    .await;
    assert_eq!(resp.status(), 400, "Duplicate username should be rejected");

    let body: Value = resp.json().await.unwrap();
    assert!(
        body["detail"].as_str().unwrap().contains("Username"),
        "Error should mention username: got {:?}",
        body["detail"]
    );
}

/// Login with wrong password should fail.
#[tokio::test]
async fn login_wrong_password_fails() {
    let client = common::client();
    let suffix = common::unique_suffix();
    let email = format!("wrongpw_{suffix}@alpine.dev");

    // Register first
    let resp = register_user(&client, &format!("wrongpw_{suffix}"), &email, "correctpass1").await;
    assert_eq!(resp.status(), 200);

    // Login with wrong password
    let resp = login_user(&client, &email, "wrongpassword").await;
    assert_eq!(resp.status(), 400, "Wrong password should be rejected");
}

/// Login with nonexistent email should fail.
#[tokio::test]
async fn login_nonexistent_email_fails() {
    let client = common::client();
    let resp = login_user(&client, "nobody_at_all@alpine.dev", "whatever123").await;
    assert_eq!(resp.status(), 400, "Nonexistent email should be rejected");
}

/// GET /me without a token should fail.
#[tokio::test]
async fn me_without_token_fails() {
    let client = common::client();
    let resp = client
        .get(common::url("/api/auth/me"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401, "No token should return 401");
}

/// GET /me with an invalid token should fail.
#[tokio::test]
async fn me_with_invalid_token_fails() {
    let client = common::client();
    let resp = get_me(&client, "this.is.not.a.valid.jwt").await;
    assert_eq!(resp.status(), 401, "Invalid token should return 401");
}

/// Username validation: too short.
#[tokio::test]
async fn register_username_too_short() {
    let client = common::client();
    let suffix = common::unique_suffix();
    let resp = register_user(
        &client,
        "ab",
        &format!("short_{suffix}@alpine.dev"),
        "testpass123",
    )
    .await;
    assert_eq!(resp.status(), 400, "Username < 3 chars should be rejected");
}

/// Password validation: too short.
#[tokio::test]
async fn register_password_too_short() {
    let client = common::client();
    let suffix = common::unique_suffix();
    let resp = register_user(
        &client,
        &format!("shortpw_{suffix}"),
        &format!("shortpw_{suffix}@alpine.dev"),
        "short",
    )
    .await;
    assert_eq!(resp.status(), 400, "Password < 8 chars should be rejected");
}
