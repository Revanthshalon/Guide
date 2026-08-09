# Architecture Patterns — Learning Index

The order to read this category in. It's derived from what the docs actually depend on: each entry's prerequisites are all *above* it, so no doc sends you forward for vocabulary you don't have yet.

Read `learning.md` top-to-bottom for each; the matching `reference.md` is for later, when you're implementing.

## The order

| # | Topic | Depends on | Why here |
| --- | --- | --- | --- |
| 1 | [Replication & Consistency Models](replication-and-consistency/learning.md) | — | The vocabulary every other doc borrows: lag, staleness, linearizable vs. eventual, read-your-writes. Nothing else parses correctly without it. |
| 2 | [Idempotency & Delivery Semantics](idempotency-and-delivery-semantics/learning.md) | 1 | Why the network forces retries at all, and what at-least-once obligates. Half the later patterns exist to satisfy this one. |
| 3 | [Consensus & Leader Election](consensus-and-leader-election/learning.md) | 1, 2 | How linearizability and safe failover are actually implemented. Starts from the crashed-vs-slow ambiguity of #2, and fencing shows up everywhere afterwards. |
| 4 | [Event-Driven Architecture](event-driven-architecture/learning.md) | 2 | The framing for the whole integration block: topics, partitions, choreography, consumer lag. Deliberately omits *reliable publishing* — that's the next doc. |
| 5 | [Outbox Pattern](outbox-pattern/learning.md) | 2, 4 | Fills #4's hole: the dual write, and why intent must be derived from one committed write. The producer half of at-least-once. |
| 6 | [Change Data Capture](change-data-capture/learning.md) | 1, 5 | Generalizes the outbox's log-tailing relay, and forces the row-diffs-vs-authored-intent decision against a pattern you already know. |
| 7 | [Event Sourcing & CQRS](event-sourcing/learning.md) | 1, 2, 5, 6 | The log as primary store. Projection lag *is* replication lag; the dual-write of #5 is why the store must be the only write. Heaviest doc in the category — don't start here. |
| 8 | [Saga Pattern](saga-pattern/learning.md) | 2, 5, 7 | Multi-service workflows. Rides outbox-grade messaging, demands idempotent participants, and takes the aggregate boundary from #7 as its local-transaction unit. |
| 9 | [Sharding](sharding/learning.md) | 1, 3, 6, 7, 8 | The orthogonal axis to replication — splitting instead of copying. Cross-shard transactions become sagas; the aggregate boundary is the natural shard key. |
| 10 | [Circuit Breaker](circuit-breaker/learning.md) | 2 | Start of the resilience block: stop calling what's already failing. |
| 11 | [Backpressure & Rate Limiting](backpressure-and-rate-limiting/learning.md) | 2, 10 | The companion to #10 — Little's Law, retry budgets, load shedding. Read them as a pair. |
| 12 | [Load Balancing & Service Discovery](load-balancing-and-service-discovery/learning.md) | 3, 9, 10, 11 | Instance-granularity versions of the two above (outlier ejection, ejection cascades), plus registries inheriting the failure-detector limits from #3. |
| 13 | [Caching Strategies](caching-strategies/learning.md) | 1, 6, 9, 10, 11 | Caches are the loosest replicas — all lag anomalies from #1 apply, CDC is the drift-free invalidation feed, and #10–11 are what protects the origin on the miss path. |
| 14 | [Encryption & Key Management](encryption-and-key-management/learning.md) | 2, 7 | Cross-cutting; slots in anywhere after #7 (crypto-shredding is event sourcing's GDPR answer). Pair with [OpenBao](../oss-tools/openbao/learning.md), which plays the KMS role. |
| 15 | [Strangler Fig](strangler-fig/learning.md) | 1, 4, 5, 6, 9, 12 | Last on purpose: it composes almost everything above into a migration — CDC/outbox for sync, LB for traffic shifting, sharding's dual-write/backfill/verify/cutover sequence. |

## Shorter paths

- **Just need reliable messaging between services:** 1 → 2 → 4 → 5, then 8 if there are multi-step workflows.
- **Just need the service to stay up under load:** 2 → 10 → 11 → 12 → 13.
- **Breaking up a monolith:** 1 → 2 → 5 → 6 → 15 (then 9 if data has to split too).

## Pairs that should be read together

- [Outbox](outbox-pattern/learning.md) + [Idempotency & Delivery Semantics](idempotency-and-delivery-semantics/learning.md) — producer and consumer halves of the same guarantee; neither works alone.
- [Circuit Breaker](circuit-breaker/learning.md) + [Backpressure & Rate Limiting](backpressure-and-rate-limiting/learning.md) — stop calling vs. stop accepting.
- [Replication & Consistency](replication-and-consistency/learning.md) + [Sharding](sharding/learning.md) — copies vs. splits, the two orthogonal axes.
- [Outbox](outbox-pattern/learning.md) + [Change Data Capture](change-data-capture/learning.md) — authored intent vs. row diffs, the sibling decision.

## Where this category meets performance

Several architecture patterns are the same idea one scale down; read the pair once you have both sides.

| Architecture | Performance counterpart | The shared idea |
| --- | --- | --- |
| [Sharding](sharding/learning.md) | [NUMA Awareness](../performance-optimization/numa-awareness/learning.md), [False Sharing](../performance-optimization/false-sharing/learning.md) | Partition so owners don't contend |
| [Caching Strategies](caching-strategies/learning.md) | [Cache Locality](../performance-optimization/cache-locality/learning.md) | Same policies, silicon instead of config |
| [Backpressure & Rate Limiting](backpressure-and-rate-limiting/learning.md) | [Async & I/O](../performance-optimization/async-and-io/learning.md), [Batching & Amortization](../performance-optimization/batching-and-amortization/learning.md) | Bounded queues, Little's Law |
| [Event Sourcing & CQRS](event-sourcing/learning.md) | [Lock-Free Concurrency](../performance-optimization/lock-free-concurrency/learning.md) | `expected_version` and CAS are one optimistic-concurrency idea |
| [Event-Driven Architecture](event-driven-architecture/learning.md) | [Serialization & Encoding](../performance-optimization/serialization-and-encoding/learning.md) | Schema evolution as the real contract |
