# Selection & Order Statistics — Quick Reference

## At a Glance

Find the k-th element without producing the sorted array. Partitioning places one element permanently and lets you recurse into **one** side: `n + n/2 + n/4 + … = 2n` instead of `n log n`.

**Invariant:** after partitioning at index p, everything in `[lo, p)` is ≤ `a[p]` and everything in `(p, hi]` is ≥ `a[p]` — so `a[p]` is exactly the p-th order statistic. **Neither side is sorted.**

## The Number

Top-10 of 1M `u64`, measured:

| Approach | Time |
| --- | --- |
| `sort_unstable` + take 10 | 13.01 ms |
| `select_nth_unstable(10)` | **1.21 ms** (**10.7×**) |

## Complexity

| Approach | Time | Space |
| --- | --- | --- |
| Full sort, take k | Θ(n log n) | Θ(1)–Θ(n) |
| **Quickselect (random pivot)** | **Θ(n) expected**, Θ(n²) worst | Θ(1) |
| Median-of-medians | Θ(n) **worst** | Θ(log n) |
| Introselect (`select_nth_unstable`) | Θ(n) worst | Θ(log n) |
| Min-heap size k | Θ(n log k) | **Θ(k)** |
| Partial sort (k ordered) | Θ(n log k) | Θ(1) |
| Quantile sketch | Θ(n) | Θ(1/ε) |

## Choose This When

| Use | For |
| --- | --- |
| **`select_nth_unstable`** | Have the array; need k-th or top/bottom k |
| …+ `v[..k].sort_unstable()` | …and they must be **ordered** |
| `BinaryHeap` size k | Streaming, n unknown/huge, k small, Θ(k) memory |
| `iter().min()/.max()` | k = 1 |
| Full sort | k is a large fraction of n |
| t-digest / HDR histogram | Percentiles over a stream, **mergeable across hosts** |
| Order-statistic / Fenwick tree | Rank queries over a **changing** set |
| Two heaps | Running median |

## Snippets

```rust
v.select_nth_unstable(k);
let (below, kth, above) = (&v[..k], &v[k], &v[k+1..]);   // below is NOT sorted

v.select_nth_unstable(k); v[..k].sort_unstable();        // when order matters

v.select_nth_unstable_by_key(|x| Reverse(x.score));      // k largest
v.select_nth_unstable_by(|a, b| a.s.total_cmp(&b.s));    // floats

// Streaming top-k: Θ(k) memory, n unknown
let mut heap: BinaryHeap<Reverse<u64>> = BinaryHeap::with_capacity(k);
for x in stream {
    if heap.len() < k { heap.push(Reverse(x)); }
    else if x > heap.peek().unwrap().0 { heap.pop(); heap.push(Reverse(x)); }
}
```

## Pivot Strategies

| Strategy | Guarantee |
| --- | --- |
| First/last | Θ(n²) on sorted — never |
| **Random** | Θ(n) **expected on every input** (adversary-proof) |
| Median-of-three | Θ(n) typical, constructible Θ(n²) |
| Median-of-medians | Θ(n) worst, ~10–20× constant |
| Introselect | Θ(n) worst — what libraries ship |

## Rules of Thumb

- "Top k", "median", "p99", "k-th largest" → selection, not sorting.
- `select_nth_unstable` **reorders the whole slice** and sorts neither side.
- Sort the prefix iff a consumer depends on order.
- Streaming/unbounded n → size-k min-heap.
- **Never average percentiles** — merge sketches instead.
- Changing set → maintain a structure (two heaps / Fenwick), don't recompute.
- Random pivots convert *average*-case into *expected*-case.
- All-equal input needs a **three-way** partition, not a better pivot.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Sorting for top-k | 10.7× slower than necessary |
| Assumed `v[..k]` sorted | Right set, arbitrary order; binary search on it silently wrong |
| Naive pivot on sorted input | Θ(n²) — effective hang; DoS if input is attacker-controlled |
| Two-way partition, all-equal data | Θ(n²) on 1M identical elements |
| Averaging per-host p99 | Understates the true tail; SLOs based on fiction |
| Re-selecting after every insert | Θ(n·m) leaderboard that degrades as it grows |

## Key References

- Blum, Floyd, Pratt, Rivest, Tarjan (1973) — median-of-medians
- [`slice::select_nth_unstable`](https://doc.rust-lang.org/std/primitive.slice.html#method.select_nth_unstable) — read the postcondition
- Dunning & Ertl, ["t-Digest"](https://arxiv.org/abs/1902.04023) · Gil Tene, "How NOT to Measure Latency"
