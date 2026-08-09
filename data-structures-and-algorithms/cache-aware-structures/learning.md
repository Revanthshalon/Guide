# Cache-Aware & Cache-Oblivious Structures — Learning Notes

## Mental Model

**When two structures have the same asymptotic complexity, the one that matches the memory hierarchy wins — and the gap can be larger than an asymptotic improvement would give you.**

This stage's premise is that the RAM model (unit-cost random access) is a fiction. The honest model counts **block transfers**: memory moves in units of B — a 64-byte cache line, a 4 KB page — and an algorithm's cost is how many of those it triggers. That's the **I/O model** from [complexity analysis](../complexity-analysis/learning.md), and taking it seriously is what produced [B-trees](../b-trees/learning.md) (measured there: `BTreeMap` beat a well-shaped binary BST 2× at identical asymptotics).

The two design philosophies:

- **Cache-aware:** the structure knows B and is tuned to it. B-trees pick a fanout that fills a page; blocked matrix multiplication picks a tile that fills L1. Optimal for the level you tuned, and needs retuning per machine.
- **Cache-oblivious:** the structure performs well at *every* level of the hierarchy simultaneously, without knowing B. Achieved by recursive decomposition — van Emde Boas layout, cache-oblivious B-trees, recursive matrix multiply. The recursion eventually produces subproblems that fit whatever B happens to be, at every level at once.

Now the measurement, which closes an open question filed back in Stage 2. Eytzinger layout stores a sorted array in BFS order of its implicit binary search tree, so a node's children are adjacent in memory. Binary search over 1,000,000 random `u32` queries:

| n | `std::binary_search` | Branchless binary | Eytzinger (no prefetch) | **Eytzinger + prefetch** |
| --- | --- | --- | --- | --- |
| 100,000 | 32.37 ms | 36.42 ms | 32.43 ms | **26.81 ms** |
| 1,000,000 | **24.55 ms** | 36.83 ms | 32.20 ms | 28.38 ms |
| 10,000,000 | 128.41 ms | 173.41 ms | 141.02 ms | **76.68 ms** |
| 50,000,000 | 226.87 ms | 322.22 ms | 269.28 ms | **120.68 ms** |

**The layout alone is not the win — the prefetch is.** Plain Eytzinger *loses* to `std::binary_search` at every size above 100k (269 ms vs 227 ms at n = 50M). Add a prefetch of the great-grandchildren and it becomes **1.88× faster** at 50M. And branchless binary search, often recommended, was the worst of the four throughout.

That's the lesson of this whole topic in one table: **the reordering only pays because it makes the *next* accesses predictable enough to prefetch.** Reorganizing data without exploiting the resulting predictability buys nothing.

## The Invariant

**The I/O (external memory) model:**

> Memory has two levels: a fast cache of size M and slow storage, transferred in blocks of B. Cost = number of block transfers. Computation within the cache is free.

Under this model the fundamental bounds change:

| Operation | RAM model | **I/O model** |
| --- | --- | --- |
| Scan n elements | Θ(n) | **Θ(n/B)** |
| Sort n elements | Θ(n log n) | **Θ((n/B) log_{M/B}(n/B))** |
| Search (binary, sorted array) | Θ(log n) | Θ(log n) — **B doesn't help** |
| Search (B-tree) | Θ(log n) | **Θ(log_B n)** |
| Permute n elements | Θ(n) | Θ(min(n, (n/B) log_{M/B}(n/B))) |

The third and fourth rows are the whole argument: binary search on a sorted array touches log n *scattered* positions, so a block transfer delivers one useful element. A B-tree's node fills a block, so a transfer delivers B useful comparisons.

**Eytzinger's invariant:**

> The sorted array is stored in BFS order of the implicit search tree: `a[1]` is the root, `a[k]`'s children are `a[2k]` and `a[2k+1]`.

