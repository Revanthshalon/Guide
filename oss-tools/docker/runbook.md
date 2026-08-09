# Docker & Docker Compose — Setup & Operations Runbook

> **Accuracy note:** Reflects **Docker Engine 27+ / Compose V2** (2024–2025). Verify with `docker version` and `docker compose version`. Concepts are in [learning.md](learning.md); this is the procedure.

## Part 1 — Development setup

### 1.1 Install and verify

```sh
docker version                 # client AND server must both respond
docker compose version         # V2 is `docker compose` (space), not `docker-compose`
docker run --rm hello-world
docker buildx version          # BuildKit — needed for secret and cache mounts
```

On macOS/Windows, Docker Desktop runs a Linux VM; **filesystem bind mounts cross that VM boundary and are slow**. For large source trees use a named volume for build artifacts (`target/`, `node_modules/`) rather than bind-mounting them.

**Licensing:** Docker Desktop requires a paid subscription for companies above the free-tier threshold. Alternatives: Podman Desktop, Colima, OrbStack, or plain Docker Engine on Linux.

### 1.2 What dev setup does differently

| Dev shortcut | Production requirement | Why it matters |
| --- | --- | --- |
| `:latest` tags | **Pin by digest** | Builds aren't reproducible; supply-chain risk |
| Root user | Non-root `USER` | RCE becomes container root, possibly host root |
| Bind-mounted source | Baked-in artifact | The image isn't self-contained |
| No resource limits | `--memory`, `--cpus`, `--pids-limit` | One container can take down the host |
| Secrets in `.env` | Injected from a secret store | `.env` gets committed; images get pushed |
| No health checks | Liveness + readiness | Orchestrator can't tell broken from starting |
| Compose | Real orchestrator | No rescheduling or rolling updates |

### 1.3 `.dockerignore` — do this first

```
target/
node_modules/
.git/
.env*
**/*.log
Dockerfile*
compose*.yaml
```

Without it, the build context ships your entire `.git` and `target/` to the daemon — slow builds and secrets in the context.

## Part 2 — Building images properly

### 2.1 The multi-stage Rust Dockerfile

```dockerfile
# syntax=docker/dockerfile:1
ARG RUST_VERSION=1.83

FROM rust:${RUST_VERSION}-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*        # same layer, or the cache stays in the image

# Dependency layer — survives source edits (the biggest CI win)
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
 && cargo build --release --locked \
 && rm -rf src target/release/deps/app*

COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /app/target/release/app /app
USER nonroot:nonroot
EXPOSE 8080
ENTRYPOINT ["/app"]            # exec form: the binary is PID 1
```

`--locked` makes the build fail rather than silently update dependencies — the reproducibility requirement from [Rust releasing](../../language-best-practices/rust/releasing.md).

For a serious Rust setup, use **`cargo-chef`**, which computes a dependency-only recipe so the cache layer is exact rather than a dummy-`main.rs` approximation.

### 2.2 Build secrets — never `COPY` them

```dockerfile
# syntax=docker/dockerfile:1
RUN --mount=type=secret,id=netrc,target=/root/.netrc \
    cargo build --release          # .netrc exists only during this instruction

RUN --mount=type=ssh git clone git@github.com:org/private.git
```

```sh
docker build --secret id=netrc,src=$HOME/.netrc --ssh default -t app .
```

Verify nothing leaked:

```sh
docker history --no-trunc app | grep -i -E 'secret|token|password|key'
docker run --rm -it --entrypoint sh app -c 'ls -la /root 2>/dev/null'
```

### 2.3 Multi-architecture builds

```sh
docker buildx create --use --name multi
docker buildx build --platform linux/amd64,linux/arm64 \
  -t registry/app:1.2.3 --push .
```

Building `arm64` on `amd64` (or vice versa) uses QEMU emulation and is **very slow** — use native runners per architecture where build time matters.

## Part 3 — Compose for local development

`compose.yaml`:

```yaml
name: myapp

services:
  api:
    build:
      context: .
      target: builder          # dev: keep the toolchain for fast rebuilds
    init: true
    restart: unless-stopped
    ports: ["8080:8080"]
    environment:
      DATABASE_URL: postgres://app:${DB_PASSWORD:?set DB_PASSWORD}@db:5432/app
      RUST_LOG: info
    depends_on:
      db: { condition: service_healthy }
    healthcheck:
      test: ["CMD", "/app", "--health-check"]
      interval: 10s
      timeout: 3s
      retries: 3
      start_period: 30s          # grace while starting — failures here don't count
    volumes:
      - ./src:/app/src:ro        # bind mount source for reload
      - cargo-target:/app/target # named volume: build artifacts stay OFF the bind mount

  db:
    image: postgres:17@sha256:...   # pin by digest
    environment:
      POSTGRES_USER: app
      POSTGRES_PASSWORD: ${DB_PASSWORD:?set DB_PASSWORD}
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U app"]
      interval: 5s
      retries: 10

volumes:
  pgdata:
  cargo-target:
```

