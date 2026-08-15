---
type: community
cohesion: 0.25
members: 9
---

# Consistent Hashing and Shard Skew

**Cohesion:** 0.25 - loosely connected
**Members:** 9 nodes

## Members
- [[Bounded-Load Consistent Hashing for Affinity]] - concept - architecture-patterns/load-balancing-and-service-discovery/learning.md
- [[Consistent Hashing and Virtual Nodes]] - concept - architecture-patterns/sharding/learning.md
- [[Cross-Category Interlocks Table]] - document - data-structures-and-algorithms/LEARNING-INDEX.md
- [[DeCandia et al., Dynamo Amazon's Highly Available Key-value Store]] - paper - architecture-patterns/sharding/learning.md
- [[Graefe, Modern B-Tree Techniques (2011)]] - paper - data-structures-and-algorithms/b-trees/learning.md
- [[Karger et al., Consistent Hashing and Random Trees (STOC 1997)]] - paper - architecture-patterns/sharding/learning.md
- [[Node Occupancy on Disk (50% Trail, Bulk Loading, Fill Factor)]] - rationale - data-structures-and-algorithms/b-trees/learning.md
- [[Partitioning Strategies (Range, Hash, Directory)]] - concept - architecture-patterns/sharding/learning.md
- [[The Hot Shard (Skew, Celebrities, Monotonic Keys)]] - rationale - architecture-patterns/sharding/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Consistent_Hashing_and_Shard_Skew
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_B-Trees and Low-Link Origins]]
- 1 edge to [[_COMMUNITY_Strangler Fig Migration]]
- 1 edge to [[_COMMUNITY_Shard Keys and SCC Algorithms]]

## Top bridge nodes
- [[Consistent Hashing and Virtual Nodes]] - degree 6, connects to 1 community
- [[Partitioning Strategies (Range, Hash, Directory)]] - degree 3, connects to 1 community
- [[Node Occupancy on Disk (50% Trail, Bulk Loading, Fill Factor)]] - degree 3, connects to 1 community
- [[Cross-Category Interlocks Table]] - degree 2, connects to 1 community