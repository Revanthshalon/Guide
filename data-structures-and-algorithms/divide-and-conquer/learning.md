# Divide & Conquer — Learning Notes

## Mental Model

**Split the problem into independent subproblems of the same shape, solve them recursively, combine.** Three steps, and the third is where the algorithm actually lives — splitting is usually trivial, recursion is free, and *combine* is where you either get a good bound or don't.

The distinction from [dynamic programming](../dynamic-programming/learning.md) is exactly one word: **independent**. D&C subproblems don't overlap, so there's nothing to memoize; DP subproblems do overlap, which is why it needs a cache. If you write a D&C and notice subproblems repeating, you've written a DP.

The cost is a recurrence, `T(n) = a·T(n/b) + f(n)` — *a* subproblems of size *n/b*, plus f(n) to split and combine. The Master Theorem reads off the answer by comparing f(n) against `n^(log_b a)`, i.e. **asking whether the leaves or the root dominate**:

| Case | Condition | Result | Example |
| --- | --- | --- | --- |
| Leaves dominate | f(n) = O(n^(log_b a − ε)) | Θ(n^(log_b a)) | Karatsuba: 3T(n/2) + n → Θ(n^1.585) |
| **Balanced** | f(n) = Θ(n^(log_b a)) | Θ(n^(log_b a) · log n) | Merge sort: 2T(n/2) + n → **Θ(n log n)** |
| Root dominates | f(n) = Ω(n^(log_b a + ε)) | Θ(f(n)) | 2T(n/2) + n² → Θ(n²) |

The lever worth internalizing: **reducing `a` — the number of subproblems — changes the exponent.** Naive multiplication does 4 half-size multiplications (Θ(n²)); Karatsuba does **3** with extra additions, giving Θ(n^1.585). Strassen cuts 8 matrix multiplications to 7, giving Θ(n^2.807). Neither made the combine step cheaper — both traded more addition for fewer recursive calls, because `a` sits in the exponent and `f(n)` doesn't.

## The Invariant

> Subproblems **partition** the input (or cover it with bounded overlap), are **independent** — no subproblem's solution depends on another's — and the combine step is correct given correct subsolutions.

Three consequences:

- **Independence is what permits parallelism.** D&C is the natural shape for [work stealing](../../performance-optimization/parallelism-and-work-stealing/learning.md): fork the subproblems, join the results. `rayon::join` is literally this.
- **Independence is also what makes memoization pointless.** Overlap → DP.
- **The base case must be big enough to be efficient.** Recursing to n = 1 pays call overhead per element. Every production D&C cuts over to a simple algorithm at small n — `sort_unstable` switches to insertion sort at ~20 elements ([sorting](../sorting/learning.md)), which is the [complexity analysis](../complexity-analysis/learning.md) n₀ made concrete.

## Mechanics

### The recurrences worth knowing cold

| Recurrence | Result | Where |
| --- | --- | --- |
| T(n) = T(n/2) + Θ(1) | Θ(log n) | Binary search |
| T(n) = 2T(n/2) + Θ(1) | Θ(n) | Tree traversal, heapify |
| T(n) = 2T(n/2) + Θ(n) | **Θ(n log n)** | Merge sort, closest pair |
| T(n) = T(n/2) + Θ(n) | **Θ(n)** | Quickselect (one side only) |
| T(n) = 3T(n/2) + Θ(n) | Θ(n^1.585) | Karatsuba |
| T(n) = 7T(n/2) + Θ(n²) | Θ(n^2.807) | Strassen |
| T(n) = T(n−1) + Θ(n) | **Θ(n²)** | "removes one element" — the anti-pattern |

The fourth row is why [selection](../selection-and-order-statistics/learning.md) is Θ(n) and sorting is Θ(n log n): recursing into **one** side gives n + n/2 + n/4 + … = 2n; recursing into both gives n log n. Measured in Stage 2: top-10 of 1M was 1.21 ms by selection versus 13.01 ms by sorting.

