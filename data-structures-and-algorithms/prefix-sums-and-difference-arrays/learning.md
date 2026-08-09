# Prefix Sums & Difference Arrays — Learning Notes

## Mental Model

**Precompute once so every query is Θ(1).** A prefix-sum array turns "sum of `a[l..r]`" from a Θ(n) scan into one subtraction:

```
prefix[i] = a[0] + a[1] + … + a[i-1]        (prefix[0] = 0)
sum(l..r) = prefix[r] - prefix[l]
```

Three lines of setup, Θ(n) once, Θ(1) forever. **This is the first thing to try for any range-query problem**, and it's frequently the whole answer — [range query structures](../range-query-structures/learning.md) exists only because prefix sums can't handle *updates*.

The **difference array** is the exact dual, and it's the less-known half:

```
diff[i] = a[i] - a[i-1]
range_update(l, r, +v):  diff[l] += v;  diff[r] += -v      // two point updates
a = prefix_sum(diff)                                        // recover the array
```

So: **prefix sums make range *queries* Θ(1) at the cost of Θ(n) updates; difference arrays make range *updates* Θ(1) at the cost of Θ(n) queries.** They're inverse operations — prefix-sum and difference are discrete integration and differentiation, and each undoes the other.

That duality is the whole topic, and it generalizes: whenever you face "many range updates, then read the final array once", the difference array is Θ(n + u) where the naive approach is Θ(n·u). Whenever you face "build once, many range queries", prefix sums are Θ(n + q) against Θ(n·q).

The measured contrast with the structure you'd otherwise reach for ([Stage 4](../range-query-structures/learning.md)): 100,000 interleaved updates and queries over 1,000,000 elements cost **3.9 ms with a Fenwick tree** against an extrapolated **~5,623 ms** rescanning a plain array. Prefix sums sit at the other extreme — unbeatable when there are *no* updates, useless when there are many.

## The Invariant

> `prefix[i]` is the aggregate of `a[0..i]` — a **half-open** prefix, with `prefix[0]` the identity element. The array has **n+1** entries.

Two consequences that eliminate most bugs:

- **The n+1 length and `prefix[0] = 0` are not conventions, they're what makes `prefix[r] - prefix[l]` work for every `l ≤ r`**, including the empty range and ranges starting at 0. An n-length prefix array forces a special case at `l = 0`, which is where the off-by-one bugs live.
- **The operation must be invertible.** `sum(l..r) = prefix[r] - prefix[l]` needs subtraction. Sums, XOR, and products (of non-zero values) work; **min and max do not** — knowing `min(0..r)` and `min(0..l)` tells you nothing about `min(l..r)`. That's the same limitation as the Fenwick tree, and the reason segment trees exist.

For the difference array:

> `diff[i] = a[i] - a[i-1]` (with `a[-1] = 0`), so `a[i] = Σ diff[0..=i]`. A range update `[l, r)` by `+v` is exactly `diff[l] += v; diff[r] -= v`.

## Mechanics

### 1-D prefix sums

```rust
// Build: n+1 entries, prefix[0] = 0.
let mut prefix = Vec::with_capacity(a.len() + 1);
prefix.push(0i64);
for &x in a { prefix.push(prefix.last().unwrap() + x as i64); }

let range_sum = prefix[r] - prefix[l];        // [l, r), no special case at l = 0
```

### 2-D prefix sums — inclusion–exclusion

```rust
// p[i][j] = sum of the rectangle [0..i) × [0..j)
for i in 1..=rows {
    for j in 1..=cols {
        p[i][j] = a[i-1][j-1] + p[i-1][j] + p[i][j-1] - p[i-1][j-1];   // ← subtract the double-count
    }
}
// Rectangle sum [r1..r2) × [c1..c2):
let s = p[r2][c2] - p[r1][c2] - p[r2][c1] + p[r1][c1];
```

The `- p[i-1][j-1]` in the build and the `+ p[r1][c1]` in the query are the same inclusion–exclusion correction: the overlapping corner is counted twice by the two subtractions, so it's added back once. Drawing the four rectangles once makes this permanent.

### Difference arrays — range update, point query

```rust
// Apply u range updates in Θ(u), then materialize in Θ(n).
for &(l, r, v) in &updates { diff[l] += v; diff[r] -= v; }     // r exclusive
let mut running = 0;
for i in 0..n { running += diff[i]; a[i] = running; }
```

This is the "many updates then read once" pattern — Θ(n + u) instead of Θ(n·u). Common in scheduling ("add 1 to every hour in this booking"), rendering, and interval counting.

**2-D difference arrays** work the same way with four corner updates:

```rust
diff[r1][c1] += v;  diff[r1][c2] -= v;  diff[r2][c1] -= v;  diff[r2][c2] += v;
// then a 2-D prefix sum recovers the array
```

### Prefix sums beyond addition

