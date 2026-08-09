# Range Query Structures — Learning Notes

## Mental Model

**These structures all answer one question — "what is the aggregate over `[l, r]`?" — and they differ only in what mix of updates and queries they're willing to pay for.**

Start from the two extremes, which are both trivial:

| Approach | Update a point | Query a range | When it's right |
| --- | --- | --- | --- |
| Plain array | **Θ(1)** | Θ(n) — scan | Updates ≫ queries |
| Prefix-sum array | Θ(n) — rebuild | **Θ(1)** — `p[r] − p[l]` | Queries ≫ updates, data static |

Everything in this topic exists because real workloads sit *between* those, and both extremes degrade to Θ(n·q) when they do. The insight is:

> Precompute aggregates over a **hierarchy of overlapping blocks** instead of over every prefix. Then a range decomposes into Θ(log n) precomputed pieces, and a point update touches only the Θ(log n) blocks containing it.

That single idea produces the Fenwick tree, the segment tree, and sqrt decomposition — they differ in which hierarchy they use and therefore what they can do.

Measured on this machine, 100,000 point updates interleaved with 100,000 prefix-sum queries over an array of 1,000,000:

| Approach | Time |
| --- | --- |
| **Fenwick tree** | **3.9 ms** |
| Naive array + rescan | ~5,623 ms (extrapolated from 112.5 ms for 2,000 pairs) |

**~1,400×.** And that ratio grows linearly with n, because the naive version is Θ(n) per query while the Fenwick is Θ(log n).

The practical filter to apply first, though: **if the data doesn't change, you don't need any of this.** A prefix-sum array is 3 lines and Θ(1) per query. These structures earn their complexity only when updates and queries are interleaved.

## The Invariant

**Fenwick tree (Binary Indexed Tree):**

> `tree[i]` holds the aggregate of the `i & (-i)` elements ending at index `i`. That is, each index is responsible for a range whose length is its lowest set bit.

That one sentence is the entire structure. Index 12 (`0b1100`, lowest set bit 4) covers elements 9–12; index 8 (`0b1000`) covers 1–8; index 7 (`0b0111`, lowest bit 1) covers only 7. Walking `i -= i & (-i)` from any index visits a disjoint set of blocks that exactly tile `[1, i]` — which is why a prefix query is Θ(log n) with no tree, no pointers, and one array of size n.

**Segment tree:**

> Each node stores the aggregate of a contiguous range; the root covers `[0, n)`; a node covering `[l, r)` has children covering `[l, m)` and `[m, r)`.

More general and roughly 2× the memory, but it stores a *real* range per node rather than an implicit one, which is what lets it handle non-invertible operations and lazy propagation.

**Sparse table:**

> `st[k][i]` holds the aggregate of the `2^k` elements starting at `i`.

A query over `[l, r]` is covered by *two overlapping* blocks of size 2^⌊log(r−l+1)⌋. Overlap is fine only if the operation is **idempotent** (min, max, gcd — where combining a value with itself changes nothing), which is exactly the constraint that makes it Θ(1) per query but static.

## Mechanics

### Fenwick tree — the whole implementation is six lines

```rust
struct Fenwick(Vec<i64>);           // 1-indexed internally

impl Fenwick {
    fn add(&mut self, mut i: usize, delta: i64) {      // point update
        i += 1;
        while i < self.0.len() { self.0[i] += delta; i += i & i.wrapping_neg(); }
    }
    fn prefix_sum(&self, mut i: usize) -> i64 {        // sum of [0, i)
        let mut s = 0;
        while i > 0 { s += self.0[i]; i -= i & i.wrapping_neg(); }
        s
    }
    fn range_sum(&self, l: usize, r: usize) -> i64 {   // [l, r)
        self.prefix_sum(r) - self.prefix_sum(l)        // requires INVERTIBILITY
    }
}
```

`i & i.wrapping_neg()` is `i & (-i)` — isolate the lowest set bit. Update walks *up* by adding it; query walks *down* by subtracting it. Both are Θ(log n) with an extremely small constant: no pointers, one contiguous array, and the access pattern is a handful of positions.

