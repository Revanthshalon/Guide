---
type: community
cohesion: 0.17
members: 12
---

# Secret Engines and State Secrets

**Cohesion:** 0.17 - loosely connected
**Members:** 12 nodes

## Members
- [[Backend Bootstrap (The Chicken-and-Egg Step)]] - rationale - oss-tools/opentofu/runbook.md
- [[Backends, Locking, and Workspace Layout]] - concept - oss-tools/opentofu/learning.md
- [[KV v2 Secret Engine]] - concept - oss-tools/openbao/learning.md
- [[Least-Privilege Service Policy]] - concept - oss-tools/openbao/runbook.md
- [[OpenBao Policies]] - concept - oss-tools/openbao/learning.md
- [[Pitfall Secrets in State (and in Plans)]] - rationale - oss-tools/opentofu/learning.md
- [[Pitfall State Loss, Corruption, or Concurrent Writes]] - rationale - oss-tools/opentofu/learning.md
- [[Pitfall Static Secrets In, Static Habits Kept]] - rationale - oss-tools/openbao/learning.md
- [[Secret Engines]] - concept - oss-tools/openbao/learning.md
- [[State Encryption (OpenTofu's Flagship Divergence)]] - rationale - oss-tools/opentofu/learning.md
- [[Transit Engine — Envelope Encryption KMS Role]] - rationale - oss-tools/openbao/learning.md
- [[pg_hba.conf — No Trailing Catch-All]] - rationale - oss-tools/postgres/runbook.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Secret_Engines_and_State_Secrets
SORT file.name ASC
```

## Connections to other communities
- 2 edges to [[_COMMUNITY_Leases, Audit, and Root Tokens]]
- 1 edge to [[_COMMUNITY_State File Layout and Surgery]]

## Top bridge nodes
- [[Secret Engines]] - degree 4, connects to 1 community
- [[State Encryption (OpenTofu's Flagship Divergence)]] - degree 4, connects to 1 community
- [[OpenBao Policies]] - degree 2, connects to 1 community