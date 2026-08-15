---
type: community
cohesion: 0.50
members: 4
---

# State File Layout and Surgery

**Cohesion:** 0.50 - moderately connected
**Members:** 4 nodes

## Members
- [[Pitfall The Monolithic State File]] - rationale - oss-tools/opentofu/learning.md
- [[State Layout — Decide Before Twenty Services]] - rationale - oss-tools/opentofu/runbook.md
- [[State Surgery (Always Back Up First)]] - rationale - oss-tools/opentofu/runbook.md
- [[State The Crown Jewel]] - concept - oss-tools/opentofu/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/State_File_Layout_and_Surgery
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Secret Engines and State Secrets]]
- 1 edge to [[_COMMUNITY_Terraform State Drift Model]]

## Top bridge nodes
- [[State The Crown Jewel]] - degree 4, connects to 2 communities