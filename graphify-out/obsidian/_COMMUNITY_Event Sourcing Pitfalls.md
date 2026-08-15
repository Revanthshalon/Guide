---
type: community
cohesion: 0.20
members: 10
---

# Event Sourcing Pitfalls

**Cohesion:** 0.20 - loosely connected
**Members:** 10 nodes

## Members
- [[Distributed Tracing & Observability (deferred)]] - concept - README.md
- [[Event]] - concept - architecture-patterns/event-sourcing/learning.md
- [[Event Sourcing Production Checklist]] - document - architecture-patterns/event-sourcing/reference.md
- [[Event Store]] - concept - architecture-patterns/event-sourcing/learning.md
- [[Pitfall Event-Sourcing Everything (Complexity Tax)]] - rationale - architecture-patterns/event-sourcing/learning.md
- [[Pitfall Losing Causality]] - concept - architecture-patterns/event-driven-architecture/learning.md
- [[Pitfall Row-Diffs Mistaken for Domain Events]] - rationale - architecture-patterns/change-data-capture/learning.md
- [[Pitfall Treating the Cache as Source of Truth]] - concept - architecture-patterns/caching-strategies/learning.md
- [[Pitfall Trusting Broker Exactly-Once Flags]] - rationale - architecture-patterns/idempotency-and-delivery-semantics/learning.md
- [[Projection  Read Model]] - concept - architecture-patterns/event-sourcing/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Event_Sourcing_Pitfalls
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Events vs Commands (EDA)]]
- 1 edge to [[_COMMUNITY_Consensus, Leases, Fencing]]
- 1 edge to [[_COMMUNITY_Change Data Capture]]
- 1 edge to [[_COMMUNITY_Cache-Aside and Snapshot Bootstrap]]
- 1 edge to [[_COMMUNITY_Breaker Placement and Deferred Topics]]

## Top bridge nodes
- [[Projection  Read Model]] - degree 6, connects to 3 communities
- [[Event]] - degree 4, connects to 1 community
- [[Distributed Tracing & Observability (deferred)]] - degree 2, connects to 1 community