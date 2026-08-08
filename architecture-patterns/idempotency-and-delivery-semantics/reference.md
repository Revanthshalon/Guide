# Idempotency & Delivery Semantics — Quick Reference

Core model: a timeout is unresolvable ambiguity → transport gives at-most-once (loss) or at-least-once (duplicates), never exactly-once. "Effectively-once" = at-least-once delivery + idempotent receivers. Ask of every arrow: what happens on duplicate? on loss? Details in [learning.md](learning.md).

## Per-Arrow Decision

| Situation | Choose | Mechanism |
| --- | --- | --- |
| Loss tolerable (metrics, cache warm) | At-most-once | Fire and forget, no machinery |
| Business effect, in-store | At-least-once + idempotent receiver | Key table / unique constraint / ledger, same txn as effect |
| External effect, duplicate OK (email) | Send-then-record | Accept rare double-send |
| External effect, duplicate forbidden (payout) | Record-then-send + verify | Query provider by your reference id on ambiguity |
| Operation is a set/upsert/delete | Natural idempotency | Absolute write + version guard (`WHERE version = n`) for races |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Retry middleware on non-idempotent calls | No retry without stated idempotency; per-endpoint config | Duplicates spike during incidents |
| Dedup key stored apart from effect (Redis + DB) | Key + effect commit in one transaction; insert-first claim via unique constraint | Check-then-act is a race; gaps are crash windows |
| Retry arrives while original in-flight | Three-state keys: claim `in_progress` before executing; duplicates wait/409 | Add lease + verify-before-reexecute for crashed workers |
| Broker "exactly-once" trusted globally | Dedup at terminal effect regardless; EOS is scoped optimization | SQS FIFO 5-min window; Kafka EOS ends at external calls; DLQ replays |
| Silent loss (ack-before-process) | Process-then-ack everywhere; confirmed sends; reconciliation job | Weakest hop sets the chain's guarantee; catch-and-continue = loss |
| Dedup window shorter than operational replays | Key on permanent business identity, or rebuildable targets (replay = rebuild) | Operators replay days; clients retry seconds |
| `x += delta` duplicated | Ledger keyed by business id, derive total; or dedup + delta in one txn | Deltas need dedup; absolutes need versioning |

## Production Checklist

- [ ] Every arrow labeled at-most-once / at-least-once in design docs
- [ ] Keys minted at source of intent (one click = one key, survives retries)
- [ ] Key recording atomic with effect (same store, same txn)
- [ ] In-progress state handled (concurrent duplicate cannot re-execute)
- [ ] Derived operations keyed deterministically (`order_id + step`)
- [ ] External effects: duplicate-vs-loss chosen and documented per effect
- [ ] Ack discipline audited hop-by-hop (process-then-ack, confirmed sends)
- [ ] Reconciliation job comparing source of truth to downstream effects
- [ ] Key-table pruning policy consistent with documented replay window

## Key References

- Stripe, [Designing robust and predictable APIs with idempotency](https://stripe.com/blog/idempotency).
- Kleppmann, *DDIA* ch. 8 & 11.
- Helland, *Life Beyond Distributed Transactions*.
