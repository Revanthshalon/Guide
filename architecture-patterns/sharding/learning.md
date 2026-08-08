# Sharding — Learning Notes

## Mental Model

**Replication makes copies of the same data; sharding splits different data across machines. They are orthogonal axes, and production systems use both — shards for capacity, replicas per shard for durability and reads.**

You shard when one machine can no longer hold or serve the data: the working set exceeds RAM, the write rate exceeds one primary's capacity, or the dataset exceeds one disk. Note what's *not* on that list — read load (add [replicas](../replication-and-consistency/learning.md)), slow queries (fix indexes), or ambition. Sharding is the most operationally expensive scaling move available, and the last one to reach for.

The single decision that dominates everything: **the shard key**. It determines which rows live together, and therefore which queries are cheap (one shard) versus expensive (all shards), whether load spreads evenly or piles onto one node, and whether transactions stay local or become [distributed workflows](../saga-pattern/learning.md). The shard key is also the most *irreversible* decision in the system — changing it means rewriting and physically moving every row while serving traffic. Most sharding pain traces back to a key chosen from what was convenient rather than from what the query patterns demanded.

Three consequences that reorganize how you think about a sharded system:

1. **Locality becomes a design product, not an accident.** In a single database, "join these two tables" is free. Sharded, it's free *only if both rows live on the same shard* — which happens only if you chose a key that co-locates them. You are now designing data placement the way you'd design a schema.
2. **Anything spanning shards degrades to a distributed problem.** A scatter-gather query's latency is the *slowest* shard's response ([the tail](../../performance-optimization/batching-and-amortization/learning.md), and it gets worse as shard count grows). A cross-shard write loses atomicity — you're in saga-and-compensation territory. Both are solvable; both are dramatically more expensive than the single-node versions they replaced.
3. **The same reasoning appears at every scale.** Partitioning by key to avoid contention is [NUMA-aware placement](../../performance-optimization/numa-awareness/learning.md) inside one box, [sharded counters](../../performance-optimization/false-sharing/learning.md) inside one process, and per-shard Raft groups inside one database. Sharding is the distributed-systems name for a pattern you've already met three times.

## Core Concepts

### Shard key and partitioning strategy

- **What it is:** The column (or derived value) whose value decides a row's shard, plus the function mapping key → shard. The three families: **range** (shard by key intervals — `A–F`, `G–M`; or by date), **hash** (shard by `hash(key) mod N` or a hash ring), and **directory** (an explicit lookup table mapping key → shard, consulted on every request).
- **Why it exists:** Each family trades the same two properties against each other. Range partitioning preserves **order**, so range scans (`WHERE created_at BETWEEN ...`) hit few shards — but any monotonic key (timestamps, auto-increment ids) sends *all new writes to the last shard*, the single most common sharding failure. Hash partitioning destroys order (range scans become scatter-gather) but distributes writes evenly by construction. Directory partitioning buys total flexibility — arbitrary placement, per-tenant moves, easy rebalancing — at the cost of a lookup on every operation and a new critical dependency to keep available and consistent.
- **Example:** Sharding orders by `created_at` (range): today's writes all land on one shard while the others sit idle — a *time-based* hot spot that no amount of hardware fixes. Sharding by `hash(order_id)`: writes spread perfectly, but "all orders for customer 42" now queries every shard. Sharding by `hash(customer_id)`: customer queries hit exactly one shard, writes spread — unless one customer is 5% of your traffic (see the hot-shard pitfall).

### Consistent hashing and virtual nodes

- **What it is:** Hash both keys *and* nodes onto a ring; a key belongs to the next node clockwise. Adding or removing a node reassigns only the keys in its arc — roughly `1/N` of the data — instead of the near-everything that `hash(key) mod N` reshuffles when `N` changes. **Virtual nodes** (each physical node owning many small arcs) fix consistent hashing's own weakness: with few nodes, arcs are unevenly sized and load is lumpy.
- **Why it exists:** Naive modulo hashing makes topology changes catastrophically expensive — going from 4 to 5 shards moves ~80% of rows. Since capacity changes are routine and node failures are not optional, minimizing data movement per topology change is what makes a hash-sharded system operable at all.
- **Example:** 4 nodes × 256 vnodes each = 1024 arcs. Adding a fifth node steals ~205 arcs spread across all four existing nodes — ~20% of data moves, from every node in parallel, rather than 80% moving in a stop-the-world reshuffle. This is why Cassandra, DynamoDB, and every modern hash-sharded store use vnodes.