Consequences: the first few levels occupy the first few cache lines and stay resident (so the top of every search is free), and a node's children are *adjacent*, so one line holds several potential next steps. But the search loses the ability to stop early on an exact match — you always descend to depth log n and recover the answer from the path — which is part of why plain Eytzinger loses.

## Mechanics

### Layouts for binary search

| Layout | Order | Property |
| --- | --- | --- |
| **Sorted** | in-order | Range scans are contiguous; probes are scattered |
| **Eytzinger** | BFS | Children adjacent; prefetchable; **no range scans** |
| B-tree layout | B-ary levels | log_B n transfers; what `BTreeMap` does |
| **van Emde Boas** | recursive halving of the tree's *height* | **Cache-oblivious** — optimal at every B simultaneously |

The van Emde Boas layout is the elegant one: recursively split the tree at half its *height*, lay out the top half, then each bottom subtree contiguously. Any subtree of height ≤ log B lands in one block regardless of what B is — which is why it needs no tuning.

```rust
// Eytzinger search: descend to a leaf, then recover the answer from the path.
let mut k = 1usize;
while k < a.len() {
    let p = k * 16;                                    // ← the prefetch is the point
    if p < a.len() { unsafe { std::ptr::read_volatile(a.as_ptr().add(p)); } }
    k = 2 * k + (a[k] < x) as usize;                   // branchless descent
}
let j = k >> ((!k).trailing_zeros() + 1);              // last left-turn = lower bound
```

The `k * 16` prefetch distance is tuned to fetch the cache line holding the great-grandchildren — four levels ahead, so the data arrives before the descent needs it. Rust has no stable prefetch intrinsic on aarch64, so the `read_volatile` above is a stand-in; on x86 use `_mm_prefetch`.

### The other cache-aware techniques

**Blocking / tiling.** Naive matrix multiply streams entire rows and columns; a tiled version processes b×b blocks that fit in cache, reducing transfers from Θ(n³) to Θ(n³/(B√M)). Same arithmetic, an order of magnitude fewer misses.

**Structure-of-arrays.** Splitting `Vec<Struct>` into parallel arrays so a pass touching one field doesn't drag the others through cache — the [data-oriented design](../../performance-optimization/data-oriented-design/learning.md) move, and the same reason [graph representations](../graph-representations/learning.md) keeps `tgt` and `wt` separate.

**Implicit structures.** A [binary heap](../heaps-and-priority-queues/learning.md) has no pointers because a complete tree's shape is determined by its size. Same for Fenwick trees and Eytzinger. **Removing pointers removes both memory and dependent loads.**

**Packed / succinct structures.** Rank-select bit vectors, wavelet trees, and FM-indexes ([suffix structures](../suffix-structures/learning.md)) store data in near-information-theoretic space, so more of the structure fits in cache — trading a few instructions per access for far fewer transfers.

### Where the recurring pattern shows up

Across this whole category, **the flat contiguous structure keeps beating the pointer-based one at equal asymptotics**:

| Comparison | Measured | Where |
| --- | --- | --- |
| `Vec` vs scattered linked list | **641×** | [linked lists](../linked-lists/learning.md) |
| CSR vs `Vec<Vec<_>>` (BFS) | 1.76× | [graph representations](../graph-representations/learning.md) |
| `BTreeMap` vs binary BST | ~2× | [b-trees](../b-trees/learning.md) |
| Sorted `Vec` vs uncompressed trie | 11× | [tries](../tries-and-radix-trees/learning.md) |
| Suffix array vs suffix tree | 3–5× less memory | [suffix structures](../suffix-structures/learning.md) |
| Bitset vs `Vec<bool>` | 79× | [bit manipulation](../bit-manipulation/learning.md) |

That is not six coincidences. It's one mechanism observed six times.

## Complexity

