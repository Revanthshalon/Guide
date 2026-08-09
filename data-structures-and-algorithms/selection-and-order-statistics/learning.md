# Selection & Order Statistics — Learning Notes

## Mental Model

**Selection asks a strictly smaller question than sorting, so it should cost strictly less — and it does, by about an order of magnitude.**

The k-th order statistic is the element that *would* be at index k if the array were sorted. The insight is that you can find it without producing that sorted array: **partitioning tells you which side the answer is on, and you recurse into only one side.** Quicksort recurses into both halves and pays Θ(n log n); quickselect recurses into one and pays Θ(n) expected — the recursion becomes `n + n/2 + n/4 + … = 2n` instead of `n log n`.

Measured on this machine, top-10 of 1M `u64`:

| Approach | Time | Relative |
| --- | --- | --- |
| `sort_unstable` then take 10 | 13.01 ms | 10.7× |
| `select_nth_unstable(10)` | **1.21 ms** | **1×** |

Same ten values. The full sort computes the relative order of 999,990 elements that get discarded.

The framing to carry: **"top k", "median", "p99", "the k-th largest" are selection problems, and reaching for a sort is the single most common unnecessary Θ(n log n) in ordinary code.** Once you notice the pattern you find it everywhere — leaderboards, "worst N offenders" reports, latency percentiles, trimmed means, and the pivot choice inside sorting itself.

There's a second, more interesting half. When n is unknown or unbounded — a stream, a file too large for memory, a firehose of latency samples — you cannot partition at all, because partitioning requires the whole array. That constraint produces an entirely different family: **heaps for exact streaming top-k**, and **sketches for approximate streaming quantiles**. The move from "I have the array" to "I have a stream" changes the achievable bounds, and knowing which regime you're in is most of the decision.

## The Invariant

Quickselect's partition establishes, and each recursion preserves:

> After partitioning around a pivot that lands at index p: everything in `[lo, p)` is ≤ `a[p]`, everything in `(p, hi]` is ≥ `a[p]`. Therefore `a[p]` is *exactly* the p-th order statistic — its final position is known, permanently, without sorting either side.

That last clause is the whole algorithm. One partition pass permanently places one element. If `p == k` you're done; if `p > k` the answer is strictly left; otherwise strictly right. Nothing you learn later can move `a[p]`.

Rust's `select_nth_unstable` states exactly this postcondition: after the call, the element at index k is the one that belongs there, everything before is ≤ it, everything after is ≥ it — **and neither side is sorted**. That's the contract, and misreading it as "partially sorted" is the pitfall below.

For the heap approach the invariant is different:

> A min-heap of size k over the stream so far contains exactly the k largest elements seen; its root is the k-th largest, and any new element ≤ the root can be discarded immediately.

## Mechanics

### Quickselect

```
select(a, lo, hi, k):
    loop:
        if lo == hi: return a[lo]
        p = partition(a, lo, hi)        # pivot lands at its final index
        if k == p: return a[k]
        if k <  p: hi = p - 1
        else:      lo = p + 1
```

Note the loop rather than recursion — the recursive call is in tail position, so it's a `while`, which makes the space Θ(1) rather than Θ(log n).

**The cost analysis.** With a pivot that splits the array in half each time: n + n/2 + n/4 + … = **2n** — Θ(n), with a small constant. With a pivot that always lands at an end (sorted input, naive first-element pivot): n + (n−1) + (n−2) + … = **Θ(n²)**. The gap between those two is entirely down to pivot choice, which makes pivot selection the whole engineering problem — exactly as in quicksort.

**Pivot strategies:**

| Strategy | Guarantee | In practice |
| --- | --- | --- |
| First/last element | Θ(n²) on sorted input | Never use |
| Random | Θ(n) **expected**, on *every* input | The standard defense — expected, not average, so adversarial input doesn't help an attacker |
| Median-of-three | Θ(n) typical, Θ(n²) constructible | Common; defeated by crafted input |
| **Median-of-medians** | Θ(n) **worst case** | Constant ~10–20× worse; theoretical guarantee, rarely shipped |
| Introselect | Θ(n) worst case | Start fast, fall back to median-of-medians on bad luck — what libraries ship |