### Logical shards (the over-provisioning trick)

- **What it is:** Map keys to a fixed, large number of **logical** shards (say 1024) chosen once, then map logical shards to physical nodes — a much smaller, mutable mapping. Scaling moves *logical shards* between nodes; the key→logical-shard function never changes.
- **Why it exists:** It converts resharding — the hardest operation in this document — into a *migration of whole logical shards*, which is mechanical, resumable, and requires no rehashing of individual rows. Pick a logical count you'll never exceed (1024 or 4096 is standard; it costs almost nothing while small) and the "we need to reshard" crisis becomes "we need to rebalance," which is a Tuesday.
- **Example:** Slack, Notion, and Vitess-based systems all do this. 1024 logical shards on 4 physical nodes = 256 each; growing to 8 nodes moves 128 logical shards, each a bulk copy-then-cutover, no row-level hashing anywhere.

### Routing: where the key→shard decision is made

- **What it is:** Three placements. **Client-side** (the application library knows the topology and connects directly — fastest, but every client must be updated on topology change). **Proxy** (a routing tier — Vitess, ProxySQL, Envoy — parses queries and forwards; adds a hop and a component to operate, centralizes the logic). **Coordinator-in-the-database** (any node accepts any query and forwards internally — Cassandra, CockroachDB; simplest for clients, costs an internal hop).
- **Why it exists:** The routing layer is where topology knowledge lives, and *someone* must hold it. The choice determines how a rebalance is rolled out (push new maps to every client vs. update one proxy fleet) and how much intelligence clients need. Whatever you pick, the topology map itself becomes critical state — usually held in a [consensus store](../consensus-and-leader-election/learning.md), because two clients with disagreeing maps write the same key to two different shards.
- **Example:** Vitess's `vtgate` is the proxy model at scale: applications speak ordinary MySQL, vtgate rewrites and routes, and shard splits happen behind it with the app largely unaware.

### Cross-shard operations

- **What it is:** Anything touching more than one shard: **scatter-gather queries** (fan out, merge results — aggregations, non-shard-key lookups), **cross-shard joins** (usually forbidden or emulated in the application), and **cross-shard writes** (which have no atomicity guarantee).
- **Why it exists:** Understanding the cost shape is what makes shard-key choice rational. A scatter-gather's latency is `max()` over shards, not `avg()` — so p99 latency degrades as shard count rises, and one slow shard slows every broad query. Cross-shard writes are the deeper problem: there's no transaction spanning independent databases, so you're choosing between two-phase commit (blocking, and rarely available), [sagas with compensation](../saga-pattern/learning.md), or redesigning so the write doesn't span shards.
- **Example:** "Sum today's revenue" on 64 shards = 64 queries, waiting for the slowest, and repeat for every dashboard refresh. The standard answer isn't a faster scatter-gather: it's a [materialized read model](../event-sourcing/learning.md) maintained by [CDC](../change-data-capture/learning.md) — precompute the aggregate, query one place.

### Secondary indexes in a sharded world

- **What it is:** Any lookup by a non-shard-key column. Two implementations: **local indexes** (each shard indexes its own rows — queries must scatter-gather, but writes stay local) and **global indexes** (a separate index structure sharded by the *indexed* column — queries hit one shard, but writes must update two shards, reintroducing cross-shard consistency).
- **Why it exists:** This is the shard key's cost made explicit: you get *one* cheap access path for free, and every other access pattern must be paid for. Deciding which queries deserve global indexes (and their write amplification) versus scatter-gather versus a denormalized read model is the bulk of post-sharding design work.
- **Example:** Orders sharded by `customer_id`. "Find order by `order_id`" is a scatter-gather — unless you build a global index (`order_id → shard`), which now must be written transactionally-ish with the order itself (or eventually, via [outbox](../outbox-pattern/learning.md), accepting a window where the index lags).

