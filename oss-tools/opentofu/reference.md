# OpenTofu — Quick Reference

Core model: reconciles three things — **config** (desired), **state** (what tofu believes exists), **reality** (what the API reports). Every problem is a divergence among them. MPL-licensed Linux Foundation fork of Terraform 1.5.x; `tofu` mirrors `terraform`. Details in [learning.md](learning.md); setup and operations in [runbook.md](runbook.md).

## Quick Facts

- **Alternative to:** HashiCorp Terraform
- **License:** MPL 2.0 (Terraform is BUSL 1.1 since Aug 2023)
- **Backed by:** Linux Foundation, open TSC
- **Registry:** registry.opentofu.org (mirrors the community provider namespace)
- **Killer feature:** client-side **state encryption** (Terraform has no built-in equivalent)
- **Sibling fork:** [OpenBao](../openbao/learning.md) (Vault, same licensing event)

## Comparison

| Aspect | Terraform | OpenTofu |
| --- | --- | --- |
| Core loop (HCL, providers, state, plan/apply, backends) | ✓ | ✓ drop-in |
| Providers | Same plugin protocol — work with both | ✓ |
| **State encryption (client-side)** | ✗ | **✓** with KMS / OpenBao key providers |
| Early variable evaluation (module source, backend config) | Limited | ✓ |
| `for_each` on providers | ✗ | ✓ |
| `-exclude` flag | ✗ (`-target` only) | ✓ |
| Post-1.5 Terraform features (test framework, stacks…) | ✓ | Per-feature check |
| Managed platform | Terraform Cloud | Third-party (Spacelift, env0, Scalr) or self-hosted |

## Divergence Classification (name it, then fix it)

| Symptom | Divergence | Fix |
| --- | --- | --- |
| Plan shows pending changes | config ≠ state | `apply` |
| Unexpected diff after nobody changed config | reality ≠ state (**drift**) | Fix reality, update config, or `ignore_changes` |
| Resource exists in cloud, no plan mentions it | unmanaged | `import` block |
| Plan wants to create something that exists | state lost/removed | Restore state version, or import |
| Plan destroys + recreates after a rename | address changed | `moved` block |

## Common Commands

```sh
tofu init                        # providers + backend; writes .terraform.lock.hcl (COMMIT IT)
tofu init -lockfile=readonly     # CI: fail if lock would change
tofu fmt -recursive / validate
tofu plan -out=tfplan            # save it
tofu show tfplan / -json tfplan  # review / policy-check
tofu apply tfplan                # apply the REVIEWED artifact (fails if state moved — a feature)
tofu plan -refresh-only -detailed-exitcode   # drift check; exit 2 = drift
tofu apply -replace=aws_instance.web         # deliberate recreate (replaces old `taint`)
tofu state list / show / mv / rm / pull / push
tofu force-unlock <ID>           # only after confirming no apply is running
```

## Rules of Thumb

- **Pin providers + commit `.terraform.lock.hcl`**; CI runs `-lockfile=readonly`.
- Save the plan, review it, apply *that artifact* — never `-auto-approve` on production merges.
- `sensitive = true` redacts **output only** — the value is in state in plaintext. Encrypt state; better, keep secrets out of it entirely.
- `encryption` block needs `enforced = true` on **both** `state` and `plan` (plan files leak the same values).
- Bucket **versioning** is what makes corrupt state recoverable — one setting, enormous payoff.
- `moved` blocks for every rename/extraction; `for_each` over `count` (key-addressed, not index-addressed).
- Split state by blast radius and rate of change; cross-layer references via provider **data sources**, not `terraform_remote_state`.
- `depends_on` only for hidden dependencies — references create edges for free and can't go stale.
- Drift detection on a schedule, not at apply time.
- Any plan with `destroy`/`replace` on a stateful resource needs justification in the PR.

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| State loss / corruption / concurrent writes | Remote backend + locking + versioning; `state pull` backup before surgery | Local state is the default — nothing warns you |
| Secrets in state | State encryption + restricted backend access + secrets out of OpenTofu | `sensitive` protects nothing at rest |
| Monolithic state | Split by layer; data sources for cross-references | Habitual `-target` use is the symptom |
| Drift from console changes | Scheduled `-refresh-only`; `ignore_changes` for legitimate external mutation | An apply silently reverts an emergency fix |
| Rename → destroy/recreate | `moved` blocks; `for_each` not `count` | A pure refactor proposing to drop your database |
| Auto-apply unreviewed plans | Saved plan artifact + approval gate + policy check on plan JSON | The reviewed plan and the executed one differ |
| Unpinned providers | `~>` constraints + committed lock file | Clean yesterday, rebuild today |

## Migration from Terraform

1. Inventory risk: post-1.5 Terraform features, tools hardcoding `terraform`, Terraform Cloud dependency
2. **Check version compatibility** — state from newer Terraform may be unreadable; going back can be impossible
3. Trial: `tofu init && tofu plan` on existing state — an **empty plan is the green light**
4. Back up state (versioned bucket + explicit `state pull`)
5. Migrate environment by environment, lowest stakes first; update CI in the same change
6. Adopt state encryption as a *separate*, independently revertible change

## Key References

- [OpenTofu docs](https://opentofu.org/docs/) — especially [state encryption](https://opentofu.org/docs/language/state/encryption/).
- [Migration guide](https://opentofu.org/docs/intro/migration/) — version compatibility.
- Brikman, *Terraform: Up & Running* — the shared conceptual model; translate the CLI name.
