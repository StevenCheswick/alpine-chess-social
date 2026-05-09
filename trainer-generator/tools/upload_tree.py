"""Upload an opening tree JSON to the trainer-trees API.

Uploading a tree auto-creates the trainer_opening_meta entry (root FEN, user color)
so the opening appears in the catalog immediately.

Usage:
    python upload_tree.py upload <tree.json> --id evans-h6 --name "Evans Gambit: 8...h6" --opening "Evans Gambit"
    python upload_tree.py list
    python upload_tree.py delete --id evans-h6

--opening defaults to --name if omitted. Use it to group multiple trees under one opening.

Reads ADMIN_SECRET from environment (or .env file in trainer-generator/).
Defaults to local API at http://localhost:8000.
"""
import argparse
import json
import os
import sys
import urllib.request
import urllib.error

DEFAULT_URL = "http://localhost:8000"


def load_env():
    env_path = os.path.join(os.path.dirname(__file__), "..", ".env")
    if os.path.exists(env_path):
        with open(env_path) as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    key, val = line.split("=", 1)
                    os.environ.setdefault(key.strip(), val.strip())


def api_request(url, path, method="GET", data=None, secret=None):
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


def cmd_upload(args):
    if not os.path.exists(args.file):
        print(f"File not found: {args.file}", file=sys.stderr)
        sys.exit(1)

    with open(args.file) as f:
        data = json.load(f)

    tree = data.get("tree")
    if tree is None:
        print("JSON missing 'tree' field — is this a build_tree.py output?", file=sys.stderr)
        sys.exit(1)

    color = data.get("color", "white")
    start_moves = data.get("start_moves", "")
    start_fen = tree.get("fen")
    if not start_fen:
        print("tree.fen missing", file=sys.stderr)
        sys.exit(1)

    nodes_count = data.get("nodes_after_prune") or data.get("nodes_top_n") or _count_nodes(tree)
    lines_count = _count_leaves(tree)

    opening = getattr(args, "opening", None) or args.name
    payload = {
        "id": args.id,
        "name": args.name,
        "color": color,
        "start_moves": start_moves,
        "start_fen": start_fen,
        "nodes_count": nodes_count,
        "lines_count": lines_count,
        "tree": tree,
        "opening_name": opening,
    }

    print(f"Uploading: id={args.id} opening={opening!r} name={args.name!r} color={color} nodes={nodes_count} lines={lines_count} ...")
    result = api_request(args.url, "/api/admin/trainer/trees/upload",
                         method="POST", data=payload, secret=_get_secret())
    print(f"OK: {result}")


def _count_nodes(node):
    return 1 + sum(_count_nodes(c) for c in node.get("children", []))


def _count_leaves(node):
    """Count leaf paths (= unique variations from root to leaf)."""
    children = node.get("children", [])
    if not children:
        return 1
    return sum(_count_leaves(c) for c in children)


def cmd_list(args):
    # The list endpoint requires AuthUser, not admin secret.
    # For prototype: hit it without auth and let it fail if needed.
    # Better: use a simple GET against the admin variant if/when added.
    print("Listing trees (requires user auth — currently this script can only upload/delete).")
    print("Open the trainer tab in the app to see uploaded trees.")


def cmd_delete(args):
    print(f"Deleting tree: id={args.id} ...")
    result = api_request(args.url, "/api/admin/trainer/trees/delete",
                         method="POST", data={"id": args.id}, secret=_get_secret())
    print(f"OK: {result}")


def main():
    load_env()
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default=DEFAULT_URL)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("upload")
    p.add_argument("file")
    p.add_argument("--id", required=True, help="slug, e.g. 'evans-gambit-white'")
    p.add_argument("--name", required=True, help="display name")
    p.add_argument("--opening", help="opening name for catalog grouping (defaults to --name)")
    p.set_defaults(func=cmd_upload)

    p = sub.add_parser("list")
    p.set_defaults(func=cmd_list)

    p = sub.add_parser("delete")
    p.add_argument("--id", required=True)
    p.set_defaults(func=cmd_delete)

    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
