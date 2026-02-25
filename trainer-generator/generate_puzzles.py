#!/usr/bin/env python3
"""
All-in-one puzzle generation pipeline: find mistakes → Maia filter → build trees.

Usage:
  python -u generate_puzzles.py --eco C51,C52 --side black --count 100 -o puzzles.json
"""

import argparse
import asyncio
import chess
import chess.engine
import duckdb
import json
import sys
from pathlib import Path

# ── Constants ────────────────────────────────────────────────────────────────

STOCKFISH_PATH = "../backend-rust/stockfish.exe"
MAIA_DIR = Path("C:/Users/steve/OneDrive/Desktop/maia")
TREE_DB = "move_tree.duckdb"
EVALS_DB = "lichess_evals.duckdb"
SF_NODES = 200_000
MAIA_RATING = 1900
MAIA_OPP_NODES = 100
MAIA_MIN_PROB = 20.0
SOLVER_MAIA_MIN = 5.0
MAX_DEPTH = 30
MAX_NODES = 100
OPP_FLAT = 50

# ── Phase 1: Find Mistakes ──────────────────────────────────────────────────


def games_column_expr(min_rating):
    """Build SQL expression for summing relevant rating bucket columns."""
    if min_rating <= 0:
        return "games"
    buckets = []
    if min_rating <= 0: buckets.append("games_0")
    if min_rating <= 1000: buckets.append("games_1000")
    if min_rating <= 1400: buckets.append("games_1400")
    if min_rating <= 1800: buckets.append("games_1800")
    buckets.append("games_2200")
    return " + ".join(buckets)


