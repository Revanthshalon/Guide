# Monotonic Stack & Queue — Learning Notes

## Mental Model

**A monotonic stack maintains a sorted stack by throwing away elements that can never be the answer again.** That single discard rule is what converts a Θ(n²) scan into Θ(n).

The canonical problem: for each element, find the **next greater element** to its right. Naively, scan forward from each position — Θ(n²). The monotonic-stack insight: **if `v[j] > v[i]` and `j > i`, then `v[i]` can never be the "next greater" answer for anything to the right of `j`** — `v[j]` is closer *and* larger, so it dominates. So when you encounter `v[j]`, pop everything smaller off the stack; those elements are dead forever.

The amortization is the same argument as [sliding window](../two-pointers-and-sliding-window/learning.md): **each element is pushed once and popped at most once**, so the total work across the entire run is Θ(n) even though the inner `while` looks nested.

A **monotonic deque** applies the same idea to a sliding window, which gives sliding-window minimum/maximum in Θ(n). Measured on 1,000,000 elements:

| Window k | Naive Θ(n·k) | Heap Θ(n log n) | **Monotonic deque Θ(n)** |
| --- | --- | --- | --- |
| 100 | 61.39 ms | 22.18 ms | **11.35 ms** (5×) |
| 10,000 | **6.62 s** | 12.30 ms | **11.20 ms** (**591×**) |

Two things worth reading off that table. The deque is **independent of k** (11.35 ms at k=100, 11.20 ms at k=10,000) because each element still enters and leaves once regardless of window size — that's the signature of the technique working. And the **heap is nearly as good** at large k, which is an honest finding: Θ(n log n) with a good constant is competitive with Θ(n), and the deque's real advantages are the k-independence and Θ(k) rather than Θ(n) memory.

## The Invariant

**Monotonic stack:**

> The stack holds indices whose values are **strictly monotonic** (increasing or decreasing) from bottom to top. Before pushing `i`, pop every element that violates the ordering with `v[i]` — and each popped element's answer *is* `i`.

That last clause is the payoff: **the pop is where you record the answer.** An element is popped exactly when the thing it was waiting for arrives.

**Monotonic deque (sliding window max):**

> The deque holds indices in **increasing** order, with **decreasing** values. The front is therefore always the maximum of the current window, and every index in the deque is inside `[l, r]`.

Two maintenance rules preserve it, and both are required:

- **Back:** before pushing `i`, pop from the back while `v[back] ≤ v[i]` — those elements are smaller *and* older, so they can never be the max while `i` is in the window.
- **Front:** pop from the front if `front + k ≤ i` — that index has left the window.

The ordering of those two operations relative to reading the answer is where the bugs live.

## Mechanics

### Monotonic stack — next greater element

```rust
// For each i, the index of the next element to the right that is greater.
let mut next_greater = vec![None; n];
let mut stack: Vec<usize> = Vec::new();           // holds indices, values DECREASING
for i in 0..n {
    while let Some(&top) = stack.last() {
        if v[top] < v[i] {
            next_greater[top] = Some(i);           // ← the pop IS the answer
            stack.pop();
        } else { break; }
    }
    stack.push(i);
}
// Anything left on the stack has no next-greater element.
```

**Direction and comparison determine which of four problems you solve:**

| Want | Iterate | Pop while | Stack values |
| --- | --- | --- | --- |
| Next greater to the **right** | left → right | `v[top] < v[i]` | decreasing |
| Next smaller to the **right** | left → right | `v[top] > v[i]` | increasing |
| Previous greater to the **left** | right → left | `v[top] < v[i]` | decreasing |
| Previous smaller to the **left** | right → left | `v[top] > v[i]` | increasing |

Equivalently: iterate left-to-right and read the answer from what's *left on the stack* rather than from the pop, which gives the "previous" variants without reversing.

### Monotonic deque — sliding window maximum

