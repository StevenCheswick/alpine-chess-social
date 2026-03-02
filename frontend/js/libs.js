// ══════════════════════════════════════════
// CHESSGROUND + CHESS.JS (loaded async)
// ══════════════════════════════════════════
let Chessground = null;
let Chess = null;
let _cgInstance = null; // current chessground board instance
let _chessInstance = null; // chess.js game for replaying moves
let _analysisMoveIndex = -1; // -1 = starting position
let _analysisPositions = []; // FEN at each half-move index
let _analysisMoveCount = 0;

(async function loadChessLibs() {
  try {
    const [cgMod, chessMod] = await Promise.all([
      import('https://esm.sh/chessground@9.2.1'),
      import('https://esm.sh/chess.js@1.0.0-beta.8'),
    ]);
    Chessground = cgMod.Chessground;
    Chess = chessMod.Chess;
    console.log('Chessground + chess.js loaded');
  } catch (err) {
    console.error('Failed to load chess libs:', err);
  }
})();
