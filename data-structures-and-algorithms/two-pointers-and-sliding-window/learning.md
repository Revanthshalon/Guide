# Two Pointers & Sliding Window — Learning Notes

## Mental Model

**Two pointers turns a nested loop into a single pass by exploiting monotonicity.** The naive algorithm considers every pair — Θ(n²). The two-pointer version maintains two indices that each move in **one direction only**, so the total work is Θ(n) even though the pointers describe Θ(n²) pairs implicitly.

The measured payoff, counting pairs in a sorted array with sum below a target:

| n | Nested loops | Two pointers | Ratio |
| --- | --- | --- | --- |
| 10,000 | 12.05 ms | 24.88 µs | **484×** |
| 100,000 | **1.19 s** | **231.21 µs** | **5,135×** |

Same answer (2,493,951,592 pairs at n=100k). The ratio grows linearly with n, as Θ(n²) vs Θ(n) demands.

The key question — and the one that decides whether the technique applies — is: **when I move a pointer, can I prove I'll never need to move it back?** For the sorted-pairs problem: if `v[l] + v[r] < target`, then every element between `l` and `r` also pairs with `v[l]` below target, so you can count `r - l` pairs at once and advance `l` forever. That "forever" is the monotonicity, and without it the technique is simply wrong.

**Sliding window is the same idea with a different shape:** a contiguous range `[l, r)` that grows on the right and shrinks on the left, where the shrink condition is monotone. The amortization argument is the one to internalize — **each index is added once and removed once across the entire run**, so even though the inner `while` loop looks nested, the total work is Θ(n). That "each element enters and leaves once" argument recurs throughout Stage 6 ([monotonic stack](../monotonic-stack-and-queue/learning.md) uses exactly the same one).

## The Invariant

**Opposite-direction two pointers** (sorted array, converging):

> `l < r`, and every pair `(i, j)` with `i < l` or `j > r` has already been correctly accounted for. Each step moves `l` right or `r` left, strictly shrinking the range, so the loop runs at most n times.

**Same-direction two pointers / sliding window:**

> The window `[l, r)` always satisfies the invariant property (sum ≤ target, at most k distinct characters, no repeats). `r` only increases; `l` only increases. Therefore each index is visited at most twice — once by `r`, once by `l`.

The second clause is the amortization. It's why a `while` inside a `for` is still linear here, and it's what you must check before claiming Θ(n): **if `l` can ever move backwards, the bound is gone.**

The precondition that makes both work:

> Moving a pointer must **monotonically** change the quantity you're testing. Advancing `r` must only increase the window sum (or only relax the constraint); advancing `l` must only decrease it.

**Negative numbers break sliding window for sum problems** precisely because advancing `r` no longer monotonically increases the sum. That's the single most common misapplication, and the fix is a prefix-sum-plus-hash-map approach instead.

## Mechanics

### The three shapes

**1. Opposite ends, converging** — requires a *sorted* array:

```rust
let (mut l, mut r) = (0, n - 1);
while l < r {
    match v[l] + v[r] {
        s if s < target => { count += r - l; l += 1; }   // all pairs (l, l+1..=r) qualify
        _               => { r -= 1; }
    }
}
```

**2. Same direction, variable window** — "longest/shortest subarray satisfying P":

```rust
let (mut l, mut best) = (0, 0);
let mut sum = 0;
for r in 0..n {
    sum += v[r];                                  // grow right
    while sum > target { sum -= v[l]; l += 1; }   // shrink left until valid — amortized Θ(1)
    best = best.max(r - l + 1);
}
```

**3. Same direction, fixed window** — size k, no inner loop needed:

```rust
let mut sum: i64 = v[..k].iter().sum();
let mut best = sum;
for r in k..n {
    sum += v[r] - v[r - k];                       // add one, drop one
    best = best.max(sum);
}
```

**4. Fast/slow pointers** — a different use of "two pointers", on linked structures:

```rust
// Cycle detection (Floyd's): slow moves 1, fast moves 2. They meet iff there's a cycle.
// Also: find the middle of a list in one pass; find the k-th from the end.
```

### The decision procedure

Given a problem, ask in order:

