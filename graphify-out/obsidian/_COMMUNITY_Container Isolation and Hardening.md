---
type: community
cohesion: 0.22
members: 9
---

# Container Isolation and Hardening

**Cohesion:** 0.22 - loosely connected
**Members:** 9 nodes

## Members
- [[Compose Is a Local-Dev Orchestrator, Not Production]] - rationale - oss-tools/docker/learning.md
- [[Compose service_healthy, healthcheck, named volumes]] - document - oss-tools/docker/reference.md
- [[Day-1 Hardening Sequence]] - concept - oss-tools/docker/runbook.md
- [[Docker Common Mistakes → Consequences]] - document - oss-tools/docker/runbook.md
- [[Docker Security Flags (cap-drop, read-only, no-new-privileges)]] - document - oss-tools/docker/reference.md
- [[Namespaces and cgroups — Container Is Not a VM]] - concept - oss-tools/docker/learning.md
- [[OSS Tool Runbook Template]] - document - oss-tools/_template-runbook.md
- [[Pitfall Running as Root (container UID 0 is host UID 0)]] - concept - oss-tools/docker/learning.md
- [[Podman (daemonless, rootless by default)]] - concept - oss-tools/docker/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Container_Isolation_and_Hardening
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Content-Addressed Object Models]]

## Top bridge nodes
- [[Docker Common Mistakes → Consequences]] - degree 3, connects to 1 community