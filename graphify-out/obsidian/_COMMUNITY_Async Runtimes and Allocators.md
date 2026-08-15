---
type: community
cohesion: 0.18
members: 11
---

# Async Runtimes and Allocators

**Cohesion:** 0.18 - loosely connected
**Members:** 11 nodes

## Members
- [[Allocator Size Classes and Per-Thread Caches]] - concept - performance-optimization/allocation-strategies/learning.md
- [[Blocking the Runtime — The Cardinal Sin]] - rationale - performance-optimization/async-and-io/learning.md
- [[Cancellation Safety (Drop at Any Await)]] - rationale - performance-optimization/async-and-io/learning.md
- [[Completion Model — io_uring]] - concept - performance-optimization/async-and-io/learning.md
- [[Executor, Reactor, and Waker]] - concept - performance-optimization/async-and-io/learning.md
- [[Poll-Time Histogram (tokio-console) as the Flamegraph of Async]] - concept - performance-optimization/async-and-io/learning.md
- [[Readiness Models — epoll  kqueue]] - concept - performance-optimization/async-and-io/learning.md
- [[The Cardinal Hazards of Async]] - concept - performance-optimization/async-and-io/reference.md
- [[The Fixed-Cost Ladder]] - rationale - performance-optimization/batching-and-amortization/learning.md
- [[async fn as a State-Machine Enum]] - rationale - performance-optimization/async-and-io/learning.md
- [[spawn_blocking and the Rayon Bridge]] - concept - performance-optimization/async-and-io/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Async_Runtimes_and_Allocators
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Performance Doc Templates]]
- 2 edges to [[_COMMUNITY_Connection Pooling and C10K]]
- 1 edge to [[_COMMUNITY_PostgreSQL Storage and Planner]]
- 1 edge to [[_COMMUNITY_Allocation Levers and Batching]]

## Top bridge nodes
- [[The Fixed-Cost Ladder]] - degree 4, connects to 2 communities
- [[Readiness Models — epoll  kqueue]] - degree 4, connects to 1 community
- [[async fn as a State-Machine Enum]] - degree 3, connects to 1 community
- [[Allocator Size Classes and Per-Thread Caches]] - degree 2, connects to 1 community
- [[Completion Model — io_uring]] - degree 2, connects to 1 community