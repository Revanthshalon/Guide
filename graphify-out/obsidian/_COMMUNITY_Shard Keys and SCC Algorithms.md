---
type: community
cohesion: 0.25
members: 8
---

# Shard Keys and SCC Algorithms

**Cohesion:** 0.25 - loosely connected
**Members:** 8 nodes

## Members
- [[Access-Pattern Table (Choose the Key from Measured Queries)]] - rationale - architecture-patterns/sharding/learning.md
- [[Kosaraju SCC]] - concept - data-structures-and-algorithms/advanced-graph-algorithms/learning.md
- [[Premature Sharding (Exhaust the Cheaper Ladder First)]] - rationale - architecture-patterns/sharding/learning.md
- [[SCC Condensation Yields a DAG]] - rationale - data-structures-and-algorithms/advanced-graph-algorithms/learning.md
- [[Tarjan SCC]] - concept - data-structures-and-algorithms/advanced-graph-algorithms/learning.md
- [[The Shard Key (the Dominating, Irreversible Decision)]] - rationale - architecture-patterns/sharding/learning.md
- [[Write DFS Iteratively (Recursion Aborts at ~200k Depth)]] - rationale - data-structures-and-algorithms/advanced-graph-algorithms/learning.md
- [[petgraph crate]] - code - data-structures-and-algorithms/advanced-graph-algorithms/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Shard_Keys_and_SCC_Algorithms
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_B-Trees and Low-Link Origins]]
- 1 edge to [[_COMMUNITY_Delivery Semantics and Idempotency]]
- 1 edge to [[_COMMUNITY_Flow, Matching, and Reductions]]
- 1 edge to [[_COMMUNITY_Consistent Hashing and Shard Skew]]
- 1 edge to [[_COMMUNITY_Outbox Relay and Replication Lag]]
- 1 edge to [[_COMMUNITY_Power of Two Choices]]
- 1 edge to [[_COMMUNITY_The Vec Invariant]]

## Top bridge nodes
- [[The Shard Key (the Dominating, Irreversible Decision)]] - degree 7, connects to 4 communities
- [[Tarjan SCC]] - degree 6, connects to 2 communities
- [[Write DFS Iteratively (Recursion Aborts at ~200k Depth)]] - degree 2, connects to 1 community