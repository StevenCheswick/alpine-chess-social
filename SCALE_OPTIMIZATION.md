# Scale Optimization: Lichess Opening Explorer Approach

This document outlines Lichess's data storage strategies for their opening explorer, which handles **trillions of positions** efficiently. These techniques should be considered if Alpine Chess grows to a large userbase.

---

## Current Approach (PostgreSQL + FEN)

```sql
CREATE TABLE opening_book (
    parent_fen  TEXT NOT NULL,        -- ~60 bytes
    move_san    TEXT NOT NULL,        -- ~5 bytes
    result_fen  TEXT NOT NULL,        -- ~60 bytes
    games       INTEGER NOT NULL,
    white_wins  INTEGER NOT NULL,
    draws       INTEGER NOT NULL,
    black_wins  INTEGER NOT NULL,
    avg_rating  SMALLINT,
    PRIMARY KEY (parent_fen, move_san)
);
```

**Pros:** Simple, queryable, human-readable, standard tooling
**Cons:** Large key sizes, doesn't scale to billions of positions

---

## Lichess Approach: Key Optimizations

### 1. Zobrist Hashing (Position Keys)

Instead of storing FEN strings (~60 bytes), Lichess uses **Zobrist hashes** (8-16 bytes).

```
FEN:     "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3"  (56 bytes)
Zobrist: 0x823c9b50fd114196 (8 bytes)
```

**How it works:**
- Each piece-square combination has a precomputed random 64-bit number
- XOR all piece-square values together = unique position hash
- O(1) lookup, 7x smaller than FEN

**Trade-off:** Not human-readable, requires code to decode

### 2. Key Structure (14 bytes total)

```
Key = [base (4 bytes)] XOR [zobrist (8 bytes)] XOR [variant (constant)] + [year-month (2 bytes)]
```

The temporal suffix enables efficient time-range queries without scanning all records.

### 3. RocksDB Instead of PostgreSQL

| Feature | PostgreSQL | RocksDB |
|---------|------------|---------|
| Query language | SQL | Key-value API |
| ACID transactions | Full | Limited |
| Write performance | Good | Excellent |
| Compression | Table-level | Block-level (better) |
| Merge operations | UPDATE | Native merge operators |
| Horizontal scaling | Complex | Simpler |

**When to switch:**
- Write-heavy workloads (>10K inserts/sec)
- Storage costs becoming significant
- Simple key-value access patterns

### 4. Hierarchical Data Nesting

Instead of flat rows, Lichess nests data:

```
Position
  └── Move (UCI)
        └── Speed (bullet/blitz/rapid/classical)
              └── Rating Bracket (1000/1200/1400/.../2600+)
                    └── Stats { white_wins, draws, black_wins, rating_sum }
                    └── Game IDs []
```

**Benefits:**
- Single read gets all data for a position
- Filter by speed/rating without separate queries
- Compact binary serialization

### 5. Binary Serialization

Custom binary format instead of JSON/text:

```rust
// Instead of JSON (verbose)
{"white": 1234, "draws": 567, "black": 890}  // 43 bytes

// Binary (compact)
[u64 white][u64 draws][u64 black]  // 24 bytes
```

Lichess claims: **"Database size < 3x compressed PGN size"**

### 6. Merge Operations

RocksDB supports **merge operators** - combine values without read-modify-write:

```rust
// Traditional approach (3 operations)
value = db.get(key)
value.white_wins += 1
db.put(key, value)

// Merge approach (1 operation)
db.merge(key, increment_white_wins)
```

**Benefits:**
- Faster concurrent writes
- No read amplification
- Atomic updates

### 7. Column Families

Separate data into logical groups:

| Column Family | Purpose | Access Pattern |
|--------------|---------|----------------|
| `masters` | Master game positions | Read-heavy |
| `masters_game` | Game metadata | Sparse reads |
| `lichess` | All Lichess positions | Write-heavy |
| `lichess_game` | Game references | Batch writes |
| `player` | Per-player stats | User-specific |
| `player_status` | Index status | Admin |

**Benefits:**
- Different compaction strategies per family
- Isolate hot/cold data
- Independent backups

---

## Migration Triggers

Consider migrating when:

| Metric | Current OK | Consider Migration |
|--------|------------|-------------------|
| Positions stored | < 100M | > 500M |
| Daily writes | < 100K | > 1M |
| Storage size | < 50 GB | > 200 GB |
| Query latency p99 | < 100ms | > 500ms |
| DB costs/month | < $100 | > $500 |

---

## Incremental Migration Path

### Phase 1: Zobrist Keys (Low effort)
Keep PostgreSQL, but add a `zobrist_hash BIGINT` column:
```sql
ALTER TABLE opening_book ADD COLUMN zobrist_hash BIGINT;
CREATE INDEX idx_opening_book_zobrist ON opening_book (zobrist_hash);
```
Query by hash, fall back to FEN for debugging.

### Phase 2: Binary Stats (Medium effort)
Store stats as packed binary instead of separate columns:
```sql
ALTER TABLE opening_book ADD COLUMN stats_bin BYTEA;
-- Encode: white_wins (4) + draws (4) + black_wins (4) + rating_sum (8) = 20 bytes
```

### Phase 3: RocksDB Service (High effort)
Extract opening book to separate Rust service:
- Keep PostgreSQL for relational data (users, games, posts)
- RocksDB microservice for opening explorer
- gRPC or REST API between them

---

## Reference Implementation

Lichess opening explorer source: https://github.com/lichess-org/lila-openingexplorer

Key files:
- `src/db.rs` - RocksDB setup, column families
- `src/model/stats.rs` - Stats struct
- `src/model/key.rs` - Zobrist key encoding
- `src/model/lichess.rs` - Hierarchical entry structure
- `src/opening.rs` - ECO code lookup

---

## Summary

| Optimization | Storage Savings | Complexity |
|--------------|-----------------|------------|
| Zobrist keys | ~7x smaller keys | Low |
| Binary serialization | ~2x smaller values | Medium |
| RocksDB | Better compression | High |
| Merge operations | Faster writes | High |
| Column families | Better isolation | Medium |

**Recommendation:** Start simple (current PostgreSQL approach), add Zobrist index when > 100M positions, consider RocksDB extraction when > 500M positions or write throughput becomes a bottleneck.
