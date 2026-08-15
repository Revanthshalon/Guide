---
type: community
cohesion: 0.25
members: 8
---

# Terraform State Drift Model

**Cohesion:** 0.25 - loosely connected
**Members:** 8 nodes

## Members
- [[Divergence Classification (Name It, Then Fix It)]] - rationale - oss-tools/opentofu/reference.md
- [[Drift Detection (Scheduled, Not Discovered)]] - rationale - oss-tools/opentofu/runbook.md
- [[Importing Existing Infrastructure]] - concept - oss-tools/opentofu/runbook.md
- [[OpenTofu — Learning Notes]] - document - oss-tools/opentofu/learning.md
- [[OpenTofu — Quick Reference]] - document - oss-tools/opentofu/reference.md
- [[OpenTofu — Setup & Operations Runbook]] - document - oss-tools/opentofu/runbook.md
- [[Pitfall Drift from Manual Changes]] - rationale - oss-tools/opentofu/learning.md
- [[The Three-Way Model (Config, State, Reality)]] - rationale - oss-tools/opentofu/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Terraform_State_Drift_Model
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_State File Layout and Surgery]]

## Top bridge nodes
- [[The Three-Way Model (Config, State, Reality)]] - degree 5, connects to 1 community