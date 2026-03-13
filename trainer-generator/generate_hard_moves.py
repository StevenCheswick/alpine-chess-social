#!/usr/bin/env python3
"""
Generate hard-move anti-puzzles: positions where humans commonly play an
inaccuracy instead of the correct move.

Pipeline:
  Pre-filter (cheap):
    1. find-mistakes (Rust binary, --threshold 50) -> raw candidates
    2. Eval/swing filter (+-75cp, 50-100cp swing) + FEN dedup
    3. Maia confirmation (mistake is popular: top move or >= 20%)
    4. SF 30K confirmation (loss 50-100cp)
  Verification (expensive, runs on pre-filter survivors only):
    5. SF 600K MultiPV 2 — re-validate eval/loss at higher depth, check gap >= 30cp
       + Maia best-move probability lookup
    6. Filter to best_maia_pct <= threshold, take top N by game count

Usage:
  python -u generate_hard_moves.py \
    --play e2e4,c7c5,g1f3,d7d6,d2d4,c5d4,f3d4,g8f6,b1c3,g7g6 \
    --side black --count 100 -o hard_moves_dragon.json
"""

import argparse
import asyncio
import chess
import chess.engine
import json
import subprocess
import sys
from pathlib import Path

# ── Constants ────────────────────────────────────────────────────────────────

STOCKFISH_PATH = "../backend-rust/stockfish.exe"
MAIA_DIR = Path("C:/Users/steve/OneDrive/Desktop/maia")
FIND_MISTAKES_BIN = Path(__file__).parent.parent / "lila-openingexplorer" / "target" / "release" / "find-mistakes.exe"
EXPLORER_DB = Path(__file__).parent.parent / "lila-openingexplorer" / "_db"
MAIA_RATING = 1900
PREFILTER_NODES = 30_000
VERIFY_NODES = 600_000


# ── Step 1: Find Mistakes ────────────────────────────────────────────────────


def step1_find_mistakes(play, side, target):
    """Shell out to the Rust find-mistakes binary (queries RocksDB directly)."""
    print(f"Step 1: find-mistakes (target={target})")
    print(f"  Binary: {FIND_MISTAKES_BIN}")
    print(f"  DB: {EXPLORER_DB}")

    if not FIND_MISTAKES_BIN.exists():
        print(f"ERROR: {FIND_MISTAKES_BIN} not found. Build with:")
        print(f"  cd lila-openingexplorer && cargo build --release --bin find-mistakes")
        sys.exit(1)

    cmd = [
        str(FIND_MISTAKES_BIN),
        "--play", play,
        "--side", side,
        "--threshold", "50",
        "--target", str(target),
        "--min-games", "2",
        "--db", str(EXPLORER_DB),
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, check=True)
    mistakes = json.loads(result.stdout)
    if result.stderr:
        print(result.stderr, end="")
    print(f"  -> {len(mistakes)} raw candidates")
    return mistakes


# ── Step 2: Eval/Swing Filter ────────────────────────────────────────────────


def step2_filter(mistakes, top, eval_min, eval_max, swing_min, swing_max):
    """Filter for equal positions with inaccuracy-range swing, dedup by FEN."""
    filtered = []
    for m in mistakes:
        side = m["side"]
        eval_mover = m["eval_before"] if side == "white" else -m["eval_before"]
        if eval_mover < eval_min or eval_mover > eval_max:
            continue
        if m["cp_loss"] < swing_min or m["cp_loss"] > swing_max:
            continue
        filtered.append(m)

    # Dedup by FEN — keep highest game count
    seen = {}
    for m in filtered:
        if m["fen"] not in seen or m["games"] > seen[m["fen"]]["games"]:
            seen[m["fen"]] = m
    before = len(filtered)
    filtered = sorted(seen.values(), key=lambda x: -x["games"])[:top]

    print(f"Step 2: eval/swing filter -> {len(filtered)} (deduped {before - len(filtered) - (before - len(seen))} FENs)")
    return filtered


# ── Step 3: Maia Confirmation ────────────────────────────────────────────────


