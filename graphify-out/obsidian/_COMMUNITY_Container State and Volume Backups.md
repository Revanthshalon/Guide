---
type: community
cohesion: 0.33
members: 6
---

# Container State and Volume Backups

**Cohesion:** 0.33 - loosely connected
**Members:** 6 nodes

## Members
- [[Copy-on-Write Layer vs Volumes for State]] - concept - oss-tools/docker/learning.md
- [[Disk Fills Silently — prune and Log Rotation]] - concept - oss-tools/docker/runbook.md
- [[Docker Migration Checklist]] - document - oss-tools/docker/reference.md
- [[Docker Monitoring Signals Table]] - document - oss-tools/docker/runbook.md
- [[Migration Walkthrough VM → Containers]] - concept - oss-tools/docker/learning.md
- [[Volume Backups and Restore Drill]] - concept - oss-tools/docker/runbook.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Container_State_and_Volume_Backups
SORT file.name ASC
```
