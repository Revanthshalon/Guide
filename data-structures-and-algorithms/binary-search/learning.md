# Binary Search — Learning Notes

## Mental Model

**Binary search is not "find a value in a sorted array." It is: find the boundary in a monotone predicate.**

That reframing is the entire value of this topic. The array version is one instance of a far more general tool:

> Given a range of candidates and a predicate `p` that is `false, false, …, false, true, true, …, true` over that range — never flipping back — binary search finds the flip point in Θ(log n) evaluations of `p`.

Nothing in that statement mentions arrays, sortedness, or even memory. `p` can be "is `a[i] >= key`" (the textbook case), but it can equally be "does a machine with this much RAM handle the load", "is this timestamp after the incident", "can we ship in this many days", or "does this build still contain the bug" (that last one is `git bisect`). The array is incidental; **monotonicity is the requirement.**

Once you see it this way, two things follow that the textbook framing hides:

- **"Binary search on the answer" becomes a standard technique.** When a problem asks for a minimum or maximum value satisfying some condition, and the condition is monotone in that value, you don't need a clever construction — you binary search the answer space and use a simple feasibility check. This converts many hard-looking optimization problems into easy ones.
- **The off-by-one taxonomy disappears.** Almost all binary search bugs come from thinking in terms of "did I find it" rather than "where is the boundary." Written as a boundary search, the loop has one shape, always terminates, and the answer is the boundary index — which is simultaneously "where it is" and "where it would go."

Rust encodes exactly this: `slice::partition_point(pred)` *is* the boundary form, and it's the one to reach for by default.

The famous caveat, worth stating early: binary search is Θ(log n) *comparisons* but its memory access pattern is the worst possible — unpredictable jumps, each a potential cache miss, each with an unpredictable branch. Measured on this machine, it only beats a linear scan of a `Vec<u32>` from about **n = 24**. That's the crossover, and it's earlier than folklore claims but not zero.

## The Invariant

The loop invariant that makes it correct, stated for the boundary form:

> The answer lies in `[lo, hi]`. Everything before `lo` fails the predicate; everything from `hi` onward satisfies it. Each iteration halves `hi - lo` while preserving that.

Two obligations that make it terminate and be correct:

- **Monotonicity of the predicate.** If `p` flips back and forth, binary search returns *a* flip point, not *the* flip point, and the result is meaningless — silently. Nothing checks this for you; it's the precondition you must own. For the array case, "sorted by the same ordering the comparator uses" is what supplies it — and if your comparator disagrees with the sort order, you get wrong answers with no error, which is the `Ord`-contract trap from [Rust for data structures](../rust-for-data-structures/learning.md).
- **Strict progress.** The interval must shrink every iteration. `mid = (lo + hi) / 2` with `lo = mid` (rather than `mid + 1`) is the classic infinite loop: when `hi = lo + 1`, `mid == lo`, and nothing changes.

## Mechanics

### The one implementation worth memorizing

Write the boundary form. Everything else is a wrapper around it:

```rust
/// Smallest i in [0, n] such that pred(i) is true.
/// Requires: pred is monotone false→true over 0..=n.
fn lower_bound(n: usize, pred: impl Fn(usize) -> bool) -> usize {
    let (mut lo, mut hi) = (0usize, n);      // answer in [lo, hi]
    while lo < hi {
        let mid = lo + (hi - lo) / 2;        // overflow-safe
        if pred(mid) { hi = mid } else { lo = mid + 1 }
    }
    lo                                        // == hi
}
```

Properties worth noting: `hi` starts at `n`, not `n - 1`, so "no element satisfies the predicate" returns `n` naturally — no special case. The loop is `lo < hi`, never `<=`. There is no `return` inside the loop, so there's no early-exit path to get wrong. It runs exactly ⌈log₂(n+1)⌉ iterations regardless of input, which means it's also branch-count-constant, and it works when `n == 0`.

`mid = lo + (hi - lo) / 2` rather than `(lo + hi) / 2` is the famous overflow fix — the bug that sat in the JDK's `Arrays.binarySearch` for nine years and in Bentley's own published version for twenty. In Rust with `usize` on 64-bit it's not reachable in practice, but the habit costs nothing and matters in C, in 32-bit contexts, and when `lo`/`hi` are timestamps or file offsets.