async def step3_maia_confirm(candidates, maia):
    """Gate: Maia's top move IS the mistake, OR mistake has >= 20% probability."""
    print(f"Step 3: Maia confirmation ({len(candidates)} candidates)")
    confirmed = []
    rejected = 0

    for i, m in enumerate(candidates):
        maia_result = await maia.analyze_fen(m["fen"], top_n=10)
        maia_moves = {mv["san"]: mv["probability"] for mv in maia_result["moves"]}
        maia_top = maia_result["moves"][0] if maia_result["moves"] else None

        if not maia_top:
            rejected += 1
            continue

        mistake_pct = maia_moves.get(m["move"], 0.0)
        if maia_top["san"] == m["move"] or mistake_pct >= 20.0:
            m["maia_top_move"] = maia_top["san"]
            m["maia_top_pct"] = maia_top["probability"]
            m["mistake_maia_pct"] = mistake_pct
            confirmed.append(m)
        else:
            rejected += 1

        if (i + 1) % 100 == 0:
            print(f"  [{i+1}/{len(candidates)}] confirmed={len(confirmed)} rejected={rejected}")

    print(f"  -> {len(confirmed)} confirmed, {rejected} rejected")
    return confirmed


# ── Step 4: SF Pre-filter ────────────────────────────────────────────────────


async def step4_sf_prefilter(candidates, sf, min_loss, max_loss):
    """SF at 30K nodes: confirm loss is in range."""
    print(f"Step 4: SF pre-filter at {PREFILTER_NODES} nodes ({len(candidates)} candidates)")
    confirmed = []

    for i, m in enumerate(candidates):
        board = chess.Board(m["fen"])

        # Best move eval
        info_best = await sf.analyse(board, chess.engine.Limit(nodes=PREFILTER_NODES))
        score_best = info_best["score"].pov(board.turn)
        sf_best_eval = score_best.score() if not score_best.is_mate() else (10000 if score_best.mate() > 0 else -10000)
        sf_best_move = board.san(info_best["pv"][0]) if info_best.get("pv") else "?"

        # Eval after mistake
        mistake_chess_move = board.parse_san(m["move"])
        board.push(mistake_chess_move)
        info_mistake = await sf.analyse(board, chess.engine.Limit(nodes=PREFILTER_NODES))
        score_mistake = info_mistake["score"].pov(not board.turn)
        sf_mistake_eval = score_mistake.score() if not score_mistake.is_mate() else (10000 if score_mistake.mate() > 0 else -10000)
        board.pop()

        sf_loss = sf_best_eval - sf_mistake_eval
        if sf_loss >= min_loss and sf_loss <= max_loss:
            m["prefilter_best_move"] = sf_best_move
            m["prefilter_best_eval"] = sf_best_eval
            m["prefilter_loss"] = sf_loss
            confirmed.append(m)

        if (i + 1) % 100 == 0:
            print(f"  [{i+1}/{len(candidates)}] confirmed={len(confirmed)}")

    print(f"  -> {len(confirmed)} confirmed")
    return confirmed


# ── Step 5: 600K MultiPV Verification ────────────────────────────────────────


