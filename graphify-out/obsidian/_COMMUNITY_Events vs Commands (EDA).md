---
type: community
cohesion: 0.22
members: 10
---

# Events vs Commands (EDA)

**Cohesion:** 0.22 - loosely connected
**Members:** 10 nodes

## Members
- [[Aggregate (Consistency Boundary)]] - concept - architecture-patterns/event-sourcing/learning.md
- [[Architecture Patterns Study Order]] - document - architecture-patterns/LEARNING-INDEX.md
- [[Command]] - concept - architecture-patterns/event-sourcing/learning.md
- [[Events vs Commands]] - rationale - architecture-patterns/event-driven-architecture/learning.md
- [[Internal vs Integration Events]] - rationale - architecture-patterns/event-sourcing/learning.md
- [[Pitfall The Distributed Monolith (Events as RPC)]] - rationale - architecture-patterns/event-driven-architecture/learning.md
- [[Saga Pattern]] - concept - architecture-patterns/LEARNING-INDEX.md
- [[Sharding]] - concept - architecture-patterns/LEARNING-INDEX.md
- [[Shorter Reading Paths]] - rationale - architecture-patterns/LEARNING-INDEX.md
- [[The Three Patterns (Fowler)]] - concept - architecture-patterns/event-driven-architecture/reference.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Events_vs_Commands_EDA
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Change Data Capture]]
- 1 edge to [[_COMMUNITY_Backpressure and Queueing]]
- 1 edge to [[_COMMUNITY_Encryption and Key Hierarchy]]
- 1 edge to [[_COMMUNITY_Quorums and Broker Semantics]]
- 1 edge to [[_COMMUNITY_Event Sourcing Pitfalls]]
- 1 edge to [[_COMMUNITY_Cache-Aside and Snapshot Bootstrap]]
- 1 edge to [[_COMMUNITY_Breaker Placement and Deferred Topics]]

## Top bridge nodes
- [[Architecture Patterns Study Order]] - degree 9, connects to 5 communities
- [[Aggregate (Consistency Boundary)]] - degree 5, connects to 2 communities