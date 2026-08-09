# Sorting — Quick Reference

## At a Glance

The universal preprocessing step. You won't implement one — the skills are choosing the std sort, designing the key, and recognizing when a full sort isn't needed.

**Invariant:** output is a **permutation** of the input, ordered under a **total order** comparator.
**Stability (optional):** equal elements keep input order — required for multi-pass sorting and for LSD radix.

## Complexity

| Algorithm | Best | Average | Worst | Space | Stable |
| --- | --- | --- | --- | --- | --- |
| Insertion | Θ(n) | Θ(n²) | Θ(n²) | Θ(1) | yes |
| Merge | Θ(n log n) | Θ(n log n) | Θ(n log n) | Θ(n) | yes |
| Quicksort | Θ(n log n) | Θ(n log n) | **Θ(n²)** | Θ(log n) | no |
| Heapsort | Θ(n log n) | Θ(n log n) | Θ(n log n) | Θ(1) | no |
| `sort_unstable` | Θ(n) runs | Θ(n log n) | Θ(n log n) | Θ(log n) | no |
| `sort` | Θ(n) runs | Θ(n log n) | Θ(n log n) | **Θ(n)** | yes |
| Counting | Θ(n+k) | Θ(n+k) | Θ(n+k) | Θ(k) | yes |
| Radix LSD | Θ(n·d) | Θ(n·d) | Θ(n·d) | Θ(n+b) | yes |

**Ω(n log n) applies to *comparison* sorts only.** Radix/counting use key structure, so they're legal.

## Measured — 1M `u64`, this machine

| Input shape | `sort` (stable) | `sort_unstable` |
| --- | --- | --- |
| Random | 15.5–17.1 ms | **12.1–12.4 ms** |
| Already sorted | **0.32 ms** | 0.33 ms |
| Reversed | **0.46 ms** | 0.47 ms |
| 100 perturbations | 17.7 ms | **11.0 ms** |
| **1,000 perturbations** | **29.2 ms** ⚠ | **12.8 ms** |
| 10,000 perturbations | 11.8 ms | 12.0 ms |

⚠ **Nearly-sorted is NOT automatically fast** — the stable sort's pessimal middle ground is ~2× its own random-input time.

**Top-10 of 1M:** full sort 13.01 ms vs `select_nth_unstable` 1.21 ms — **10.7×**.

## Choose This When

| Use | For |
| --- | --- |
| **`sort_unstable`** | Default — ~30% faster, no allocation |
| `sort` | Equal elements must keep input order; multi-pass sorting |
| `sort_by_cached_key` | Key extraction allocates/formats/hashes/looks up |
| `select_nth_unstable` | Top/bottom k, or a median |
| `BinaryHeap` (size k) | Streaming top-k, n unknown |
| `radsort` | Fixed-width integer keys, large n |
| `par_sort_unstable` | n large enough to amortize threads |
| External merge sort | Doesn't fit in RAM |
| **Don't sort** | Membership → `HashSet`; min/max → `iter().min()`; top-k → select |

## Snippets

```rust
v.sort_unstable();                                   // default
v.sort_by_cached_key(|x| expensive(&x.name));        // extract once, not n log n times
v.sort_unstable_by_key(|x| Reverse(x.score));        // descending
v.sort_unstable_by(|a, b| a.score.total_cmp(&b.score));   // floats — NOT partial_cmp().unwrap()
v.sort_unstable_by(|a, b| a.dept.cmp(&b.dept)
    .then_with(|| b.salary.cmp(&a.salary)));         // composite, lazy on tie

v.select_nth_unstable(k); let top = &v[..k];         // 10.7× vs sorting
if !v.is_sorted() { v.sort_unstable(); }             // skip if already ordered

v.sort_by_key(|x| x.name.clone());                   // multi-pass: secondary...
v.sort_by_key(|x| x.dept);                           // ...then primary (needs stability)
```

## Rules of Thumb

- `sort_unstable` unless you can name why you need stability.
- Prefer a **total key** (add a tiebreaker) over relying on stability — it survives algorithm changes.
- Key cost decides `sort_by_key` vs `sort_by_cached_key`: field access → former; anything allocating → latter.
- Floats → `total_cmp` or `ordered_float::NotNan`.
- Top-k → `select_nth_unstable`, never a full sort.
- Fully sorted data → `is_sorted()` first, or merge the new tail instead of re-sorting.
- Sorting `Vec<String>` is Θ(k·n log n) — intern or cache keys.
- Sort a permutation of indices when elements are expensive to move.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| `partial_cmp().unwrap()` on floats | Panic the first time a `NaN` appears |
| Non-total comparator | "comparison function does not correctly implement a total order" panic |
| `sort_by_key` with allocating key | Profile dominated by `malloc`; ~n log n allocations |
| Sorting for top-k | 10× slower than necessary |
| Accidental stability dependence | Nondeterministic output; flaky tests across sizes |
| Unstable inner sort in LSD radix | Silently corrupted result |
| Assumed nearly-sorted is fast | 2× slower than random input |

## Key References

- [pdqsort](https://github.com/orlp/pdqsort) — the design behind `sort_unstable`
- CPython `listsort.txt` — Timsort run detection, ancestor of `sort`
- [`slice::sort` docs](https://doc.rust-lang.org/std/primitive.slice.html#method.sort) — current guarantees
