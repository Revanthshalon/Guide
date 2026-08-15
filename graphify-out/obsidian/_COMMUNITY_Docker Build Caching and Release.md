---
type: community
cohesion: 0.22
members: 10
---

# Docker Build Caching and Release

**Cohesion:** 0.22 - loosely connected
**Members:** 10 nodes

## Members
- [[Binary That Won't Run on the Deployment Target (glibc vs musl)]] - concept - language-best-practices/rust/releasing.md
- [[Deploy by Digest, Not by Tag]] - rationale - oss-tools/docker/runbook.md
- [[Do .dockerignore First]] - concept - oss-tools/docker/runbook.md
- [[Multi-Stage Builds and Minimal Runtime Images]] - concept - oss-tools/docker/learning.md
- [[Registry-Backed BuildKit Cache for Ephemeral CI Runners]] - concept - oss-tools/docker/runbook.md
- [[Reproducible Release (clean tagged checkout, lockfile, pinned toolchain, embedded SHA)]] - concept - language-best-practices/rust/releasing.md
- [[The Build Cache Is Order-Dependent]] - concept - oss-tools/docker/learning.md
- [[The Dockerfile Shape That Caches]] - document - oss-tools/docker/reference.md
- [[The Multi-Stage Rust Dockerfile]] - document - oss-tools/docker/runbook.md
- [[cargo-chef (Rust dependency-layer caching)]] - concept - oss-tools/docker/learning.md

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Docker_Build_Caching_and_Release
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Content-Addressed Object Models]]
- 1 edge to [[_COMMUNITY_ripgrep Skipping and -u Ladder]]

## Top bridge nodes
- [[The Build Cache Is Order-Dependent]] - degree 5, connects to 2 communities