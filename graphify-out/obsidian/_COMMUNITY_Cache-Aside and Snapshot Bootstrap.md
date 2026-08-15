---
type: community
cohesion: 0.40
members: 5
---

# Cache-Aside and Snapshot Bootstrap

**Cohesion:** 0.40 - moderately connected
**Members:** 5 nodes

## Members
- [[Cache-Aside (Lazy Loading)]] - concept - architecture-patterns/caching-strategies/learning.md
- [[Hit-Rate Math]] - concept - architecture-patterns/caching-strategies/reference.md
- [[Read-Through  Write-Through  Write-Behind]] - concept - architecture-patterns/caching-strategies/learning.md
- [[Snapshot]] - concept - architecture-patterns/event-sourcing/learning.md
- [[Snapshot + Streaming (Bootstrap Problem)]] - concept - architecture-patterns/change-data-capture/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Cache-Aside_and_Snapshot_Bootstrap
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Overload Defense and Rate Limiting]]
- 1 edge to [[_COMMUNITY_Quorums and Broker Semantics]]
- 1 edge to [[_COMMUNITY_Event Sourcing Pitfalls]]
- 1 edge to [[_COMMUNITY_Change Data Capture]]
- 1 edge to [[_COMMUNITY_Events vs Commands (EDA)]]

## Top bridge nodes
- [[Cache-Aside (Lazy Loading)]] - degree 5, connects to 2 communities
- [[Snapshot]] - degree 3, connects to 1 community
- [[Read-Through  Write-Through  Write-Behind]] - degree 2, connects to 1 community
- [[Snapshot + Streaming (Bootstrap Problem)]] - degree 2, connects to 1 community