async def step5_verify(candidates, sf, maia, min_gap, eval_min, eval_max, min_loss, max_loss, count=None):
    """600K MultiPV 2 verification + Maia best-move lookup.
    Re-validates eval/loss criteria at higher node count.
    Stops early once count verified positions are found (candidates are pre-sorted by game count)."""
    print(f"Step 5: 600K MultiPV 2 verification ({len(candidates)} candidates{f', target={count}' if count else ''})")
    results = []
    gap_rejected = 0
    eval_rejected = 0
    loss_rejected = 0

    for i, m in enumerate(candidates):
        board = chess.Board(m["fen"])

        sf.send_line("ucinewgame")
        await sf.ping()

        # MultiPV 2 at 600K
        mpv = await sf.analyse(board, chess.engine.Limit(nodes=VERIFY_NODES), multipv=2)

        best_score = mpv[0]["score"].pov(board.turn)
        best_eval = best_score.score() if not best_score.is_mate() else (10000 if best_score.mate() > 0 else -10000)
        best_san = board.san(mpv[0]["pv"][0])

        if len(mpv) >= 2:
            second_score = mpv[1]["score"].pov(board.turn)
            second_eval = second_score.score() if not second_score.is_mate() else (10000 if second_score.mate() > 0 else -10000)
            second_san = board.san(mpv[1]["pv"][0])
            gap = best_eval - second_eval
        else:
            second_eval = None
            second_san = "?"
            gap = 9999

        # Re-validate eval range at 600K (mover's POV = best_eval)
        if best_eval < eval_min or best_eval > eval_max:
            eval_rejected += 1
            if (i + 1) % 50 == 0:
                print(f"  [{i+1}/{len(candidates)}] verified={len(results)} gap_rej={gap_rejected} eval_rej={eval_rejected} loss_rej={loss_rejected}")
            continue

        # Re-validate loss at 600K
        # Eval after mistake move
        mistake_chess_move = board.parse_san(m["move"])
        board.push(mistake_chess_move)
        info_mistake = await sf.analyse(board, chess.engine.Limit(nodes=VERIFY_NODES))
        score_mistake = info_mistake["score"].pov(not board.turn)
        mistake_eval = score_mistake.score() if not score_mistake.is_mate() else (10000 if score_mistake.mate() > 0 else -10000)
        board.pop()

        verified_loss = best_eval - mistake_eval
        if verified_loss < min_loss or verified_loss > max_loss:
            loss_rejected += 1
            if (i + 1) % 50 == 0:
                print(f"  [{i+1}/{len(candidates)}] verified={len(results)} gap_rej={gap_rejected} eval_rej={eval_rejected} loss_rej={loss_rejected}")
            continue

        # Gap filter
        if gap < min_gap:
            gap_rejected += 1
            if (i + 1) % 50 == 0:
                print(f"  [{i+1}/{len(candidates)}] verified={len(results)} gap_rej={gap_rejected} eval_rej={eval_rejected} loss_rej={loss_rejected}")
            continue

        # Maia best-move probability
        maia_result = await maia.analyze_fen(m["fen"], top_n=10)
        maia_moves = {mv["san"]: mv["probability"] for mv in maia_result["moves"]}
        best_pct = maia_moves.get(best_san, 0.0)

        # Convert evals to White's POV for output
        # (internal filtering uses mover's POV, but stored evals are always White's)
        wpov = 1 if board.turn == chess.WHITE else -1
        seq_parts = m["sequence"].split("|")
        entry = {
            "fen": m["fen"],
            "eco": m["eco"],
            "sequence": m["sequence"],
            "ply": len(seq_parts),
            "side": m["side"],
            "games": m["games"],
            "best_move": best_san,
            "best_eval_cp": best_eval * wpov,
            "best_maia_pct": round(best_pct, 1),
            "second_move": second_san,
            "second_eval_cp": second_eval * wpov if second_eval is not None else None,
            "gap_cp": gap,
            "mistake_move": m["move"],
            "mistake_eval_cp": mistake_eval * wpov,
            "mistake_maia_pct": round(m["mistake_maia_pct"], 1),
            "eval_loss_cp": verified_loss,
            "maia_top_3": [
                {"san": mv["san"], "pct": round(mv["probability"], 1)}
                for mv in maia_result["moves"][:3]
            ],
        }
        results.append(entry)

        if count and len(results) >= count:
            print(f"  [{i+1}/{len(candidates)}] reached target ({count}) — stopping early")
            break

        if (i + 1) % 50 == 0:
            print(f"  [{i+1}/{len(candidates)}] verified={len(results)} gap_rej={gap_rejected} eval_rej={eval_rejected} loss_rej={loss_rejected}")

    print(f"  -> {len(results)} verified (rejected: {gap_rejected} gap, {eval_rejected} eval, {loss_rejected} loss)")
    return results


# ── Main ─────────────────────────────────────────────────────────────────────