| Structure/operation | Time | **Transfers** | Cache-oblivious? |
| --- | --- | --- | --- |
| Sorted array, binary search | Θ(log n) | Θ(log n) | — |
| Eytzinger + prefetch | Θ(log n) | Θ(log n), **overlapped** | no (prefetch distance is tuned) |
| B-tree | Θ(log n) | **Θ(log_B n)** | no (fanout tuned to B) |
| van Emde Boas layout | Θ(log n) | **Θ(log_B n)** | **yes** |
| Naive matrix multiply | Θ(n³) | Θ(n³/B) | — |
| Tiled matrix multiply | Θ(n³) | **Θ(n³/(B√M))** | no |
| Recursive matrix multiply | Θ(n³) | Θ(n³/(B√M)) | **yes** |
| External merge sort | Θ(n log n) | **Θ((n/B) log_{M/B}(n/B))** | no |

**Where the table misleads.** Every row has the same time complexity as its naive counterpart — these are *entirely* constant-factor techniques in the RAM model, and their whole justification lives in a column the RAM model doesn't have. Measured: Eytzinger + prefetch is 1.88× at n = 50M with identical Θ(log n).

And the negative result matters as much: **Θ(log n) transfers "overlapped" is doing the work in that table.** Plain Eytzinger has exactly the same transfer count as `std::binary_search` and is *slower*, because it also loses early exit. Prefetching doesn't reduce transfers — it hides their latency by issuing them ahead of use. Counting transfers alone would predict no difference.

## Use Cases

- **Database indexes** — B+ trees tuned to the page size; this is the original motivation for the entire I/O model.
- **Read-only lookup tables** — Eytzinger or a B-tree layout for large static tables queried in a hot loop: IP-to-ASN maps, symbol tables, timestamp indexes.
- **Numeric kernels** — tiled BLAS is the reason a tuned GEMM is 10× a naive triple loop at identical FLOP counts.
- **Graph processing** — CSR plus [direction-optimizing BFS](../graph-traversal/learning.md); frameworks like Ligra are largely locality engineering.
- **Text indexes** — FM-indexes and wavelet trees fit a genome index in memory where a suffix tree wouldn't ([suffix structures](../suffix-structures/learning.md)).
- **External sorting** — the k-way merge phase is chosen precisely to minimize passes over disk.
- **Columnar analytics** — column stores are structure-of-arrays at file scale; a scan of one column doesn't read the others.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Sorted array + `binary_search`** | Default. Range scans, simple, and hard to beat below ~10⁶ |
| **Eytzinger + prefetch** | Static table, n ≳ 10⁷, lookups dominate — measured **1.88×** at 50M |
| **B-tree layout** | You also need updates or range scans |
| van Emde Boas | Multiple hierarchy levels matter and you can't tune per machine |
| Tiling | Dense numeric kernels — or just call a tuned BLAS |
| Structure-of-arrays | A pass touches a subset of fields |
| Implicit (pointer-free) | The shape is derivable from the size — heaps, Fenwick |
| **Nothing** | n is small, or the structure isn't in the profile |

## Pitfalls in Depth

### Pitfall: Reordering data without exploiting the predictability

