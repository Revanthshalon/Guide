# OpenTofu — Learning Notes

> **Accuracy note:** OpenTofu ships fast and its divergence from Terraform grows with each release. Feature claims below reflect roughly the 1.8–1.10 line as of early 2026 — verify against [opentofu.org/docs](https://opentofu.org/docs/) before relying on any specific capability. For setup and operations, see [runbook.md](runbook.md).

## What It Is & Why It Exists

OpenTofu is an infrastructure-as-code tool: you declare the resources you want in HCL, and it computes and executes the changes needed to make reality match. It is the community **fork of the last MPL-licensed Terraform** (the 1.5.x line), created after HashiCorp relicensed Terraform to the source-available **BUSL** in August 2023 — the same event that produced [OpenBao from Vault](../openbao/learning.md), and with the same resolution: the fork was adopted by the **Linux Foundation**, keeps MPL 2.0, and is governed by an open technical steering committee rather than a vendor.

What matters practically about the fork:

- **The mental model is Terraform's**, entirely. HCL syntax, the plan/apply cycle, state, providers, modules, backends — all identical in concept and mostly in surface. The CLI is `tofu`, mirroring `terraform` command for command. Terraform knowledge, tutorials, and Stack Overflow answers transfer almost completely.
- **The provider ecosystem is shared.** Providers are separate binaries with a stable protocol, so the AWS/Google/Azure/Kubernetes providers work with both. OpenTofu runs its own registry (`registry.opentofu.org`) that mirrors the community provider namespace.
- **Divergence is real and growing in both directions.** OpenTofu has shipped features Terraform doesn't have — most notably **client-side state encryption**, plus early variable evaluation (variables usable in module sources and backend config), `for_each` over providers, and an `-exclude` flag. Terraform 1.6+ has shipped features OpenTofu may lack or implement differently. Neither side's post-fork additions transfer automatically; every specific feature needs a per-item check.
- **Governance is the reason to choose it**, not a feature list. If your objection is depending on one vendor's licensing decisions, that's the argument. Expecting feature parity with current Terraform is not the deal.

The strategic read is the same as OpenBao's: for the **core workflow** — HCL, providers, modules, state, plan/apply, remote backends — OpenTofu is a drop-in. The risk sits at the edges: a Terraform-only feature you depend on, a vendor tool that hardcodes the `terraform` binary or its registry, or a managed platform (Terraform Cloud) whose equivalent you'd need to source elsewhere.

## Architecture & Core Concepts

### The three-way model (config, state, reality)

- **What it is:** OpenTofu continuously reconciles three things: your **configuration** (desired state, in HCL), the **state file** (what OpenTofu believes exists, including each resource's real-world ID and last-known attributes), and **reality** (what the provider's API actually reports). A `plan` refreshes state from reality, diffs it against config, and produces the change set.
- **Why it matters operationally:** Essentially every OpenTofu problem is a divergence among these three, and naming which divergence you have tells you the fix. *Config ≠ state* is a normal pending change (`apply`). *Reality ≠ state* is **drift** — someone changed it outside OpenTofu (`plan` shows an unexpected diff; fix the reality or update the config). *State ≠ reality because the resource is gone* is an orphan (remove it from state, or let apply recreate it). *Resource exists in reality but not in state* is unmanaged infrastructure (`import` it).
- **Example:** Someone widens a security group rule in the cloud console. Config still says port 443 only; state still records 443. The next `plan` refreshes, sees 443+22 in reality, and proposes removing 22 — which is usually correct (config is the intended source of truth) and occasionally a disaster (the manual change was an emergency fix nobody wrote down). This is why plans are reviewed by humans.

### State: the crown jewel

- **What it is:** A JSON document mapping each configuration address (`aws_instance.web`) to a real resource ID (`i-0abc123`), plus the last-read attribute values, dependency edges, and metadata. Stored locally by default; in a **backend** (S3, GCS, Azure Blob, Postgres, HTTP) for real use.
- **Why it matters operationally:** State is *not* a cache — it's the only record of the mapping between your config and real resources. Lose it and OpenTofu no longer knows anything it created exists; the next apply tries to create everything again (usually failing on name conflicts, sometimes succeeding and duplicating your infrastructure). Corrupt it and behavior becomes arbitrary. That's why remote state, versioning, and backups are non-negotiable, and why **state locking** exists: two concurrent applies against one state file interleave writes and corrupt it.
- **Example:** State also historically contains **secrets in plaintext** — a database password passed to a provider is stored as an attribute value, regardless of whether the variable was marked `sensitive` (which only redacts CLI output). This is the single most-cited Terraform security complaint, and OpenTofu's answer is the next concept.

### State encryption (OpenTofu's flagship divergence)

- **What it is:** Client-side encryption of the state file (and plan files) via an `encryption` block, with pluggable key providers: PBKDF2 from a passphrase, AWS/GCP/Azure KMS, or **[OpenBao](../openbao/learning.md)/Vault transit**. The state is encrypted before it reaches the backend, so the backend never sees plaintext.
- **Why it matters operationally:** It converts "the state bucket is a secrets store that everyone with read access can mine" into "the state bucket holds ciphertext." Combined with a KMS key whose access is separately controlled, it makes bucket-read access insufficient for secret extraction — a meaningful reduction in blast radius, and precisely the [envelope-encryption model](../../architecture-patterns/encryption-and-key-management/learning.md) applied to a file. It also supports key rotation with a fallback key, so you can re-encrypt without a flag day.
- **Example:** With `key_provider "aws_kms"` and `method "aes_gcm"`, an engineer who obtains the state bucket's contents gets AES-GCM ciphertext; they'd additionally need KMS decrypt permission on the key. Note the caveat that keeps this honest: encryption protects state *at rest in the backend*. It does not stop a secret from existing in plaintext in memory, in the plan output, or in the logs of whoever ran the apply — the deeper fix is still not putting long-lived secrets into OpenTofu-managed resources at all (see the pitfall).

### Providers and the resource lifecycle

- **What it is:** Providers are separately-distributed plugins implementing CRUD for a platform's resources over a stable RPC protocol. OpenTofu downloads them at `init`, records exact versions and checksums in a **lock file** (`.terraform.lock.hcl`), and calls them during plan and apply.
- **Why it matters operationally:** Two properties drive most surprises. First, **plan-time vs apply-time knowledge**: attributes that only exist after creation show as `(known after apply)`, and anything computed from them can't be fully planned — which is why a plan can't always tell you the complete outcome, and why `count = length(some_computed_list)` fails with "value depends on resource attributes that cannot be determined until apply." Second, **the provider decides whether a change is in-place or destroy-and-recreate**: changing an immutable attribute (an EC2 AMI, a database engine version in some providers) forces replacement, and the plan output's `-/+ destroy and then create replacement` is the most important line to read before approving anything.
- **Example:** Pin providers in `required_providers` with `~>` constraints *and* commit the lock file. An unpinned provider upgrade between two applies can change resource defaults, deprecate arguments, or alter what counts as a replacement-forcing change — turning a routine apply into an unplanned rebuild of production.

### The dependency graph

- **What it is:** OpenTofu builds a DAG from **implicit** dependencies (resource A references `resource.b.id`, so B must exist first) and **explicit** ones (`depends_on`), then walks it with parallelism (default 10).
- **Why it matters operationally:** Implicit dependencies are almost always the right mechanism — they're derived from the data flow you already wrote, so they can't get stale. `depends_on` is for hidden relationships the references don't capture (an IAM policy that must exist before a service can assume a role, even though nothing references it), and overusing it serializes the graph and slows applies. Note that `depends_on` on a *module* is coarse and can create surprisingly wide ordering constraints.
- **Example:** `subnet_id = aws_subnet.main.id` creates the edge for free. Adding `depends_on = [aws_subnet.main]` on top adds nothing but noise. Conversely, a null_resource provisioner that shells out to configure something has no references at all — that genuinely needs `depends_on`, and is also a sign the work probably belongs outside OpenTofu.

### Modules, and the addressing that constrains refactoring

- **What it is:** A module is a directory of configuration with inputs (variables), outputs, and resources — invoked with a `module` block and versioned when sourced from a registry or Git tag. Every resource has an **address** (`module.network.aws_subnet.main["a"]`) which is the key OpenTofu uses to match config to state.
- **Why it matters operationally:** Because state is keyed by address, **renaming or moving a resource in config looks like "destroy the old one, create a new one"** unless you tell OpenTofu the resource moved. That's what `moved` blocks are for, and their absence is the cause of the most alarming plan outputs in practice. The same mechanism explains the `count`-vs-`for_each` rule: `count` addresses by *index* (`[0]`, `[1]`), so removing the first element of a list shifts every subsequent index and OpenTofu plans to destroy and recreate all of them; `for_each` addresses by *key* (`["us-east-1a"]`), so removing one element affects exactly that one.
- **Example:** Extracting resources into a module is a pure refactor in config and a total rewrite of addresses in state. Adding `moved { from = aws_subnet.main, to = module.network.aws_subnet.main }` makes the plan show *no changes*, which is what a refactor should look like. Without it, the plan destroys and recreates your subnets.

### Backends, locking, and workspace layout

- **What it is:** A **backend** stores state remotely and (usually) provides locking — S3 (with DynamoDB or, in newer versions, native S3 locking), GCS, Azure Blob, Postgres, or an HTTP backend. **Workspaces** allow multiple named states from one configuration directory.
- **Why it matters operationally:** Locking prevents concurrent-apply corruption and is the reason not to use plain local or unlocked remote state with a team. Workspaces are the more contentious choice: they're convenient for near-identical environments but keep all environments in one configuration and one backend, so a mistake in the shared config reaches all of them and per-environment differences accrete as conditionals. **Separate directories with separate backends** is the more common production choice for prod/staging isolation, with workspaces reserved for ephemeral per-developer or per-PR environments.
- **Example:** Splitting state is also a *blast-radius* decision: one monolithic state means every plan refreshes every resource (slow), every apply holds one lock (contention), and every mistake can touch everything. Splitting by lifecycle and ownership — network, data, per-service — is the standard remedy, with cross-state reads via `terraform_remote_state` data sources or, better, by looking resources up through provider data sources so the coupling is to the cloud rather than to another state file.

## Comparison in Depth

| Aspect | HashiCorp Terraform | OpenTofu |
| --- | --- | --- |
| License | BUSL 1.1 (source-available) since Aug 2023 | MPL 2.0 |
| Governance | HashiCorp (now IBM) | Linux Foundation, open TSC |
| Lineage | Continuous | Forked from Terraform 1.5.x (last MPL) |
| CLI / language | `terraform`, HCL | `tofu`, same HCL |
| Providers | Same plugin protocol | Same — providers work with both |
| Registry | registry.terraform.io | registry.opentofu.org (mirrors community namespace) |
| State encryption | Not built in (rely on backend-side encryption) | **Built-in client-side encryption** with pluggable key providers |
| Early variable evaluation | Limited | Variables usable in module sources / backend config |
| Provider `for_each` | No | Yes |
| `-exclude` flag | No (`-target` only) | Yes |
| Post-1.5 Terraform features (test framework, stacks, etc.) | Yes | Per-feature check required |
| Managed platform | Terraform Cloud/Enterprise | Third-party (Spacelift, env0, Scalr, Terrakube) or self-hosted |
| Ecosystem tooling | Deepest; some tools hardcode `terraform` | Broad and growing; most tools support both |

The honest read: the **core loop is identical**, the **fork-specific wins are real** (state encryption is the standout — a long-requested capability), and the **risk is at the integration edges** — a CI action, a policy tool, or a vendor product that assumes the Terraform binary, registry, or Cloud API.

## Hands-On Notes

```sh
tofu version
tofu init                 # download providers, configure backend, write lock file
tofu fmt -recursive       # canonical formatting
tofu validate             # syntax + internal consistency (no API calls)
tofu plan -out=tfplan     # compute the diff; SAVE it so apply runs exactly this
tofu apply tfplan         # apply the saved plan (no re-plan, no surprises)

tofu state list                       # every managed address
tofu state show aws_instance.web      # attributes as recorded
tofu output -json                     # outputs for scripts

tofu plan -refresh-only               # detect drift without proposing config changes
tofu apply -replace=aws_instance.web  # deliberate recreate (replaces old `taint`)
```

Things worth doing while learning, in a scratch account: create a resource, change it in the cloud console, and run `plan` to see drift; rename a resource without a `moved` block to see the destroy/recreate plan, then add the block and watch it become a no-op; convert a `count` to `for_each` and observe the address churn; delete your local state file and experience exactly why remote state is not optional.

## Pitfalls in Depth

### Pitfall: State loss, corruption, or concurrent writes

- **What goes wrong:** Local state on a laptop that dies; a state file overwritten by two engineers applying simultaneously; a backend without versioning where a bad apply leaves state describing infrastructure that doesn't match reality. In the worst case OpenTofu no longer knows your resources exist, and the next apply tries to create a duplicate production environment.
- **Why it happens (the mechanism):** State is a single mutable JSON document that must be read-modify-written atomically. Without locking, two applies interleave; without versioning, there's nothing to roll back to; without a remote backend, it lives on one machine. Nothing in the default experience (`tofu init` with no backend → local state) prevents any of this, so teams discover the requirement the hard way.
- **How to handle it in production, and why that works:** Remote backend with **locking** (S3 + DynamoDB or native S3 locking, GCS, Azure Blob) and **object versioning enabled** — versioning is what makes a corrupt state recoverable, and it's one bucket setting. Never commit state to Git. Back up before any state surgery (`tofu state pull > backup.tfstate`). Treat the state bucket as tier-0 infrastructure with the same backup and access discipline as a database.
- **Trade-offs of the fix:** A remote backend is a bootstrap dependency (something must create the bucket — usually a small, separately-managed config or a one-time manual step). Locking means a stuck lock blocks the team until it's force-unlocked, which is a documented procedure people need before they need it.

### Pitfall: Secrets in state (and in plans)

- **What goes wrong:** A database password, a generated private key, or an API token passed to a resource is recorded in state in plaintext. Anyone with read access to the state backend can extract every secret the configuration touches — often a much wider group than the secrets' intended audience, because state buckets are frequently readable by all engineers and every CI job.
- **Why it happens (the mechanism):** State must record attribute values to compute diffs, and there's no general way for a provider to diff a value it doesn't store. `sensitive = true` only suppresses *display* in CLI output — it changes nothing about state contents, which is a common and dangerous misunderstanding. Plan files have the same exposure.
- **How to handle it in production, and why that works:** Layered. (1) Enable **[state encryption](#state-encryption-opentofus-flagship-divergence)** with a KMS or [OpenBao](../openbao/learning.md) key provider, so backend read access alone is insufficient. (2) Restrict state-backend access to CI and a small operator group — most engineers need `plan` via CI, not direct bucket access. (3) Best of all, **don't put long-lived secrets in OpenTofu-managed resources**: have the resource generate its own credential, or store the secret in OpenBao and have applications fetch it at runtime, so OpenTofu manages the *reference* rather than the value. (4) Where a secret must transit, treat it as compromised-on-rotation-schedule and rotate after any state exposure.
- **Trade-offs of the fix:** State encryption adds a key dependency to every plan/apply (a KMS outage blocks infrastructure changes) and a key-rotation procedure. Keeping secrets out of state entirely often means an extra runtime lookup and a bootstrap question ("how does the app authenticate to OpenBao?" — [secure introduction](../openbao/runbook.md) again).

### Pitfall: The monolithic state file

- **What goes wrong:** One state for the entire organization. Plans take twenty minutes because every resource is refreshed; every change holds the single lock, so the team serializes; and a mistargeted `destroy` or a bad module change can affect everything. Eventually people start using `-target` routinely to make plans tolerable, which quietly abandons the whole-configuration consistency guarantee.
- **Why it happens (the mechanism):** One state is the natural starting point and there's no forcing function to split — degradation is gradual, and by the time plans are painful, splitting means a large state-surgery migration. Cross-resource references also make splitting *feel* impossible, because everything appears connected.
- **How to handle it in production, and why that works:** Split by **blast radius and rate of change**, which usually means layering: foundational/network (changes rarely, many dependents), data stores (changes rarely, high consequence), per-service or per-team application infrastructure (changes constantly). Wire layers together through provider **data sources** (look up the VPC by tag) rather than `terraform_remote_state` where possible — that couples you to the cloud's own API rather than to another team's state layout, so their refactor doesn't break you. Split *before* it hurts; retrofitting requires `state mv` across backends or import-and-remove, which is careful work.
- **Trade-offs of the fix:** More states means more backends to configure, more CI pipelines, and orchestration questions when a change spans layers (usually: apply the lower layer first, then the upper). Cross-state lookups also mean a dependency's change can surprise a dependent — which is why data-source lookups with explicit contracts (tags, naming conventions) beat reaching into another state.

### Pitfall: Drift from manual changes

- **What goes wrong:** Someone fixes production in the console during an incident (correctly — the incident mattered more than the process) and doesn't backport it to config. Weeks later an unrelated apply reverts the fix, reproducing the incident. Or drift accumulates until nobody trusts the plan output, and every apply requires archaeology.
- **Why it happens (the mechanism):** OpenTofu's model assumes config is the sole source of truth, but console access exists, emergencies happen, and other tools (autoscalers, operators, cloud-native services) legitimately modify resources. The tool can only reconcile what it's told to manage, and it treats any unrecorded change as something to undo.
- **How to handle it in production, and why that works:** Detect drift continuously rather than discovering it at apply time — a scheduled `tofu plan -refresh-only -detailed-exitcode` in CI that alerts on a non-zero result makes drift a *daily* signal instead of an incident-time surprise. Reduce the surface: read-only console access for most engineers, break-glass write access that's audited and triggers a backport task. Where a resource is *legitimately* modified externally (an autoscaler adjusting capacity, a service managing its own tags), use `lifecycle { ignore_changes = [...] }` to declare that explicitly, which is far better than the plan proposing to fight the autoscaler every run.
- **Trade-offs of the fix:** Drift detection runs cost API calls and can be noisy where cloud providers mutate attributes themselves (a common annoyance requiring `ignore_changes` tuning). Restricting console access has an incident-response cost that needs a real break-glass path.

### Pitfall: Refactoring without `moved` (destroy/recreate surprises)

- **What goes wrong:** A resource is renamed, extracted into a module, or converted from `count` to `for_each`. The plan proposes to **destroy and recreate** it — a database, a load balancer, a persistent volume — because from state's perspective the old address disappeared and a new one appeared. Approved carelessly, this is a production outage caused by a refactor that changed no behavior.
- **Why it happens (the mechanism):** State is keyed by configuration address; the address *is* the identity as far as OpenTofu is concerned. Renaming is therefore indistinguishable from delete-plus-create unless you say otherwise — and `count`'s index-based addressing makes it worse, since removing one element from the middle of a list shifts every subsequent index and cascades into recreating many resources.
- **How to handle it in production, and why that works:** Use **`moved` blocks** for every rename/extraction — they're declarative, reviewable in the diff, safe to leave in place, and turn the plan into a no-op (which is exactly what a refactor should produce). Default to **`for_each` over `count`** for anything that isn't a simple on/off toggle, so identity is stable under list changes. And make the review rule explicit: *any plan containing `destroy` or `replace` on a stateful resource requires justification in the PR*, which is a cheap gate that catches this class before it lands.
- **Trade-offs of the fix:** `moved` blocks accumulate as clutter (they can be removed after everyone has applied, but tracking that requires care). `for_each` requires stable keys, which sometimes forces awkward map construction from lists — worth it.

### Pitfall: Auto-apply without a reviewed plan

- **What goes wrong:** CI runs `tofu apply -auto-approve` on merge. A provider upgrade, a drifted resource, or an unnoticed `force_replacement` turns a routine merge into a destroyed database. Or the plan reviewed on the PR isn't the plan that runs on merge — state changed in between — so the reviewed artifact and the executed change differ.
- **Why it happens (the mechanism):** Plan and apply are separate operations against a *moving* target: state and reality both change between them. `apply` without a saved plan file re-plans at apply time, so what executes was never reviewed by anyone. Auto-approve makes that gap invisible.
- **How to handle it in production, and why that works:** **Save the plan and apply that artifact**: `tofu plan -out=tfplan` on the PR, publish it for review, then `tofu apply tfplan` on merge — OpenTofu refuses to apply a saved plan if state has changed since, which converts a silent divergence into an explicit failure. Add automated policy checks on the plan JSON (OPA/Conftest, or a script) that fail the build on deletion of protected resource types, and `lifecycle { prevent_destroy = true }` on genuinely irreplaceable resources as a last-resort guard. Require human approval for production applies; auto-apply is defensible for ephemeral or low-stakes environments.
- **Trade-offs of the fix:** A saved-plan workflow makes merges fail when state moved (a real annoyance requiring a re-plan) — that annoyance is the safety property working. Human approval slows delivery, which is why the gate should scale with blast radius rather than applying uniformly.

### Pitfall: Unpinned providers and modules

- **What goes wrong:** `required_providers` has no version constraint, or the lock file isn't committed. Two engineers get different provider versions; CI gets a third. A new provider release changes a default, deprecates an argument, or alters what forces replacement — and an apply that was clean yesterday now proposes to rebuild half the environment.
- **Why it happens (the mechanism):** Version resolution happens at `init`, so it's invisible in the configuration under review and varies by *when* and *where* init ran. The lock file exists precisely to freeze this, and it only works if it's committed and respected in CI.
- **How to handle it in production, and why that works:** Pin providers with `~>` constraints in `required_providers`, **commit `.terraform.lock.hcl`**, and run CI with `tofu init -lockfile=readonly` so an unexpected version change fails the build rather than silently happening. Pin module sources to tags or commit SHAs, never a moving branch. Upgrade deliberately: bump the constraint, run `init -upgrade`, review the resulting plan across environments, and merge the lock-file change as its own reviewable commit.
- **Trade-offs of the fix:** Pinning means deliberate upgrade work rather than drifting forward for free, and lock-file conflicts in busy repos are mildly annoying. Both are trivially better than a surprise rebuild.

## Migration Walkthrough

From Terraform to OpenTofu, the shape that works:

1. **Inventory the risk surface**, not the code: which Terraform features post-1.5 do you use (`test` framework? stacks? Cloud-specific features?), which CI actions/tools invoke `terraform` by name, which policy or cost tools parse Terraform-specific output, and whether you depend on Terraform Cloud/Enterprise (the biggest item — you need a replacement platform or self-hosted CI).
2. **Verify the version path.** OpenTofu reads Terraform state from the 1.5.x lineage forward; state written by *newer* Terraform versions may not be readable, and migrating *back* from OpenTofu-written state is version-dependent and sometimes impossible. Check your current Terraform version against OpenTofu's documented compatibility before anything else — this is the one-way-door check.
3. **Trial in a low-stakes environment**: install `tofu`, run `tofu init` and `tofu plan` against an existing state *without applying*, and confirm the plan is empty. An empty plan is the migration's green light; a non-empty one means something interprets your config differently and must be understood before proceeding.
4. **Back up state** (versioned bucket plus an explicit `state pull` snapshot) before the first `tofu apply` against it.
5. **Migrate environment by environment**, lowest stakes first, applying the empty-plan check at each. Update CI in the same change so the pipeline and the local workflow don't diverge.
6. **Update tooling**: CI actions (`opentofu/setup-opentofu` or equivalent), pre-commit hooks, linters (`tflint` supports both), docs generators, and any `terraform`-named wrapper scripts.
7. **Then adopt the divergent features** — state encryption in particular — as a *separate* change, so the migration and the new capability are independently revertible.

Rollback plan at each step: the previous Terraform binary plus the pre-migration state snapshot, valid until you've applied with OpenTofu (after which state may have moved forward — which is why step 2 matters).

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Name the three things OpenTofu reconciles, and classify each of these: an unexpected diff after a console change; a resource in the cloud that no plan mentions; a plan proposing changes right after a successful apply.
2. Why is state not a cache? Describe precisely what happens on the next apply if you lose it.
3. Does `sensitive = true` keep a password out of state? What does it actually do, and what are the four layers of the real fix?
4. Why does renaming a resource propose destroy-and-recreate? Give the mechanism and the declarative fix.
5. Explain the `count` vs `for_each` addressing difference, and what happens when you remove the first element of a list under each.
6. What does `(known after apply)` mean, and why does it make `count = length(<computed>)` an error?
7. Why does `tofu apply tfplan` (saved plan) fail if state changed since planning — and why is that a feature rather than an inconvenience?
8. Give three ways to split a monolithic state, and explain why data-source lookups beat `terraform_remote_state` for cross-layer references.

Hands-on exercises (scratch account only):

- Create a resource, modify it in the console, and run `plan -refresh-only`. Then set `ignore_changes` on that attribute and watch the drift stop being reported — the two legitimate responses to drift, felt.
- Rename a resource without a `moved` block and read the plan; add the block and re-plan. The before/after is the most useful thirty seconds in this doc.
- Enable state encryption with the PBKDF2 key provider locally, inspect the state file to confirm it's ciphertext, then rotate to a second key using the fallback mechanism.
- Delete your local state file with resources still deployed, then recover them with `import` blocks. Painful, instructive, and exactly the drill you want to have already done.

## Open Questions

- Current feature-parity matrix: which post-1.5 Terraform features have OpenTofu equivalents now (test framework? stacks-like composition?), and which OpenTofu features has Terraform since matched?
- State encryption in practice: performance impact on large states, the operational story when the KMS key is unavailable, and whether the OpenBao key provider is production-ready.
- Managed platforms: Spacelift vs env0 vs Scalr vs self-hosted Atlantis for OpenTofu — which handle state encryption and OpenTofu-specific features natively?
- Provider registry reliability and mirroring: what's the recommended air-gapped/mirror setup, and how does provider signing/verification differ from Terraform's?
- Migration reversibility: precisely which OpenTofu → Terraform paths still work at current versions, since this determines how much of a one-way door the migration is.

## References

- [OpenTofu documentation](https://opentofu.org/docs/) — the authority; the state-encryption and migration pages are the ones with no Terraform equivalent to fall back on.
- [OpenTofu migration guide](https://opentofu.org/docs/intro/migration/) — version compatibility and the step sequence, from the project itself.
- Terraform documentation and *Terraform: Up & Running* (Yevgeniy Brikman) — still the best conceptual teaching material for the shared model; translate `terraform` → `tofu` and treat post-1.5 features as unverified.
- [OpenTofu GitHub](https://github.com/opentofu/opentofu) — the changelog is the honest record of post-fork divergence.
- Related topics in this repo: [OpenBao](../openbao/learning.md) (the sibling fork from the same licensing event, and a state-encryption key provider), [Encryption & Key Management](../../architecture-patterns/encryption-and-key-management/learning.md) (what state encryption is doing, and why keeping secrets out of state is the deeper fix), [Strangler Fig](../../architecture-patterns/strangler-fig/learning.md) (importing existing infrastructure into IaC is exactly that pattern), [Consensus & Leader Election](../../architecture-patterns/consensus-and-leader-election/learning.md) (what state locking is protecting against).
