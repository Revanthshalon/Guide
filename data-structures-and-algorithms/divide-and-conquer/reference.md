# Divide & Conquer — Quick Reference

## At a Glance

Split into **independent** subproblems of the same shape, recurse, **combine**. The combine step is where the algorithm lives.

**vs DP:** one word — *independent*. Overlapping subproblems ⇒ memoize ⇒ it's DP.
**Invariant:** subproblems partition the input, are independent, and the combine is correct given correct subsolutions.

## Master Theorem

`T(n) = a·T(n/b) + f(n)`, compare f(n) to `n^(log_b a)`:

| f smaller | f equal | f larger |
| --- | --- | --- |
| Θ(n^(log_b a)) | Θ(n^(log_b a)·log n) | Θ(f(n)) |

**The lever:** reducing `a` changes the *exponent*; making f(n) cheaper doesn't. Karatsuba 4→3 mults ⇒ Θ(n^1.585). Strassen 8→7 ⇒ Θ(n^2.807).

## Recurrences to Know Cold

| Recurrence | Result | Where |
| --- | --- | --- |
| T(n/2) + Θ(1) | Θ(log n) | Binary search |
| 2T(n/2) + Θ(1) | Θ(n) | Tree traversal, heapify |
| **2T(n/2) + Θ(n)** | **Θ(n log n)** | Merge sort, closest pair |
| **T(n/2) + Θ(n)** | **Θ(n)** | Quickselect — **one side only** |
| 3T(n/2) + Θ(n) | Θ(n^1.585) | Karatsuba |
| T(n−1) + Θ(n) | **Θ(n²)** | the anti-pattern |

Recursing into **one** side (2n total) vs both (n log n) is why selection beat sorting **10.7×** in Stage 2.

## Complexity

| Algorithm | Time | Space | Span |
| --- | --- | --- | --- |
| Merge sort | Θ(n log n) | **Θ(n)** | Θ(log² n) |
| Quicksort | Θ(n log n) avg, Θ(n²) worst | Θ(log n) | Θ(log² n) |
| Quickselect | **Θ(n)** expected | Θ(1) | — |
| Closest pair | Θ(n log n) | Θ(n) | Θ(log² n) |
| Karatsuba | Θ(n^1.585) | Θ(n) | Θ(log² n) |
| Strassen | Θ(n^2.807) | Θ(n²) | Θ(log² n) |
| FFT | Θ(n log n) | Θ(n) | Θ(log² n) |

**Crossover, not exponent, is the number that matters.** Strassen crosses over around n ≈ 1000; Karatsuba around 300–600 bits.

## Parallelism

```rust
fn par_sort(v: &mut [T]) {
    if v.len() <= 1024 { v.sort_unstable(); return; }   // measured cutover
    let (a, b) = v.split_at_mut(v.len() / 2);           // disjoint &mut, borrow checker approves
    rayon::join(|| par_sort(a), || par_sort(b));
    merge(a, b);
}
```

`rayon::join` **is** divide and conquer. Independence makes it safe by construction.

## Rules of Thumb

- Write the recurrence **before** implementing; check which Master case it lands in.
- Always cut over to a simple algorithm at small n (std sorts cut over at ~20).
- Enumerate three cases: entirely left · entirely right · **crossing**.
- Need only part of the answer? Recurse into **one** side.
- Overlapping subproblems ⇒ stop, it's DP.
- Verify against brute force on small n — that's what catches a missing crossing case.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| No base-case cutover | Several× slower than the "worse" algorithm |
| Missing crossing case | Misses solutions spanning the midpoint |
| Combine dominates | Collapses to Θ(f(n)) — no gain over naive |
| Forking below ~1,000 elements | Task overhead exceeds the work |
| Recursion depth from input | Stack overflow abort |

## Key References

- CLRS ch. 4 — recurrences, Master Theorem, Strassen
- Karatsuba (1962) · Strassen (1969) — reducing `a`
- Cooley & Tukey (1965) — FFT