The catch is in `range_sum`: subtracting two prefixes **requires an invertible operation**. Sums and XOR work. **Min and max do not** — knowing `min[0,r]` and `min[0,l]` tells you nothing about `min[l,r]`. That's the single most important limitation of the structure, and it's the reason segment trees exist.

### Segment tree — generality, at 2× the memory

Iterative bottom-up form, stored in a flat array of size 2n:

```rust
struct SegTree { n: usize, t: Vec<i64> }               // t[n..2n] = leaves

impl SegTree {
    fn set(&mut self, mut i: usize, v: i64) {
        i += self.n; self.t[i] = v;
        while i > 1 { i /= 2; self.t[i] = self.t[2*i].min(self.t[2*i+1]); }   // any assoc. op
    }
    fn query(&self, mut l: usize, mut r: usize) -> i64 {     // [l, r)
        let (mut res_l, mut res_r) = (i64::MAX, i64::MAX);
        l += self.n; r += self.n;
        while l < r {
            if l & 1 == 1 { res_l = res_l.min(self.t[l]); l += 1; }
            if r & 1 == 1 { r -= 1; res_r = self.t[r].min(res_r); }
            l /= 2; r /= 2;
        }
        res_l.min(res_r)
    }
}
```

It works for any **associative** operation — min, max, gcd, matrix product, "sum and count of maxima" — with no invertibility requirement. Keeping the left and right partial results separate matters for **non-commutative** operations (matrix products, string concatenation), which is easy to get wrong.

### Lazy propagation — the reason to prefer segment trees

A Fenwick tree does point updates. A segment tree with **lazy propagation** does *range* updates ("add 5 to everything in [l, r]") in Θ(log n) by storing a pending operation at a node and only pushing it down when a query descends through. That turns Θ(n) range updates into Θ(log n) and is the capability that makes segment trees the general answer.

### Sqrt decomposition — the one you can always derive

Split the array into blocks of size ⌊√n⌋ and keep a per-block aggregate. A query sums whole blocks in the middle and scans partial blocks at the ends: Θ(√n). An update touches one element and one block aggregate: Θ(1).

At n = 10⁶ that's ~1,000 operations per query versus a segment tree's ~20 — much worse asymptotically. It's worth knowing anyway because it's trivially adaptable to weird operations (mode of a range, count of distinct values) where no logarithmic structure is obvious, and Mo's algorithm builds on it for offline query batching.

### Choosing: the decision table

| You need | Use | Why |
| --- | --- | --- |
| Static data, sum/any invertible op | **Prefix sums** | Θ(1) query, 3 lines — don't over-engineer |
| Static data, min/max/gcd | **Sparse table** | Θ(1) query; overlap is safe for idempotent ops |
| Point update + prefix/range **sum** | **Fenwick** | Simplest, smallest, fastest constant |
| Point update + **min/max/any associative** | **Segment tree** | Fenwick can't — not invertible |
| **Range** update + range query | **Segment tree + lazy** | The general answer |
| Weird aggregate, no clean structure | **Sqrt decomposition** | Θ(√n) but always derivable |
| 2-D range sums | **2-D Fenwick** | Θ(log² n), still simple |
| Range updates, point queries only | **Fenwick over a difference array** | Cheapest trick in the topic |

That last row is worth knowing: to support "add v to [l, r]" with point queries, keep a Fenwick over the *difference* array — `add(l, +v)`, `add(r+1, −v)` — and a point query becomes a prefix sum. Two calls, no lazy propagation.

## Complexity

| Structure | Build | Point update | Range query | Range update | Space |
| --- | --- | --- | --- | --- | --- |
| Plain array | Θ(n) | **Θ(1)** | Θ(n) | Θ(n) | n |
| Prefix sums | Θ(n) | Θ(n) | **Θ(1)** | Θ(n) | n |
| **Fenwick** | Θ(n) | **Θ(log n)** | **Θ(log n)** | Θ(log n)* | **n** |
| **Segment tree** | Θ(n) | Θ(log n) | Θ(log n) | Θ(n) | 2n |
| Segment tree + lazy | Θ(n) | Θ(log n) | Θ(log n) | **Θ(log n)** | 2n + lazy |
| Sparse table | Θ(n log n) | **rebuild** | **Θ(1)** † | — | n log n |
| Sqrt decomposition | Θ(n) | Θ(1) | Θ(√n) | Θ(√n) | n + √n |

