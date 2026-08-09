# Docker & Docker Compose — Learning Notes

## What It Is & Why It Exists

**Docker packages a process together with its filesystem so it runs identically everywhere.** It did not invent containers — Linux namespaces, cgroups, and chroot predate it, and LXC exposed them years earlier. What Docker contributed was **an image format, a build file, and a registry**, which turned an operating-system feature into a distribution mechanism. That's the actual innovation: `docker pull` is the point, not `docker run`.

The lineage matters operationally because the ecosystem has since split apart:

- **Docker Engine** — the daemon (`dockerd`) plus CLI. Apache 2.0. **Docker Desktop** (the Mac/Windows GUI) is separately licensed and requires a paid subscription for larger companies — the one licensing trap in this stack.
- **containerd** — extracted from Docker in 2017, donated to the CNCF, and now the runtime Kubernetes actually uses. Docker Engine runs on top of it.
- **runc** — the OCI reference runtime that actually calls `clone()` with namespace flags.
- **OCI** — the image and runtime specs, so images are portable across Docker, containerd, Podman, and Kubernetes. **This is why "Docker image" is really "OCI image"** and why you're not locked in.

**Rootless alternatives** worth knowing: **Podman** (daemonless, rootless by default, drop-in CLI compatible) and **BuildKit/buildah** for building without a daemon. On a CI runner or a hardened host, "no long-running root daemon" is a real advantage.

## Architecture & Core Concepts

### Images are layered, content-addressed, and shared

- **What it is:** An image is an ordered stack of read-only filesystem layers, each identified by the SHA-256 of its content, plus a JSON manifest. A container is those layers plus a thin writable layer on top (copy-on-write).
- **Why it matters operationally:** Layers are **cached and shared** — ten containers from the same image consume one copy on disk. Every `Dockerfile` instruction that changes the filesystem creates a layer, and **a layer is immutable once created**, which is why `RUN rm secret.txt` after `COPY secret.txt` does *not* remove the secret: it's still in the earlier layer, and anyone with the image can extract it. This is the same structural-sharing model as [persistent structures](../../data-structures-and-algorithms/persistent-immutable-structures/learning.md), with the same consequence that nothing is ever really deleted.

### The build cache is order-dependent

- **What it is:** Docker caches each instruction's result keyed on the instruction and the content it touches. A cache hit requires all *previous* layers to have hit too.
- **Why it matters operationally:** Instruction order *is* build performance. `COPY . .` before `RUN cargo build` invalidates the dependency build on every source change; copying `Cargo.toml`/`Cargo.lock` first and building dependencies as a separate layer keeps that cache. This is the single biggest lever on CI build time.

### Namespaces and cgroups — what isolation actually means

- **What it is:** Namespaces virtualize *what a process can see* (PID, network, mount, UTS, IPC, user); cgroups limit *what it can consume* (CPU, memory, I/O, PIDs).
- **Why it matters operationally:** **A container is not a VM.** It shares the host kernel, so a kernel exploit crosses the boundary, and `--privileged` or a mounted Docker socket effectively hands over root on the host. Containers are a packaging and resource-isolation mechanism, not a security boundary against untrusted code — for that you need gVisor, Kata, Firecracker, or a real VM.

### Compose is a local-development orchestrator

- **What it is:** A declarative multi-container spec (`compose.yaml`) with a dependency graph, shared networks, and volumes. Compose V2 is a Go plugin (`docker compose`), replacing the Python V1 (`docker-compose`).
- **Why it matters operationally:** Compose is excellent for local development and small single-host deployments, and it is **not** a production orchestrator — no rescheduling, no rolling updates with health gating, no multi-host scheduling. The natural production path is Kubernetes, Nomad, or a managed container service. Treating Compose as production is a decision to have no failover.

### The layered-filesystem/write-path trade

- **What it is:** Writes go to the container's thin writable layer via copy-on-write (overlayfs), so modifying a large file first copies it up.
- **Why it matters operationally:** Container filesystems are for *ephemeral* data. Databases and anything write-heavy belong on a **volume** (a bind mount or a named volume that bypasses the layered filesystem), both for performance and because the writable layer is destroyed with the container.

## Comparison in Depth

| Aspect | VM | **Container** | Podman |
| --- | --- | --- | --- |
| Isolation | Hardware-level, own kernel | **Shared kernel, namespaces** | Same |
| Startup | Seconds–minutes | **Milliseconds** | Milliseconds |
| Overhead | GBs, full OS | **MBs, one process tree** | Same |
| Security boundary | **Strong** | Weak against kernel exploits | Better (rootless default) |
| Daemon | Hypervisor | **`dockerd` runs as root** | **Daemonless** |
| Density | Tens per host | Hundreds | Hundreds |