| Operation | Prefix form | Range query | Works? |
| --- | --- | --- | --- |
| Sum | running sum | `p[r] - p[l]` | ✅ |
| XOR | running XOR | `p[r] ^ p[l]` | ✅ (self-inverse) |
| Product | running product | `p[r] / p[l]` | ⚠️ zeros break it |
| Count of a predicate | running count | `p[r] - p[l]` | ✅ |
| **Min / max** | — | — | ❌ **not invertible** — use a sparse table |
| GCD | — | — | ❌ use a sparse table |

XOR prefix sums are worth knowing: because `x ^ x = 0`, XOR is its own inverse, so `xor(l..r) = p[r] ^ p[l]`. That's the basis of several "find the missing/duplicate number" tricks.

### Prefix sums + hash map — the pattern that handles negatives

The single most useful application, and the fix for the [sliding-window](../two-pointers-and-sliding-window/learning.md) limitation:

```rust
// Count subarrays with sum exactly k — works with NEGATIVE numbers, unlike a sliding window.
let mut seen: HashMap<i64, usize> = HashMap::from([(0, 1)]);   // empty prefix seen once
let (mut running, mut count) = (0i64, 0usize);
for &x in a {
    running += x;
    count += seen.get(&(running - k)).copied().unwrap_or(0);   // how many earlier prefixes work
    *seen.entry(running).or_insert(0) += 1;
}
```

The insight: `sum(l..r) == k` iff `prefix[r] - prefix[l] == k` iff `prefix[l] == prefix[r] - k`. So counting subarrays becomes counting *earlier prefix values*, which a hash map does in Θ(1). **Θ(n), sign-agnostic**, and it generalizes to "sum divisible by k" (key on `running % k`) and "equal counts of two symbols" (key on the running difference).

## Complexity

| Structure | Build | Point update | Range query | Range update |
| --- | --- | --- | --- | --- |
| Plain array | Θ(n) | **Θ(1)** | Θ(n) | Θ(n) |
| **Prefix sums** | Θ(n) | Θ(n) rebuild | **Θ(1)** | Θ(n) |
| **Difference array** | Θ(n) | Θ(1) | Θ(n) materialize | **Θ(1)** |
| 2-D prefix sums | Θ(n·m) | Θ(n·m) | **Θ(1)** | — |
| [Fenwick](../range-query-structures/learning.md) | Θ(n) | Θ(log n) | Θ(log n) | Θ(log n)* |
| [Segment tree](../range-query-structures/learning.md) | Θ(n) | Θ(log n) | Θ(log n) | Θ(log n) lazy |

`*` via a difference array underneath

**Where the table misleads.** Prefix sums' Θ(1) query is genuinely Θ(1) — two array reads and a subtraction, no branches, perfectly cache-friendly. A Fenwick tree's Θ(log n) is ~20 scattered accesses at n = 10⁶. So when the data is static, prefix sums aren't just asymptotically better than a tree, they're better by a large constant too. **The moment updates appear, the ranking inverts completely** — which is why "does the data change?" is the first question, not the last.

Overflow is the practical limit: summing 10⁶ values near `i32::MAX` overflows `i32` immediately. Prefix arrays should almost always be `i64`/`u64` even when the data is 32-bit.

## Use Cases

- **Range sum queries on static data** — the default. Analytics over a fixed dataset, cumulative distributions, running totals.
- **Subarray-sum problems** — "count subarrays summing to k", "longest subarray with equal 0s and 1s", "subarray sum divisible by k". All are prefix-sum-plus-hash-map.
- **2-D image and grid queries** — integral images in computer vision compute box filters in Θ(1) per window regardless of window size; this is the foundation of Viola-Jones face detection.
- **Interval counting / booking systems** — difference array for "how many events overlap each hour".
- **Rendering and painting** — difference arrays for range-add operations applied in bulk, then materialized once.
- **Probability and sampling** — a prefix sum of weights plus a binary search gives weighted random sampling in Θ(log n).
- **Rate limiting and windowed metrics** — cumulative counters where a window count is a difference of two cumulative reads.
- **Preprocessing for DP** — many DP transitions involve a range sum, which prefix sums reduce from Θ(k) to Θ(1) per state ([dynamic programming](../dynamic-programming/learning.md)).

## When to Use Which

| Reach for | When |
| --- | --- |
| **Prefix sums** | Static data, many range queries — **try this first** |
| **Difference array** | Many range updates, then read the array once |
| Both together | Range update + point query (difference array, then prefix-sum it) |
| **Prefix sums + `HashMap`** | Subarray sums with **negatives**, divisibility, balance problems |
| 2-D prefix sums | Static grid, rectangle queries |
| [Sparse table](../range-query-structures/learning.md) | Static, **min/max/gcd** — not invertible, so prefix sums can't |
| [Fenwick tree](../range-query-structures/learning.md) | Interleaved point updates and range sums |
| [Segment tree](../range-query-structures/learning.md) | Interleaved range updates and range queries, or non-invertible ops |

