//! Backfill king_mate, castling_mate, en_passant_mate tags for already-analyzed games.
//!
//! Lightweight — replays TCN to get final board position, runs 3 detectors,
//! inserts any new tags. No Stockfish needed.
//!
//! Usage:
//!   cargo run -p analysis-worker --bin backfill-mate-tags
//!
//! Set DATABASE_URL env var or use .env file.

use chess::{Board, ChessMove, Color, MoveGen, Piece};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set (or use .env file)");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&db_url)
        .await?;

    // Fetch all analyzed games
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, tcn, user_color FROM user_games WHERE analyzed_at IS NOT NULL AND tcn IS NOT NULL",
    )
    .fetch_all(&pool)
    .await?;

    println!("Found {} analyzed games to check", rows.len());

    let mut tagged_king = 0u32;
    let mut tagged_castling = 0u32;
    let mut tagged_ep = 0u32;
    let mut errors = 0u32;

    for (game_id, tcn, user_color_str) in &rows {
        let user_color = if user_color_str == "white" {
            Color::White
        } else {
            Color::Black
        };

        // Decode TCN → SAN
        let san_moves = match chess_core::tcn::decode_tcn_to_san(tcn) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("  game {game_id}: TCN decode error: {e}");
                errors += 1;
                continue;
            }
        };

        if san_moves.is_empty() {
            continue;
        }

        // Replay moves to get final board and board before last move
        let mut board = Board::default();
        let mut last_move: Option<ChessMove> = None;
        let mut board_before_last = board;

        let mut ok = true;
        for san in &san_moves {
            let chess_move = match find_san_move(&board, san) {
                Some(m) => m,
                None => {
                    eprintln!("  game {game_id}: invalid SAN '{san}'");
                    errors += 1;
                    ok = false;
                    break;
                }
            };
            board_before_last = board;
            last_move = Some(chess_move);
            board = board.make_move_new(chess_move);
        }
        if !ok {
            continue;
        }

        let final_board = board;
        let last_move = match last_move {
            Some(m) => m,
            None => continue,
        };

        // Run detectors
        let mut new_tags: Vec<&str> = Vec::new();

        if analysis_worker::king_mate::detect_king_mate(
            &final_board, &board_before_last, last_move, user_color,
        ) {
            new_tags.push("king_mate");
        }

        if analysis_worker::castling_mate::detect_castling_mate(
            &final_board, &board_before_last, last_move, user_color,
        ) {
            new_tags.push("castling_mate");
        }

        if analysis_worker::en_passant_mate::detect_en_passant_mate(
            &final_board, &board_before_last, last_move, user_color,
        ) {
            new_tags.push("en_passant_mate");
        }

        if new_tags.is_empty() {
            continue;
        }

        // Insert new tags
        for tag in &new_tags {
            sqlx::query(
                "INSERT INTO game_tags (game_id, tag) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(game_id)
            .bind(*tag)
            .execute(&pool)
            .await?;
        }

        // Update denormalized tags JSONB on user_games
        sqlx::query(
            "UPDATE user_games SET tags = (
                SELECT COALESCE(jsonb_agg(DISTINCT t.tag), '[]'::jsonb)
                FROM game_tags t WHERE t.game_id = $1
            ) WHERE id = $1",
        )
        .bind(game_id)
        .execute(&pool)
        .await?;

        for tag in &new_tags {
            match *tag {
                "king_mate" => tagged_king += 1,
                "castling_mate" => tagged_castling += 1,
                "en_passant_mate" => tagged_ep += 1,
                _ => {}
            }
        }
        println!(
            "  game {game_id}: tagged {:?}",
            new_tags
        );
    }

    println!("\nDone!");
    println!("  king_mate: {tagged_king}");
    println!("  castling_mate: {tagged_castling}");
    println!("  en_passant_mate: {tagged_ep}");
    println!("  errors: {errors}");

    Ok(())
}

/// Minimal SAN parser — finds the legal move matching a SAN string.
fn find_san_move(board: &Board, san: &str) -> Option<ChessMove> {
    let clean = san.trim_end_matches(|c: char| c == '+' || c == '#' || c == '!' || c == '?');
    let legal_moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();

    // Castling
    if clean == "O-O" || clean == "0-0" {
        return legal_moves.iter().find(|m| {
            board.piece_on(m.get_source()) == Some(Piece::King) && {
                let sf = m.get_source().get_file().to_index();
                let df = m.get_dest().get_file().to_index();
                df > sf && (df - sf) == 2
            }
        }).copied();
    }
    if clean == "O-O-O" || clean == "0-0-0" {
        return legal_moves.iter().find(|m| {
            board.piece_on(m.get_source()) == Some(Piece::King) && {
                let sf = m.get_source().get_file().to_index();
                let df = m.get_dest().get_file().to_index();
                sf > df && (sf - df) == 2
            }
        }).copied();
    }

    let bytes = clean.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let (piece, rest) = if bytes[0].is_ascii_uppercase() {
        let p = match bytes[0] {
            b'K' => Piece::King,
            b'Q' => Piece::Queen,
            b'R' => Piece::Rook,
            b'B' => Piece::Bishop,
            b'N' => Piece::Knight,
            _ => return None,
        };
        (p, &clean[1..])
    } else {
        (Piece::Pawn, clean)
    };

    // Promotion
    let (rest, promotion) = if let Some(eq_pos) = rest.find('=') {
        let promo = match rest.as_bytes().get(eq_pos + 1) {
            Some(b'Q') => Some(Piece::Queen),
            Some(b'R') => Some(Piece::Rook),
            Some(b'B') => Some(Piece::Bishop),
            Some(b'N') => Some(Piece::Knight),
            _ => None,
        };
        (&rest[..eq_pos], promo)
    } else {
        (rest, None)
    };

    let rest = rest.replace('x', "");
    let rb = rest.as_bytes();
    if rb.len() < 2 {
        return None;
    }

    let dest_file = rb[rb.len() - 2];
    let dest_rank = rb[rb.len() - 1];
    if !(b'a'..=b'h').contains(&dest_file) || !(b'1'..=b'8').contains(&dest_rank) {
        return None;
    }

    let dest = chess::Square::make_square(
        chess::Rank::from_index((dest_rank - b'1') as usize),
        chess::File::from_index((dest_file - b'a') as usize),
    );

    let disambig = &rest[..rest.len() - 2];

    let mut candidates: Vec<ChessMove> = legal_moves
        .into_iter()
        .filter(|m| {
            m.get_dest() == dest
                && board.piece_on(m.get_source()) == Some(piece)
                && m.get_promotion() == promotion
        })
        .collect();

    if candidates.len() == 1 {
        return Some(candidates[0]);
    }

    if !disambig.is_empty() {
        let db = disambig.as_bytes();
        candidates.retain(|m| {
            let src = m.get_source();
            for &b in db {
                if (b'a'..=b'h').contains(&b) {
                    if src.get_file().to_index() != (b - b'a') as usize {
                        return false;
                    }
                } else if (b'1'..=b'8').contains(&b) {
                    if src.get_rank().to_index() != (b - b'1') as usize {
                        return false;
                    }
                }
            }
            true
        });
    }

    if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        None
    }
}