async def run(args):
    BATCH_SIZE = args.batch_size

    # Start engines once (shared across all batches)
    sys.path.insert(0, str(MAIA_DIR))
    from maia_engine import MaiaEngine
    print(f"Starting Maia-{MAIA_RATING}...")
    maia = MaiaEngine(MAIA_RATING)
    await maia.start()

    print("Starting Stockfish...")
    _, sf = await chess.engine.popen_uci(STOCKFISH_PATH)
    await sf.configure({"Threads": 1, "Hash": 256})

    results = []
    seen_fens = set()
    fm_target = 0
    total_raw = 0
    batch_num = 0

    while len(results) < args.count:
        batch_num += 1
        fm_target += BATCH_SIZE
        print(f"\n{'='*60}")
        print(f"Batch {batch_num}: find-mistakes target={fm_target} (have {len(results)}/{args.count} verified)")
        print(f"{'='*60}")

        # Step 1: find-mistakes (cumulative target, returns most popular first)
        raw = step1_find_mistakes(args.play, args.side, fm_target)
        if not raw:
            print("No raw candidates found. Stopping.")
            break
        total_raw = len(raw)

        # Skip already-processed FENs
        new_raw = [m for m in raw if m["fen"] not in seen_fens]
        if not new_raw:
            print(f"No new candidates in this batch. Stopping.")
            break
        print(f"  {len(new_raw)} new candidates ({len(raw) - len(new_raw)} already seen)")

        # Step 2: eval/swing filter
        candidates = step2_filter(new_raw, len(new_raw), args.eval_min, args.eval_max, args.swing_min, args.swing_max)
        seen_fens.update(m["fen"] for m in new_raw)
        if not candidates:
            print("No candidates after filter. Trying next batch...")
            continue

        # Step 3: Maia confirmation
        candidates = await step3_maia_confirm(candidates, maia)
        if not candidates:
            print("No candidates survived Maia. Trying next batch...")
            continue

        # Step 4: SF pre-filter
        candidates = await step4_sf_prefilter(candidates, sf, args.min_loss, args.max_loss)
        if not candidates:
            print("No candidates survived SF pre-filter. Trying next batch...")
            continue

        # Step 5: 600K verification + Maia best-move lookup
        need = args.count - len(results)
        batch_results = await step5_verify(
            candidates, sf, maia,
            min_gap=args.min_gap,
            eval_min=args.eval_min, eval_max=args.eval_max,
            min_loss=args.min_loss, max_loss=args.max_loss,
            count=need,
        )
        results.extend(batch_results)
        print(f"\n  Batch {batch_num} done: +{len(batch_results)} verified, {len(results)}/{args.count} total")

    await sf.quit()
    await maia.close()

    # Final: filter by max_maia, sort by games, take top N
    if args.max_maia is not None:
        before = len(results)
        results = [r for r in results if r["best_maia_pct"] <= args.max_maia]
        print(f"\nMaia filter: {len(results)} with best_maia_pct <= {args.max_maia}% (dropped {before - len(results)})")

    results.sort(key=lambda x: -x["games"])
    results = results[:args.count]

    with open(args.output, "w") as f:
        json.dump(results, f, indent=2)

    print(f"\n{'='*60}")
    print(f"Done: {len(results)} hard-move positions -> {args.output}")
    print(f"  Batches: {batch_num}")
    print(f"  Raw candidates pulled: {total_raw}")
    print(f"  FENs processed: {len(seen_fens)}")
    if results:
        print(f"\nTop 10 hardest (lowest Maia % for best move):")
        by_best = sorted(results, key=lambda x: x["best_maia_pct"])
        for i, r in enumerate(by_best[:10]):
            print(f"  {i+1:>3}. {r['games']:>5}g | best: {r['best_move']:>6} Maia {r['best_maia_pct']:>4.1f}% | "
                  f"gap {r['gap_cp']:>3}cp | "
                  f"mistake: {r['mistake_move']:>6} Maia {r['mistake_maia_pct']:>4.1f}% | loss {r['eval_loss_cp']}cp")


def main():
    parser = argparse.ArgumentParser(description="Generate hard-move anti-puzzles")
    parser.add_argument("--play", required=True,
                        help="Starting position as UCI moves (comma-separated)")
    parser.add_argument("--side", required=True, choices=["white", "black"],
                        help="Which side makes the inaccuracy")
    parser.add_argument("-o", "--output", default="hard_moves.json",
                        help="Output file (default: hard_moves.json)")
    parser.add_argument("--count", type=int, default=100,
                        help="Max positions in final output (default: 100)")
    parser.add_argument("--batch-size", type=int, default=1000,
                        help="Raw candidates per batch from find-mistakes (default: 1000)")
    parser.add_argument("--eval-min", type=int, default=-75,
                        help="Min eval from mover's POV (default: -75)")
    parser.add_argument("--eval-max", type=int, default=75,
                        help="Max eval from mover's POV (default: 75)")
    parser.add_argument("--swing-min", type=int, default=50,
                        help="Min cp loss in pre-filter (default: 50)")
    parser.add_argument("--swing-max", type=int, default=100,
                        help="Max cp loss in pre-filter (default: 100)")
    parser.add_argument("--min-loss", type=int, default=50,
                        help="Min SF-confirmed cp loss (default: 50)")
    parser.add_argument("--max-loss", type=int, default=100,
                        help="Max SF-confirmed cp loss (default: 100)")
    parser.add_argument("--min-gap", type=int, default=30,
                        help="Min gap between #1 and #2 at 600K nodes (default: 30)")
    parser.add_argument("--max-maia", type=float, default=None,
                        help="Max Maia %% for best move (e.g. 5.0). No filter if omitted.")
    args = parser.parse_args()
    asyncio.run(run(args))


if __name__ == "__main__":
    main()
