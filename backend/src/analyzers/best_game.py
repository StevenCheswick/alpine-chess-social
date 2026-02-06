"""
Unified analyzer for finding the best game based on a scoring system.
Tracks total points per game and finds the highest scoring game.
"""
from typing import List, Dict, Any, Optional
import json
import os
from ..unified_analyzer import UnifiedAnalyzerBase
from ..game_data import GameData


class UnifiedBestGameAnalyzer(UnifiedAnalyzerBase):
    """
    Unified analyzer that tracks game scores and finds the best game.
    Calls get_game_points() on all other analyzers and sums them.
    """
    
    def __init__(self, username: str, config_path: str = "config/game_scoring_config.json"):
        """Initialize with username and config path."""
        super().__init__(username)
        self.config_path = config_path
        self.user_average_elo = None  # Set externally for config selection
        self.config = self._load_config()
        self.best_game_score = float('-inf')  # Allow negative scores
        self.best_game_ref = None
        self.all_game_refs = []  # Store ALL game breakdowns for potential fallback rescore
        self.other_analyzers = []  # Will be set by UnifiedAnalyzer
    
    def _load_config(self) -> dict:
        """
        Load scoring configuration from JSON file.
        Selects appropriate config based on user's average ELO.
        """
        # Check for ELO-specific config files
        config_path = self.config_path

        if self.user_average_elo is not None:
            # Check low_elo config
            low_elo_path = "config/game_scoring_config_low_elo.json"
            if os.path.exists(low_elo_path):
                with open(low_elo_path, 'r', encoding='utf-8') as f:
                    low_elo_config = json.load(f)
                threshold = low_elo_config.get("elo_threshold", 600)
                if self.user_average_elo < threshold:
                    return low_elo_config

        if not os.path.exists(config_path):
            # Return default config if file doesn't exist
            return {
                "rook_sacrifice": 20,
                "queen_sacrifice": 30,
                "smothered_mate": 50,
                "castle_mate": 50,
                "king_mate": 40,
                "pawn_mate": 40,
                "windmill": 25,
                "knight_fork": {"base_points": 15, "per_piece_value": 2},
                "capture_sequence": {"base_points": 10, "per_capture": 2},
                "hung_queen": -5,
                "rare_moves": 5
            }

        with open(config_path, 'r', encoding='utf-8') as f:
            return json.load(f)
    
    def set_other_analyzers(self, analyzers: List[UnifiedAnalyzerBase]):
        """Set the list of other analyzers to get points from."""
        self.other_analyzers = analyzers

    def set_user_average_elo(self, average_elo: float):
        """
        Set user's average ELO and reload config based on it.
        Called externally after initialization.
        """
        self.user_average_elo = average_elo
        self.config = self._load_config()  # Reload config with new ELO

    def rescore_with_fallback_config(self):
        """
        Re-score all games using fallback config for users without sacrifices.
        Called after analysis if no queen/rook sacrifices found and ELO > threshold.

        This re-applies point values from fallback config to stored breakdowns
        and selects a new best game. Very fast - O(n) simple math operations.
        """
        fallback_path = "config/game_scoring_config_fallback.json"
        if not os.path.exists(fallback_path):
            return  # No fallback config, keep original scoring

        with open(fallback_path, 'r', encoding='utf-8') as f:
            fallback_config = json.load(f)

        # Check ELO threshold - only apply fallback if user is above threshold
        elo_threshold = fallback_config.get("elo_threshold", 600)
        if self.user_average_elo is not None and self.user_average_elo < elo_threshold:
            return  # Low ELO users keep original scoring

        # Re-score all games with fallback config
        self.best_game_score = float('-inf')  # Allow negative scores
        self.best_game_ref = None

        for game_ref in self.all_game_refs:
            breakdown = game_ref["breakdown"]

            # Recalculate total using fallback config point values
            new_total = 0
            new_breakdown = {}

            for key, original_points in breakdown.items():
                # Get the fallback config value for this key
                fallback_value = fallback_config.get(key)

                if fallback_value is not None:
                    # Handle simple integer values
                    if isinstance(fallback_value, (int, float)):
                        new_points = fallback_value if original_points != 0 else 0
                        # Preserve sign for penalties (e.g., hung_queen, bullet_penalty)
                        if original_points < 0:
                            new_points = -abs(fallback_value) if fallback_value > 0 else fallback_value
                        new_total += new_points
                        new_breakdown[key] = new_points
                    else:
                        # Complex config (thresholds, etc.) - keep original points
                        new_total += original_points
                        new_breakdown[key] = original_points
                else:
                    # Key not in fallback config - keep original
                    new_total += original_points
                    new_breakdown[key] = original_points

            # Update game ref with new scoring
            game_ref["total_points"] = new_total
            game_ref["breakdown"] = new_breakdown

            # Track new best
            if new_total > self.best_game_score:
                self.best_game_score = new_total
                self.best_game_ref = game_ref

        # Update config reference for get_final_results
        self.config = fallback_config

    def start_game(self, game_data: GameData, user_is_white: bool, user_is_black: bool):
        """Initialize analyzer state for a new game."""
        super().start_game(game_data, user_is_white, user_is_black)
        self.current_game_points = 0
        self.current_game_breakdown = {}
    
    def process_move(self, context):
        """No processing needed - we calculate points in finish_game()."""
        pass
    
    def finish_game(self) -> List[Dict[str, Any]]:
        """Called by UnifiedAnalyzer - we do nothing here, scoring happens in finish_game_scoring()."""
        return []
    
    def finish_game_scoring(self):
        """
        Calculate total points for this game by summing points from all analyzers.
        Track if this is the best game so far.
        
        Called AFTER all other analyzers' finish_game() methods have been called.
        """
        # Calculate points from all other analyzers
        total_points = 0
        breakdown = {}
        
        for analyzer in self.other_analyzers:
            if analyzer == self:
                continue  # Skip ourselves

            analyzer_name = analyzer.__class__.__name__
            # Convert class name to config key (e.g., "UnifiedRookSacrificeAnalyzer" -> "rook_sacrifice")
            config_key = self._get_config_key(analyzer_name)

            if config_key:
                points = analyzer.get_game_points(self.config)
                if points != 0:  # Include both positive and negative points
                    total_points += points
                    breakdown[config_key] = points

        # Check if user won
        result = self.game_data.metadata.result
        user_won = (result == "1-0" and self.user_is_white) or (result == "0-1" and self.user_is_black)
        
        # Add checkmate win bonus (O(1) operation - very fast)
        if user_won and self.game_data.moves and '#' in self.game_data.moves[-1]:
            checkmate_bonus = self.config.get("checkmate_win", 0)
            if checkmate_bonus:
                total_points += checkmate_bonus
                breakdown["checkmate_win"] = checkmate_bonus
        
        # Add game speed bonus - bonus/penalty based on when the game ended (O(1) operations - very fast)
        if user_won and self.game_data.moves:
            full_moves = (len(self.game_data.moves) + 1) // 2  # Convert half-moves to full moves
            
            # Get game speed thresholds from config
            # Try both old key name (for backward compatibility) and new key name
            game_speed_config = self.config.get("game_ends_before_moves") or self.config.get("checkmate_before_moves", {})
            if game_speed_config and "thresholds" in game_speed_config:
                thresholds = game_speed_config["thresholds"]
                # Sort by moves ASCENDING (lowest move count first)
                # This ensures we check thresholds in order from most restrictive to least
                # For a 15-move game, we check <20 before <30, so negative penalties apply correctly
                for threshold in sorted(thresholds, key=lambda x: x["moves"]):
                    if full_moves < threshold["moves"]:
                        bonus = threshold["points"]
                        total_points += bonus
                        breakdown["game_speed"] = bonus
                        break  # Use first (most restrictive) matching threshold

        # Apply bullet penalty (configurable)
        bullet_config = self.config.get("bullet_penalty", {})
        if bullet_config.get("enabled", False):
            max_base_time = bullet_config.get("max_base_time_seconds", 179)
            penalty = bullet_config.get("points", -20)

            # Get base time from time_control (handles "60", "180", "180+2" formats)
            time_control = getattr(self.game_data.metadata, "time_control", None)
            if time_control:
                try:
                    # Parse base time (first number before any "+")
                    base_time = int(str(time_control).split("+")[0])
                    if base_time <= max_base_time:
                        total_points += penalty
                        breakdown["bullet_penalty"] = penalty
                except (ValueError, TypeError):
                    pass

        # Apply mating piece bonus (configurable per-piece bonus for checkmates)
        mating_piece_config = self.config.get("mating_piece_bonus")
        if mating_piece_config and user_won and self.game_data.moves and '#' in self.game_data.moves[-1]:
            last_move = self.game_data.moves[-1]
            piece = self._get_mating_piece(last_move)
            if piece:
                bonus = mating_piece_config.get(piece, 0)
                if bonus:
                    total_points += bonus
                    breakdown["mating_piece_bonus"] = bonus

        # Apply sacrifice comeback bonus OR material deficit penalty
        # These are mutually exclusive based on whether a sacrifice was found
        if user_won:
            has_sacrifice = self._check_has_sacrifice()
            material_deficit = self._get_material_deficit()

            if has_sacrifice and material_deficit > 0:
                # Sacrifice + deficit = intentional brilliance, apply comeback bonus
                comeback_bonus = self._get_sacrifice_comeback_bonus(material_deficit)
                if comeback_bonus > 0:
                    total_points += comeback_bonus
                    breakdown["sacrifice_comeback"] = comeback_bonus
            elif not has_sacrifice and material_deficit > 0:
                # No sacrifice + deficit = sloppy play, apply penalty
                penalty = self._get_material_deficit_penalty(material_deficit)
                if penalty != 0:
                    total_points += penalty
                    breakdown["material_deficit_penalty"] = penalty

        # Store this game's breakdown for potential fallback rescore
        game_ref = {
            "game_data": self.game_data,
            "total_points": total_points,
            "breakdown": breakdown,
            "user_is_white": self.user_is_white
        }
        self.all_game_refs.append(game_ref)

        # Track best game
        if total_points > self.best_game_score:
            self.best_game_score = total_points
            self.best_game_ref = game_ref
    
    def _get_config_key(self, analyzer_name: str) -> Optional[str]:
        """Convert analyzer class name to config key."""
        # Remove "Unified" prefix and "Analyzer" suffix, convert to snake_case
        name = analyzer_name.replace("Unified", "").replace("Analyzer", "")
        # Convert CamelCase to snake_case
        import re
        name = re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower()
        
        # Map to config keys
        mapping = {
            "rook_sacrifice": "rook_sacrifice",
            "queen_sacrifice": "queen_sacrifice",
            "capture_sequence": "capture_sequence",
            "king_mate": "king_mate",
            "smothered_mate": "smothered_mate",
            "rare_moves": "rare_moves",
            "hung_queen": "hung_queen",
            "windmill": "windmill",
            "knight_fork": "knight_fork",
            "castle_mate": "castle_mate",
            "pawn_mate": "pawn_mate",
            "knight_promotion_mate": "knight_promotion_mate",
            "promotion_mate": "promotion_mate",
            "en_passant_mate": "en_passant_mate",
            "king_walk": "king_walk",
            "biggest_comeback": "biggest_comeback"
        }
        
        return mapping.get(name)

    def _check_has_sacrifice(self) -> bool:
        """Check if current game has a queen or rook sacrifice."""
        for analyzer in self.other_analyzers:
            analyzer_name = analyzer.__class__.__name__
            if "QueenSacrifice" in analyzer_name or "RookSacrifice" in analyzer_name:
                if getattr(analyzer, 'sacrifice_found', False):
                    return True
        return False

    def _get_material_deficit(self) -> int:
        """Get material deficit from biggest_comeback analyzer for current game."""
        for analyzer in self.other_analyzers:
            analyzer_name = analyzer.__class__.__name__
            if "BiggestComeback" in analyzer_name:
                return getattr(analyzer, 'current_game_deficit', 0)
        return 0

    def _get_sacrifice_comeback_bonus(self, deficit: int) -> int:
        """
        Calculate comeback bonus for sacrifice games based on material deficit.
        Only applies when a queen/rook sacrifice was detected.
        """
        comeback_config = self.config.get("sacrifice_comeback_bonus", {})
        thresholds = comeback_config.get("thresholds", [])

        if not thresholds:
            return 0

        # Sort thresholds by deficit descending to find highest matching tier
        sorted_thresholds = sorted(thresholds, key=lambda x: x.get("deficit", 0), reverse=True)

        for threshold in sorted_thresholds:
            if deficit >= threshold.get("deficit", 0):
                return threshold.get("points", 0)

        return 0

    def _get_material_deficit_penalty(self, deficit: int) -> int:
        """
        Calculate penalty for being down material without a sacrifice.
        Indicates sloppy play rather than intentional brilliance.
        """
        penalty_config = self.config.get("material_deficit_penalty", {})
        threshold = penalty_config.get("threshold", 3)
        penalty = penalty_config.get("points", 0)

        if deficit >= threshold:
            return penalty
        return 0

    def _get_mating_piece(self, move: str) -> Optional[str]:
        """Determine which piece delivered checkmate from move notation.

        Returns: 'knight', 'bishop', 'rook', 'queen', or None for pawn/king/castling
        """
        if not move:
            return None

        # Remove check/mate symbols and capture notation for cleaner parsing
        first_char = move[0]

        if first_char == 'N':
            return "knight"
        elif first_char == 'B':
            return "bishop"
        elif first_char == 'R':
            return "rook"
        elif first_char == 'Q':
            return "queen"
        # Pawn, King, and Castling are handled by other analyzers (pawn_mate, king_mate, castle_mate)
        return None

    def get_matched_game_links(self) -> List[str]:
        """Fast path: return just the game links that matched."""
        return [ref["game_data"].metadata.link for ref in self.all_game_refs 
                if ref.get("game_data") and ref["game_data"].metadata.link]

    def get_final_results(self) -> List[Dict[str, Any]]:
        """
        Get final results - the best game.
        
        Returns:
            List with single best game finding
        """
        if not self.best_game_ref:
            return []
        
        ref = self.best_game_ref
        game_data = ref["game_data"]
        
        # Extract ELO
        import re
        pgn = game_data.pgn
        white_elo = None
        black_elo = None
        
        white_match = re.search(r'\[WhiteElo\s+"(\d+)"\]', pgn)
        if white_match:
            try:
                white_elo = int(white_match.group(1))
            except ValueError:
                pass
        
        black_match = re.search(r'\[BlackElo\s+"(\d+)"\]', pgn)
        if black_match:
            try:
                black_elo = int(black_match.group(1))
            except ValueError:
                pass
        
        # Key move is the first move (move 1) to highlight the entire game
        key_move_number = 1
        
        # Build replay data structure for frontend (uses config file)
        from ..replay_helper import build_replay_data
        # Key position is 1 move into the game (instead of 0 for starting position)
        key_half_move = min(1, len(game_data.moves)) if game_data.moves else 0
        replay_data = build_replay_data(
            game_data=game_data,
            key_half_move=key_half_move,
            feature_name="best_game"  # Loads settings from replay_config.json
        )
        
        finding = {
            "feature_name": "best_game",
            "display_name": "Best Game",
            "game_metadata": {
                "white": game_data.metadata.white,
                "black": game_data.metadata.black,
                "result": game_data.metadata.result,
                "date": game_data.metadata.date,
                "link": game_data.metadata.link,
                "white_elo": white_elo,
                "black_elo": black_elo,
                "user_color": "white" if ref["user_is_white"] else "black",
                # New simplified format for frontend navigation
                "all_moves": replay_data["all_moves"],
                "key_position_index": replay_data["key_position_index"],
                "fen": replay_data["fen"],
                # Include full PGN - contains all clock annotations/timestamps
                "pgn": game_data.pgn
            },
            "position_link": f"{game_data.metadata.link}?move={key_move_number}" if game_data.metadata.link else None,
            "result_data": {
                "total_points": {
                    "value": ref["total_points"],
                    "label": "Total Points"
                },
                "breakdown": {
                    "value": ref.get("breakdown", {}),
                    "label": "Score Breakdown"
                }
            }
        }

        return [finding]

