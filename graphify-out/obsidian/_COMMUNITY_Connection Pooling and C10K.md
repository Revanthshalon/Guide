---
type: community
cohesion: 0.33
members: 6
---

# Connection Pooling and C10K

**Cohesion:** 0.33 - loosely connected
**Members:** 6 nodes

## Members
- [[Connection Pooling with PgBouncer (Not Optional)]] - rationale - oss-tools/postgres/runbook.md
- [[Hoisting — Pay the Fixed Cost Once]] - rationale - performance-optimization/batching-and-amortization/learning.md
- [[Pitfall Connection Exhaustion]] - rationale - oss-tools/postgres/learning.md
- [[Process-Per-Connection Model]] - concept - oss-tools/postgres/learning.md
- [[The C10K Problem (Thread-Per-Connection Rent)]] - concept - performance-optimization/async-and-io/learning.md
- [[When Threads Win Anyway]] - rationale - performance-optimization/async-and-io/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Connection_Pooling_and_C10K
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Async Runtimes and Allocators]]
- 1 edge to [[_COMMUNITY_PostgreSQL Storage and Planner]]
- 1 edge to [[_COMMUNITY_Performance Doc Templates]]
- 1 edge to [[_COMMUNITY_Allocation Levers and Batching]]

## Top bridge nodes
- [[The C10K Problem (Thread-Per-Connection Rent)]] - degree 4, connects to 2 communities
- [[Process-Per-Connection Model]] - degree 3, connects to 2 communities
- [[Hoisting — Pay the Fixed Cost Once]] - degree 2, connects to 1 community