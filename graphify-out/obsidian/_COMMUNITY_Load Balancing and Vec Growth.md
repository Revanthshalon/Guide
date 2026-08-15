---
type: community
cohesion: 0.22
members: 9
---

# Load Balancing and Vec Growth

**Cohesion:** 0.22 - loosely connected
**Members:** 9 nodes

## Members
- [[Envoy Load Balancing Documentation]] - document - architecture-patterns/load-balancing-and-service-discovery/learning.md
- [[Geometric Growth and Amortized Push]] - rationale - data-structures-and-algorithms/arrays-and-dynamic-arrays/learning.md
- [[Graceful Drain (Ordered Shutdown Sequence)]] - rationale - architecture-patterns/load-balancing-and-service-discovery/learning.md
- [[L4 vs L7 Balancing and the gRPCHTTP2 Multiplexing Trap]] - rationale - architecture-patterns/load-balancing-and-service-discovery/learning.md
- [[Load Balancer Diagnostic Signatures Table]] - document - architecture-patterns/load-balancing-and-service-discovery/reference.md
- [[Service Discovery Mechanisms (DNS, Registry, Platform, Mesh)]] - concept - architecture-patterns/load-balancing-and-service-discovery/learning.md
- [[Slow Start for Cold Instances]] - concept - architecture-patterns/load-balancing-and-service-discovery/learning.md
- [[The Reallocation Latency Spike (Amortization Is Accounting, Not Scheduling)]] - rationale - data-structures-and-algorithms/arrays-and-dynamic-arrays/learning.md
- [[std RawVecgrow_amortized (the Real Growth Policy)]] - code - data-structures-and-algorithms/arrays-and-dynamic-arrays/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Load_Balancing_and_Vec_Growth
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Strangler Fig Migration]]
- 1 edge to [[_COMMUNITY_Power of Two Choices]]
- 1 edge to [[_COMMUNITY_The Vec Invariant]]

## Top bridge nodes
- [[Slow Start for Cold Instances]] - degree 4, connects to 2 communities
- [[Geometric Growth and Amortized Push]] - degree 3, connects to 1 community