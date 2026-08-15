---
type: community
cohesion: 0.22
members: 10
---

# Change Data Capture

**Cohesion:** 0.22 - loosely connected
**Members:** 10 nodes

## Members
- [[At-Most-Once  At-Least-Once  Effectively-Once]] - concept - architecture-patterns/idempotency-and-delivery-semantics/learning.md
- [[CDC Production Checklist]] - document - architecture-patterns/change-data-capture/reference.md
- [[Log-Based Capture]] - concept - architecture-patterns/change-data-capture/learning.md
- [[Outbox Pattern]] - concept - architecture-patterns/LEARNING-INDEX.md
- [[Pitfall Dual Write (Append and Publish)]] - concept - architecture-patterns/event-sourcing/learning.md
- [[Pitfall Key Recorded Separately From the Effect]] - concept - architecture-patterns/idempotency-and-delivery-semantics/learning.md
- [[Replication Slots and Retention]] - concept - architecture-patterns/change-data-capture/learning.md
- [[Strangler Fig]] - concept - architecture-patterns/LEARNING-INDEX.md
- [[The Change Event]] - concept - architecture-patterns/change-data-capture/learning.md
- [[Trigger- and Query-Based Capture]] - concept - architecture-patterns/change-data-capture/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Change_Data_Capture
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Cache Invalidation and Idempotency Keys]]
- 2 edges to [[_COMMUNITY_Events vs Commands (EDA)]]
- 1 edge to [[_COMMUNITY_Overload Defense and Rate Limiting]]
- 1 edge to [[_COMMUNITY_Quorums and Broker Semantics]]
- 1 edge to [[_COMMUNITY_Event Sourcing Pitfalls]]
- 1 edge to [[_COMMUNITY_Cache-Aside and Snapshot Bootstrap]]

## Top bridge nodes
- [[At-Most-Once  At-Least-Once  Effectively-Once]] - degree 6, connects to 4 communities
- [[Log-Based Capture]] - degree 7, connects to 2 communities
- [[Outbox Pattern]] - degree 4, connects to 1 community
- [[Strangler Fig]] - degree 2, connects to 1 community