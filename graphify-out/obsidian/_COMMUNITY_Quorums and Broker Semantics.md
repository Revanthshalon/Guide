---
type: community
cohesion: 0.18
members: 11
---

# Quorums and Broker Semantics

**Cohesion:** 0.18 - loosely connected
**Members:** 11 nodes

## Members
- [[A Cache Is a Priced Bet on Staleness]] - rationale - architecture-patterns/caching-strategies/learning.md
- [[Consumer Groups, Offsets, and Lag]] - concept - architecture-patterns/event-driven-architecture/learning.md
- [[Dead Letter Queues and Poison Messages]] - concept - architecture-patterns/event-driven-architecture/learning.md
- [[EDA Diagnostic Signatures]] - document - architecture-patterns/event-driven-architecture/reference.md
- [[Log-Based vs Queue-Based Brokers]] - concept - architecture-patterns/event-driven-architecture/learning.md
- [[Membership and Reconfiguration]] - concept - architecture-patterns/consensus-and-leader-election/learning.md
- [[Partitions, Ordering, and the Key Choice]] - concept - architecture-patterns/event-driven-architecture/learning.md
- [[Pitfall Losing Instead of Duplicating]] - concept - architecture-patterns/idempotency-and-delivery-semantics/learning.md
- [[Quorum Intersection]] - concept - architecture-patterns/consensus-and-leader-election/learning.md
- [[Quorum Sizing & Placement]] - document - architecture-patterns/consensus-and-leader-election/reference.md
- [[Replication & Consistency Models]] - concept - architecture-patterns/LEARNING-INDEX.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Quorums_and_Broker_Semantics
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Consensus, Leases, Fencing]]
- 1 edge to [[_COMMUNITY_Change Data Capture]]
- 1 edge to [[_COMMUNITY_Events vs Commands (EDA)]]
- 1 edge to [[_COMMUNITY_Cache-Aside and Snapshot Bootstrap]]

## Top bridge nodes
- [[Replication & Consistency Models]] - degree 5, connects to 2 communities
- [[Quorum Intersection]] - degree 4, connects to 1 community
- [[A Cache Is a Priced Bet on Staleness]] - degree 2, connects to 1 community