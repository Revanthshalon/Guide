---
type: community
cohesion: 0.20
members: 10
---

# Strangler Fig Migration

**Cohesion:** 0.20 - loosely connected
**Members:** 10 nodes

## Members
- [[Dual-Write  Backfill  Verify  Cutover Migration Sequence]] - rationale - architecture-patterns/sharding/learning.md
- [[Feathers, Working Effectively with Legacy Code (Seams)]] - document - architecture-patterns/strangler-fig/learning.md
- [[Fowler, StranglerFigApplication]] - document - architecture-patterns/strangler-fig/learning.md
- [[Logical Shards (Over-Provisioning Indirection)]] - rationale - architecture-patterns/sharding/learning.md
- [[Newman, Monolith to Microservices]] - document - architecture-patterns/strangler-fig/learning.md
- [[Reversibility as the Pattern's Actual Product]] - rationale - architecture-patterns/strangler-fig/learning.md
- [[Shadow Traffic and Parallel Run Verification]] - concept - architecture-patterns/strangler-fig/learning.md
- [[Slicing Strategy (Bounded Context, Reads Before Writes)]] - rationale - architecture-patterns/strangler-fig/learning.md
- [[Strangler Fig Pattern]] - concept - architecture-patterns/strangler-fig/learning.md
- [[The Interception Point (Seam  Slice Zero)]] - concept - architecture-patterns/strangler-fig/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Strangler_Fig_Migration
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Load Balancing and Vec Growth]]
- 1 edge to [[_COMMUNITY_Outbox Relay and Replication Lag]]
- 1 edge to [[_COMMUNITY_Consistent Hashing and Shard Skew]]

## Top bridge nodes
- [[Strangler Fig Pattern]] - degree 6, connects to 2 communities
- [[Logical Shards (Over-Provisioning Indirection)]] - degree 2, connects to 1 community