# PostgreSQL — Setup & Operations Runbook

> **Accuracy note:** Reflects **PostgreSQL 16/17** (2024–2025 defaults). Verify settings against `SHOW <name>;` and the [docs for your exact version](https://www.postgresql.org/docs/current/) — defaults do change between majors. Unlike this repo's DSA docs, the numbers here are from official documentation, **not measured on this machine**. Concepts are in [learning.md](learning.md); this is the procedure.

## Part 1 — Development setup

### 1.1 Docker (fastest path)

```sh
docker run -d --name pg -e POSTGRES_PASSWORD=dev \
  -p 5432:5432 -v pgdata:/var/lib/postgresql/data postgres:17
psql "postgresql://postgres:dev@localhost:5432/postgres"
```

**What dev mode does differently, and why each disqualifies it for production:**

| Dev shortcut | Production requirement | Why it matters |
| --- | --- | --- |
| Trust/plain password auth | `scram-sha-256` + TLS | Credentials cross the network in the clear |
| Default `postgres` superuser for the app | Dedicated least-privilege role | An SQL-injection bug becomes a full compromise |
| Data in an anonymous volume | Managed volume + verified backups | `docker rm` deletes the database |
| Defaults (`shared_buffers=128MB`) | Tuned to the machine | Off by ~10× on a real server |
| No WAL archiving | `archive_mode=on` + PITR | Recovery is limited to the last full dump |

### 1.2 A minimal schema and a Rust client

```sh
psql -c "CREATE DATABASE app;"
psql -d app -c "CREATE TABLE users (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now());"
```

```rust
// sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "macros"] }
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(10)                       // ← per instance; see Part 6 on pooling
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    // Compile-time-checked query (requires DATABASE_URL at build time, or `cargo sqlx prepare`)
    let row = sqlx::query!("SELECT id, email FROM users WHERE email = $1", "a@b.com")
        .fetch_optional(&pool)
        .await?;
    println!("{row:?}");
    Ok(())
}
```

Always use **parameterized queries** (`$1`), never string interpolation — that's the SQL-injection boundary, and `sqlx::query!` additionally verifies the SQL against the live schema at compile time.

## Part 2 — Production installation

### 2.1 Decide the topology first

| Topology | Use when | Cost |
| --- | --- | --- |
| Single instance + PITR backups | Non-critical; RTO in hours acceptable | Simplest; downtime on host loss |
| **Primary + streaming replica** | **The default for production** | One extra host; manual or managed failover |
| Primary + sync replica | Zero data loss required | Commit latency includes a network round trip |
| Primary + N async replicas | Read scaling | Replica lag is visible to readers |
| Patroni/etcd or a managed service | Automated failover needed | Real operational complexity — see 2.2 |

**Choose a managed service (RDS, Cloud SQL, Aurora) unless you have a specific reason not to.** Self-hosting Postgres means owning failover, backup verification, and major-version upgrades. Justify that ownership before taking it on.

### 2.2 Host prerequisites

```sh
# Filesystem: ext4 or xfs. Data and WAL on separate volumes if I/O-bound.
# Disable transparent huge pages (causes latency spikes)
echo never > /sys/kernel/mm/transparent_hugepage/enabled

# Kernel: allow the shared memory Postgres wants
sysctl -w vm.overcommit_memory=2        # avoid the OOM killer choosing postgres
sysctl -w vm.swappiness=1

# Dedicated unprivileged user, data dir not world-readable
install -d -o postgres -g postgres -m 0700 /var/lib/postgresql/17/main
```

### 2.3 The production config file

`postgresql.conf` — the settings that matter, annotated. Everything omitted can stay default.

```ini
# ---- Connections ----
listen_addresses = '10.0.1.5'        # NOT '*' — bind to the private interface only
max_connections = 200                # keep low; PgBouncer multiplexes (Part 6)
password_encryption = scram-sha-256  # never md5

# ---- TLS (required if anything crosses a network) ----
ssl = on
ssl_cert_file = '/etc/postgresql/tls/server.crt'
ssl_key_file  = '/etc/postgresql/tls/server.key'   # chmod 0600, owned by postgres
ssl_min_protocol_version = 'TLSv1.2'

# ---- Memory (sized for a 64 GB host) ----
shared_buffers = 16GB                # ~25% RAM
effective_cache_size = 48GB          # planner HINT — allocates nothing
work_mem = 32MB                      # PER SORT/HASH PER BACKEND. See the warning below.
maintenance_work_mem = 2GB           # vacuum, CREATE INDEX
huge_pages = try

# ---- Planner (the single highest-value change) ----
random_page_cost = 1.1               # SSD. Default 4.0 assumes spinning disks.
effective_io_concurrency = 200       # SSD/NVMe
default_statistics_target = 100      # raise per-column for skewed data

# ---- WAL / durability / replication ----
wal_level = replica                  # 'logical' if you need CDC or logical replication
synchronous_commit = on              # 'off' trades durability for latency — decide explicitly
max_wal_size = 8GB
min_wal_size = 2GB
checkpoint_timeout = 15min
checkpoint_completion_target = 0.9   # spread the flush; avoids I/O spikes
wal_compression = zstd
archive_mode = on
archive_command = 'wal-g wal-push %p'   # or pgbackrest; see Part 7
max_wal_senders = 10
wal_keep_size = 2GB                  # or use a replication slot (see the warning in Part 7)

# ---- Autovacuum (raise the defaults; they are too lazy for busy tables) ----
autovacuum_max_workers = 6
autovacuum_vacuum_scale_factor = 0.05    # default 0.2 = vacuum only at 20% dead
autovacuum_analyze_scale_factor = 0.02
autovacuum_vacuum_cost_limit = 2000      # default 200 throttles vacuum severely

# ---- Safety rails ----
statement_timeout = '60s'                     # per-role override for analytics
idle_in_transaction_session_timeout = '60s'   # THE bloat preventer
lock_timeout = '3s'                           # override to 0 for maintenance windows

# ---- Observability ----
shared_preload_libraries = 'pg_stat_statements'
log_min_duration_statement = 250ms   # log slow queries, not everything
log_checkpoints = on
log_lock_waits = on
log_autovacuum_min_duration = 0
log_line_prefix = '%m [%p] %q%u@%d app=%a '
```

> **`work_mem` is the setting that OOMs machines.** It is per *operation*, not per connection. A query with three sorts and a hash join uses ~4× `work_mem`; 200 such connections at 32 MB is 25 GB. Size it as `(RAM − shared_buffers − OS) / (max_connections × 2)` and raise it per-session for known-heavy queries with `SET LOCAL work_mem`.

`pg_hba.conf` — authentication, evaluated **top to bottom, first match wins**:

```
# TYPE  DATABASE  USER      ADDRESS         METHOD
local   all       postgres                  peer
hostssl app       app_rw    10.0.1.0/24     scram-sha-256
hostssl app       app_ro    10.0.1.0/24     scram-sha-256
hostssl replication repl    10.0.1.0/24     scram-sha-256
# No trailing catch-all. Anything not matched is rejected.
```

Use `hostssl` (not `host`) so a client cannot silently negotiate a plaintext connection.

## Part 3 — Initialization

```sh
# 1. Initialize with checksums ON — you cannot enable this later without a rebuild
initdb -D /var/lib/postgresql/17/main --data-checksums \
       --auth-host=scram-sha-256 --auth-local=peer -U postgres

# 2. Start, then create roles BEFORE any application connects
systemctl start postgresql@17-main

psql -U postgres <<'SQL'
CREATE ROLE app_rw LOGIN PASSWORD 'CHANGEME';   -- from your secret store, not a literal
CREATE ROLE app_ro LOGIN PASSWORD 'CHANGEME';
CREATE ROLE repl REPLICATION LOGIN PASSWORD 'CHANGEME';
CREATE DATABASE app OWNER app_rw;

\c app
REVOKE ALL ON SCHEMA public FROM PUBLIC;         -- PG15+ already restricts this; be explicit
GRANT USAGE ON SCHEMA public TO app_rw, app_ro;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO app_ro;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO app_ro;  -- future tables too
SQL
```

`--data-checksums` is the one that cannot be retrofitted cheaply — it detects silent corruption, and enabling it later requires `pg_checksums` with the cluster offline.

## Part 4 — Day-1 hardening (before real data)

Order matters; each step assumes the previous.

1. **Never let the app use a superuser.** `app_rw` owns its schema and nothing more. An SQL-injection bug should not be able to read `pg_shadow` or write files.
2. **TLS on, `hostssl` only**, and verify from a client: `psql "sslmode=verify-full sslrootcert=ca.crt ..."`. `sslmode=require` does **not** verify the certificate — it only encrypts.
3. **Set the three timeouts** (`statement_timeout`, `idle_in_transaction_session_timeout`, `lock_timeout`). These prevent the two worst failure modes: unbounded queries and vacuum starvation.
4. **Enable `pg_stat_statements`** and confirm it's collecting — you cannot diagnose what you never measured.
5. **Configure and then *test* backups** (Part 7). An untested backup is not a backup.
6. **Row-level security** if the schema is multi-tenant — `ALTER TABLE t ENABLE ROW LEVEL SECURITY` plus a policy, so a missing `WHERE tenant_id` clause fails closed.
7. **Restrict `pg_hba.conf`** to known CIDRs, no catch-all.

## Part 5 — Migrations without downtime (the thing most often gotten wrong)

**Always set `lock_timeout` before DDL.** A blocked DDL statement queues every subsequent query behind it, turning a 3-second lock wait into a site-wide outage.

```sql
SET lock_timeout = '3s';                -- fail fast rather than pile up traffic
```

| Operation | Naive form | Safe form |
| --- | --- | --- |
| Add index | `CREATE INDEX` (blocks writes) | **`CREATE INDEX CONCURRENTLY`** (2–3× slower, no txn) |
| Drop index | `DROP INDEX` | `DROP INDEX CONCURRENTLY` |
| Add column with default | fine on **PG 11+** (no rewrite) | On older: add nullable, backfill, then set default |
| Add `NOT NULL` | full table scan under lock | Add `CHECK (col IS NOT NULL) NOT VALID`, `VALIDATE`, then convert (PG 12+) |
| Add FK / CHECK | validates under lock | `... NOT VALID;` then `VALIDATE CONSTRAINT` |
| Change column type | full rewrite | New column + backfill in batches + swap |
| Drop column | fast (metadata only) | Fine — but the space returns only after vacuum |
| Rename column | breaks running app | Expand/contract: add new, dual-write, migrate reads, drop old |

Backfills must be **batched** so each transaction is short:

```sql
-- Loop until 0 rows affected; keeps each txn short so vacuum and replicas keep up
UPDATE users SET status = 'active'
WHERE id IN (SELECT id FROM users WHERE status IS NULL LIMIT 5000);
```

`CREATE INDEX CONCURRENTLY` cannot run inside a transaction, and a failure leaves an `INVALID` index that must be dropped and retried — check with:

```sql
SELECT indexrelid::regclass FROM pg_index WHERE NOT indisvalid;
```

## Part 6 — Connection pooling (not optional)

Postgres forks a process per connection ([learning.md](learning.md)), so a few hundred is the ceiling. **PgBouncer in transaction mode** is the standard answer.

```ini
# pgbouncer.ini
[databases]
app = host=10.0.1.5 port=5432 dbname=app

[pgbouncer]
listen_addr = 10.0.1.6
listen_port = 6432
auth_type = scram-sha-256
auth_file = /etc/pgbouncer/userlist.txt
pool_mode = transaction          # the sweet spot — returns the server conn at COMMIT
max_client_conn = 5000           # what clients see
default_pool_size = 25           # actual server connections. Start at 2–4 × CPU cores.
reserve_pool_size = 5
server_tls_sslmode = verify-full
```

**Transaction mode breaks session state.** Audit for these before enabling:

| Breaks | Fix |
| --- | --- |
| `SET`/`SET SESSION` | Use `SET LOCAL` inside a transaction |
| `LISTEN`/`NOTIFY` | Separate direct connection (session mode) |
| Session advisory locks | Use transaction-scoped `pg_advisory_xact_lock` |
| `WITH HOLD` cursors | Avoid, or use session mode |
| Server-side prepared statements | `sqlx`: `statement_cache_capacity(0)`, or use PgBouncer ≥ 1.21 |
| Temp tables across statements | Keep within one transaction |

Application pool sizing: `instances × max_connections_per_instance` must stay under PgBouncer's `max_client_conn`, and `default_pool_size` must stay under Postgres' `max_connections` with headroom for maintenance sessions.

## Part 7 — Day-2 operations

### 7.1 Backups — and the restore drill

Two mechanisms, and you want both:

| Method | Gives | Cost |
| --- | --- | --- |
| `pg_dump -Fc` | Logical, portable across versions, per-table restore | Slow on large DBs; **not** point-in-time |
| **Physical + WAL archive** (`pgBackRest`, `wal-g`) | **PITR to any second**, fast restore | Same-version restore only |

```sh
# pgBackRest — the recommended tool
pgbackrest --stanza=app --type=full backup
pgbackrest --stanza=app --type=incr backup

# Restore to a point in time
pgbackrest --stanza=app --type=time --target="2025-08-09 14:30:00" restore
```

> **An untested backup is not a backup.** Schedule a monthly restore drill into a scratch host, run `pg_amcheck` or a checksum query against it, and record the wall-clock **RTO you actually achieved**. Most backup failures are discovered during the first real incident.

### 7.2 Upgrades — order matters

**Minor** (17.2 → 17.3): binaries only, restart required. Replicas first, then primary, to keep the replicas' version ≥ primary.

**Major** (16 → 17): the data directory format changes.

```sh
# Option A: pg_upgrade with hard links — minutes of downtime, no rollback after start
pg_upgrade -b /usr/lib/postgresql/16/bin -B /usr/lib/postgresql/17/bin \
           -d /var/lib/postgresql/16/main -D /var/lib/postgresql/17/main --link --check
# --check first, ALWAYS. Then run without it.
# ANALYZE afterwards — pg_upgrade does not carry statistics over, and plans will be terrible until you do.

# Option B: logical replication — near-zero downtime, rollback possible, more setup
```

**Extensions gate major upgrades.** Verify every extension has a version built for the target major before scheduling.

### 7.3 Routine procedures

```sql
-- Rotate an application password (no restart needed)
ALTER ROLE app_rw PASSWORD 'new-from-secret-store';

-- Reclaim bloat without an ACCESS EXCLUSIVE lock
-- (VACUUM FULL rewrites the table and blocks everything — use pg_repack instead)
-- $ pg_repack -d app -t big_table

-- Refresh planner statistics after a bulk load
ANALYZE VERBOSE big_table;

-- Promote a replica (failover)
-- $ pg_ctl promote -D /var/lib/postgresql/17/main
```

### 7.4 Monitoring — signals and thresholds

| Signal | Query / source | Alert when |
| --- | --- | --- |
| **Replication lag** | `pg_wal_lsn_diff(pg_current_wal_lsn(), replay_lsn)` | > 50 MB or > 30 s |
| **Oldest transaction age** | `max(age(clock_timestamp(), xact_start))` | > 5 min |
| **TXID wraparound headroom** | `age(datfrozenxid)` | > 500M (hard stop ≈ 2B) |
| **Replication slot retention** | `pg_replication_slots.wal_status` | `extended`/`lost` — **a dead slot fills the disk** |
| Dead tuples | `pg_stat_user_tables.n_dead_tup` | > 20% of live tuples |
| Cache hit ratio | `pg_stat_database` blks_hit/(hit+read) | < 0.99 sustained |
| Connection utilization | `numbackends / max_connections` | > 80% |
| Checkpoint frequency | `log_checkpoints` output | "checkpoints occurring too frequently" |
| Disk free on WAL volume | OS | < 20% |
| Failed logins | Log scrape | Any spike |

The **replication slot** row is the one that pages people at 3 a.m.: an inactive slot pins WAL indefinitely and will fill the disk, taking the primary down. Either monitor and drop abandoned slots, or set `max_slot_wal_keep_size` so Postgres invalidates the slot rather than dying.

## Part 8 — Dev → production checklist

**Before first real data**
- [ ] `initdb --data-checksums` (cannot be retrofitted cheaply)
- [ ] `scram-sha-256`, `ssl = on`, `hostssl`-only in `pg_hba.conf`, no catch-all rule
- [ ] Application role is **not** superuser; `app_ro` for read paths
- [ ] `statement_timeout`, `idle_in_transaction_session_timeout`, `lock_timeout` set
- [ ] `shared_buffers`, `effective_cache_size`, `work_mem`, **`random_page_cost = 1.1`**
- [ ] `pg_stat_statements` loaded and collecting
- [ ] Autovacuum scale factors lowered from defaults

**Before taking traffic**
- [ ] PgBouncer in transaction mode; session-state audit complete
- [ ] Pool arithmetic checked: app pools ≤ `max_client_conn`, `default_pool_size` ≤ `max_connections`
- [ ] `archive_mode = on` and WAL archiving verified to be *landing in the target*
- [ ] **Restore drill performed**, RTO recorded
- [ ] Streaming replica built and lag monitored
- [ ] Monitoring for all ten signals above, with the slot-retention alert wired

**Ongoing**
- [ ] Monthly restore drill
- [ ] Extension compatibility checked before each major upgrade
- [ ] `ANALYZE` after every bulk load and after `pg_upgrade`
- [ ] Unused indexes reviewed quarterly (`idx_scan = 0`)

## Common mistakes → what actually happens

| Mistake | Consequence |
| --- | --- |
| No connection pooler | Fork storm, OOM, refused connections at a few hundred clients |
| `work_mem` set high with many connections | Machine OOMs — it's per *operation*, not per connection |
| Left `random_page_cost = 4.0` on SSD | Planner avoids indexes; sequential scans on large tables |
| Ran `CREATE INDEX` without `CONCURRENTLY` | Writes blocked for the build; queries pile up behind the lock |
| No `lock_timeout` before DDL | A 3-second lock wait cascades into a site-wide outage |
| App connects as superuser | SQL injection becomes full host compromise |
| Backups never restored | Discovered broken during the incident that needed them |
| Abandoned replication slot | WAL accumulates until the **primary's disk fills** |
| `sslmode=require` assumed to verify | Encrypted but unauthenticated — MITM works |
| `VACUUM FULL` on a live table | `ACCESS EXCLUSIVE` lock; full outage for the duration |
| Forgot `ANALYZE` after `pg_upgrade` | Catastrophic plans until statistics are rebuilt |
| Long-running analytics on the primary | Vacuum starves; database-wide bloat |

## References

- [PostgreSQL Server Administration](https://www.postgresql.org/docs/current/admin.html) — the authoritative source; check your exact major version
- [pgBackRest](https://pgbackrest.org/) · [PgBouncer](https://www.pgbouncer.org/) — the two tools this runbook assumes
- [Don't Do This (wiki)](https://wiki.postgresql.org/wiki/Don%27t_Do_This)
- Related in this repo: [learning.md](learning.md) (why the settings matter), [reference.md](reference.md), [Replication & Consistency](../../architecture-patterns/replication-and-consistency/learning.md), [Backpressure & Rate Limiting](../../architecture-patterns/backpressure-and-rate-limiting/learning.md) (pool sizing is admission control), [B-Trees](../../data-structures-and-algorithms/b-trees/learning.md)