`*` via a difference array, point queries only  ·  `†` idempotent operations only

**Where the table misleads.** The Fenwick and segment tree share Θ(log n), and the Fenwick is several times faster in practice: one array of size n versus 2n, ~log n array accesses at bit-determined positions versus a tree descent, and no recursion. When both apply, take the Fenwick. The segment tree earns its cost only when you need non-invertible operations or lazy range updates.

Also note the sparse table's Θ(n log n) space — at n = 10⁶ that's ~20M entries. It buys Θ(1) queries, but only for static, idempotent aggregates.

## Rust Implementation

```rust
// The check to run FIRST: does the data change?
let prefix: Vec<i64> = std::iter::once(0)
    .chain(data.iter().scan(0i64, |a, &x| { *a += x; Some(*a) }))
    .collect();
let range_sum = prefix[r] - prefix[l];              // Θ(1). Done. Stop here if static.

// Range update + point query, without lazy propagation:
// keep a Fenwick over the DIFFERENCE array.
fen.add(l, v);
fen.add(r, -v);                    // r exclusive
let value_at_i = fen.prefix_sum(i + 1);

// 2-D Fenwick: nest the loops, Θ(log² n).
fn add2(t: &mut Vec<Vec<i64>>, mut i: usize, j0: usize, v: i64) {
    i += 1;
    while i < t.len() {
        let mut j = j0 + 1;
        while j < t[i].len() { t[i][j] += v; j += j & j.wrapping_neg(); }
        i += i & i.wrapping_neg();
    }
}
```

**Watch the index base.** Fenwick trees are naturally 1-indexed (index 0 has no lowest set bit, so `i -= i & -i` would loop forever). Keep the 1-indexing *internal* and expose a 0-indexed API, as above — mixing the two at call sites is the most common bug in this topic.

**Crates:** there is no dominant Rust crate here, and that's appropriate — these structures are 20–60 lines each and are usually specialized to the operation. `ndarray` for the 2-D array underneath; `superslice` for the binary-search helpers around them.

## Use Cases

- **Analytics over a live window** — "sum of events in this time range" where events keep arriving. The interleaved update/query pattern that gave the measured 1,400×.
- **Order statistics over a value domain** — a Fenwick over value-buckets counts "how many items ≤ x", giving rank queries and k-th smallest in Θ(log n). This is the counting-based cousin of [selection](../selection-and-order-statistics/learning.md), and it works on a *changing* set.
- **Counting inversions** — sweep the array, query "how many already-seen values exceed this one." Θ(n log n) with a Fenwick.
- **Range-minimum queries** — sparse table when static (Θ(1) queries); segment tree when not. RMQ underpins lowest-common-ancestor via Euler tour (Stage 5).
- **Competitive programming and query-heavy interview problems** — this topic is disproportionately represented there, which is worth knowing if that's a goal.
- **Time-series rollups and histograms** — 2-D Fenwick for "count of events in this (time, value) rectangle."
- **Coordinate compression + Fenwick** — the standard combination when the value domain is sparse but the index space is huge.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Prefix sums** | Data is static. Do this first, always |
| **Fenwick** | Point updates + prefix/range sums (or any invertible op) |
| **Segment tree** | Non-invertible op (min/max/gcd) or you need lazy range updates |
| Segment tree + lazy | Range update **and** range query |
| Sparse table | Static + idempotent (min/max/gcd), want Θ(1) queries |
| Sqrt decomposition | The aggregate has no clean logarithmic structure |
| Fenwick over difference array | Range update, point query — two lines, no lazy needed |
| `BTreeMap::range` | The *keys* are sparse and you want the elements, not an aggregate |

