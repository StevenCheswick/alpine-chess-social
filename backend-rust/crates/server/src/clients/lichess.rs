use reqwest::Client;
use serde_json::Value;

pub struct LichessClient {
    client: Client,
}

impl LichessClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("AlpineChess/1.0")
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap();
        Self { client }
    }

    /// Fetch games for a user from Lichess.
    /// Returns list of (pgn, game_id) tuples.
    /// `since` is an optional epoch-millisecond timestamp to only fetch games after that time.
    pub async fn fetch_user_games(
        &self,
        username: &str,
        max_games: Option<usize>,
        since: Option<i64>,
    ) -> Result<Vec<(String, String)>, String> {
        let mut url = format!("https://lichess.org/api/games/user/{}", username);

        let mut params = vec![
            ("pgnInJson", "true".to_string()),
            ("opening", "true".to_string()),
            ("rated", "true".to_string()),
        ];

        if let Some(max) = max_games {
            params.push(("max", max.to_string()));
        }

        if let Some(since_ms) = since {
            params.push(("since", since_ms.to_string()));
        }

        // Rate limit
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let resp = self
            .client
            .get(&url)
            .query(&params)
            .header("Accept", "application/x-ndjson")
            .send()
            .await
            .map_err(|e| format!("Request error: {e}"))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err("User not found".to_string());
        }

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        let text = resp
            .text()
            .await
            .map_err(|e| format!("Body read error: {e}"))?;

        let mut results = Vec::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<Value>(line) {
                Ok(game_data) => {
                    if let Some(pgn) = game_data.get("pgn").and_then(|v| v.as_str()) {
                        if !pgn.is_empty() {
                            let game_id = game_data
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            results.push((pgn.to_string(), game_id));
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse Lichess game JSON: {e}");
                }
            }
        }

        Ok(results)
    }
}
