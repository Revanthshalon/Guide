# PostgreSQL — Learning Notes

## What It Is & Why It Exists

**PostgreSQL is an open-source object-relational database that competes with Oracle, SQL Server, and DB2 on capability rather than on price.** That framing matters: unlike most entries in this category it isn't a fork or a licence-driven reimplementation — it's a 1986 Berkeley research project (POSTGRES, successor to Ingres) that has been continuously developed for ~40 years and independently arrived at feature parity with the commercial systems.

The properties that decide adoption:

- **PostgreSQL Licence** — a permissive BSD/MIT-style licence. No copyleft, no commercial edition, **no feature gating**. Everything the project has is in the free version, which is the sharpest contrast with the vendor databases (Oracle charges per-core for partitioning; SQL Server gates availability groups by edition) and with MySQL's dual-licence model.
- **No single corporate owner.** Governance is a core team plus committers across many companies (EDB, Crunchy Data, Microsoft, AWS, Fujitsu). There is no entity that can relicense it — the practical lesson of the HashiCorp BUSL change that produced [OpenBao](../openbao/learning.md) and [OpenTofu](../opentofu/learning.md).
- **Extensibility as an architectural choice.** Custom types, operators, index access methods, procedural languages, and foreign data wrappers are first-class extension points, which is why PostGIS, TimescaleDB, and pgvector exist as extensions rather than forks.

**What it replaces, and the honest gaps:** Postgres matches or exceeds Oracle/SQL Server on SQL conformance, transactional semantics, JSON, extensibility, and indexing breadth. It is genuinely behind on: built-in multi-master replication (Oracle RAC has no direct equivalent), mature built-in sharding (Citus is an extension), enterprise tooling polish, and the connection model (below).

## Architecture & Core Concepts

### Process-per-connection

- **What it is:** Postgres forks an OS process for every connection — not a thread, not a green task. Each backend has its own memory (`work_mem`, catalogue caches) and its own file descriptors.
- **Why it matters operationally:** A connection costs ~5–10 MB of RSS and a fork, so **a few hundred connections is the practical ceiling** on a normal machine. This is the single most common operational surprise for people arriving from MySQL or from a threaded server, and it's why a connection pooler is not optional at scale. It also means `max_connections` is a *memory* setting, not just an admission limit — every `work_mem` allocation is per-operation, per-backend.

### MVCC and the visibility map

- **What it is:** Every write creates a **new row version** rather than overwriting; each version carries `xmin`/`xmax` transaction stamps, and a snapshot decides which versions a transaction can see. Readers never block writers and writers never block readers.
- **Why it matters operationally:** Dead row versions accumulate and must be reclaimed by **vacuum**. This is the mechanism behind bloat, transaction-ID wraparound, and most "why is my table 10× the size of its data" questions. It's the same copy-on-write idea as the persistent structures in [persistent & immutable structures](../../data-structures-and-algorithms/persistent-immutable-structures/learning.md) — including the same failure mode: **old versions that stay reachable are never reclaimed**, and a long-running transaction is exactly a retained old root.

### WAL — write-ahead log

- **What it is:** Every change is written to a sequential log and fsynced *before* the data pages are modified. Recovery replays the log; replication ships it.
- **Why it matters operationally:** WAL is simultaneously the durability mechanism, the replication transport, and the backup foundation (PITR). Checkpoints flush dirty pages and let old WAL be recycled, so checkpoint tuning is a trade between write amplification and recovery time. The append-then-merge shape is the same one as [LSM trees](../../data-structures-and-algorithms/lsm-trees/learning.md), though Postgres' heap itself is update-in-place.

### The B-tree heap/index split

- **What it is:** Table data lives in an unordered **heap**; indexes are separate structures (B-tree by default) pointing at heap tuples by physical location (`ctid`).
- **Why it matters operationally:** Postgres has **no clustered index** — unlike InnoDB or SQL Server, the primary key does not determine physical order. Every index lookup is therefore an index scan *plus* a heap fetch, unless the query is covered by an **index-only scan** (which additionally requires the visibility map to say the page is all-visible, i.e. recently vacuumed). This is the [B-tree](../../data-structures-and-algorithms/b-trees/learning.md) topic in production, and it explains why `VACUUM` affects read performance and not just space.

