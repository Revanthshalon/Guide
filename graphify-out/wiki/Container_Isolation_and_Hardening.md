# Container Isolation and Hardening

> 9 nodes · cohesion 0.22

## Key Concepts

- **Pitfall: Running as Root (container UID 0 is host UID 0)** (3 connections) — `oss-tools/docker/learning.md`
- **Docker Common Mistakes → Consequences** (3 connections) — `oss-tools/docker/runbook.md`
- **Compose Is a Local-Dev Orchestrator, Not Production** (2 connections) — `oss-tools/docker/learning.md`
- **Namespaces and cgroups — Container Is Not a VM** (2 connections) — `oss-tools/docker/learning.md`
- **Docker Security Flags (cap-drop, read-only, no-new-privileges)** (2 connections) — `oss-tools/docker/reference.md`
- **Day-1 Hardening Sequence** (2 connections) — `oss-tools/docker/runbook.md`
- **OSS Tool Runbook Template** (1 connections) — `oss-tools/_template-runbook.md`
- **Podman (daemonless, rootless by default)** (1 connections) — `oss-tools/docker/learning.md`
- **Compose: service_healthy, healthcheck, named volumes** (1 connections) — `oss-tools/docker/reference.md`

## Relationships

- [Content-Addressed Object Models](Content-Addressed_Object_Models.md) (1 shared connections)

## Source Files

- `oss-tools/_template-runbook.md`
- `oss-tools/docker/learning.md`
- `oss-tools/docker/reference.md`
- `oss-tools/docker/runbook.md`

## Audit Trail

- EXTRACTED: 8 (89%)
- INFERRED: 1 (11%)
- AMBIGUOUS: 0 (0%)

---

*Part of the graphify knowledge wiki. See [index](index.md) to navigate.*