## Pitfalls in Depth

### Pitfall: Building a Fenwick tree for min or max

- **What goes wrong:** A Fenwick is built for range-minimum queries by analogy with sums. Prefix-min works. `range_min(l, r)` computed as some combination of `prefix_min(r)` and `prefix_min(l)` returns wrong answers on most inputs — often *plausible* ones, since a min of a larger range is frequently the same as the min of the smaller one, so it passes casual testing and fails on the cases that matter.
- **Why it happens (the mechanism):** `range_sum(l, r) = prefix(r) − prefix(l)` works because addition has an **inverse**. Min does not: knowing `min[0,r] = 3` and `min[0,l] = 3` tells you nothing about `min[l,r]` — the 3 might lie entirely inside `[0,l)`. The Fenwick's whole query strategy is prefix decomposition plus subtraction, so it is structurally limited to invertible operations.
- **How to handle it in production, and why that works:** Use a segment tree, which stores a genuine aggregate per range node and answers a query by *combining* Θ(log n) disjoint nodes rather than subtracting two prefixes — no inverse required, so any associative operation works. If the data is static, a sparse table is better still (Θ(1) queries), because min is idempotent and overlapping blocks are safe.
- **Trade-offs of the fix:** A segment tree is 2n memory versus the Fenwick's n, has a measurably larger constant, and is more code. A sparse table costs Θ(n log n) space and can't be updated at all. There's a real tax for giving up invertibility — it's just not optional when the operation demands it.

### Pitfall: Reaching for a segment tree when the data is static

- **What goes wrong:** A range-sum requirement produces a 200-line segment tree with lazy propagation, when the underlying data is loaded once and never modified. A three-line prefix-sum array would answer every query in Θ(1) — *faster* than the segment tree's Θ(log n) — with no update machinery, no recursion, and nothing to get wrong.
- **Why it happens (the mechanism):** "Range query" triggers pattern-matching to "segment tree" because that's how the topic is taught. But the entire hierarchy exists to make *updates* cheap; if there are no updates, you're paying for a capability you don't use, and paying in the query path.
- **How to handle it in production, and why that works:** Ask "does the underlying data change between queries?" first. No → prefix sums (invertible ops) or a sparse table (min/max/gcd). Yes → then ask which operation and whether updates are point or range, and pick from the decision table. This ordering eliminates most of the topic for most real problems.
- **Trade-offs of the fix:** If the data turns out to change later, a prefix-sum array is Θ(n) to rebuild, so a workload that mutates even occasionally may want the tree anyway. The deciding number is the ratio of updates to queries — with rare updates, rebuild-on-write can still beat a tree.

### Pitfall: Off-by-one from Fenwick's 1-indexing

- **What goes wrong:** Fenwick trees are naturally 1-indexed, but Rust arrays are 0-indexed. Mixing them produces answers that are correct for most ranges and wrong at the boundaries — or an **infinite loop**, because at `i = 0` the expression `i -= i & (-i)` subtracts zero forever.
- **Why it happens (the mechanism):** The structure depends on the lowest set bit, and 0 has none. Every published implementation is 1-indexed for that reason, so translated code carries a hidden convention that isn't visible at the call site. Then some call sites add 1 and some don't.
- **How to handle it in production, and why that works:** Keep the 1-indexing strictly *inside* the type and expose a 0-indexed, half-open API (`add(i, v)`, `sum(l, r)` for `[l, r)`) — the conversion happens in one place, so it can only be wrong once. Match std's half-open range convention so the API composes with Rust's ranges.
- **Trade-offs of the fix:** Slightly more code inside the type, and internal code must remember it's in the 1-indexed world. Worth it: this is the most common bug in the topic, and it's entirely preventable by encapsulation.

### Pitfall: Non-commutative operations combined in the wrong order