1. **Is it about contiguous subarrays/substrings?** → sliding window candidate.
2. **Is it about pairs/triples in a sorted array?** → converging two pointers.
3. **Is the constraint monotone?** — does growing the window only ever move the metric one way? If no → **not** two pointers; use prefix sums + hash map, or DP.
4. **Do I need the window contents, not just an aggregate?** → you may need a [monotonic deque](../monotonic-stack-and-queue/learning.md) inside the window.

Step 3 is the one that saves you. "Longest subarray with sum ≤ k" is monotone for non-negative values and **not monotone** with negatives — the correct tool there is prefix sums with a `HashMap` of earliest-seen prefix, which is Θ(n) but a different algorithm.

### Sliding window with auxiliary state

For "at most k distinct characters" or "no repeated character", the window carries a counter:

```rust
let mut counts: HashMap<u8, usize> = HashMap::new();
let mut l = 0;
for r in 0..n {
    *counts.entry(s[r]).or_insert(0) += 1;
    while counts.len() > k {                       // shrink until valid
        let c = s[l];
        if let Some(v) = counts.get_mut(&c) {
            *v -= 1;
            if *v == 0 { counts.remove(&c); }      // MUST remove, or len() is wrong
        }
        l += 1;
    }
    best = best.max(r - l + 1);
}
```

The `if *v == 0 { remove }` is the classic bug site — leaving zero-count entries makes `counts.len()` wrong and the window never shrinks correctly. For a small alphabet, use `[usize; 256]` plus a separate distinct-count instead of a `HashMap`; it's faster and removes the bug.

## Complexity

| Pattern | Time | Space | Precondition |
| --- | --- | --- | --- |
| Converging two pointers | **Θ(n)** | Θ(1) | **Sorted** input |
| …including the sort | Θ(n log n) | Θ(1) | — |
| Variable sliding window | **Θ(n)** amortized | Θ(1) or Θ(window) | **Monotone** constraint |
| Fixed sliding window | Θ(n) | Θ(1) | — |
| Fast/slow pointers | Θ(n) | **Θ(1)** | Linked structure |
| Naive nested loops | Θ(n²) | Θ(1) | none |

**Where the table misleads.** The Θ(n) for a variable window is *amortized*, not per-iteration — a single outer step can run the inner `while` many times. That's fine for the total, but it means the per-element latency is not uniform, which occasionally matters for real-time work.

Also, the converging version's Θ(n) usually hides a Θ(n log n) sort. If the data isn't already sorted, two pointers is Θ(n log n) overall — still enormously better than Θ(n²), as the measured 5,135× shows, but the sort is the dominant term.

## Use Cases

