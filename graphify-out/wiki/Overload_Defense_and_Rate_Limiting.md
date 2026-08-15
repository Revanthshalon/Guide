# Overload Defense and Rate Limiting

> 18 nodes · cohesion 0.12

## Key Concepts

- **Breaker State Machine and Thresholds** (6 connections) — `architecture-patterns/circuit-breaker/learning.md`
- **Concurrency Limits** (5 connections) — `architecture-patterns/backpressure-and-rate-limiting/learning.md`
- **Retry Amplification and Metastable Collapse** (4 connections) — `architecture-patterns/backpressure-and-rate-limiting/learning.md`
- **The Two-Generals Ambiguity** (4 connections) — `architecture-patterns/idempotency-and-delivery-semantics/learning.md`
- **Bulkheads (Isolation)** (3 connections) — `architecture-patterns/circuit-breaker/learning.md`
- **Cache Avalanche (Mass Expiry or Cache Loss)** (2 connections) — `architecture-patterns/caching-strategies/learning.md`
- **Cache Stampede (Thundering Herd)** (2 connections) — `architecture-patterns/caching-strategies/learning.md`
- **What Counts as a Failure** (2 connections) — `architecture-patterns/circuit-breaker/learning.md`
- **Pitfall: Half-Open Thundering Herd** (2 connections) — `architecture-patterns/circuit-breaker/learning.md`
- **Pitfall: KMS as Availability/Latency Chokepoint** (2 connections) — `architecture-patterns/encryption-and-key-management/learning.md`
- **Adaptive Limits** (1 connections) — `architecture-patterns/backpressure-and-rate-limiting/learning.md`
- **Rate Limiting Algorithms** (1 connections) — `architecture-patterns/backpressure-and-rate-limiting/learning.md`
- **Metrics That Predict Incidents** (1 connections) — `architecture-patterns/caching-strategies/reference.md`
- **Pitfall: Retries and Breakers Fighting Each Other** (1 connections) — `architecture-patterns/circuit-breaker/learning.md`
- **Resilience Family Deployment Priority Order** (1 connections) — `architecture-patterns/circuit-breaker/reference.md`
- **Starting Breaker Configuration** (1 connections) — `architecture-patterns/circuit-breaker/reference.md`
- **Pitfall: Hand-Rolling Leader Election** (1 connections) — `architecture-patterns/consensus-and-leader-election/learning.md`
- **Pitfall: Event Storms and Feedback Loops** (1 connections) — `architecture-patterns/event-driven-architecture/learning.md`

## Relationships

- [Backpressure and Queueing](Backpressure_and_Queueing.md) (1 shared connections)
- [Breaker Placement and Deferred Topics](Breaker_Placement_and_Deferred_Topics.md) (1 shared connections)
- [Cache-Aside and Snapshot Bootstrap](Cache-Aside_and_Snapshot_Bootstrap.md) (1 shared connections)
- [Change Data Capture](Change_Data_Capture.md) (1 shared connections)

## Source Files

- `architecture-patterns/backpressure-and-rate-limiting/learning.md`
- `architecture-patterns/caching-strategies/learning.md`
- `architecture-patterns/caching-strategies/reference.md`
- `architecture-patterns/circuit-breaker/learning.md`
- `architecture-patterns/circuit-breaker/reference.md`
- `architecture-patterns/consensus-and-leader-election/learning.md`
- `architecture-patterns/encryption-and-key-management/learning.md`
- `architecture-patterns/event-driven-architecture/learning.md`
- `architecture-patterns/idempotency-and-delivery-semantics/learning.md`

## Audit Trail

- EXTRACTED: 13 (59%)
- INFERRED: 9 (41%)
- AMBIGUOUS: 0 (0%)

---

*Part of the graphify knowledge wiki. See [index](index.md) to navigate.*