Median-of-medians (the "BFPRT" algorithm) is worth understanding as the proof that Θ(n) worst-case selection is *possible*: split into groups of 5, take each group's median, recursively select the median of those, and use it as the pivot. It guarantees the pivot is between the 30th and 70th percentile, which bounds the recursion. Its constant factor is so bad that essentially nobody runs it directly — but it's the fallback that makes introselect's worst-case guarantee real, which is the same relationship heapsort has with introsort.

### The streaming family

When you can't hold the data:

| Problem | Structure | Cost |
| --- | --- | --- |
| Exact top-k, streaming | Min-heap of size k | Θ(n log k) time, **Θ(k) space** |
| Exact k-th, one pass, unbounded | Impossible in o(n) space | — |
| Approximate quantiles | t-digest, GK, KLL sketch | Θ(n) time, Θ(1/ε) space |
| Approximate quantiles, mergeable | t-digest / KLL | Merge across shards — the key property |

The min-heap approach is the workhorse: keep a min-heap of the k largest seen; for each new element, if it's larger than the root, pop and push. The root is always the k-th largest, and elements smaller than it are rejected in Θ(1). For k ≪ n most elements are rejected after one comparison, so the practical cost is close to Θ(n).

**Why percentile monitoring needs sketches.** Computing an exact p99 requires all n samples. At millions of requests per second across dozens of hosts that's impossible, so systems use sketches (t-digest, DDSketch, HDR histogram) with bounded error and — critically — **mergeability**: per-host sketches combine into a global one without re-reading the raw data. This is the reason a naive "average the p99s" is wrong, and the reason monitoring systems ship a specific sketch type. See [profiling & measurement](../../performance-optimization/profiling-and-measurement/learning.md) for why tail latency is the number that matters.

### The related order statistics

- **Median.** `select_nth_unstable(n/2)`. For even n, the average of the two middle elements needs two selections (or one selection plus a max over the left part).
- **Trimmed mean / winsorized statistics.** Two selections give the bounds; then one linear pass. Robust to outliers in a way the mean isn't — worth knowing when summarizing latency data.
- **Median of medians.** Both a pivot strategy and a statistic in its own right (used in robust regression).
- **Rank / count-less-than.** The inverse question: "what is x's position?" Sorted array → `partition_point`. Dynamic → an order-statistic tree or Fenwick tree (Stage 4).
- **Selection in a sorted matrix / union of sorted lists.** Solved by binary searching on the *value* — the "binary search on the answer" technique from [binary search](../binary-search/learning.md), and a good demonstration of how those two topics compose.

## Complexity

| Approach | Time | Space | Notes |
| --- | --- | --- | --- |
| Full sort, take k | Θ(n log n) | Θ(1)–Θ(n) | 10.7× measured overkill for k = 10, n = 1M |
| Quickselect (random pivot) | **Θ(n) expected**, Θ(n²) worst | Θ(1) | `select_nth_unstable` |
| Median-of-medians | **Θ(n) worst** | Θ(log n) | Constant ~10–20× worse |
| Introselect | Θ(n) worst | Θ(log n) | What libraries ship |
| Min-heap of size k | Θ(n log k) | **Θ(k)** | Streaming; k ≪ n → near-Θ(n) |
| k passes of max | Θ(n·k) | Θ(1) | Only sane for k ≤ 2 |
| Partial sort (sort first k) | Θ(n log k) | Θ(1) | When you need them *ordered* |
| Quantile sketch | Θ(n) | Θ(1/ε) | Approximate, mergeable |

**Where the table misleads:** quickselect's Θ(n) is *expected*, and the constant is roughly 2n element moves plus poor branch predictability during partitioning — so for small k it can lose to a size-k heap that rejects most elements after one comparison. And "Θ(n) beats Θ(n log n)" hides that the log factor at n = 10⁶ is only 20; the measured 10.7× comes as much from the sort touching every element repeatedly (cache traffic) as from the comparison count.

**The k-approaching-n regime:** as k → n, selection degenerates toward sorting. If you need the top half of the data *in order*, just sort.

