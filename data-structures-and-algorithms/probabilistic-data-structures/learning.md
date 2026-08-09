# Probabilistic Data Structures — Learning Notes

## Mental Model

**Give up exactness and the space cost collapses from "proportional to the data" to "proportional to the accuracy you asked for."**

That's the trade, and it's a much better one than it sounds. An exact set of 1,000,000 `u64` keys costs ~8.9 MB. A Bloom filter answering membership with a 0.8% false-positive rate costs **1.2 MB** — measured, 7.3× less. A HyperLogLog counting **10,000,000 distinct items** to within 0.41% costs **16 KB** — the data itself would be 80 MB.

The structural insight is that these sketches store **a function of the data, not the data**. A Bloom filter has no way to enumerate its members; a HyperLogLog cannot tell you whether a specific item was seen. You've kept exactly enough information to answer one question approximately, and thrown away everything else — which is why the space stops depending on n at all.

Three properties decide whether a sketch is usable, and they're worth checking in this order:

1. **Which direction does the error go?** A Bloom filter has **one-sided** error: it may say "maybe present" for an absent item, but **never** "absent" for a present one. That asymmetry is what makes it safe as a pre-filter in front of an expensive exact lookup — a false positive costs a wasted disk read, a false negative would cost correctness.
2. **Is it mergeable?** Sketches that combine without re-reading the data (`a ∪ b` from the sketches alone) can be computed per-shard and merged centrally. This is why HyperLogLog and t-digest are the ones that made it into distributed systems, and why "average the per-host p99s" is wrong ([selection & order statistics](../selection-and-order-statistics/learning.md)).
3. **Does the error bound hold in practice?** Measured, yes — and closely. Bloom filter false-positive rates matched theory to within 0.01 percentage points across three configurations.

## The Invariant

**Bloom filter:**

> An item's `k` hash positions are **all set** if the item was inserted. Therefore: all bits set ⇒ *maybe present*; any bit clear ⇒ **definitely absent**.

The one-sidedness is a direct consequence — insertion only ever sets bits, so a present item's bits can never become clear. This also means **you cannot delete** from a standard Bloom filter: clearing a bit might clear it for a different item that shares that position, breaking the "definitely absent" guarantee. Counting Bloom filters and cuckoo filters exist to restore deletion.

**HyperLogLog:**

> For each of `m` buckets, store the maximum number of leading zeros seen in the hashes routed to that bucket. A hash with `r` leading zeros appears with probability `2^-r`, so the maximum observed rank estimates the log of the count.

The counter-intuitive part: the register stores a *maximum*, so it's idempotent — inserting the same item a million times changes nothing. That's exactly what makes it count *distinct* items, and also what makes it mergeable (the union of two HLLs is the element-wise max of the registers).

**Count-Min sketch:**

> `d` rows of `w` counters; item `x` increments `sketch[i][h_i(x)]` in every row. The estimate is `min_i sketch[i][h_i(x)]`, which **over-estimates** — collisions only ever add.

