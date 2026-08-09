# Sorting — Learning Notes

## Mental Model

**Sorting is the universal preprocessing step.** Very few problems are *about* sorting; a great many become easy once the data is sorted — deduplication, binary search, range queries, grouping, merging, finding duplicates, detecting overlaps, sweep-line algorithms. The Θ(n log n) you spend is almost always bought back by the Θ(log n) or Θ(n) that follows, which is why "sort first" is a reflex worth developing.

The second thing to hold: **you will essentially never implement a comparison sort in production.** `sort_unstable` is a heavily-tuned hybrid that beats anything you'd write. The skills that actually matter are:

1. **Choosing between the two std sorts** (stable vs unstable — a real decision with a measurable cost).
2. **Designing the key**, which is where nearly all sorting bugs and most of the cost live.
3. **Recognizing when you don't need a full sort at all** — partial sorting, selection, or a heap is often 10× cheaper. Measured here: getting the top 10 of 1M `u64` costs 13.01 ms by full sort and 1.21 ms by `select_nth_unstable` — **10.7×** for the same answer.
4. **Knowing when to escape comparison sorting entirely** via radix/counting sort, which the Ω(n log n) bound does not apply to.

The lower bound is worth understanding because it tells you exactly how to beat it. Any sort that only asks "is a < b" must distinguish n! possible orderings, and each comparison yields one bit, so it needs ≥ log₂(n!) ≈ n log₂ n − 1.44n comparisons. **That bound is a statement about comparisons, not about sorting.** Counting and radix sorts don't compare — they use the key's structure directly — so they're Θ(n·k) and entirely legal. Escaping a lower bound always means violating one of its assumptions; here the assumption is "comparison is all you can do."

## The Invariant

A sort establishes:

> The output is a **permutation** of the input (nothing lost, nothing added) in which `a[i] ≤ a[i+1]` for all i, under a comparator that is a **total order**.

Two clauses do the work:

- **Permutation.** This is why sorting can be done in place and why a "sort" that drops or duplicates elements is a corruption, not a wrong order. `retain`-style filtering during a sort is a category error.
- **Total order required.** The comparator must be transitive, antisymmetric, and total. Break it and std's sort may panic (`user-provided comparison function does not correctly implement a total order`), produce garbage, or — historically, in other languages — read out of bounds. The float trap is the standard instance: `partial_cmp().unwrap()` panics on `NaN`, and `NaN` makes the order non-total by definition.

**Stability** is a separate, optional guarantee:

> Elements comparing equal appear in the output in their input order.

Stability is what makes *multi-pass sorting* work: sort by secondary key, then stably sort by primary key, and the secondary order survives within each primary group. That's a genuinely useful technique and the main reason to pay for a stable sort.

## Mechanics

### The algorithms worth knowing, and what each teaches

| Algorithm | Time | Space | Stable | The idea it teaches |
| --- | --- | --- | --- | --- |
| Insertion sort | Θ(n²), **Θ(n) on nearly-sorted** | Θ(1) | yes | Adaptivity; why every hybrid falls back to it at small n |
| Merge sort | Θ(n log n) always | **Θ(n)** | yes | Divide and conquer; the merge step is the reusable primitive |
| Quicksort | Θ(n log n) avg, **Θ(n²) worst** | Θ(log n) | no | Partitioning; why pivot choice is a security property |
| Heapsort | Θ(n log n) always | Θ(1) | no | In-place with a hard guarantee — the safety net for hybrids |
| Counting sort | Θ(n + k) | Θ(k) | yes | Escaping the comparison bound via key structure |
| Radix sort (LSD) | Θ(n · d) | Θ(n + b) | yes | Stability as a load-bearing requirement, not a nicety |
| Timsort / driftsort | Θ(n log n), Θ(n) on runs | Θ(n) | yes | Real data has runs; exploit them |
| pdqsort / ipnsort | Θ(n log n) worst | Θ(log n) | no | Pattern defeating: detect the bad case and switch |

