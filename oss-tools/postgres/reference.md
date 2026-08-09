# PostgreSQL — Quick Reference

## Quick Facts

- **Alternative to:** Oracle, SQL Server, DB2 (and the usual comparison, MySQL)
- **License:** PostgreSQL Licence (permissive, BSD-style) — **no feature gating, no commercial edition**
- **Backed by:** Independent core team + committers across EDB, Crunchy, Microsoft, AWS, Fujitsu. **No single owner who can relicense it.**

## Comparison

| Aspect | Oracle / SQL Server | **PostgreSQL** |
| --- | --- | --- |
| Licence | Per-core | **Free, no gating** |
| Clustered index | Yes | **No** — heap + separate indexes |
| Connections | Threaded, thousands | **Process-per-connection**, hundreds |
| Multi-master | RAC (Oracle) | Extension/external only |
| Sharding | Built-in (Oracle) | Citus extension |
| Transactional DDL | Partial | **Yes — migrations roll back** |
| JSON | Good | **Excellent** (`jsonb` + GIN) |
| Extensibility | Limited | **First-class** (types, index AMs, FDWs) |

## Settings That Matter (defaults are sized for a laptop)

| Setting | Default | Production starting point |
| --- | --- | --- |
| `shared_buffers` | 128 MB | **~25% of RAM** |
| `effective_cache_size` | 4 GB | ~50–75% of RAM (planner hint only) |
| `work_mem` | 4 MB | `RAM_for_queries / (max_conn × sorts)` — **per operation!** |
| `maintenance_work_mem` | 64 MB | ~1 GB |
| **`random_page_cost`** | **4.0** | **1.1 on SSD** — often flips bad seq scans to index scans |
| `max_connections` | 100 | Keep low; **use PgBouncer** |
| `wal_compression` | off | `zstd` |
| `checkpoint_completion_target` | 0.9 | 0.9 (fine) |
| `default_statistics_target` | 100 | 100–500 for skewed columns |

> `work_mem` is **per sort/hash per backend**. One query with 3 sorts × 200 connections = 600× the setting.

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Connection exhaustion | **PgBouncer, transaction mode** | Breaks session `SET`, `LISTEN/NOTIFY`, advisory locks |
| Long transactions block vacuum | `idle_in_transaction_session_timeout`, `statement_timeout` | `hot_standby_feedback` moves bloat to the primary |
| Default config in prod | Set the 6 settings above | `work_mem` is per-operation — can OOM |
| Index not used | `EXPLAIN (ANALYZE, BUFFERS)`; expression indexes | Estimated vs actual rows = the real signal |
| DDL locks the table | `CREATE INDEX CONCURRENTLY`, `NOT VALID` + `VALIDATE` | **Always `SET lock_timeout` first** |
| Table bloat | Autovacuum tuning, `pg_repack` | `VACUUM FULL` takes `ACCESS EXCLUSIVE` |
| TXID wraparound | Monitor `age(datfrozenxid)` | Read-only lockdown at 1M remaining |

## Common Commands

```sh
# Connect / inspect
psql -h host -U user -d db
\dt+  \di+  \d+ table   \l   \du   \x auto

# What's happening right now
SELECT pid, age(clock_timestamp(), xact_start) AS xact_age, state, left(query,60)
FROM pg_stat_activity WHERE state <> 'idle' ORDER BY xact_start;

# Blocking chain
SELECT pid, pg_blocking_pids(pid), left(query,60) FROM pg_stat_activity
WHERE cardinality(pg_blocking_pids(pid)) > 0;

# Bloat / vacuum health
SELECT relname, n_dead_tup, last_autovacuum FROM pg_stat_user_tables
ORDER BY n_dead_tup DESC LIMIT 20;

# Unused indexes (pure write cost)
SELECT relname, indexrelname, idx_scan FROM pg_stat_user_indexes WHERE idx_scan = 0;

# Slowest statements (needs pg_stat_statements)
SELECT mean_exec_time, calls, left(query,60) FROM pg_stat_statements
ORDER BY mean_exec_time DESC LIMIT 20;

# Plans — always ANALYZE + BUFFERS; compare estimated vs actual rows
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) SELECT ...;

# Backup / restore
pg_dump -Fc -d mydb > mydb.dump          # logical, portable
pg_restore -d mydb --jobs=4 mydb.dump
pg_basebackup -D /backup -Fp -Xs -P -R   # physical, for PITR/replicas

# Safe DDL
SET lock_timeout = '3s';
CREATE INDEX CONCURRENTLY idx ON t (col);
ALTER TABLE t ADD CONSTRAINT c CHECK (...) NOT VALID;
ALTER TABLE t VALIDATE CONSTRAINT c;
```

## Migration Checklist

- [ ] Schema converted (`pgloader` / `ora2pg`); stored procedures reviewed by hand
- [ ] Identifier case checked — Postgres folds unquoted to **lower**, Oracle to upper
- [ ] Empty string ≠ `NULL` (differs from Oracle)
- [ ] `SERIAL`/`IDENTITY` replaces auto-increment; sequences reset after bulk load
- [ ] `ANALYZE` run after load — plans are garbage without statistics
- [ ] Extensions available on the target (managed services restrict these)
- [ ] PgBouncer in front, transaction mode, session-state audit done
- [ ] `random_page_cost`, `shared_buffers`, `work_mem` set
- [ ] Backups verified by an actual **restore drill**, not just a successful dump
- [ ] Monitoring: replication lag, oldest transaction, dead tuples, `datfrozenxid` age

## Key References

- [PostgreSQL docs](https://www.postgresql.org/docs/current/) — Server Administration + Performance Tips
- [Don't Do This (wiki)](https://wiki.postgresql.org/wiki/Don%27t_Do_This) — read once, end to end
- [Use The Index, Luke](https://use-the-index-luke.com/) — why your index isn't used
