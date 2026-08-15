---
type: community
cohesion: 0.22
members: 9
---

# Machine Identity and CI Credentials

**Cohesion:** 0.22 - loosely connected
**Members:** 9 nodes

## Members
- [[AppRole (Machine Identity Without Platform Attestation)]] - concept - oss-tools/openbao/learning.md
- [[Auth Methods and Identity]] - concept - oss-tools/openbao/learning.md
- [[CI Credentials via OIDC (No Stored Keys)]] - rationale - oss-tools/opentofu/runbook.md
- [[Kubernetes Auth (Platform Identity)]] - concept - oss-tools/openbao/runbook.md
- [[OpenTofu CICD Pipeline]] - concept - oss-tools/opentofu/runbook.md
- [[Pitfall Auto-Apply Without a Reviewed Plan]] - rationale - oss-tools/opentofu/learning.md
- [[Policy Gate (Rego  no_destroy)]] - concept - oss-tools/opentofu/runbook.md
- [[Response-Wrapped SecretID Delivery]] - rationale - oss-tools/openbao/runbook.md
- [[Secure Introduction]] - rationale - oss-tools/openbao/runbook.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Machine_Identity_and_CI_Credentials
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Leases, Audit, and Root Tokens]]

## Top bridge nodes
- [[Auth Methods and Identity]] - degree 2, connects to 1 community