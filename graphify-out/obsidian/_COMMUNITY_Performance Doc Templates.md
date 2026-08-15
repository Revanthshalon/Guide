---
type: community
cohesion: 0.23
members: 13
---

# Performance Doc Templates

**Cohesion:** 0.23 - loosely connected
**Members:** 13 nodes

## Members
- [[Allocation Strategies — Learning Notes]] - document - performance-optimization/allocation-strategies/learning.md
- [[Allocation Strategies — Quick Reference]] - document - performance-optimization/allocation-strategies/reference.md
- [[Async & IO — Learning Notes]] - document - performance-optimization/async-and-io/learning.md
- [[Async & IO — Quick Reference]] - document - performance-optimization/async-and-io/reference.md
- [[Batching & Amortization — Learning Notes]] - document - performance-optimization/batching-and-amortization/learning.md
- [[Batching & Amortization — Quick Reference]] - document - performance-optimization/batching-and-amortization/reference.md
- [[Cache Locality — Learning Notes]] - document - performance-optimization/cache-locality/learning.md
- [[Cache Locality — Quick Reference]] - document - performance-optimization/cache-locality/reference.md
- [[Never Apply Anything Without a Profile Pointing at It]] - rationale - performance-optimization/LEARNING-INDEX.md
- [[Performance Optimization — Learning Index]] - document - performance-optimization/LEARNING-INDEX.md
- [[Performance Technique — Learning Template]] - document - performance-optimization/_template-learning.md
- [[Performance Technique — Reference Template]] - document - performance-optimization/_template-reference.md
- [[Set-Associative Eviction and Power-of-Two Striding]] - concept - performance-optimization/cache-locality/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Performance_Doc_Templates
SORT file.name ASC
```

## Connections to other communities
- 4 edges to [[_COMMUNITY_Branch Prediction and Branchless Code]]
- 2 edges to [[_COMMUNITY_Async Runtimes and Allocators]]
- 1 edge to [[_COMMUNITY_TLB, Cache, Branch Signatures]]
- 1 edge to [[_COMMUNITY_Allocation Levers and Batching]]
- 1 edge to [[_COMMUNITY_Connection Pooling and C10K]]
- 1 edge to [[_COMMUNITY_Cache Lines and Working Sets]]

## Top bridge nodes
- [[Cache Locality — Learning Notes]] - degree 9, connects to 3 communities
- [[Allocation Strategies — Learning Notes]] - degree 7, connects to 2 communities
- [[Performance Optimization — Learning Index]] - degree 6, connects to 1 community
- [[Batching & Amortization — Learning Notes]] - degree 5, connects to 1 community
- [[Performance Technique — Learning Template]] - degree 4, connects to 1 community