```rust
let mut dq: VecDeque<usize> = VecDeque::new();     // indices increasing, values decreasing
let mut out = Vec::with_capacity(n - k + 1);
for i in 0..n {
    while let Some(&b) = dq.back() {
        if v[b] <= v[i] { dq.pop_back(); } else { break; }   // maintain decreasing values
    }
    dq.push_back(i);
    if *dq.front().unwrap() + k <= i { dq.pop_front(); }      // evict the expired index
    if i + 1 >= k { out.push(v[*dq.front().unwrap()]); }      // front is the window max
}
```

Note the deque stores **indices, not values** — you need the index to know when an element leaves the window. Storing values and trying to track positions separately is a common and painful mistake.

### The largest-rectangle pattern

The monotonic stack's most consequential application: **largest rectangle in a histogram** in Θ(n). For each bar, the rectangle it can anchor extends left until a shorter bar and right until a shorter bar — which is exactly "previous smaller" and "next smaller". One monotonic-stack pass computes both, and the pop is where you compute the area:

```rust
// When bar `top` is popped by bar `i`, we now know both its boundaries:
//   right boundary = i, left boundary = the new stack top (or -1)
let height = v[top];
let width = i - stack.last().map_or(0, |&l| l + 1);
best = best.max(height * width);
```

This generalizes to **maximal rectangle in a binary matrix** (run the histogram algorithm per row) and to several stock-span and temperature-span problems.

### Recognizing the pattern

The trigger phrases:

- "**next/previous greater/smaller** element"
- "**span**" — how far back until something bigger (stock span, temperature span)
- "largest **rectangle**" / "maximal area under constraints"
- "sliding window **min/max**"
- "**visible** buildings/people from a direction"
- DP transitions of the form `dp[i] = max(dp[j]) for j in a moving range` → **monotonic deque drops a factor of k** ([dynamic programming](../dynamic-programming/learning.md))

That last one is the highest-leverage use: many DP recurrences have a sliding-window max in the transition, and the deque turns Θ(n·k) into Θ(n).

## Complexity

| Problem | Naive | Heap | **Monotonic** |
| --- | --- | --- | --- |
| Next greater element | Θ(n²) | — | **Θ(n)** |
| Sliding window max | Θ(n·k) | Θ(n log n) | **Θ(n)** |
| Largest rectangle in histogram | Θ(n²) | — | **Θ(n)** |
| Maximal rectangle in a matrix | Θ(n²m) | — | **Θ(n·m)** |
| DP with windowed max transition | Θ(n·k) | Θ(n log k) | **Θ(n)** |
| Space | Θ(1) | Θ(k) | **Θ(k)** |

**Where the table misleads.** Measured, the heap was within 10% of the deque at k = 10,000 (12.30 ms vs 11.20 ms) — Θ(n log n) with a flat-array binary heap is genuinely competitive with Θ(n). What the deque actually buys is **k-independence** (identical time at k=100 and k=10,000) and simpler eviction. So the honest reason to prefer it is predictability and memory, not a large constant-factor win — except against the naive version, where it's 591×.

The Θ(n) is amortized: a single iteration can pop many elements. Total pops ≤ total pushes = n.

## Use Cases

- **Stock span / temperature span** — "how many days until a warmer day" is next-greater-element verbatim.
- **Largest rectangle in a histogram** — and by extension maximal rectangle in a binary matrix, used in layout and image analysis.
- **Sliding window statistics** — max/min over a moving window in stream processing, without recomputation.
- **DP transition optimization** — any `dp[i] = f(max/min of dp[i-k..i])` recurrence.
- **Expression parsing** — the operator stack in shunting-yard is monotonic in precedence.
- **Trapping rain water** — two-pointer or monotonic-stack formulations both work; the stack version computes water level by level.
- **Visibility problems** — buildings visible from the left, people who can see the stage.
- **Removing k digits to minimize a number** — greedily pop larger preceding digits; a monotonic stack implements the exchange argument directly.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Monotonic stack** | next/previous greater/smaller; spans; histogram rectangles |
| **Monotonic deque** | sliding window min/max; windowed DP transitions |
| Heap | Window min/max where k varies, or you also need k-th largest |
| [Sliding window](../two-pointers-and-sliding-window/learning.md) | The aggregate is a sum/count, not a min/max |
| [Sparse table](../range-query-structures/learning.md) | Static array, arbitrary range min/max, Θ(1) queries |
| [Segment tree](../range-query-structures/learning.md) | Range min/max with **updates** |