## Rust Implementation

```rust
// The 10.7× win: partition, don't sort.
v.select_nth_unstable(k);
let (bottom_k, kth, _rest) = (&v[..k], &v[k], &v[k+1..]);
// NOTE: bottom_k is NOT sorted — it's just "the k smallest, in some order".

// Want them ordered too? Sort only the prefix: Θ(n) + Θ(k log k).
v.select_nth_unstable(k);
v[..k].sort_unstable();

// Top-k (largest) — select by a reversed comparator, or select n-k.
v.select_nth_unstable_by(|a, b| b.cmp(a));       // descending: v[..k] are the k largest
v.select_nth_unstable_by_key(|x| Reverse(x.score));

// Median.
let mid = v.len() / 2;
let (_, median, _) = v.select_nth_unstable(mid);

// Streaming top-k: min-heap of size k, Θ(k) memory, n unknown.
use std::collections::BinaryHeap;
use std::cmp::Reverse;
let mut heap: BinaryHeap<Reverse<u64>> = BinaryHeap::with_capacity(k);
for x in stream {
    if heap.len() < k { heap.push(Reverse(x)); }
    else if x > heap.peek().unwrap().0 { heap.pop(); heap.push(Reverse(x)); }
}
// heap.peek() is the k-th largest; drain for all k.

// Floats: total_cmp, as always.
v.select_nth_unstable_by(|a, b| a.score.total_cmp(&b.score));
```

**The API contract to internalize:** `select_nth_unstable` returns `(&mut [T], &mut T, &mut [T])` — the elements before k, the k-th element, and the elements after. It **reorders the entire slice** and sorts neither side. If you hand `&v[..k]` to something expecting sorted data, you get wrong answers with no error.

**Crates:** `itertools::Itertools::k_smallest` (heap-based streaming top-k over any iterator), `tdigest` / `hdrhistogram` (mergeable approximate quantiles for latency monitoring), `order-stat` (median-of-medians and friends when you want the worst-case guarantee).

## Use Cases

- **Leaderboards and "top N" reports.** The canonical case, and the one where the 10.7× is free.
- **Latency percentiles.** p50/p95/p99 from a batch of samples is selection; from a live stream it's a sketch. Getting this wrong — averaging percentiles across hosts — is one of the most common monitoring errors.
- **Median as a robust summary.** Unlike the mean, one outlier can't move it. Standard for "typical" latency, file size, or response time.
- **Trimmed means.** Two selections plus a pass; used in benchmarking to discard the noisiest runs.
- **Pivot selection inside sorting.** Median-of-medians exists mainly to make introsort/introselect's worst-case guarantee real.
- **Outlier detection.** "Everything above the 99th percentile" — one selection, one partition, done.
- **Load balancing and capacity work.** "Which 10 shards are hottest" is top-k over a metric, recomputed frequently.
- **Nearest-neighbour search.** k-NN keeps a size-k heap of the best candidates seen — the same streaming top-k pattern, inside a spatial index (Stage 9).

## When to Use Which

| Reach for | When |
| --- | --- |
| **`select_nth_unstable`** | You have the array, need the k-th or the top/bottom k. |
| `select_nth_unstable` + sort prefix | …and you need those k *in order*. |
| `BinaryHeap` of size k | Streaming, n unknown or huge, k small. Θ(k) memory. |
| `iter().min()` / `.max()` | k = 1. Don't overthink it. |
| Full sort | k is a large fraction of n, or you need the whole order anyway. |
| Quantile sketch (t-digest, HDR) | Percentiles over a stream, or mergeable across hosts. |
| Order-statistic tree / Fenwick | Rank queries over a **changing** set (Stage 4). |
| Median-of-medians | You need a Θ(n) *worst-case* guarantee against adversarial input. |

## Pitfalls in Depth

### Pitfall: Sorting to get the top k