- **Pair/triple sums in sorted data** — two-sum, three-sum, count-pairs-below-target, closest pair to a target.
- **Longest/shortest subarray with a property** — longest substring without repeats, minimum window covering a set, longest subarray with sum ≤ k.
- **Merging sorted sequences** — merge step of merge sort, intersection/union of sorted sets, k-way merge with a pointer per list.
- **In-place array partitioning** — Dutch national flag (three pointers), `retain`-style compaction, removing duplicates from a sorted array.
- **Palindrome checks** — converge from both ends.
- **Linked-list problems** — cycle detection (Floyd's), finding the middle, k-th from the end, all in Θ(1) space.
- **Stream processing** — a fixed window over a metric stream, computed incrementally rather than recomputed.
- **String matching preliminaries** — the window discipline underlies Rabin-Karp's rolling hash ([hashing techniques](../hashing-techniques/learning.md)).

## When to Use Which

| Reach for | When |
| --- | --- |
| **Converging two pointers** | Sorted array, looking for pairs/triples |
| **Variable sliding window** | Contiguous range, **monotone** constraint |
| Fixed sliding window | Window size given |
| Fast/slow pointers | Linked structure, Θ(1) space required |
| **Prefix sums + `HashMap`** | Subarray sums with **negative** numbers — window fails |
| [Monotonic deque](../monotonic-stack-and-queue/learning.md) | Need min/max *within* the window |
| [Prefix sums](../prefix-sums-and-difference-arrays/learning.md) | Many range queries, static data |
| DP | The choice at each step has non-local consequences |

## Pitfalls in Depth

### Pitfall: Sliding window with negative numbers

- **What goes wrong:** "Longest subarray with sum ≤ k" is implemented as a sliding window, and the array contains negative values. The window shrinks when the sum exceeds k — but with negatives, extending the window right can *decrease* the sum, so a valid longer window exists beyond a point where the algorithm already shrank. The answer is too small, and it's correct on all-positive test data.
- **Why it happens (the mechanism):** The window's correctness depends on monotonicity: advancing `r` must only increase the sum, so that once the constraint is violated the only remedy is advancing `l`. A negative element breaks that — the sum is no longer monotone in `r`, so "shrink from the left" is not a valid response to a violation.
- **How to handle it in production, and why that works:** Use prefix sums with a `HashMap` from prefix value to earliest index: for "subarray with sum exactly k" look up `prefix[r] - k`; for "sum ≤ k" you need a monotonic structure over prefixes or a BIT. Both are Θ(n) or Θ(n log n) and impose no sign restriction, because they compare *absolute* prefix values rather than assuming directional movement.
- **Trade-offs of the fix:** Prefix-sum-plus-map costs Θ(n) memory against the window's Θ(1), and the "at most k" variants are genuinely harder than the "exactly k" ones. Also, prefix sums over floats accumulate error where a sliding window doesn't — a real concern for numeric data.

### Pitfall: A pointer that moves backwards

- **What goes wrong:** The left pointer is reset (`l = 0` or `l = r`) inside the loop, or recomputed from scratch when the constraint breaks. The algorithm is still correct but is now Θ(n²) — and it *looks* like a sliding window, so the bound is assumed rather than checked. On the measured shape, that's the difference between 231 µs and 1.19 s.
- **Why it happens (the mechanism):** The Θ(n) bound comes entirely from the amortization argument: each index is added once and removed once. Resetting `l` re-processes indices already consumed, so the total work becomes Θ(n) per outer step. Nothing about the code's shape signals this — the nested `while` looks the same either way.
- **How to handle it in production, and why that works:** Assert the monotonicity: `l` is only ever incremented, never assigned. If the algorithm seems to require moving `l` backwards, the constraint isn't monotone and the technique doesn't apply — that's information, not an obstacle to work around. A quick check is to instrument the total inner-loop iterations and confirm it's ≤ 2n.
- **Trade-offs of the fix:** Sometimes you genuinely need a non-monotone window, and then the right answer is a different algorithm (prefix sums, DP, or a segment tree over the range). Forcing a window onto a non-monotone problem produces subtle wrongness rather than slowness, which is worse.

### Pitfall: Forgetting to remove zero counts

- **What goes wrong:** A window tracking distinct elements decrements a counter to zero but leaves the key in the `HashMap`. `counts.len()` then over-reports the distinct count, so the window shrinks too aggressively and the answer is too small — or, in the mirror case, an "at least k distinct" condition never triggers.
- **Why it happens (the mechanism):** `HashMap::len()` counts *keys*, not non-zero values. The decrement and the removal are two separate operations, and the removal is easy to omit because the map still "works" — every lookup returns the right count.
- **How to handle it in production, and why that works:** Either remove the key when its count hits zero, or maintain a separate `distinct` counter incremented on 0→1 and decremented on 1→0 and never call `len()`. The second is faster and makes the invariant explicit. For a bounded alphabet, `[u32; 256]` plus a distinct counter removes both the hash cost and the bug.
- **Trade-offs of the fix:** Removing on zero costs a hash lookup and a potential rehash; a separate counter costs one extra variable and the discipline of updating it in exactly two places. The fixed array only works for small, known alphabets.

### Pitfall: Off-by-one in window boundaries

- **What goes wrong:** Window length computed as `r - l` when the window is `[l, r]` inclusive (should be `r - l + 1`), or `r - l + 1` when it's half-open `[l, r)`. The answer is consistently off by one, which looks like a boundary condition rather than a systematic error.
- **Why it happens (the mechanism):** Both conventions are used in the literature, and the shrink loop's placement (before or after updating the answer) interacts with which one is correct. Mixing conventions between the loop and the length computation is the specific failure.
- **How to handle it in production, and why that works:** Commit to **half-open `[l, r)`** throughout, matching Rust's range convention — then length is `r - l`, an empty window is `l == r`, and `&v[l..r]` is directly usable. Consistency with std's ranges means the slice syntax is a continual check on the convention.
- **Trade-offs of the fix:** Some problems read more naturally with inclusive bounds (palindromes converging from both ends). Use inclusive there, but don't mix within one function.