### The planner is cost-based, and statistics-driven

- **What it is:** The planner enumerates plans and picks by estimated cost, using column statistics gathered by `ANALYZE` and cost constants (`random_page_cost`, `seq_page_cost`, `effective_cache_size`).
- **Why it matters operationally:** Bad plans almost always come from **bad estimates**, not a bad planner. The two levers are better statistics (raise `default_statistics_target`, add extended statistics for correlated columns) and honest cost constants (`random_page_cost = 1.1` on SSD, not the default 4.0 which assumes spinning disks).

### Extensions

- **What it is:** Loadable modules that add types, functions, index methods, and hooks. `CREATE EXTENSION postgis;`
- **Why it matters operationally:** Extensions are the reason to choose Postgres for specialized workloads — PostGIS for geospatial ([spatial data structures](../../data-structures-and-algorithms/spatial-data-structures/learning.md) in production), pgvector for embeddings (HNSW, from the same topic), TimescaleDB for time-series, Citus for sharding. But they constrain upgrades and managed-service choice: an extension must be available and version-compatible on your target.

## Comparison in Depth

| Aspect | Oracle / SQL Server | **PostgreSQL** |
| --- | --- | --- |
| Licence cost | Per-core, significant | **Free, no feature gating** |
| Feature gating by edition | Extensive | **None** |
| Concurrency model | MVCC (Oracle), lock+MVCC (MSSQL) | MVCC, no read locks |
| Clustered index | Yes | **No** — heap + separate indexes |
| Connections | Threaded, thousands | **Process-per-connection**, hundreds |
| Multi-master | Oracle RAC | **Extension/external only** |
| Sharding | Built-in (Oracle) | **Citus extension** |
| Stored procedures | PL/SQL, T-SQL | PL/pgSQL, plus Python/Perl/JS/Rust |
| JSON | Good | **Excellent** (`jsonb`, GIN indexes) |
| Extensibility | Limited | **First-class** — custom types, index AMs |
| Upgrade friction | Vendor tooling | `pg_upgrade`, extension compatibility |

**vs MySQL** — the more common real comparison: Postgres wins on SQL conformance, transactional DDL (migrations roll back!), JSON, indexing breadth (GiST/GIN/BRIN/SP-GiST), and extensions. MySQL wins on the connection model (threads), replication maturity for simple topologies, and read-heavy simplicity. The clustered-index difference cuts both ways — InnoDB's PK-ordered storage makes range scans on the PK faster; Postgres' heap avoids the write amplification of maintaining that order.

## Pitfalls in Depth

### Pitfall: Connection exhaustion from process-per-connection

- **What goes wrong:** An application with a 50-connection pool per instance is scaled to 40 instances. That's 2,000 connections against a `max_connections` of 100–200; connections are refused, or `max_connections` is raised and the server runs out of memory because each backend costs ~5–10 MB plus `work_mem` per sort/hash *per operation*.
- **Why it happens (the mechanism):** Each connection is an OS process. Beyond a few hundred, context switching and memory dominate, and Postgres' shared structures (lock table, snapshot computation) start to contend. Raising `max_connections` doesn't add capacity — it converts a refusal into thrashing, which is worse because it degrades everyone.
- **How to handle it in production, and why that works:** Put **PgBouncer in transaction mode** in front. It multiplexes thousands of client connections onto a small server pool (typically `2–4 × cores`), because most connections are idle between statements. Transaction mode is the sweet spot: it returns the server connection at each `COMMIT`, so a handful of backends serve a very large client population.
- **Trade-offs of the fix:** Transaction mode breaks anything that spans transactions on one connection: session-level `SET`, `LISTEN/NOTIFY`, advisory locks held across statements, `WITH HOLD` cursors, and **prepared statements** unless you enable statement-level tracking. Session mode preserves all of it and gives up most of the pooling benefit. Auditing the application for session state is the real work.

### Pitfall: Long transactions blocking vacuum