- **What goes wrong:** A segment tree over matrix products, string concatenations, or function compositions returns garbage. Sums and mins hide this completely because they're commutative, so the bug appears only when the structure is reused for a non-commutative operation — usually long after it was written and trusted.
- **Why it happens (the mechanism):** The iterative bottom-up query walks in from both ends simultaneously. If both sides accumulate into one variable, the pieces get combined out of order. For `+` or `min` that's harmless; for matrix multiply, `A·B ≠ B·A`, and the answer is silently wrong.
- **How to handle it in production, and why that works:** Keep **separate left and right accumulators** and combine them at the end in the correct order (as in the snippet above), so left-side pieces are always combined left-to-right and right-side pieces right-to-left. The recursive top-down formulation gets this right naturally, which is a reason to prefer it when the operation isn't commutative.
- **Trade-offs of the fix:** Two accumulators and an identity element for each side — slightly more code and one more thing to get right (the identity must genuinely be an identity). The recursive form is clearer but has function-call overhead and recursion depth, though Θ(log n) depth is never a stack risk.

### Pitfall: Lazy propagation that isn't pushed down consistently

- **What goes wrong:** A lazy segment tree returns stale values, or applies an update twice. The failures are input-order dependent and often appear only after a specific interleaving of overlapping range updates and queries, making them very hard to reproduce.
- **Why it happens (the mechanism):** Lazy propagation maintains a second invariant — "a node's stored aggregate is correct for its range, *assuming* all pending updates at its ancestors have been applied." Every path that reads or writes a node must push pending updates down first, and every path that modifies a child must recompute the parent afterwards. Miss one of those in one code path and the invariant breaks locally, then spreads.
- **How to handle it in production, and why that works:** Funnel *all* node access through `push_down(node)` and `pull_up(node)` helpers and never touch `t[node]` directly — that makes it structurally impossible to skip. Then write a `#[cfg(test)]` checker that recomputes every node's aggregate from its children (after a full push-down) and compare against a brute-force array, driven by `proptest` over random interleaved update/query sequences. That's the same discipline as the doubly-linked list in [linked lists](../linked-lists/learning.md) and the augmented BST in [binary search trees](../binary-search-trees/learning.md).
- **Trade-offs of the fix:** The helpers add a function call on every node visit, measurable in tight loops. The brute-force checker is Θ(n) per assertion so it stays behind `cfg(test)`. Both are cheap next to debugging an intermittently-wrong aggregate in production.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if updates kept old versions queryable? | **Persistent segment tree** — query the array as of any past version; the basis of "k-th smallest in range" |
| Batch it | What if queries were known in advance? | **Mo's algorithm** — sort queries to reorder work, Θ((n+q)√n); offline beats online |
| Approximate it | What if the aggregate were approximate? | Sketches (Count-Min, t-digest) — Θ(1) space per stream, not per element (Stage 8) |
| Randomize it | What if the block boundaries were random? | Skip-list-based range structures; treap with subtree aggregates |
| Externalize it | What if the array were on disk? | B+ tree with per-node aggregates; columnar zone maps / min-max indexes |
| Parallelize it | Where's the independence? | Segment-tree build and disjoint-range queries are embarrassingly parallel; prefix sums become a **scan** (Θ(log n) depth) |
| Invert it | What if you stored differences instead of values? | **Difference array** — range update becomes two point updates |
| Augment it | What does a second aggregate per node buy? | (min, count-of-min); (sum, max-prefix-sum) → maximum-subarray in Θ(log n) per query |
| Specialize it | What if the operation were idempotent? | **Sparse table** — overlapping blocks are safe, giving Θ(1) queries |
| Amortize it | What if you rebuilt periodically? | Sqrt decomposition with periodic rebuild; the LSM-style buffer-then-merge pattern |

**Questions:**