def find_mistakes(eco_codes, side, min_rating, threshold):
    """Find common mistakes from move_tree + lichess_evals."""
    tree_con = duckdb.connect(TREE_DB, read_only=False)
    evals_con = duckdb.connect(EVALS_DB, read_only=True)

    games_expr = games_column_expr(min_rating)
    rating_label = f"{min_rating}+" if min_rating > 0 else "all ratings"
    print(f"Phase 1: Finding mistakes")
    print(f"  ECO: {','.join(eco_codes)}")
    print(f"  Side: {side} (mistakes)")
    print(f"  Rating: {rating_label}")
    print(f"  Threshold: {threshold}cp")

    # Get all sequences with FENs and 2+ games in the rating range
    rows = tree_con.execute(f"""
        SELECT eco, sequence, move, ply, ({games_expr}) as filtered_games, fen, eval_cp
        FROM move_tree
        WHERE fen IS NOT NULL AND ({games_expr}) >= 2 AND ply >= 1
        ORDER BY filtered_games DESC
    """).fetchall()

    # Build lookups
    fen_by_key = {}
    eval_by_key = {}
    games_by_key = {}
    for eco, seq, move, ply, filtered_games, fen, eval_cp in rows:
        fen_by_key[(eco, seq)] = fen
        games_by_key[(eco, seq)] = filtered_games
        if eval_cp is not None:
            eval_by_key[(eco, seq)] = eval_cp

    # Include ply-0 rows
    ply0 = tree_con.execute(f"""
        SELECT eco, sequence, fen, eval_cp, ({games_expr}) as filtered_games
        FROM move_tree
        WHERE fen IS NOT NULL AND ply = 0
    """).fetchall()
    for eco, seq, fen, eval_cp, filtered_games in ply0:
        fen_by_key[(eco, seq)] = fen
        games_by_key[(eco, seq)] = filtered_games
        if eval_cp is not None:
            eval_by_key[(eco, seq)] = eval_cp

    has_inline = len(eval_by_key)
    total_keys = len(fen_by_key)
    print(f"  Inline evals: {has_inline:,}/{total_keys:,} ({100*has_inline/total_keys:.1f}%)")

    # Build parent-child pairs
    pairs = []
    for eco, seq, move, ply, filtered_games, fen, eval_cp in rows:
        parts = seq.split("|")
        if len(parts) < 2:
            continue
        parent_seq = "|".join(parts[:-1])
        parent_key = (eco, parent_seq)
        child_key = (eco, seq)
        parent_fen = fen_by_key.get(parent_key)
        if parent_fen is None:
            continue
        pairs.append((parent_key, child_key, move, filtered_games, eco, seq, parent_fen, fen))

    # Find positions needing external eval
    need_external = set()
    for parent_key, child_key, move, games, eco, seq, before_fen, after_fen in pairs:
        if parent_key not in eval_by_key:
            need_external.add(before_fen)
        if child_key not in eval_by_key:
            need_external.add(after_fen)

    # Batch lookup from lichess_evals
    external_evals = {}
    if need_external:
        print(f"  Looking up {len(need_external):,} external evals...")
        fen_list = list(need_external)
        CHUNK = 5000
        for i in range(0, len(fen_list), CHUNK):
            chunk = fen_list[i:i+CHUNK]
            results = evals_con.execute(
                "SELECT DISTINCT ON (fen) fen, cp, mate FROM positions WHERE fen = ANY($1)",
                [chunk]
            ).fetchall()
            for fen, cp, mate in results:
                if mate is not None:
                    external_evals[fen] = 10000 if mate > 0 else -10000
                elif cp is not None:
                    external_evals[fen] = cp
        print(f"  Found {len(external_evals):,}/{len(need_external):,} external evals")

        # Write found evals back to move_tree so future runs don't need external lookup
        writeback = []
        for (eco, seq), fen in fen_by_key.items():
            if (eco, seq) not in eval_by_key and fen in external_evals:
                writeback.append((eco, seq, external_evals[fen]))
        if writeback:
            tree_con.execute("CREATE TEMP TABLE _wb (eco VARCHAR, sequence VARCHAR, eval_cp INTEGER)")
            tree_con.executemany("INSERT INTO _wb VALUES ($1, $2, $3)", writeback)
            tree_con.execute("""
                UPDATE move_tree SET eval_cp = _wb.eval_cp
                FROM _wb WHERE move_tree.eco = _wb.eco AND move_tree.sequence = _wb.sequence
            """)
            tree_con.execute("DROP TABLE _wb")
            print(f"  Wrote {len(writeback):,} evals back to move_tree")

    def get_eval(key, fen):
        if key in eval_by_key:
            return eval_by_key[key]
        return external_evals.get(fen)

    # Find mistakes matching criteria
    eco_set = set(eco_codes)
    mistakes = []
    for parent_key, child_key, move, games, eco, seq, before_fen, after_fen in pairs:
        if eco not in eco_set:
            continue
        eval_before = get_eval(parent_key, before_fen)
        eval_after = get_eval(child_key, after_fen)
        if eval_before is None or eval_after is None:
            continue

        white_to_move = " w " in before_fen
        if white_to_move:
            cp_loss = eval_before - eval_after
        else:
            cp_loss = eval_after - eval_before

        # Position after mistake must be >= 200cp for the solver
        solver_eval = -eval_after if white_to_move else eval_after
        if solver_eval < 200:
            continue

        mistake_side = "white" if white_to_move else "black"
        if mistake_side != side:
            continue

        if cp_loss >= threshold:
            mistakes.append({
                "fen": before_fen,
                "move": move,
                "sequence": seq,
                "eco": eco,
                "cp_loss": cp_loss,
                "games": games,
                "eval_before": eval_before,
                "eval_after": eval_after,
                "side": mistake_side,
            })

    mistakes.sort(key=lambda x: x["games"], reverse=True)

    tree_con.close()
    evals_con.close()

    print(f"  Found {len(mistakes)} mistakes with >= {threshold}cp loss")
    return mistakes


# ── Phase 2: Maia Filter ────────────────────────────────────────────────────