- **What goes wrong:** Eytzinger layout is implemented because the paper reports large speedups, and it comes out **slower** than `std::binary_search` — measured 269.28 ms against 226.87 ms at n = 50,000,000, and worse at every size above 100,000.
- **Why it happens (the mechanism):** The layout's benefit is not fewer transfers — it has the same Θ(log n) probe count — but that consecutive probes become *predictable*, so they can be issued before they're needed. Without an explicit prefetch, you get the same number of dependent cache misses as sorted binary search, plus you lose early exit on an exact match (Eytzinger always descends to a leaf) and pay the trailing-zeros path reconstruction. The reorganization alone is a net cost.
- **How to handle it in production, and why that works:** Add the prefetch, tuned to fetch several levels ahead (`k * 16` fetches the great-grandchildren's line). Measured, that turns a 0.84× loss into a **1.88× win** at n = 50M. On x86 use `_mm_prefetch`; Rust has no stable aarch64 prefetch intrinsic, so a volatile read is the portable stand-in.
- **Trade-offs of the fix:** The prefetch distance is machine- and size-dependent — too near and the data hasn't arrived, too far and you evict something useful, so it needs measuring rather than deriving. Eytzinger also gives up **range scans** entirely (the array is no longer in sorted order) and requires a full rebuild on any insertion, so it's only for static lookup tables.

### Pitfall: Optimizing layout when the structure isn't the bottleneck

- **What goes wrong:** Significant effort goes into a cache-optimal layout for a structure that accounts for 3% of runtime, or one that comfortably fits in L2 so every access already hits. Measured, at n = 1,000,000 (4 MB of `u32`) `std::binary_search` was the *fastest* of all four variants — the fancy layouts only started winning at 10,000,000.
- **Why it happens (the mechanism):** These techniques have dramatic published numbers, which are measured at sizes far beyond cache. Below that, the entire structure is resident and there are no transfers to optimize — the techniques' costs (lost early exit, extra arithmetic, prefetch instructions) remain while their benefit is zero.
- **How to handle it in production, and why that works:** Profile first ([profiling & measurement](../../performance-optimization/profiling-and-measurement/learning.md)): confirm the structure is hot *and* that the hot region shows a high cache-miss rate. Then check the size against the cache hierarchy — if the working set fits in L2, layout optimization has nothing to work with.
- **Trade-offs of the fix:** Profiling adds a step before an optimization that looks obviously beneficial. The counter-risk is real: at genuinely large n the win is 1.88×, which is worth having when it applies.

### Pitfall: Assuming a cache-aware structure transfers between machines

- **What goes wrong:** A fanout, tile size, or prefetch distance is tuned on one machine and shipped. On different hardware — a different cache size, a different line size, an ARM server versus x86, a VM with a different page size — it's mistuned and can be slower than the untuned version.
- **Why it happens (the mechanism):** Cache-aware means *aware of a specific B and M*. Those vary: 64-byte lines are near-universal but L2/L3 sizes differ by an order of magnitude across server, laptop, and phone, and page sizes differ (4 KB vs 16 KB on Apple Silicon). A parameter derived from one configuration is an assumption about all of them.
- **How to handle it in production, and why that works:** Either detect the parameters at runtime and select a tuned variant, or use a **cache-oblivious** structure (van Emde Boas layout, recursive matrix multiply) whose recursive decomposition adapts to every level automatically. Cache-oblivious versions are typically a small constant factor behind a perfectly-tuned cache-aware one and far ahead of a mistuned one.
- **Trade-offs of the fix:** Runtime detection means multiple code paths to maintain and test. Cache-oblivious structures are harder to implement and their recursion has real overhead at small sizes, so they usually need a base-case cutover — the same [divide & conquer](../divide-and-conquer/learning.md) discipline.

### Pitfall: Believing "branchless is faster"

- **What goes wrong:** Binary search is rewritten branchlessly (using arithmetic instead of an `if`) on the theory that eliminating a 50%-mispredicted branch must help. Measured, branchless binary search was the **slowest** of four variants at every size — 322.22 ms against `std::binary_search`'s 226.87 ms at n = 50M.
- **Why it happens (the mechanism):** Removing the branch converts a *control* dependency into a *data* dependency. A mispredicted branch costs ~15 cycles but the CPU speculates past it and issues the next load early; a data dependency serializes the address computation against the previous load's result, so the memory latency (~100 ns out of cache) can no longer be overlapped. When the bottleneck is memory rather than branch misses, branchless is strictly worse.
- **How to handle it in production, and why that works:** Measure both. Branchless wins when the data is cache-resident and branch misprediction dominates; it loses when the loads miss cache, because then latency hiding matters more than branch cost. That's also why Eytzinger's *branchless descent plus prefetch* works — the prefetch restores the memory-level parallelism the branchless form gave up.
- **Trade-offs of the fix:** The right answer is workload-dependent, so this is one more thing to benchmark rather than a rule to apply. `std::binary_search` is already well-tuned and is a good default.
