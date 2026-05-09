"""Upload play-vs-Maia trainer positions to the API.

Usage:
    python upload_maia_position.py upload position.json
    python upload_maia_position.py upload positions.json   # array also supported
    python upload_maia_position.py list
    python upload_maia_position.py delete <id>

Position JSON shape:
    {
        "id": "evans-h6",
        "title": "Evans Gambit: 8...h6",
        "fen": "r1bqk1nr/ppp2pp1/2np3p/b7/2BPP3/5N2/P4PPP/RNBQ1RK1 w kq - 0 9",
        "user_side": "white",
        "notes": "optional"
    }

Reads ADMIN_SECRET from environment (or .env file one directory up).
"""
import argparse
import json
import os
import sys
import urllib.error
import urllib.request

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
        print(f"HTTP {e.code}: {e.read().decode()}", file=sys.stderr)
        sys.exit(1)


def _get_secret():
    secret = os.environ.get("ADMIN_SECRET")
    if not secret:
        print("Error: ADMIN_SECRET not set (check .env or environment)", file=sys.stderr)
        sys.exit(1)
    return secret


REQUIRED = ("id", "title", "fen", "user_side")
OPTIONAL = ("notes", "opening_name")


def _validate(item):
    for k in REQUIRED:
        if k not in item:
            raise ValueError(f"missing required field '{k}' in {item!r}")
    if item["user_side"] not in ("white", "black"):
        raise ValueError(f"user_side must be 'white' or 'black' (got {item['user_side']!r})")


def cmd_upload(args):
    secret = _get_secret()
    with open(args.file, encoding="utf-8") as f:
        data = json.load(f)
    items = data if isinstance(data, list) else [data]
    for item in items:
        _validate(item)

    for item in items:
        payload = {
            "id": item["id"],
            "title": item["title"],
            "fen": item["fen"],
            "user_side": item["user_side"],
        }
        for key in OPTIONAL:
            if key in item:
                payload[key] = item[key]
        result = api_request(
            args.url,
            "/api/admin/trainer/maia-positions/upload",
            method="POST",
            data=payload,
            secret=secret,
        )
        print(f"  {item['id']}: {result}")
    print(f"Uploaded {len(items)} position(s) to {args.url}")


def cmd_list(args):
    result = api_request(args.url, "/api/trainer/maia-positions")
    if not result:
        print("No Maia positions found.")
        return
    for p in result:
        notes = f" — {p['notes']}" if p.get("notes") else ""
        print(f"  [{p['user_side']:5}] {p['id']}: {p['title']}{notes}")


def cmd_delete(args):
    secret = _get_secret()
    result = api_request(
        args.url,
        "/api/admin/trainer/maia-positions/delete",
        method="POST",
        data={"id": args.id},
        secret=secret,
    )
    print(f"Done: {result}")


def main():
    load_env()
    parser = argparse.ArgumentParser(description="Upload Maia trainer positions")
    parser.add_argument("--url", default=DEFAULT_URL, help="API base URL")
    sub = parser.add_subparsers(dest="command")

    up = sub.add_parser("upload", help="Upload a position JSON file")
    up.add_argument("file", help="JSON file (object or array of objects)")

    sub.add_parser("list", help="List all Maia positions")

    dl = sub.add_parser("delete", help="Delete a position by id")
    dl.add_argument("id", help="Position id (e.g. 'evans-h6')")

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