async def maia_filter(mistakes, maia, target=None):
    """Drop mistakes where Maia gives 0% to the mistake move.
    If target is set, stop early once we have enough candidates (with buffer)."""
    stop_at = int(target * 1.5) + 10 if target else None
    print(f"\nPhase 2: Maia filter ({len(mistakes)} candidates{f', need ~{stop_at}' if stop_at else ''})")
    kept = []
    dropped = 0
    for i, m in enumerate(mistakes):
        result = await maia.analyze_fen(m["fen"], top_n=20, nodes=100)
        maia_map = {mv["san"]: mv["probability"] for mv in result["moves"]}
        prob = maia_map.get(m["move"], 0.0)
        if prob > 0:
            m["maia_prob"] = prob
            kept.append(m)
        else:
            dropped += 1
        if (i + 1) % 50 == 0:
            print(f"  [{i+1}/{len(mistakes)}] kept={len(kept)} dropped={dropped}")
        if stop_at and len(kept) >= stop_at:
            print(f"  [{i+1}/{len(mistakes)}] reached {len(kept)} — enough candidates")
            break

    print(f"  After filter: {len(kept)} kept, {dropped} dropped")
    return kept


# ── Phase 3: Generate Trees ─────────────────────────────────────────────────


def score_cp(score, pov_color):
    s = score.pov(pov_color)
    if s.is_mate():
        return 10000 if s.mate() > 0 else -10000
    return s.score()


def _mate_in(score, pov_color):
    s = score.pov(pov_color)
    if s.is_mate() and s.mate() > 0:
        return s.mate()
    return None


async def build_tree(sf, maia, board, solver_color, root_eval, stats, depth=0, linear=False):
    fen = board.fen()
    is_solver = (board.turn == solver_color)

    if stats["nodes"] >= MAX_NODES:
        print(f"  [{stats['nodes']}] CUTOFF max_nodes depth={depth}")
        return {"fen": fen, "type": "cutoff", "reason": "max_nodes"}
    if depth >= MAX_DEPTH:
        print(f"  [{stats['nodes']}] CUTOFF max_depth depth={depth}")
        return {"fen": fen, "type": "cutoff", "reason": "max_depth"}
    if board.is_game_over():
        status = "checkmate" if board.is_checkmate() else "draw"
        print(f"  [{stats['nodes']}] TERMINAL {status} depth={depth}")
        return {"fen": fen, "type": "terminal", "status": status}

    stats["nodes"] += 1
    if depth > stats["max_depth"]:
        stats["max_depth"] = depth

    if is_solver:
        return await _solver_node(sf, maia, board, solver_color, root_eval, stats, depth, linear=linear)
    else:
        return await _opponent_node(sf, maia, board, solver_color, root_eval, stats, depth, linear=linear)