- **What goes wrong:** An analytics query, an idle-in-transaction connection, or a stalled replica holds an old snapshot open. Vacuum cannot reclaim any row version newer than that snapshot, so dead tuples accumulate across the *whole database*. Tables bloat, index scans slow, and in the worst case transaction-ID wraparound protection forces the database into read-only mode to prevent data loss.
- **Why it happens (the mechanism):** MVCC visibility means a dead tuple is only removable once **no** snapshot can see it. One old snapshot pins every version created since it began. This is exactly the retained-old-root problem from [persistent structures](../../data-structures-and-algorithms/persistent-immutable-structures/learning.md) — structural sharing means one retained reference keeps a lot of garbage alive — and the failure is equally silent: nothing errors, storage just grows.
- **How to handle it in production, and why that works:** Set `idle_in_transaction_session_timeout` (kill connections holding a transaction open while doing nothing) and `statement_timeout` so no query runs unboundedly. Monitor `pg_stat_activity` for `xact_start` age and alert on the oldest transaction. On replicas, either set `hot_standby_feedback = on` (which pushes the problem to the primary, honestly) or accept query cancellation via `max_standby_streaming_delay`.
- **Trade-offs of the fix:** `hot_standby_feedback = on` makes replica queries reliable at the cost of letting a replica's long query bloat the *primary* — you've moved the problem, not solved it. Aggressive `statement_timeout` breaks legitimate long analytics, so the usual answer is a separate replica with a different timeout profile.

### Pitfall: Default configuration in production

- **What goes wrong:** Postgres ships with settings sized for a machine that can barely run it (`shared_buffers = 128 MB`, `work_mem = 4 MB`, `random_page_cost = 4.0`). On a 64 GB server these are off by more than an order of magnitude: queries spill sorts and hashes to disk, the planner avoids index scans because it thinks random I/O is 4× sequential, and the buffer cache is a rounding error.
- **Why it happens (the mechanism):** The defaults are deliberately conservative so `initdb` succeeds on any machine including small containers and developer laptops. They are a *compatibility* choice, not a recommendation, and nothing in the startup output says so.
- **How to handle it in production, and why that works:** Set the handful that matter: `shared_buffers` ≈ 25% of RAM, `effective_cache_size` ≈ 50–75% of RAM (a planner hint, not an allocation), `work_mem` sized as `RAM_for_queries / (max_connections × expected_concurrent_sorts)`, `maintenance_work_mem` ≈ 1 GB, and **`random_page_cost = 1.1` on SSD** — that last one alone frequently flips bad sequential scans into correct index scans.
- **Trade-offs of the fix:** `work_mem` is the dangerous one: it's per *operation*, not per connection, so a query with three sorts and a hash join can use 4× the setting, and 200 connections doing that simultaneously can OOM the machine. Size it conservatively and raise it per-session for known-heavy queries instead.

### Pitfall: Assuming an index will be used

- **What goes wrong:** An index is created and the query still does a sequential scan. Or the index exists but the query uses a function (`WHERE lower(email) = ...`), a leading wildcard (`LIKE '%foo'`), or a type mismatch (`WHERE int_col = '123'` with an implicit cast) that makes it unusable.
- **Why it happens (the mechanism):** A B-tree index is ordered by the indexed *expression*. Any transformation of the column breaks the ordering, so the index cannot answer the predicate. And even when usable, the planner may correctly reject it: if a query returns a large fraction of the table, a sequential scan genuinely is cheaper because Postgres must visit the heap for each index hit anyway (no clustered index).
- **How to handle it in production, and why that works:** Read the plan (`EXPLAIN (ANALYZE, BUFFERS)`) rather than guessing, and compare *estimated* against *actual* rows — a large divergence means the statistics are wrong, which is the real problem. Create **expression indexes** (`CREATE INDEX ON users (lower(email))`) to match the predicate, use `pg_trgm` GIN indexes for leading-wildcard search, and fix type mismatches.
- **Trade-offs of the fix:** Every index slows writes and consumes space, and unused indexes are pure cost — check `pg_stat_user_indexes` for `idx_scan = 0` before adding more. Expression indexes only help queries using that exact expression.

### Pitfall: `CREATE INDEX` (and other DDL) locking a live table

