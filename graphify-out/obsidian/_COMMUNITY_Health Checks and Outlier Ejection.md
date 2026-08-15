---
type: community
cohesion: 0.50
members: 4
---

# Health Checks and Outlier Ejection

**Cohesion:** 0.50 - moderately connected
**Members:** 4 nodes

## Members
- [[Panic Threshold (Ignore Health When 50% Unhealthy)]] - rationale - architecture-patterns/load-balancing-and-service-discovery/learning.md
- [[Pitfall Deep Readiness Probe Ejects the Whole Fleet]] - rationale - architecture-patterns/load-balancing-and-service-discovery/learning.md
- [[Retry + Outlier Ejection Amplification Cascade]] - rationale - architecture-patterns/load-balancing-and-service-discovery/learning.md
- [[The Health-Check Split (Liveness vs Readiness vs Passive)]] - rationale - architecture-patterns/load-balancing-and-service-discovery/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Health_Checks_and_Outlier_Ejection
SORT file.name ASC
```
