---
type: community
cohesion: 0.12
members: 18
---

# Overload Defense and Rate Limiting

**Cohesion:** 0.12 - loosely connected
**Members:** 18 nodes

## Members
- [[Adaptive Limits]] - concept - architecture-patterns/backpressure-and-rate-limiting/learning.md
- [[Breaker State Machine and Thresholds]] - concept - architecture-patterns/circuit-breaker/learning.md
- [[Bulkheads (Isolation)]] - concept - architecture-patterns/circuit-breaker/learning.md
- [[Cache Avalanche (Mass Expiry or Cache Loss)]] - concept - architecture-patterns/caching-strategies/learning.md
- [[Cache Stampede (Thundering Herd)]] - concept - architecture-patterns/caching-strategies/learning.md
- [[Concurrency Limits]] - concept - architecture-patterns/backpressure-and-rate-limiting/learning.md
- [[Metrics That Predict Incidents]] - concept - architecture-patterns/caching-strategies/reference.md
- [[Pitfall Event Storms and Feedback Loops]] - concept - architecture-patterns/event-driven-architecture/learning.md
- [[Pitfall Half-Open Thundering Herd]] - concept - architecture-patterns/circuit-breaker/learning.md
- [[Pitfall Hand-Rolling Leader Election]] - rationale - architecture-patterns/consensus-and-leader-election/learning.md
- [[Pitfall KMS as AvailabilityLatency Chokepoint]] - concept - architecture-patterns/encryption-and-key-management/learning.md
- [[Pitfall Retries and Breakers Fighting Each Other]] - concept - architecture-patterns/circuit-breaker/learning.md
- [[Rate Limiting Algorithms]] - concept - architecture-patterns/backpressure-and-rate-limiting/learning.md
- [[Resilience Family Deployment Priority Order]] - rationale - architecture-patterns/circuit-breaker/reference.md
- [[Retry Amplification and Metastable Collapse]] - concept - architecture-patterns/backpressure-and-rate-limiting/learning.md
- [[Starting Breaker Configuration]] - document - architecture-patterns/circuit-breaker/reference.md
- [[The Two-Generals Ambiguity]] - rationale - architecture-patterns/idempotency-and-delivery-semantics/learning.md
- [[What Counts as a Failure]] - rationale - architecture-patterns/circuit-breaker/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Overload_Defense_and_Rate_Limiting
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Backpressure and Queueing]]
- 1 edge to [[_COMMUNITY_Breaker Placement and Deferred Topics]]
- 1 edge to [[_COMMUNITY_Cache-Aside and Snapshot Bootstrap]]
- 1 edge to [[_COMMUNITY_Change Data Capture]]

## Top bridge nodes
- [[Breaker State Machine and Thresholds]] - degree 6, connects to 1 community
- [[Concurrency Limits]] - degree 5, connects to 1 community
- [[The Two-Generals Ambiguity]] - degree 4, connects to 1 community
- [[Pitfall KMS as AvailabilityLatency Chokepoint]] - degree 2, connects to 1 community