## Worked Example

An orders system outgrowing one Postgres instance: 4 TB, 12 K writes/s, hitting the write ceiling. The shard-key decision, run honestly.

**1. Enumerate the access patterns first** (this is the whole exercise — do it before considering any key):

```
A. get order by order_id              — 40% of reads
B. list orders for customer           — 45% of reads
C. orders by date range (ops/reports) — 10%
D. write new order                    — all writes
E. "revenue by region today"          — dashboards, low volume, high visibility
```

**2. Score the candidate keys against those patterns:**

```
key              A (by id)      B (by customer)   C (by date)     D (writes)
────────────────────────────────────────────────────────────────────────────
created_at       scatter        scatter           1 shard         ALL on one shard ✗
                                                                  (monotonic — fatal)
hash(order_id)   1 shard        scatter (45%!)    scatter         even ✓
hash(customer_id) scatter*      1 shard ✓         scatter         even, unless whales
```

`hash(customer_id)` wins because it makes the *largest* read class (B, 45%) single-shard and keeps writes even. Pattern A is recovered with a global index (`order_id → customer_id`) or by embedding the shard in the id itself — the standard trick: generate `order_id` with the customer's shard encoded in a prefix, so routing needs no lookup at all. Pattern C and E stop being shard queries entirely: they move to a read model fed by [CDC](../change-data-capture/learning.md), which is cheaper *and* faster than any scatter-gather would have been.

**3. Choose the physical layout:**

```
1024 logical shards  (fixed forever)
  → 8 physical nodes  (128 logical shards each, room to grow to 1024 nodes)
  → each node: 1 primary + 2 replicas  (sharding × replication, the orthogonal axes)
routing: proxy tier, topology map in etcd (consensus-backed — no split-brain maps)
```

