# Binary Search — Quick Reference

## At a Glance

Not "find a value in a sorted array" — **find the boundary in a monotone predicate**. The array is incidental; monotonicity is the requirement.

**Invariant:** the answer is in `[lo, hi]`; everything before `lo` fails the predicate, everything from `hi` on satisfies it; the interval strictly shrinks each iteration.

## The One Implementation

```rust
/// Smallest i in [0, n] with pred(i) true. Requires pred monotone false→true.
fn lower_bound(n: usize, pred: impl Fn(usize) -> bool) -> usize {
    let (mut lo, mut hi) = (0, n);       // hi = n, NOT n-1
    while lo < hi {                       // <, NOT <=
        let mid = lo + (hi - lo) / 2;     // overflow-safe
        if pred(mid) { hi = mid } else { lo = mid + 1 }   // mid+1, NOT mid
    }
    lo
}
```

No early return, always terminates, handles `n == 0` and "no match" with no special cases.

## Everything from `partition_point`

| Want | Expression |
| --- | --- |
| `lower_bound` (first ≥ key) | `v.partition_point(\|&x\| x < key)` |
| `upper_bound` (first > key) | `v.partition_point(\|&x\| x <= key)` |
| Exists | `lo < v.len() && v[lo] == key` |
| Count of equal | `upper - lower` |
| All equal, as a slice | `&v[lower..upper]` |
| Insertion point | `lower` |

`binary_search` returns `Ok(i)` / `Err(insertion_point)` — **`Err` is the answer, not a failure**. On duplicates it returns *some* match, not the first.

## Complexity

| Aspect | Cost |
| --- | --- |
| Comparisons | ⌈log₂(n+1)⌉ exactly |
| In-cache time | ~20–30 ns at n = 4096 |
| Out-of-cache | Θ(log n) **cache misses** at ~100 ns each |
| Space | Θ(1) iterative |
| Precondition | sorted / monotone — Θ(n log n) to establish |

## Crossover vs Linear Scan (measured, `Vec<u32>`, ns/lookup)

| n | 8 | 16 | **32** | 128 | 1024 | 4096 |
| --- | --- | --- | --- | --- | --- | --- |
| Linear | **11.0** | **21.6** | 52.1 | 108.6 | 416.4 | 1040.9 |
| Binary | 13.2 | 25.7 | **31.2** | **28.2** | **25.2** | **21.8** |

**Binary wins from n ≈ 24.** Note how flat the second row is — that's Θ(log n).

## Choose This When

| Use | For |
| --- | --- |
| `partition_point` | **Default** — boundaries, bounds, counts, ranges |
| `binary_search` | Want found-vs-not *and* insertion point in one call |
| Linear scan | n < ~24, or unsorted and queried once |
| Galloping/exponential | Target near the start, or range has no known end |
| B-tree | Probes cost a disk/page transfer |
| Eytzinger layout | Huge read-only table, lookups dominate |
| `HashMap` | Point lookups only, no ordering needed |

## Binary Search on the Answer

```rust
// "Smallest capacity c such that feasible(c)". feasible must be monotone.
let (mut lo, mut hi) = (1, MAX);
while lo < hi { let mid = lo + (hi - lo) / 2;
                if feasible(mid) { hi = mid } else { lo = mid + 1 } }
```

Instances: min ship capacity in D days, min largest chunk when splitting into k, min max-load scheduling, `git bisect`, capacity planning.

## Rules of Thumb

- Prefer `partition_point`; don't hand-roll.
- Say what's true on the **right** when writing the predicate.
- The comparator must match the sort key — `debug_assert!(v.is_sorted_by_key(..))`.
- `Err(i)` is the insertion point; use it.
- Duplicates → use the boundary form, not `binary_search`.
- Floats → `total_cmp`, never `partial_cmp().unwrap()`.
- Prove monotonicity in a comment for answer-space searches.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| `lo = mid` instead of `mid + 1` | Infinite loop when `hi == lo + 1` |
| `hi = n - 1` with `while lo < hi` | Last element never found |
| `n - 1` on empty slice | `usize` underflow panic |
| Predicate not monotone | Plausible wrong answer, no error, ever |
| Comparator ≠ sort key | Silent garbage |
| Assumed first match on duplicates | Right until duplicates appear in prod |
| `(lo + hi) / 2` | Overflow (the 9-year JDK bug) |

## Key References

- Bloch, ["Nearly All Binary Searches … are Broken"](https://research.google/blog/extra-extra-read-all-about-it-nearly-all-binary-searches-and-mergesorts-are-broken/)
- [`slice::partition_point`](https://doc.rust-lang.org/std/primitive.slice.html#method.partition_point)
- Khuong & Morin, ["Array Layouts for Comparison-Based Searching"](https://arxiv.org/abs/1509.05053)
