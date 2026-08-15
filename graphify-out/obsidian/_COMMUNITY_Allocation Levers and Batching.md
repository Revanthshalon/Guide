---
type: community
cohesion: 0.20
members: 10
---

# Allocation Levers and Batching

**Cohesion:** 0.20 - loosely connected
**Members:** 10 nodes

## Members
- [[A Batch Is a Shared Fate]] - rationale - performance-optimization/batching-and-amortization/learning.md
- [[Accidental-Allocation Lint List (Hot Loops)]] - concept - performance-optimization/allocation-strategies/reference.md
- [[Buffer Reuse via clear() Keeping Capacity]] - concept - performance-optimization/allocation-strategies/learning.md
- [[Four Levers Allocate Less, Once, Together, Elsewhere]] - rationale - performance-optimization/allocation-strategies/learning.md
- [[Inline Small Storage (SmallVec  ArrayVec  CompactStr)]] - concept - performance-optimization/allocation-strategies/learning.md
- [[N × (F + m) vs. F + N × m]] - rationale - performance-optimization/batching-and-amortization/learning.md
- [[Size-or-Time Flush Shape (N items or T ms)]] - rationale - performance-optimization/batching-and-amortization/learning.md
- [[Swapping the Global Allocator (jemalloc  mimalloc)]] - rationale - performance-optimization/allocation-strategies/learning.md
- [[The Knee N ≈ Fm Sizing Formula]] - rationale - performance-optimization/batching-and-amortization/learning.md
- [[with_capacity — Allocate Once]] - concept - performance-optimization/allocation-strategies/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Allocation_Levers_and_Batching
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Performance Doc Templates]]
- 1 edge to [[_COMMUNITY_Async Runtimes and Allocators]]
- 1 edge to [[_COMMUNITY_Cache Lines and Working Sets]]
- 1 edge to [[_COMMUNITY_Connection Pooling and C10K]]

## Top bridge nodes
- [[Four Levers Allocate Less, Once, Together, Elsewhere]] - degree 7, connects to 2 communities
- [[N × (F + m) vs. F + N × m]] - degree 4, connects to 2 communities