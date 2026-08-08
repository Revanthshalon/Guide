# OpenBao — Learning Notes

> Accuracy note: OpenBao moves fast and some specifics below (feature landings, storage-backend support, version numbers) are as of early 2026 — verify against [openbao.org](https://openbao.org/) before relying on them operationally.

## What It Is & Why It Exists

OpenBao is an open-source secrets-management and encryption-as-a-service server: it stores and access-controls secrets, mints short-lived dynamic credentials, runs PKI, and performs encrypt/decrypt/rewrap operations (the transit engine) so applications never hold long-lived key material. It is the [KMS role](../../architecture-patterns/encryption-and-key-management/learning.md) you can self-host.

The lineage is the point: in August 2023, HashiCorp relicensed Vault (and Terraform, Consul, etc.) from the open-source MPL 2.0 to the source-available **BUSL** — free to use for most, but with field-of-use restrictions that made downstream products, distros, and competitors legally uncomfortable. OpenBao is the community **fork of the last MPL-licensed Vault** (the 1.14 line), adopted by the **Linux Foundation** (under LF Edge originally), keeping the MPL 2.0 license and open governance. The same fork-on-relicense event produced OpenTofu from Terraform — same story, same foundation-backed resolution.

Consequences of being a fork worth internalizing:

- **The mental model is Vault's.** Concepts, API shape, CLI ergonomics (`bao` mirrors `vault`), policy language, auth methods — Vault knowledge transfers almost entirely, as does most community material. The API keeps Vault's `/v1/...` paths and even the `X-Vault-Token` header conventions for compatibility.
- **Divergence is real and growing.** OpenBao trimmed the supported surface at fork time (notably: integrated Raft storage as *the* storage story, dropping the long tail of Vault's storage backends and some plugins), then started adding its own features — some that Vault kept enterprise-only (namespaces landed in OpenBao's open version), plus its own work (e.g. inline/transactional storage improvements, UI work). Post-fork Vault features (1.15+) do not automatically exist in OpenBao, and vice versa.
- **Governance is the differentiator, not features.** You choose OpenBao to not depend on a single vendor's licensing decisions; feature parity with current Vault Enterprise is not the promise and shouldn't be the expectation.

## Architecture & Core Concepts

### Seal / unseal and the security barrier

- **What it is:** Everything OpenBao persists is encrypted under a keyring protected by a **root key**, which is *not stored* — at rest the server is **sealed** (data is ciphertext, the process can serve nothing). Unsealing reconstructs the root key: from a quorum of **Shamir key shares** (k-of-n, held by humans) or via **auto-unseal** (root key wrapped by a cloud KMS or another OpenBao's transit engine).
- **Why it matters operationally:** Every restart begins sealed. Shamir means a human quorum at 3 a.m.; auto-unseal means a hard dependency on the KMS that wraps your root. This is the availability/trust dial from the [encryption notes](../../architecture-patterns/encryption-and-key-management/learning.md), and it is the first decision a deployment makes.

### Storage backend (integrated Raft)

- **What it is:** OpenBao's supported storage story is **integrated storage**: a Raft consensus cluster among the OpenBao nodes themselves (the same design Vault converged on) — plus Postgres as a supported alternative for single-node-style deployments. The Raft cluster gives HA (leader + standbys), snapshots for backup, and no external storage dependency.
- **Why it matters operationally:** Your secrets infrastructure is a [consensus system](../../architecture-patterns/consensus-and-leader-election/learning.md): quorum math applies (3 or 5 nodes), disk latency on the leader gates write throughput, snapshots are your disaster story, and losing quorum means a sealed-equivalent outage. All the leader-election and split-brain material applies directly.

### Auth methods and identity

- **What it is:** How clients prove who they are before getting a token: Kubernetes service accounts, cloud instance identity (AWS/GCP/Azure), OIDC/JWT, TLS certs, AppRole (machine-to-machine role id + secret id), userpass/LDAP for humans. Every method resolves to an **entity** with policies attached; tokens carry TTLs and are renewed or expire.
- **Why it matters operationally:** This is the **secure introduction** problem — the service must prove its identity *without* a pre-shared long-lived secret, or you've reinvented the password in a config file. Platform identity (K8s/cloud) is the answer: the platform already attests who the workload is; OpenBao verifies that attestation. AppRole is the fallback where no platform identity exists, and its secret-id delivery needs real design.

### Policies

- **What it is:** Path-based ACLs in HCL: `path "secret/data/orders/*" { capabilities = ["read"] }`. Deny by default; policies attach to identities via auth methods. Fine-grained parameters exist (allowed/denied parameters, response wrapping requirements).
- **Why it matters operationally:** The policy set *is* your security posture — it encodes "which workload may read/use which secret/key." Treat policies as code (reviewed, versioned, applied by pipeline), never hand-edited in production. The sharpest edges: overly-broad wildcards, and the root token (see pitfalls).

### Secret engines

- **What it is:** Mountable backends, each doing one job:
  - **KV (v2)** — versioned static secrets storage: the workhorse for config secrets, with soft-delete/undelete and version history.
  - **Transit** — encryption as a service: named keys that never leave the server; encrypt/decrypt/rewrap/sign/verify/HMAC over the API; key versioning with `min_decryption_version` as the rotation ratchet. This is the KMS role in the [envelope-encryption worked example](../../architecture-patterns/encryption-and-key-management/learning.md).
  - **Database** — dynamic credentials: OpenBao holds a privileged DB connection and mints short-TTL roles per request, revoking them at lease end.
  - **PKI** — an X.509 CA issuing short-lived certificates over an API — the practical way to run internal mTLS without certificate-request tickets.
  - Plus: TOTP, SSH certificate signing, cloud-credential engines.
- **Why it matters operationally:** The engines are why OpenBao is more than an encrypted KV store: dynamic secrets and short-lived certs *change the failure mode* — leaked credentials expire in minutes instead of living in config files for years. Each engine mount has its own policies, TTL/lease configuration, and operational surface.

### Leases, renewal, and revocation

- **What it is:** Every dynamic secret and token carries a **lease** — a TTL after which it's automatically revoked (the DB role dropped, the token dead). Clients renew leases while alive; revocation can also be forced (by lease, by mount, by prefix) during incidents.
- **Why it matters operationally:** Leases are the mechanism behind "leaked credential expires before it's abused." The operational flip side: lease *volume* is real state (see pitfalls), renewal logic must live in your clients (or an agent), and mass revocation is your breach lever — know the command before the incident.

### Audit devices

- **What it is:** Append-only logs of **every request and response** (secrets HMAC'd, not plaintext), to file/syslog/socket. Enabled explicitly — not on by default.
- **Why it matters operationally:** The audit log is half the reason to centralize secrets at all: "which identity accessed which secret when" becomes a queryable fact, which is the compliance story and the forensics story. Caveat: if *all* audit devices become unwritable, OpenBao blocks requests rather than operate unaudited — size the disk and monitor it.

## Comparison in Depth

| Aspect | HashiCorp Vault | OpenBao |
| --- | --- | --- |
| License | BUSL 1.1 (source-available, field-of-use restricted) since Aug 2023 | MPL 2.0 (true open source) |
| Governance | HashiCorp (now IBM) product decisions | Linux Foundation project, open TSC |
| Lineage | Continuous | Forked from Vault 1.14 (last MPL release) |
| Core concepts/API | The original | Same model; `/v1` API- and header-compatible for the common surface |
| CLI | `vault` | `bao` (mirrored ergonomics) |
| Storage backends | Integrated Raft + long legacy list; Enterprise adds DR/perf replication | Integrated Raft (+ Postgres); legacy backend list trimmed at fork |
| Namespaces (multi-tenancy) | Enterprise feature | In the open version (post-fork addition) |
| Enterprise-only surface (HSM auto-unseal variants, replication, Sentinel, etc.) | Paid tier | Not present as such; some equivalents arriving via community work — check per feature |
| Post-fork features (either side) | Vault 1.15+ additions absent from OpenBao unless reimplemented | OpenBao additions (namespaces, storage/txn work) absent from Vault OSS |
| Ecosystem/integrations | Deepest (every tool integrates Vault first) | Rides Vault compatibility for most integrations; first-class support growing |
| Agent/sidecar injection | Vault Agent, K8s injector, CSI | Bao agent + the same patterns; K8s tooling maturing — verify current state |

The strategic read: for the **core workflow** (KV, transit, database creds, PKI, K8s auth, policies) OpenBao is a drop-in with a clean conscience about licensing. The risk surface is at the **edges** — a specific Vault Enterprise feature, a niche plugin, a vendor integration that hardcodes Vault — each of which needs a per-item check, not an assumption in either direction.

## Hands-On Notes

Local exploration (dev mode — in-memory, auto-unsealed, root token printed; never production):

```sh
bao server -dev

export BAO_ADDR=http://127.0.0.1:8200
bao status

# KV v2
bao secrets enable -path=secret kv-v2
bao kv put secret/orders/db password=hunter2
bao kv get secret/orders/db

# Transit: the envelope-encryption KMS role
bao secrets enable transit
bao write -f transit/keys/pii-kek
bao write transit/encrypt/pii-kek plaintext=$(echo -n "32-byte-dek-here" | base64)
#   → ciphertext: bao:v1:...          (wrapped DEK — store it beside your data)
bao write transit/decrypt/pii-kek ciphertext="bao:v1:..."
bao write -f transit/keys/pii-kek/rotate                      # v2; old ciphertexts still decrypt
bao write transit/rewrap/pii-kek ciphertext="bao:v1:..."      # → bao:v2:... without exposing the DEK
bao write transit/keys/pii-kek/config min_decryption_version=2  # the ratchet

# Policy + AppRole (machine identity without platform attestation)
bao policy write orders-svc orders-svc.hcl
bao auth enable approle
bao write auth/approle/role/orders token_policies=orders-svc token_ttl=15m
```

Things to actually try when studying: kill and restart the server to *feel* sealed-by-default; run a 3-node Raft cluster in containers and kill the leader mid-write; enable the file audit device and read what a `kv get` actually logs; wire the transit calls into the [envelope-encryption worked example](../../architecture-patterns/encryption-and-key-management/learning.md) from a small Rust client.

## Pitfalls in Depth

### Pitfall: The root token that never died

- **What goes wrong:** Initialization produces a root token; it gets used for setup, then saved "for emergencies" in a password manager, a CI variable, three engineers' shell histories. It never expires, bypasses all policy, and its use is indistinguishable from any other root use in the audit log. The secrets platform's own god-credential becomes its weakest point.
- **Why it happens (the mechanism):** Setup genuinely needs root; revoking it afterward feels like locking yourself out ("what if we need it?"). The correct emergency path — regenerating a root token from unseal-key quorum — is unknown or untested, so the standing token feels safer than it is.
- **How to handle it in production, and why that works:** Root token lifecycle: use for initial setup → configure real auth methods + admin policies → **revoke it** (`bao token revoke`). Emergencies use `bao operator generate-root`, which requires the *unseal quorum* — putting root access behind the same multi-human bar as the root of trust itself, with an audit trail. Rehearse that flow once so it's boring.
- **Trade-offs of the fix:** Break-glass now takes a quorum ceremony (minutes, not seconds) — that's the intended cost. Document it in the runbook so the 3 a.m. responder isn't discovering the procedure live.

### Pitfall: Treating OpenBao as just-a-database (missing the tier-0 reality)

- **What goes wrong:** OpenBao is deployed like any stateless service: single node, no snapshot schedule, monitoring = "process is up." Then: the node dies and the *entire company's* secrets, PKI, and encryption keys are in one unrestorable Raft directory; or the cluster loses quorum during a k8s node rotation and every service that needs a credential at startup is down; or a restart leaves it sealed and nobody's paged with the unseal procedure.
- **Why it happens (the mechanism):** Secrets managers are adopted *by* application teams, and application-service operational habits come along. But OpenBao sits below everything — it is tier-0 by construction: its outage is everyone's outage, its data loss is unrecoverable by definition (that's what "the secrets" means), and its consistency is a Raft quorum with all the [consensus operational physics](../../architecture-patterns/consensus-and-leader-election/learning.md) that implies.
- **How to handle it in production, and why that works:** Run it like the database it is: 3 or 5 Raft nodes across failure domains; **scheduled Raft snapshots shipped off-cluster, restore-tested** (an untested backup of a sealed encrypted store is a hope, not a backup); auto-unseal (with its KMS dependency understood) or a rehearsed quorum-unseal page; monitoring on seal status, leadership, quorum health, audit-disk fullness; and an explicit dependency map of "what breaks when OpenBao is down" — which also motivates client-side grace (cached credentials outliving short outages: agents renew leases early, apps cache transit-unwrapped DEKs per the [encryption notes](../../architecture-patterns/encryption-and-key-management/learning.md)).
- **Trade-offs of the fix:** Real operational spend for a tool adopted partly to save money over cloud KMS/Vault Enterprise. Price it honestly against managed alternatives; "self-hosted because free" without the ops budget is how secrets get lost.

### Pitfall: Static secrets in, static habits kept

- **What goes wrong:** Migration to OpenBao = every long-lived password moved from config files into KV. Security posture barely changes: the same eternal credentials exist, just fetched from a different place — often *by* a long-lived OpenBao token sitting in the same config file the DB password used to. One layer of indirection, zero change in blast radius or rotation reality.
- **Why it happens (the mechanism):** KV migration is the easy visible win and demos well; dynamic secrets require per-system integration work (database engine config, app retry/renewal logic) that has no deadline forcing it. The static-token-to-fetch-static-secrets anti-pattern is the same mechanism recursing: nobody designed the *introduction* step.
- **How to handle it in production, and why that works:** Sequence the migration by *credential lifetime*, not by ease: platform-identity auth first (K8s/cloud attestation — kills the bootstrap token), then dynamic engines for the highest-blast-radius systems (databases, cloud creds), then short-lived PKI for internal TLS; KV remains for the genuinely-static remainder (third-party API keys — rotated on a schedule via the API). Measure the posture with one number: *what fraction of credentials expire within an hour?* That metric moving is the migration succeeding.
- **Trade-offs of the fix:** Dynamic credentials add moving parts — renewal logic, TTL tuning, revocation storms on mass restart (see next pitfall), and apps must tolerate credential refresh mid-life (connection re-establishment). This is real engineering; it's also the entire point of the tool.

### Pitfall: Lease and TTL explosions

- **What goes wrong:** Every dynamic credential and token is server-side state with an expiry timer. A fleet of pods each requesting fresh DB creds on every restart (crash loop = credential mint loop), TTLs set long "to be safe," and no renewal discipline: hundreds of thousands of live leases accumulate, revocation storms hammer the backing database when they expire together, storage bloats, and unseal/recovery time stretches with the lease table.
- **Why it happens (the mechanism):** Leases are invisible until they aren't — nothing in the happy path surfaces their count. Defaults get copied (32-day max TTLs), crash loops multiply mints silently, and batch workloads mint per-task instead of per-worker. The state lives where nobody's dashboard looks.
- **How to handle it in production, and why that works:** TTLs sized to actual process lifetimes (15m–1h tokens with renewal beats 30-day tokens); reuse over re-mint (agents/sidecars hold and renew one credential per workload, not per request); batch-job patterns that share a lease per run; monitoring on lease count by mount with alerts on growth rate; and periodic revoke-by-prefix hygiene for orphaned mounts. Know `bao lease revoke -prefix` for incident-time mass revocation *and* for cleanup.
- **Trade-offs of the fix:** Short TTLs + renewal = more moving parts in clients and more sensitivity to OpenBao availability blips (mitigated by early renewal windows). The lease system is a garbage-collected heap; you are the GC tuning.

### Pitfall: Migrating from Vault on vibes (compatibility assumed transitively)

- **What goes wrong:** "It's a fork, everything works" → the migration plan is a DNS switch. Then: a workflow depends on a Vault-1.15+ feature that postdates the fork; a niche auth plugin or storage backend was trimmed from OpenBao; a vendor product hardcodes Vault version checks; raft snapshots don't restore across implementations in the direction assumed; the `vault` CLI in every CI job needs swapping for `bao` with subtle flag drift.
- **Why it happens (the mechanism):** Fork compatibility is *strong at the core and undefined at the edges*, and it decays over time as both sides evolve independently. The compatible 95% makes the incompatible 5% invisible until production touches it.
- **How to handle it in production, and why that works:** Inventory before migrating: every mount, auth method, plugin, policy feature, and *client integration* (agents, injectors, CI, vendor tools), checked item-by-item against OpenBao's current support matrix — the checklist in [reference.md](reference.md). Migrate by standing up OpenBao alongside Vault and moving mounts/consumers incrementally (secrets re-created or exported per engine, not assumed to bit-copy), with the old system read-only-then-decommissioned rather than big-banged. Pin and test client-library versions against OpenBao in CI before any production traffic.
- **Trade-offs of the fix:** Parallel-run migration is slower and runs two tier-0 systems for a while. The alternative discovers the incompatibility list in production, one outage per item.

## Migration Walkthrough

From Vault OSS to OpenBao, the shape that works:

1. **Inventory** — `vault secrets list`, `vault auth list`, `vault plugin list`; every consumer (apps, agents, injectors, CI jobs, vendor integrations); every policy feature in use. Check each against OpenBao's docs. Anything Enterprise-only or post-1.14 goes on the risk list with a per-item plan.
2. **Stand up OpenBao properly** — Raft cluster, unseal story, audit devices, snapshot schedule — as a fresh cluster (don't try to make Vault data directories into OpenBao ones; treat data as migrating through the API layer per engine: KV exported/imported, dynamic engines reconfigured from IaC, PKI either re-rooted or intermediate-signed, transit keys *recreated and data re-wrapped* — exportable transit keys are the exception, not the rule).
3. **Move machine identity first** — configure the same K8s/cloud/OIDC auth methods and policies (from code) on OpenBao; point a canary service's client at it.
4. **Migrate mounts incrementally** — lowest-risk first (a team's KV), dynamic DB creds next (dual-configure the database briefly), PKI and transit last (they have the longest-lived downstream artifacts: issued certs and wrapped ciphertexts respectively — plan `min_decryption_version`-style cutovers).
5. **Flip clients per mount** (`VAULT_ADDR`→`BAO_ADDR`, `vault`→`bao` in CI, agent images swapped), watching audit logs on both sides to find stragglers.
6. **Vault read-only, then off** — after a full lease-lifetime + one rotation cycle has completed on OpenBao with nothing reading Vault.

Rollback plan at every step = the mount still existing on Vault until its consumers are verified on OpenBao.

## Open Questions

- Current state check (docs move fast): which Vault 1.15+ features have OpenBao equivalents now, what's the K8s injector/CSI story maturity, and what does the official support matrix list for storage backends and plugins?
- Transit key migration specifics: is there any supported wrapped-export path for non-exportable keys, or is re-encrypt-everything genuinely the only route off Vault transit?
- Raft snapshot compatibility between Vault and OpenBao — supported in which direction, at which versions, if at all?
- Namespaces in OpenBao: semantics vs. Vault Enterprise namespaces — same isolation model? Policy path implications?
- Benchmark: transit encrypt/decrypt throughput (single + batch endpoints) on a 3-node Raft cluster — is the batch API enough for the per-user DEK unwrap load in the envelope design at our scale?
- Bao agent: caching and auto-auth behavior during an OpenBao outage — exactly how much client-side grace does it buy?

## References

- [openbao.org](https://openbao.org/) and [OpenBao docs](https://openbao.org/docs/) — the source of truth for current feature state; structured like (because forked from) Vault's docs.
- [OpenBao GitHub](https://github.com/openbao/openbao) — the changelog is the honest record of post-fork divergence; watch releases.
- HashiCorp Vault documentation — still useful for *concepts* (the architecture/internals pages) given the shared model; treat feature pages as Vault-specific until verified.
- The Linux Foundation OpenBao announcement + project page — the governance story, which is the reason the project exists.
- Related topics in this repo: [Encryption & Key Management](../../architecture-patterns/encryption-and-key-management/learning.md) (the envelope pattern OpenBao's transit engine implements — read together), [Consensus & Leader Election](../../architecture-patterns/consensus-and-leader-election/learning.md) (Raft integrated storage is this), [OpenTofu](../opentofu/learning.md) (the sibling fork from the same licensing event).
