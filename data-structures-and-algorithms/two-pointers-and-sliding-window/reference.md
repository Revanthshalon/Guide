# Two Pointers & Sliding Window — Quick Reference

## At a Glance

Turns a nested loop into a single pass by exploiting **monotonicity**. The question that decides applicability: *when I move a pointer, can I prove I'll never move it back?*

**Invariant (converging):** `l < r`, everything outside `[l, r]` already accounted for; each step strictly shrinks the range.
**Invariant (window):** `[l, r)` always satisfies the property; **`r` only increases, `l` only increases** ⇒ each index enters and leaves once ⇒ Θ(n) amortized.

## The Number

Count pairs with sum < target, sorted array (measured):

| n | Nested | Two pointers | Ratio |
| --- | --- | --- | --- |
| 10,000 | 12.05 ms | 24.88 µs | **484×** |
| 100,000 | **1.19 s** | **231.21 µs** | **5,135×** |

## Complexity

| Pattern | Time | Space | Precondition |
| --- | --- | --- | --- |
| Converging | Θ(n) | Θ(1) | **sorted** |
| …with the sort | Θ(n log n) | Θ(1) | — |
| Variable window | **Θ(n) amortized** | Θ(1)–Θ(w) | **monotone constraint** |
| Fixed window | Θ(n) | Θ(1) | — |
| Fast/slow | Θ(n) | **Θ(1)** | linked structure |
| Naive | Θ(n²) | Θ(1) | — |

## The Four Shapes

```rust
// 1. Converging (sorted)
let (mut l, mut r) = (0, n - 1);
while l < r {
    if v[l] + v[r] < target { count += r - l; l += 1; } else { r -= 1; }
}

// 2. Variable window
for r in 0..n {
    sum += v[r];
    while sum > target { sum -= v[l]; l += 1; }   // amortized Θ(1)
    best = best.max(r - l + 1);
}

// 3. Fixed window
for r in k..n { sum += v[r] - v[r - k]; best = best.max(sum); }

// 4. Fast/slow — Floyd's cycle detection, middle of list, k-th from end
```

## Decision Procedure

1. Contiguous subarray/substring? → window candidate
2. Pairs/triples in a **sorted** array? → converging
3. **Is the constraint monotone?** No → **not** two pointers; use prefix sums + `HashMap`
4. Need min/max *inside* the window? → [monotonic deque](../monotonic-stack-and-queue/reference.md)

## Choose This When

| Use | For |
| --- | --- |
| Converging | Sorted, pairs/triples |
| Variable window | Contiguous + **monotone** |
| **Prefix sums + `HashMap`** | Subarray sums with **negatives** |
| Monotonic deque | min/max within the window |
| Fast/slow | Linked structure, Θ(1) space |
| DP | Non-local consequences |

## Rules of Thumb

- **Negatives break sliding-window sum problems** — the sum stops being monotone in `r`.
- `l` is only ever **incremented**, never assigned. If you need to move it back, the technique doesn't apply.
- Use half-open `[l, r)` — length is `r - l`, and `&v[l..r]` checks the convention for you.
- Remove zero-count keys, or keep a separate `distinct` counter and never call `len()`.
- Small alphabet → `[u32; 256]` instead of a `HashMap`.
- Instrument total inner iterations; ≤ 2n confirms the amortization.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Window on data with negatives | Answer too small; correct on all-positive tests |
| `l` reset inside the loop | Silently Θ(n²) — 5,135× at n=100k |
| Zero counts left in the map | `len()` over-reports; window shrinks too far |
| Mixed inclusive/half-open | Consistently off by one |
| Forgot the array was unsorted | Converging pointers give garbage |

## Key References

- CLRS ch. 2 — the merge step is the original two-pointer pattern
- Floyd's cycle detection — the fast/slow classic
