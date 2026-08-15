---
type: community
cohesion: 0.22
members: 9
---

# Outbox Relay and Replication Lag

**Cohesion:** 0.22 - loosely connected
**Members:** 9 nodes

## Members
- [[Decision Reads vs Display Reads (Per-Query Routing)]] - rationale - architecture-patterns/replication-and-consistency/learning.md
- [[Derived Copies Lag the Source (Unifying Frame)]] - rationale - architecture-patterns/replication-and-consistency/learning.md
- [[Log-Tailing Relay (Outbox via CDC  Debezium)]] - concept - architecture-patterns/outbox-pattern/learning.md
- [[MorlingDebezium, Reliable Microservices Data Exchange With the Outbox Pattern]] - document - architecture-patterns/outbox-pattern/learning.md
- [[Per-Aggregate Ordering and Partition Key]] - rationale - architecture-patterns/outbox-pattern/learning.md
- [[Polling Relay (Publisher)]] - concept - architecture-patterns/outbox-pattern/learning.md
- [[Replication Lag Anomalies (Read-Your-Writes, Monotonic, Causal)]] - concept - architecture-patterns/replication-and-consistency/learning.md
- [[Single-Leader Replication]] - concept - architecture-patterns/replication-and-consistency/learning.md
- [[The `id  last_seen` Poll Skips Rows Forever]] - rationale - architecture-patterns/outbox-pattern/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Outbox_Relay_and_Replication_Lag
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Delivery Semantics and Idempotency]]
- 1 edge to [[_COMMUNITY_Strangler Fig Migration]]
- 1 edge to [[_COMMUNITY_Shard Keys and SCC Algorithms]]

## Top bridge nodes
- [[Polling Relay (Publisher)]] - degree 4, connects to 1 community
- [[Derived Copies Lag the Source (Unifying Frame)]] - degree 3, connects to 1 community
- [[Single-Leader Replication]] - degree 3, connects to 1 community