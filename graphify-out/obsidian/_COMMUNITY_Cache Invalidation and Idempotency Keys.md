---
type: community
cohesion: 0.33
members: 7
---

# Cache Invalidation and Idempotency Keys

**Cohesion:** 0.33 - loosely connected
**Members:** 7 nodes

## Members
- [[CDC-Driven Invalidation]] - concept - architecture-patterns/caching-strategies/learning.md
- [[Deduplication Window and Consumer Contract]] - concept - architecture-patterns/idempotency-and-delivery-semantics/learning.md
- [[Idempotency Key]] - concept - architecture-patterns/idempotency-and-delivery-semantics/learning.md
- [[Immutable  Versioned Keys]] - concept - architecture-patterns/caching-strategies/learning.md
- [[Invalidation Strategies]] - concept - architecture-patterns/caching-strategies/learning.md
- [[Natural Idempotency]] - concept - architecture-patterns/idempotency-and-delivery-semantics/learning.md
- [[TTL From a Stated Staleness Budget]] - rationale - architecture-patterns/caching-strategies/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Cache_Invalidation_and_Idempotency_Keys
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Change Data Capture]]
- 1 edge to [[_COMMUNITY_Consensus, Leases, Fencing]]

## Top bridge nodes
- [[TTL From a Stated Staleness Budget]] - degree 3, connects to 1 community
- [[Idempotency Key]] - degree 3, connects to 1 community
- [[CDC-Driven Invalidation]] - degree 2, connects to 1 community