# Event-Driven Architecture — Quick Reference

Core model: EDA inverts control flow — the producer announces a fact and doesn't know its consumers. That buys extensibility and temporal decoupling; it costs eventual consistency, deliberate causality instrumentation, and guarantees that become explicit work. This is the system-level layer over [outbox](../outbox-pattern/learning.md), [idempotency](../idempotency-and-delivery-semantics/learning.md), [CDC](../change-data-capture/learning.md), and [sagas](../saga-pattern/learning.md). Details in [learning.md](learning.md).

## The Three Patterns (Fowler)

| Pattern | Payload | Removes | Adds |
| --- | --- | --- | --- |
| Event notification | Thin ("order 991 placed") | Data coupling | **Runtime coupling** — callbacks need the producer up; breaks replay |
| **Event-carried state transfer** | Fat, self-contained | Runtime coupling | Schema contract, duplication, PII in transit |
| Event sourcing | The log *is* truth | — | A [storage commitment](../event-sourcing/learning.md), not just messaging |

**Default to state transfer** for integration events — a callback throws away the reason you went async.

## Event vs Command

| | Event | Command |
| --- | --- | --- |
| Tense | Past fact: `OrderPlaced` | Imperative: `ShipOrder` |
| Recipients | Unknown, any number | Exactly one correct handler |
| Transport | Pub/sub topic | Direct call or dedicated queue |

Test: if the producer would be unaffected by every consumer disappearing, it's an event. A command published as an event is RPC with extra hops.

## Broker Models

| | Log-based (Kafka/Redpanda) | Queue-based (SQS/RabbitMQ) |
| --- | --- | --- |
| Gives you | **Replay**, retention, many consumer groups, per-partition order | Per-message retry/delay/DLQ, fine-grained concurrency |
| Can't do well | Per-message operations (a stuck message blocks the partition) | History, replay, new-consumer backfill |
| Pick for | Rebuilding projections, onboarding consumers, entity ordering | Independent work items (send email, resize image) |

## Rules of Thumb

- **The partition key is the [shard key](../sharding/learning.md)** — same decision, same hot-partition failure modes. Key by the entity whose ordering matters.
- Ordering holds within a **partition, within a topic** — nothing more. Don't split mutually-ordered events across topics.
- Partition count caps consumer parallelism and is disruptive to change — over-provision.
- Mandatory envelope: `event_id`, `event_type` + `version`, `occurred_at`, **`correlation_id`**, **`causation_id`**, `partition_key`. Cannot be retrofitted onto history.
- Outbox on the producer, inbox/dedup on the consumer — at-least-once is a given.
- Registry with backward compatibility = breaking changes fail in CI, not at 3 a.m. Breaking change ⇒ new topic version + dual publish.
- Retention longer than your worst realistic outage (24 h retention is a tripwire).
- DLQ depth is a first-class alert — an unwatched DLQ is a data-loss queue with extra steps.
- Keep per-message processing well under the poll timeout, or rebalance storms.
- Queries stay synchronous; single-handler commands stay direct; only facts become events.

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Distributed monolith (events as RPC) | Naming test; explicit saga/process manager for workflows | Coupling of RPC + debuggability of async |
| Lost causality | correlation + causation ids, trace propagation via headers | Day-one only — old events can't gain ids |
| Assumed ordering | Key by entity, one topic per ordered set; version/sequence + tolerant consumers | Cross-topic and cross-partition have *no* order |
| Lag as a silent crisis | Alert on lag, **lag derivative**, and time-to-drain | No errors while the system drifts from reality |
| Poison message | DLQ + retry-vs-dead-letter classification | DLQ silently sacrifices per-key ordering |
| Event storms / cycles | Generated topology map in CI; hop-count TTL; separate backfill topics | Fan-out is free for the producer, costly for consumers |
| EDA where RPC belonged | Three-way test per interaction | Read-your-writes needs don't fit eventual paths |

## Diagnostic Signatures

| Symptom | Likely cause |
| --- | --- |
| Read spike on the producer whenever it publishes | Thin notifications with callbacks |
| Consumer sees effect before cause | Wrong/absent partition key, or split topics |
| Downstream data wrong but no errors anywhere | Consumer lag, unmonitored |
| One partition's consumer stuck, others fine | Poison message, no DLQ |
| "Why did this happen?" takes days | Missing correlation/causation ids |

## Key References

- Fowler, ["What do you mean by 'Event-Driven'?"](https://martinfowler.com/articles/201701-event-driven.html) — the taxonomy.
- Kleppmann, *DDIA* ch. 11 — logs, partitions, offsets, derived state.
- Stopford, *Designing Event-Driven Systems* (free) — the log-based view at length.
