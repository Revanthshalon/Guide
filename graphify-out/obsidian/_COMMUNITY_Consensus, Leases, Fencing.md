---
type: community
cohesion: 0.33
members: 6
---

# Consensus, Leases, Fencing

**Cohesion:** 0.33 - loosely connected
**Members:** 6 nodes

## Members
- [[Fencing Tokens]] - concept - architecture-patterns/consensus-and-leader-election/learning.md
- [[Formal Methods (TLA+) (deferred)]] - concept - README.md
- [[Leases]] - concept - architecture-patterns/consensus-and-leader-election/learning.md
- [[Linearizability from Consensus]] - concept - architecture-patterns/consensus-and-leader-election/learning.md
- [[Pitfall The Zombie Projector]] - concept - architecture-patterns/event-sourcing/learning.md
- [[Raft]] - concept - architecture-patterns/consensus-and-leader-election/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Consensus_Leases_Fencing
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Quorums and Broker Semantics]]
- 1 edge to [[_COMMUNITY_Event Sourcing Pitfalls]]
- 1 edge to [[_COMMUNITY_Cache Invalidation and Idempotency Keys]]
- 1 edge to [[_COMMUNITY_Breaker Placement and Deferred Topics]]

## Top bridge nodes
- [[Raft]] - degree 4, connects to 1 community
- [[Leases]] - degree 3, connects to 1 community
- [[Pitfall The Zombie Projector]] - degree 2, connects to 1 community
- [[Formal Methods (TLA+) (deferred)]] - degree 2, connects to 1 community