async def _solver_node(sf, maia, board, solver_color, root_eval, stats, depth, linear=False):
    fen = board.fen()

    # MultiPV 3 with Stockfish
    mpv = await sf.analyse(board, chess.engine.Limit(nodes=SF_NODES), multipv=3)
    stats["sf_evals"] += 3
    best_cp = score_cp(mpv[0]["score"], solver_color)
    worst_cp = score_cp(mpv[-1]["score"], solver_color) if len(mpv) >= 3 else None

    print(f"  [{stats['nodes']}] solver depth={depth} best={best_cp} worst3={worst_cp} root={root_eval}{' [linear]' if linear else ''}")

    # Bucketed advantage_secured cutoff
    if depth > 2 and worst_cp is not None and best_cp < 10000:
        cutoff = False
        max_spread = 999
        if worst_cp >= root_eval + 100:
            cutoff = True; max_spread = 999
        elif root_eval >= 625 and worst_cp >= 500: cutoff = True; max_spread = 150
        elif root_eval >= 400 and worst_cp >= root_eval - 125: cutoff = True; max_spread = 100
        elif root_eval >= 300 and worst_cp >= 250: cutoff = True; max_spread = 75
        elif root_eval >= 150 and worst_cp >= root_eval - 50: cutoff = True; max_spread = 50
        if cutoff and (best_cp - worst_cp) > max_spread:
            cutoff = False
        # Top-2 check: if best 2 moves are both above root_eval and within 250cp, position is won
        if not cutoff and len(mpv) >= 2:
            second_cp = score_cp(mpv[1]["score"], solver_color)
            if second_cp >= root_eval and (best_cp - second_cp) <= 250:
                cutoff = True
                print(f"  [{stats['nodes']}] top2 cutoff: best={best_cp} second={second_cp} spread={best_cp - second_cp}")
        if cutoff:
            print(f"  [{stats['nodes']}] CUTOFF secured! worst3={worst_cp} root={root_eval}")
            return {"fen": fen, "type": "cutoff", "reason": "advantage_secured"}

    # In linear mode, only play SF #1 — we're just extending past a capture/check
    if linear:
        move = mpv[0]["pv"][0]
        san = board.san(move)
        cp = best_cp
        uci = move.uci()
        board.push(move)
        child = await build_tree(sf, maia, board, solver_color, root_eval, stats, depth + 1, linear=True)
        board.pop()
        return {"fen": fen, "type": "solver", "moves": {
            uci: {"san": san, "cp": cp, "maia_pct": 0.0, "accepted": True, "result": child}
        }}

    # Accept good MultiPV moves, filtered by Maia human-playability
    best_mate = _mate_in(mpv[0]["score"], solver_color)
    max_accepted = 2 if depth == 0 else 3

    # Maia predictions for solver filtering
    maia_result = await maia.analyze_fen(fen, top_n=10)
    stats["maia_evals"] += 1
    maia_map = {m["san"]: m["probability"] for m in maia_result["moves"]}

    # Collect SF candidates that pass CP_GAP
    candidates = []
    # Dynamic CP_GAP
    if depth == 0: gap = 25
    elif root_eval >= 625: gap = 50
    elif root_eval >= 400: gap = 40
    elif root_eval >= 300: gap = 30
    else: gap = 20

    for pv_info in mpv:
        move = pv_info["pv"][0]
        cp = score_cp(pv_info["score"], solver_color)
        mate = _mate_in(pv_info["score"], solver_color)
        san = board.san(move)
        prob = maia_map.get(san, 0.0)

        if best_mate is not None and mate != best_mate:
            print(f"    SF: {san} cp={cp} maia={prob:.1f}% — REJECTED (mate mismatch)")
            break

        if best_mate is None and best_cp - cp > gap:
            print(f"    SF: {san} cp={cp} maia={prob:.1f}% — REJECTED (gap {best_cp - cp} > {gap})")
            break
        if len(candidates) >= max_accepted:
            print(f"    SF: {san} cp={cp} maia={prob:.1f}% — REJECTED (max {max_accepted})")
            break

        print(f"    SF: {san} cp={cp} maia={prob:.1f}% — ACCEPTED")
        candidates.append((move, san, cp, mate, prob))

    # Maia filter: drop <5% when multiple candidates
    if len(candidates) > 1:
        filtered = [c for c in candidates if c[4] >= SOLVER_MAIA_MIN]
        if filtered:
            dropped = [c[1] for c in candidates if c[4] < SOLVER_MAIA_MIN]
            if dropped:
                print(f"    Maia filter dropped: {dropped}")
            candidates = filtered

    moves = {}
    for move, san, cp, mate, prob in candidates:
        uci = move.uci()
        board.push(move)
        child = await build_tree(sf, maia, board, solver_color, root_eval, stats, depth + 1)
        board.pop()
        moves[uci] = {
            "san": san, "cp": cp, "maia_pct": prob,
            "accepted": True, "result": child,
        }

    return {"fen": fen, "type": "solver", "moves": moves}


