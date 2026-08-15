# Docker Build Caching and Release

> 10 nodes · cohesion 0.22

## Key Concepts

- **The Build Cache Is Order-Dependent** (5 connections) — `oss-tools/docker/learning.md`
- **Multi-Stage Builds and Minimal Runtime Images** (4 connections) — `oss-tools/docker/learning.md`
- **Binary That Won't Run on the Deployment Target (glibc vs musl)** (2 connections) — `language-best-practices/rust/releasing.md`
- **Reproducible Release (clean tagged checkout, lockfile, pinned toolchain, embedded SHA)** (2 connections) — `language-best-practices/rust/releasing.md`
- **cargo-chef (Rust dependency-layer caching)** (2 connections) — `oss-tools/docker/learning.md`
- **The Dockerfile Shape That Caches** (2 connections) — `oss-tools/docker/reference.md`
- **The Multi-Stage Rust Dockerfile** (2 connections) — `oss-tools/docker/runbook.md`
- **Deploy by Digest, Not by Tag** (1 connections) — `oss-tools/docker/runbook.md`
- **Do .dockerignore First** (1 connections) — `oss-tools/docker/runbook.md`
- **Registry-Backed BuildKit Cache for Ephemeral CI Runners** (1 connections) — `oss-tools/docker/runbook.md`

## Relationships

- [Content-Addressed Object Models](Content-Addressed_Object_Models.md) (1 shared connections)
- [ripgrep Skipping and -u Ladder](ripgrep_Skipping_and_-u_Ladder.md) (1 shared connections)

## Source Files

- `language-best-practices/rust/releasing.md`
- `oss-tools/docker/learning.md`
- `oss-tools/docker/reference.md`
- `oss-tools/docker/runbook.md`

## Audit Trail

- EXTRACTED: 8 (67%)
- INFERRED: 4 (33%)
- AMBIGUOUS: 0 (0%)

---

*Part of the graphify knowledge wiki. See [index](index.md) to navigate.*