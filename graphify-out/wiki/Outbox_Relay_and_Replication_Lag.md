# Outbox Relay and Replication Lag

> 9 nodes · cohesion 0.22

## Key Concepts

- **Polling Relay (Publisher)** (4 connections) — `architecture-patterns/outbox-pattern/learning.md`
- **Log-Tailing Relay (Outbox via CDC / Debezium)** (3 connections) — `architecture-patterns/outbox-pattern/learning.md`
- **Derived Copies Lag the Source (Unifying Frame)** (3 connections) — `architecture-patterns/replication-and-consistency/learning.md`
- **Single-Leader Replication** (3 connections) — `architecture-patterns/replication-and-consistency/learning.md`
- **Replication Lag Anomalies (Read-Your-Writes, Monotonic, Causal)** (2 connections) — `architecture-patterns/replication-and-consistency/learning.md`
- **The `id > last_seen` Poll Skips Rows Forever** (1 connections) — `architecture-patterns/outbox-pattern/learning.md`
- **Morling/Debezium, Reliable Microservices Data Exchange With the Outbox Pattern** (1 connections) — `architecture-patterns/outbox-pattern/learning.md`
- **Per-Aggregate Ordering and Partition Key** (1 connections) — `architecture-patterns/outbox-pattern/learning.md`
- **Decision Reads vs Display Reads (Per-Query Routing)** (1 connections) — `architecture-patterns/replication-and-consistency/learning.md`

## Relationships

- [Delivery Semantics and Idempotency](Delivery_Semantics_and_Idempotency.md) (1 shared connections)
- [Strangler Fig Migration](Strangler_Fig_Migration.md) (1 shared connections)
- [Shard Keys and SCC Algorithms](Shard_Keys_and_SCC_Algorithms.md) (1 shared connections)

## Source Files

- `architecture-patterns/outbox-pattern/learning.md`
- `architecture-patterns/replication-and-consistency/learning.md`

## Audit Trail

- EXTRACTED: 9 (82%)
- INFERRED: 2 (18%)
- AMBIGUOUS: 0 (0%)

---

*Part of the graphify knowledge wiki. See [index](index.md) to navigate.*