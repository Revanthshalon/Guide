# Stacks & Queues — Quick Reference

## At a Glance

Disciplines, not structures — defined by what they forbid. `Vec` is the stack, `VecDeque` (a ring buffer) is the queue. The real decisions are *recursion vs explicit stack* and *bounded vs unbounded*.

**Invariant (ring buffer):** element *i* lives at `(head + i) % cap`; the occupied run is `len` slots from `head` and **may wrap**; `head == tail` alone can't distinguish full from empty — store `len`.

## Complexity

| Operation | `Vec` (stack) | `VecDeque` |
| --- | --- | --- |
| Push/pop back | Θ(1) amortized | Θ(1) amortized |
| Push/pop front | **Θ(n)** | **Θ(1)** |
| Peek either end | Θ(1) | Θ(1) |
| Index | Θ(1) | Θ(1) + wrap math |
| Contiguous slice | free | Θ(n) `make_contiguous` |
| Space | Θ(cap) ≤ 2n | Θ(cap) ≤ 2n |

`VecDeque<u64>` growth (measured): 4, 8, 16, 32, 64, …

## Choose This When

| Use | For |
| --- | --- |
| `Vec` | Pure stack — smaller (24 B vs 32 B), contiguous, scans faster |
| `VecDeque` | FIFO or both ends — the default queue |
| Fixed ring (`arraydeque`, `heapless`) | Bounded memory, drop-oldest telemetry, no allocator |
| `tokio::sync::mpsc::channel(n)` | Async producer/consumer with real backpressure |
| `crossbeam::queue::ArrayQueue` | Lock-free bounded MPMC |
| `BinaryHeap` | "Most important", not oldest/newest |
| `Vec::remove(0)` | **Never** — that's the Θ(n²) queue |

## Full-Queue Policy — Choose Explicitly

| Policy | On full | Use when |
| --- | --- | --- |
| Unbounded | grows → OOM | producer provably bounded |
| Block | producer waits | backpressure should propagate |
| Drop newest | reject arrival | load shedding |
| Drop oldest | overwrite head | telemetry, latest-wins |
| Fail fast | error to caller | caller has a fallback |

**Little's Law:** L = λW. Standing depth 50,000 at 1,000/s = **50 s added latency per item**.

## Snippets

```rust
// BFS ⇄ DFS: change the container and the pop end. Nothing else.
let mut frontier = VecDeque::from([start]);
while let Some(u) = frontier.pop_front() { /* BFS */ }
// let mut frontier = vec![start];
// while let Some(u) = frontier.pop() { /* DFS */ }

// VecDeque is TWO runs when wrapped
let (a, b) = q.as_slices();          // e.g. ([0], [1, 2]) — never use .0 alone
let all = q.make_contiguous();       // Θ(n), needs &mut

// Drop-oldest ring
if ring.len() == CAP { ring.pop_front(); }
ring.push_back(sample);
```

## Rules of Thumb

- Bound every queue; pick the full-policy explicitly.
- Monitor **depth** and **age of oldest** — target "returns to ~0 between bursts", not "below the bound".
- User-controlled recursion depth is a DoS: explicit stack, or an explicit depth limit.
- Recursion ceiling: ~250k frames on 8 MB main thread, ~4× lower on a 2 MB spawned thread.
- Explicit DFS: decide mark-on-push vs mark-on-pop; visit order mirrors the recursive version.
- Power-of-two capacity turns `%` into `&`.
- `with_capacity` to avoid the doubling spike.

## Implementation Checklist

- [ ] Queue is bounded, policy chosen and documented
- [ ] Depth + age-of-oldest metrics exported
- [ ] Never `Vec::remove(0)`
- [ ] `as_slices()` — both halves handled, or `.iter()`, or `make_contiguous()`
- [ ] Recursion on user input replaced or depth-limited
- [ ] `with_capacity` on latency-sensitive queues
- [ ] Custom ring: `Drop` covers only the occupied (possibly wrapped) region; Miri-checked

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Unbounded queue | OOM kill during an incident; blamed on downstream |
| `Vec::remove(0)` drain | Hangs at 200k items, fine at 1k |
| Used `as_slices().0` only | Silently processes a prefix once the ring wraps |
| Recursive parse of user input | Process **aborts** (not a panic) on nested input |
| Standing deep queue | Latency includes the whole wait; no burst headroom left |
| `head == tail` with no `len` | Full and empty indistinguishable |

## Key References

- [`VecDeque` docs](https://doc.rust-lang.org/std/collections/struct.VecDeque.html) — `as_slices`, `make_contiguous`
- Okasaki, *Purely Functional Data Structures* ch. 5–6 — banker's queue
- LMAX Disruptor paper — the ring buffer as a performance artifact