async def _opponent_node(sf, maia, board, solver_color, root_eval, stats, depth, linear=False):
    fen = board.fen()

    # Flat-position cutoff: if opponent's top 3 moves are all within OPP_FLAT cp
    # Skip if solver's last move was a capture or check — extend linearly instead
    last_move = board.peek() if board.move_stack else None
    is_check = board.is_check()
    board.pop()
    was_capture = board.is_capture(last_move) if last_move else False
    board.push(last_move)
    skip_flat = is_check or was_capture

    # In linear mode, cut off once we've passed the capture/check
    if linear and not skip_flat:
        print(f"  [{stats['nodes']}] CUTOFF linear complete depth={depth} — no longer capture/check")
        return {"fen": fen, "type": "cutoff", "reason": "advantage_secured"}

    if depth > 2 and not linear:
        mpv = await sf.analyse(board, chess.engine.Limit(nodes=SF_NODES), multipv=3)
        stats["sf_evals"] += 3
        if len(mpv) >= 3:
            opp_evals = [score_cp(pv["score"], solver_color) for pv in mpv]
            spread = max(opp_evals) - min(opp_evals)
            if spread <= OPP_FLAT:
                if skip_flat:
                    # Line was ready to end but last move was capture/check — extend linearly
                    print(f"  [{stats['nodes']}] FLAT skipped (capture/check) depth={depth} — entering linear mode")
                    linear = True
                else:
                    print(f"  [{stats['nodes']}] CUTOFF opponent depth={depth} — flat position (spread={spread}cp)")
                    return {"fen": fen, "type": "cutoff", "reason": "advantage_secured"}

    # Maia opponent moves
    maia_result = await maia.analyze_fen(fen, top_n=10, nodes=MAIA_OPP_NODES)
    stats["maia_evals"] += 1
    all_moves = maia_result["moves"]
    if all_moves:
        if linear:
            maia_moves = [all_moves[0]]  # linear mode: top move only
        else:
            maia_moves = [all_moves[0]]  # top move always included
            extras = [m for m in all_moves[1:] if m["probability"] >= MAIA_MIN_PROB]
            if extras:
                maia_moves.append(extras[0])  # at most 1 extra (2 total)
    else:
        maia_moves = []

    move_summary = [(m["san"], f"{m['probability']}%") for m in maia_moves]
    print(f"  [{stats['nodes']}] opponent depth={depth} maia({MAIA_OPP_NODES}n): {move_summary}{' [linear]' if linear else ''}")

    moves = {}
    for mm in maia_moves:
        move = chess.Move.from_uci(mm["move"])
        san = mm["san"]
        uci = mm["move"]
        # lc0 outputs castling as king-captures-rook (e.g. e8h8); normalize to standard UCI (e8g8)
        if board.is_castling(move):
            rank = chess.square_rank(move.from_square)
            file = 6 if chess.square_file(move.to_square) > chess.square_file(move.from_square) else 2
            uci = chess.square_name(move.from_square) + chess.square_name(chess.square(file, rank))
        prob = mm["probability"]

        board.push(move)
        child = await build_tree(sf, maia, board, solver_color, root_eval, stats, depth + 1, linear=linear)
        board.pop()

        moves[uci] = {"san": san, "probability": prob, "result": child}

    # Trim if all children are cutoffs
    computed_results = [m["result"] for m in moves.values() if "result" in m]
    if computed_results and all(r.get("type") == "cutoff" for r in computed_results):
        print(f"  [{stats['nodes']}] TRIMMED opponent depth={depth} — all children are cutoffs")
        return {"fen": fen, "type": "cutoff", "reason": "advantage_secured"}

    return {"fen": fen, "type": "opponent", "moves": moves}


def count_nodes(node):
    if not isinstance(node, dict) or "moves" not in node:
        return 1
    total = 1
    for m in node["moves"].values():
        if "result" in m:
            total += count_nodes(m["result"])
    return total


