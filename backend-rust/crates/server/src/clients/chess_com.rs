use reqwest::Client;
use serde_json::Value;

pub struct ChessComClient {
    client: Client,
}

impl ChessComClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("AlpineChess/1.0")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        Self { client }
    }

    /// Fetch games for a user from Chess.com.
    /// Returns list of (pgn, Option<tcn>) tuples.
    pub async fn fetch_user_games(
        &self,
        username: &str,
        year: Option<i32>,
        month: Option<u32>,
        include_tcn: bool,
    ) -> Result<Vec<(String, Option<String>)>, String> {
        let url = format!(
            "https://api.chess.com/pub/player/{}/games/{}/{:02}",
            username,
            year.unwrap_or(2025),
            month.unwrap_or(1)
        );

        // Rate limit
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request error: {e}"))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(vec![]);
        }

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {e}"))?;

        let games = data["games"].as_array().cloned().unwrap_or_default();
        let mut results = Vec::new();

        for game in games {
            // Skip unrated games
            if !game.get("rated").and_then(|v| v.as_bool()).unwrap_or(true) {
                continue;
            }

            // Skip variant games
            let rules = game.get("rules").and_then(|v| v.as_str()).unwrap_or("chess");
            if rules != "chess" {
                continue;
            }

            if let Some(pgn) = game.get("pgn").and_then(|v| v.as_str()) {
                let tcn = if include_tcn {
                    game.get("tcn").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    None
                };
                results.push((pgn.to_string(), tcn));
            }
        }

        Ok(results)
    }
}
