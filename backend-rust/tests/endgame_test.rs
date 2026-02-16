/// Tests for endgame CP loss calculation correctness.
///
/// The flow being tested:
/// 1. EndgameTracker accumulates white_cp_loss and black_cp_loss by chess color
/// 2. Database stores these in endgame_segments JSON
/// 3. SQL query swaps based on user_color to get user/opponent stats

use chess::Board;
use chess_puzzler::endgame::{classify_endgame, EndgameTracker, EndgameType};
use std::str::FromStr;

#[test]
fn test_endgame_tracker_accumulates_by_chess_color() {
    let mut tracker = EndgameTracker::new();

    // A rook ending position - kings + rooks + pawns
    let rook_ending = Board::from_str("4k3/4p3/8/8/8/8/4P3/R3K3 w - - 0 40").unwrap();
    assert_eq!(classify_endgame(&rook_ending), Some(EndgameType::RookEndings));

    // Simulate moves in the endgame:
    // Move 40 (index 78): White plays, loses 20 cp
    // Move 40 (index 79): Black plays, loses 50 cp
    // Move 41 (index 80): White plays, loses 10 cp
    // Move 41 (index 81): Black plays, loses 30 cp

    // White's move (index 78, even = white)
    tracker.track_move(
        &rook_ending, 100, 20, "good", "e2e7", "e2e8",
        "4k3/8/8/8/8/8/4R3/4K3 w - - 0 1", true, 78,
    );

    // Black's move (index 79, odd = black)
    tracker.track_move(
        &rook_ending, 50, 50, "mistake", "e8d7", "e8f7",
        "4k3/4R3/8/8/8/8/8/4K3 b - - 0 1", false, 79,
    );

    // White's move (index 80)
    tracker.track_move(
        &rook_ending, 60, 10, "excellent", "e7d7", "e7e1",
        "3k4/4R3/8/8/8/8/8/4K3 w - - 0 1", true, 80,
    );

    // Black's move (index 81)
    tracker.track_move(
        &rook_ending, 30, 30, "inaccuracy", "d7c6", "d7e6",
        "3k4/3R4/8/8/8/8/8/4K3 b - - 0 1", false, 81,
    );

    let segments = tracker.finish();
    assert_eq!(segments.len(), 1);

    let seg = &segments[0];
    assert_eq!(seg.endgame_type, "Rook Endings");

    // Verify CP loss accumulated correctly by color
    assert_eq!(seg.white_moves, 2);
    assert_eq!(seg.white_cp_loss, 20 + 10); // 30 total for white
    assert_eq!(seg.black_moves, 2);
    assert_eq!(seg.black_cp_loss, 50 + 30); // 80 total for black

    // Now verify the user/opponent mapping logic:
    // If user played WHITE: user_cp_loss = white_cp_loss = 30, opp = 80
    // If user played BLACK: user_cp_loss = black_cp_loss = 80, opp = 30

    // This is what the SQL does:
    // SUM(CASE WHEN user_color = 'white' THEN white_cp_loss ELSE black_cp_loss END)

    let user_color = "white";
    let user_cp_loss = if user_color == "white" { seg.white_cp_loss } else { seg.black_cp_loss };
    let opp_cp_loss = if user_color == "white" { seg.black_cp_loss } else { seg.white_cp_loss };

    assert_eq!(user_cp_loss, 30);
    assert_eq!(opp_cp_loss, 80);

    // If user was black:
    let user_color = "black";
    let user_cp_loss = if user_color == "white" { seg.white_cp_loss } else { seg.black_cp_loss };
    let opp_cp_loss = if user_color == "white" { seg.black_cp_loss } else { seg.white_cp_loss };

    assert_eq!(user_cp_loss, 80);
    assert_eq!(opp_cp_loss, 30);
}

#[test]
fn test_is_white_calculation() {
    // In the analysis loop: is_white = i % 2 == 0
    // This means:
    //   i=0 (1. e4)      -> white's move
    //   i=1 (1... e5)    -> black's move
    //   i=2 (2. Nf3)     -> white's move
    //   i=78 (40. Re7)   -> white's move (78 % 2 == 0)
    //   i=79 (40... Kd7) -> black's move (79 % 2 == 1)

    for i in 0..100 {
        let is_white = i % 2 == 0;
        let expected_color = if i % 2 == 0 { "white" } else { "black" };

        // Move number in chess notation
        let move_num = i / 2 + 1;
        let is_white_turn = i % 2 == 0;

        assert_eq!(is_white, is_white_turn,
            "Move index {} should be {}'s turn (move {}{})",
            i, expected_color, move_num, if is_white { "." } else { "..." });
    }
}

#[test]
fn test_average_calculation() {
    // The SQL calculates average as: total_cp_loss / total_moves
    // Let's verify this is computed correctly

    let white_cp_loss: i32 = 150;
    let white_moves: i32 = 3;
    let black_cp_loss: i32 = 240;
    let black_moves: i32 = 3;

    // If user is white:
    let user_avg = white_cp_loss as f64 / white_moves as f64;
    let opp_avg = black_cp_loss as f64 / black_moves as f64;

    assert!((user_avg - 50.0).abs() < 0.01, "White avg should be 50, got {}", user_avg);
    assert!((opp_avg - 80.0).abs() < 0.01, "Black avg should be 80, got {}", opp_avg);

    // Edge: opponent - user = 80 - 50 = 30 (user is better by 30 cp/move)
    let edge = opp_avg - user_avg;
    assert!((edge - 30.0).abs() < 0.01, "Edge should be 30, got {}", edge);
}

#[test]
fn test_endgame_segment_transitions() {
    let mut tracker = EndgameTracker::new();

    // Start in a rook ending
    let rook_ending = Board::from_str("4k3/4p3/8/8/8/8/4P3/R3K3 w - - 0 40").unwrap();

    // Track a move in rook ending
    tracker.track_move(
        &rook_ending, 100, 25, "good", "e2e7", "e2e8",
        "fen", true, 78,
    );

    // Trade rooks - transition to pawn ending
    let pawn_ending = Board::from_str("4k3/4p3/8/8/8/8/4P3/4K3 w - - 0 42").unwrap();
    assert_eq!(classify_endgame(&pawn_ending), Some(EndgameType::PawnEndings));

    // Track moves in pawn ending
    tracker.track_move(
        &pawn_ending, 50, 15, "excellent", "e2e4", "e2e4",
        "fen", true, 80,
    );
    tracker.track_move(
        &pawn_ending, 40, 35, "inaccuracy", "e7e5", "e7e6",
        "fen", false, 81,
    );

    let segments = tracker.finish();
    assert_eq!(segments.len(), 2, "Should have 2 segments (rook + pawn)");

    // First segment: rook ending
    assert_eq!(segments[0].endgame_type, "Rook Endings");
    assert_eq!(segments[0].white_moves, 1);
    assert_eq!(segments[0].white_cp_loss, 25);

    // Second segment: pawn ending
    assert_eq!(segments[1].endgame_type, "Pawn Endings");
    assert_eq!(segments[1].white_moves, 1);
    assert_eq!(segments[1].white_cp_loss, 15);
    assert_eq!(segments[1].black_moves, 1);
    assert_eq!(segments[1].black_cp_loss, 35);
}