1. Fenwick handles sums but not min. State the exact algebraic property that's missing, and explain why the segment tree doesn't need it.
2. A sparse table answers queries in Θ(1) using *overlapping* blocks. Which property makes overlap safe, and give an operation where it silently breaks.
3. Under "invert it", a difference array turns a range update into two point updates. Derive it, then explain why the corresponding query becomes a prefix sum.
4. Under "augment it", storing `(sum, max_prefix, max_suffix, max_subarray)` per node solves maximum-subarray for arbitrary ranges. Give the combine rule for two children, and state the general condition for an augmentation to be mergeable.
5. Sqrt decomposition is Θ(√n) — ~1,000 operations at n = 10⁶ versus a segment tree's ~20. Name two situations where you'd still choose it.
6. Under "batch it", Mo's algorithm reorders *queries*. What does that buy that no online structure can, and what does it cost?
7. Under "parallelize it", prefix sums become a parallel scan with Θ(log n) depth and Θ(n) work. Sketch the two-phase algorithm and say why the sequential version has no parallelism at all.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the Fenwick invariant in one sentence using "lowest set bit."
2. Give the measured Fenwick-vs-naive numbers and explain why the ratio grows with n.
3. Why can't a Fenwick tree do range-min? What replaces it, and what does that cost?
4. Give the decision rule for prefix sums vs Fenwick vs segment tree vs sparse table, in the order you'd apply it.
5. Explain the difference-array trick for range updates with point queries.
6. Why is `i -= i & (-i)` an infinite loop at `i = 0`, and how do you prevent it structurally?

Build exercises:

- Implement a Fenwick tree with a 0-indexed, half-open public API and 1-indexed internals. Property-test `range_sum` against a brute-force scan over random update/query sequences. Then reproduce the measurement: 100k updates interleaved with 100k queries over n = 10⁶, against the naive rescan. The ~1,400× is the argument for the whole topic.
- Implement an iterative segment tree for range-min, then deliberately try to build the same thing as a Fenwick and watch `range_min` produce wrong answers. Finding the counterexample yourself makes the invertibility requirement permanent.
- Add lazy propagation for range-add/range-sum. Write the invariant checker (recompute every node from its children after a full push-down) and drive it with `proptest`. Then remove one `push_down` call and confirm the checker localizes it.
- Implement the "maximum subarray in a range" segment tree with the four-tuple augmentation, and verify against Kadane's algorithm on random ranges. This is the clearest demonstration that the aggregate can be much richer than a sum.

## Open Questions

- Where exactly does a Fenwick beat a segment tree on this machine for pure range-sum, and by how much? Both are Θ(log n) — measure the constant.
- At what n does sqrt decomposition lose to a segment tree in practice, given the much better constant per operation?
- Is there a Rust crate worth using for these, or is hand-rolling genuinely the norm? (My impression is the latter, but it's untested.)
- 2-D Fenwick vs a proper 2-D segment tree for realistic rectangle-sum workloads — where's the crossover?
- Persistent segment trees in Rust: does the `Rc`-per-node cost make them impractical versus an arena with versioned roots?

## References

- Peter Fenwick, "A New Data Structure for Cumulative Frequency Tables" (1994) — the original BIT paper; short, and the lowest-set-bit derivation is worth reading in the author's own words.
- Al.Cash, ["Efficient and easy segment trees"](https://codeforces.com/blog/entry/18051) — the iterative bottom-up segment tree used above; far simpler than the recursive presentation and the reason the code fits on one screen.
- Bender & Farach-Colton, "The LCA Problem Revisited" (2000) — sparse tables and RMQ, and the reduction between LCA and range-minimum that Stage 5 uses.
- [CP-Algorithms: Fenwick tree / Segment tree](https://cp-algorithms.com/data_structures/fenwick.html) — the most complete practical treatment, including lazy propagation and 2-D variants.
- Related in this repo: [Arrays & Dynamic Arrays](../arrays-and-dynamic-arrays/learning.md) (prefix sums, the structure to try first), [Binary Search Trees](../binary-search-trees/learning.md) (augmentation — the same "maintainable from children" rule), [Heaps & Priority Queues](../heaps-and-priority-queues/learning.md) (the other flat-array implicit tree), [Selection & Order Statistics](../selection-and-order-statistics/learning.md) (Fenwick-over-values as the dynamic rank structure), [Complexity Analysis](../complexity-analysis/learning.md) (why Θ(log n) beats Θ(n) here by 1,400× and growing).
