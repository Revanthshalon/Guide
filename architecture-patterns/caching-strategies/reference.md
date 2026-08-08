# Caching Strategies — Quick Reference

Core model: a cache is a bet that staleness costs less than recomputation — the loosest replica, so every [replication](../replication-and-consistency/learning.md) lag anomaly applies. Value concentrates at the top of the hit-rate curve; **so does risk** — at 99% hit rate the origin is provisioned for 1% of traffic, and every catastrophic cache incident is a miss-storm, not a stale-data bug. Details in [learning.md](learning.md).

## Hit-Rate Math (1 ms cache, 100 ms origin)

| Hit rate | Effective latency | Origin sees |
| --- | --- | --- |
| 80% | 20.8 ms | 20% of traffic |
| 95% | 6.0 ms | 5% |
| 99% | 2.0 ms | 1% ← origin sized for this |
| Cache empty | 100 ms | **100%** — the avalanche question |

## Patterns

| Pattern | Shape | Use when |
| --- | --- | --- |
| **Cache-aside** (default) | App: read→miss→load→populate; write→origin then **invalidate** | Almost always; cache never in the write path |
| Read/write-through | Cache layer owns load/store | Many services share the data; enforce consistency centrally |
| Write-behind | Ack from cache, flush async | Loss-tolerable data only (counters, last-seen) — acked writes *can* vanish |
| Stale-while-revalidate | Serve expired entry, refresh in background | Protects origin during load spikes; correctness relaxes under stress |

## Invalidation, weakest → strongest

| Strategy | Guarantee | Cost |
| --- | --- | --- |
| TTL | Staleness bounded by TTL, nothing more | Zero coordination; guaranteed staleness |
| Explicit delete | Precise *if* every derived key is enumerated | Rots silently as new cached views appear |
| Event-driven (CDC/domain events) | Caches subscribe; writers don't know consumers | Pipeline to operate + propagation delay |
| **Versioned/immutable keys** | Structurally cannot be stale | Orphans occupy memory until eviction |

## Rules of Thumb

- Invalidate, don't update, on write (update-on-write has a lost-update race).
- **Jitter every TTL** — keys born together expire together forever otherwise.
- **Single-flight** every miss path: N concurrent misses → 1 origin call. Highest-value defense in the doc.
- Keep a TTL backstop even with precise invalidation: turns "wrong forever" into "wrong for N seconds."
- Cache negative results (short TTL) — otherwise invalid-id scans bypass the cache entirely (penetration); Bloom filter for huge key spaces.
- Bound origin concurrency on the miss path (semaphore + circuit breaker) so the origin can *never* be offered more than it serves.
- Every entry must be reconstructible from a system of record — prove it by flushing in staging under load.
- Decide fail-open vs fail-closed per data type (permissions: closed; recommendations: open).
- Staleness budget in seconds, written down, per data class — TTLs derive from it, not from vibes.
- Two tiers (in-process L1 + Redis L2) compound staleness — budget the sum.
- Redis `maxmemory-policy` is consequential: `allkeys-lru` for a pure cache; `noeviction` makes it act like a database.

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Stampede (hot key expiry) | Single-flight + jitter + probabilistic early expiry | Size = arrival_rate × origin_latency — slower origins are worse |
| Avalanche (node loss / mass expiry) | Origin concurrency cap, circuit breaker, stale-serving, replicated cache, warm start | System can stay collapsed after the trigger passes |
| Invalidation misses derived keys | Versioned keys or event-driven subscribe; TTL backstop | Fails silently; worsens as new views are added |
| Cache penetration (nonexistent keys) | Negative caching + Bloom filter | Attack/scraper bypasses the cache completely |
| Cache became the source of truth | Flush drill in staging; if it fails, it needs a real store | Erodes gradually, one reasonable addition at a time |
| Writer sees stale own data | Bypass cache for writer's window; invalidate after commit; delayed double-delete | Concurrent read repopulates from a lagging replica |
| LRU + scan pollution | LFU / admission policy (W-TinyLFU) | One batch job evicts the whole hot set |

## Metrics That Predict Incidents

- Hit rate **per key prefix** (not global — global hides useless caches and hot keys)
- Miss latency (stampede fingerprint: latency spike concurrent with a miss cluster)
- Origin QPS during deploys/restarts
- Eviction rate (rising = undersized, hit rate about to fall)

## Key References

- Facebook, ["Scaling Memcache at Facebook"](https://www.usenix.org/system/files/conference/nsdi13/nsdi13-final170_update.pdf) — leases and herd defenses at scale.
- [XFetch paper](https://cseweb.ucsd.edu/~avattani/papers/cache_stampede.pdf) — probabilistic early expiration.
- [Redis eviction docs](https://redis.io/docs/latest/develop/reference/eviction/) — behavior under memory pressure.
