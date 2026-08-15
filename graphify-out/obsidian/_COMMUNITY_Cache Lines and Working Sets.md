---
type: community
cohesion: 0.33
members: 6
---

# Cache Lines and Working Sets

**Cohesion:** 0.33 - loosely connected
**Members:** 6 nodes

## Members
- [[Bandwidth Wall and Wasted Line Fraction]] - rationale - performance-optimization/cache-locality/learning.md
- [[Bump Arena (bumpalo) — Lifetime-Grouped Allocation]] - rationale - performance-optimization/allocation-strategies/learning.md
- [[Pitfall Default Configuration in Production]] - rationale - oss-tools/postgres/learning.md
- [[Settings That Matter (Defaults Sized for a Laptop)]] - rationale - oss-tools/postgres/reference.md
- [[The Cache Line as the Unit of Transfer]] - rationale - performance-optimization/cache-locality/learning.md
- [[The Working-Set Cliff]] - rationale - performance-optimization/cache-locality/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Cache_Lines_and_Working_Sets
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Branch Prediction and Branchless Code]]
- 1 edge to [[_COMMUNITY_Performance Doc Templates]]
- 1 edge to [[_COMMUNITY_Allocation Levers and Batching]]

## Top bridge nodes
- [[The Cache Line as the Unit of Transfer]] - degree 5, connects to 2 communities
- [[Bump Arena (bumpalo) — Lifetime-Grouped Allocation]] - degree 2, connects to 1 community