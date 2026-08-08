# Event Sourcing & CQRS — Quick Reference

Core model: `state = fold(apply, initial, events)` — the append-only log is truth; all state (aggregates, read models) is a rebuildable cache. Details in [learning.md](learning.md).

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| The business cares how state came to be (money, inventory, legal, workflow) | The domain is plain CRUD with no interesting history |
| Audit trail must be the system of record, not a side table | Team hasn't operated eventual consistency before and the domain doesn't force it |
| Temporal queries / replay / retroactive fixes have real value | You only want CQRS read scaling (do CQRS without ES) |
| Per-context adoption — event-source the core, CRUD the satellites | You'd be applying it system-wide by default |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Schema evolution vs. immutable events | Additive changes with read-time defaults; upcasters (v1→v2→v3) at one choke point | Upcaster chains need tests against archived real events |
| Dual write (store + publish) | Event store is the only write; publication tails the store (subscription/CDC) | Delivery becomes at-least-once → consumers must be idempotent |
| Projection double-apply on redelivery | Update + checkpoint in one transaction; else idempotent handlers (set, not increment) | Increments are the classic corruption |
| Stale reads after write | Read-your-own-writes: return stream version, wait for projection checkpoint ≥ version | Never validate commands against a projection |
| Cross-aggregate uniqueness (email) | Reservation stream per value, or detect-and-compensate | A projection lookup check is a race, not a guarantee |
| Aggregate too big / too small | Boundary = smallest cluster enforcing a real invariant | Signals: version-conflict rate (too big); invariants leaking into sagas (too small) |
| Unbounded streams | Close the books: end stream per period, successor seeded with summary event | Under-specified closing event = silent data loss |
| GDPR vs. immutability | Crypto-shredding (destroy per-user key) or PII outside events | Decide before the first PII-bearing event; remember backups |
| Version conflict under concurrency | `append(stream, expected_version)` + rehydrate-and-retry loop | Re-validate after retry — the command may now be invalid |

## Production Checklist

- [ ] Event versioning strategy decided before first event written (additive rules + upcaster slot)
- [ ] PII strategy decided (crypto-shred or PII-out-of-events)
- [ ] Publication path has no dual write (subscription or CDC off the store)
- [ ] Every projection idempotent or transactionally checkpointed
- [ ] Global ordering for subscriptions is gap-free (single writer / solved by store)
- [ ] Internal events not exposed as integration contracts — translation at context boundary
- [ ] Version-conflict retry loop with bounded attempts; conflict rate monitored
- [ ] Projection lag monitored; read-your-writes for screens that need it
- [ ] Rebuild-from-zero for each projection tested in anger
- [ ] Snapshots only where rehydration latency measured as a problem

## Key References

- Greg Young, *Versioning in an Event Sourced System* — read before the first schema.
- Kleppmann, *DDIA* ch. 11 — logs, ordering, derived state.
- [Kurrent docs](https://docs.kurrent.io/) — reference semantics for streams/subscriptions.
