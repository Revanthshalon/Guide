# Monotonic Stack & Queue — Quick Reference

## At a Glance

Maintain a sorted stack by **discarding elements that can never be the answer again**. If `v[j] > v[i]` and `j > i`, then `v[i]` is dead — `v[j]` is closer *and* larger.

**Invariant (stack):** holds **indices** with monotonic values; before pushing `i`, pop violators — **each pop's answer is `i`**.
**Invariant (deque):** indices increasing, values decreasing ⇒ **front is the window max**, and every index is inside the window.

Amortization: each element pushed once, popped once ⇒ **Θ(n)**.

## The Number

Sliding-window maximum, n = 1,000,000 (measured):

| k | Naive Θ(n·k) | Heap Θ(n log n) | **Deque Θ(n)** |
| --- | --- | --- | --- |
| 100 | 61.39 ms | 22.18 ms | **11.35 ms** (5×) |
| 10,000 | **6.62 s** | 12.30 ms | **11.20 ms** (**591×**) |

Deque is **k-independent** (11.35 → 11.20 ms) — the signature of the technique. Note the **heap is within 10%** at large k; the deque's real wins are k-independence and Θ(k) memory.

## Direction Table

| Want | Iterate | Pop while | Stack values |
| --- | --- | --- | --- |
| Next greater → right | L→R | `v[top] < v[i]` | decreasing |
| Next smaller → right | L→R | `v[top] > v[i]` | increasing |
| Previous greater ← left | R→L | `v[top] < v[i]` | decreasing |
| Previous smaller ← left | R→L | `v[top] > v[i]` | increasing |

## Snippets

```rust
// Next greater element — the pop IS the answer
for i in 0..n {
    while let Some(&top) = stack.last() {
        if v[top] < v[i] { next_greater[top] = Some(i); stack.pop(); } else { break; }
    }
    stack.push(i);
}
// Whatever remains on the stack has NO next-greater element.

// Sliding window max — ORDER MATTERS: maintain back → evict front → read front
for i in 0..n {
    while let Some(&b) = dq.back() { if v[b] <= v[i] { dq.pop_back(); } else { break; } }
    dq.push_back(i);
    if *dq.front().unwrap() + k <= i { dq.pop_front(); }
    if i + 1 >= k { out.push(v[*dq.front().unwrap()]); }
}
```

## Complexity

| Problem | Naive | **Monotonic** |
| --- | --- | --- |
| Next greater element | Θ(n²) | **Θ(n)** |
| Sliding window max | Θ(n·k) | **Θ(n)** |
| Largest rectangle in histogram | Θ(n²) | **Θ(n)** |
| Maximal rectangle in matrix | Θ(n²m) | **Θ(n·m)** |
| DP with windowed max transition | Θ(n·k) | **Θ(n)** |

## Recognizing It

- "next/previous greater/smaller" · "**span**" · "largest **rectangle**"
- "sliding window min/max" · "**visible** from a direction"
- **DP transition `dp[i] = max(dp[j])` over a moving range** ← highest-leverage use

## Choose This When

| Use | For |
| --- | --- |
| **Monotonic stack** | next/previous greater/smaller, spans, histogram |
| **Monotonic deque** | window min/max, windowed DP transitions |
| Heap | k varies, or you need the k-th largest too |
| Sliding window | The aggregate is a **sum**, not a min/max |
| Sparse table | Static array, arbitrary ranges, Θ(1) |
| Segment tree | Range min/max **with updates** |

## Rules of Thumb

- Store **indices**, never values — every maintenance rule is positional.
- Write the spec first ("next *strictly* greater"), then derive `<` vs `<=`.
- Order: maintain back → evict front → read front.
- Handle what's left on the stack — those have no answer.
- Keep the naive Θ(n·k) version as a permanent test oracle.
- Test with an **all-equal array** and a plateau; random data hides tie bugs.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Stored values not indices | Can't detect window expiry; duplicates break |
| `<` vs `<=` wrong | Correct on distinct data, wrong on plateaus |
| Read front before evicting | Max from outside the window, rarely |
| Ignored leftover stack | Suffix maxima get default answers |
| Assumed Θ(n) without checking pushes | It's amortized — verify pops ≤ pushes |

## Key References

- The histogram/rectangle problem is the canonical Θ(n) application
- CLRS ch. 15 — DP transitions the deque optimizes
