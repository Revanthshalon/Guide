# Caching Strategies — Learning Notes

## Mental Model

**A cache is a bet that staleness costs less than recomputation. Every cache decision is that bet, priced.** Which makes caches the loosest members of the family the [replication doc](../replication-and-consistency/learning.md) describes: a derived copy that lags its source, where you choose what to promise readers. Everything from that doc applies here — read-your-writes, monotonic reads, the anomalies lag produces — with one extra property that makes caching harder than replication: **a cache is allowed to be incomplete.** A replica missing data is broken; a cache missing data is just a miss. That freedom is what makes caches cheap, and it's also the source of their distinctive failure modes, because "miss" behavior under load is where caches collapse.

The value proposition is arithmetic and worth actually doing, because it's non-linear at the top:

```
effective latency = hit_rate × cache_latency + (1 − hit_rate) × origin_latency
80% hit rate, 1 ms cache, 100 ms origin → 20.8 ms   (~5× better)
95%                                     →  6.0 ms   (~17×)
99%                                     →  2.0 ms   (~50×)
```

Two readings. First, **the last few percent of hit rate carry most of the value** — going 95% → 99% is a bigger win than 80% → 95%, which is why hit-rate tuning is worth real effort. Second, and more sobering: **the miss path is the whole system's exposure.** At 99% hit rate your origin sees 1% of traffic — so it's probably provisioned for 1% of traffic, and any event that briefly drops the hit rate (a deploy that flushes the cache, mass key expiry, a cache node failing) sends 100× the expected load at an origin sized for a fraction of it. Most catastrophic cache incidents aren't stale-data bugs; they're **miss-storm** bugs, and they're covered in depth below.

Three framing rules for the rest:

1. **Correctness first, hit rate second.** A cache that serves wrong data quickly is worse than no cache. Decide what staleness each piece of data tolerates (in seconds, written down) *before* choosing TTLs — it's the same read-classification exercise as [replication's decision-vs-display reads](../replication-and-consistency/learning.md).
2. **Invalidation is the hard half.** Phil Karlton's joke ("there are only two hard things: cache invalidation and naming things") is load-bearing: computing what to cache is easy, knowing when it stopped being true is a distributed-systems problem, and the honest answers are few — expire it, be told by the source ([CDC](../change-data-capture/learning.md)), or make keys immutable so invalidation never happens.
3. **Caching is the same idea at every layer**, and you're probably running five of them: CPU caches ([hardware](../../performance-optimization/cache-locality/learning.md)), the database's buffer pool, an in-process map, a shared Redis, a CDN. They compose — and they also compound staleness, so knowing which layers a given piece of data passes through is part of the design.

## Core Concepts

### Cache-aside (lazy loading) — the default pattern

- **What it is:** The application owns the logic: on read, check cache; on miss, load from origin and populate the cache; on write, update the origin and *invalidate* (not update) the cache entry.
- **Why it exists:** It's the simplest pattern with the fewest failure modes: the cache is never in the write path, so a cache outage degrades to "slow but correct," and only requested data occupies memory. The invalidate-don't-update rule is subtle and important — writing the new value into the cache seems better but creates a race (two concurrent writers can interleave so the cache ends up with the older value permanently), whereas deleting means the next read repopulates from the source of truth.
- **Example:** `get_user(id)`: `redis.get(k)` → miss → `db.query(...)` → `redis.setex(k, ttl, v)` → return. On update: `db.update(...)` then `redis.del(k)`. Note the ordering — delete *after* the DB write commits, and consider deleting again after a short delay ("delayed double delete") if a concurrent read could have repopulated a stale value in the window between the DB write and the delete.

### Read-through, write-through, write-behind