- **What goes wrong:** The most common unnecessary Θ(n log n) in ordinary code. A "top 10" endpoint sorts a million records on every request. Measured here: **13.01 ms vs 1.21 ms — 10.7×** for identical output. At scale that's the difference between one server and eleven.
- **Why it happens (the mechanism):** `sort` is the obvious verb and a single line, while `select_nth_unstable` is a less familiar name with a more awkward return type. The sort computes the total order of every element, then discards 99.999% of that information.
- **How to handle it in production, and why that works:** `select_nth_unstable(k)` to partition in Θ(n), then `v[..k].sort_unstable()` if the k need to be ordered — Θ(k log k) on a tiny slice is free. For streams or unknown n, a size-k `BinaryHeap` at Θ(n log k) with Θ(k) memory.
- **Trade-offs of the fix:** Selection *reorders the input slice*, so it's destructive in a way `sorted_by` on a copy isn't — if the original order matters you now need a clone, which may eat the win for small n. Its Θ(n) is expected, not worst-case. And below a few thousand elements the difference is noise, so the extra API complexity isn't worth it.

### Pitfall: Assuming the prefix is sorted

- **What goes wrong:** `v.select_nth_unstable(k)` then `&v[..k]` is passed to a binary search, rendered as a ranked list, or fed to code that assumes descending order. The output is subtly wrong — the right *set* of k elements in an arbitrary order — and it looks correct in tests where k is small and the data happens to land favourably.
- **Why it happens (the mechanism):** "Partial sort" is a misleading mental label. The postcondition is a *partition*: everything before k is ≤ `v[k]`, everything after is ≥ it. Neither side is ordered internally, and that's precisely why it's Θ(n) — establishing order within the prefix is the work it deliberately skips.
- **How to handle it in production, and why that works:** Sort the prefix explicitly when order matters: `v.select_nth_unstable(k); v[..k].sort_unstable();`. Two operations, Θ(n) + Θ(k log k), still far cheaper than a full sort. Making the sort explicit also documents that the prefix wasn't ordered before it.
- **Trade-offs of the fix:** It's an extra line that's easy to forget in one code path out of five. If the k elements are consumed in an order-independent way (a sum, a set, a membership test), sorting them is wasted work — so the right rule is "sort the prefix iff a consumer depends on order," which requires knowing your consumers.

### Pitfall: Quickselect's Θ(n²) on adversarial or structured input

- **What goes wrong:** A hand-rolled quickselect with a first-element or median-of-three pivot meets sorted input — a common shape, since data often arrives ordered — and degrades to Θ(n²). At n = 10⁶ that's 10¹² operations: effectively a hang. If the input is attacker-influenced, it's a denial-of-service primitive.
- **Why it happens (the mechanism):** A pivot that always lands near an end shrinks the range by 1 instead of halving it, so the recursion is n + (n−1) + … = Θ(n²). Sorted, reverse-sorted, and all-equal inputs all trigger it with naive pivots — and all-equal is the sneakiest, because a two-way partition puts every element on one side.
- **How to handle it in production, and why that works:** Use `select_nth_unstable`, which is introselect — it starts with a fast pivot strategy and falls back to a guaranteed-Θ(n) method when it detects poor splits, so the worst case is bounded regardless of input. If you must hand-roll: random pivots (converting *average*-case into *expected*-case, which holds even for adversarial input) plus a three-way (Dutch-flag) partition so runs of equal elements are consumed in one pass.
- **Trade-offs of the fix:** Random pivots need an RNG in the hot loop and make timing non-reproducible, which complicates benchmarking and debugging. Three-way partitioning is slower than two-way on data with no duplicates. Median-of-medians gives a hard worst-case bound at a 10–20× constant — almost never the right trade outside adversarial settings.

### Pitfall: Averaging percentiles

- **What goes wrong:** Each of 20 hosts reports its p99 latency; the dashboard averages them and calls the result "the p99." It isn't — it's a number with no statistical meaning, and it systematically *understates* the true global p99. Capacity decisions and SLO reporting are then made on a fiction.
- **Why it happens (the mechanism):** Percentiles are not linear, so they don't average. The p99 of a union is not a function of the per-part p99s: a single host with a badly skewed distribution can dominate the global tail while its own p99 looks unremarkable in the mean. The only correct combination needs the underlying *distributions*.
- **How to handle it in production, and why that works:** Use a **mergeable sketch** — t-digest, DDSketch, or HDR histogram — computed per host and merged centrally, then take the percentile of the merged sketch. Mergeability is the whole point: it reconstructs (approximately) the global distribution without shipping raw samples. Bounded, quantified error, and the arithmetic is actually valid.
- **Trade-offs of the fix:** Sketches are approximate, with error concentrated differently depending on the algorithm (t-digest is most accurate at the tails, which is usually what you want; equal-width histograms are not). They require choosing an accuracy parameter, and they're more machinery than a single number per host. But the alternative is a number that is simply wrong.