def _subtree_depth(node):
    if not isinstance(node, dict) or node.get("type") == "cutoff" or "moves" not in node:
        return 0
    depths = [_subtree_depth(m.get("result", {})) for m in node["moves"].values() if "result" in m]
    return 1 + max(depths) if depths else 0


def _subtree_nodes(node):
    if not isinstance(node, dict) or node.get("type") == "cutoff" or "moves" not in node:
        return 0
    total = 1
    for m in node["moves"].values():
        if "result" in m:
            total += _subtree_nodes(m["result"])
    return total


PRUNE_NODE_RATIO = 3.0
PRUNE_NODE_MIN = 20
PRUNE_DEPTH_GAP = 6
PRUNE_DEPTH_NODE_MIN = 8
PRUNE_PREFER_CLEAN_MIN = 8  # if secondary cutoffs immediately and SF#1 has 8+ more nodes, drop SF#1


def prune_tree(node):
    """Bottom-up pruning of disproportionate secondary solver moves.
    Criteria (either triggers removal):
      - Node ratio: secondary nodes > SF#1 nodes * 3 AND secondary nodes > 20
      - Depth gap: secondary depth - SF#1 depth > 6 AND secondary nodes > 8
    Treat 0-node subtrees as 1 for ratio calculations."""
    if not isinstance(node, dict) or node.get("type") == "cutoff" or "moves" not in node:
        return 0

    # Recurse into all children first (bottom-up)
    pruned = 0
    for m in node["moves"].values():
        if "result" in m:
            pruned += prune_tree(m["result"])

    # Evaluate solver nodes with multiple moves
    if node.get("type") == "solver" and len(node["moves"]) > 1:
        move_list = list(node["moves"].items())
        sf1_result = move_list[0][1].get("result", {})
        sf1_n = max(_subtree_nodes(sf1_result), 1)
        sf1_d = _subtree_depth(sf1_result)

        # Prefer clean win: if a secondary move immediately cutoffs and SF#1 is bloated, drop SF#1
        sf1_key = move_list[0][0]
        sf1_san = move_list[0][1].get("san", sf1_key)
        for key, mdata in move_list[1:]:
            result = mdata.get("result", {})
            if (result.get("type") == "cutoff"
                    and sf1_n - 1 >= PRUNE_PREFER_CLEAN_MIN):
                san = mdata.get("san", key)
                print(f"  PRUNE: {sf1_san} — prefer clean {san} (SF#1 has {sf1_n} nodes, {san} cutoffs immediately)")
                del node["moves"][sf1_key]
                pruned += 1
                break  # SF#1 is gone, don't check further

        if sf1_key in node["moves"]:
            to_remove = []
            for key, mdata in move_list[1:]:
                result = mdata.get("result", {})
                sec_n = max(_subtree_nodes(result), 1)
                sec_d = _subtree_depth(result)

                node_bad = sec_n > sf1_n * PRUNE_NODE_RATIO and sec_n > PRUNE_NODE_MIN
                depth_bad = (sec_d - sf1_d) > PRUNE_DEPTH_GAP and sec_n > PRUNE_DEPTH_NODE_MIN

                if node_bad or depth_bad:
                    san = mdata.get("san", key)
                    reason = []
                    if node_bad:
                        reason.append(f"nodes {sec_n}>{sf1_n}*{PRUNE_NODE_RATIO:.0f}")
                    if depth_bad:
                        reason.append(f"depth gap {sec_d}-{sf1_d}={sec_d - sf1_d}")
                    print(f"  PRUNE: {san} ({', '.join(reason)})")
                    to_remove.append(key)
                    pruned += 1

            for key in to_remove:
                del node["moves"][key]

    return pruned


