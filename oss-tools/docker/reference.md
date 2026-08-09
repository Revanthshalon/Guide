# Docker & Docker Compose — Quick Reference

## Quick Facts

- **Alternative to:** VMs (for packaging/isolation); Podman/buildah (daemonless equivalents)
- **License:** Docker Engine Apache 2.0 · **Docker Desktop requires a paid subscription for larger companies** — the one licensing trap
- **Backed by:** Docker Inc.; runtime (containerd, runc) and specs (OCI) are CNCF/vendor-neutral, so images are portable

## The Model in Four Facts

| Fact | Consequence |
| --- | --- |
| Layers are **immutable and additive** | `RUN rm secret` does **not** remove it from the image |
| Build cache is **positional** | Instruction order *is* build performance |
| Container shares the **host kernel** | Not a security boundary against untrusted code |
| PID 1 has **special signal semantics** | No `SIGTERM` handler ⇒ signals ignored ⇒ `SIGKILL` after 10 s |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Secrets in layers | `RUN --mount=type=secret` (BuildKit) | Needs `# syntax=docker/dockerfile:1` |
| `docker stop` hangs 10 s | **Exec form** `CMD ["./app"]` + `SIGTERM` handler | Shell form runs `sh` as PID 1 |
| Zombie processes | `--init` / `init: true` | Only needed if you spawn children |
| Running as root | `USER`, `--cap-drop=ALL`, `--read-only` | Ports < 1024; volume UID mismatch |
| 1.5 GB images | **Multi-stage** + slim/distroless base | alpine = musl (slower allocator for Rust) |
| Cache always misses | Copy manifests before source | `COPY . .` early kills everything after |
| Compose in production | Real orchestrator, or document the gap | `depends_on` waits for *start*, not readiness |
| Data lost on restart | Named volume or bind mount | Writable layer dies with the container |
| `:latest` tag | Pin by **digest** | Reproducibility, and supply-chain |

## Dockerfile — the shape that caches

```dockerfile
# syntax=docker/dockerfile:1
FROM rust:1.83-slim AS builder
WORKDIR /app
# 1. Dependency manifests FIRST — this layer survives source edits
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs && cargo build --release && rm -rf src
# 2. Now the source
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    touch src/main.rs && cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/target/release/app /app
USER nonroot:nonroot
ENTRYPOINT ["/app"]          # exec form — your process is PID 1
```

Order: base → system packages → **dependency manifests** → dependency build → source → app build.

## Compose — the parts that matter

```yaml
services:
  api:
    build: .
    init: true                       # tini as PID 1: signal forwarding + zombie reaping
    restart: unless-stopped
    environment:
      DATABASE_URL: postgres://app:${DB_PASSWORD}@db:5432/app
    depends_on:
      db:
        condition: service_healthy   # NOT bare `depends_on` — that waits for start only
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 10s
      timeout: 3s
      retries: 3
      start_period: 20s
    deploy:
      resources:
        limits: { cpus: "2.0", memory: 1G }

  db:
    image: postgres:17
    volumes: [pgdata:/var/lib/postgresql/data]   # named volume — NOT the writable layer
    environment:
      POSTGRES_PASSWORD: ${DB_PASSWORD:?required}
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      retries: 10

volumes:
  pgdata:
```

## Common Commands

```sh
# Build / run
docker build -t app:dev .
docker build --progress=plain --no-cache .          # debug a failing build
docker run --rm -it --init -p 8080:8080 app:dev
docker run --rm -it --entrypoint sh app:dev         # poke around an image

# Inspect
docker ps -a                     docker logs -f --tail 100 <c>
docker exec -it <c> sh           docker inspect <c> | jq '.[0].State'
docker stats                     docker top <c>
docker history --no-trunc app:dev        # ← where the layer bloat is
docker image ls --format '{{.Size}}\t{{.Repository}}:{{.Tag}}' | sort -h

# Compose
docker compose up -d --build     docker compose logs -f api
docker compose ps                docker compose exec api sh
docker compose down              docker compose down -v      # -v DELETES volumes
docker compose config            # render the merged, variable-substituted file
docker compose --profile debug up            # optional services

# Cleanup (disk fills silently)
docker system df                 # what's using space
docker system prune              # dangling images, stopped containers, unused networks
docker system prune -a --volumes # EVERYTHING unused — including named volumes
docker builder prune             # build cache only

# Copy in/out
docker cp <c>:/app/out.json ./   docker cp ./cfg.toml <c>:/etc/app/
```

## Security Flags

```sh
docker run \
  --user 1000:1000 \
  --read-only --tmpfs /tmp \
  --cap-drop=ALL --cap-add=NET_BIND_SERVICE \
  --security-opt=no-new-privileges \
  --pids-limit=200 --memory=1g --cpus=2 \
  app:prod
```

**Never**: `--privileged`, `-v /var/run/docker.sock:/var/run/docker.sock` (both are root on the host).

## Migration Checklist

- [ ] Config from environment/mounted files — one image, all environments
- [ ] All writes go to volumes; nothing persists in the writable layer
- [ ] Logs to `stdout`/`stderr`, never files
- [ ] `/health` (liveness) and `/ready` (readiness) endpoints
- [ ] `SIGTERM` handled: drain and exit inside the grace period
- [ ] Multi-stage build; runtime image has no compiler
- [ ] Non-root `USER`; base image pinned by digest
- [ ] `.dockerignore` excludes `target/`, `.git`, `node_modules`
- [ ] Image scanned (`trivy`, `docker scout`)
- [ ] Resource limits set — an unlimited container can take the host down

## Key References

- [Dockerfile best practices](https://docs.docker.com/build/building/best-practices/)
- [Compose file reference](https://docs.docker.com/reference/compose-file/)
- [`cargo-chef`](https://github.com/LukeMathWalker/cargo-chef) — Rust dependency-layer caching
