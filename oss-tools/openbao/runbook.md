# OpenBao — Setup & Operations Runbook

> **Accuracy note:** OpenBao moves fast. Config-stanza names, flags, and feature availability below reflect the Vault-1.14-lineage API as of early 2026 — verify against [openbao.org/docs](https://openbao.org/docs/) before running any of this in production. Commands are `bao`; the equivalent `vault` commands are identical in shape.
>
> Concepts are explained in [learning.md](learning.md); this document is the *procedure*. If a step here doesn't make sense, the why is there.

---

## Part 1 — Development setup

### 1.1 Dev mode (exploration only)

```sh
bao server -dev -dev-root-token-id=root
# in another shell:
export BAO_ADDR='http://127.0.0.1:8200'
export BAO_TOKEN='root'
bao status
```

**What dev mode silently does — every item is a reason it is not production:**

| Dev mode behavior | Production reality |
| --- | --- |
| In-memory storage — all data lost on exit | Raft on disk, snapshotted offsite |
| Auto-unsealed, no unseal keys generated | Sealed on every start; needs quorum or a KMS |
| Root token printed to your terminal | Root token revoked after setup |
| Listens on plain HTTP | TLS everywhere, including cluster traffic |
| Single node | 3 or 5 nodes across failure domains |
| KV v2 pre-mounted at `secret/` | Every engine mounted deliberately |
| No audit device | Audit enabled *before* any secret is written |

### 1.2 Persistent local dev (closer to real)

When you need restarts to preserve state and want to *feel* the seal/unseal cycle, run a real single-node config locally:

```hcl
# dev-local.hcl
storage "raft" {
  path    = "./bao-data"
  node_id = "local-1"
}
listener "tcp" {
  address     = "127.0.0.1:8200"
  tls_disable = true          # local only — never in production
}
api_addr     = "http://127.0.0.1:8200"
cluster_addr = "http://127.0.0.1:8201"
disable_mlock = true
ui = true
```

```sh
mkdir -p ./bao-data
bao server -config=dev-local.hcl
# then follow Part 3's init ceremony — this node behaves like production
```

### 1.3 First engines and a secret

```sh
bao secrets enable -path=secret kv-v2
bao kv put secret/myapp/db username=app password=s3cret
bao kv get secret/myapp/db
bao kv get -field=password secret/myapp/db      # scriptable single value

bao secrets enable transit
bao write -f transit/keys/app-kek
```

### 1.4 Reading a secret from Rust

No official OpenBao Rust SDK is needed — the HTTP API is small. A minimal typed client:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct KvResponse { data: KvData }
#[derive(Deserialize)]
struct KvData { data: serde_json::Value }   // KV v2 nests: data.data

async fn read_kv(addr: &str, token: &str, path: &str) -> anyhow::Result<serde_json::Value> {
    let url = format!("{addr}/v1/secret/data/{path}");   // note: /data/ for KV v2
    let resp = reqwest::Client::new()
        .get(url)
        .header("X-Vault-Token", token)     // OpenBao keeps the Vault header name
        .send().await?
        .error_for_status()?
        .json::<KvResponse>().await?;
    Ok(resp.data.data)
}
```

Two gotchas this encodes: **KV v2 paths insert `/data/`** (`secret/myapp/db` reads at `/v1/secret/data/myapp/db`) — the single most common first-API-call mistake — and the header is still `X-Vault-Token` for compatibility. Wrap the token in a [`secrecy::SecretString`](../../language-best-practices/rust/learning.md) so it can't land in a log line.

---

## Part 2 — Production installation

### 2.1 Decide the topology first

| Decision | Guidance |
| --- | --- |
| Node count | **3** (tolerates 1 failure) or **5** (tolerates 2, survives failure during maintenance). Never even numbers — see [consensus](../../architecture-patterns/consensus-and-leader-election/learning.md) |
| Placement | No single failure domain holds a majority. 5 nodes across 3 zones = 2/2/1 |
| Unseal | Auto-unseal (cloud KMS) unless a human-quorum ceremony is a deliberate requirement |
| Storage | Integrated Raft on **local NVMe** — Raft fsyncs every commit; network/burstable disks cause election storms |
| Exposure | API behind a load balancer with health checks on `/v1/sys/health`; cluster port never public |

### 2.2 Host prerequisites

```sh
# Dedicated unprivileged user, no shell
useradd --system --home /etc/openbao --shell /bin/false openbao

# Directories
install -d -o openbao -g openbao -m 0750 /etc/openbao /opt/openbao/data /var/log/openbao

# File descriptor limit — Raft + many clients exhaust the default
# (in the systemd unit: LimitNOFILE=65536)

# Swap OFF on the host: secrets in memory must never reach disk
swapoff -a    # and remove from /etc/fstab
```

### 2.3 The production config file

```hcl
# /etc/openbao/config.hcl  — annotated; every line matters

# ── Storage: integrated Raft ────────────────────────────────────────────────
storage "raft" {
  path    = "/opt/openbao/data"
  node_id = "bao-1"                     # UNIQUE per node — duplicates corrupt the cluster

  # Every node lists the OTHER nodes. Nodes retry until the cluster forms,
  # so start order doesn't matter.
  retry_join { leader_api_addr = "https://bao-2.internal:8200" }
  retry_join { leader_api_addr = "https://bao-3.internal:8200" }
}

# ── Listener: API + UI ──────────────────────────────────────────────────────
listener "tcp" {
  address       = "0.0.0.0:8200"
  tls_cert_file = "/etc/openbao/tls/server.crt"
  tls_key_file  = "/etc/openbao/tls/server.key"
  tls_min_version = "tls12"

  # Cluster traffic (port 8201) — Raft replication between nodes
  tls_client_ca_file = "/etc/openbao/tls/ca.crt"
}

# ── Addresses: REQUIRED for HA. Wrong values = cluster won't form ───────────
api_addr     = "https://bao-1.internal:8200"   # how CLIENTS reach THIS node
cluster_addr = "https://bao-1.internal:8201"   # how PEERS reach THIS node

# ── Auto-unseal (omit this stanza to use Shamir human-quorum unseal) ────────
seal "awskms" {
  region     = "eu-west-1"
  kms_key_id = "arn:aws:kms:eu-west-1:123456789012:key/abcd-..."
  # Credentials come from the instance role — never static keys in this file
}

# ── Operational ─────────────────────────────────────────────────────────────
disable_mlock = true      # recommended WITH integrated storage; requires swap off
ui            = true
log_level     = "info"
log_format    = "json"

telemetry {
  prometheus_retention_time = "24h"
  disable_hostname          = true
}
```

**The four settings that most often break a first deployment:**

1. `node_id` duplicated across nodes — the cluster forms wrong and misbehaves subtly.
2. `api_addr`/`cluster_addr` set to `127.0.0.1` or a name peers can't resolve — nodes never join.
3. `disable_mlock = true` **without disabling swap** — memory containing secrets can page to disk. Do both or neither.
4. TLS omitted "temporarily" — tokens then travel in plaintext; temporary becomes permanent.

### 2.4 systemd unit

```ini
# /etc/systemd/system/openbao.service
[Unit]
Description=OpenBao
After=network-online.target
Wants=network-online.target

[Service]
User=openbao
Group=openbao
ExecStart=/usr/local/bin/bao server -config=/etc/openbao/config.hcl
ExecReload=/bin/kill -HUP $MAINPID
KillSignal=SIGINT
LimitNOFILE=65536
LimitMEMLOCK=infinity
Restart=on-failure
RestartSec=5
# Hardening
ProtectSystem=full
ProtectHome=read-only
PrivateTmp=yes
NoNewPrivileges=yes

[Install]
WantedBy=multi-user.target
```

```sh
systemctl daemon-reload && systemctl enable --now openbao
bao status     # expect: Initialized false, Sealed true  ← correct at this stage
```

---

## Part 3 — Initialization ceremony (happens exactly once, ever)

> Run this on **one node only**. The other nodes join and inherit the initialized state. Re-running `init` on an initialized cluster fails; running it on a *second* fresh cluster by mistake creates a split you'll have to destroy.

### 3.1a With auto-unseal (recommended)

```sh
export BAO_ADDR='https://bao-1.internal:8200'
bao operator init -recovery-shares=5 -recovery-threshold=3
```

Output contains **5 recovery keys** and an **initial root token**. With auto-unseal the KMS unseals automatically on restart; recovery keys are *not* for unsealing — they authorize `generate-root` and `rekey`, i.e. break-glass.

### 3.1b With Shamir (no KMS)

```sh
bao operator init -key-shares=5 -key-threshold=3
# → 5 unseal keys + root token
bao operator unseal   # run 3 times, each with a different key, on EVERY node
```

### 3.2 Handling the keys — the rules that prevent the worst outcomes

- **Distribute to different humans.** Five keys in one password manager is a one-share system wearing a costume.
- **Never store keys and the root token together.** Together they are total, unlogged compromise.
- **Never on the OpenBao hosts.** Offline, encrypted (GPG per holder), and geographically apart from each other.
- **Record who holds which** — a quorum you can't assemble is a cluster you can't recover.
- **Rehearse assembling quorum once**, before you need it at 3 a.m.

### 3.3 Verify the cluster formed

```sh
bao status                       # Initialized true, Sealed false, HA Mode active|standby
bao operator raft list-peers     # expect all nodes, exactly one "leader"
```

If a peer is missing: check `cluster_addr` reachability on 8201, `node_id` uniqueness, and TLS trust between nodes.

---

## Part 4 — Day-1 hardening (before any real secret is written)

**Order matters.** Audit first, so every subsequent action is recorded.

```sh
# 1. AUDIT FIRST — no unlogged setup actions
bao audit enable file file_path=/var/log/openbao/audit.log
#    Warning: if ALL audit devices fail to write, OpenBao BLOCKS every request.
#    Monitor that disk and configure logrotate with copytruncate.

# 2. An admin policy (not root) for humans
cat > admin.hcl <<'EOF'
path "sys/health"                { capabilities = ["read", "sudo"] }
path "sys/policies/acl/*"        { capabilities = ["create","read","update","delete","list"] }
path "sys/auth"                  { capabilities = ["read","list"] }
path "sys/auth/*"                { capabilities = ["create","read","update","delete","sudo"] }
path "sys/mounts"                { capabilities = ["read","list"] }
path "sys/mounts/*"              { capabilities = ["create","read","update","delete","sudo"] }
path "auth/token/create"         { capabilities = ["create","update"] }
path "secret/*"                  { capabilities = ["create","read","update","delete","list"] }
EOF
bao policy write admin admin.hcl

# 3. A human auth method (OIDC preferred; userpass shown for brevity)
bao auth enable oidc
# ... configure oidc_discovery_url, client id/secret, and a role mapping to `admin`

# 4. VERIFY you can log in as admin and do admin things — before the next step
bao login -method=oidc
bao policy list

# 5. NOW revoke the root token
bao token revoke <initial-root-token>
#    Break-glass afterwards: `bao operator generate-root` (needs recovery/unseal quorum)
```

Step 5 is the one people skip. A standing root token is an unexpiring, policy-bypassing credential whose use is indistinguishable from any other root action.

---

## Part 5 — Writing policies correctly

### 5.1 Capabilities

| Capability | HTTP | Note |
| --- | --- | --- |
| `create` | POST/PUT (new) | |
| `read` | GET | |
| `update` | POST/PUT (existing) | **Most "write" operations need `update`, not `create`** — including transit encrypt/decrypt |
| `patch` | PATCH | KV v2 partial update |
| `delete` | DELETE | |
| `list` | LIST | Listing keys is *separate* from reading them |
| `sudo` | — | Required for root-protected paths (e.g. `sys/seal`) |
| `deny` | — | **Overrides everything else, always** |

### 5.2 Path matching

```hcl
path "secret/data/app/*"      { }   # `*` — wildcard, only valid as the LAST character
path "secret/data/+/config"   { }   # `+` — exactly one path segment
path "secret/data/app/db"     { }   # exact match wins over globs
```

**Deny always wins**, regardless of specificity — an accidental broad `deny` silently disables narrower grants.

### 5.3 A least-privilege service policy

```hcl
# orders-svc.hcl — read one secret, use one transit key. Nothing else.
path "secret/data/orders/db" {
  capabilities = ["read"]
}

# Transit: encrypt/decrypt are UPDATE operations, and the key itself is never readable
path "transit/encrypt/pii-kek" { capabilities = ["update"] }
path "transit/decrypt/pii-kek" { capabilities = ["update"] }

# Token self-management so the app can renew its own lease
path "auth/token/renew-self"  { capabilities = ["update"] }
path "auth/token/lookup-self" { capabilities = ["read"] }
```

Note what's absent: no `list` (the service doesn't enumerate secrets), no `read` on `transit/keys/*` (the KEK can be *used* but never *exported* — the property the [envelope-encryption design](../../architecture-patterns/encryption-and-key-management/learning.md) depends on).

### 5.4 Test policies before trusting them

```sh
bao token create -policy=orders-svc -ttl=5m         # → a test token
BAO_TOKEN=<test> bao kv get secret/orders/db        # should succeed
BAO_TOKEN=<test> bao kv get secret/other/db         # should be denied
bao token capabilities <test-token> secret/data/orders/db   # → ["read"]
```

Policy review means running these three commands, not reading the HCL.

---

## Part 6 — Secure introduction (the step most often botched)

**The problem:** a service must authenticate to OpenBao *without* a long-lived credential in its config — otherwise that credential is just the secret you were trying to protect, one level up.

### 6.1 Kubernetes (best answer when available)

The platform already attests workload identity; OpenBao verifies the attestation.

```sh
bao auth enable kubernetes

bao write auth/kubernetes/config \
    kubernetes_host="https://kubernetes.default.svc:443" \
    kubernetes_ca_cert=@/var/run/secrets/kubernetes.io/serviceaccount/ca.crt

bao write auth/kubernetes/role/orders \
    bound_service_account_names=orders-svc \
    bound_service_account_namespaces=production \
    policies=orders-svc \
    ttl=1h
```

The pod logs in with its projected service-account token — no secret is ever distributed:

```sh
JWT=$(cat /var/run/secrets/kubernetes.io/serviceaccount/token)
bao write auth/kubernetes/login role=orders jwt="$JWT"
```

### 6.2 AppRole (when there's no platform identity)

AppRole needs `role_id` (non-secret, bake into config) + `secret_id` (secret, delivered at runtime). Delivering the secret_id in plaintext recreates the problem — so use **response wrapping**:

```sh
bao auth enable approle
bao write auth/approle/role/orders \
    token_policies=orders-svc token_ttl=1h token_max_ttl=4h \
    secret_id_ttl=10m secret_id_num_uses=1        # single-use, short-lived

bao read auth/approle/role/orders/role-id          # → role_id, safe in config

# A trusted orchestrator generates a WRAPPED secret_id and hands the app the wrapping token:
bao write -wrap-ttl=60s -f auth/approle/role/orders/secret-id
# → wrapping_token. The app unwraps it exactly once:
bao unwrap <wrapping_token>                        # → the real secret_id
```

Response wrapping gives **tamper evidence**: a wrapping token can be unwrapped only once, so if the app's unwrap fails, someone else got there first — an alertable event, not a silent interception.

### 6.3 What not to do

- A static `BAO_TOKEN` in environment variables, CI config, or a `.env` file — an unexpiring credential in the place you were trying to remove credentials from.
- A shared token across services — destroys per-service policy and audit attribution.
- Long `secret_id_ttl` with unlimited uses — an AppRole secret_id that never expires is a password.

---

## Part 7 — Application integration

### 7.1 Two patterns

| Pattern | How | Use when |
| --- | --- | --- |
| **Direct API** | App authenticates, holds the token, renews it, calls the API | You want explicit control (Rust services, transit-heavy paths) |
| **Bao Agent sidecar** | Agent authenticates and renders secrets to a file/env; app just reads a file | The app can't be modified, or many languages share one pattern |

### 7.2 Direct: token lifecycle in Rust

```rust
// Sketch — the three things a real client must do:
// 1. Authenticate via the platform identity (K8s JWT / AppRole unwrap).
// 2. Renew the token before ~2/3 of its TTL elapses, in a background task.
// 3. Re-authenticate from scratch if renewal fails (token expired or revoked).

async fn renew_loop(client: BaoClient, token: SecretString, ttl: Duration) {
    let mut tick = tokio::time::interval(ttl.mul_f32(0.66));
    loop {
        tick.tick().await;
        if client.renew_self(&token).await.is_err() {
            // token gone — full re-auth, don't just retry renewal
            if let Err(e) = client.reauthenticate().await {
                tracing::error!(?e, "bao re-auth failed");   // never log the token itself
            }
        }
    }
}
```

For transit-based envelope encryption, cache unwrapped DEKs in memory with a bounded TTL and `zeroize` on eviction — the [encryption doc](../../architecture-patterns/encryption-and-key-management/learning.md) explains why this is what keeps a KMS outage from becoming an application outage.

### 7.3 Agent: file-rendered secrets

```hcl
# bao-agent.hcl
auto_auth {
  method "kubernetes" { mount_path = "auth/kubernetes"
                        config = { role = "orders" } }
  sink "file" { config = { path = "/run/bao/token" } }
}
template {
  contents    = "{{ with secret \"secret/data/orders/db\" }}DB_PASSWORD={{ .Data.data.password }}{{ end }}"
  destination = "/run/secrets/db.env"
  command     = "systemctl reload orders-svc"    # or signal the app to re-read
}
```

The agent handles auth, renewal, and re-rendering on rotation — at the cost of secrets existing on a filesystem (use `tmpfs`, never a persistent volume).

---

## Part 8 — Day-2 operations

### 8.1 Backups (the one that ends companies if skipped)

```sh
# Automated snapshot — run on a schedule from a host with a snapshot-only policy
bao operator raft snapshot save /tmp/bao-$(date -u +%Y%m%dT%H%M%SZ).snap
# Ship offsite, encrypted. Snapshots contain ALL your secrets, encrypted with the
# root key — which means: a snapshot is useless without the unseal/KMS key, and
# equally, losing that key makes every snapshot unrecoverable.
```

Restore drill (do it quarterly — an untested backup is a hope):

```sh
bao operator raft snapshot restore /path/to/bao.snap
# The restored cluster keeps the ORIGINAL root key — you need the same
# KMS key / unseal shares that were in use when the snapshot was taken.
```

Snapshot policy for the backup agent (least privilege):

```hcl
path "sys/storage/raft/snapshot" { capabilities = ["read"] }
```

### 8.2 Upgrades (order matters — getting it wrong causes an outage)

```sh
# 1. Snapshot first, always.
# 2. Upgrade STANDBY nodes one at a time:
systemctl stop openbao && <install new binary> && systemctl start openbao
bao status                      # wait for Sealed=false and HA Mode=standby
bao operator raft list-peers    # confirm it rejoined before touching the next node

# 3. LAST: the leader. Force a clean handover instead of killing it:
bao operator step-down          # leadership moves to an upgraded standby
# then upgrade the old leader as a standby
```

Never upgrade the leader first, and never upgrade two nodes at once in a 3-node cluster — you lose quorum and the cluster stops serving.

### 8.3 Rotation procedures

```sh
bao operator rekey -init -key-shares=5 -key-threshold=3   # change unseal shares (needs current quorum)
bao operator generate-root -init                          # break-glass root (needs quorum)
bao write -f transit/keys/pii-kek/rotate                  # new KEK version; old still decrypts
bao write sys/rotate                                      # rotate the underlying encryption key
```

Transit rotation is only half-done until every ciphertext is re-wrapped and `min_decryption_version` is raised — see the [encryption doc's](../../architecture-patterns/encryption-and-key-management/learning.md) rotation-theater pitfall.

### 8.4 Monitoring — the signals that matter

| Signal | Source | Alert when |
| --- | --- | --- |
| Seal status | `/v1/sys/health` | Any node sealed unexpectedly |
| Leader / quorum | `bao operator raft list-peers`, `bao.raft.leader` metrics | No leader, or peers < quorum |
| Audit disk free | Host metric | < 20% — **audit failure blocks all requests** |
| Lease count | `bao.expire.num_leases` | Sustained growth (lease explosion) |
| Token/secret TTL churn | `bao.token.creation` rate | Spikes = re-auth storms (crash loops) |
| Raft fsync latency | `bao.raft.fsm.apply`, disk p99 | Rising — election storms follow |
| 5xx / rate-limit responses | LB + audit log | Any sustained rate |

Health-check endpoints for load balancers: `/v1/sys/health` returns **200** active, **429** standby, **501** uninitialized, **503** sealed — configure the LB to route only to 200 unless you deliberately want standby reads.

---

## Part 9 — Dev → production checklist

Print this. Every unchecked line is a known way to get hurt.

**Before first secret is written**
- [ ] 3 or 5 nodes, no failure domain holds a majority
- [ ] TLS on API (8200) **and** cluster (8201); certs have a renewal owner
- [ ] `node_id` unique per node; `api_addr`/`cluster_addr` resolvable by peers
- [ ] Swap disabled on hosts; `disable_mlock` set consistently with that
- [ ] `LimitNOFILE=65536`; data dir on local NVMe
- [ ] Unseal decided: auto-unseal KMS (with its own key policy/backup) or Shamir with named holders
- [ ] `bao operator init` run **once**; keys distributed to different people, offline, separate from the root token
- [ ] Quorum assembly rehearsed at least once
- [ ] **Audit device enabled before anything else**; logrotate configured; disk alerted
- [ ] Admin policy + human auth method (OIDC) working and verified
- [ ] **Initial root token revoked**; `generate-root` break-glass documented and rehearsed

**Before applications connect**
- [ ] Per-service least-privilege policies, verified with `bao token capabilities`
- [ ] Secure introduction via platform identity (K8s) or response-wrapped AppRole — **no static tokens anywhere**
- [ ] Token TTLs sized to process lifetime (15 m–1 h) with renewal implemented
- [ ] Dynamic secrets used for databases/cloud where available (not KV'd static passwords)
- [ ] Transit keys non-exportable; app policies grant `update` on encrypt/decrypt only

**Before you can call it production**
- [ ] Automated offsite snapshots; **restore tested end to end**
- [ ] Upgrade procedure documented (standbys → step-down → old leader)
- [ ] Monitoring + alerts on the Part 8.4 table
- [ ] Documented answer to "what breaks when OpenBao is down?" and client-side grace (cached DEKs, agent-held tokens)
- [ ] Incident runbooks: sealed cluster, lost quorum, suspected token compromise (`bao lease revoke -prefix`)

---

## Common mistakes → what actually happens

| Mistake | Consequence |
| --- | --- |
| Forgot `/data/` in a KV v2 path | 404 on your first API call; the classic first hour |
| Root token kept "for emergencies" | Unexpiring policy-bypassing credential, indistinguishable in audit |
| Unseal keys in one password manager | k-of-n is theatre; one compromise unseals everything |
| Init run on a second node by mistake | Two separate clusters; one must be destroyed |
| `disable_mlock=true` with swap enabled | Secrets can page to disk in plaintext |
| Audit disk full | **OpenBao blocks every request** — a total outage from a log file |
| Upgraded the leader first | Avoidable failover mid-upgrade; with 2 nodes down, quorum loss |
| No snapshot restore test | Discovering the backup is unusable during the incident |
| Static `BAO_TOKEN` in env/CI | The credential problem you adopted OpenBao to solve |
| Long-lived `secret_id`, unlimited uses | AppRole degenerates into a password |
| Transit key rotated, never re-wrapped | Rotation theatre — old key still required forever |
| KMS key for auto-unseal deleted/lost | Cluster unrecoverable; snapshots unrecoverable too |

---

## References

- [OpenBao documentation](https://openbao.org/docs/) — the authority; verify version-specific details here.
- [OpenBao configuration reference](https://openbao.org/docs/configuration/) — every stanza in Part 2.3.
- HashiCorp Vault production-hardening guide — still the best checklist for the shared lineage; translate `vault` → `bao`.
- [learning.md](learning.md) — the concepts behind every procedure here; [reference.md](reference.md) — the command cheat sheet.
- [Encryption & Key Management](../../architecture-patterns/encryption-and-key-management/learning.md) — why transit is shaped this way; [Consensus & Leader Election](../../architecture-patterns/consensus-and-leader-election/learning.md) — why the node counts and upgrade order are what they are.
