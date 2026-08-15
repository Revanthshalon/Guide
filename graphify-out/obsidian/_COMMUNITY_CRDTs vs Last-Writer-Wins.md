---
type: community
cohesion: 1.00
members: 2
---

# CRDTs vs Last-Writer-Wins

**Cohesion:** 1.00 - tightly connected
**Members:** 2 nodes

## Members
- [[CRDTs and Version Vectors]] - concept - architecture-patterns/replication-and-consistency/learning.md
- [[Last-Writer-Wins Silently Eating Writes]] - rationale - architecture-patterns/replication-and-consistency/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/CRDTs_vs_Last-Writer-Wins
SORT file.name ASC
```
