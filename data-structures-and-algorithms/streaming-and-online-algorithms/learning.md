# Streaming & Online Algorithms — Learning Notes

## Mental Model

**Two different constraints, often confused, and each defines a field:**

- **Streaming** — the data is too large to store. You get one pass (or few), sublinear memory, and must answer approximately. The question is *"what can I compute in o(n) space?"*
- **Online** — the data arrives over time and you must **commit irrevocably** before seeing the rest. Memory isn't the issue; the future is. The question is *"how much worse than a clairvoyant algorithm am I?"*

Cache eviction is online (decide what to evict now, without knowing future requests). Counting distinct visitors is streaming (the data would fit conceptually, but not in memory). Some problems are both.

**The online measure is the competitive ratio:**

> An online algorithm is **c-competitive** if for every input sequence, `cost(online) ≤ c · cost(optimal-offline) + constant`, where the offline optimum knows the entire future.

The theory says LRU is exactly **k-competitive** for a cache of size k — with k = 256, LRU may be 256× worse than optimal. Measured on this machine against Bélády's optimal (which requires the future and is therefore unimplementable):

| Cache size k | LRU misses | OPT misses | **LRU / OPT** | Theory says |
| --- | --- | --- | --- | --- |
| 16 | 83.5% | 53.9% | **1.55×** | 16× |
| 64 | 43.5% | 22.4% | **1.94×** | 64× |
| 256 | 18.3% | 9.0% | **2.03×** | 256× |

**The competitive ratio overstates the real gap by two orders of magnitude.** That's not a flaw in the theory — the bound is tight, achieved by an adversary who requests exactly the page you just evicted — but it means the ratio describes an adversary, not a workload. On realistic (Zipf-like) access patterns LRU lands around 2× optimal and stays there.

That gap is the honest lesson of this topic, and it mirrors the approximation-ratio finding in [intractability](../intractability-and-approximation/learning.md): **worst-case competitive analysis predicts adversarial behaviour, not typical behaviour.**

## The Invariant

**The streaming model:**

> Elements arrive one at a time, in one pass, in an order you don't control. Memory is **sublinear** in the stream length — typically Θ(log n) or Θ(1/ε). You cannot revisit an element.