- **What goes wrong:** `CREATE INDEX` on a large table takes an `ACCESS EXCLUSIVE`-adjacent lock that blocks writes for the duration — minutes to hours. Worse, `ALTER TABLE ... ADD COLUMN ... DEFAULT` (before PG 11), `ALTER COLUMN TYPE`, and adding a `FOREIGN KEY` or `CHECK` constraint each take a full table lock and rewrite or scan the table. The migration takes the site down.
- **Why it happens (the mechanism):** The lock is needed for correctness while the structure is built. Crucially, a blocked DDL statement **also blocks every query queued behind it**, so a lock wait of a few seconds on a busy table produces a cascading pile-up far larger than the DDL itself.
- **How to handle it in production, and why that works:** Use `CREATE INDEX CONCURRENTLY` (two passes, no write lock — but it can't run in a transaction and leaves an `INVALID` index if it fails, which you must drop and retry). Add constraints as `NOT VALID` first, then `VALIDATE CONSTRAINT` in a separate transaction which takes a weaker lock. **Always set `lock_timeout` before DDL** (e.g. `SET lock_timeout = '3s'`) so a migration fails fast rather than queueing traffic behind it.
- **Trade-offs of the fix:** `CONCURRENTLY` is roughly 2–3× slower and needs cleanup on failure. `NOT VALID` constraints don't enforce existing rows until validated. `lock_timeout` means migrations fail more often — which is the point, since a failed migration is recoverable and a locked production table is not.

## Migration Walkthrough

Coming from MySQL/Oracle, the compatibility surface and cutover:

1. **Schema conversion** — `pgloader` handles MySQL well and does data and schema in one pass; `ora2pg` for Oracle. Expect manual work on stored procedures (PL/SQL → PL/pgSQL is not mechanical), auto-increment (`SERIAL`/`IDENTITY`), and case sensitivity (Postgres folds unquoted identifiers to *lower* case; Oracle folds to upper).
2. **Behavioural differences to test** — `NULL` handling in unique indexes, empty string ≠ `NULL` (Oracle treats them the same), default isolation is `READ COMMITTED` (same as Oracle, unlike MySQL's `REPEATABLE READ`), and **DDL is transactional** in Postgres, which means migrations can roll back.
3. **Dual-write or logical replication** — for low-downtime cutover, replicate into Postgres with a CDC tool ([change data capture](../../architecture-patterns/change-data-capture/learning.md)) and verify with row counts and checksums before switching reads, then writes.
4. **Rollback plan** — keep the source authoritative until the verification window passes; reverse replication is much harder than forward, so the rollback is "switch back and replay", which requires the source to still be receiving writes.

## Open Questions

- Which managed offering (RDS, Aurora, Cloud SQL, Neon, Supabase) is right for a Rust service — specifically, which support the extensions and PgBouncer topology assumed here?
- `pgvector` HNSW at realistic scale: index build time, memory, and recall against the [spatial structures](../../data-structures-and-algorithms/spatial-data-structures/learning.md) numbers.
- Citus vs application-level [sharding](../../architecture-patterns/sharding/learning.md) — at what data size does the extension earn its operational cost?
- How much does `random_page_cost = 1.1` actually change plan choice on a representative schema? Measure with `EXPLAIN` before/after.
- Logical replication as a migration path *out* of Postgres — is it symmetric enough to be a genuine rollback?

## References

- [PostgreSQL documentation](https://www.postgresql.org/docs/current/) — unusually good; the "Server Administration" and "Performance Tips" chapters are the operational core.
- [PostgreSQL Wiki: Don't Do This](https://wiki.postgresql.org/wiki/Don%27t_Do_This) — a short list of common mistakes with reasons; worth reading once end to end.
- Markus Winand, [Use The Index, Luke](https://use-the-index-luke.com/) — the best explanation of index behaviour and why queries don't use them.
- [PGTune](https://pgtune.leopard.in.ua/) — generates a reasonable starting configuration from hardware; a starting point, not an answer.
- Related in this repo: [runbook.md](runbook.md) (the procedures), [reference.md](reference.md), [B-Trees](../../data-structures-and-algorithms/b-trees/learning.md) (the index structure), [LSM Trees](../../data-structures-and-algorithms/lsm-trees/learning.md) (the write-optimized alternative and the RUM trade), [Persistent & Immutable Structures](../../data-structures-and-algorithms/persistent-immutable-structures/learning.md) (MVCC as copy-on-write), [Replication & Consistency](../../architecture-patterns/replication-and-consistency/learning.md), [Caching Strategies](../../architecture-patterns/caching-strategies/learning.md), [Change Data Capture](../../architecture-patterns/change-data-capture/learning.md) (logical replication).