def trim_opponent_leaves(node):
    """Remove opponent moves whose child is a leaf (solver turn with no moves).
    This ensures every line ends on the solver's move, never the opponent's."""
    if not isinstance(node, dict) or "moves" not in node:
        return 0
    trimmed = 0
    # Recurse first (bottom-up)
    for m in node["moves"].values():
        if "result" in m:
            trimmed += trim_opponent_leaves(m["result"])
    # Only trim opponent nodes
    if node.get("type") == "opponent":
        to_remove = []
        for uci, mdata in node["moves"].items():
            child = mdata.get("result", {})
            # Child is a solver-turn leaf if it has no moves (cutoff/terminal/empty)
            child_moves = child.get("moves", {})
            if not child_moves:
                to_remove.append(uci)
        for uci in to_remove:
            trimmed += 1
            del node["moves"][uci]
    return trimmed


def dedup_puzzles(puzzles):
    """Remove puzzles whose root FEN appears inside another puzzle's tree."""
    def _collect_internal_fens(node, depth=0, fens=None):
        if fens is None:
            fens = set()
        if not isinstance(node, dict):
            return fens
        if depth > 0 and "fen" in node:
            fens.add(node["fen"])
        for m in node.get("moves", {}).values():
            if "result" in m:
                _collect_internal_fens(m["result"], depth + 1, fens)
        return fens

    all_internal = set()
    for p in puzzles:
        all_internal |= _collect_internal_fens(p["tree"])

    kept = []
    for p in puzzles:
        if p["post_mistake_fen"] in all_internal:
            print(f"  DEDUP: {p['id']} — root FEN is inside another puzzle's tree")
        else:
            kept.append(p)
    return kept


# ── Main ─────────────────────────────────────────────────────────────────────