### Pitfall: Using selection where the set is changing

- **What goes wrong:** "The current median" or "the current top 10" is recomputed with `select_nth_unstable` after every insertion. Each call is Θ(n), so maintaining the statistic over m updates is Θ(n·m) — a live leaderboard that gets quadratically slower as it grows.
- **Why it happens (the mechanism):** Selection is a one-shot algorithm over a static array; it has no incremental form, because its whole speed comes from destructively partitioning the data it was given. Re-running it discards all previous work.
- **How to handle it in production, and why that works:** Maintain a structure instead of recomputing a statistic. Two heaps (a max-heap of the lower half, a min-heap of the upper half, kept balanced) gives a **running median** in Θ(log n) per update with Θ(1) reads. For a running top-k, a size-k min-heap updates in Θ(log k). For arbitrary rank queries over a changing set, an order-statistic tree or Fenwick tree gives Θ(log n) per operation (Stage 4).
- **Trade-offs of the fix:** The two-heap median requires rebalancing logic and doesn't support deletion of arbitrary elements without a lazy-deletion scheme or an indexed heap. These structures are more code and more state than a one-line selection call — worth it once updates outnumber a handful, and over-engineering below that.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if you needed the k-th of every prefix? | Running median via two heaps; order-statistic trees |
| Batch it | What if many k's were requested at once? | One multi-pivot partition answering all of them; or just sort |
| Approximate it | What if the 99th ± 1% sufficed? | t-digest, KLL, DDSketch — **Θ(1/ε) space instead of Θ(n)** |
| Randomize it | What if the pivot were random? | Quickselect's Θ(n) *expected* on every input, adversary-proof |
| Externalize it | What if it didn't fit in RAM? | Sampling-based selection; distributed quantiles via mergeable sketches |
| Parallelize it | Where's the independence? | Partition in parallel, then recurse into one side; or per-shard sketches merged |
| Invert it | What if you asked "what rank is x?" instead of "what is rank k?" | `partition_point` (static) / Fenwick tree (dynamic) — the dual problem |
| Augment it | What does storing subtree sizes buy? | Order-statistic tree: Θ(log n) select *and* rank on a changing set |
| Specialize it | What if k = 1, or k = n/2, or values were bounded integers? | min/max in one pass; median-specific algorithms; counting-based selection in Θ(n + k) |
| Amortize it | What if one query could be terrible? | Sort once, then every order statistic is Θ(1) forever |

**Questions:**

1. Quickselect recurses into one side and gets Θ(n); quicksort recurses into both and gets Θ(n log n). Write both recurrences and show exactly where the log factor enters.
2. Median-of-medians guarantees Θ(n) worst case and is almost never used. Explain the guarantee it buys, then argue why introselect's *expected*-with-fallback is the better engineering choice in nearly all cases.
3. Under "invert it", rank and select are duals. Give the structure for each of the four combinations: static/dynamic × rank/select.
4. A t-digest uses Θ(1/ε) space regardless of n. What property of quantiles makes bounded-space approximation possible, and why is the same not true for, say, an exact count of distinct values?
5. Why can't per-host p99s be averaged, and what exactly does mergeability give you that averaging doesn't? Construct a two-host example where the average of the p99s is far below the true p99.
6. Under "amortize it", sorting once makes every order statistic Θ(1). Derive the number of distinct order-statistic queries at which sorting first beats repeated selection, for n = 10⁶.
7. Three-way (Dutch flag) partitioning helps quickselect on all-equal input. Describe what happens with two-way partitioning on 1M identical elements, and why the fix is a *partitioning* change rather than a *pivot* change.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State `select_nth_unstable`'s postcondition precisely, and say what is *not* guaranteed about `v[..k]`.
2. Give the measured full-sort vs selection numbers for top-10 of 1M, and explain where the factor comes from beyond the comparison count.
3. Give the recurrence for quickselect with a median pivot and with a worst-case pivot, and the closed form of each.
4. You need the top 100 from an unbounded stream with 8 MB of memory. Give the structure, the complexity, and the memory used.
5. Why is averaging per-host p99s wrong, and what's the correct mechanism?
6. Your leaderboard updates thousands of times per second and reads the top 10 constantly. Why is `select_nth_unstable` the wrong tool, and what replaces it?

