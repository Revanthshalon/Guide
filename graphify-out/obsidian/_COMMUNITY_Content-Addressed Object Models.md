---
type: community
cohesion: 0.22
members: 9
---

# Content-Addressed Object Models

**Cohesion:** 0.22 - loosely connected
**Members:** 9 nodes

## Members
- [[BuildKit Secret and SSH Mounts]] - concept - oss-tools/docker/learning.md
- [[Git Content-Addressed Object Model]] - concept - developer-tooling/git/reference.md
- [[Layers Are Immutable, Additive, Content-Addressed]] - concept - oss-tools/docker/learning.md
- [[OSS Tool Quick Reference Template]] - document - oss-tools/_template-reference.md
- [[PID 1 Signal Semantics and Exec Form]] - concept - oss-tools/docker/learning.md
- [[Pitfall Secrets Baked Into Image Layers]] - concept - oss-tools/docker/learning.md
- [[SIGTERM Drain Handler and Stop Sequence]] - concept - oss-tools/docker/runbook.md
- [[The Docker Model in Four Facts]] - document - oss-tools/docker/reference.md
- [[Working Tree → Index → Repository]] - concept - developer-tooling/git/reference.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Content-Addressed_Object_Models
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Git Undo and Recovery]]
- 1 edge to [[_COMMUNITY_Docker Build Caching and Release]]
- 1 edge to [[_COMMUNITY_Container Isolation and Hardening]]
- 1 edge to [[_COMMUNITY_Secret Rotation and Worktrees]]
- 1 edge to [[_COMMUNITY_Rust Ownership Practices]]

## Top bridge nodes
- [[Pitfall Secrets Baked Into Image Layers]] - degree 4, connects to 2 communities
- [[The Docker Model in Four Facts]] - degree 4, connects to 1 community
- [[Git Content-Addressed Object Model]] - degree 3, connects to 1 community
- [[SIGTERM Drain Handler and Stop Sequence]] - degree 2, connects to 1 community