## Pitfalls in Depth

### Pitfall: Integer overflow in the prefix array

- **What goes wrong:** Prefix sums are computed in the element type — `i32` for `i32` data — and the cumulative total exceeds the range long before any individual element does. Summing 10⁶ values averaging 10⁶ needs 10¹², which overflows `i32` at 2.1×10⁹. In release builds it wraps silently and produces plausible-looking negative sums; in debug it panics.
- **Why it happens (the mechanism):** The elements fit comfortably in the type, so the type looks right. But the prefix array's magnitude grows with n, not with the element range — it's a fundamentally larger quantity that happens to be built from the same values.
- **How to handle it in production, and why that works:** Always widen: `i64`/`u64` prefix arrays over `i32` data, `i128` if elements are already 64-bit and n is large. The memory cost is 2× on the prefix array only, and it removes the entire failure mode. Where widening isn't enough (modular problems), take the modulus at each step — but then remember subtraction needs `(p[r] - p[l] + MOD) % MOD` to stay non-negative.
- **Trade-offs of the fix:** Double the memory for the prefix array, which matters at 10⁸ elements and not before. The modular variant adds a branch or an addition per query and is only needed when the answer is genuinely modular.

### Pitfall: Using prefix sums for min or max

- **What goes wrong:** By analogy with sums, someone builds a "prefix min" array and computes `range_min(l, r)` from `prefix_min[r]` and `prefix_min[l]`. There's no correct formula — the result is wrong whenever the overall minimum lies in `[0, l)`, which is common — and the wrongness is plausible because a range's min often *is* the global min.
- **Why it happens (the mechanism):** `prefix[r] - prefix[l]` works because addition has an inverse. Min doesn't: from `min(0..r) = 3` and `min(0..l) = 3` you cannot determine `min(l..r)`, since the 3 might be entirely inside the excluded prefix. The technique is structurally tied to invertibility.
- **How to handle it in production, and why that works:** Static data → **sparse table**, which is Θ(n log n) to build and Θ(1) to query using two *overlapping* blocks; overlap is safe precisely because min is idempotent. With updates → segment tree. Both combine disjoint (or idempotent) precomputed pieces rather than subtracting prefixes.
- **Trade-offs of the fix:** A sparse table costs Θ(n log n) memory — at n = 10⁶ that's ~20M entries — and cannot be updated. A segment tree is 2n memory and Θ(log n) per query rather than Θ(1). Neither is as cheap as a prefix array, which is the price of giving up invertibility.

### Pitfall: Off-by-one from an n-length prefix array

- **What goes wrong:** The prefix array is built with n entries where `prefix[i]` is the sum of `a[0..=i]` (inclusive). Then `range_sum(l, r)` needs `prefix[r] - prefix[l-1]`, which underflows or panics at `l = 0`. A special case is added, and half the call sites forget it.
- **Why it happens (the mechanism):** The inclusive convention feels natural ("prefix[i] is the sum up to i") but it has no slot for the empty prefix, so `l = 0` has nothing to subtract. The half-open convention with `prefix[0] = 0` gives the empty prefix a home, and every range then works uniformly.
- **How to handle it in production, and why that works:** Build **n+1** entries with `prefix[0] = 0` and treat all ranges as half-open `[l, r)`. Then `prefix[r] - prefix[l]` is correct for every `0 ≤ l ≤ r ≤ n` including empty ranges, with no special cases and no possibility of an index underflow. This also matches Rust's slice-range convention, so `&a[l..r]` and `prefix[r] - prefix[l]` describe the same range — a continual consistency check.
- **Trade-offs of the fix:** One extra element and a mental adjustment if you learned the inclusive form. There is no downside; the half-open form is strictly better.

### Pitfall: Rebuilding prefix sums after every update

- **What goes wrong:** Prefix sums are used for a workload that also has updates, and each update triggers a Θ(n) rebuild. With u updates that's Θ(n·u) — measured in [Stage 4](../range-query-structures/learning.md), the naive rescan approach extrapolated to ~5,623 ms where a Fenwick tree took **3.9 ms**, a factor of ~1,400.
- **Why it happens (the mechanism):** Prefix sums are *so* good for the static case that they get adopted first, and updates arrive later as a requirement change. Each rebuild is individually fast enough not to notice, and the quadratic total only appears at scale.
- **How to handle it in production, and why that works:** Ask "does this array change between queries?" before choosing. If yes, use a Fenwick tree (invertible ops) or a segment tree (anything associative) — both give Θ(log n) for *both* operations rather than Θ(1)/Θ(n). If updates are rare and batched, rebuilding is fine — the deciding number is the update-to-query ratio.
- **Trade-offs of the fix:** A Fenwick tree's Θ(log n) query is measurably slower than a prefix array's Θ(1) — ~20 scattered accesses versus two. So for a genuinely static array, switching to a tree is a pessimization. Keep the prefix array when you can.