async def generate_puzzles(args):
    eco_codes = [e.strip() for e in args.eco.split(",")]
    side = args.side
    count = args.count
    output = args.output or f"puzzles_{'_'.join(eco_codes)}.json"
    eco_label = ",".join(eco_codes)

    # Phase 1: Find mistakes (DuckDB only)
    mistakes = find_mistakes(eco_codes, side, args.min_rating, args.threshold)
    if not mistakes:
        print("No mistakes found. Exiting.")
        return

    # Start single Maia for phase 2 (lightweight)
    sys.path.insert(0, str(MAIA_DIR))
    from maia_engine import MaiaEngine
    print(f"\nStarting Maia-{MAIA_RATING}...")
    maia = MaiaEngine(MAIA_RATING)
    await maia.start()

    # Phase 2: Maia filter
    mistakes = await maia_filter(mistakes, maia, target=count)
    await maia.close()
    if not mistakes:
        print("No mistakes survived Maia filter. Exiting.")
        return

    # Phase 3: Generate trees with parallel workers + dedup/backfill
    NUM_WORKERS = 12
    print(f"\nPhase 3: Generating puzzle trees (target: {count}, {len(mistakes)} candidates, {NUM_WORKERS} workers)")

    async def _worker(worker_id, queue, results, lock):
        _, sf = await chess.engine.popen_uci(STOCKFISH_PATH)
        await sf.configure({"Threads": 1, "Hash": 256})
        w_maia = MaiaEngine(MAIA_RATING)
        await w_maia.start()

        while True:
            try:
                idx, mistake = queue.get_nowait()
            except asyncio.QueueEmpty:
                break

            sf.send_line("ucinewgame")
            await sf.ping()

            seq_parts = mistake["sequence"].split("|")
            mistake_san = seq_parts[-1]
            opening_sans = seq_parts[:-1]
            eco_code = mistake["eco"]

            board = chess.Board()
            for san in opening_sans:
                board.push_san(san)
            pre_mistake_fen = board.fen()

            mistake_move = board.parse_san(mistake_san)
            mistake_uci = mistake_move.uci()
            board.push(mistake_move)

            solver_color = board.turn
            root_sf = await sf.analyse(board, chess.engine.Limit(nodes=SF_NODES))
            root_eval = score_cp(root_sf["score"], solver_color)

            if root_eval < 200:
                print(f"  [W{worker_id}] #{idx+1} {eco_code}_{mistake_san} SKIPPED — root={root_eval}cp < 200")
                continue

            stats = {"nodes": 0, "max_depth": 0, "sf_evals": 1, "maia_evals": 0}
            tree = await build_tree(sf, w_maia, board, solver_color, root_eval, stats)
            n_before = count_nodes(tree)
            n_pruned = 0
            while True:
                p = prune_tree(tree)
                if p == 0:
                    break
                n_pruned += p
            n_trimmed = trim_opponent_leaves(tree)
            n = count_nodes(tree)

            flag = ""
            if n_pruned > 0: flag += f" (pruned {n_pruned}, was {n_before})"
            if n_trimmed > 0: flag += f" (trimmed {n_trimmed} opp-leaves)"
            if stats["nodes"] >= MAX_NODES: flag += " *** HIT NODE LIMIT ***"
            print(f"  [W{worker_id}] #{idx+1} {eco_code}_{mistake_san} => {n} nodes, depth={stats['max_depth']}{flag}")

            puzzle = {
                "id": f"{eco_code}_{mistake_san}_{idx}",
                "eco": eco_code,
                "mistake_san": mistake_san,
                "mistake_uci": mistake_uci,
                "pre_mistake_fen": pre_mistake_fen,
                "post_mistake_fen": board.fen(),
                "title": f"Punishing {mistake_san} in {eco_label}",
                "solver_color": "w" if solver_color == chess.WHITE else "b",
                "root_eval": root_eval,
                "cp_loss": mistake["cp_loss"],
                "games": mistake["games"],
                "tree": tree,
            }

            async with lock:
                results[idx] = puzzle

        await sf.quit()
        await w_maia.close()

    puzzles = []
    cursor = 0

    while len(puzzles) < count and cursor < len(mistakes):
        need = count - len(puzzles)
        batch = mistakes[cursor:cursor + need]
        print(f"\n--- Batch: generating {len(batch)} (have {len(puzzles)}, need {need}) ---")

        queue = asyncio.Queue()
        for i, m in enumerate(batch):
            queue.put_nowait((cursor + i, m))

        results = {}
        lock = asyncio.Lock()
        workers = [asyncio.create_task(_worker(w, queue, results, lock)) for w in range(NUM_WORKERS)]
        await asyncio.gather(*workers)

        new_puzzles = [results[k] for k in sorted(results.keys())]
        puzzles.extend(new_puzzles)
        cursor += need

        # Dedup after each batch
        before_dedup = len(puzzles)
        puzzles = dedup_puzzles(puzzles)
        if len(puzzles) < before_dedup:
            print(f"  Dedup removed {before_dedup - len(puzzles)}, {len(puzzles)} remain")

        with open(output, "w") as f:
            json.dump(puzzles, f, indent=2)

    # Final output
    print(f"\n{'='*60}")
    print(f"Final: {len(puzzles)} puzzles (target was {count})")
    if len(puzzles) < count:
        print(f"  (ran out of candidates — only {len(mistakes)} available)")

    with open(output, "w") as f:
        json.dump(puzzles, f, indent=2)
    print(f"Wrote {output}")


def main():
    parser = argparse.ArgumentParser(description="Generate opening punishment puzzles")
    parser.add_argument("--eco", required=True,
                        help="ECO codes (comma-separated, e.g. C51,C52)")
    parser.add_argument("--side", required=True, choices=["white", "black"],
                        help="Which side makes the mistake (solver is the other side)")
    parser.add_argument("--count", type=int, default=100,
                        help="Number of puzzles to generate (default: 100)")
    parser.add_argument("-o", "--output",
                        help="Output file (default: puzzles_{eco}.json)")
    parser.add_argument("--min-rating", type=int, default=1800,
                        help="Minimum rating filter (default: 1800)")
    parser.add_argument("--threshold", type=int, default=200,
                        help="Minimum cp loss to qualify as mistake (default: 200)")
    args = parser.parse_args()
    asyncio.run(generate_puzzles(args))


if __name__ == "__main__":
    main()
