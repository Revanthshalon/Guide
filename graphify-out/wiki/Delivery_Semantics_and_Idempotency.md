# Delivery Semantics and Idempotency

> 46 nodes · cohesion 0.05

## Key Concepts

- **The Outbox Table** (9 connections) — `architecture-patterns/outbox-pattern/learning.md`
- **Saga (Local Transactions with Compensations)** (9 connections) — `architecture-patterns/saga-pattern/learning.md`
- **Effectively-Once (At-Least-Once + Idempotent Receiver)** (7 connections) — `architecture-patterns/idempotency-and-delivery-semantics/reference.md`
- **Consistency Model as a Spectrum of Promises** (5 connections) — `architecture-patterns/replication-and-consistency/learning.md`
- **Linearizability** (5 connections) — `architecture-patterns/replication-and-consistency/learning.md`
- **Orchestration (Saga Executor / Persisted State Machine)** (5 connections) — `architecture-patterns/saga-pattern/learning.md`
- **At-Least-Once Delivery** (4 connections) — `architecture-patterns/idempotency-and-delivery-semantics/reference.md`
- **Dedup Key Table (Key + Effect in One Transaction)** (3 connections) — `architecture-patterns/idempotency-and-delivery-semantics/reference.md`
- **The Dual Write Problem** (3 connections) — `architecture-patterns/outbox-pattern/learning.md`
- **Causal and Eventual Consistency** (3 connections) — `architecture-patterns/replication-and-consistency/learning.md`
- **PACELC** (3 connections) — `architecture-patterns/replication-and-consistency/learning.md`
- **Semantic Lock (Reserve/Confirm/Release Idiom)** (3 connections) — `architecture-patterns/saga-pattern/learning.md`
- **Kleppmann, Designing Data-Intensive Applications** (2 connections) — `architecture-patterns/idempotency-and-delivery-semantics/reference.md`
- **Timeout as Unresolvable Ambiguity** (2 connections) — `architecture-patterns/idempotency-and-delivery-semantics/reference.md`
- **Outbox Payload as a Published Integration-Event Contract** (2 connections) — `architecture-patterns/outbox-pattern/learning.md`
- **Compensating Action (Compensation ≠ Rollback)** (2 connections) — `architecture-patterns/saga-pattern/learning.md`
- **Parallel Branches (Fork/Join) and Partial Compensation** (2 connections) — `architecture-patterns/saga-pattern/learning.md`
- **The Pivot and the Three Step Zones** (2 connections) — `architecture-patterns/saga-pattern/learning.md`
- **Routing Layer and the Topology Map** (2 connections) — `architecture-patterns/sharding/learning.md`
- **Scatter-Gather Query (Latency = max over shards)** (2 connections) — `architecture-patterns/sharding/learning.md`
- **Local vs Global Secondary Indexes in a Sharded World** (2 connections) — `architecture-patterns/sharding/learning.md`
- **Anti-Corruption Layer** (2 connections) — `architecture-patterns/strangler-fig/learning.md`
- **Sync Direction: New-Owns-With-Sync-Back Makes Progress Monotonic** (2 connections) — `architecture-patterns/strangler-fig/learning.md`
- **Copy-on-Write B-Trees (Btrfs, LMDB — Root Swap as Atomic Commit)** (2 connections) — `data-structures-and-algorithms/b-trees/learning.md`
- **At-Most-Once Delivery** (1 connections) — `architecture-patterns/idempotency-and-delivery-semantics/reference.md`
- *... and 21 more nodes in this community*

## Relationships

- [Outbox Relay and Replication Lag](Outbox_Relay_and_Replication_Lag.md) (1 shared connections)
- [Shard Keys and SCC Algorithms](Shard_Keys_and_SCC_Algorithms.md) (1 shared connections)
- [B-Trees and Low-Link Origins](B-Trees_and_Low-Link_Origins.md) (1 shared connections)

## Source Files

- `architecture-patterns/idempotency-and-delivery-semantics/reference.md`
- `architecture-patterns/outbox-pattern/learning.md`
- `architecture-patterns/replication-and-consistency/learning.md`
- `architecture-patterns/saga-pattern/learning.md`
- `architecture-patterns/sharding/learning.md`
- `architecture-patterns/strangler-fig/learning.md`
- `data-structures-and-algorithms/b-trees/learning.md`

## Audit Trail

- EXTRACTED: 48 (89%)
- INFERRED: 6 (11%)
- AMBIGUOUS: 0 (0%)

---

*Part of the graphify knowledge wiki. See [index](index.md) to navigate.*