### The three questions it answers

| Question | Predicate | std |
| --- | --- | --- |
| First index where `a[i] >= key` | `a[i] >= key` | `partition_point(\|x\| x < key)` |
| First index where `a[i] > key` | `a[i] > key` | `partition_point(\|x\| x <= key)` |
| Is `key` present, and where | — | `binary_search(&key)` |

`lower_bound` and `upper_bound` between them give you: existence (`lower != upper`), count of equal elements (`upper - lower`), insertion point (`lower`), and range queries (`lower(a)..lower(b)`). That's the whole API surface, from one function.

### What std gives you

```rust
// Boundary form — the default. Returns the partition index.
let i = v.partition_point(|&x| x < key);

// Value form — Ok(index) if found, Err(insertion_point) if not.
// The Err case is not an error: it's the answer to "where would it go".
match v.binary_search(&key) {
    Ok(i)  => v[i] = new,
    Err(i) => v.insert(i, new),
}

// By key / by comparator, for structs.
v.binary_search_by_key(&id, |item| item.id);
v.binary_search_by(|item| item.score.total_cmp(&target));
```

Two behaviours to know: **on duplicates, `binary_search` returns an unspecified matching index**, not the first — use `partition_point` if you need the first or last. And **`Err(i)` is a feature**, not a failure: it's the insertion point that keeps the slice sorted, which is what makes the sorted-`Vec`-as-a-map pattern work.

### Binary search on the answer

The technique that makes this topic disproportionately valuable. Shape:

```rust
// "What is the smallest capacity that lets us finish in `deadline`?"
// feasible() must be monotone: if capacity c works, every c' > c works.
let answer = lower_bound(MAX_CAP, |c| feasible(c));
```

Canonical instances: minimum ship capacity to move packages in D days; smallest maximum-page-count when splitting a book into k chapters; the "aggressive cows" placement problem; minimizing the maximum load in a scheduling problem. The recipe is always: **identify the answer space, prove the feasibility check is monotone, then binary search it.** The feasibility check is usually a simple greedy scan — the difficulty of the original problem evaporates.

The same shape appears in operations: `git bisect` (monotone: "bug present" is false then true over commits), capacity planning against a load test, and finding the first log entry after a timestamp in a huge file.

### Variants worth knowing

- **Branchless binary search.** Replace the `if` with arithmetic (`lo += (!pred) as usize * (mid - lo + 1)`) or use conditional moves so there's no mispredicted branch. Pays off when the data fits in cache and branch misses dominate.
- **Eytzinger layout.** Store the array in BFS order of the implicit search tree, so each step's children are adjacent in memory and prefetchable. Beats sorted-order binary search substantially on large arrays — a Stage 9 topic, and one of the clearest examples that layout beats asymptotics.
- **Interpolation search.** Guess the position by linear interpolation instead of halving. Θ(log log n) on *uniformly distributed* data, Θ(n) when the distribution is adversarial. Rarely worth the fragility.
- **Exponential (galloping) search.** Double an index until you overshoot, then binary search the last range. Θ(log i) where i is the answer's position — better than Θ(log n) when the target is near the start, and the standard way to search an *unbounded* range (a stream, an infinite sequence, an unknown-length file).
- **Ternary search.** For finding the extremum of a *unimodal* function, not a monotone predicate. Different tool, adjacent idea.

## Complexity

| Aspect | Cost |
| --- | --- |
| Comparisons | Θ(log n) — exactly ⌈log₂(n+1)⌉ for the boundary form |
| Time (in-cache) | Θ(log n), ~20–30 ns at n = 4096 (measured) |
| Time (out-of-cache) | Θ(log n) **cache misses** — ~100 ns each; this dominates |
| Space | Θ(1) iterative, Θ(log n) if written recursively |
| Precondition | Sorted / monotone predicate — Θ(n log n) to establish |

**Where the table misleads, in two directions.**