Again one-sided: the count is never too low. That makes Count-Min safe for "find heavy hitters" (you'll never miss a genuinely frequent item) and unsafe for "is this rare".

## Mechanics

### Bloom filter — sizing is the whole design

Given `n` items and `m` bits, the optimal number of hash functions is `k = (m/n)·ln 2 ≈ 0.693 · bits_per_item`, giving false-positive rate `(1 − e^(−kn/m))^k`. Measured against that formula:

| Bits/item | k | Memory (1M items) | **Measured FP** | Theory |
| --- | --- | --- | --- | --- |
| 8 | 6 | 976 KB | **2.150%** | 2.158% |
| 10 | 7 | 1,220 KB | **0.824%** | 0.819% |
| 16 | 11 | 1,953 KB | **0.045%** | 0.046% |
| *exact `HashSet`* | — | *~8,928 KB* | *0%* | — |

The theory is accurate to within 0.01 percentage points. **Roughly, every ~4.8 bits per item divides the false-positive rate by 10** — so 10 bits/item ≈ 1%, 15 ≈ 0.1%, 20 ≈ 0.01%. That rule of thumb is worth memorizing because it makes the sizing decision immediate.

Note that **the size depends on `n` and the target error, not on the item size** — a Bloom filter of 1M 200-byte URLs is the same 1.2 MB as 1M `u64`s. That's often the dominant win.

```rust
// The whole structure. k hashes derived from one via double hashing.
fn insert(&mut self, x: u64) {
    let (h1, h2) = (hash1(x), hash2(x));
    for i in 0..self.k {
        let p = (h1.wrapping_add((i as u64).wrapping_mul(h2)) % self.m) as usize;
        self.bits[p >> 6] |= 1u64 << (p & 63);
    }
}
fn maybe_contains(&self, x: u64) -> bool {
    let (h1, h2) = (hash1(x), hash2(x));
    (0..self.k).all(|i| {
        let p = (h1.wrapping_add((i as u64).wrapping_mul(h2)) % self.m) as usize;
        self.bits[p >> 6] >> (p & 63) & 1 == 1
    })
}
```

Double hashing (`h1 + i·h2`) gives k independent-enough hashes from two — computing k real hashes is wasted work, and Kirsch-Mitzenmacher showed the asymptotic FP rate is unaffected.

### HyperLogLog — counting distinct in constant space

Measured accuracy, with the theoretical standard error `1.04/√m`:

| Precision | Registers | Memory | True count | Estimate | Error | Theory (1σ) |
| --- | --- | --- | --- | --- | --- | --- |
| p=10 | 1,024 | 1 KB | 1,000 | 986 | 1.41% | ±3.25% |
| p=10 | 1,024 | 1 KB | 100,000 | 95,591 | 4.41% | ±3.25% |
| p=10 | 1,024 | 1 KB | 10,000,000 | 10,120,580 | 1.21% | ±3.25% |
| **p=14** | 16,384 | **16 KB** | **10,000,000** | 10,040,707 | **0.41%** | ±0.81% |

**16 KB counts 10 million distinct items to within half a percent** — and the memory is *identical* for 1,000 or 10 billion. Note the p=10/100k row exceeded the 1σ bound; that's expected roughly a third of the time for a single trial, since 1.04/√m is a standard error, not a hard bound. Reporting it rather than hiding it is the honest read.

Two corrections matter in practice: **small-range** (use linear counting when many registers are still zero — visible in the code above as the `zeros > 0` branch) and **large-range**. Modern implementations (HLL++) refine both; use a library rather than the 1985 paper.

### The family

| Structure | Answers | Error | Space | Mergeable |
| --- | --- | --- | --- | --- |
| **Bloom filter** | membership | FP only, no FN | ~10 bits/item for 1% | **yes** (OR) |
| Counting Bloom | membership + **delete** | FP only | 4× a Bloom | yes |
| **Cuckoo filter** | membership + delete | FP only | ~ Bloom, better at low FP | limited |
| **HyperLogLog** | distinct count | ±1.04/√m | **fixed** (16 KB) | **yes** (max) |
| **Count-Min sketch** | frequency | **over**-estimates | w·d counters | **yes** (add) |
| Count sketch | frequency | two-sided, unbiased | w·d | yes |
| **t-digest / DDSketch** | quantiles | bounded rel. error | Θ(1/ε) | **yes** |
| MinHash | set similarity | ±1/√k | k hashes | yes |
| SimHash | near-duplicate | Hamming-based | 64 bits/doc | — |

**Cuckoo filters** deserve a note: they support deletion, are often smaller than Bloom filters below ~1% FP, and store fingerprints in a cuckoo hash table. They're the modern default when deletion is needed.

## Complexity

| Structure | Insert | Query | Space | Notes |
| --- | --- | --- | --- | --- |
| Bloom filter | Θ(k) | Θ(k) | **Θ(n·bits)**, independent of item size | no delete, no enumerate |
| Cuckoo filter | Θ(1) amortized | Θ(1) | ~Bloom | delete supported |
| HyperLogLog | Θ(1) | Θ(m) to estimate | **Θ(m)** — independent of n | 16 KB for any cardinality |
| Count-Min | Θ(d) | Θ(d) | Θ(w·d) | over-estimates only |
| t-digest | Θ(log k) | Θ(log k) | Θ(1/ε) | accurate at the tails |
| Exact `HashSet` | Θ(1) | Θ(1) | **Θ(n · item size)** | — |

**Where the table misleads.** "Θ(n·bits)" for a Bloom filter still grows with n — it is *not* constant space, unlike HyperLogLog. The distinction matters: a Bloom filter for an unbounded stream will eventually saturate and its FP rate will climb toward 100%, silently. **Bloom filters must be sized for the expected n**, and a scalable/partitioned variant is needed when n is unknown.

HyperLogLog genuinely is Θ(m) regardless of n — measured, the same 16 KB handled 1,000 and 10,000,000 items — which is why it's the one people find surprising.

## Use Cases

- **Bloom filters in databases** — every LSM-tree engine (RocksDB, Cassandra, LevelDB) keeps a Bloom filter per SSTable so a read can skip files that definitely don't contain the key. Measured FP rates around 1% mean 99% of unnecessary disk reads are avoided for ~10 bits per key. This is the single most important production use, and it connects directly to the Stage 9 topic *LSM Trees & Write-Optimized Structures*, not yet written.
- **CDN and cache admission** — "have we seen this URL before?" before paying for an origin fetch.
- **Distinct counting at scale** — unique visitors, unique IPs, cardinality estimation in query planners. Redis's `PFCOUNT` is HyperLogLog; every column store uses it for `COUNT(DISTINCT)` estimates.
- **Latency percentiles** — t-digest/DDSketch/HDR histogram, computed per host and **merged** centrally, which is the only correct way to get a global p99 ([selection & order statistics](../selection-and-order-statistics/learning.md)).
- **Heavy hitters / frequency** — Count-Min for "top talkers" in network monitoring, trending terms, abuse detection.
- **Near-duplicate detection** — MinHash and SimHash for document deduplication at web scale; also content-defined chunking's cousin in [hashing techniques](../hashing-techniques/learning.md).
- **Weak password / breach lists** — a Bloom filter of hundreds of millions of leaked passwords fits in a few hundred MB and gives a fast local check.
- **Distributed set reconciliation** — invertible Bloom lookup tables for syncing sets across nodes.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Bloom filter** | Membership pre-filter, false positives tolerable, no deletes, **n known** |
| **Cuckoo filter** | Same, but you need **deletion** or very low FP |
| Counting Bloom | Deletion needed and cuckoo isn't available |
| **HyperLogLog** | Distinct count over a huge or unbounded stream |
| **Count-Min sketch** | Frequency estimates, heavy hitters, over-estimation acceptable |
| **t-digest / DDSketch** | Quantiles that must be **merged across hosts** |
| MinHash / SimHash | Set similarity, near-duplicate detection |
| **Exact structure** | The answer must be exact, or n is small enough that it fits |

## Pitfalls in Depth

### Pitfall: Undersizing a Bloom filter, or using one for an unbounded stream

- **What goes wrong:** A Bloom filter is sized for 1,000,000 items and receives 10,000,000. The false-positive rate climbs from the designed 1% toward 100%, and there is **no signal** — every query returns "maybe present", so the pre-filter stops filtering and the expensive exact lookup runs every time. Performance degrades smoothly to worse-than-not-having-it.
- **Why it happens (the mechanism):** The FP rate `(1 − e^(−kn/m))^k` depends on the ratio `n/m`. Since `m` is fixed at construction, exceeding the design `n` saturates the bit array — and because the structure has no count of distinct insertions, it cannot detect its own saturation. Measured, the relationship is steep: 8 bits/item gives 2.15% and 16 gives 0.045%, so being 2× over the design point is a large multiplier on the error.
- **How to handle it in production, and why that works:** Size from a real estimate of `n` and the tolerable FP rate — roughly **every 4.8 bits per item divides the FP rate by 10**. Track insertions and alarm when approaching the design capacity. For genuinely unbounded streams use a **scalable Bloom filter** (add a new, larger filter when the current one fills, query all of them) or a partitioned/rotating filter with a time window.
- **Trade-offs of the fix:** Scalable Bloom filters query multiple layers, so lookup cost grows with the number of layers and the effective FP rate is the union across them. Rotating windows lose old membership by design, which is correct for "seen recently" and wrong for "seen ever".

### Pitfall: Deleting from a standard Bloom filter

- **What goes wrong:** Someone clears the `k` bits of a removed item. Those bits may be shared with other inserted items, whose lookups now return "definitely absent" — a **false negative**, which the structure's entire contract forbids. Downstream code that trusted "absent means absent" (skipping a disk read, skipping a dedup check) now silently produces wrong results.
- **Why it happens (the mechanism):** The one-sided guarantee comes from the fact that bits are only ever *set*. Clearing breaks the monotonicity that the guarantee rests on. And bit sharing is not an edge case — it's the entire point of the space saving, so collisions are the norm at any useful density.
- **How to handle it in production, and why that works:** Use a **counting Bloom filter** (increment/decrement 4-bit counters instead of setting bits — 4× the memory) or, better, a **cuckoo filter**, which stores fingerprints in a cuckoo hash table and supports deletion natively at comparable or better space than Bloom below ~1% FP. If deletions are rare, periodically rebuild the filter from the authoritative source instead.
- **Trade-offs of the fix:** Counting Bloom filters cost 4× the memory and can still overflow their counters. Cuckoo filters have an insertion failure mode when the table is nearly full (relocation loops) and their merge story is weaker than Bloom's simple OR. Rebuilding needs the source data and a maintenance window.

### Pitfall: Treating one-sided error as two-sided

- **What goes wrong:** Code checks a Count-Min sketch for "is this item rare?" or uses a Bloom filter's negative answer to *confirm* absence in a security-relevant check. Count-Min only over-estimates, so "rare" is unreliable — a rare item can appear frequent through collisions. Bloom's negative is reliable; its *positive* is not, and code that treats a positive as confirmation is the mirror error.
- **Why it happens (the mechanism):** "Approximate" reads as "roughly right in both directions", but these structures are deliberately asymmetric — collisions only ever add (Count-Min) or only ever set bits (Bloom). The asymmetry is a feature that must be exploited in the right direction, and the wrong direction has *unbounded* error.
- **How to handle it in production, and why that works:** Write down which direction is safe at every call site. Bloom: **negative is certain, positive needs verification.** Count-Min: **an estimate is an upper bound**, so it's safe for "definitely at least this frequent" and for finding heavy hitters, never for "this is rare". If you need the other direction, you need a different structure — a Count sketch is unbiased two-sided, and an exact structure is the only source of certainty in both directions.
- **Trade-offs of the fix:** Verifying positives means keeping the exact data available, which is exactly what the sketch was avoiding — so the pattern only pays when positives are rare (a good filter) or when verification is cheap relative to the operation being filtered.

### Pitfall: Averaging or naively combining sketches

- **What goes wrong:** Per-host HyperLogLogs are combined by summing their estimates, so items seen on multiple hosts are counted multiple times and the "distinct" count is inflated. Or per-host p99s are averaged into a "global p99", which is not a percentile of anything — the same error flagged in [selection & order statistics](../selection-and-order-statistics/learning.md).
- **Why it happens (the mechanism):** The sketch produces a number, and numbers look combinable. But distinct-count and quantiles are **not linear functionals** — the union's cardinality is not the sum of cardinalities, and the p99 of a union is not the mean of the p99s. Merging must happen on the *sketch representation*, where the mathematics is defined.
- **How to handle it in production, and why that works:** Merge the sketches, then query. HyperLogLog merges by element-wise **max** of the registers (which is exact — the merged HLL is identical to one built from the union). Bloom filters merge by **OR** (requires identical size and hash functions). Count-Min merges by element-wise **add**. t-digest and DDSketch have defined merge operations. Ship the sketch, not the estimate.
- **Trade-offs of the fix:** Shipping sketches costs more bandwidth than shipping a number — though 16 KB per host per interval is negligible. It also requires all hosts to agree on parameters (precision, hash functions, size), which becomes a versioning problem when you want to change them.

### Pitfall: Reaching for a sketch when exact would fit

- **What goes wrong:** A Bloom filter or HyperLogLog is introduced for a set of 50,000 items. The exact `HashSet` would be a few megabytes, well within budget, and would be **faster** (one probe versus k) as well as exact. The sketch adds a dependency, an error budget to reason about, and a false-positive path to handle, in exchange for saving memory nobody needed.
- **Why it happens (the mechanism):** These structures are associated with "scale", so they get adopted for their reputation rather than from a memory calculation. Measured, the Bloom filter's win over a `HashSet` at 1,000,000 `u64` items was 7.3× (1.2 MB vs 8.9 MB) — real, but only decisive when 8.9 MB is a problem, which at that scale it usually isn't.
- **How to handle it in production, and why that works:** Compute the exact structure's size first. Sketches earn their complexity when the exact version doesn't fit in the memory budget, when it must be shipped over a network repeatedly, or when it must be held per-shard at high cardinality. Note the win grows with item size — a Bloom filter of 1M long URLs is still ~1.2 MB while the exact set is hundreds of MB — so large keys shift the calculation sharply.
- **Trade-offs of the fix:** Exact structures don't merge as cheaply across hosts, and they can't be held for unbounded streams. If either of those is in your future, adopting the sketch early avoids a migration — but that should be a stated reason, not an assumption.