**The three that matter in practice** are the last three — because they're what libraries actually ship. Everything above them is a component inside them.

### What Rust actually runs

- **`sort_unstable`** — a pattern-defeating quicksort (pdqsort lineage; `ipnsort` in current std). Introsort-style: quicksort normally, insertion sort below ~20 elements, and a switch to heapsort if recursion goes too deep, which converts quicksort's Θ(n²) worst case into a Θ(n log n) guarantee. It also detects already-sorted and reverse-sorted runs, and uses a deterministic-but-unpredictable pivot to defeat adversarial input. In place, no allocation.
- **`sort`** — a stable, adaptive merge sort (Timsort lineage; `driftsort` in current std). Detects existing runs and merges them, so real-world partially-ordered data can be much faster than random. **Allocates** up to n/2 elements of scratch space.

The insertion-sort cutoff at ~20 elements is the n₀ from [complexity analysis](../complexity-analysis/learning.md) made concrete: Θ(n²) genuinely wins there, because 20 elements are one or two cache lines with a perfectly-predicted branch pattern.

### Measured: stable vs unstable, and the surprise

1M `u64`, this machine, clone cost subtracted:

| Input shape | `sort` (stable) | `sort_unstable` |
| --- | --- | --- |
| Random | 15.5–17.1 ms | **12.1–12.4 ms** |
| Already sorted | **0.32 ms** | 0.33 ms |
| Reversed | **0.46 ms** | 0.47 ms |
| 100 perturbations in 1M | 17.7 ms | **11.0 ms** |
| **1,000 perturbations in 1M** | **29.2 ms** | **12.8 ms** |
| 10,000 perturbations | 11.8 ms | 12.0 ms |
| 100,000 perturbations | 13.0 ms | 12.3 ms |

Three readings:

1. **Unstable is ~30% faster on random data** and never meaningfully slower. If you don't need stability, don't pay for it.
2. **Both detect fully sorted and fully reversed input** and drop to ~0.3 ms — a 40× win from adaptivity. "Is my data already mostly ordered?" is a question worth asking.
3. **The surprise: nearly-sorted is not automatically fast.** At 1,000 scattered perturbations in 1M elements, the *stable* sort takes 29.2 ms — nearly **twice** its own time on fully random input (15.5 ms), and reproducibly so. Adaptivity has a pessimal middle ground: enough disorder to break the long runs, not enough to skip run detection entirely. Sorted-ness is not a monotone advantage, and "our data is nearly sorted" is not by itself a reason to expect speed.

### Key design — where the real work is

Almost every sorting decision in practice is about the key, not the algorithm:

- **`sort_by_key` vs `sort_unstable_by`.** `sort_by_key` calls the extractor on *every comparison*, so an expensive key (a string format, a computed hash, a database of lookups) is recomputed Θ(n log n) times. `sort_by_cached_key` computes each key once — Θ(n) extractions — and is the right choice whenever extraction isn't trivial.
- **Decorate–sort–undecorate.** Build `Vec<(Key, T)>`, sort by the cheap key, then strip it. This is `sort_by_cached_key` done manually, and it's how you sort by an expensive-to-compute or awkward-to-compare property.
- **Composite keys.** `a.cmp(b).then_with(|| c.cmp(d))` chains comparisons cleanly — `Ordering::then_with` is lazy, so the second key is only evaluated on a tie.
- **Reverse order.** `sort_by_key(|x| Reverse(x.score))`, not a subtraction — subtraction overflows and isn't a valid ordering for floats.
- **Floats.** `total_cmp` gives a genuine total order including `NaN`. `partial_cmp().unwrap()` is a time bomb.

### Escaping the comparison bound

