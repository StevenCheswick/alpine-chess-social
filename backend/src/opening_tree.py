"""
Opening tree builder for chess repertoire analysis.
"""
import chess
from typing import Dict, List, Any, Optional


def build_opening_tree(games: List[Dict[str, Any]], max_depth: int = 15) -> Dict[str, Any]:
    """Build a tree structure from a list of games.

    Args:
        games: List of games with 'moves' (list of SAN moves) and 'result' (W/L/D)
        max_depth: Maximum depth in full moves (15 = 30 half-moves)

    Returns:
        Tree structure with stats at each node
    """
    # Root node represents the starting position
    root = {
        "move": "start",
        "fen": chess.STARTING_FEN,
        "games": 0,
        "wins": 0,
        "losses": 0,
        "draws": 0,
        "children": {},
    }

    for game in games:
        result = game.get("result", "D")  # W, L, D
        moves = game.get("moves", [])

        # Limit to max_depth full moves (2 * max_depth half-moves)
        moves = moves[: max_depth * 2]

        # Walk through moves and build tree
        current_node = root
        board = chess.Board()

        for move_san in moves:
            try:
                # Make the move on the board
                move = board.parse_san(move_san)
                board.push(move)
                fen = board.fen()

                # Create child node if doesn't exist
                if move_san not in current_node["children"]:
                    current_node["children"][move_san] = {
                        "move": move_san,
                        "fen": fen,
                        "games": 0,
                        "wins": 0,
                        "losses": 0,
                        "draws": 0,
                        "children": {},
                    }

                # Move to child node
                current_node = current_node["children"][move_san]

                # Increment stats
                current_node["games"] += 1
                if result == "W":
                    current_node["wins"] += 1
                elif result == "L":
                    current_node["losses"] += 1
                else:
                    current_node["draws"] += 1

            except (chess.InvalidMoveError, chess.IllegalMoveError, chess.AmbiguousMoveError):
                # Invalid move, stop processing this game
                break

    return root


def convert_tree_for_response(node: Dict[str, Any]) -> Dict[str, Any]:
    """Convert tree node to API response format (children as array).

    Args:
        node: Tree node with children as dict

    Returns:
        Tree node with children as sorted array
    """
    children = []
    for move, child_node in node["children"].items():
        child_response = convert_tree_for_response(child_node)
        children.append(child_response)

    # Sort children by number of games (most played first)
    children.sort(key=lambda x: x["games"], reverse=True)

    # Calculate win rate
    games = node["games"]
    win_rate = (node["wins"] / games * 100) if games > 0 else 0

    return {
        "move": node["move"],
        "fen": node["fen"],
        "games": node["games"],
        "wins": node["wins"],
        "losses": node["losses"],
        "draws": node["draws"],
        "winRate": round(win_rate, 1),
        "children": children,
    }


def get_node_at_path(root: Dict[str, Any], path: List[str]) -> Optional[Dict[str, Any]]:
    """Get a node at a specific path in the tree.

    Args:
        root: Root node of the tree
        path: List of moves representing the path

    Returns:
        Node at the path, or None if not found
    """
    current = root
    for move in path:
        if move not in current.get("children", {}):
            return None
        current = current["children"][move]
    return current
