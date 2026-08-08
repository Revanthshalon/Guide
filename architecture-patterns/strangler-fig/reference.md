# Strangler Fig — Quick Reference

Core model: replace a legacy system incrementally by intercepting calls at a boundary and redirecting slice by slice, until nothing routes to the old system and it is **deleted**. The product isn't the new system — it's **reversibility**: one irreversible event becomes many small revertible ones. Details in [learning.md](learning.md).

## When to Use / When Not

| Use it when | Don't when |
| --- | --- |
| Legacy is large, live, and must keep shipping during replacement | System is small enough to rewrite in weeks — the scaffolding costs more than the risk |
| Requirements live in the code, not in documents | The system is being decommissioned, not replaced — migrate data and switch off |
| An interception point exists or can be created cheaply | No seam can be created at any reasonable cost |
| You can commit to *finishing* (including deletion) | Nobody will own completion — you'll end with two systems forever |

## The Sequence

| Stage | Move | Risk |
| --- | --- | --- |
| 0 | Interception point, 100% → legacy (no-op deploy) | None — and everything depends on it |
| 1 | Read slice: shadow → diff → 1/10/50/100% | Low, fully revertible |
| 2 | Write slice: new owns, outbox-syncs back to legacy | Medium — data strategy required |
| 3 | Backfill history, flip reads, reverse sync off, repoint consumers | High — unknown consumers surface here |
| 4 | **Delete**: prove zero calls → remove code → drop route → drop tables | The step that realizes the benefit |

## Rules of Thumb

- **Sync direction decides whether you finish**: new-owns → sync-back-to-legacy makes ownership transfer monotonic. Legacy-owns → sync-forward never ends.
- Slice by **bounded context** (data moves with behavior), sequence by risk: reads before writes, low-traffic before high, internal before external.
- Sizing test: *can this be rolled back with a routing change and no data cleanup?* If not, it's too big or the data strategy isn't ready.
- Do one easy slice to build the machinery, then pull **hard slices earlier** than their value justifies — they won't get easier, your appetite won't grow.
- Shadow reads for a full business cycle; diff structurally, not by string equality. Every mismatch is a finding (new bug, or an undocumented legacy behavior callers depend on).
- Shadowed writes must have side effects suppressed. Shadowing doubles load on shared dependencies.
- Use [outbox](../outbox-pattern/learning.md)/[CDC](../change-data-capture/learning.md) for sync — never a dual write — and reconcile continuously.
- Every temporary state (shared DB, dual paths, ACL, flags, sync) gets an **expiry date and an owner**, or it's permanent.
- Track "slices deleted" separately from "slices migrated" — the gap is the debt being accrued.
- Assume the documented consumer list is incomplete; instrument legacy tables to find the rest.

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Never finishing** (the dominant failure) | Named owner for the whole migration, decommission date as a deliverable, **legacy feature freeze** | Marginal value falls as difficulty rises — it's an incentive problem, not a skill one |
| Shared DB never split | Slice by data ownership; freeze legacy schema; per-entity authority table | Behavior extraction shows progress; data extraction shows none |
| Slices too big | Weeks not months; rollback test as the sizing gate | "More efficient" is true about effort, false about risk |
| No real rollback | Test the revert before shifting traffic; keep legacy path *executable* for 1–2 weeks | Cleanup pressure peaks right when the risk window is still open |
| New system inherits old model | Anti-corruption layer from slice one, deleted when legacy is | Direct port is always the cheaper next step |
| Flip without verification | Shadow + structural diff over a full business cycle | Tests encode known requirements; prod traffic is the real spec |

## Key References

- Fowler, [StranglerFigApplication](https://martinfowler.com/bliki/StranglerFigApplication.html) — the origin.
- Newman, *Monolith to Microservices* — interception patterns and database decomposition (the hard part).
- Feathers, *Working Effectively with Legacy Code* — creating seams where none exist.
