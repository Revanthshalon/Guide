# Change Data Capture — Quick Reference

Core model: the database's WAL/binlog is already a complete, ordered change log — CDC tails it and streams every committed insert/update/delete downstream, with zero application change. Row *effects*, never intent. Details in [learning.md](learning.md).

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| Consumer wants state copies: search index, cache, warehouse, replica | Consumer needs business meaning (refunds, emails, workflow) → authored events via outbox |
| Deletes and every intermediate state must be captured | Polling on `updated_at` would genuinely suffice and log access is unavailable |
| Source app can't/shouldn't be modified | You're deriving intent from diffs (`if before.x != after.x`) — that's inference rot |
| Rebuildable downstream views are the goal | You need cross-table atomicity downstream without buffering machinery |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Row-diffs wired to business logic | Technical replication → CDC; business reactions → outbox events | Backfills/migrations/admin edits fire fake "events" |
| Forgotten slot fills primary disk | Slot-lag alerts + `max_slot_wal_keep_size` + slots lifecycle-managed as code | Slot invalidation = consumer re-snapshot (correct sacrifice) |
| DDL breaks all consumers at once | Schema registry with compatibility rules; topic versioning for breaks; captured tables reviewed as APIs | Nothing tells the DBA who's downstream |
| Header-without-lines downstream | Idempotent upserts + tolerate eventual join; transaction-metadata buffering only where it truly matters | States that never existed in the source |
| Giant initial snapshot (long txn, unresumable) | Incremental/watermark snapshots (DBLog-style); or restore-backup + stream-from-LSN | Consumers must upsert: snapshot and stream events interleave per key |
| Duplicates on connector restart | Sinks upsert by key or dedup by source position (LSN) | At-least-once is the contract; plan for it |

## Production Checklist

- [ ] Log-based capture (not triggers/polling) wherever the platform allows
- [ ] `REPLICA IDENTITY FULL` set before before-images are needed
- [ ] Slot lag alerting with thresholds far below disk capacity
- [ ] `max_slot_wal_keep_size` set; invalidated-slot re-snapshot runbook written
- [ ] Slots created/destroyed by the same automation as connectors
- [ ] Schema registry compatibility mode enforced on captured topics
- [ ] Sinks idempotent (upsert by natural key)
- [ ] Event key = row PK (per-key ordering preserved)
- [ ] 6-hour-outage answer written: WAL headroom + catch-up burst capacity
- [ ] Downstream documented as eventually consistent (lag visible to its readers)

## Key References

- Kleppmann, *DDIA* ch. 11 + "Turning the Database Inside-Out."
- [Debezium docs](https://debezium.io/documentation/) — Postgres connector page, end to end.
- Netflix, *DBLog* — incremental snapshot algorithm.
