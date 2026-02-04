"""Chess game analyzers."""

from .queen_sacrifice import UnifiedQueenSacrificeAnalyzer
from .knight_fork import UnifiedKnightForkAnalyzer
from .rook_sacrifice import UnifiedRookSacrificeAnalyzer
from .back_rank_mate import UnifiedBackRankMateAnalyzer
from .best_game import UnifiedBestGameAnalyzer
from .biggest_comeback import UnifiedBiggestComebackAnalyzer
from .capture_sequence import UnifiedCaptureSequenceAnalyzer
from .castle_mate import UnifiedCastleMateAnalyzer
from .clutch_win import UnifiedClutchWinAnalyzer
from .en_passant_mate import UnifiedEnPassantMateAnalyzer
# from .favorite_gambit import UnifiedFavoriteGambitAnalyzer  # TODO: needs gambit_tree_fast module
from .hung_queen import UnifiedHungQueenAnalyzer
from .king_mate import UnifiedKingMateAnalyzer
from .king_walk import UnifiedKingWalkAnalyzer
from .knight_bishop_mate import UnifiedKnightBishopMateAnalyzer
from .knight_promotion_mate import UnifiedKnightPromotionMateAnalyzer
from .longest_game import UnifiedLongestGameAnalyzer
from .pawn_mate import UnifiedPawnMateAnalyzer
from .promotion_mate import UnifiedPromotionMateAnalyzer
from .quickest_mate import UnifiedQuickestMateAnalyzer
# from .rare_moves import UnifiedRareMovesAnalyzer  # TODO: needs rare_moves module
# from .signature_opening import UnifiedSignatureOpeningAnalyzer  # TODO: needs opening_tree module
from .smothered_mate import UnifiedSmotheredMateAnalyzer
from .stalemate import UnifiedStalemateAnalyzer
from .windmill import UnifiedWindmillAnalyzer

__all__ = [
    'UnifiedQueenSacrificeAnalyzer',
    'UnifiedKnightForkAnalyzer',
    'UnifiedRookSacrificeAnalyzer',
    'UnifiedBackRankMateAnalyzer',
    'UnifiedBestGameAnalyzer',
    'UnifiedBiggestComebackAnalyzer',
    'UnifiedCaptureSequenceAnalyzer',
    'UnifiedCastleMateAnalyzer',
    'UnifiedClutchWinAnalyzer',
    'UnifiedEnPassantMateAnalyzer',
    # 'UnifiedFavoriteGambitAnalyzer',
    'UnifiedHungQueenAnalyzer',
    'UnifiedKingMateAnalyzer',
    'UnifiedKingWalkAnalyzer',
    'UnifiedKnightBishopMateAnalyzer',
    'UnifiedKnightPromotionMateAnalyzer',
    'UnifiedLongestGameAnalyzer',
    'UnifiedPawnMateAnalyzer',
    'UnifiedPromotionMateAnalyzer',
    'UnifiedQuickestMateAnalyzer',
    # 'UnifiedRareMovesAnalyzer',
    # 'UnifiedSignatureOpeningAnalyzer',
    'UnifiedSmotheredMateAnalyzer',
    'UnifiedStalemateAnalyzer',
    'UnifiedWindmillAnalyzer',
]
