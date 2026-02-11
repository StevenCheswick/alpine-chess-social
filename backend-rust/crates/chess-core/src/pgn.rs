//! PGN parsing utilities — lightweight regex-based parser.

use regex::Regex;

use crate::game_data::{GameData, GameMetadata};
use crate::tcn;

const STANDARD_START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Parse a PGN string into a GameData struct.
/// If `tcn` is provided, uses that for moves (from Chess.com API).
/// Otherwise parses SAN moves from the PGN and generates TCN.
pub fn parse_pgn(pgn: &str, tcn: Option<&str>) -> Option<GameData> {
    // Extract headers
    let header_re = Regex::new(r#"\[(\w+)\s+"([^"]*)"\]"#).ok()?;

    let mut white = "Unknown".to_string();
    let mut black = "Unknown".to_string();
    let mut result = "*".to_string();
    let mut date = None;
    let mut time_control = None;
    let mut eco = None;
    let mut event = None;
    let mut link = None;
    let mut setup = None;
    let mut fen = None;

    for cap in header_re.captures_iter(pgn) {
        let key = &cap[1];
        let value = cap[2].to_string();
        match key {
            "White" => white = value,
            "Black" => black = value,
            "Result" => result = value,
            "Date" => date = Some(value),
            "TimeControl" => time_control = Some(value),
            "ECO" => eco = Some(value),
            "Event" => event = Some(value),
            "Link" => link = Some(value),
            "SetUp" => setup = Some(value),
            "FEN" => fen = Some(value),
            _ => {}
        }
    }

    // Filter non-standard positions
    if setup.as_deref() == Some("1") {
        if let Some(ref f) = fen {
            if f != STANDARD_START_FEN {
                return None;
            }
        }
    }

    let metadata = GameMetadata {
        white,
        black,
        result,
        date,
        time_control,
        eco,
        event,
        link,
    };

    // Extract SAN moves
    let moves = extract_moves(pgn);

    if moves.is_empty() && tcn.is_none() {
        return None;
    }

    // Generate TCN if not provided
    let final_tcn = if let Some(t) = tcn {
        Some(t.to_string())
    } else if !moves.is_empty() {
        let move_strs: Vec<String> = moves.iter().map(|s| s.to_string()).collect();
        tcn::encode_san_to_tcn(&move_strs).ok()
    } else {
        None
    };

    Some(GameData {
        metadata,
        moves: moves.into_iter().map(|s| s.to_string()).collect(),
        pgn: pgn.to_string(),
        tcn: final_tcn,
    })
}

/// Extract SAN moves from PGN text (after removing headers, comments, variations).
fn extract_moves(pgn: &str) -> Vec<String> {
    // Remove headers
    let header_re = Regex::new(r"\[[^\]]*\]").unwrap();
    let no_headers = header_re.replace_all(pgn, "");

    // Remove comments
    let comment_re = Regex::new(r"\{[^}]*\}").unwrap();
    let no_comments = comment_re.replace_all(&no_headers, "");

    // Remove variations
    let variation_re = Regex::new(r"\([^)]*\)").unwrap();
    let no_variations = variation_re.replace_all(&no_comments, "");

    // Extract moves
    let move_re =
        Regex::new(r"[KQRBN]?[a-h]?[1-8]?x?[a-h][1-8](?:=[QRBN])?[+#]?|O-O-O|O-O").unwrap();

    move_re
        .find_iter(&no_variations)
        .map(|m| m.as_str().to_string())
        .collect()
}

/// Extract a string value from a PGN header (e.g. WhiteTitle, BlackTitle).
pub fn extract_header(pgn: &str, header_name: &str) -> Option<String> {
    let pattern = format!(r#"\[{}\s+"([^"]*)"\]"#, regex::escape(header_name));
    let re = Regex::new(&pattern).ok()?;
    let value = re.captures(pgn)?.get(1)?.as_str().to_string();
    if value.is_empty() { None } else { Some(value) }
}

/// Extract an integer value from a PGN header.
pub fn extract_header_int(pgn: &str, header_name: &str) -> Option<i32> {
    let pattern = format!(r#"\[{}\s+"(\d+)"\]"#, regex::escape(header_name));
    let re = Regex::new(&pattern).ok()?;
    re.captures(pgn)?
        .get(1)?
        .as_str()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pgn_basic() {
        let pgn = r#"[White "Player1"]
[Black "Player2"]
[Result "1-0"]
[Date "2025.01.15"]
[TimeControl "600"]

1. e4 e5 2. Nf3 Nc6 1-0"#;

        let game = parse_pgn(pgn, None).unwrap();
        assert_eq!(game.metadata.white, "Player1");
        assert_eq!(game.metadata.black, "Player2");
        assert_eq!(game.metadata.result, "1-0");
        assert_eq!(game.moves.len(), 4);
        assert_eq!(game.moves[0], "e4");
    }

    #[test]
    fn test_extract_header_int() {
        let pgn = r#"[WhiteElo "1500"]
[BlackElo "1600"]"#;

        assert_eq!(extract_header_int(pgn, "WhiteElo"), Some(1500));
        assert_eq!(extract_header_int(pgn, "BlackElo"), Some(1600));
        assert_eq!(extract_header_int(pgn, "Missing"), None);
    }
}