That constraint immediately makes most exact answers impossible. Exact distinct-count requires Θ(n) space (you must remember what you've seen); exact median requires Θ(n). So **streaming is inherently approximate**, which is why it and [probabilistic data structures](../probabilistic-data-structures/learning.md) are the same subject viewed from different sides. Measured there: HyperLogLog counted **10,000,000 distinct items in 16 KB to within 0.41%**.

**Online algorithms:**

> Each decision is made with knowledge of only the prefix seen so far, and cannot be revised. Compare against OPT, which sees everything.

Two subtleties worth stating:

- **Deterministic online algorithms have hard lower bounds.** No deterministic paging algorithm beats k-competitive — an adversary simply requests whatever you just evicted. Randomization helps genuinely: the randomized marking algorithm is `2·H_k`-competitive (≈ 2 ln k), which for k = 256 is ~11 rather than 256.
- **The constant matters.** "c-competitive plus a constant" allows an additive term, which is what lets algorithms amortize a bad start.

## Mechanics

### The streaming toolkit

| Problem | Structure | Space |
| --- | --- | --- |
| **Distinct count** | HyperLogLog | **Θ(1)** — 16 KB for any n |
| Membership | Bloom / cuckoo filter | Θ(n·bits) |
| **Heavy hitters** | Count-Min sketch, Misra-Gries | Θ(1/ε) |
| **Quantiles / p99** | t-digest, DDSketch, GK | Θ(1/ε) |
| **Uniform sample** | Reservoir sampling | **Θ(k)** |
| Frequency moments | AMS sketch | Θ(1/ε²) |
| Set similarity | MinHash | Θ(k) |
| Sliding-window aggregates | Exponential histograms | Θ((1/ε) log n) |

All are covered in [probabilistic data structures](../probabilistic-data-structures/learning.md); the streaming framing adds *why* they must be approximate — the one-pass sublinear-space constraint makes exactness provably impossible, not merely expensive.

**Misra-Gries** is worth knowing alongside Count-Min: it finds all items appearing more than n/k times using k counters, by decrementing *all* counters when a new item arrives and no slot is free. It's deterministic, simple, and gives one-sided guarantees.

### The online classics

| Problem | Online algorithm | Competitive ratio |
| --- | --- | --- |
| **Paging / caching** | LRU, FIFO | **k** (tight, deterministic) |
| Paging, randomized | Marking algorithm | **2·H_k ≈ 2 ln k** |
| **Ski rental** | Buy when rental cost = purchase price | **2** (deterministic), e/(e−1) ≈ 1.58 randomized |
| Load balancing | Greedy (least loaded) | 2 − 1/m |
| **Secretary problem** | Observe n/e, then take the first better one | 1/e ≈ 0.368 success probability |
| List update | Move-to-front | 2 |
| k-server | Work function algorithm | 2k−1 |
| Bin packing | First-fit-decreasing | 11/9·OPT + 6/9 |

**Ski rental is the canonical "rent or buy" problem** and it generalizes far beyond skiing: when to spin down a disk, when to migrate a VM, when to buy reserved instances instead of on-demand, when to give up on a lock and sleep. The answer — pay rental until the accumulated cost equals the purchase price, then buy — is 2-competitive, and randomization improves it to ≈1.58.

### Cache eviction — the practical case

| Policy | Idea | Weakness |
| --- | --- | --- |
| **LRU** | Evict least recently used | Scan-resistant? No — one big scan evicts everything |
| **LFU** | Evict least frequently used | Slow to adapt; old hot items linger |
| **ARC** | Adaptively balance recency and frequency | More state; patented (historically) |
| **2Q / SLRU** | Probationary + protected segments | Tuning |
| **TinyLFU / W-TinyLFU** | Frequency sketch admits only worthy items | The modern default (Caffeine, `moka`) |
| **Bélády (OPT)** | Evict the one used farthest in the future | **Unimplementable** — needs the future |

Bélády's algorithm matters precisely *because* it's unimplementable: it's the benchmark. Measuring your policy against OPT on a replayed trace tells you how much room is left — measured above, LRU sits around 2× OPT on Zipf-like data, so a better policy can win at most ~2×, not 10×.

**W-TinyLFU is the current practical answer**: it uses a Count-Min sketch of recent frequencies (a streaming structure!) to decide whether a *newly arriving* item deserves to displace the eviction candidate. That combination — a streaming sketch inside an online algorithm — is the neatest instance of the two halves of this topic meeting.

### Recognizing which you have

Ask two questions:

1. **Can I store everything?** No → streaming; accept approximation.
2. **Must I decide before seeing the rest?** Yes → online; compare against OPT.

If both, you're in the hardest quadrant (e.g. cache admission over an unbounded key space), and the answer is usually a sketch feeding a heuristic.

## Complexity

| Problem | Offline (exact) | Streaming/online | Guarantee |
| --- | --- | --- | --- |
| Distinct count | Θ(n) space | **Θ(1)** (HLL, 16 KB) | ±1.04/√m |
| Quantiles | Θ(n) space | Θ(1/ε) | ±ε rank error |
| Heavy hitters | Θ(n) space | Θ(1/ε) | over-estimate only |
| Uniform k-sample | Θ(n) | **Θ(k)**, one pass | exact uniformity |
| **Paging** | Θ(n) (Bélády) | Θ(k) state | **k-competitive** (measured ~2×) |
| Ski rental | trivial with foresight | Θ(1) | 2-competitive |
| Load balancing | NP-hard exactly | Θ(1) per job | 2 − 1/m |
| Sorting | Θ(n log n) | **impossible** in sublinear space | — |

**Where the table misleads.** The competitive-ratio column describes an adversary. Measured, LRU's k-competitive bound (256 at k=256) corresponded to **2.03× in practice** — a factor of 126 between the bound and the observation. Competitive analysis is a *worst-case* tool, and its predictions about typical workloads are close to worthless.

This has driven real research: **resource augmentation** (compare an online algorithm with cache size 2k against OPT with size k, which gives much more realistic bounds) and **distributional analysis** exist precisely because the plain competitive ratio is so pessimistic.

## Use Cases

- **Analytics at scale** — unique visitors (HyperLogLog), trending terms (Count-Min), p99 latency (t-digest). All streaming, all merged across shards.
- **Network monitoring** — heavy hitters for DDoS detection, flow-size distributions, at line rate where storing packets is impossible.
- **Cache and CDN eviction** — online; W-TinyLFU or ARC in production, benchmarked against Bélády on replayed traces.
- **Ad serving and recommendation** — online decisions with no rewind; multi-armed bandits are the online-learning version.
- **Cloud cost decisions** — reserved vs on-demand instances is ski rental; so is autoscaling hysteresis.
- **Load balancing** — assign each request on arrival; "power of two choices" is an online algorithm with a large practical win.
- **Log and telemetry pipelines** — sampling (reservoir), aggregation (sketches), and shedding under load.
- **Database query planning** — cardinality estimation from streaming sketches over columns.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Reservoir sampling** | Uniform sample from an unbounded stream, Θ(k) space |
| **HyperLogLog** | Distinct count — Θ(1) space |
| **Count-Min / Misra-Gries** | Heavy hitters, frequency estimates |
| **t-digest / DDSketch** | Quantiles, **mergeable across hosts** |
| Exponential histograms | Sliding-window aggregates |
| **LRU** | Default cache policy — measured ~2× OPT, and simple |
| **W-TinyLFU** (`moka`) | Cache where hit rate matters; scan-resistant |
| ARC / 2Q | Mixed recency/frequency workloads |
| **Bélády** | As a *benchmark* on replayed traces — never in production |
| Ski-rental reasoning | Any "keep paying or commit?" decision |
| Exact offline | It fits, and you can make two passes |

## Pitfalls in Depth

### Pitfall: Taking the competitive ratio as a performance estimate

- **What goes wrong:** LRU is rejected because it's "k-competitive" — potentially 256× worse than optimal at k = 256 — and effort goes into a complex adaptive policy. Measured on a Zipf-like trace, **LRU was 2.03× OPT at k = 256**, so the ceiling on any improvement was 2×, not 256×. The complex policy can capture at most a fraction of that.
- **Why it happens (the mechanism):** The competitive ratio is a **worst-case bound against an adaptive adversary** who, knowing your policy, requests exactly the page you just evicted. That input is achievable and the bound is tight — but it bears no resemblance to real access patterns, which have temporal locality precisely because programs do.
- **How to handle it in production, and why that works:** **Benchmark against Bélády's optimal on a replayed trace of your own workload.** That gives the real headroom: if LRU is at 2× OPT, you know the maximum available win. Then evaluate candidate policies against the same trace. This converts an unbounded theoretical worry into a measured number.
- **Trade-offs of the fix:** Replaying a trace requires capturing one, and traces are workload- and time-specific — a policy tuned to last month's trace may not survive a traffic shift. The competitive ratio's value is precisely that it survives any shift, which matters if adversarial access is a real threat (a multi-tenant cache, for example).

### Pitfall: Confusing streaming with online

- **What goes wrong:** A problem is treated as streaming (reach for a sketch, accept approximation) when the real constraint is *irrevocability*, or vice versa. For example, a cache-admission decision gets a HyperLogLog when what's needed is an eviction *policy*; or a distinct-count over 10 TB gets a two-pass exact algorithm that can't run because the data won't fit.
- **Why it happens (the mechanism):** Both involve "data arriving over time", so the phrase covers two different constraints. Streaming's limit is **space**; online's is **information about the future**. An algorithm can be online with unlimited memory (paging with a full history) or streaming but offline (a sketch computed over a stored file).
- **How to handle it in production, and why that works:** Ask the two questions separately. *Can I store everything?* No → streaming; expect approximation, choose a sketch. *Must I commit before seeing the rest?* Yes → online; expect a competitive ratio, and benchmark against the offline optimum. Answering them independently picks the right toolbox.
- **Trade-offs of the fix:** Some problems are genuinely both, and then you compose — W-TinyLFU is exactly that (a streaming frequency sketch driving an online admission decision), and composition means the approximation error of the sketch feeds into the online decision quality.

### Pitfall: Sketches that can't be merged

- **What goes wrong:** Per-host metrics are computed with a structure that has no merge operation, so the central view is assembled by averaging estimates — inflating distinct counts (items seen on several hosts counted repeatedly) or producing a "p99" that is not a percentile of anything.
- **Why it happens (the mechanism):** Distinct-count and quantiles are **not linear functionals**: the cardinality of a union is not the sum of cardinalities, and the p99 of a union is not the mean of the p99s. Merging must happen on the *sketch representation*, where the operation is defined — HyperLogLog merges by element-wise max, Count-Min by addition, t-digest by a defined merge.
- **How to handle it in production, and why that works:** **Choose mergeable sketches and ship the sketch, not the estimate.** A 16 KB HyperLogLog per host per interval is negligible bandwidth and reconstructs the exact union's estimate. This is the same error flagged in [selection & order statistics](../selection-and-order-statistics/learning.md) and [probabilistic data structures](../probabilistic-data-structures/learning.md) — it recurs because the intuition that "numbers combine" is strong.
- **Trade-offs of the fix:** All hosts must agree on sketch parameters (precision, hash functions), which becomes a versioning problem when you want to change them. Shipping sketches costs more bandwidth than shipping a number, though not meaningfully at these sizes.

### Pitfall: LRU on a scanning workload

- **What goes wrong:** A cache uses LRU and a single large sequential scan — a backup, a full-table analytics query, a batch reindex — walks through it. Every scanned item is inserted, evicting the entire working set, and the hit rate collapses to near zero for the duration and stays poor afterward while the working set is refetched.
- **Why it happens (the mechanism):** LRU's recency heuristic assumes recently-used implies soon-to-be-used-again, which is exactly false for a scan: every scanned item is used once and never again, yet each one is maximally recent on insertion. LRU has no mechanism to distinguish "hot" from "just arrived", so it is not scan-resistant by construction.
- **How to handle it in production, and why that works:** Use a policy with an **admission** decision rather than only an eviction decision. W-TinyLFU keeps a Count-Min sketch of recent frequencies and admits a new item only if it looks more valuable than the eviction candidate — so single-use scan items are rejected and never displace the working set. ARC and 2Q achieve similar resistance with probationary segments. `moka` implements W-TinyLFU in Rust.
- **Trade-offs of the fix:** Admission policies carry extra state (the frequency sketch) and can be slow to admit genuinely new hot items, since a newcomer must out-score an incumbent. They're also more complex to reason about when debugging a hit-rate regression.

### Pitfall: Assuming a single pass when you have two

- **What goes wrong:** A sketch with ±2% error is used to compute a figure that appears on an invoice or a compliance report, when the data is actually on disk and a second exact pass was available. The approximation is a *choice*, and here it was the wrong one.
- **Why it happens (the mechanism):** Streaming machinery is adopted for its association with scale rather than from the constraint. If the data is stored and re-readable, you're not in the streaming model at all — you have random access and as many passes as you're willing to pay for.
- **How to handle it in production, and why that works:** Confirm the constraint is real. Genuinely unbounded or too large to store → streaming, and document the error bound where the number is consumed. Stored and re-readable → an exact aggregate is usually affordable, and sketches are then an *optimization* to justify by measurement rather than a necessity. A common good middle ground is a sketch for the live dashboard and an exact nightly batch for reporting.
- **Trade-offs of the fix:** Exact passes over large data cost time and I/O, and for interactive dashboards that latency is unacceptable — which is the legitimate case for sketches. The point is to make it a decision with a stated error budget rather than a default.
