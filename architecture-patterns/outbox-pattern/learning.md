# Outbox Pattern — Learning Notes

## Mental Model

**You cannot atomically write to two systems. So write to one, and let the second write be *derived* from the first.**

The problem it solves is the **dual write**: a service commits a business change to its database *and* publishes a message about it to a broker. Those are two independent systems; no transaction spans them. Whatever order you pick, a crash between the two writes breaks the story:

```
commit DB, then publish   → crash between → state changed, world never told   (silent divergence)
publish, then commit DB   → crash between → world told about a change that never happened (phantom)
```

Both failure modes are silent, rare, and corrosive — downstream systems slowly drift from the source of truth, and nobody can say when or why.

The outbox insight: **turn the two writes into one.** In the *same local transaction* that changes business state, insert the message into an `outbox` table in the same database. The transaction commits or doesn't — state change and intent-to-publish are now atomically inseparable. A separate **relay** process then reads committed outbox rows and publishes them to the broker, retrying until acknowledged. The relay can crash anywhere and nothing is lost — the message is durably parked in the database until delivery succeeds.

What you've really built is a small, local event log feeding an asynchronous publisher — the same "derive the second write from the first" move as [event sourcing](../event-sourcing/learning.md) (where the log *is* the primary write) and [CDC](../change-data-capture/learning.md) (where the database's own WAL plays the outbox). The price, and it's non-negotiable: the relay retries, so delivery is **at-least-once** — every consumer must be idempotent ([delivery semantics](../idempotency-and-delivery-semantics/learning.md)). Guaranteed delivery is bought by accepting duplicates.

## Core Concepts

### The outbox table

- **What it is:** A table in the service's own database: `outbox(id, aggregate_id, event_type, payload, created_at, published_at NULL)`. Written only inside business transactions; read only by the relay.
- **Why it exists:** It's the bridge that lets a plain ACID transaction cover "change state + record message." The table is infrastructure, not domain — services write it and forget it.
- **Example:** Order placement, one transaction: `INSERT INTO orders ...; INSERT INTO outbox (aggregate_id, event_type, payload) VALUES ('ord-991', 'OrderPlaced', '{...}'); COMMIT`. Crash before commit: neither exists. After: both exist, delivery is now the relay's problem — the crash window is gone by construction.

### The relay (polling publisher)

- **What it is:** A loop: `SELECT * FROM outbox WHERE published_at IS NULL ORDER BY id LIMIT 100` → publish each to the broker → on broker ack, set `published_at` (or delete). Crash anywhere → rows stay unpublished → next pass retries.
- **Why it exists:** It converts "durably recorded intent" into "delivered message" with retry until success. Note it has its own mini dual-write (publish, then mark published) — a crash between the two *republishes*. That's deliberate: the ambiguity is resolved toward duplication, never loss, because duplicates are handleable (idempotency) and loss is invisible.
- **Example:** Relay publishes outbox row 4711 to Kafka, gets the ack, crashes before `UPDATE`. On restart, 4711 is published again. Consumer dedups on `event_id = 4711`. Nothing lost; nobody paged.

### Log-tailing relay (outbox via CDC)