### The canonical algorithms

- **Merge sort** — split in half, sort both, merge. Stable, Θ(n log n) always, Θ(n) extra space. The merge step is the reusable primitive: k-way merge with a heap is external sorting's second phase.
- **Quicksort / quickselect** — partition around a pivot, recurse. The split is data-dependent, which is why pivot choice is a correctness-adjacent concern (Θ(n²) on sorted input with a naive pivot).
- **Closest pair of points** — sort by x, split, recurse, then check only points within δ of the dividing line. The combine step is the entire insight: a strip check that's Θ(n) because at most 7 points can be within δ in the strip.
- **Karatsuba / Strassen** — reduce `a` at the cost of more additions.
- **FFT** — Θ(n log n) polynomial multiplication by splitting into even and odd coefficients. The most consequential D&C algorithm, and the one whose combine step (butterfly) is least obvious.
- **Binary search** — degenerate D&C: one subproblem, trivial combine. See [binary search](../binary-search/learning.md).

### Parallelism

```rust
// rayon::join IS divide and conquer. Independence makes this safe by construction.
fn par_merge_sort(v: &mut [T]) {
    if v.len() <= 1024 { v.sort_unstable(); return; }   // base case: cut over
    let mid = v.len() / 2;
    let (a, b) = v.split_at_mut(mid);                    // disjoint &mut — the borrow checker approves
    rayon::join(|| par_merge_sort(a), || par_merge_sort(b));
    merge(a, b);
}
```

`split_at_mut` is the mechanism: it proves disjointness to the borrow checker, so two threads can hold `&mut` into one buffer. That's the [Rust for data structures](../rust-for-data-structures/learning.md) split-borrow strategy, and D&C is its natural consumer.

The **span** (critical path) is Θ(log n) for the recursion plus whatever the combine costs, so parallelism is roughly n/log n — good. The base-case cutover matters more here than sequentially: forking below ~1,000 elements costs more in task overhead than it saves.

## Complexity

| Algorithm | Time | Space | Parallel span |
| --- | --- | --- | --- |
| Binary search | Θ(log n) | Θ(1) | Θ(log n) — no parallelism |
| Merge sort | Θ(n log n) | **Θ(n)** | Θ(log² n) |
| Quicksort | Θ(n log n) avg, Θ(n²) worst | Θ(log n) | Θ(log² n) |
| Quickselect | **Θ(n)** expected | Θ(1) | — |
| Closest pair | Θ(n log n) | Θ(n) | Θ(log² n) |
| Karatsuba | Θ(n^1.585) | Θ(n) | Θ(log² n) |
| Strassen | Θ(n^2.807) | Θ(n²) | Θ(log² n) |
| FFT | Θ(n log n) | Θ(n) | Θ(log² n) |

**Where the table misleads.** Strassen's Θ(n^2.807) beats Θ(n³) asymptotically and is rarely used: it needs Θ(n²) extra memory, is numerically less stable, and its constant means the crossover is around n ≈ 1000 even in tuned implementations. Karatsuba's crossover against schoolbook multiplication is around 300–600 bits. **For D&C, the crossover point is the number that matters**, not the exponent — and every one of these algorithms ships with a measured cutover to a simpler method.

## Use Cases

- **Sorting and selection** — the whole of [Stage 2](../sorting/learning.md).
- **Parallel computation** — D&C is the shape `rayon` is built around; any associative reduction is a D&C.
- **Numeric algorithms** — FFT (signal processing, big-integer multiplication, polynomial arithmetic), Karatsuba/Toom-Cook in every bignum library.
- **Computational geometry** — closest pair, convex hull (merge hulls), Delaunay triangulation.
- **String algorithms** — Hirschberg's linear-space LCS reconstruction is D&C over the DP table.
- **Balanced tree construction** — building a balanced BST or a segment tree from a sorted array.
- **MapReduce and query engines** — partition, compute, combine; the same shape at cluster scale.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Divide & conquer** | Subproblems are **independent** and the combine is cheap |
| [Dynamic programming](../dynamic-programming/learning.md) | Subproblems **overlap** |
| Recurse into **one** side | You only need part of the answer — selection, binary search |
| `rayon::join` | The subproblems are large enough to amortize a task spawn |
| Iterative | Depth scales with input and could be adversarial |
| Simple Θ(n²) | Below the measured crossover — always have one |

