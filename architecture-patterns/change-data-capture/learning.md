# Change Data Capture — Learning Notes

## Mental Model

**Your database already keeps a perfect, ordered log of every change it commits — the write-ahead log. CDC is the act of reading that log and turning it into a stream others can consume.**

Every durable database writes changes to an append-only log (Postgres WAL, MySQL binlog) *before* applying them — that's how crash recovery and [replication](../replication-and-consistency/learning.md) work. A physical replica is just a process that tails this log and reapplies it. CDC's insight: **anything can be that replica.** A search index, a cache, an analytics warehouse, another team's Kafka topic — point a log reader at the WAL and every committed change flows out, in commit order, with nothing missed, at millisecond latency, and with zero cost to the application (which doesn't even know it's happening).

Contrast the alternatives for keeping a second store in sync, and CDC's shape becomes obvious:

- **Dual writes from the app** ("save to DB, also write to Elasticsearch") — the crash-window problem the [outbox](../outbox-pattern/learning.md) exists for, plus interleaving races that reorder concurrent writes differently in each store. Drift is guaranteed at scale.
- **Polling** (`WHERE updated_at > last_check`) — misses deletes entirely, misses intermediate states, hammers the table, and its ordering is the *query's*, not the commit order.
- **CDC** — the database's own serialization of history, delete events included, ordered, complete.

The deeper frame (Kleppmann's): this **turns the database inside out** — the internal log, once an implementation detail, becomes the integration point, and downstream stores become materialized views over it. It's the same architecture as [event sourcing](../event-sourcing/learning.md) with one crucial inversion: event sourcing puts the log *first* (intent-carrying events are the system of record; tables derive), CDC puts the log *second* (tables are the record; the extracted log derives). That ordering difference decides what the stream can carry — which is the central limitation below.

Two prices, both structural: CDC events describe **row changes, not intent** (an `UPDATE` says *what* changed, never *why*); and the pipeline is a distributed system with the usual at-least-once duplicates, lag, and schema-coupling problems — plus one operational trap (slot retention) sharp enough to page you.

## Core Concepts

### Log-based capture (the real thing)

- **What it is:** A connector registers with the database as a *logical replication client* (Postgres: replication slot + logical decoding; MySQL: binlog client) and receives every committed change — insert, update, delete — decoded into structured records, in commit order.
- **Why it exists:** It's the only capture mechanism that is simultaneously complete (deletes included), ordered (the database's own commit order), low-overhead (reading a file the DB writes anyway — no queries, no triggers in the write path), and non-invasive (zero application change).
- **Example:** Debezium on Postgres: `CREATE PUBLICATION` + logical replication slot; `UPDATE accounts SET balance=120 WHERE id=42` surfaces as a change record with `before={balance:70}`, `after={balance:120}`, source position (LSN), and transaction id — milliseconds after commit.

### Trigger- and query-based capture (the fallbacks)

- **What it is:** Trigger-based: `AFTER INSERT/UPDATE/DELETE` triggers copy changes to a shadow table, which is then drained. Query-based: periodic polling on a timestamp/version column.
- **Why it exists:** For databases or managed tiers where log access isn't available. Know their costs: triggers execute *inside the write transaction* (latency tax on every write, and the drain is a polling problem anyway); polling misses deletes and intermediate states and loads the primary. Both are last resorts.
- **Example:** Polling `WHERE updated_at > $last` at 30 s: a row inserted and deleted within the window *never appears at all*; a row updated three times appears once with only the final state. If either matters, polling was the wrong tool.

### The change event

- **What it is:** The unit of the stream: operation type (c/u/d + r for snapshot reads), `before` and `after` row images, source metadata (position/LSN, transaction id, timestamp), and the key (primary key of the row).
- **Why it exists:** `before` images are what make deletes and key-changes processable downstream (a delete without the old image is just "something with id 42 vanished" — a cache invalidator needs to know *what* it was). Position metadata is what makes the pipeline resumable and deduplicable.
- **Example:** Kafka Connect convention: the event *key* is the row's PK (so a topic partition carries one row's history in order), the value carries before/after. Postgres nuance: full `before` images require `REPLICA IDENTITY FULL` on the table — a setting you must flip *before* you need it; default replica identity yields only the PK in `before`.

### Snapshot + streaming (the bootstrap problem)