**4. The migration** (the part people underestimate — it's most of the work):

```
1. dual-write: app writes to old DB and new sharded cluster       weeks
2. backfill:   copy historical rows, shard by shard, resumable    weeks
3. verify:     continuous comparison of reads from both           weeks
4. flip reads: shard-by-shard, with instant rollback              days
5. stop dual-write, decommission old primary
```

**5. What it cost, stated plainly:** cross-shard transactions are gone (an order touching two customers now needs a [saga](../saga-pattern/learning.md)); ad-hoc analytics against the primary are gone (they go to the read model); operational surface went from one database to 24 instances plus a proxy tier plus a topology store. The 12 K writes/s ceiling is gone. That's the trade, and it's only worth it because the ceiling was real.

## Pitfalls in Depth

### Pitfall: The hot shard (skew, celebrities, and monotonic keys)

- **What goes wrong:** One shard receives disproportionate load while others idle — the cluster's capacity is effectively one node's. Three common causes: a **monotonic shard key** (timestamps, sequential ids → all new writes to the newest shard), a **celebrity key** (one customer/tenant/product that is 10% of traffic — hash distributes *keys* evenly, not *load*), and **naturally skewed data** (a power-law tenant distribution, which is the normal case for B2B systems).
- **Why it happens (the mechanism):** Hashing guarantees uniform distribution of *distinct key values*, not of *requests*. If access frequency per key is power-law distributed — which it almost always is — a uniform key distribution still produces wildly non-uniform load. And monotonic keys defeat hashing's premise entirely: consecutive values differ, but they arrive *in order*, so range-partitioned systems concentrate them by construction.
- **How to handle it in production, and why that works:** Never range-partition on a monotonic column (or if range partitioning is required for scan locality, prefix the key with a hashed bucket: `(hash(id) % 16, created_at)` — 16 write hot spots instead of one, scans still ordered within a bucket). For celebrity keys: **split the hot key's space** — append a random suffix to shard a single logical entity across N sub-keys, aggregating on read (the [sharded-counter](../../performance-optimization/false-sharing/learning.md) pattern, one layer up); or give the whale its own dedicated shard (directory-based placement earns its keep here). Detect before it hurts: per-shard QPS/latency/storage dashboards with a *variance* alert, not just totals — an average that looks healthy hides the hot shard.
- **Trade-offs of the fix:** Hash-prefixing forfeits global ordering and complicates range queries. Key-splitting turns single-row reads into gather-and-merge. Dedicated shards mean manual placement — the directory model's operational burden. All of them beat the alternative, which is buying hardware for a cluster that's bottlenecked on one node.

### Pitfall: Choosing the shard key from the data model instead of the query patterns

- **What goes wrong:** The key is picked because it's the primary key, or because it's "the natural entity" — and then the dominant query turns out not to include it. Every read becomes a scatter-gather; p99 latency is the slowest of N shards; adding shards makes it *worse*, not better. The system now scales writes and de-scales reads.
- **Why it happens (the mechanism):** The shard key is chosen at design time, when access patterns are assumptions, and it's the one decision that can't be revised cheaply once data exists. Data-model thinking ("orders belong to customers, so shard by order") and query thinking ("we look up by customer 45% of the time") produce different answers, and only the second one matters to the machine.
- **How to handle it in production, and why that works:** Write the access-pattern table *first* (worked example step 1) with measured percentages from the current system's query log — not estimates. Score every candidate key against it. Accept that one access path will be cheap and the rest will need global indexes, denormalized read models, or scatter-gather, and decide *explicitly* which gets which. If two access patterns both need to be single-shard and no key satisfies both, that's the signal to store the data twice, keyed differently, synchronized by [CDC](../change-data-capture/learning.md) — deliberate denormalization beats a compromise key that serves neither.
- **Trade-offs of the fix:** Multiple copies keyed differently means eventual consistency between them and double the write path. That's a real cost, chosen consciously — versus a scatter-gather tax paid on every request forever.

### Pitfall: Resharding under load (the operation nobody rehearsed)

- **What goes wrong:** Growth demands more shards. With `hash mod N`, changing N remaps most rows: a full data reshuffle while serving traffic, with a correctness hazard the whole time (a request routed with the old map reads a row the new map has already moved). Teams discover mid-migration that there's no atomic cutover, no clean rollback, and no way to pause.
- **Why it happens (the mechanism):** Sharding schemes are designed and tested at their initial size; the *topology change* path is exercised for the first time in production, under pressure, on the largest dataset it will ever have faced. And modulo-based mapping makes the data movement maximal by construction.
- **How to handle it in production, and why that works:** Design for it up front: **fixed logical shards** (so key→logical never changes) plus **consistent hashing with vnodes** (so physical moves are ~1/N). Make the migration primitive be "move logical shard X from node A to node B," implemented as copy → catch-up via change stream → brief write-freeze on X only → flip the map → verify — resumable, per-shard, with the blast radius of one logical shard. Keep the topology map in a [consensus store](../consensus-and-leader-election/learning.md) so all routers see the flip atomically (two routers with different maps is a data-corruption bug, not a latency bug). Rehearse a shard move in staging *and* production during quiet hours before you need one.
- **Trade-offs of the fix:** Logical shards add an indirection level and a topology service to operate. Per-shard freezes mean a brief write-unavailability window for a slice of keys — usually acceptable, and vastly better than a global maintenance window.

### Pitfall: Cross-shard transactions attempted anyway

- **What goes wrong:** A transfer between two accounts on different shards is written as two updates and hoped for. A crash between them leaves money debited and not credited — silent, rare, and financially real. Or the team adopts two-phase commit and discovers its cost: a blocking protocol where a coordinator failure leaves participants holding locks indefinitely.
- **Why it happens (the mechanism):** Single-database transactions made atomicity free, and the code that relied on it didn't announce itself. Sharding removes the guarantee silently — the same SQL keeps working, right up until the two rows land on different shards. Nothing fails at deploy time.
- **How to handle it in production, and why that works:** First, **design the key so the transaction doesn't span shards** — co-locating the entities that change together is exactly what the shard key is for (the same "consistency boundary" reasoning as an [aggregate](../event-sourcing/learning.md), one level up). Where that's impossible, use a [saga with compensations](../saga-pattern/learning.md) and accept the intermediate visible state, or route the operation through a single-shard "coordinator entity" that owns the workflow. Audit for the pattern deliberately: grep for multi-entity transactions and check whether their entities share a shard key — the ones that don't are latent bugs.
- **Trade-offs of the fix:** Co-location constrains the shard key (and may conflict with load-spreading). Sagas add compensation design and eventual consistency. Both are honest costs; a silently non-atomic multi-shard write is not a cost, it's a defect.

### Pitfall: Sharding prematurely

- **What goes wrong:** A team shards at 50 GB and 500 writes/s "to be ready," and immediately pays every cost in this document — cross-shard joins gone, transactions constrained, operational surface multiplied, migrations required for every schema change — for a dataset a single well-indexed instance would serve for years on one machine.
- **Why it happens (the mechanism):** Sharding is the most *legible* scaling story ("we're web-scale"), and it's easier to justify in a design doc than the unglamorous alternatives. Meanwhile modern single-node capability is wildly underestimated: a commodity server with fast NVMe handles multiple TB and tens of thousands of writes/s given decent indexes and connection pooling.
- **How to handle it in production, and why that works:** Exhaust the cheaper ladder first, in order: index and query tuning (usually the actual problem), vertical scaling (a bigger instance is one afternoon and no architecture change), [read replicas](../replication-and-consistency/learning.md) for read load, [caching](../caching-strategies/learning.md) for hot data, archiving cold data out of the primary, and functional partitioning (splitting *services* by domain — different tables to different databases — which is far easier than splitting rows). Shard only when a *measured* single-node ceiling on writes or storage is the binding constraint. Then, when you do: pick the key from measured access patterns and use logical shards, because you'll only get one cheap shot at it.
- **Trade-offs of the fix:** Deferring means a future migration under more load with more data — which is real, and is exactly what fixed logical shards and a good key choice make survivable. The trade is "one hard migration later" versus "all the costs immediately, plus that migration anyway when the first key choice proves wrong."

## Design Decisions & Trade-offs

**Hash vs. range vs. directory.** Hash by default — even distribution is the property that's hardest to retrofit. Range when ordered scans dominate and you can prevent monotonic hot spots (bucketed prefixes). Directory when placement must be manual (per-tenant isolation, whale customers, regulatory data residency) and you'll accept operating a lookup service that's on every request path.

**Shard count: pick logical shards absurdly high, physical nodes from capacity.** 1024 logical shards costs nothing at 4 nodes and saves the resharding crisis at 40. The only real constraint is per-shard overhead (connections, files, memory) — which is why the indirection to physical nodes exists.

**Co-location is the shard key's real job.** Ask "what must be transactionally consistent?" and "what is queried together?" — if those answers agree, that's your key. When they conflict, co-locate for *transactions* (correctness) and solve queries with read models (performance) — reversing that priority is how systems end up with silently non-atomic writes.

**Sharding is orthogonal to replication, and you need both.** Shards give capacity; replicas per shard give durability and read scaling. Their failure modes compose: a shard's primary failing is [a failover](../replication-and-consistency/learning.md), a *shard* being unavailable is a partial outage — which means your application must degrade per-shard rather than fail globally (a design property, not an emergent one).

**Prefer a system that shards for you.** Vitess, CockroachDB, YugabyteDB, Citus, DynamoDB, and Cassandra have each spent years on rebalancing, routing, and topology consistency. Application-level sharding means owning all of it. Choose to own it only when the managed options genuinely don't fit — the honest comparison includes the engineer-years, not just the license cost.

**Know the exits.** Some datasets shouldn't be sharded: small reference tables (replicate them to every shard instead), and anything whose access is inherently global (move it to a search index or read model). "Which tables are *not* sharded, and why" belongs in the design doc alongside the key.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Distinguish sharding from replication in one sentence each, and explain why a production system uses both. What does each one scale that the other doesn't?
2. Why does `hash(key) mod N` make adding a node catastrophic, and what two mechanisms (one hashing-level, one indirection-level) fix it? Roughly what fraction of data moves under each?
3. Your key is `hash(customer_id)` and one customer is 8% of all traffic. Hashing was supposed to distribute evenly — why didn't it, and what are your three options?
4. A scatter-gather query across 64 shards: why is its latency the max rather than the average, and why does adding shards make it worse? What's the standard architectural answer instead of optimizing it?
5. Local vs. global secondary index: state the write-cost and read-cost of each, and the consistency problem global indexes introduce.
6. Two access patterns both need to be single-shard, and no key satisfies both. What's the resolution, and what does it cost?
7. List the five cheaper alternatives to exhaust before sharding, in the order you'd try them. What measurement justifies moving past all of them?
8. Why is "two routers with different topology maps" a data-corruption bug rather than a latency bug, and what component prevents it?

Design exercises:

- Take a real schema you know and build the access-pattern table (worked example step 1) with *measured* percentages from a query log — then score three candidate shard keys against it. The winner is usually not the primary key.
- Write the resharding runbook for that design: the exact sequence to move one logical shard between nodes, including the correctness argument for why no write is lost at cutover. If you can't write it, the design isn't finished.
- Audit an existing system for latent cross-shard transactions: find every multi-entity write and check whether the entities share a shard key. Each mismatch is a future silent-corruption bug.

## Open Questions

- Vitess vs. Citus vs. CockroachDB for a Postgres-shaped workload: what actually differs in the resharding story and the cross-shard query planner — trial one against a realistic dataset.
- Shard-per-tenant vs. hash-by-tenant for B2B SaaS: at what tenant-count and skew does the isolation of dedicated shards beat the efficiency of hashing?
- Global secondary index consistency in practice: does anyone build them transactionally, or is CDC-with-lag the universal answer? What lag do real systems tolerate?
- The "encode the shard in the id" trick: what does it cost when a row must eventually *move* shards (tenant migration, rebalancing) — is the id then lying?
- How do per-shard Raft-group systems (CockroachDB, TiKV) handle a transaction spanning shards, and what's the measured latency penalty versus a single-shard write?

## References

- Martin Kleppmann, *Designing Data-Intensive Applications*, ch. 6 ("Partitioning") — the definitive treatment: partitioning strategies, secondary indexes, rebalancing, and request routing; read it beside this doc.
- [Vitess documentation](https://vitess.io/docs/) — production sharding as a system: vindexes (shard-key abstractions), resharding workflows, and the proxy model, all documented by people who ran it at YouTube scale.
- DeCandia et al., ["Dynamo: Amazon's Highly Available Key-value Store"](https://www.allthingsdistributed.com/files/amazon-dynamo-sosp2007.pdf) (SOSP 2007) — consistent hashing with virtual nodes in its original industrial context.
- Karger et al., "Consistent Hashing and Random Trees" (STOC 1997) — the original algorithm, if you want the derivation rather than the application.
- Slack Engineering and Notion Engineering blog posts on their sharding migrations — the honest operational accounts of the dual-write/backfill/verify/cutover sequence, including what went wrong.
- Related topics in this repo: [Replication & Consistency](../replication-and-consistency/learning.md) (the orthogonal axis — copies vs. splits), [Consensus & Leader Election](../consensus-and-leader-election/learning.md) (topology maps and per-shard leader election), [Saga Pattern](../saga-pattern/learning.md) (what cross-shard transactions become), [Change Data Capture](../change-data-capture/learning.md) (feeding the read models that replace scatter-gather), [Event Sourcing](../event-sourcing/learning.md) (the aggregate boundary as a natural shard key), [NUMA Awareness](../../performance-optimization/numa-awareness/learning.md) + [False Sharing](../../performance-optimization/false-sharing/learning.md) (the same partitioning logic at hardware scale).