- **What it is:** Read-through moves the load-on-miss logic *into* the cache layer (the app only ever talks to the cache). Write-through writes to cache and origin synchronously. Write-behind (write-back) writes to the cache and flushes to the origin asynchronously.
- **Why it exists:** Read/write-through centralize the caching logic so every caller gets it consistently — good when many services touch the same data and you don't trust each to implement cache-aside correctly. Write-behind is the one with a genuinely different risk profile: it makes writes fast by acknowledging before durability, which means **acknowledged writes can be lost** if the cache node dies before flushing — the same [dual-write and durability question](../outbox-pattern/learning.md) as everywhere else, and acceptable only where loss is tolerable (view counts, metrics) or the cache is itself durable.
- **Example:** Write-behind for a "last seen at" timestamp: losing 30 seconds of updates on a crash is fine, and it collapses a per-request DB write into a periodic batch — the [amortization](../../performance-optimization/batching-and-amortization/learning.md) win. Write-behind for account balances: absolutely not.

### Invalidation strategies

- **What it is:** How an entry stops being served. Four families: **TTL** (expire after N seconds — the source of truth is never consulted; staleness is bounded by the TTL, and that's the entire guarantee), **explicit invalidation** (the writer deletes the key — precise, but only as reliable as your ability to enumerate every affected key), **event-driven** ([CDC](../change-data-capture/learning.md) or domain events invalidate downstream — the writer doesn't need to know who caches what), and **immutable keys / versioning** (the key contains a version or content hash, so updating means writing a *new* key and old entries simply age out — invalidation is structurally impossible to get wrong).
- **Why it exists:** These are ordered by increasing correctness and increasing coupling. TTL requires no coordination but guarantees staleness. Explicit invalidation is precise but requires the writer to know every derived key — which fails silently the moment someone adds a new cached view and forgets to add its invalidation. Event-driven decouples that (the cache subscribes rather than the writer publishing to it), which is why it scales to many consumers. Immutable keys are the strongest: `user:42:v7` or `/static/app.a3f9c2.js` can never be stale, because a change produces a different key.
- **Example:** The versioned-key trick applied to a hard case: instead of invalidating every cached page containing a user's name, keep `user:42:version` (a counter bumped on any user change) and build cached page keys as `page:{id}:user:42:v{version}` — bumping the version orphans every derived entry at once, and they expire naturally. One write invalidates an unbounded set, with no enumeration.

### Eviction policies

