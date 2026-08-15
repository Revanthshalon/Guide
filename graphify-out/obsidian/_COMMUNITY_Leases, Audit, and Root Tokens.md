---
type: community
cohesion: 0.25
members: 8
---

# Leases, Audit, and Root Tokens

**Cohesion:** 0.25 - loosely connected
**Members:** 8 nodes

## Members
- [[Audit Devices]] - concept - oss-tools/openbao/learning.md
- [[Bao Agent — File-Rendered Secrets]] - concept - oss-tools/openbao/runbook.md
- [[Day-1 Hardening (Audit First, Root Token Last)]] - rationale - oss-tools/openbao/runbook.md
- [[Leases, Renewal, and Revocation]] - concept - oss-tools/openbao/learning.md
- [[OpenBao — Learning Notes]] - document - oss-tools/openbao/learning.md
- [[OpenBao — Quick Reference]] - document - oss-tools/openbao/reference.md
- [[Pitfall Lease and TTL Explosions]] - rationale - oss-tools/openbao/learning.md
- [[Pitfall The Root Token That Never Died]] - rationale - oss-tools/openbao/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Leases_Audit_and_Root_Tokens
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Raft Storage, Seal, and Backup]]
- 2 edges to [[_COMMUNITY_Secret Engines and State Secrets]]
- 1 edge to [[_COMMUNITY_Machine Identity and CI Credentials]]

## Top bridge nodes
- [[OpenBao — Learning Notes]] - degree 9, connects to 3 communities