- **What it is:** A new consumer needs current state *plus* changes — the WAL only reaches back so far. Connectors solve it with an initial **snapshot** (read all rows as synthetic "read" events) then a seamless switch to streaming from the position where the snapshot began.
- **Why it exists:** Without a coherent handoff, you get gaps (snapshot at t₀, streaming from t₁ > t₀ misses the interval) or floods of conflicts. The connector's snapshot-position bookkeeping is precisely what makes "spin up a new downstream store from scratch" a routine operation instead of a migration project.
- **Example:** Debezium initial snapshot of a 200 GB table streams `r`-events for hours *while writes continue*; it records the WAL position at snapshot start and replays the overlap afterward — consumers see snapshot rows possibly followed by slightly-older changes for the same keys, then clean ordered flow. (Consumers must therefore tolerate that overlap — idempotent upserts by key handle it naturally.)

### Replication slots and retention (the operational heart)

- **What it is:** The database's contract with a logical consumer: a **replication slot** (Postgres) marks the consumer's position and *pins WAL from that point forward* — the DB will not delete log segments an acknowledged-behind consumer still needs. MySQL differs: binlog retention is a global time/size setting, not per-consumer.
- **Why it exists:** It's what makes CDC lossless across connector restarts: the log waits for you. The same mechanism is the trap: the log waits for you *even if you never come back*.
- **Example:** Connector decommissioned; slot not dropped. Postgres pins WAL forever; disk fills over days; the *primary* stops accepting writes. A stopped-but-not-removed consumer is a time bomb — slot lag monitoring plus `max_slot_wal_keep_size` (bounding the damage by sacrificing the slot) is non-optional production hygiene.

## Worked Example

Goal: product rows in Postgres must appear in Elasticsearch for search, with deletes handled, without touching the product service's code.