**Docker vs Podman**: Podman is CLI-compatible, runs rootless by default, needs no daemon, and integrates with systemd for service management. Docker has broader tooling and Compose maturity. For CI runners and hardened hosts, Podman's lack of a root daemon is a genuine security improvement.

## Pitfalls in Depth

### Pitfall: Secrets baked into image layers

- **What goes wrong:** A build copies a `.npmrc`, an SSH key, or an `.env` into the image, uses it, then deletes it — `RUN rm -f /root/.ssh/id_rsa`. The image "doesn't contain" the key, yet `docker history` and any layer extraction recover it immediately. The image is then pushed to a registry, and the secret is distributed to everyone who can pull it.
- **Why it happens (the mechanism):** Layers are immutable and additive. Deleting a file in a later layer records a *whiteout* marker; the bytes remain in the earlier layer, which is still part of the image and still content-addressed in the registry. `--squash` helps only if enabled and doesn't cover already-pushed images.
- **How to handle it in production, and why that works:** Use **BuildKit secret mounts** — `RUN --mount=type=secret,id=npmrc ...` makes the file available only during that instruction's execution and never writes it to a layer. For private Git dependencies use `--mount=type=ssh` to forward the agent. For runtime secrets, inject at *run* time via environment or a mounted file from a secret store ([OpenBao](../openbao/learning.md)), never at build time.
- **Trade-offs of the fix:** BuildKit secret mounts require BuildKit (default since Docker 23, but not on old CI images) and the `# syntax=docker/dockerfile:1` header. They also can't be used by tooling that shells out to a plain `docker build` on an old daemon — which is usually the reason people fall back to the unsafe pattern.

### Pitfall: PID 1 and signal handling

- **What goes wrong:** `docker stop` appears to hang for 10 seconds, then the container is killed. Graceful shutdown never runs — in-flight requests are dropped, connections aren't drained, buffers aren't flushed.
- **Why it happens (the mechanism):** Two distinct causes. First, `CMD python app.py` in **shell form** runs `/bin/sh -c` as PID 1, and `sh` does not forward `SIGTERM` to its child. Second, PID 1 in a namespace has **special kernel semantics**: default signal handlers are not installed, so a process that doesn't explicitly handle `SIGTERM` simply ignores it. Docker then waits `--time` (default 10 s) and sends `SIGKILL`.
- **How to handle it in production, and why that works:** Use **exec form** — `CMD ["./server"]` — so your process *is* PID 1 with no shell in between, and install an explicit `SIGTERM` handler that drains and exits. If the process genuinely can't handle signals or spawns children that need reaping, run `docker run --init` (or `init: true` in Compose), which inserts `tini` as PID 1 to forward signals and reap zombies.
- **Trade-offs of the fix:** Exec form doesn't expand shell variables or globs, so `CMD ["sh","-c","exec ./server --port=$PORT"]` is the pattern when you need them — note the `exec`, which replaces the shell so your process still becomes PID 1. `--init` adds a tiny process and is almost free.

### Pitfall: Running as root

