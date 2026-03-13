"""Upload puzzle JSON files to the trainer API.

Usage:
    python upload_puzzles.py upload puzzles_evans.json "Evans Gambit"
    python upload_puzzles.py upload --type hard-move hard_moves_dragon.json "Sicilian Dragon"
    python upload_puzzles.py list
    python upload_puzzles.py list --type hard-move
    python upload_puzzles.py delete "Evans Gambit"
    python upload_puzzles.py delete --type hard-move "Sicilian Dragon"

Reads ADMIN_SECRET from environment (or .env file in this directory).
"""
import argparse
import hashlib
import json
import os
import sys
import urllib.request
import urllib.error

DEFAULT_URL = "http://localhost:8000"


def load_env():
    """Load .env file from current directory if it exists."""
    env_path = os.path.join(os.path.dirname(__file__), "..", ".env")
    if os.path.exists(env_path):
        with open(env_path) as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    key, val = line.split("=", 1)
                    os.environ.setdefault(key.strip(), val.strip())


def api_request(url, path, method="GET", data=None, secret=None):
    """Make an API request and return parsed JSON."""
    full_url = f"{url.rstrip('/')}{path}"
    headers = {"Content-Type": "application/json"}
    if secret:
        headers["X-Admin-Secret"] = secret

    body = json.dumps(data).encode() if data else None
    req = urllib.request.Request(full_url, data=body, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req) as resp:
            return json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        body = e.read().decode()
        print(f"HTTP {e.code}: {body}", file=sys.stderr)
        sys.exit(1)


def _get_secret():
    secret = os.environ.get("ADMIN_SECRET")
    if not secret:
        print("Error: ADMIN_SECRET not set (check .env or environment)", file=sys.stderr)
        sys.exit(1)
    return secret


def _is_hard_move(args):
    return getattr(args, "type", None) == "hard-move"


def _api_paths(hard_move):
    if hard_move:
        return "/api/admin/trainer/hard-moves/upload", "/api/admin/trainer/hard-moves/list", "/api/admin/trainer/hard-moves/delete"
    return "/api/admin/trainer/upload", "/api/admin/trainer/list", "/api/admin/trainer/delete"


def _ensure_ids(items):
    """Add 'id' field to hard move items that don't have one (hash of FEN)."""
    for item in items:
        if "id" not in item:
            fen = item.get("fen", "")
            h = hashlib.sha256(fen.encode()).hexdigest()[:12]
            item["id"] = f"hm_{h}"
    return items


def cmd_upload(args):
    secret = _get_secret()
    hard_move = _is_hard_move(args)
    upload_path, _, _ = _api_paths(hard_move)

    with open(args.file) as f:
        puzzles = json.load(f)

    if not isinstance(puzzles, list):
        puzzles = [puzzles]

    if hard_move:
        puzzles = _ensure_ids(puzzles)

    kind = "hard moves" if hard_move else "puzzles"
    print(f"Uploading {len(puzzles)} {kind} as '{args.opening_name}' to {args.url}")
    result = api_request(
        args.url,
        upload_path,
        method="POST",
        data={"opening_name": args.opening_name, "puzzles": puzzles},
        secret=secret,
    )
    print(f"Done: {result}")


def cmd_list(args):
    secret = _get_secret()
    hard_move = _is_hard_move(args)
    _, list_path, _ = _api_paths(hard_move)

    result = api_request(args.url, list_path, secret=secret)
    if not result:
        kind = "hard move openings" if hard_move else "openings"
        print(f"No {kind} found.")
        return
    for opening in result:
        count_key = "count" if hard_move else "puzzle_count"
        kind = "hard moves" if hard_move else "puzzles"
        print(f"  {opening['opening_name']}: {opening[count_key]} {kind}")


def cmd_delete(args):
    secret = _get_secret()
    hard_move = _is_hard_move(args)
    _, _, delete_path = _api_paths(hard_move)

    kind = "hard moves" if hard_move else "puzzles"
    print(f"Deleting all {kind} for '{args.opening_name}' from {args.url}")
    result = api_request(
        args.url,
        delete_path,
        method="POST",
        data={"opening_name": args.opening_name},
        secret=secret,
    )
    print(f"Done: {result}")


def main():
    load_env()

    parser = argparse.ArgumentParser(description="Upload trainer puzzles to API")
    parser.add_argument("--url", default=DEFAULT_URL, help="API base URL")
    sub = parser.add_subparsers(dest="command")

    up = sub.add_parser("upload", help="Upload a puzzle file")
    up.add_argument("file", help="Puzzle JSON file")
    up.add_argument("opening_name", help="Opening name (e.g. 'Evans Gambit')")
    up.add_argument("--type", choices=["puzzle", "hard-move"], default="puzzle", help="Puzzle type")

    ls = sub.add_parser("list", help="List openings in the database")
    ls.add_argument("--type", choices=["puzzle", "hard-move"], default="puzzle", help="Puzzle type")

    dl = sub.add_parser("delete", help="Delete all puzzles for an opening")
    dl.add_argument("opening_name", help="Opening name to delete")
    dl.add_argument("--type", choices=["puzzle", "hard-move"], default="puzzle", help="Puzzle type")

    args = parser.parse_args()

    if args.command == "upload":
        cmd_upload(args)
    elif args.command == "list":
        cmd_list(args)
    elif args.command == "delete":
        cmd_delete(args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