**`condition: service_healthy` is the important line.** Bare `depends_on` waits only for the container to *start*, so the app races the database's readiness and crash-loops on first boot.

Layer environment-specific overrides:

```sh
docker compose -f compose.yaml -f compose.override.yaml up -d   # override is implicit
docker compose -f compose.yaml -f compose.prod.yaml up -d
docker compose config          # ALWAYS check the merged result before deploying
```

## Part 4 — Day-1 hardening

Run in order; each assumes the previous.

1. **Non-root user.** `USER nonroot`. Verify: `docker run --rm app id` must not print `uid=0`.
2. **Drop capabilities and go read-only.**
   ```sh
   docker run --read-only --tmpfs /tmp --cap-drop=ALL \
     --cap-add=NET_BIND_SERVICE --security-opt=no-new-privileges app
   ```
3. **Resource limits on every container** — `--memory`, `--cpus`, `--pids-limit`. Without them one container can OOM the host or fork-bomb it.
4. **Pin base images by digest**, not tag. Tags are mutable; digests are not.
5. **Scan before pushing.** `trivy image app:1.2.3` or `docker scout cves app:1.2.3`, wired into CI as a gate.
6. **Never mount the Docker socket** into a container, and never `--privileged`. Both are equivalent to handing over host root.
7. **Enable rootless mode** (`dockerd-rootless-setuptool.sh install`) or user-namespace remapping so container UID 0 maps to an unprivileged host UID.

## Part 5 — Signals and lifecycle (the thing most often gotten wrong)

The stop sequence: Docker sends `SIGTERM` → waits `--time` (default **10 s**) → sends `SIGKILL`.

```rust
// Handle SIGTERM or every deploy drops in-flight requests.
use tokio::signal::unix::{signal, SignalKind};

let mut term = signal(SignalKind::terminate())?;
tokio::select! {
    _ = server => {},
    _ = term.recv() => {
        tracing::info!("SIGTERM received; draining");
        // stop accepting, finish in-flight, close pools
    }
}
```

Diagnose a container that won't stop gracefully:

```sh
docker inspect <c> --format '{{.Config.Entrypoint}} {{.Config.Cmd}}'   # exec form? array = yes
docker exec <c> ps -ef                       # is your process PID 1, or is `sh`?
time docker stop <c>                         # ~10 s = signal is being ignored
```

| Symptom | Cause | Fix |
| --- | --- | --- |
| `stop` takes exactly 10 s | Signal ignored | Exec form + `SIGTERM` handler |
| Zombie processes accumulate | PID 1 not reaping | `--init` / `init: true` |
| Logs lost on stop | Buffered stdout | Unbuffered/line-buffered output, flush on exit |
| Restart loop | Health check fails during startup | Raise `start_period` |

## Part 6 — Registry and CI integration

```sh
# Authenticate (CI: use a token with push-only scope, not a personal login)
echo "$REGISTRY_TOKEN" | docker login registry.example.com -u ci --password-stdin

# Tag by immutable identity, not by `latest`
docker build -t registry/app:${GIT_SHA} -t registry/app:1.2.3 .
docker push registry/app:${GIT_SHA}
docker push registry/app:1.2.3

# Resolve the digest and deploy THAT
docker inspect --format='{{index .RepoDigests 0}}' registry/app:${GIT_SHA}
```

**Deploy by digest.** A tag can be moved; a digest cannot. This is the container equivalent of the clean-checkout requirement in [Rust releasing](../../language-best-practices/rust/releasing.md).

Cache across CI runs (ephemeral runners have no local cache):

```sh
docker buildx build \
  --cache-from type=registry,ref=registry/app:buildcache \
  --cache-to   type=registry,ref=registry/app:buildcache,mode=max \
  -t registry/app:${GIT_SHA} --push .
```

## Part 7 — Day-2 operations

### 7.1 Disk — the failure that arrives silently

```sh
docker system df                 # images / containers / volumes / build cache
docker system df -v              # per-object detail

docker image prune -a --filter "until=168h"   # unused images older than a week
docker builder prune --keep-storage 20GB
docker container prune
```

> `docker system prune -a --volumes` **deletes named volumes**, which is your database data. Never run it as a reflex on a host with stateful containers.

Cap log growth — the default `json-file` driver grows without bound:

```json
// /etc/docker/daemon.json
{
  "log-driver": "json-file",
  "log-opts": { "max-size": "50m", "max-file": "5" },
  "live-restore": true
}
```

