use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};

pub const BASE_URL: &str = "http://localhost:8000";

/// Build a reqwest client for tests.
pub fn client() -> Client {
    Client::new()
}

/// Generate a unique suffix based on timestamp + random bits to avoid collisions.
pub fn unique_suffix() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}", ts % 1_000_000_000)
}

/// Build a URL for an API endpoint.
pub fn url(path: &str) -> String {
    format!("{}{}", BASE_URL, path)
}
