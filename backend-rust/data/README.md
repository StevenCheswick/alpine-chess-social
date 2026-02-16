# Data Files

## Opening Book (`opening_book.bin`)

Binary serialized opening book for instant in-memory lookups.

### Regenerating the Book

To export from the production database:

```bash
# From backend-rust directory
DATABASE_URL=$(aws secretsmanager get-secret-value \
  --secret-id alpine-chess/database-url \
  --region us-east-1 \
  --query SecretString \
  --output text) \
cargo run -p server --bin export-book
```

Or from local database:

```bash
cargo run -p server --bin export-book
```

### How It Works

1. **Source of truth**: PostgreSQL `opening_book` table (prod)
2. **Export**: `export-book` binary queries all rows, serializes to bincode
3. **Runtime**: Server loads binary at startup into `HashMap` for O(1) lookups

### Updating the Book

1. Edit rows in prod PostgreSQL (INSERT/UPDATE/DELETE)
2. Re-run the export script above
3. Commit the new `opening_book.bin`
4. Deploy

### Table Schema

```sql
CREATE TABLE opening_book (
    parent_fen TEXT NOT NULL,    -- Normalized FEN (position + side + castling + ep)
    move_san TEXT NOT NULL,      -- Move in SAN notation
    games INT NOT NULL,          -- Number of games with this move
    white_wins INT NOT NULL,     -- White wins
    draws INT NOT NULL,          -- Draws
    black_wins INT NOT NULL,     -- Black wins
    PRIMARY KEY (parent_fen, move_san)
);
```
