---
type: community
cohesion: 0.05
members: 46
---

# Delivery Semantics and Idempotency

**Cohesion:** 0.05 - loosely connected
**Members:** 46 nodes

## Members
- [[Abadi, Consistency Tradeoffs in Modern Distributed Database System Design]] - paper - architecture-patterns/replication-and-consistency/learning.md
- [[Anti-Corruption Layer]] - concept - architecture-patterns/strangler-fig/learning.md
- [[At-Least-Once Delivery]] - concept - architecture-patterns/idempotency-and-delivery-semantics/reference.md
- [[At-Most-Once Delivery]] - concept - architecture-patterns/idempotency-and-delivery-semantics/reference.md
- [[CAP (as a Slogan)]] - concept - architecture-patterns/replication-and-consistency/learning.md
- [[Causal and Eventual Consistency]] - concept - architecture-patterns/replication-and-consistency/learning.md
- [[Choreography (Event-Chained Sagas)]] - concept - architecture-patterns/saga-pattern/learning.md
- [[Compensating Action (Compensation ≠ Rollback)]] - rationale - architecture-patterns/saga-pattern/learning.md
- [[Consistency Model as a Spectrum of Promises]] - rationale - architecture-patterns/replication-and-consistency/learning.md
- [[Copy-on-Write B-Trees (Btrfs, LMDB — Root Swap as Atomic Commit)]] - concept - data-structures-and-algorithms/b-trees/learning.md
- [[Dedup Key Table (Key + Effect in One Transaction)]] - rationale - architecture-patterns/idempotency-and-delivery-semantics/reference.md
- [[Effectively-Once (At-Least-Once + Idempotent Receiver)]] - rationale - architecture-patterns/idempotency-and-delivery-semantics/reference.md
- [[Evans, Domain-Driven Design]] - document - architecture-patterns/strangler-fig/learning.md
- [[Garcia-Molina & Salem, Sagas (SIGMOD 1987)]] - paper - architecture-patterns/saga-pattern/learning.md
- [[Helland, Life Beyond Distributed Transactions]] - paper - architecture-patterns/idempotency-and-delivery-semantics/reference.md
- [[Jepsen Analyses and the Consistency Models Map]] - document - architecture-patterns/replication-and-consistency/learning.md
- [[Kleppmann, Designing Data-Intensive Applications]] - paper - architecture-patterns/idempotency-and-delivery-semantics/reference.md
- [[Leaderless Quorum Replication (W + R  N)]] - concept - architecture-patterns/replication-and-consistency/learning.md
- [[Ledger Keyed by Business Id (Deltas vs Absolutes)]] - rationale - architecture-patterns/idempotency-and-delivery-semantics/reference.md
- [[Linearizability_1]] - concept - architecture-patterns/replication-and-consistency/learning.md
- [[Local vs Global Secondary Indexes in a Sharded World]] - concept - architecture-patterns/sharding/learning.md
- [[Orchestration (Saga Executor  Persisted State Machine)]] - concept - architecture-patterns/saga-pattern/learning.md
- [[Outbox Payload as a Published Integration-Event Contract]] - rationale - architecture-patterns/outbox-pattern/learning.md
- [[Outbox Retention  Pruning Policy]] - rationale - architecture-patterns/outbox-pattern/learning.md
- [[PACELC]] - concept - architecture-patterns/replication-and-consistency/learning.md
- [[Parallel Branches (ForkJoin) and Partial Compensation]] - rationale - architecture-patterns/saga-pattern/learning.md
- [[Process Manager (vs Saga, Terminology)]] - concept - architecture-patterns/saga-pattern/learning.md
- [[Process-Then-Ack Discipline]] - rationale - architecture-patterns/idempotency-and-delivery-semantics/reference.md
- [[Richardson, Microservices Patterns ch. 4]] - document - architecture-patterns/saga-pattern/learning.md
- [[Richardson, Transactional Outbox]] - document - architecture-patterns/outbox-pattern/learning.md
- [[Routing Layer and the Topology Map]] - concept - architecture-patterns/sharding/learning.md
- [[Saga (Local Transactions with Compensations)]] - concept - architecture-patterns/saga-pattern/learning.md
- [[Sagas Everywhere as a Service-Boundary Smell]] - rationale - architecture-patterns/saga-pattern/learning.md
- [[Scatter-Gather Query (Latency = max over shards)]] - concept - architecture-patterns/sharding/learning.md
- [[Semantic Lock (ReserveConfirmRelease Idiom)]] - concept - architecture-patterns/saga-pattern/learning.md
- [[Stripe — Designing Robust and Predictable APIs with Idempotency]] - document - architecture-patterns/idempotency-and-delivery-semantics/reference.md
- [[Sync Direction New-Owns-With-Sync-Back Makes Progress Monotonic]] - rationale - architecture-patterns/strangler-fig/learning.md
- [[Temporal Documentation (Durable Execution)]] - document - architecture-patterns/saga-pattern/learning.md
- [[The Dual Write Problem]] - rationale - architecture-patterns/outbox-pattern/learning.md
- [[The Inbox Pattern (Consumer-Side Dedup)]] - concept - architecture-patterns/outbox-pattern/learning.md
- [[The Outbox Table]] - concept - architecture-patterns/outbox-pattern/learning.md
- [[The Pivot and the Three Step Zones]] - rationale - architecture-patterns/saga-pattern/learning.md
- [[The Stuck Saga and Timeout Policy on Every Awaiting State]] - rationale - architecture-patterns/saga-pattern/learning.md
- [[Timeout as Unresolvable Ambiguity]] - rationale - architecture-patterns/idempotency-and-delivery-semantics/reference.md
- [[Vitess (vtgate, Vindexes, Resharding Workflows)]] - document - architecture-patterns/sharding/learning.md
- [[Vogels, Eventually Consistent (CACM 2009)]] - paper - architecture-patterns/replication-and-consistency/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Delivery_Semantics_and_Idempotency
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_B-Trees and Low-Link Origins]]
- 1 edge to [[_COMMUNITY_Outbox Relay and Replication Lag]]
- 1 edge to [[_COMMUNITY_Shard Keys and SCC Algorithms]]

## Top bridge nodes
- [[The Outbox Table]] - degree 9, connects to 1 community
- [[Consistency Model as a Spectrum of Promises]] - degree 5, connects to 1 community
- [[Copy-on-Write B-Trees (Btrfs, LMDB — Root Swap as Atomic Commit)]] - degree 2, connects to 1 community