**1. The naive version, for contrast.** Product service dual-writes: `UPDATE products` then `es.index(doc)`. Crash between → drift. Two concurrent updates racing to ES in the wrong order → *permanent* wrong search doc (last-writer-to-ES wins, which isn't last-committer-to-PG). And the delete path was forgotten entirely — sold-out products haunt search for months.

**2. The CDC pipeline.**

```
products table ──WAL──► Debezium connector ──► topic products.changes (key = product_id)
                                                    │
                                              sink consumer ──► Elasticsearch
```

Setup: `ALTER TABLE products REPLICA IDENTITY FULL;` publication + slot; connector snapshots existing rows (`r` events), then streams.

**3. The flow, per operation.**

```
INSERT product p-77                → {op:c, after:{id:p-77, name:"...", stock:3}}   → ES index doc p-77
UPDATE stock 3→0                   → {op:u, before:{stock:3}, after:{stock:0}}      → ES update doc p-77
DELETE p-77                        → {op:d, before:{id:p-77, ...}}                  → ES delete doc p-77
```

Per-row ordering holds because the event key is the PK: all of p-77's history sits in one partition, in commit order. The sink upserts by document id = product id, making replays and duplicates harmless (idempotent by natural key — the [idempotency](../idempotency-and-delivery-semantics/learning.md) playbook).

**4. A rebuild, free of drama.** Search team wants a new index with different analyzers: stand up a new sink from a fresh snapshot (or replay the topic if retention allows), build the new index in parallel, flip the alias. The primary never noticed. This — downstream stores as cheap, rebuildable materialized views — is the payoff that keeps CDC pipelines multiplying once one exists.

**5. What this pipeline is *not*.** The `products.changes` topic is row-diffs. If the pricing team now asks "emit an event when a product is discounted" — that's *intent*, and `{op:u, before:{price:90}, after:{price:75}}` is not `ProductDiscounted` (was it a sale? a correction? a currency fix?). Deriving intent from diffs is guesswork that breaks on the next schema change. Intent must be *authored* at write time → [outbox](../outbox-pattern/learning.md) with an explicit event, or event sourcing. Knowing where this line sits is knowing when CDC is the right tool.

## Pitfalls in Depth

### Pitfall: Row-diffs mistaken for domain events

- **What goes wrong:** Teams wire business logic to raw CDC topics: "when `orders.status` changes to `'cancelled'`, refund." Then a backfill script touches status, a migration rewrites rows, an admin fixes a typo — refunds fire. Consumers accumulate fragile inference (`if before.x != after.x and ...`) that encodes guesses about *why* rows change, and the guesses rot.
- **Why it happens (the mechanism):** The WAL records effects, not causes — intent existed only in the application code path that issued the write, and CDC sits below that. Any "event" reconstructed from a diff is an inference, and every new write path (backfills, migrations, admin tools) silently violates it.
- **How to handle it in production, and why that works:** Draw the line by consumer type: **technical replication** (caches, search, warehouse — consumers that want state copies) reads CDC directly; **business reactions** (refunds, notifications, workflow) consume *authored* events via the [outbox](../outbox-pattern/learning.md), written in the same transaction by the code that knows the why. CDC can even transport the outbox (log-tailing relay) — the distinction is what's *in* the payload, not the plumbing.
- **Trade-offs of the fix:** Two streams to run and a judgment call per consumer. The discipline pays the first time someone runs a backfill without fear of triggering refunds.

### Pitfall: The forgotten replication slot fills the primary's disk

- **What goes wrong:** A connector is paused for an incident, decommissioned without cleanup, or just crash-loops unnoticed. Its slot dutifully pins WAL; disk usage climbs linearly for days; then the primary hits the wall and stops accepting *all* writes. A downstream convenience feature takes down the source of truth.
- **Why it happens (the mechanism):** The slot contract is deliberately protective — never throw away log a registered consumer hasn't confirmed — and has no natural backpressure to the operator: the cost accrues silently on the primary's disk, far from the team that owns the stalled consumer.
- **How to handle it in production, and why that works:** Three layers: **alert** on slot lag bytes (`pg_replication_slots.restart_lsn` distance) with thresholds far below disk capacity; **bound** the blast radius with `max_slot_wal_keep_size` (Postgres 13+ — a slot exceeding it is invalidated: the consumer breaks instead of the primary, the correct sacrifice); **lifecycle** slots as code (created/destroyed by the same automation as the connector, so "decommissioned connector, live slot" can't happen).
- **Trade-offs of the fix:** An invalidated slot means that consumer must re-snapshot — expensive, but strictly better than a down primary. The real cost is remembering that CDC made *downstream health a primary-database concern*; that mental link is the fix.

### Pitfall: Schema coupling — every consumer inherits your DDL

- **What goes wrong:** `ALTER TABLE products RENAME COLUMN name TO title` — and the search sink, two warehouse jobs, and another team's consumer all break at once. The table schema has become a public API with unknown consumers, and ordinary migrations now need cross-team choreography.
- **Why it happens (the mechanism):** CDC events are generated *from* the table structure, so DDL flows into every payload. The coupling is total and invisible — nothing in the database tells you who's downstream, and nothing in a consumer tells the DBA it exists.
- **How to handle it in production, and why that works:** Interpose a contract: run change events through a **schema registry with compatibility rules** (additive-only evolves freely; breaking changes are rejected at the connector) — turning "consumer broke at runtime" into "migration rejected at CI." For deliberate breaks, version the topic (`products.v2`), run both during consumer migration. Organizationally: a table with CDC consumers is marked as having a published interface, and its migrations get API-change review.
- **Trade-offs of the fix:** Registry + compatibility discipline constrains DDL freedom on captured tables — which is honest: they *are* interfaces now. The outbox alternative (decouple by authoring payloads) trades this problem for a mapping layer.

### Pitfall: Expecting cross-table transactional atomicity downstream

- **What goes wrong:** A transaction updates `orders` and `order_lines` together; downstream, the warehouse loads them via separate topics/sinks and shows a header with no lines (or totals that don't add up) — a state that *never existed* in the source. Analysts file data-quality bugs against numbers that are merely mid-flight.
- **Why it happens (the mechanism):** The WAL is transactionally ordered, but the pipeline shreds it: events route to per-table topics consumed at independent rates. Atomicity survives capture and dies in transport. (Same family as the [replication](../replication-and-consistency/learning.md) causal anomalies — the observer sees effects from different points in time.)
- **How to handle it in production, and why that works:** Usually: **design downstream to tolerate it** — idempotent upserts per row converge; queries join on arrival ("eventual join"); readers of the warehouse learn that in-flight seconds are fuzzy. When true transactional grouping matters: connectors emit transaction markers/metadata (Debezium's transaction topic) letting a stateful consumer buffer until a transaction's events are complete before applying — real machinery, used sparingly. Sometimes the honest fix is upstream: emit one authored event carrying the whole consistent picture (outbox again).
- **Trade-offs of the fix:** Buffering consumers add state, latency, and failure modes; tolerate-and-converge is cheaper but pushes the fuzziness to data consumers. Pick per pipeline; don't pretend the problem away.

### Pitfall: Snapshot pain at scale (the bootstrap that eats the weekend)

- **What goes wrong:** Adding CDC to a 2 TB table: the initial snapshot runs 30 hours, holds long transactions that stall vacuum (bloat piles up), saturates the connector, and — if it dies at hour 29 — restarts from zero. Meanwhile the change stream can't begin until the snapshot completes.
- **Why it happens (the mechanism):** Bootstrap must produce a consistent image *coordinated with a log position* while the table keeps changing; the naive implementation is one giant repeatable-read transaction — exactly the thing large OLTP tables punish.
- **How to handle it in production, and why that works:** Use **incremental snapshotting** (Debezium's watermark-based approach — DBLog-style): the table is chunked by PK, chunks are read in short transactions *interleaved with live streaming*, watermarks resolve chunk-vs-stream conflicts. Resumable, pausable, no long transaction, stream flows from minute one. For one-off migrations, a backup-restore into the sink + start-stream-from-backup-LSN sidesteps snapshotting entirely.
- **Trade-offs of the fix:** Incremental snapshots interleave old (`r`) and new (`u`/`d`) events for the same keys — consumers must upsert idempotently (which they needed anyway). The backup trick requires position bookkeeping done exactly right, once.

## Design Decisions & Trade-offs

**CDC vs. outbox — the decision, condensed.** What does the consumer need? *State copies* (search, cache, warehouse, replicas): CDC, directly on the tables — no app changes, deletes included. *Business meaning* (anything that triggers money, messages, or workflow): authored events via outbox. Mixed systems run both, often through the same connector infrastructure. The wrong choice in either direction hurts: outbox-for-replication means hand-maintaining "events" that are really just row images; CDC-for-business-logic means inference rot.

**Capture mechanism:** log-based always, if the platform allows (self-hosted or a managed tier exposing logical replication — most do now). Triggers only when it doesn't; polling only when even triggers can't be had *and* deletes/intermediate states don't matter.

**Deployment shape:** the dominant stack is connector-on-Kafka-Connect (Debezium) → topics → sink connectors, which buys you the ecosystem (offsets, rebalancing, sinks, registry). Lighter-weight: embedded connector in your own service, or direct logical-replication client (Rust: `pgoutput` decoding via `tokio-postgres`'s replication support or the `pg_replicate`-style crates) when Kafka isn't in the picture — you take on offset management and delivery yourself.

**Guarantees to design against (not hope around):** at-least-once delivery (duplicates on connector restart — sinks upsert by key or dedup by position); per-key ordering only (cross-table, cross-key interleaving is real); seconds-scale lag normally, unbounded during incidents — downstream reads are [eventually consistent](../replication-and-consistency/learning.md), with everything that implies for read-your-writes.

**The one page to write before production:** slot-lag alert thresholds, `max_slot_wal_keep_size` value and the re-snapshot runbook for an invalidated slot, schema-registry compatibility mode, and per-pipeline answer to "what happens during a 6-hour connector outage?" (WAL pinning on the primary + catch-up burst on recovery — both need headroom).

## Open Questions

- Postgres logical replication protocol details: what exactly does `pgoutput` emit per operation, and what do failover slots (PG 17) change about connector HA?
- Rust CDC clients: evaluate the current state of `pg_replicate` / `supabase-etl` (or successors) vs. embedded Debezium for a Kafka-less pipeline — offset management ergonomics in particular.
- Incremental snapshot internals: walk through the DBLog watermark algorithm until the chunk/stream conflict resolution is obvious enough to re-derive.
- What does Debezium's transaction-metadata topic actually contain, and how much consumer code does transaction-buffering take in practice?
- Measure: end-to-end p99 latency (commit → ES visible) and connector-restart catch-up behavior on a realistic write load.

## References

- Martin Kleppmann, *Designing Data-Intensive Applications*, ch. 11 — CDC, "turning the database inside out," and derived data as the unifying frame; the intellectual backbone.
- Martin Kleppmann, "Turning the Database Inside-Out" (talk, 2014) — the argument in its most memorable form.
- [Debezium documentation](https://debezium.io/documentation/) — the reference implementation's semantics: snapshots (incremental included), replica identity, transaction metadata, outbox router. Read the Postgres connector page end to end once.
- Netflix, "DBLog: A Generic Change-Data-Capture Framework" (blog/paper) — the watermark-based incremental snapshot algorithm Debezium adopted.
- Related topics in this repo: [Outbox Pattern](../outbox-pattern/learning.md) (authored intent vs. row diffs — the sibling decision), [Event Sourcing & CQRS](../event-sourcing/learning.md) (log-first vs. log-derived), [Replication & Consistency](../replication-and-consistency/learning.md) (CDC consumers are replicas; all lag anomalies apply), [Caching Strategies](../caching-strategies/learning.md) (CDC-driven invalidation as the drift-free feed).