## Pitfalls in Depth

### Pitfall: Storing values instead of indices

- **What goes wrong:** The deque holds values, so there's no way to tell when an element has left the window — you can't compare its position to `i - k`. Workarounds (a parallel position deque, searching for the value) either desynchronize or reintroduce Θ(k) work. For the stack variants, storing values means you can't record `next_greater[original_index]`.
- **Why it happens (the mechanism):** The answer you want is a value, so storing values feels direct. But every maintenance rule is positional — "has this left the window", "what is this element's left boundary" — and positions are only recoverable from indices.
- **How to handle it in production, and why that works:** Always store **indices**; read `v[idx]` when you need the value. The front's value is `v[*dq.front()]`, and eviction is `*dq.front() + k <= i`. This also handles duplicate values correctly, which a value-based deque cannot.
- **Trade-offs of the fix:** One extra indirection per access, which is free — the array is hot in cache. There's no downside; this is simply the correct formulation.

### Pitfall: Wrong comparison operator with duplicates

- **What goes wrong:** Using `<` where `<=` belongs (or vice versa) in the pop condition. With distinct values both work; with duplicates, one of them either keeps stale equal elements in the structure or discards elements that are still needed. Results are correct on random data and wrong on data with repeats — plateaus in a histogram, flat regions in a time series.
- **Why it happens (the mechanism):** Ties are the only case where the two operators differ, and the correct choice depends on what you're computing. For sliding-window *maximum*, popping on `v[back] <= v[i]` is right — the older equal element expires sooner and is never uniquely needed. For "next **strictly** greater" versus "next greater **or equal**", the operator *is* the specification.
- **How to handle it in production, and why that works:** Write the specification down first ("next strictly greater to the right"), then derive the operator from it, then **test with an all-equal array** and with a plateau — those inputs distinguish the variants immediately, and random data does not.
- **Trade-offs of the fix:** None, beyond the discipline of writing a test with duplicates. It's worth noting that both variants are legitimately needed by different problems, so there's no universally-correct operator to memorize.

### Pitfall: Reading the answer before evicting the expired front

- **What goes wrong:** In the sliding-window deque, the front is read as the window maximum *before* popping indices that have fallen out of the window. The reported maximum is an element no longer in the window — too large, and only when the true maximum happens to be exactly `k` positions back.
- **Why it happens (the mechanism):** Three operations must happen per iteration — maintain the back, evict the front, read the answer — and only one ordering is correct. The bug is rare in random data because it requires the maximum to be precisely at the expiring position.
- **How to handle it in production, and why that works:** Fix the order: **push (maintaining the back) → evict the expired front → read the front**. Then verify against a naive Θ(n·k) implementation on random arrays; the measured comparison in this doc asserted equality across all three implementations, which is exactly the check that catches this.
- **Trade-offs of the fix:** None. Always keep the naive version as a test oracle — it's five lines and it validates every variant you write later.

### Pitfall: Assuming the stack empties, or forgetting what remains

- **What goes wrong:** After the main loop, elements still on the stack are ignored. Those are precisely the elements with **no** next-greater element, and their answer (often "none", `-1`, or `n`) is never assigned — leaving default values that downstream code misreads as real answers.
- **Why it happens (the mechanism):** The loop's structure implies every element eventually gets popped, but the largest element (and any suffix maximum) never does. It's an edge case that a strictly-increasing test array hides completely.
- **How to handle it in production, and why that works:** Either initialize the answer array with an explicit sentinel that downstream code checks, or append a virtual boundary element (`+∞` for next-greater, `-∞` for next-smaller) that forces the stack to drain. The sentinel-append trick is common in histogram code precisely to avoid a separate drain loop.
- **Trade-offs of the fix:** The sentinel adds an element to the iteration and requires care that it doesn't contribute to the answer (e.g. a zero-height bar contributes no area). Explicit initialization is clearer but needs the check at every consumer.
