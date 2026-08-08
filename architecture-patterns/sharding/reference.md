# Sharding — Quick Reference

Core model: replication copies the same data; sharding splits different data. The **shard key** decides which queries are cheap (one shard), whether load spreads, and whether transactions stay local — and it's near-irreversible. Choose it from *measured access patterns*, not the data model. Details in [learning.md](learning.md).

## When to Use / When Not

| Shard when | Don't shard when |
| --- | --- |
| Measured single-node **write** or **storage** ceiling is binding | Read load is the problem → replicas |
| Working set exceeds what one machine can hold | Queries are slow → indexes (usually the actual issue) |
| You have an access-pattern table and a key that fits it | "To be ready" — you pay every cost immediately |
| You'll use logical shards + a managed sharding system | Vertical scaling / caching / archiving / functional split untried |

## Partitioning Strategies

| Strategy | Ordered scans | Write spread | Cost |
| --- | --- | --- | --- |
| **Range** | ✓ cheap | ✗ monotonic keys = one hot shard (fatal) | Needs bucketed prefix to be safe |
| **Hash** | ✗ scatter-gather | ✓ even by construction | Loses ordering; celebrity keys still skew |
| **Directory** | flexible | manual | Lookup on every request; a new critical dependency |

## Rules of Thumb

- Access-pattern table **first**, with percentages from a real query log; score candidate keys against it.
- Fixed **logical shards** (1024) → physical nodes: turns resharding into rebalancing. Costs nothing early, saves the crisis later.
- Consistent hashing + **vnodes**: topology change moves ~1/N, not ~everything.
- Never range-partition a monotonic column; if you must, prefix with `hash % 16`.
- Co-locate for **transactions** (correctness) first; solve queries with read models (performance) second.
- Topology map lives in a consensus store — two routers with different maps is *corruption*, not latency.
- One access path is cheap; every other one is paid for (global index, read model, or scatter-gather) — decide explicitly.
- Scatter-gather latency = max() over shards, worsening with shard count → precompute via CDC instead of optimizing.
- Small reference tables: replicate to every shard, don't shard them.
- Prefer a system that shards for you (Vitess, CockroachDB, Citus, DynamoDB) — app-level sharding means owning rebalancing, routing, and topology consistency.

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Hot shard (monotonic key / celebrity / skew) | Hash-bucket prefix; split hot key with random suffix + gather; dedicated shard for whales | Hashing spreads *keys*, not *load* — power-law access still skews |
| Key chosen from data model, not queries | Measured access-pattern table; two copies keyed differently if two paths must both be local | Scatter-gather tax paid forever, worsening as you add shards |
| Resharding under load | Logical shards + vnodes; per-shard move (copy → catch-up → freeze one shard → flip → verify) | Untested topology-change path is a first-time-in-prod operation |
| Cross-shard transactions | Co-locate via key; else saga + compensation | Same SQL keeps working until the rows land apart — nothing fails at deploy |
| Premature sharding | Exhaust indexes → vertical → replicas → cache → archive → functional split | Modern single nodes handle multi-TB and 10k+ writes/s |

## Migration Sequence (the part that's underestimated)

1. Dual-write to old + new — weeks
2. Backfill historical data, shard by shard, resumable — weeks
3. Continuous read-comparison verification — weeks
4. Flip reads shard-by-shard with instant rollback — days
5. Stop dual-write, decommission

## Key References

- Kleppmann, *DDIA* ch. 6 — the definitive treatment.
- [Vitess docs](https://vitess.io/docs/) — production sharding as a system.
- [Dynamo paper](https://www.allthingsdistributed.com/files/amazon-dynamo-sosp2007.pdf) — consistent hashing + vnodes.
