use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMetadata {
    pub white: String,
    pub black: String,
    pub result: String, // "1-0", "0-1", "1/2-1/2"
    pub date: Option<String>,
    pub time_control: Option<String>,
    pub eco: Option<String>,
    pub event: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameData {
    pub metadata: GameMetadata,
    pub moves: Vec<String>,  // SAN notation
    pub pgn: String,
    pub tcn: Option<String>,
}