- **What it is:** Instead of polling, a CDC connector (e.g. Debezium's outbox event router) tails the database's WAL, sees committed outbox inserts within milliseconds, and publishes them.
- **Why it exists:** Removes polling latency and query load, and rides infrastructure you may already run. The outbox *table* still exists — CDC replaces only the *relay's read path*.
- **Example:** Debezium reads the Postgres WAL, extracts `outbox` inserts, routes `payload` to a topic per `event_type`. The table can be pruned aggressively (even inserted-and-deleted in the same transaction — the WAL still carries the insert; a Debezium-supported trick that keeps the table permanently near-empty).

### Ordering and partitioning

- **What it is:** The discipline that events for one aggregate arrive in the order they were committed: relay reads in commit order (`ORDER BY id`), publishes with `aggregate_id` as the partition key, so one aggregate's events land in one partition, in order.
- **Why it exists:** Consumers reasonably assume per-aggregate order (`OrderPlaced` before `OrderShipped`). Global order across aggregates is neither promised nor needed — chasing it serializes everything through one partition and caps throughput for no consumer benefit.
- **Example:** Events for `ord-991` → partition `hash('ord-991')`, strictly ordered. Events for `ord-992` may interleave arbitrarily relative to 991. A parallelized relay must preserve exactly this: parallel across aggregates, serial within one.

### Outbox vs. listen/notify vs. direct CDC on business tables

- **What it is:** The adjacent options. `LISTEN/NOTIFY`-style triggers: fast but fire-and-forget (a listener that's down misses the notification — no durability, not a substitute). Direct CDC on business tables: no outbox table at all — but your *table schema* becomes the published contract, and intent is lost (an `UPDATE orders SET status='shipped'` row-change is not an `OrderShipped` domain event; consumers must reverse-engineer meaning).
- **Why it exists:** The outbox's under-appreciated second job: the payload is **authored at write time** — an intentional, versioned contract carrying business meaning, decoupled from table layout. That's the same internal-vs-integration-events boundary event sourcing draws.
- **Example:** Team A ships direct CDC on `orders`; team B builds consumers parsing row images. Team A renames a column — every consumer breaks. With an outbox, the rename is invisible; the `OrderPlaced` contract didn't change.

## Worked Example

Order service (Postgres) must tell the warehouse service (via Kafka) to ship.

**1. The broken version first.**

```
BEGIN; INSERT INTO orders VALUES ('ord-991', 'placed'); COMMIT;
kafka.publish("OrderPlaced", ord-991)        -- ← crash HERE
```

Order exists; warehouse never hears. No error anywhere — the gap surfaces days later as "customer paid, nothing shipped." Reordering (publish first) just flips it: warehouse ships an order that rolled back.

**2. The outbox version.**

```
BEGIN;
  INSERT INTO orders  VALUES ('ord-991', 'placed');
  INSERT INTO outbox  VALUES (4711, 'ord-991', 'OrderPlaced', '{"order_id":"ord-991", ...}', now(), NULL);
COMMIT;                                       -- atomic: both or neither
```

**3. The relay pass.**

```
SELECT * FROM outbox WHERE published_at IS NULL ORDER BY id LIMIT 100;   -- → row 4711
kafka.publish(topic="orders", key="ord-991", value=payload)              -- broker acks
UPDATE outbox SET published_at = now() WHERE id = 4711;
```

Crash matrix: before publish → row waits, retried next pass (delayed, not lost). After publish, before update → republished next pass (duplicated, not lost). Every path degrades to *duplicate or delay* — never loss. That asymmetry is the entire point.

**4. The consumer closes the loop.**

```
warehouse, one transaction:
  INSERT INTO processed_events(event_id) VALUES (4711);   -- unique constraint: duplicate → abort, ack, done
  INSERT INTO shipment_jobs(order_id) VALUES ('ord-991');
COMMIT; then ack Kafka offset
```

**5. Steady state:** monitor two numbers — unpublished outbox depth (relay health) and oldest-unpublished age (delivery lag). Prune published rows. That's the whole operational surface.

## Pitfalls in Depth

### Pitfall: Publishing directly "just this once" (bypassing the outbox)

- **What goes wrong:** The pattern is in place, but a new feature publishes straight to the broker from request-handling code — it's less typing, and works in every test. The dual-write crash window is back on that path; months later one message in ten thousand vanishes during a deploy-time crash, and the drift investigation starts from zero because "we use the outbox pattern."
- **Why it happens (the mechanism):** The outbox is a *discipline*, not a feature flag — nothing structurally prevents direct publishes; each one is individually harmless-looking and only fails under crash timing nobody reproduces locally.
- **How to handle it in production, and why that works:** Remove the temptation structurally: services get no broker producer credentials at all — only the relay publishes; application code's only "publish" API is the outbox insert (wrap it in the repository/UoW layer so the domain never sees either). A lint/architecture test failing on broker-client imports outside the relay makes the rule reviewable.
- **Trade-offs of the fix:** All messaging gains the relay's latency (typically tens of ms with CDC, up to the poll interval otherwise); genuinely fire-and-forget telemetry may justify a sanctioned direct path — make it an explicit, named exception, not a convenience.

### Pitfall: Polling relay melts the database (or lags into uselessness)

- **What goes wrong:** Poll interval tuned tight (50 ms) to minimize latency: the unpublished-scan plus `UPDATE` per message becomes the table's hottest workload, bloating it (each row written twice) and stealing capacity from the actual business. Tuned loose (5 s): every cross-service flow inherits multi-second lag, and the [sagas](../saga-pattern/learning.md) built on these events crawl.
- **Why it happens (the mechanism):** Polling cost scales with frequency *regardless of traffic* — the empty-table scan isn't free either (dead published rows, index bloat). Latency scales inversely. There is no good static point under bursty load; the knob is fighting physics.
- **How to handle it in production, and why that works:** Escape the trade rather than tune it: **log-tailing (CDC) relay** — WAL push gives ~ms latency at zero query load; the poll loop remains only as fallback. If polling must stay: partial index `WHERE published_at IS NULL` (scan cost tracks *backlog*, not table size), batch publishes, delete-instead-of-update published rows, and adaptive intervals (tight while draining, backing off when idle).
- **Trade-offs of the fix:** CDC adds an infrastructure component (connector + its own offset/state management, slot monitoring on Postgres) — real operational weight, usually worth it the moment more than one service uses the pattern. Delete-on-publish trades the audit trail for table health; keep payload history elsewhere if you need it.

### Pitfall: Parallel relay breaks per-aggregate ordering

- **What goes wrong:** One relay thread can't keep up; someone runs four, each grabbing unpublished rows (`FOR UPDATE SKIP LOCKED`). Two events for the same order land in different threads; thread B publishes `OrderShipped` before thread A publishes `OrderPlaced`. Consumers see effect before cause — the warehouse discards or misprocesses, intermittently, only under load.
- **Why it happens (the mechanism):** `SKIP LOCKED` distributes rows with no affinity; ordering within an aggregate survives only if one publisher owns that aggregate's rows at a time. Broker partitioning can't repair it — Kafka preserves the order *of arrival*, and arrival order is what broke.
- **How to handle it in production, and why that works:** Parallelize *by aggregate*, never by row: shard rows across workers by `hash(aggregate_id)` so each aggregate has exactly one publisher (preserving commit order within the shard); or keep one relay and batch harder (a single thread publishing 500-row batches sustains surprisingly high throughput — measure before sharding). Consumers should still tolerate rare misordering (drop stale by version, or buffer-and-reorder) as defense in depth.
- **Trade-offs of the fix:** Hash-sharding pins a hot aggregate's throughput to one worker (correct — its order *demands* serialization; the ceiling is intrinsic). Consumer-side reordering logic is complexity you hope never fires; version-stamping events makes it cheap.

### Pitfall: Unbounded outbox growth

- **What goes wrong:** Published rows are never pruned. The table quietly becomes the largest in the database; the unpublished scan slows (index bloat even with partial indexes), vacuum runs long, backups balloon, and one day a migration on the outbox table locks a 400 GB relic for hours.
- **Why it happens (the mechanism):** Nothing *functionally* breaks as it grows — no error, no failed request — so pruning never makes the sprint. The cost accrues as background drag until it crosses an operational cliff.
- **How to handle it in production, and why that works:** Decide the retention story on day one, as part of the pattern: delete-on-publish (leanest; the broker/log downstream is the history), TTL pruning job (`DELETE WHERE published_at < now() - '7 days'`, batched to avoid lock storms), or native partitioning by day with partition drops (cheapest deletes at scale). With CDC, the insert-and-delete-in-same-transaction trick keeps the table empty *by construction*.
- **Trade-offs of the fix:** Pruning forfeits the outbox as an audit/replay source — if replay matters, that's a job for the broker's retention or an [event-sourced](../event-sourcing/learning.md) log, not the outbox (it was never the system of record for history, only for *undelivered intent*).

### Pitfall: Outbox payload becomes a leaked internal contract

- **What goes wrong:** Payloads are serialized straight from internal domain objects (`json(order)`). Every consumer now couples to the service's internal shape; an internal refactor (rename, restructure) breaks other teams' consumers — the microservice boundary has silently dissolved into a shared schema.
- **Why it happens (the mechanism):** At write time, the internal object is *right there*; designing a separate contract feels like duplication. The coupling is invisible until the first internal change that was supposed to be private.
- **How to handle it in production, and why that works:** Treat the outbox payload as a **published API**: an explicit integration-event schema (its own type, versioned, mapped deliberately from domain state at write time), evolved additively, checked in a schema registry or contract tests. This is the same internal-vs-integration boundary as event sourcing's — the outbox is precisely where that translation belongs, because it's the door out of the service.
- **Trade-offs of the fix:** A mapping layer to maintain and the perennial "it's the same fields, why the ceremony" argument — until the first painless internal refactor pays for all of it.

## Design Decisions & Trade-offs

**Polling vs. CDC relay.** Start with polling: no new infrastructure, one afternoon to build, entirely adequate for one service at modest volume. Move to CDC (Debezium or equivalent) when latency, database load, or a proliferation of per-service relays starts to hurt. The application-visible contract (outbox insert in the business transaction) is identical for both — you can swap the relay later without touching services, which is what makes "start simple" safe here.

**Delete vs. mark-published.** Mark-and-prune is friendlier while developing (visible history, easy debugging); delete-on-publish (or same-transaction delete with CDC) is the terminal state for table health. Either way: the decision is *when* rows leave, not whether.

**One outbox per service database — never shared.** The pattern's atomicity derives from the outbox living in the *same database* as the business tables. A centralized "outbox service" reintroduces the dual write it was invented to kill.

**Scope honestly.** The outbox guarantees *this service's committed changes get published, at least once, in per-aggregate order*. It does not orchestrate multi-service workflows (that's [sagas](../saga-pattern/learning.md), which *use* it as their reliable-messaging substrate), doesn't dedup for consumers, and doesn't replace an event store (it records undelivered intent, not history). Most "outbox didn't work" stories are one of these misassignments.

**When is it overkill?** When the database's own change feed is genuinely sufficient (pure cache-invalidation/search-indexing pipelines with no intent semantics — direct [CDC](../change-data-capture/learning.md) is less machinery), or when loss is truly acceptable (metrics). The moment a *business process* hangs on the message, the outbox is the floor, not gold-plating.

## Open Questions

- Measure: single-threaded batched relay on Postgres — what msgs/sec before sharding is actually needed on our hardware?
- Debezium outbox router specifics: configuration for the insert-and-delete-in-one-transaction pattern, and what happens to its offsets across connector restarts?
- Postgres replication-slot risk with CDC: an abandoned slot pins WAL and can fill the disk — what monitoring/auto-drop policy makes this safe to run unattended?
- Rust implementation: sketch a relay (sqlx + rdkafka) with `SKIP LOCKED` batching and hash-sharding — where do the tricky bits actually live?
- How do teams contract-test outbox payloads against consumers in practice — schema registry with compatibility rules, or consumer-driven contracts (Pact-style)?

## References

- Chris Richardson, [Transactional Outbox](https://microservices.io/patterns/data/transactional-outbox.html) — the canonical pattern write-up, with the polling/log-tailing split.
- Gunnar Morling / Debezium, [Reliable Microservices Data Exchange With the Outbox Pattern](https://debezium.io/blog/2019/02/19/reliable-microservices-data-exchange-with-the-outbox-pattern/) — the definitive CDC-relay implementation guide, including the event-router and table-pruning tricks.
- Martin Kleppmann, *Designing Data-Intensive Applications*, ch. 11 — the "derive all other state from one log" framing that situates outbox, CDC, and event sourcing as one family.
- Related topics in this repo: [Idempotency & Delivery Semantics](../idempotency-and-delivery-semantics/learning.md) (the consumer half — mandatory pairing), [Change Data Capture](../change-data-capture/learning.md) (the log-tailing relay generalized), [Event Sourcing & CQRS](../event-sourcing/learning.md) (dual-write is why its store must be the only write), [Saga Pattern](../saga-pattern/learning.md) (built on outbox-grade messaging).