| Sort | When it applies | Cost |
| --- | --- | --- |
| Counting sort | Small integer keys in a known range k | Θ(n + k), beats comparison when k = O(n) |
| Radix sort (LSD) | Fixed-width keys (integers, fixed strings) | Θ(n · d) for d digit-passes; needs a **stable** inner sort |
| Bucket sort | Keys ~uniformly distributed over a range | Θ(n) expected |

Radix sort's dependence on stability is the clearest illustration of why stability is a real property: LSD radix sorts by the least significant digit first, and each subsequent pass must preserve the previous pass's order within equal digits. An unstable inner sort silently destroys the result.

In Rust these aren't in std; `radsort` and `voracious_radix_sort` provide them, and they can be 2–4× faster than `sort_unstable` for `u32`/`u64` keys at large n. They're worth reaching for exactly when the key is a fixed-width integer and n is large.

### External sorting

When the data doesn't fit in memory, the model changes to counting *block transfers* rather than comparisons: split into memory-sized chunks, sort each, write runs to disk, then k-way merge them with a heap. This is what `sort(1)` does, what a database does for a large `ORDER BY`, and it's the I/O model from [complexity analysis](../complexity-analysis/learning.md) applied.

## Complexity

| Algorithm | Best | Average | Worst | Space | Stable |
| --- | --- | --- | --- | --- | --- |
| Insertion | Θ(n) | Θ(n²) | Θ(n²) | Θ(1) | yes |
| Merge | Θ(n log n) | Θ(n log n) | Θ(n log n) | Θ(n) | yes |
| Quicksort | Θ(n log n) | Θ(n log n) | **Θ(n²)** | Θ(log n) | no |
| Heapsort | Θ(n log n) | Θ(n log n) | Θ(n log n) | Θ(1) | no |
| `sort_unstable` | Θ(n) on runs | Θ(n log n) | Θ(n log n) | Θ(log n) | no |
| `sort` | Θ(n) on runs | Θ(n log n) | Θ(n log n) | **Θ(n)** | yes |
| Counting | Θ(n + k) | Θ(n + k) | Θ(n + k) | Θ(k) | yes |
| Radix (LSD) | Θ(n·d) | Θ(n·d) | Θ(n·d) | Θ(n + b) | yes |

**Where the table misleads:**

- **The comparison count is not the cost.** Merge sort and quicksort do similar comparison counts; quicksort wins in practice because partitioning is a sequential scan (prefetcher-friendly, cache-resident) while merging touches two streams plus an output buffer. Constant factors and memory behaviour decide, not the bound.
- **Θ(n log n) with an expensive comparator is Θ(k · n log n).** Sorting `Vec<String>` by content costs the string comparisons too — the wrong-variable trap. `sort_by_cached_key` or interning is the fix.
- **"Nearly sorted is fast" is false in general**, as the measurement above shows. Adaptivity helps at the extremes and can hurt in between.

## Rust Implementation

```rust
// Default. Unstable is faster and allocates nothing.
v.sort_unstable();

// Stable only when you actually need it — equal elements keep input order.
v.sort();

// Expensive key: extract ONCE per element, not once per comparison.
v.sort_by_cached_key(|item| expensive_normalize(&item.name));

// Cheap key: fine to recompute.
v.sort_unstable_by_key(|item| item.id);

// Descending.
v.sort_unstable_by_key(|item| Reverse(item.score));

// Composite: second key evaluated only on a tie.
v.sort_unstable_by(|a, b| {
    a.dept.cmp(&b.dept)
        .then_with(|| b.salary.cmp(&a.salary))   // descending within dept
        .then_with(|| a.name.cmp(&b.name))
});

// Floats — total_cmp is a real total order; partial_cmp().unwrap() panics on NaN.
v.sort_unstable_by(|a, b| a.score.total_cmp(&b.score));

// Multi-pass, using stability deliberately: secondary first, then primary.
v.sort_by_key(|x| x.name.clone());     // secondary
v.sort_by_key(|x| x.dept);             // primary — name order survives ties

// Don't sort to get the top k.
v.select_nth_unstable(k);              // 10.7× faster, measured
let top = &v[..k];

// Already sorted? Check before paying.
if !v.is_sorted() { v.sort_unstable(); }
```