- **What it is:** What gets dropped when the cache is full. **LRU** (least recently used) is the default; **LFU** (least frequently used) resists one-off scans polluting the cache; **TTL-only** (no capacity eviction — entries live until expiry); modern hybrids (W-TinyLFU in Caffeine, `allkeys-lru`/`volatile-lru` variants in Redis) mix recency and frequency.
- **Why it exists:** Eviction policy decides which working set survives under memory pressure, and the classic failure is LRU meeting a scan: a batch job touching a million cold keys evicts the hot working set, and the hit rate collapses for everyone until it refills. LFU or admission-policy caches (which refuse to admit a new entry unless it looks more valuable than the eviction candidate) resist this.
- **Example:** Redis `maxmemory-policy` is a genuinely consequential setting: `noeviction` (writes fail when full — makes the cache act like a database and surprises everyone), `allkeys-lru` (evict anything — the usual choice for a pure cache), `volatile-lru` (evict only keys with a TTL — dangerous if untTL'd keys grow unbounded). Choose it deliberately; the default may not be what you want.

### Cache placement: the layer decides the trade

- **What it is:** Where the cache lives. **In-process** (a `HashMap`/Caffeine/moka in the app): nanosecond access, no network, but *per-instance* — N app servers means N copies, N× origin load on cold start, and no coordinated invalidation. **Distributed** (Redis/Memcached): shared across instances (one copy, coordinated invalidation, higher hit rate), at the cost of a network hop (~0.5 ms) and a new critical dependency. **Client/CDN** (browser cache, edge): eliminates the request entirely — the best possible outcome — but invalidation is weakest and staleness longest.
- **Why it exists:** These compose into a hierarchy with the same shape as [CPU cache levels](../../performance-optimization/cache-locality/learning.md): small-fast-local backed by large-slower-shared. A two-tier setup (in-process L1 with a short TTL, Redis L2) is common and effective — L1 absorbs the hottest keys with zero network, L2 provides sharing and survives instance restarts. The cost is *compounded staleness*: an entry can be stale in L2 and staler in L1, so total staleness is the sum of both TTLs.
- **Example:** A permissions check hit 50 K times/second: L1 with a 5-second TTL takes 99% of it in-process; L2 (Redis, 60 s) serves the L1 misses; CDC-driven invalidation clears both on permission changes, with the 5-second L1 TTL as the backstop for anything the invalidation missed.

## Worked Example

A product page endpoint: 40 ms of database work per render, 8 K req/s at peak, product data changes rarely (a few hundred edits/day). Four stages, each fixing the failure the previous one introduced.

**Stage 0 — no cache.** 8 K × 40 ms of DB work: the database is saturated, p99 climbs, and every replica added buys linear (expensive) headroom.

**Stage 1 — cache-aside, 5-minute TTL.**

```
hit rate 98.5%, p50 1.2 ms, DB load down ~60×
```

Excellent — until the failure modes arrive.

**Stage 2 — the stampede** (the incident that teaches the topic). A popular product's entry expires at 14:03:00. In the ~40 ms before the first request repopulates it, ~320 concurrent requests all miss, all query the database for the same row, all compute the same result:

```
14:03:00.000  key expires
14:03:00.000–.040   ~320 concurrent misses → 320 identical DB queries
                    DB connection pool exhausted → latency spike across ALL endpoints
```

This is a **cache stampede** (thundering herd), and note its shape: the cache was working perfectly at 98.5% hit rate; the failure was in the *transition*. Two fixes, used together:

```rust
// 1. Single-flight: one loader per key, everyone else waits for its result.
let value = singleflight.get_or_load(key, || db.fetch(key)).await;   // 320 → 1 DB query

// 2. Jittered TTL: never let a cohort of keys expire simultaneously.
let ttl = base_ttl + rand::random_range(0..base_ttl / 10);
```

**Stage 3 — the avalanche.** A cache node restarts at 15:00; every key is gone at once; 8 K req/s hits a database provisioned for ~120 req/s. The origin falls over, requests time out, retries amplify the load, and the system cannot recover on its own even after the cache is back — a [metastable failure](../backpressure-and-rate-limiting/learning.md). Fixes are layered defense:

- **Serve-stale-while-revalidate:** keep entries past their TTL and return them (marked stale) while a background task refreshes — the origin sees a trickle instead of a flood, and users see slightly-old data instead of errors.
- **A [concurrency limiter](../backpressure-and-rate-limiting/learning.md) in front of the origin** (a semaphore capping in-flight loads) so the origin is *never* offered more than it can serve, whatever the cache does.
- **[Circuit breaker](../circuit-breaker/learning.md)** so an already-overwhelmed origin isn't hammered by retries.
- **Warmed restarts:** replicated cache nodes, or a warmup pass before a cold instance takes traffic.

**Stage 4 — correctness.** A price edit must not take 5 minutes to appear. Add [CDC-driven invalidation](../change-data-capture/learning.md): the products table's change stream deletes the affected keys within ~100 ms, with the TTL retained purely as a backstop for anything the pipeline misses. The final shape:

```
key:      product:{id}:v{version}         (versioned — structurally can't go stale)
L1:       in-process, 5 s TTL, jittered   (absorbs the hottest keys)
L2:       Redis, 5 min TTL, jittered      (shared, singleflight-protected)
invalidate: CDC on products table         (≈100 ms propagation)
protect:  singleflight + stale-while-revalidate + origin concurrency cap
```

Every element of that configuration exists because a specific failure mode demanded it — which is the honest way to arrive at a cache design.

## Pitfalls in Depth

### Pitfall: Cache stampede (thundering herd on a hot key)

- **What goes wrong:** A single popular key expires and every concurrent request for it misses simultaneously, sending N identical expensive queries to the origin. The origin — sized for the *cached* request rate — saturates, and the resulting latency spike affects endpoints that have nothing to do with the hot key.
- **Why it happens (the mechanism):** Cache-aside has an unguarded window between "entry expired" and "entry repopulated." Every request arriving in that window sees a miss, because nothing tells them a load is already in flight. The width of the window is the origin's latency, and the number of requests in it is `arrival_rate × origin_latency` — so *slower origins and hotter keys make it worse*, which is exactly backwards from what you want.
- **How to handle it in production, and why that works:** **Single-flight** (request coalescing): the first miss registers an in-flight load; concurrent missers await its result rather than issuing their own. This collapses N queries to 1 by construction and is the single highest-value cache defense — most languages have it off the shelf (`moka`'s `get_with`, Go's `singleflight`, Caffeine's `AsyncLoadingCache`). Add **probabilistic early expiration** (XFetch-style: as an entry nears expiry, requests randomly decide to refresh it early, with probability rising toward the TTL) so refreshes are staggered *before* the cliff rather than racing at it. And **jitter every TTL** so keys written together don't expire together.
- **Trade-offs of the fix:** Single-flight serializes concurrent misses for the same key — a slow origin load makes all waiters wait (bound it with a timeout, and pair with stale-while-revalidate so waiters can be served old data instead). Early expiration spends extra origin calls to buy smoothness. Both are cheap next to an origin outage.

### Pitfall: Cache avalanche (mass expiry or cache loss)

- **What goes wrong:** A large fraction of the cache disappears at once — a node restart, a flush during deploy, a `FLUSHALL`, or simply a cohort of keys created together expiring together — and the full uncached traffic volume hits an origin provisioned for a small fraction of it. Retries and timeouts amplify the load, and the system can stay collapsed even after the trigger passes.
- **Why it happens (the mechanism):** High hit rates mean the origin is *deliberately under-provisioned* relative to total traffic — that's the point of caching, and it's also a hidden coupling: the cache's availability has silently become the origin's capacity requirement. The mass-expiry variant is subtler: keys populated in a burst (after a deploy, or a warmup script) inherit identical TTLs and therefore expire in the same second, forever after, unless jittered.
- **How to handle it in production, and why that works:** Bound the load the origin can *ever* receive — a semaphore/concurrency limiter on origin calls means excess requests wait or fail fast rather than piling on ([backpressure](../backpressure-and-rate-limiting/learning.md) applied to the miss path), and a [circuit breaker](../circuit-breaker/learning.md) stops retry amplification. Make the cache survivable: replicated or clustered nodes so one failure isn't total loss, and a warmup phase before a cold instance takes traffic. Jitter all TTLs. Serve stale on origin failure — an old value beats an error for most read paths. Finally, *know your number*: measure what the origin can actually serve uncached and compare it to peak traffic; if the answer is "we'd fall over," that's a documented risk with an owner, not a surprise.
- **Trade-offs of the fix:** Concurrency limits mean some requests are rejected or delayed during recovery — a deliberate partial outage instead of a total one. Stale-serving means correctness relaxes exactly when the system is stressed (usually the right call, occasionally not — decide per data type). Replicated caches cost money for a component that is, by definition, rebuildable.

### Pitfall: Invalidation that misses entries (the correctness rot)

- **What goes wrong:** A write invalidates the obvious key but not the derived ones: `user:42` is deleted, but `user:42:profile_page`, the `team:7:members` list containing them, and the search index entry all keep serving the old name. The bugs are intermittent, hard to reproduce, and reported as "the site shows old data sometimes."
- **Why it happens (the mechanism):** Explicit invalidation requires the *writer* to know the complete set of cached representations derived from the data — a set that grows every time someone adds a cached view, in a different module, without knowing that this write path exists. It's a coupling that fails silently and gets worse over time; nothing tests it.
- **How to handle it in production, and why that works:** Prefer designs where the problem can't arise: **versioned keys** (bump `user:42:version`, and every derived key containing that version is orphaned at once — no enumeration required) or **event-driven invalidation** where caches *subscribe* to change events ([CDC](../change-data-capture/learning.md)/domain events) rather than writers publishing to each cache — the new cached view registers its own subscription, so adding one can't break the old ones. Always keep a **TTL as a backstop** even with precise invalidation: it converts "wrong forever" into "wrong for at most N seconds," which is the difference between a data-integrity incident and a latency footnote.
- **Trade-offs of the fix:** Versioned keys leave orphaned entries occupying memory until they expire (usually fine — that's what eviction is for). Event-driven invalidation adds a pipeline to operate and a propagation delay. Both beat maintaining a hand-written map of every derived key.

### Pitfall: Caching the wrong things (and negative results not at all)

- **What goes wrong:** Two mirror-image mistakes. **Caching low-value data** — entries with near-zero reuse (a per-request unique query), or data so cheap to compute that the network hop to Redis costs more than recomputing it — wastes memory and adds latency. **Not caching negative results** — every lookup for a nonexistent key misses and hits the origin, so a scan of invalid ids (a scraper, a bug, or an attack) bypasses the cache entirely and pounds the database (**cache penetration**).
- **Why it happens (the mechanism):** Caching decisions are made per-feature by whoever's building it, using intuition about "expensive" rather than the two numbers that matter: reuse rate and cost ratio. And negative caching feels wrong ("why store nothing?") until you notice that "this id doesn't exist" is a *fact* worth remembering, and that the miss path is the unprotected one.
- **How to handle it in production, and why that works:** Cache what is *expensive to produce × frequently requested* — measure both rather than assuming; a per-key hit-rate distribution usually shows a small hot set carrying nearly all the value and a long tail that's pure overhead (cache the head, skip the tail). For penetration: **cache negative results** with a short TTL (`user:99999 → NOT_FOUND`, 30 s), and for very large key spaces add a **Bloom filter** in front so a definitely-absent key is rejected without touching either cache or origin.
- **Trade-offs of the fix:** Negative caching needs invalidation when the entity *is* created (or a short enough TTL that the delay is acceptable) — otherwise you've cached "this user doesn't exist" for a user who just signed up. Bloom filters have false positives (they let some absent keys through — harmless) and must be rebuilt as the key set changes.

### Pitfall: Treating the cache as the source of truth

- **What goes wrong:** Data exists *only* in the cache — a session, a rate-limit counter, a computed aggregate never persisted — and everyone treats a cache flush as a performance event. Then a restart logs every user out, or resets every rate limiter to zero, or loses the counters finance was reporting from. Related: the cache becoming *load-bearing for correctness*, where a miss doesn't just cost latency but produces a wrong answer (a stale-but-authoritative permission check that fails open).
- **Why it happens (the mechanism):** The boundary erodes gradually. Redis is durable-ish, fast, and already deployed, so it accumulates state that was never designed to be ephemeral — each addition individually reasonable, collectively turning a rebuildable cache into an unbacked database with an eviction policy that deletes data under memory pressure.
- **How to handle it in production, and why that works:** Keep the invariant explicit and testable: **every cache entry must be reconstructible from a system of record** — and prove it by flushing the cache in staging under load as a routine drill. Data that fails the test isn't cache; it needs a real store (Redis *can* be that store with AOF persistence and replication, but then it's a database with an SLA, budgeted and operated accordingly — not "just the cache"). For fail-open/fail-closed decisions on the miss path, decide explicitly per data type: a permissions cache should fail *closed* (deny on miss+origin-down), while a recommendations cache should fail *open* (serve defaults).
- **Trade-offs of the fix:** Some genuinely ephemeral state (sessions) is legitimately cache-shaped — the point isn't to forbid it but to make "what happens when this is gone?" an answered question rather than a discovered one.

### Pitfall: Stale reads surprising the user who just wrote

- **What goes wrong:** A user edits their profile, the write commits, the cache still holds the old value, and the next page load shows their change reverted. They edit again. Support gets a ticket about "changes not saving."
- **Why it happens (the mechanism):** This is precisely the [read-your-writes violation](../replication-and-consistency/learning.md) from the replication doc, with the cache as the lagging replica — and it's *more* likely here, because cache TTLs are typically far longer than replication lag. Invalidate-on-write helps but has a race: a concurrent read can repopulate the cache from a read replica that hasn't received the write yet, re-caching the old value for a full TTL.
- **How to handle it in production, and why that works:** For the user's own data after their own write: bypass the cache for a short window (a session flag), or read through to the primary, or write the new value into the cache as part of the write path *when* the write path is single-writer enough to be safe. To close the repopulate-from-stale-replica race: invalidate *after* the write commits, and either read from the primary during repopulation or use the delayed-double-delete pattern (invalidate, wait past replication lag, invalidate again). And keep it scoped — only the *writer* needs this guarantee; everyone else can happily see the old value for a TTL.
- **Trade-offs of the fix:** Bypassing the cache for recent writers costs hit rate proportional to how many users are in their write window (usually tiny). Double-delete adds a delayed task and a small window of extra origin load. Both are far cheaper than the alternative of raising consistency guarantees globally.

## Design Decisions & Trade-offs

**Start with cache-aside plus TTL, and add mechanisms only as failure modes demand.** It's the pattern with the fewest ways to be wrong, and the worked example's progression (stampede → avalanche → correctness) is the order those demands usually arrive in. Every additional mechanism — single-flight, stale-while-revalidate, CDC invalidation, two-tier — should trace to a specific failure you either experienced or can argue is imminent.

**Choose TTL from a stated staleness budget, not from a feel.** "Prices may be up to 60 seconds stale" is a product decision with a business owner; "TTL = 300 because that seemed reasonable" is a latent incident. Write the budget per data class (reference data: hours; user profiles: minutes; prices/permissions: seconds or event-driven; balances: don't cache, or cache with versioning), then set TTLs from it.

**Prefer immutable/versioned keys wherever the data model allows.** They convert invalidation from a distributed-systems problem into a naming problem, and they compose beautifully with CDNs and long TTLs (the content-hashed asset URL is this pattern at its purest: cache forever, deploy a new name).

**In-process vs. distributed is a hit-rate-vs-freshness trade, and two tiers is often right.** In-process L1 gives nanosecond access and zero network, but N instances mean N copies with independent staleness and N× cold-start load. Distributed gives one coherent copy and coordinated invalidation at ~0.5 ms. If you use both, budget *summed* staleness and make sure invalidation reaches both tiers (or that L1's TTL is short enough not to matter).

**Size the origin for a realistic bad day, not for the cached rate.** The uncomfortable question — "what happens if the cache is empty at peak?" — has three acceptable answers: the origin can take it (rare), the concurrency limiter sheds load gracefully (usual), or there's a documented, owned, accepted risk (honest). "We haven't thought about it" is the one that becomes an outage.

**Instrument the things that predict incidents, not just hit rate.** Hit rate alone hides everything interesting: track per-key-prefix hit rates (find the useless caches and the hot keys), miss *latency* (the stampede's fingerprint is a latency spike concurrent with a miss cluster), origin load during cache events (deploys, restarts), eviction rate (rising evictions = undersized cache, hit rate about to fall), and staleness distribution where you can measure it.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Compute effective latency at 90%, 99%, and 99.9% hit rates (1 ms cache, 100 ms origin). Why is the top of the range where the value concentrates — and why is it *also* where the risk concentrates?
2. Why does cache-aside invalidate rather than update on write? Construct the interleaving that makes update-on-write wrong.
3. A key with 320 concurrent requests and a 40 ms origin: derive the stampede's size from arrival rate and origin latency, and explain why slower origins make it worse. Name the two fixes and what each costs.
4. Distinguish stampede from avalanche by trigger, blast radius, and fix. Why can a system stay collapsed after an avalanche's trigger has passed?
5. Explicit invalidation "works" and still rots over time. What's the mechanism, and what are the two designs that structurally prevent it?
6. What is cache penetration, why does it bypass your defenses entirely, and what are the two mitigations (one simple, one for large key spaces)?
7. A user's own edit appears to revert. Name the anomaly (using the replication doc's vocabulary), explain the repopulate-from-stale-replica race, and give the scoped fix.
8. State the test that determines whether something is a cache or a database. What's the drill that proves it?

Design exercises:

- Take one cached endpoint in a system you know and write its staleness budget, TTL derivation, invalidation strategy, and answer to "what happens if the cache is empty at peak?" Most such endpoints have none of these written down.
- Compute your origin's uncached capacity versus peak traffic, and identify which of the three acceptable answers you have. If it's the fourth, that's the finding.
- Add single-flight to one hot path and measure origin QPS during a forced key expiry, before and after — the collapse from N to 1 is the most satisfying graph in caching.

## Open Questions

- Probabilistic early expiration (XFetch) in practice: what does the parameterization look like on a real workload, and does it measurably beat plain jitter + single-flight?
- Redis vs. in-process (moka/Caffeine) crossover: at what request rate and entry size does the network hop stop being worth the shared-hit-rate gain? Measure on the actual latency budget.
- CDC-driven invalidation propagation delay in a real pipeline (Debezium → consumer → Redis DEL): p50/p99, and what TTL backstop that implies.
- Negative-caching TTLs for entities that can be created at any moment (a just-registered user): what's the standard reconciliation — event-driven clear, or just accept a 30 s window?
- Cache-key design for multi-tenant systems: how do teams prevent cross-tenant key collisions structurally (prefix conventions vs. separate namespaces/databases), and what has gone wrong historically?

## References

- Martin Kleppmann, *Designing Data-Intensive Applications*, ch. 1 and 5 — caches as derived data and the lag anomalies they inherit; the framing this doc builds on.
- Vattani, Chierichetti & Lowenstein, ["Optimal Probabilistic Cache Stampede Prevention"](https://cseweb.ucsd.edu/~avattani/papers/cache_stampede.pdf) (VLDB 2015) — the XFetch early-expiration algorithm with its derivation.
- [Redis documentation on eviction policies and keyspace notifications](https://redis.io/docs/latest/develop/reference/eviction/) — the `maxmemory-policy` semantics that decide your cache's behavior under pressure.
- Facebook, ["Scaling Memcache at Facebook"](https://www.usenix.org/system/files/conference/nsdi13/nsdi13-final170_update.pdf) (NSDI 2013) — leases, stale sets, and the thundering-herd defenses at extreme scale; still the best real-world caching paper.
- [Caffeine's W-TinyLFU design notes](https://github.com/ben-manes/caffeine/wiki/Efficiency) — why admission policy beats pure LRU, with the hit-rate data.
- Related topics in this repo: [Replication & Consistency](../replication-and-consistency/learning.md) (caches as the loosest replicas — all lag anomalies apply), [Change Data Capture](../change-data-capture/learning.md) (the drift-free invalidation feed), [Backpressure & Rate Limiting](../backpressure-and-rate-limiting/learning.md) + [Circuit Breaker](../circuit-breaker/learning.md) (what protects the origin on the miss path), [Sharding](../sharding/learning.md) (read models replacing scatter-gather are a cache by another name), [Cache Locality](../../performance-optimization/cache-locality/learning.md) (the same principles at hardware scale, where the eviction policy is silicon).
