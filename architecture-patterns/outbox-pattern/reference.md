# Outbox Pattern — Quick Reference

Core model: no transaction spans DB + broker (dual write). So: insert the message into an `outbox` table in the *same transaction* as the state change; a relay publishes committed rows with retry. Every failure degrades to duplicate-or-delay, never loss → consumers must be idempotent. Details in [learning.md](learning.md).

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| A business process depends on the message (orders, payments, sagas) | Loss is acceptable (metrics, telemetry) — direct publish is fine |
| Service commits to its own ACID database | No local ACID store to anchor the transaction |
| Consumers need intent-carrying domain events | Pure cache/search sync with no intent → direct CDC on tables is less machinery |
| | You're reaching for it as an event store — it records undelivered intent, not history |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Direct publish "just this once" | Only the relay holds producer credentials; publish API = outbox insert; lint for broker imports | Fails only under crash timing — invisible in tests |
| Polling load vs. latency trade | CDC/log-tailing relay (~ms, zero query load); else partial index on unpublished + batching + adaptive interval | Postgres replication slot abandoned → WAL fills disk |
| Parallel relay breaks per-aggregate order | Shard workers by `hash(aggregate_id)`; measure single-threaded batching first | `SKIP LOCKED` has no affinity; Kafka can't fix arrival order |
| Unbounded table growth | Decide retention day one: delete-on-publish / TTL job / partition drops; CDC same-txn delete trick | No functional symptom until the operational cliff |
| Payload = serialized domain object | Explicit versioned integration-event schema, mapped at write time | Internal refactors silently break other teams |
| Relay crash between publish and mark | By design → republish; consumers dedup on event id | Never "fix" it toward loss |

## Production Checklist

- [ ] Outbox insert wrapped in repository/UoW — domain code can't publish directly
- [ ] Publish key = aggregate_id (per-aggregate ordering); relay reads in commit order
- [ ] Consumers dedup (unique constraint on event id, same txn as effect)
- [ ] Retention/pruning policy implemented, not deferred
- [ ] Monitoring: unpublished depth + oldest-unpublished age
- [ ] CDC only: replication-slot lag/disk alerts
- [ ] Payload schema versioned and contract-tested with consumers
- [ ] One outbox per service database — never centralized/shared

## Key References

- Richardson, [Transactional Outbox](https://microservices.io/patterns/data/transactional-outbox.html).
- Morling/Debezium, [Outbox pattern with CDC](https://debezium.io/blog/2019/02/19/reliable-microservices-data-exchange-with-the-outbox-pattern/).
- Kleppmann, *DDIA* ch. 11.