`live-restore: true` keeps containers running across a daemon restart — worth having before you need it.

### 7.2 Backups (volumes)

```sh
# Back up a named volume
docker run --rm -v pgdata:/data:ro -v "$PWD":/backup alpine \
  tar czf /backup/pgdata-$(date +%F).tar.gz -C /data .

# Restore
docker run --rm -v pgdata:/data -v "$PWD":/backup alpine \
  sh -c 'rm -rf /data/* && tar xzf /backup/pgdata-2025-08-09.tar.gz -C /data'
```

For a database, prefer the database's own backup tool (`pg_dump`/pgBackRest — see the [Postgres runbook](../postgres/runbook.md)); a filesystem snapshot of a running database is only safe if the engine supports it.

### 7.3 Updates and rollback

```sh
# Compose: recreate with the new image
docker compose pull && docker compose up -d
# Rollback: pin the previous digest and re-up
```

Compose has **no rolling update** — `up -d` stops and recreates, so there is a service gap. If that gap is unacceptable, you need an orchestrator, not a Compose flag.

### 7.4 Monitoring

| Signal | Source | Alert when |
| --- | --- | --- |
| **Host disk free** | OS + `docker system df` | < 20% |
| Container restart count | `docker inspect .RestartCount` | Increasing |
| **OOM kills** | `docker inspect .State.OOMKilled` | Any |
| Memory vs limit | `docker stats` / cAdvisor | > 90% sustained |
| Health check status | `.State.Health.Status` | `unhealthy` |
| Image age / CVEs | `trivy` in CI + scheduled rescan | Critical findings |
| Log volume growth | Disk | Unbounded growth ⇒ log-opts missing |

## Part 8 — Dev → production checklist

**Image**
- [ ] Multi-stage; runtime image contains no compiler or package manager
- [ ] Base image pinned by **digest**
- [ ] Non-root `USER`; verified with `docker run --rm app id`
- [ ] Exec-form `ENTRYPOINT`/`CMD`
- [ ] `.dockerignore` excludes `target/`, `.git`, `.env*`
- [ ] No secrets in layers — verified with `docker history --no-trunc`
- [ ] Scanned in CI; critical CVEs gate the build

**Runtime**
- [ ] `--read-only` + explicit `tmpfs`
- [ ] `--cap-drop=ALL`, minimal `--cap-add`, `no-new-privileges`
- [ ] Memory, CPU, and PID limits set
- [ ] No Docker socket mounted; not `--privileged`
- [ ] `SIGTERM` handled and drain tested (`time docker stop` ≪ 10 s)
- [ ] Liveness and readiness endpoints; `start_period` tuned

**Host**
- [ ] Log rotation configured in `daemon.json`
- [ ] Disk monitored; prune scheduled (**never** `-a --volumes` on a stateful host)
- [ ] `live-restore: true`
- [ ] Rootless or userns-remap enabled
- [ ] Volume backups taken **and restored once** as a drill

**Deployment**
- [ ] Deployed by digest, not tag
- [ ] Rollback = previous digest, tested
- [ ] Compose used only where the availability gap is acceptable and documented

## Common mistakes → what actually happens

| Mistake | Consequence |
| --- | --- |
| `COPY . .` before dependency install | Every source edit rebuilds all dependencies |
| Secret `COPY`d then `rm`d | Still in the layer; distributed to everyone who pulls |
| Shell-form `CMD` | `SIGTERM` ignored; 10 s stall then `SIGKILL`; requests dropped |
| No `USER` | RCE runs as root; with a socket mount, host root |
| Mounted `/var/run/docker.sock` | Container has full control of the host |
| No resource limits | One container OOMs or fork-bombs the host |
| No log rotation | Disk fills; daemon and all containers die |
| `docker system prune -a --volumes` | **Database data deleted** |
| Bare `depends_on` | App crash-loops racing the database |
| `:latest` in production | Unreproducible; silent version drift |
| Bind-mounting `target/` on macOS | Builds crawl across the VM boundary |
| Compose treated as HA | Host dies, service stays down |

## References

- [Docker docs](https://docs.docker.com/) · [Dockerfile best practices](https://docs.docker.com/build/building/best-practices/) · [Compose file reference](https://docs.docker.com/reference/compose-file/)
- [BuildKit secrets & cache mounts](https://docs.docker.com/build/building/secrets/)
- Related in this repo: [learning.md](learning.md), [reference.md](reference.md), [Postgres runbook](../postgres/runbook.md) (the stateful service this usually runs), [Rust releasing](../../language-best-practices/rust/releasing.md) (build profiles, `--locked`, digests), [OpenBao](../openbao/learning.md) (runtime secret injection)
