/** Display names for camelCase cook() theme tags */
export const TAG_DISPLAY: Record<string, string> = {
  fork: 'Fork', pin: 'Pin', skewer: 'Skewer',
  deflection: 'Deflection', attraction: 'Attraction',
  interference: 'Interference', intermezzo: 'Intermezzo',
  clearance: 'Clearance', discoveredAttack: 'Discovered Attack',
  discoveredCheck: 'Discovered Check', doubleCheck: 'Double Check',
  xRayAttack: 'X-Ray Attack', windmill: 'Windmill',
  sacrifice: 'Sacrifice', capturingDefender: 'Capturing Defender',
  hangingPiece: 'Hanging Piece', trappedPiece: 'Trapped Piece',
  overloading: 'Overloading', exposedKing: 'Exposed King',
  kingsideAttack: 'Kingside Attack', queensideAttack: 'Queenside Attack',
  attackingF2F7: 'Attacking f2/f7', advancedPawn: 'Advanced Pawn',
  promotion: 'Promotion', underPromotion: 'Under-Promotion',
  enPassant: 'En Passant', castling: 'Castling',
  defensiveMove: 'Defensive Move', quietMove: 'Quiet Move',
  zugzwang: 'Zugzwang', greekGift: 'Greek Gift',
  backRankMate: 'Back Rank Mate', smotheredMate: 'Smothered Mate',
  anastasiaMate: 'Anastasia Mate', arabianMate: 'Arabian Mate',
  bodenMate: 'Boden Mate', dovetailMate: 'Dovetail Mate',
  doubleBishopMate: 'Double Bishop Mate', balestraMate: 'Balestra Mate',
  blindSwineMate: 'Blind Swine Mate', cornerMate: 'Corner Mate',
  hookMate: 'Hook Mate', killBoxMate: 'Kill Box Mate',
  morphysMate: "Morphy's Mate", operaMate: 'Opera Mate',
  pillsburysMate: "Pillsbury's Mate", triangleMate: 'Triangle Mate',
  vukovicMate: 'Vukovic Mate', doubleCheckmate: 'Double Checkmate',
  mate: 'Mate', mateIn1: 'Mate in 1', mateIn2: 'Mate in 2',
  mateIn3: 'Mate in 3', mateIn4: 'Mate in 4', mateIn5: 'Mate in 5',
  oneMove: 'One Move', short: 'Short', long: 'Long', veryLong: 'Very Long',
  advantage: 'Advantage', crushing: 'Crushing', equality: 'Equality',
  pawnEndgame: 'Pawn Endgame', knightEndgame: 'Knight Endgame',
  bishopEndgame: 'Bishop Endgame', rookEndgame: 'Rook Endgame',
  queenEndgame: 'Queen Endgame', queenRookEndgame: 'Queen + Rook Endgame',
  // Titled opponent tags
  titled: 'Titled',
  GM: 'GM', IM: 'IM', FM: 'FM', CM: 'CM', NM: 'NM',
  WGM: 'WGM', WIM: 'WIM', WFM: 'WFM', WCM: 'WCM', WNM: 'WNM',
  // FCE endgame segment types
  'Pawn Endings': 'Pawn Endings', 'Knight Endings': 'Knight Endings',
  'Bishop Endings': 'Bishop Endings', 'Bishop vs Knight': 'Bishop vs Knight',
  'Rook Endings': 'Rook Endings', 'Rook vs Minor Piece': 'Rook vs Minor Piece',
  'Rook + Minor vs Rook + Minor': 'Rook+Minor vs Rook+Minor',
  'Rook + Minor vs Rook': 'Rook+Minor vs Rook',
  'Queen Endings': 'Queen Endings', 'Queen vs Rook': 'Queen vs Rook',
  'Queen vs Minor Piece': 'Queen vs Minor Piece',
  'Queen + Piece vs Queen': 'Queen+Piece vs Queen',
};

/** Tags shown on the games page (source, result, titled opponents) */
const GAME_PAGE_TAGS = new Set([
  'Chess.com', 'Lichess',
  'Win', 'Loss', 'Draw',
  'titled', 'GM', 'IM', 'FM', 'CM', 'NM',
  'WGM', 'WIM', 'WFM', 'WCM', 'WNM',
]);

/** Filter for games page — only show game-level tags, not puzzle tactic tags */
export function isGameTag(tag: string): boolean {
  return GAME_PAGE_TAGS.has(tag);
}

/** Filter for puzzle page — show tactic/theme tags, hide meta tags */
const HIDDEN_PUZZLE_TAGS = new Set([
  // Evaluation
  'mate', 'crushing', 'advantage', 'equality',
  // Puzzle length
  'oneMove', 'short', 'long', 'veryLong',
  // Endgame types
  'pawnEndgame', 'knightEndgame', 'bishopEndgame',
  'rookEndgame', 'queenEndgame', 'queenRookEndgame',
  // Game-level tags that don't belong on puzzles
  'Chess.com', 'Lichess', 'Win', 'Loss', 'Draw', 'titled',
  'GM', 'IM', 'FM', 'CM', 'NM', 'WGM', 'WIM', 'WFM', 'WCM', 'WNM',
]);

export function isVisibleTag(tag: string): boolean {
  return !HIDDEN_PUZZLE_TAGS.has(tag);
}

export function tagDisplayName(tag: string): string {
  return TAG_DISPLAY[tag] || tag;
}
