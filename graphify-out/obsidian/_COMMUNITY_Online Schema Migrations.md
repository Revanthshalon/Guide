---
type: community
cohesion: 1.00
members: 2
---

# Online Schema Migrations

**Cohesion:** 1.00 - tightly connected
**Members:** 2 nodes

## Members
- [[Migrations Without Downtime]] - rationale - oss-tools/postgres/runbook.md
- [[Pitfall CREATE INDEX  DDL Locking a Live Table]] - rationale - oss-tools/postgres/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Online_Schema_Migrations
SORT file.name ASC
```