**Crates:** `radsort` / `voracious_radix_sort` (radix sorts for fixed-width keys), `rayon` (`par_sort_unstable` — near-linear speedup on large inputs since sorting parallelizes well), `itertools` (`k_smallest` for streaming top-k).

## Use Cases

- **Preprocessing for everything else.** Binary search, `dedup`, grouping (`chunk_by` on sorted data), set operations by merge, sweep-line algorithms, interval merging — all require sorted input, and it's the sort that makes them Θ(n) or Θ(log n).
- **Deduplication.** `sort_unstable(); dedup();` is Θ(n log n) with no allocation and beats a `HashSet` pass at moderate n while also giving sorted output.
- **Ranking and leaderboards.** Where `select_nth_unstable` usually replaces the sort entirely.
- **Grouping by key.** Sort by the key, then walk runs — often faster than a `HashMap<K, Vec<V>>` because it's one allocation and one linear pass.
- **Deterministic output.** Sorting a `HashMap`'s entries before serializing is what makes output reproducible, since hash iteration order is deliberately non-deterministic in Rust.
- **Making joins and merges linear.** Sort-merge join is the database technique that turns a Θ(n·m) nested loop into Θ(n log n + m log m).

## When to Use Which

| Reach for | When |
| --- | --- |
| **`sort_unstable`** | Default. ~30% faster, no allocation. |
| `sort` | Equal elements must keep input order, or you're doing multi-pass sorting. |
| `sort_by_cached_key` | Key extraction is non-trivial (formatting, allocation, lookup). |
| `select_nth_unstable` | You need the top/bottom k or a median — **10.7× measured**. |
| `BinaryHeap` | Streaming top-k where n doesn't fit or isn't known. |
| Radix sort (`radsort`) | Fixed-width integer keys, large n. |
| Counting sort | Small integer range, k = O(n). |
| `par_sort_unstable` (rayon) | n large enough to amortize thread setup — measure. |
| External merge sort | Doesn't fit in memory. |
| Insertion sort | n < ~20, or nearly-sorted and you're writing the inner loop yourself. |
| **Don't sort** | You need membership (`HashSet`), min/max (`iter().min()`), or top-k. |

## Pitfalls in Depth

### Pitfall: Sorting to get the top k

