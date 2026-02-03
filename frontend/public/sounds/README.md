# Chess Move Sound Effects

Place your chess move sound files in this directory:

## Required Sound Files

- `move.mp3` - Regular piece move sound
- `capture.mp3` - Piece capture sound
- `check.mp3` - Check sound
- `promotion.mp3` - Promotion sound

## File Format

- Format: MP3 (recommended) or any browser-supported audio format
- Recommended: Short, crisp sounds (100-300ms)
- Volume: Normalized to similar levels

## How It Works

The frontend will automatically:
1. Detect move type (regular, capture, check, promotion)
2. Play the appropriate sound when user navigates moves
3. Sounds only play on manual navigation (not during initialization)

## Priority Order

When a move has multiple characteristics, sounds play in this priority:
1. **Promotion** (highest priority)
2. **Check/Checkmate**
3. **Capture**
4. **Regular move** (default)

Example: A move that is both a capture and check will play the check sound.