## Pitfalls in Depth

### Pitfall: No base-case cutover

- **What goes wrong:** The recursion goes all the way to n = 1, paying a function call, a bounds check, and a merge setup per element. A "Θ(n log n)" merge sort ends up several times slower than `sort_unstable`, and a parallel version is worse still because it spawns tasks for two-element slices.
- **Why it happens (the mechanism):** The recurrence says nothing about constants, and n = 1 is the mathematically natural base case. But at small n the Θ(n²) algorithm has fewer instructions, perfect locality, and a fully-trained branch predictor — measured in [sorting](../sorting/learning.md), std's `sort_unstable` cuts over to insertion sort at about 20 elements.
- **How to handle it in production, and why that works:** Cut over to a simple algorithm at a measured threshold — insertion sort for sorting, a direct loop for reductions, `sort_unstable` for a parallel sort's leaves. Find the threshold by sweeping it, not by picking a round number; it depends on element size and comparison cost.
- **Trade-offs of the fix:** Two code paths to test, and the threshold is machine- and type-dependent, so a value tuned on one workload may be wrong for another. It's still worth it — the cutover is usually a 2–5× constant-factor win.

### Pitfall: Subproblems that aren't actually independent

- **What goes wrong:** A D&C is written for a problem whose halves interact, and the combine step silently misses solutions that span the boundary. Closest-pair without the strip check returns the minimum of the two halves and misses any pair straddling the divide; maximum-subarray without the crossing case misses the answer whenever it spans the midpoint.
- **Why it happens (the mechanism):** Splitting the *input* is easy; splitting the *solution space* is the actual requirement. Solutions that cross the boundary belong to neither subproblem, so they must be found by the combine step — and it's easy to write a combine that only merges rather than searching the boundary.
- **How to handle it in production, and why that works:** Enumerate the three cases explicitly — entirely left, entirely right, **crossing** — and make sure the combine handles the third. For closest pair that's the δ-strip scan; for maximum subarray it's the best suffix of the left plus the best prefix of the right. Then verify against brute force on small inputs, which catches a missing crossing case immediately.
- **Trade-offs of the fix:** The crossing case is usually where the algorithm's cleverness (and its bound) lives — closest pair's Θ(n) strip check requires the geometric argument that at most 7 points fit in a δ×2δ rectangle. If you can't make the crossing case cheap, D&C won't beat the naive algorithm.

### Pitfall: A combine step that dominates

- **What goes wrong:** The recursion splits nicely but combining costs Θ(n²) or requires re-sorting, so `T(n) = 2T(n/2) + Θ(n²)` collapses to Θ(n²) — no better than the naive algorithm, with more code and worse constants.
- **Why it happens (the mechanism):** Master Theorem case 3: when f(n) dominates `n^(log_b a)`, the root's work is the whole cost and the recursion buys nothing. It's easy to design a split whose combine needs global information you destroyed by splitting.
- **How to handle it in production, and why that works:** Write the recurrence *before* implementing and check which case it lands in. If the combine is too expensive, either pre-process once so the combine can be cheap (closest pair pre-sorts by y so the strip scan is linear), or change the split so less information crosses the boundary.
- **Trade-offs of the fix:** Pre-processing adds a pass and sometimes Θ(n) memory. Occasionally there's no cheap combine and D&C is simply the wrong paradigm for the problem — recognizing that early is the win.