- **What goes wrong:** "Show the 10 highest scores" is implemented as sort-then-take-10. It's correct and it does Θ(n log n) work to answer a question that needs Θ(n). Measured on 1M `u64`: **13.01 ms for a full sort vs 1.21 ms for `select_nth_unstable` — 10.7×**, for exactly the same 10 values.
- **Why it happens (the mechanism):** Sorting is the obvious tool and it's one line. The full sort computes the complete order of all 1M elements, of which 999,990 are discarded. The information-theoretic argument is direct: identifying the top 10 needs far fewer bits than identifying the total order.
- **How to handle it in production, and why that works:** `select_nth_unstable(k)` partitions in Θ(n) expected time so that everything before index k is ≤ everything after — then sort just that k-element prefix if you need it ordered (Θ(k log k), negligible). For streaming or unknown n, a `BinaryHeap` of size k gives Θ(n log k). For k small and n huge, both crush the full sort.
- **Trade-offs of the fix:** `select_nth_unstable` reorders the whole slice (it's a partition, not a peek) and doesn't sort the prefix, so you get "the top k, unordered" and must sort them separately if order matters. Its Θ(n) is *expected*, not worst case. And below a few thousand elements the difference is noise — the full sort's simplicity wins.

### Pitfall: The float comparator that panics

- **What goes wrong:** `sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap())` runs fine for months and then panics in production the first time a `NaN` appears — from a division by zero, a parse of `"NaN"`, or a 0/0 average of an empty group. Worse, before it panics, a comparator that returns inconsistent results can make std's sort detect the violation and panic with "user-provided comparison function does not correctly implement a total order."
- **Why it happens (the mechanism):** `f64` implements `PartialOrd` but not `Ord`, precisely because `NaN` compares false against everything including itself — so the order isn't total, and `partial_cmp` correctly returns `None`. The `.unwrap()` converts a well-designed type-level warning into a runtime crash.
- **How to handle it in production, and why that works:** `a.score.total_cmp(&b.score)` — a genuine total order over all `f64` bit patterns, with `NaN` placed at a defined position. Or use `ordered_float::NotNan` as the field type, which makes it impossible to construct a `NaN` and restores `Ord` at the type level so the whole problem disappears from the sorting code.
- **Trade-offs of the fix:** `total_cmp` orders `NaN`s *somewhere*, so they still appear in your sorted output at an arbitrary-looking end — if `NaN` means "missing", you probably want to filter or partition them out explicitly rather than sort them. `NotNan` pushes the validation to construction, which is the right place but means handling the error at every parse and computation boundary.

### Pitfall: Expensive keys recomputed per comparison

- **What goes wrong:** `v.sort_by_key(|x| x.name.to_lowercase())` on 100k elements. The closure allocates a `String` on *every comparison* — roughly n log n ≈ 1.7 million allocations — so the sort is dominated by allocator traffic and the profile shows `malloc`, not comparison.
- **Why it happens (the mechanism):** `sort_by_key`'s signature invites the assumption that keys are extracted once, but the implementation calls the extractor inside the comparison to avoid allocating a parallel key array. That's the right default for cheap keys (a field access) and pathological for expensive ones.
- **How to handle it in production, and why that works:** `sort_by_cached_key` extracts each key exactly once into a temporary array — Θ(n) extractions instead of Θ(n log n) — and sorts indices alongside. For very expensive keys, decorate–sort–undecorate manually into a `Vec<(Key, T)>` so you control the representation.
- **Trade-offs of the fix:** `sort_by_cached_key` allocates a key array (Θ(n) memory) and is *slower* than `sort_by_key` when the key is cheap, because materializing the array costs more than recomputing a field access. The rule is by key cost: field access → `sort_by_key`; anything allocating, formatting, hashing, or looking up → `sort_by_cached_key`.

### Pitfall: Relying on stability you didn't ask for, or paying for stability you don't need

- **What goes wrong:** Two mirror mistakes. First: code sorts with `sort_unstable` and downstream logic silently depends on equal elements keeping input order — output is nondeterministic across runs, versions, and input sizes, producing "flaky" tests and diffs that change for no reason. Second: `sort` is used everywhere out of caution, paying ~30% and an Θ(n) allocation for a guarantee nothing uses.
- **Why it happens (the mechanism):** Stability is invisible in the common case — an unstable sort often *happens* to preserve order for small or partially-ordered inputs, so the dependency is established accidentally and only breaks when the algorithm takes a different path at a different size. And in the other direction, "stable is safer" is a plausible-sounding default that costs real time.
- **How to handle it in production, and why that works:** Decide explicitly. If ties must resolve deterministically, either use `sort` *and say why in a comment*, or better, **make the key total** — add a tiebreaker (an ID) so the order is fully determined and the stability question stops mattering. A total key is more robust than stability because it survives a change of sort algorithm, parallelization, and re-sorting elsewhere.
- **Trade-offs of the fix:** A tiebreaker key makes comparisons slightly more expensive and requires a field that's genuinely unique and meaningful. Multi-pass stable sorting (sort by secondary, then primary) is a legitimate technique that *requires* stability and can't be replaced by a total key when the passes happen in different places in the code.

### Pitfall: Assuming "nearly sorted" means fast

- **What goes wrong:** A pipeline appends new records to a sorted file and re-sorts, on the theory that adaptive sorts handle nearly-ordered data cheaply. Measured here, that assumption inverts: 1,000 scattered perturbations in 1M elements made the **stable sort take 29.2 ms — nearly twice its 15.5 ms on fully random data**, and 2.3× the unstable sort's 12.8 ms on the same input. Sorted-ness bought nothing and cost a lot.
- **Why it happens (the mechanism):** Adaptive merge sorts work by detecting ascending runs and merging them. Fully sorted input is one run (0.32 ms — a 40× win). Scattered perturbations shatter the input into many medium-length runs, so the algorithm pays full run-detection and then does a many-way merge with allocation — more work than treating the data as random. The advantage is not monotone in "how sorted" the data is; there's a pessimal middle.
- **How to handle it in production, and why that works:** Measure your actual input shape rather than reasoning about it. If the data is *fully* sorted, check with `is_sorted()` first and skip entirely. If it's "sorted plus a few new records", **merge instead of sorting**: sort only the new tail (small) and do one Θ(n) merge pass — that's linear and beats any re-sort. If the shape is unknown, `sort_unstable` was within 12–13 ms across every shape measured, making it the robust choice.
- **Trade-offs of the fix:** `is_sorted()` is a Θ(n) scan that's wasted when the data isn't sorted — cheap relative to a sort, but not free. The merge approach requires keeping the sorted and unsorted portions separate, which is a structural change to the pipeline rather than a one-line fix.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if sorting returned a new sequence sharing structure? | An index permutation — sort *indices*, leave the data alone (also the fix for expensive moves) |
| Batch it | What if data arrived in sorted chunks? | k-way merge with a heap; external merge sort; LSM tree compaction |
| Approximate it | What if "roughly sorted" sufficed? | Sampling-based partitioning; approximate quantiles; bucket-by-histogram |
| Randomize it | What if the pivot were random? | Randomized quicksort — converts average-case into *expected*, defeating adversarial input |
| Externalize it | What if it didn't fit in RAM? | External merge sort; count block transfers, not comparisons |
| Parallelize it | Where's the independence? | Parallel merge sort / sample sort — sorting parallelizes near-linearly (`par_sort_unstable`) |
| Invert it | What if you sorted by *position* rather than value? | Counting sort — compute each element's destination directly, no comparisons |
| Augment it | What does an extra pass buy? | LSD radix: d stable passes; each pass is Θ(n) and the result is total order |
| Specialize it | What if keys were fixed-width integers? | Radix/counting sort — escapes Ω(n log n) entirely |
| Amortize it | What if one operation could be terrible? | Insertion sort's Θ(n²) worst case is fine below n ≈ 20, which is why every hybrid uses it |

**Questions:**

1. The Ω(n log n) bound is proved from a decision tree over n! outcomes. State the exact assumption radix sort violates, then construct a third way to violate it that isn't radix or counting sort.
2. Radix sort *requires* a stable inner sort. Walk through LSD sorting `[21, 12, 11, 22]` with an unstable inner sort and show precisely where it breaks.
3. Measured, the stable sort is slower on 1,000-perturbation data (29.2 ms) than on fully random data (15.5 ms). Propose two mechanisms, then design an experiment using only timing that would distinguish them.
4. Under "persist it", sorting an index permutation leaves the data untouched. Give two situations where that's strictly better than sorting in place, and one where it's much worse.
5. `select_nth_unstable` is 10.7× faster than a full sort for top-10 of 1M. Derive roughly where that ratio goes as k approaches n, and find the k at which you should just sort.
6. Quicksort's Θ(n²) worst case is triggerable by an attacker who controls the input. Name the two defenses (one used by `sort_unstable`, one by randomized quicksort) and say what each guarantees.
7. Under "invert it", counting sort computes each element's destination directly rather than comparing. What property of the key makes that possible, and what's the memory cost when the key range is large but sparse?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the sorting invariant, both clauses, and say which one makes a "sort" that drops duplicates a category error.
2. Prove the Ω(n log n) comparison bound in three sentences, then name the assumption radix sort breaks.
3. Give the measured `sort` vs `sort_unstable` numbers for random, fully-sorted, and 1,000-perturbation inputs, and explain the middle-ground anomaly.
4. When is `sort_by_cached_key` right, when is it wrong, and what's the deciding property?
5. Why is `sort_by(|a,b| a.x.partial_cmp(&b.x).unwrap())` a bug? Give two fixes at different layers.
6. You need the top 10 of 10M records. Give three approaches with complexities, and say which wins and why.

Build exercises:

- Implement merge sort, quicksort (with median-of-three and a random pivot), and heapsort. Benchmark all three plus both std sorts on: random, sorted, reversed, all-equal, and the 1,000-perturbation shape. The all-equal case is the one that exposes naive partitioning — a two-way partition degrades to Θ(n²) and three-way (Dutch national flag) doesn't.
- Reproduce the perturbation anomaly: sweep perturbation counts from 10 to 100,000 in 1M elements and plot `sort` and `sort_unstable`. Find the pessimal point and write the one-paragraph explanation you'd give a colleague who says "our data is nearly sorted so it'll be fast."
- Implement LSD radix sort for `u32` with a stable counting sort inner pass, and benchmark against `sort_unstable` at n = 10⁵, 10⁶, 10⁷. Then deliberately swap the inner sort for an unstable one and observe the corruption.
- Measure the top-k crossover: `select_nth_unstable` vs full sort vs a size-k `BinaryHeap`, for k = 1, 10, 100, 10⁴, 10⁵ over 1M elements. The three crossover points are the practical decision table.

## Open Questions

- What exactly causes the stable sort's pessimal middle ground at ~1,000 perturbations? Read driftsort's run-detection thresholds and confirm against the measured curve.
- How much does `radsort` beat `sort_unstable` for `u32`/`u64` keys at 10⁶ and 10⁷ on this machine, and where's the crossover below which it loses?
- `par_sort_unstable` speedup on this core count, and the n below which thread setup dominates.
- Does `sort_by_cached_key` actually lose to `sort_by_key` for a trivial key, and by how much? Find the key-cost threshold.
- Sort-then-`dedup` vs `HashSet` for deduplication: measure the crossover for `u64` and for `String`.

## References

- Orson Peters, ["pdqsort: Pattern-defeating quicksort"](https://github.com/orlp/pdqsort) — the design behind `sort_unstable`; the write-up on defeating adversarial patterns is the clearest explanation of why modern quicksorts don't hit Θ(n²).
- Tim Peters, the original Timsort description (CPython `listsort.txt`) — run detection and the merge-invariant stack; the ancestor of `sort`'s adaptivity, and directly relevant to the perturbation anomaly above.
- CLRS ch. 6–8 — heapsort, quicksort, and the decision-tree lower bound plus counting/radix/bucket sort as the escapes from it.
- [`slice::sort` and `sort_unstable` docs](https://doc.rust-lang.org/std/primitive.slice.html#method.sort) — the current algorithm names, guarantees, and allocation behaviour; std states these precisely and they change between releases.
- Aggarwal & Vitter (1988) — external sorting in the I/O model; why the k-way merge is the right shape when data exceeds memory.
- Related topics in this repo: [Binary Search](../binary-search/learning.md) (the precondition this establishes), [Selection & Order Statistics](../selection-and-order-statistics/learning.md) (when you don't need a full sort — the 10.7× measurement), [Complexity Analysis](../complexity-analysis/learning.md) (the lower bound, amortization, and the I/O model), [Branch Prediction](../../performance-optimization/branch-prediction/learning.md) (why partitioning is branch-hostile and how sorting fixes downstream branches), [Cache Locality](../../performance-optimization/cache-locality/learning.md) (why quicksort beats merge sort despite similar comparison counts).
