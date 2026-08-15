---
type: community
cohesion: 1.00
members: 2
---

# Split Brain and RPO

**Cohesion:** 1.00 - tightly connected
**Members:** 2 nodes

## Members
- [[Split Brain and Fencing the Old Leader]] - rationale - architecture-patterns/replication-and-consistency/learning.md
- [[Synchronous vs Asynchronous Replication (RPO)]] - rationale - architecture-patterns/replication-and-consistency/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Split_Brain_and_RPO
SORT file.name ASC
```
