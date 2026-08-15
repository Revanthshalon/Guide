---
type: community
cohesion: 0.25
members: 9
---

# Raft Storage, Seal, and Backup

**Cohesion:** 0.25 - loosely connected
**Members:** 9 nodes

## Members
- [[Auto-Unseal via KMS]] - concept - oss-tools/openbao/runbook.md
- [[Initialization Ceremony (Happens Exactly Once)]] - rationale - oss-tools/openbao/runbook.md
- [[OpenBao — Setup & Operations Runbook]] - document - oss-tools/openbao/runbook.md
- [[Pitfall Treating OpenBao as Just-a-Database (Tier-0 Reality)]] - rationale - oss-tools/openbao/learning.md
- [[Raft Snapshot Backup and Restore]] - rationale - oss-tools/openbao/runbook.md
- [[Seal  Unseal and the Security Barrier]] - rationale - oss-tools/openbao/learning.md
- [[Shamir Human-Quorum Unseal]] - concept - oss-tools/openbao/runbook.md
- [[Storage Backend (Integrated Raft)]] - concept - oss-tools/openbao/learning.md
- [[pgBackRest Backups and the Restore Drill]] - rationale - oss-tools/postgres/runbook.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Raft_Storage_Seal_and_Backup
SORT file.name ASC
```

## Connections to other communities
- 3 edges to [[_COMMUNITY_Leases, Audit, and Root Tokens]]
- 1 edge to [[_COMMUNITY_PostgreSQL Storage and Planner]]

## Top bridge nodes
- [[Seal  Unseal and the Security Barrier]] - degree 4, connects to 1 community
- [[Storage Backend (Integrated Raft)]] - degree 3, connects to 1 community
- [[OpenBao — Setup & Operations Runbook]] - degree 2, connects to 1 community
- [[pgBackRest Backups and the Restore Drill]] - degree 2, connects to 1 community