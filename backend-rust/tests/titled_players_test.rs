//! Integration tests for titled player tags.
//!
//! Requires the server to be running on localhost:8000 with titled players cache loaded.
//! Syncs Hikaru's Chess.com games once, then runs all assertions against that data.

mod common;

use serde_json::{json, Value};
use tokio::sync::OnceCell;

// ---------------------------------------------------------------------------
// Shared state — register + sync once, reuse across all tests
// ---------------------------------------------------------------------------

struct TestUser {
    token: String,
}

static TEST_USER: OnceCell<TestUser> = OnceCell::const_new();

async fn get_test_user() -> &'static TestUser {
    TEST_USER
        .get_or_init(|| async {
            let client = common::client();
            let suffix = common::unique_suffix();

            // Register
            let resp = client
                .post(common::url("/api/auth/register"))
                .json(&json!({
                    "username": format!("titled_{suffix}"),
                    "email": format!("titled_{suffix}@alpine.dev"),
                    "password": "testpass123",
                    "chessComUsername": "hikaru",
                }))
                .send()
                .await
                .expect("Failed to register");

            assert_eq!(resp.status(), 200, "Register should succeed");
            let body: Value = resp.json().await.unwrap();
            let token = body["token"].as_str().unwrap().to_string();

            // Sync games (one time)
            let resp = client
                .post(common::url("/api/games/sync"))
                .bearer_auth(&token)
                .send()
                .await
                .expect("Failed to sync");

            assert_eq!(resp.status(), 200, "Sync should succeed");
            let sync_body: Value = resp.json().await.unwrap();
            let synced = sync_body["synced"].as_i64().unwrap_or(0);
            assert!(synced > 0, "Should have synced at least some games");

            TestUser { token }
        })
        .await
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn get_tags(client: &reqwest::Client, token: &str) -> Value {
    let resp = client
        .get(common::url("/api/games/tags"))
        .bearer_auth(token)
        .send()
        .await
        .expect("Failed to get tags");

    assert_eq!(resp.status(), 200);
    resp.json().await.unwrap()
}

async fn get_stored_games(
    client: &reqwest::Client,
    token: &str,
    tags: Option<&str>,
) -> Value {
    let mut url = common::url("/api/games/stored");
    if let Some(t) = tags {
        url = format!("{url}?tags={t}");
    }

    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .expect("Failed to get stored games");

    assert_eq!(resp.status(), 200);
    resp.json().await.unwrap()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Admin backfill endpoint returns proper response shape.
#[tokio::test]
async fn backfill_returns_valid_response() {
    let client = common::client();

    let resp = client
        .post(common::url("/api/admin/backfill-titled-tags"))
        .send()
        .await
        .expect("Failed to call backfill");

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();

    assert_eq!(body["success"], true);
    assert!(body["totalTagged"].is_number(), "totalTagged should be a number");
    assert!(body["chessComTagged"].is_number(), "chessComTagged should be a number");
    assert!(body["lichessTagged"].is_number(), "lichessTagged should be a number");
    assert!(body["gamesChecked"].is_number(), "gamesChecked should be a number");
}

/// Sync Hikaru's games and verify titled tags appear in tag counts.
#[tokio::test]
async fn sync_games_and_check_titled_tags() {
    let user = get_test_user().await;
    let client = common::client();

    let tags_body = get_tags(&client, &user.token).await;
    let tags = &tags_body["tags"];

    // Standard tags should be present
    assert!(tags["Win"].is_number() || tags["Loss"].is_number(),
        "Should have at least Win or Loss tags from synced games");
    assert!(tags["Chess.com"].is_number(),
        "Should have Chess.com platform tag");

    // Hikaru plays titled opponents constantly — titled tag must exist
    let titled_count = tags["titled"].as_i64().expect("Hikaru should have titled opponents");
    assert!(titled_count > 0, "titled count should be positive");

    // At least one specific title tag should also exist
    let title_tags = ["GM", "IM", "FM", "CM", "NM", "WGM", "WIM", "WFM", "WCM", "WNM"];
    let has_specific = title_tags.iter().any(|t| tags[*t].is_number());
    assert!(has_specific,
        "If 'titled' tag exists, at least one specific title tag should too");

    // Sum of specific title counts should equal the "titled" count
    let specific_sum: i64 = title_tags
        .iter()
        .filter_map(|t| tags[*t].as_i64())
        .sum();
    assert_eq!(specific_sum, titled_count,
        "Sum of specific title tags ({specific_sum}) should equal titled count ({titled_count})");
}

/// Filter games by "titled" tag — all returned games should have the tag.
#[tokio::test]
async fn filter_games_by_titled_tag() {
    let user = get_test_user().await;
    let client = common::client();

    // Filter by "titled"
    let games_body = get_stored_games(&client, &user.token, Some("titled")).await;
    let games = games_body["games"].as_array().unwrap();
    let total = games_body["total"].as_i64().unwrap_or(0);

    assert!(total > 0, "Hikaru should have titled opponents");
    assert!(!games.is_empty(), "Should return titled games");

    // Every returned game should have "titled" in its tags
    for game in games {
        let game_tags = game["tags"].as_array().unwrap();
        let tag_strs: Vec<&str> = game_tags.iter().filter_map(|t| t.as_str()).collect();
        assert!(tag_strs.contains(&"titled"),
            "Game {} should have 'titled' tag, got {:?}",
            game["id"], tag_strs);
    }
}

/// Filter by a specific title (e.g. "GM") — verify total count matches and
/// returned games have the correct tags.
#[tokio::test]
async fn filter_games_by_specific_title() {
    let user = get_test_user().await;
    let client = common::client();

    let tags_body = get_tags(&client, &user.token).await;
    let tags = &tags_body["tags"];

    // Find a specific title that has games
    let title_tags = ["GM", "IM", "FM", "CM", "NM", "WGM", "WIM", "WFM", "WCM", "WNM"];
    let found_title = title_tags.iter().find(|t| tags[**t].as_i64().unwrap_or(0) > 0);

    let title = found_title.expect("Hikaru should have at least one specific title tag");
    let expected_count = tags[*title].as_i64().unwrap();

    let games_body = get_stored_games(&client, &user.token, Some(title)).await;
    let total = games_body["total"].as_i64().unwrap();
    let games = games_body["games"].as_array().unwrap();

    // Total from the API should match the tag count
    assert_eq!(total, expected_count,
        "Total for {title} filter should match tag count");

    // Each returned game should have both "titled" and the specific title tag
    for game in games {
        let game_tags = game["tags"].as_array().unwrap();
        let tag_strs: Vec<&str> = game_tags.iter().filter_map(|t| t.as_str()).collect();
        assert!(tag_strs.contains(&"titled"),
            "Game {} should have 'titled' tag", game["id"]);
        assert!(tag_strs.contains(title),
            "Game {} should have '{title}' tag", game["id"]);
    }
}

/// Backfill after sync should be idempotent — running it again doesn't double-tag.
#[tokio::test]
async fn backfill_is_idempotent() {
    let user = get_test_user().await;
    let client = common::client();

    // Get tag counts before backfill
    let before = get_tags(&client, &user.token).await;
    let titled_before = before["tags"]["titled"].as_i64().unwrap_or(0);

    // Run backfill
    let resp = client
        .post(common::url("/api/admin/backfill-titled-tags"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Get tag counts after backfill
    let after = get_tags(&client, &user.token).await;
    let titled_after = after["tags"]["titled"].as_i64().unwrap_or(0);

    assert_eq!(titled_before, titled_after,
        "Backfill should be idempotent: titled count was {titled_before} before, {titled_after} after");
}