- **What goes wrong:** No `USER` instruction, so the container process runs as UID 0. A remote-code-execution bug in the application is then root inside the container — and with a mounted Docker socket, a bind-mounted host path, or a kernel escape, root on the host.
- **Why it happens (the mechanism):** Root is the default and everything works, so nothing prompts a change. Many base images and package installs assume root during build, and the `USER` line is easy to omit. Crucially, **container UID 0 is host UID 0** unless user namespaces are enabled — the isolation is namespace visibility, not privilege reduction.
- **How to handle it in production, and why that works:** Add a non-root `USER` after installing packages, and pair it with `--read-only`, `--cap-drop=ALL` (adding back only what's needed), and `--security-opt=no-new-privileges`. Enable rootless mode or user-namespace remapping so container UID 0 maps to an unprivileged host UID — then even a container escape lands as nobody.
- **Trade-offs of the fix:** Non-root breaks binding to ports below 1024 (use a high port and map it: `-p 80:8080`), and file permissions on mounted volumes must match the UID, which is a common source of "permission denied" on bind mounts. Read-only root filesystems need explicit `tmpfs` mounts for anything that writes.

### Pitfall: Bloated images and the build cache

- **What goes wrong:** A Rust or Node image is 1.5 GB because the build toolchain shipped with it. Pulls are slow, registry costs rise, the attack surface includes a compiler and a package manager, and CI rebuilds everything on every source change because `COPY . .` came before the dependency install.
- **Why it happens (the mechanism):** Every instruction adds a layer and layers are never removed. Build-time dependencies (compilers, headers, package caches) stay in the image unless the build is staged. And cache invalidation is *positional* — any change to a copied file invalidates that layer and everything after it.
- **How to handle it in production, and why that works:** **Multi-stage builds** — compile in a full toolchain stage, then `COPY --from=builder` only the artifact into a minimal runtime (`debian:slim`, `alpine`, `gcr.io/distroless/cc`, or `scratch` for a static binary). Order the Dockerfile from least- to most-frequently-changing: base → system packages → dependency manifests → dependency build → source → app build. That keeps the expensive dependency layer cached across source edits.
- **Trade-offs of the fix:** `alpine` uses musl, which for Rust means a different target and, per [releasing](../../language-best-practices/rust/releasing.md), a notably slower allocator — measure before adopting it for an allocation-heavy service. `distroless` and `scratch` have no shell, which makes debugging harder (use `docker debug`, an ephemeral debug container, or a separate debug image variant).

### Pitfall: Treating Compose as production

- **What goes wrong:** A `compose.yaml` is deployed to a single production host. It works until the host dies, at which point there is no rescheduling, no failover, and no rolling update — `docker compose up -d` recreates containers with a gap in service, and a failed health check does not roll back.
- **Why it happens (the mechanism):** Compose is genuinely excellent at what it does and the gap to "production" isn't visible from the file: the same YAML describes both. But Compose has no scheduler, no notion of a desired replica count across hosts, and no health-gated deployment strategy. Its `depends_on` waits for *start*, not readiness, unless you add `condition: service_healthy`.
- **How to handle it in production, and why that works:** Use Compose for local development and CI, and a real orchestrator (Kubernetes, Nomad, ECS) or a managed platform for production. If a single host genuinely is the requirement (an internal tool, a small service), be explicit that the availability model is "restart on failure" — set `restart: unless-stopped`, define health checks, and accept the downtime window rather than assuming it away.
- **Trade-offs of the fix:** Kubernetes is a large operational commitment for a small service, and that cost is real. The honest middle ground is a managed container service, or Compose with the availability limitation *documented* — the failure mode here is not using Compose, it's not knowing what it doesn't do.

## Migration Walkthrough

**From bare-metal/VM deployment to containers:**

1. **Externalize configuration** — environment variables and mounted files, no config baked into the image. The same image must run in every environment; only inputs differ.
2. **Externalize state** — anything written must go to a volume or an external service. Identify every path the app writes to; anything not on a volume is lost on restart.
3. **Fix logging** — log to `stdout`/`stderr`, not files. The container runtime collects them; a log file inside a container is invisible and fills the writable layer.
4. **Add health endpoints** — `/health` (liveness: am I alive) and `/ready` (readiness: can I serve traffic). These are what orchestrators gate on.
5. **Handle `SIGTERM`** — drain connections and exit within the grace period, or every deploy drops requests.
6. **Then containerize**, multi-stage, non-root, pinned base image digest.

## Open Questions

- Podman vs Docker for local Rust development on macOS — does the lack of a daemon cost anything in practice, and how does the VM layer compare on filesystem performance?
- `cargo-chef` vs manual dependency-layer caching for Rust builds: how much CI time does it actually save on a realistic workspace?
- musl vs glibc for a Rust service image — the allocator gap flagged in [releasing](../../language-best-practices/rust/releasing.md), measured on a real workload.
- BuildKit cache mounts (`--mount=type=cache`) for `~/.cargo` in CI: how much do they help when the runner is ephemeral?
- Is `distroless` worth the debugging cost over `debian:slim` for a service that already ships a static binary?

## References

- [Docker documentation](https://docs.docker.com/) — the build and Compose sections are the operational core
- [Dockerfile best practices](https://docs.docker.com/build/building/best-practices/) — layer ordering and multi-stage, from the source
- [OCI Image Spec](https://github.com/opencontainers/image-spec) — what an "image" actually is
- [`cargo-chef`](https://github.com/LukeMathWalker/cargo-chef) — dependency-layer caching for Rust builds
- Related in this repo: [runbook.md](runbook.md) (the procedures), [reference.md](reference.md), [Rust releasing](../../language-best-practices/rust/releasing.md) (build profiles, musl vs glibc, reproducibility), [Persistent & Immutable Structures](../../data-structures-and-algorithms/persistent-immutable-structures/learning.md) (layer sharing is structural sharing), [OpenBao](../openbao/learning.md) (runtime secret injection)