*Against binary search:* every probe is an unpredictable jump and an unpredictable branch. On an array too big for cache, the Θ(log n) becomes 20-odd cache misses — ~2 µs at n = 10⁹ — which is why B-trees ([complexity analysis](../complexity-analysis/learning.md)'s I/O model) exist. Measured crossover against a linear scan of `Vec<u32>` on this machine:

| n | 8 | 16 | **32** | 128 | 1024 | 4096 |
| --- | --- | --- | --- | --- | --- | --- |
| Linear scan (ns) | **11.0** | **21.6** | 52.1 | 108.6 | 416.4 | 1040.9 |
| `binary_search` (ns) | 13.2 | 25.7 | **31.2** | **28.2** | **25.2** | **21.8** |

Binary search wins from about **n = 24**. Note the flatness of the second row — it barely changes from n = 32 to n = 4096, which is Θ(log n) made visible.

*For binary search:* the Θ(n log n) sorting precondition is often already paid (the data arrives sorted, or is sorted once and queried many times). Amortized over q queries, the cost is (n log n + q log n)/q, so binary search wins over linear scanning as soon as q is comparable to log n. The precondition is a reason to think, not a disqualifier.

## Rust Implementation

```rust
// Default: partition_point. Say what's true on the right.
let first_ge = v.partition_point(|&x| x < key);      // lower_bound
let first_gt = v.partition_point(|&x| x <= key);     // upper_bound
let count_eq = first_gt - first_ge;
let exists   = first_ge < v.len() && v[first_ge] == key;
let range    = &v[first_ge..first_gt];               // all equal elements

// Sorted Vec as a map — insertion keeps the invariant.
match v.binary_search_by_key(&key, |e| e.key) {
    Ok(i)  => v[i].value = value,
    Err(i) => v.insert(i, Entry { key, value }),
}

// Floats: total_cmp, never partial_cmp().unwrap()
v.binary_search_by(|e| e.score.total_cmp(&target));

// Binary search on the answer: Θ(log MAX) feasibility checks, not Θ(MAX).
let min_capacity = {
    let (mut lo, mut hi) = (1usize, MAX);
    while lo < hi { let mid = lo + (hi - lo) / 2;
                    if feasible(mid) { hi = mid } else { lo = mid + 1 } }
    lo
};
```

**`partition_point` over `binary_search` as the default.** It always returns a `usize` (no `Result` to unwrap), it's unambiguous on duplicates, and it expresses the boundary idea directly. Reach for `binary_search` only when you specifically want the found/not-found distinction plus the insertion point in one call.

**The comparator must agree with the sort order.** `binary_search_by` on a slice sorted by a *different* key returns garbage — no panic, no error. This is the same silent-wrongness class as a non-total `Ord`, and it's worth a debug assertion (`debug_assert!(v.is_sorted_by_key(...))`) at the call site in any code where the sort happens far from the search.

## Use Cases

- **Sorted `Vec` as a small map.** Build once, sort once, then `partition_point` for lookups and `Err(i)` for ordered insertion. Worth it for the ordered iteration, range queries, compact footprint and single allocation — **not** for lookup speed, where `HashMap` wins from n ≈ 32 (measured in [arrays](../arrays-and-dynamic-arrays/learning.md)).
- **Range queries.** `lower_bound(a)..lower_bound(b)` gives every element in `[a, b)` as a contiguous slice. This is what makes sorted arrays good for time-series and interval work.
- **Binary search on the answer.** Capacity planning, scheduling, resource allocation, "smallest k such that…" problems.
- **`git bisect` and incident bisection.** Monotone predicate over commits or over time; the same algorithm applied to a version-control history or a log file.
- **Coordinate compression and offset lookup.** Given sorted boundaries, find which bucket a value falls into — the basis for histograms, sparse tables, and interval indexing.
- **Merging and set operations.** Galloping search is how a small sorted set is intersected efficiently with a huge one — used in search-engine posting-list intersection.
- **Anywhere a lookup table is queried far more than it's built.** The sorting cost amortizes away.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`partition_point`** | Default. Boundary, lower/upper bound, counting, ranges. |
| `binary_search` | You want found-vs-not-found *and* the insertion point in one call. |
| Linear scan | n below ~24 (measured, `u32`), or data unsorted and queried once. |
| Galloping / exponential | Target likely near the start; or the range has no known end. |
| `HashMap` | Point lookups only, no ordering or range queries needed, n large. |
| `BTreeMap` | Ordered *and* frequently mutated — sorted `Vec` insertion is Θ(n). |
| Eytzinger layout | Huge read-only table, lookups dominate the profile (Stage 9). |
| Interpolation search | Provably uniform data and you've measured a win. Rare. |

## Pitfalls in Depth

### Pitfall: The predicate isn't actually monotone

- **What goes wrong:** Binary search is applied to data sorted by one key while the predicate tests another, or to a "mostly sorted" collection, or to a feasibility check that isn't monotone (a bigger capacity sometimes fails where a smaller one succeeded). It returns a plausible index. Nothing panics. The wrong record is updated, or the "optimal" answer is quietly suboptimal, and it's found months later by a user rather than by a test.
- **Why it happens (the mechanism):** Binary search's correctness rests entirely on a precondition it cannot check — it only ever looks at Θ(log n) of the n elements, so it *cannot* detect that the rest violate the ordering. Every probe result is consistent with some valid input, so there's no signal to raise.
- **How to handle it in production, and why that works:** Make the precondition explicit and checked in debug builds: `debug_assert!(v.is_sorted_by_key(|e| e.key))` immediately before the search, and for answer-space searches, a comment stating why feasibility is monotone plus a property test that checks it over random inputs. Where the sort and the search are far apart in the code, a newtype (`Sorted<Vec<T>>`) that can only be constructed by sorting moves the guarantee into the type system.
- **Trade-offs of the fix:** `is_sorted` is Θ(n), which defeats the purpose if left in release builds — hence `debug_assert!`, which then only catches what your tests exercise. The newtype approach is airtight but viral: every function touching the collection has to speak the new type, and mutation has to go through methods that re-establish the invariant.

### Pitfall: The off-by-one family

- **What goes wrong:** Four bugs from one root: an infinite loop (`lo = mid` when `hi = lo + 1`); missing the last element (`hi = n - 1` with `while lo < hi`); a panic on the empty slice (`n - 1` underflows on `usize`); and returning "not found" for an element that's present at the boundary.
- **Why it happens (the mechanism):** There are genuinely several correct formulations — `[lo, hi)` vs `[lo, hi]`, `while lo < hi` vs `<=`, `hi = mid` vs `hi = mid - 1` — and they are not interchangeable. Mixing halves of two correct variants produces something that's right on most inputs and wrong at the edges, which is exactly what tests with n = 5 and a middle target won't catch.
- **How to handle it in production, and why that works:** Use `partition_point`/`binary_search` from std and don't hand-roll. When you must (searching a non-slice space, binary search on the answer), use the single boundary form above and never modify it: half-open `[lo, n]`, `while lo < hi`, `hi = mid` / `lo = mid + 1`, return `lo`. It has no early return, always terminates, and handles n = 0 and "no match" without special cases.
- **Trade-offs of the fix:** The boundary form doesn't tell you *whether* the element was found — you need one extra comparison after the loop. That's a genuine (tiny) cost, and it's the price of a loop with no edge cases. Accept it; the alternative is the bug family above.

### Pitfall: Binary searching data that isn't in memory the way you think

- **What goes wrong:** A binary search over a 4 GB sorted file, or a `Vec` of `Box<T>`, or a sorted `Vec<String>`. The Θ(log n) comparison count is achieved and the thing is still slow — because each of the ~32 probes is a page fault, a pointer dereference to a scattered heap allocation, or a Θ(k) string comparison. The asymptotic analysis was correct and irrelevant.
- **Why it happens (the mechanism):** Binary search's access pattern is maximally cache-hostile by design: consecutive probes are far apart, so no prefetcher can help, and the first ~log(cache_size) probes miss every level of cache. The RAM model counts comparisons; the machine charges for *memory transfers*, and this algorithm maximizes them per comparison.
- **How to handle it in production, and why that works:** Match the structure to the memory hierarchy rather than to the comparison count. On disk or at scale: a B-tree, whose fanout makes each transfer carry ~100 keys instead of 1 (~4.5 transfers vs ~30 at n = 10⁹). In memory for a hot read-only table: Eytzinger layout, so a probe's likely successors share a cache line. For `String` keys: intern to `u32` so comparisons are Θ(1), or store a fixed-size prefix inline for a cheap first comparison.
- **Trade-offs of the fix:** B-trees and Eytzinger both give up the plain sorted array, so they cost build complexity and make incremental insertion harder (Eytzinger essentially requires a rebuild). Interning adds a table and a translation boundary. All of these are worth it only when lookups dominate — measure before restructuring.

### Pitfall: Assuming `binary_search` finds the *first* match

- **What goes wrong:** With duplicate keys, `binary_search` returns *some* matching index. Code that then walks backward to find the run's start, or that uses the returned index as "the first occurrence", is right whenever duplicates are rare in testing and wrong the moment they aren't.
- **Why it happens (the mechanism):** The std method is documented to return any matching index, because stopping at the first hit is what makes it fast — pinning down the *first* match requires continuing the search after a hit, which the value-form API deliberately doesn't do.
- **How to handle it in production, and why that works:** Use the boundary form: `partition_point(|x| x < key)` gives the first occurrence and `partition_point(|x| x <= key)` gives one past the last, so the run is `first..last` and the count is the difference. Both are Θ(log n), neither has an "and then scan backward" step whose cost is Θ(run length).
- **Trade-offs of the fix:** Two searches instead of one when you need both ends — still Θ(log n), but twice the probes. If you only need existence, one `partition_point` plus one comparison is cheaper than either.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if the array were versioned? | Binary search over a persistent/immutable snapshot — the basis of MVCC index reads |
| Batch it | What if you searched for 1,000 keys at once? | Sort the queries and merge-scan (Θ(n + q)); or interleave probes so misses overlap in flight |
| Approximate it | What if "close enough" sufficed? | Interpolation search; learned indexes (a model predicts the position, binary search corrects locally) |
| Randomize it | What if the split point were random? | Quickselect's partition — randomization defends against adversarial order |
| Externalize it | What if a probe cost a disk seek? | B-trees — raise the fanout so each transfer carries ~100 keys |
| Parallelize it | Can you probe in parallel? | Not on the dependent chain — but k-ary search probes k points per round, trading comparisons for parallelism |
| Invert it | What if you searched the *answer* instead of the data? | Binary search on the answer — the technique that generalizes the topic |
| Augment it | What does storing extra per position buy? | Fractional cascading: search m sorted lists in Θ(log n + m) instead of Θ(m log n) |
| Specialize it | What if keys were uniformly distributed integers? | Direct addressing / bucketing — Θ(1), no search at all |
| Amortize it | What if you rearranged the array once? | **Eytzinger layout** — one rebuild buys cache-friendly probes forever |

**Questions:**

1. Binary search needs monotonicity, not sortedness. Give three predicates that are monotone over something other than an array, and for each name what would break monotonicity.
2. Measured, `binary_search` on `Vec<u32>` barely changes from n = 32 (31 ns) to n = 4096 (22 ns) — it got *faster*. Give two mechanisms that could explain a non-monotone curve, and design a measurement to distinguish them.
3. Under "batch it": you have 1,000 keys to look up in a 10M-element array. Compare 1,000 independent binary searches against sort-then-merge-scan. Where's the crossover, and what does the answer depend on?
4. Galloping search finds the target in Θ(log i) where i is its position. Derive why that beats Θ(log n) for intersecting a 100-element set with a 10M-element one, and give the total cost.
5. Under "augment it", fractional cascading searches m sorted lists in Θ(log n + m). What is stored extra, and why does that let subsequent searches skip their log factor?
6. A "learned index" replaces the first probes with a model predicting the position. Under which lens does that fall, what's the failure mode when the model is wrong, and why is that failure *bounded*?
7. Eytzinger layout beats sorted order despite doing the same number of comparisons. Explain the mechanism, then say what operation it makes expensive and why that's an acceptable trade for some tables and not others.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State binary search's precondition without using the words "sorted" or "array".
2. Write the boundary form from memory. Justify `hi = n` (not `n-1`), `while lo < hi` (not `<=`), and `lo = mid + 1` (not `mid`).
3. Give `lower_bound`, `upper_bound`, existence, count-of-equal, and insertion point in terms of `partition_point` only.
4. What is `mid = lo + (hi - lo) / 2` defending against, and is that reachable with `usize` on 64-bit? Why keep the habit anyway?
5. Where is the measured crossover against a linear scan for `Vec<u32>`, and name two properties of your data that would move it in each direction.
6. `binary_search` returned index 7 among duplicates. What exactly is guaranteed, and how do you get the first and last occurrence instead?

Build exercises:

- Implement the boundary form, then build `lower_bound`, `upper_bound`, `contains`, `count`, and `equal_range` on top of it — all as one-liners. Property-test against a linear-scan reference on random arrays *with duplicates*, including n = 0 and n = 1. The duplicates and the empty case are where hand-rolled versions die.
- Reproduce the crossover table: linear scan vs `partition_point` on sorted `Vec<u32>` at n = 8…4096. Then repeat with a 32-byte struct and with `String` keys, and explain why the crossover moves the way it does.
- Binary search on the answer: solve "minimum ship capacity to deliver all packages within D days." Write the feasibility check first, prove its monotonicity in one sentence, then wrap it in the boundary form. The proof is the exercise; the code is four lines.
- Implement Eytzinger layout (build the BFS-ordered array, then search it) and benchmark against `partition_point` at n = 10⁶ and 10⁷. Confirm the crossover where layout starts beating the standard version.

## Open Questions

- Why is the measured `binary_search` time non-monotone in n (31 ns at 32, 22 ns at 4096)? Branch-predictor training on the repeated key set is the suspect — verify with `perf stat` branch-miss counts.
- Where exactly does Eytzinger start beating sorted-order binary search on this machine, and does prefetching the grandchildren move it?
- Is std's `binary_search` branchless? Read the generated assembly and compare against a deliberately branchless version.
- Galloping vs plain binary search for posting-list intersection at realistic size ratios — measure rather than assume.
- Does `partition_point` on a `Vec<String>` benefit measurably from storing an inline 8-byte prefix for a cheap first comparison?

## References

- Jon Bentley, *Programming Pearls*, ch. 4 — the classic "write binary search correctly" exercise, plus the famous observation that most programmers can't on the first attempt.
- Joshua Bloch, ["Extra, Extra — Read All About It: Nearly All Binary Searches and Mergesorts are Broken"](https://research.google/blog/extra-extra-read-all-about-it-nearly-all-binary-searches-and-mergesorts-are-broken/) — the `(lo + hi)` overflow bug, live in the JDK for nine years.
- [`slice::partition_point` docs](https://doc.rust-lang.org/std/primitive.slice.html#method.partition_point) — the boundary form as std presents it; the examples are the API surface in miniature.
- Paul-Virak Khuong & Pat Morin, ["Array Layouts for Comparison-Based Searching"](https://arxiv.org/abs/1509.05053) — the definitive measurement of Eytzinger and friends; the paper that makes "layout beats asymptotics" concrete.
- Kraska et al., "The Case for Learned Index Structures" (2018) — binary search's first probes replaced by a model; interesting as an idea even where it isn't practical.
- Related topics in this repo: [Complexity Analysis](../complexity-analysis/learning.md) (the I/O model, and why probe count isn't the cost), [Arrays & Dynamic Arrays](../arrays-and-dynamic-arrays/learning.md) (the sorted-`Vec`-as-map pattern and the same crossover table), [Sorting](../sorting/learning.md) (the precondition, and where it's already paid), [Cache Locality](../../performance-optimization/cache-locality/learning.md) + [Branch Prediction](../../performance-optimization/branch-prediction/learning.md) (why this algorithm is hostile to both).