Build exercises:

- Implement quickselect three ways — first-element pivot, random pivot, and median-of-medians — and benchmark all three against `select_nth_unstable` on random, sorted, reverse-sorted, and all-equal inputs of 1M elements. The sorted and all-equal columns are the point: watch the naive version go quadratic, and watch median-of-medians be reliably slow.
- Reproduce the 10.7×: measure full sort vs `select_nth_unstable` vs a size-k `BinaryHeap` for k = 1, 10, 100, 10⁴, 10⁵ over 1M elements. The two crossover points you find are a decision table you'll reuse.
- Implement the two-heap running median, with rebalancing, and verify it against a naive re-sort on every insert over 100k random inserts. Then measure both — the Θ(log n) vs Θ(n log n) gap is dramatic and makes the "maintain a structure, don't recompute a statistic" lesson permanent.
- Implement a simple quantile sketch (a KLL or even a fixed-bucket histogram), then demonstrate the averaging bug: construct two skewed distributions where the mean of the p99s is far below the merged p99, and show your sketch getting it right.

## Open Questions

- Where exactly does a size-k `BinaryHeap` beat `select_nth_unstable` on this machine? The heap rejects most elements in one comparison, so for very small k it should win despite Θ(n log k) — find the k.
- How much does `select_nth_unstable`'s destructive reordering cost when a clone is required to preserve the input? Find the n below which "clone then select" loses to a plain sort.
- t-digest vs DDSketch vs HDR histogram for latency in a Rust service: accuracy at p99/p999 for a given memory budget, measured on realistic (log-normal-ish) data.
- Does a three-way partition actually help `select_nth_unstable` on high-duplicate data, or does std already handle it? Test with 1M elements drawn from 10 distinct values.
- Is there a practical parallel selection in Rust (rayon-based) that beats `select_nth_unstable` at 10⁷–10⁸, or does the memory bandwidth dominate?

## References

- Blum, Floyd, Pratt, Rivest, Tarjan, "Time Bounds for Selection" (1973) — the median-of-medians algorithm and the proof that Θ(n) worst-case selection exists. Worth reading once for the group-of-5 argument.
- CLRS ch. 9 — randomized-select and select in worst-case linear time, with the full analysis of both.
- [`slice::select_nth_unstable` docs](https://doc.rust-lang.org/std/primitive.slice.html#method.select_nth_unstable) — read the postcondition carefully; it states the partition guarantee and explicitly does *not* promise sorted sides.
- Ted Dunning & Otmar Ertl, ["Computing Extremely Accurate Quantiles Using t-Digests"](https://arxiv.org/abs/1902.04023) — the mergeable sketch behind most modern latency monitoring; the accuracy-at-the-tails design is the part that matters.
- Gil Tene, HdrHistogram and the "How NOT to Measure Latency" talk — why percentiles are the right statistic, and coordinated omission as the error that invalidates most latency measurement.
- Related topics in this repo: [Sorting](../sorting/learning.md) (the operation this replaces, and the shared partitioning machinery), [Binary Search](../binary-search/learning.md) (selection in a sorted matrix via binary search on the answer; `partition_point` as static rank), [Complexity Analysis](../complexity-analysis/learning.md) (expected vs average vs worst — quickselect is the cleanest example), [Profiling & Measurement](../../performance-optimization/profiling-and-measurement/learning.md) (why tail latency is the metric, and the distribution discipline).
