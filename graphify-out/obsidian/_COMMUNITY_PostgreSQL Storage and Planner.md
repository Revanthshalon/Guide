---
type: community
cohesion: 0.16
members: 14
---

# PostgreSQL Storage and Planner

**Cohesion:** 0.16 - loosely connected
**Members:** 14 nodes

## Members
- [[Autovacuum Tuning (Defaults Too Lazy for Busy Tables)]] - rationale - oss-tools/postgres/runbook.md
- [[Cost-Based, Statistics-Driven Planner]] - rationale - oss-tools/postgres/learning.md
- [[Initialize with Data Checksums On]] - rationale - oss-tools/postgres/runbook.md
- [[Logical-Replication Upgrade (Near-Zero Downtime)]] - concept - oss-tools/postgres/runbook.md
- [[MVCC and the Visibility Map]] - concept - oss-tools/postgres/learning.md
- [[Pitfall Assuming an Index Will Be Used]] - rationale - oss-tools/postgres/learning.md
- [[Pitfall Long Transactions Blocking Vacuum]] - rationale - oss-tools/postgres/learning.md
- [[PostgreSQL Extensions]] - concept - oss-tools/postgres/learning.md
- [[PostgreSQL — Learning Notes]] - document - oss-tools/postgres/learning.md
- [[PostgreSQL — Quick Reference]] - document - oss-tools/postgres/reference.md
- [[PostgreSQL — Setup & Operations Runbook]] - document - oss-tools/postgres/runbook.md
- [[The B-tree HeapIndex Split]] - concept - oss-tools/postgres/learning.md
- [[WAL — Write-Ahead Log]] - concept - oss-tools/postgres/learning.md
- [[pg_upgrade with Hard Links]] - concept - oss-tools/postgres/runbook.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/PostgreSQL_Storage_and_Planner
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Async Runtimes and Allocators]]
- 1 edge to [[_COMMUNITY_Raft Storage, Seal, and Backup]]
- 1 edge to [[_COMMUNITY_Connection Pooling and C10K]]
- 1 edge to [[_COMMUNITY_Branch Prediction and Branchless Code]]

## Top bridge nodes
- [[WAL — Write-Ahead Log]] - degree 4, connects to 2 communities
- [[PostgreSQL — Learning Notes]] - degree 8, connects to 1 community
- [[Cost-Based, Statistics-Driven Planner]] - degree 4, connects to 1 community