# OpenTofu — Setup & Operations Runbook

> **Accuracy note:** Reflects roughly the OpenTofu 1.8–1.10 line as of early 2026; backend options and encryption syntax evolve — verify against [opentofu.org/docs](https://opentofu.org/docs/). Concepts are in [learning.md](learning.md); this is the procedure.

---

## Part 1 — Development setup

### 1.1 Install and first project

```sh
brew install opentofu          # or: see opentofu.org/docs/intro/install
tofu version
```

```hcl
# main.tf — minimal but with the parts people omit
terraform {
  required_version = "~> 1.9"

  required_providers {
    aws = {
      source  = "hashicorp/aws"      # resolved via registry.opentofu.org
      version = "~> 5.60"            # PIN — unpinned providers rebuild your infra someday
    }
  }
}

provider "aws" {
  region = var.region
}

variable "region" {
  type    = string
  default = "eu-west-1"
}
```

```sh
tofu init          # downloads providers, writes .terraform.lock.hcl  ← COMMIT THIS
tofu fmt
tofu validate
tofu plan -out=tfplan
tofu apply tfplan
```

### 1.2 What local dev does differently (and why it's not production)

| Local default | Production requirement |
| --- | --- |
| Local `terraform.tfstate` file | Remote backend with locking + object versioning |
| No state encryption | Encrypted state (KMS / OpenBao key provider) |
| State on one laptop, no backup | Versioned bucket, tier-0 treatment |
| `tofu apply` re-plans at apply time | Saved plan artifact, reviewed, applied unchanged |
| Personal cloud credentials | CI identity with scoped role, human approval gate |
| One state for everything | Split by blast radius and rate of change |

### 1.3 Local safety habits worth forming now

```sh
tofu plan -out=tfplan && tofu show tfplan     # read before applying, every time
tofu state pull > backup-$(date +%s).tfstate  # before ANY state surgery
```

Add to `.gitignore` immediately — leaking state to Git is how secrets end up in history:

```gitignore
*.tfstate
*.tfstate.*
.terraform/
tfplan
*.tfvars          # except *.auto.tfvars you intend to commit
!.terraform.lock.hcl
```

---

## Part 2 — Production setup

### 2.1 Bootstrap the backend (the chicken-and-egg step)

The state backend must exist before OpenTofu can use it. Create it once, deliberately — either by hand or with a tiny separately-managed config whose own state is local and committed *after* being encrypted, or simply accepted as bootstrap.

```sh
# AWS example — do this once, per environment
aws s3api create-bucket --bucket acme-tfstate-prod --region eu-west-1 \
  --create-bucket-configuration LocationConstraint=eu-west-1

aws s3api put-bucket-versioning --bucket acme-tfstate-prod \
  --versioning-configuration Status=Enabled          # ← makes corruption recoverable

aws s3api put-bucket-encryption --bucket acme-tfstate-prod \
  --server-side-encryption-configuration '{"Rules":[{"ApplyServerSideEncryptionByDefault":{"SSEAlgorithm":"aws:kms"}}]}'

aws s3api put-public-access-block --bucket acme-tfstate-prod \
  --public-access-block-configuration "BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true"

# KMS key for CLIENT-SIDE state encryption (separate from bucket SSE)
aws kms create-key --description "opentofu state encryption prod"
```

Versioning is the single most important line here — it's what turns "state is corrupt" from an incident into a restore.

### 2.2 Backend + state encryption configuration

```hcl
terraform {
  required_version = "~> 1.9"

  backend "s3" {
    bucket       = "acme-tfstate-prod"
    key          = "network/terraform.tfstate"   # one key per state — plan your layout
    region       = "eu-west-1"
    encrypt      = true                          # server-side (bucket) encryption
    use_lockfile = true                          # native S3 locking (newer versions);
                                                 # older: dynamodb_table = "tf-locks"
  }

  # CLIENT-side state encryption — OpenTofu's flagship feature.
  # State is ciphertext before it ever reaches S3.
  encryption {
    key_provider "aws_kms" "primary" {
      kms_key_id = "arn:aws:kms:eu-west-1:123456789012:key/abcd-..."
      region     = "eu-west-1"
      key_spec   = "AES_256"
    }

    method "aes_gcm" "primary" {
      keys = key_provider.aws_kms.primary
    }

    state {
      method   = method.aes_gcm.primary
      enforced = true          # refuse to write unencrypted state
    }
    plan {
      method   = method.aes_gcm.primary
      enforced = true          # plan files contain the same secrets
    }
  }
}
```

Two things people miss: **encrypt plans too** (a plan file contains the same attribute values as state), and `enforced = true` (without it, a misconfiguration silently falls back to plaintext).

### 2.3 State layout — decide before you have twenty services

Split by **blast radius and rate of change**:

```
tfstate-prod/
  foundation/terraform.tfstate     # VPC, subnets, DNS zones — changes rarely, everything depends on it
  data/terraform.tfstate           # RDS, S3 buckets, backups — rare changes, high consequence
  platform/terraform.tfstate       # EKS cluster, shared IAM
  services/orders/terraform.tfstate    # per-service — changes constantly, small blast radius
  services/payments/terraform.tfstate
```

Wire layers together with **provider data sources**, not `terraform_remote_state`, where possible:

```hcl
# GOOD: couples to the cloud's API and a naming/tagging contract
data "aws_vpc" "main" {
  tags = { Name = "acme-prod" }
}

# WEAKER: couples to another team's state file layout and output names
data "terraform_remote_state" "foundation" {
  backend = "s3"
  config  = { bucket = "acme-tfstate-prod", key = "foundation/terraform.tfstate", region = "eu-west-1" }
}
```

### 2.4 Repository structure

```
infra/
  modules/                      # reusable, versioned, no provider blocks inside
    network/{main,variables,outputs}.tf
    service/{main,variables,outputs}.tf
  environments/
    prod/{main.tf,backend.tf,terraform.tfvars,.terraform.lock.hcl}
    staging/...
  .github/workflows/tofu.yml
```

Separate directories per environment (not workspaces) for prod/staging: independent state, independent backend config, and no shared configuration whose mistake reaches everything. Reserve workspaces for ephemeral per-PR or per-developer environments.

---

## Part 3 — Credentials and identity

**Never** long-lived cloud keys in CI secrets. Use workload identity federation so CI authenticates with a short-lived token:

```yaml
# GitHub Actions → AWS via OIDC; no stored credentials
permissions:
  id-token: write
  contents: read
steps:
  - uses: aws-actions/configure-aws-credentials@v4
    with:
      role-to-assume: arn:aws:iam::123456789012:role/tofu-plan-prod
      aws-region: eu-west-1
```

Two roles, deliberately asymmetric:

| Role | Permissions | Used by |
| --- | --- | --- |
| `tofu-plan-*` | Read-only on resources + read state + **KMS decrypt** | PR pipeline (untrusted input) |
| `tofu-apply-*` | Write on resources + read/write state + KMS encrypt/decrypt | Post-merge, protected environment, human approval |

For secrets *inside* configuration, read them from [OpenBao](../openbao/runbook.md) at plan time rather than storing them in tfvars — and prefer resources that generate their own credentials so no secret is passed through OpenTofu at all.

---

## Part 4 — The CI/CD pipeline

The core discipline: **plan on PR, publish it for review, apply that exact artifact on merge.**

```yaml
name: tofu
on:
  pull_request:
  push: { branches: [main] }

jobs:
  plan:
    runs-on: ubuntu-latest
    permissions: { id-token: write, contents: read, pull-requests: write }
    steps:
      - uses: actions/checkout@v4
      - uses: opentofu/setup-opentofu@v1
        with: { tofu_version: 1.9.0 }
      - run: tofu fmt -check -recursive
      - run: tofu init -lockfile=readonly      # fail if lock file would change
      - run: tofu validate
      - run: tofu plan -out=tfplan -input=false
      - run: tofu show -json tfplan > plan.json
      - run: conftest test plan.json           # policy gate (see below)
      - run: tofu show -no-color tfplan        # post to the PR for human review
      - uses: actions/upload-artifact@v4
        with: { name: tfplan, path: tfplan }   # the artifact that will be applied

  apply:
    needs: plan
    if: github.ref == 'refs/heads/main'
    environment: production                    # ← GitHub approval gate
    runs-on: ubuntu-latest
    permissions: { id-token: write, contents: read }
    steps:
      - uses: actions/checkout@v4
      - uses: opentofu/setup-opentofu@v1
      - uses: actions/download-artifact@v4
        with: { name: tfplan }
      - run: tofu init -lockfile=readonly
      - run: tofu apply -input=false tfplan    # applies the REVIEWED plan; fails if state moved
```

`tofu apply tfplan` refusing to run when state has changed since planning is a **feature** — it means what executes is what was reviewed. Re-plan when it fires.

### Policy gate

Fail the build on destructive changes to protected resources rather than relying on review attention:

```rego
# policy/no_destroy.rego
package main
protected := {"aws_db_instance", "aws_s3_bucket", "aws_rds_cluster"}

deny[msg] {
  rc := input.resource_changes[_]
  protected[rc.type]
  rc.change.actions[_] == "delete"
  msg := sprintf("refusing to delete protected resource: %s", [rc.address])
}
```

Belt and braces in the config itself:

```hcl
lifecycle { prevent_destroy = true }    # on genuinely irreplaceable resources
```

---

## Part 5 — Day-2 operations

### 5.1 Drift detection (scheduled, not discovered)

```yaml
on:
  schedule: [{ cron: "0 6 * * *" }]
# ...
- run: |
    tofu plan -refresh-only -detailed-exitcode -input=false
    # exit 0 = no drift, 1 = error, 2 = drift detected
  continue-on-error: true
  id: drift
- if: steps.drift.outputs.exitcode == '2'
  run: <alert to Slack/on-call with the plan output>
```

Legitimate external mutation gets declared, not fought:

```hcl
lifecycle { ignore_changes = [desired_count, tags["LastScaledAt"]] }
```

### 5.2 Importing existing infrastructure

Prefer **`import` blocks** (declarative, planned, reviewable) over the imperative `tofu import`:

```hcl
import {
  to = aws_s3_bucket.legacy_assets
  id = "acme-legacy-assets"
}

resource "aws_s3_bucket" "legacy_assets" {
  bucket = "acme-legacy-assets"
}
```

```sh
tofu plan -generate-config-out=generated.tf   # scaffold config from reality
tofu plan                                     # expect: 1 to import, 0 to change
tofu apply
```

Target an **empty plan after import** — any proposed change means your config doesn't match reality yet, and applying it would modify live infrastructure you meant only to adopt. (Adopting existing infrastructure incrementally is the [strangler fig](../../architecture-patterns/strangler-fig/learning.md) pattern applied to IaC.)

### 5.3 State surgery (always back up first)

```sh
tofu state pull > backup-$(date +%s).tfstate     # ALWAYS, before anything below

tofu state list                                   # inventory
tofu state show aws_instance.web                  # inspect
tofu state mv aws_instance.web module.app.aws_instance.web   # prefer a `moved` block instead
tofu state rm aws_instance.web                    # forget WITHOUT destroying (careful)
tofu force-unlock <LOCK_ID>                       # only after confirming no apply is running
```

Prefer declarative equivalents where they exist — they're reviewable in a PR and reproducible:

```hcl
moved   { from = aws_subnet.main, to = module.network.aws_subnet.main }
removed { from = aws_instance.old, lifecycle { destroy = false } }
```

### 5.4 Recovering corrupt or lost state

```sh
aws s3api list-object-versions --bucket acme-tfstate-prod --prefix network/terraform.tfstate
aws s3api get-object --bucket acme-tfstate-prod --key network/terraform.tfstate \
  --version-id <PREVIOUS_VERSION> restored.tfstate
tofu state push restored.tfstate                  # then plan and verify carefully
```

If state is gone entirely: rebuild with `import` blocks, resource by resource. Slow and unpleasant — which is the argument for versioning.

### 5.5 Upgrades

```sh
# Provider upgrade — deliberate, reviewable, one commit
# 1. bump the constraint in required_providers
tofu init -upgrade
tofu plan            # review carefully — providers can change defaults and replacement rules
git add .terraform.lock.hcl && git commit -m "bump aws provider to ~> 5.70"
# 2. merge through staging first, then prod

# OpenTofu version upgrade: bump in CI + required_version, plan in a low-stakes env first.
# State written by a newer version may not be readable by an older one — one-way door.
```

### 5.6 Monitoring signals

| Signal | Alert when |
| --- | --- |
| Scheduled drift plan | Exit code 2 (drift detected) |
| Apply failures | Any — a half-applied change leaves partial state |
| State lock age | Held > ~30 min (stuck apply or crashed runner) |
| State file size / resource count | Growing toward slow plans — time to split |
| Plan duration | Trending up — same signal |
| KMS key availability | Any failure — blocks *all* plans and applies |

---

## Part 6 — Dev → production checklist

**Before the first production apply**
- [ ] Remote backend with **locking** and **object versioning** enabled
- [ ] Client-side `encryption` block with `enforced = true` on both `state` and `plan`
- [ ] KMS key (or OpenBao key provider) with its own access policy and a documented rotation plan
- [ ] State layout planned by blast radius; one state key per layer
- [ ] `.gitignore` covers `*.tfstate*`, `.terraform/`, `tfplan`, `*.tfvars` — and does **not** ignore `.terraform.lock.hcl`
- [ ] Providers pinned; lock file committed; CI uses `-lockfile=readonly`
- [ ] CI identity via OIDC/workload identity — no long-lived cloud keys
- [ ] Separate plan (read-only) and apply (write) roles

**Before enabling automation**
- [ ] Plan artifact saved on PR, published for review, applied unchanged on merge
- [ ] Human approval gate on production applies
- [ ] Policy check on plan JSON blocking deletion of protected resource types
- [ ] `prevent_destroy` on irreplaceable resources
- [ ] `moved` blocks required for refactors; review rule that any `destroy`/`replace` on stateful resources needs justification

**Before you can call it production**
- [ ] Scheduled drift detection with alerting
- [ ] State restore from a bucket version **tested end to end**
- [ ] `force-unlock` procedure documented (and the "confirm nothing is running" step spelled out)
- [ ] Provider/OpenTofu upgrade path rehearsed in staging
- [ ] Documented answer to "what happens if the KMS key is unavailable?"

---

## Common mistakes → what actually happens

| Mistake | Consequence |
| --- | --- |
| Local state on a laptop | One disk failure and OpenTofu forgets your infrastructure exists |
| Bucket versioning not enabled | A corrupt state write is unrecoverable |
| No locking | Two concurrent applies interleave and corrupt state |
| State committed to Git | Every secret it contains is in history, forever |
| `sensitive = true` assumed to protect state | It only redacts CLI output — the value is in state in plaintext |
| `encryption` block without `enforced = true` | Silent fallback to plaintext state on misconfiguration |
| Plans not encrypted | Plan artifacts leak the same secrets as state |
| `apply -auto-approve` on merge | Applies a plan nobody reviewed, against state that may have moved |
| Rename without a `moved` block | Plan destroys and recreates the resource — outage from a pure refactor |
| `count` over a list that changes | Removing element 0 shifts every index → cascading recreation |
| Unpinned providers / lock file not committed | A clean apply yesterday rebuilds production today |
| One monolithic state | 20-minute plans, one global lock, unlimited blast radius |
| `tofu state rm` to "fix" something | Resource silently orphaned — still running, still billing, now unmanaged |
| `force-unlock` on a live apply | Two writers on one state file — the corruption you were preventing |
| KMS key deleted | Every existing state file becomes permanently unreadable |

---

## References

- [OpenTofu documentation](https://opentofu.org/docs/) — the authority; the [state encryption](https://opentofu.org/docs/language/state/encryption/) and backend pages are the ones with no Terraform equivalent.
- [OpenTofu migration guide](https://opentofu.org/docs/intro/migration/) — version compatibility, which decides how one-way the door is.
- [learning.md](learning.md) — the concepts behind every procedure here; [reference.md](reference.md) — the command cheat sheet.
- [OpenBao runbook](../openbao/runbook.md) — the sibling tool, and a key provider for state encryption; [Encryption & Key Management](../../architecture-patterns/encryption-and-key-management/learning.md) — what the encryption block is actually doing.
