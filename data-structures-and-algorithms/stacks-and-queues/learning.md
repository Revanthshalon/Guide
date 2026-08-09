# Stacks & Queues — Learning Notes

## Mental Model

**Stacks and queues are disciplines, not structures.** They are defined by what they *refuse* to let you do: a stack forbids access to anything but the most recent element, a queue forbids anything but the oldest. Both are backed by an array in practice — `Vec` for a stack, `VecDeque` for a queue — so this topic is not about a new layout, it's about what a restricted interface buys you.

It buys three things:

1. **An explicit representation of "work still to do."** Every traversal, parse, and search is really a loop over a pending-work container, and *which* container you pick is the algorithm. BFS and DFS differ by exactly one line — queue versus stack. That equivalence is the single most useful idea in this doc.
2. **Escape from the call stack.** Recursion is a stack you don't control, with a hard 8 MB limit (measured in Stage 0: ~250k–300k frames before an uncatchable abort). Moving to an explicit `Vec` moves that limit to the heap, where it's bounded by RAM rather than by a linker setting.
3. **A place to put backpressure.** A queue is where a fast producer meets a slow consumer, which makes it the exact point at which you must decide what happens when they don't match. That decision — block, drop, or grow — is not a data-structure detail; it's the system's failure mode, and an unbounded queue chooses "grow until OOM" by default.

The mental model to carry: **a queue is a buffer, and every buffer is a latency-versus-loss decision waiting to be made.** Little's Law (L = λW: items in the queue = arrival rate × time in queue) is the arithmetic — a queue that's persistently deep isn't absorbing a burst, it's adding latency to every item that passes through it, forever.

## The Invariant

**Stack (`Vec` as stack):** the only accessible element is at `len - 1`; push and pop both act there. Nothing else needed — it's the array invariant with a restricted interface.

**Queue (`VecDeque`, a ring buffer):** this is where the real invariant lives.

> A `head` index and a `len`, over a buffer of `cap` slots. Element *i* of the queue lives at physical slot `(head + i) % cap`. The occupied region is `len` slots starting at `head`, **wrapping around the end of the buffer**; the rest is uninitialized.

Two consequences that cause most `VecDeque` surprises:

- **The elements are not one contiguous slice.** When the occupied region wraps, the queue is physically two runs. This is why `VecDeque` has `as_slices() -> (&[T], &[T])` rather than `as_slice()`. Measured: after `push_back(1); push_back(2); push_front(0)` on a capacity-4 deque, `as_slices()` returns `([0], [1, 2])` — the logical order 0,1,2 split across the wrap.
- **Full and empty look identical if you only track indices.** With a `head` and a `tail` index and nothing else, `head == tail` means both empty and full. Implementations resolve this by storing `len` explicitly (what std does), by keeping one slot always empty, or by using non-wrapping counters and taking the modulus only on access.

## Mechanics

### The stack

`Vec` is the stack: `push`, `pop`, `last`. Nothing more is needed, and nothing is faster — pushes and pops both touch the hot end of a contiguous buffer, which is the best access pattern hardware offers.

The important use isn't storing data; it's **replacing recursion**:

```rust
// Recursive DFS: elegant, and aborts at ~250k depth on the main thread.
fn dfs(g: &Graph, u: usize, seen: &mut [bool]) {
    seen[u] = true;
    for &v in &g.adj[u] { if !seen[v] { dfs(g, v, seen); } }
}

// Explicit stack: same traversal, depth bounded by RAM instead of by 8 MB.
fn dfs_iter(g: &Graph, start: usize, seen: &mut [bool]) {
    let mut stack = vec![start];
    while let Some(u) = stack.pop() {
        if seen[u] { continue; }          // mark on POP, not on push
        seen[u] = true;
        for &v in &g.adj[u] { if !seen[v] { stack.push(v); } }
    }
}
```

Two details in that rewrite that matter later in Stage 5: **the visit order differs** (an explicit stack visits the last-pushed neighbour first, so the traversal order is the mirror of the recursive version), and **you must decide whether to mark visited on push or on pop**. Marking on push prevents duplicates in the stack but changes the order; marking on pop needs the `if seen { continue }` guard because a node can be pushed several times before it's popped.

### The queue and the ring buffer

`VecDeque` is a growable ring buffer: Θ(1) amortized `push_front`, `push_back`, `pop_front`, `pop_back`, plus Θ(1) indexing. It grows by doubling like `Vec` — measured capacity sequence for `VecDeque<u64>`: 4, 8, 16, 32, 64, … On growth it must also *unwrap* the ring into the new buffer, which is why growth is slightly more expensive than `Vec`'s.

`make_contiguous()` rotates the elements so the queue becomes one run and returns `&mut [T]`. It's Θ(n) and it's the bridge to every API that wants a slice.

### The deque as a superset

A deque (double-ended queue) supports both ends, so it is simultaneously a stack and a queue. `VecDeque` is therefore the answer to "I need a queue", "I need a deque", and "I need a stack where I also occasionally look at the front". `Vec` remains better for a pure stack: 24 bytes vs 32, a genuine contiguous slice, and no modulo arithmetic on access.

### Bounded vs unbounded — the decision that actually matters

| Policy | What happens when full | Use when |
| --- | --- | --- |
| **Unbounded** | Grows until OOM | Only when the producer is provably bounded |
| **Block** | Producer waits | Producer can be slowed (backpressure propagates) |
| **Drop newest** | Reject the arriving item | Load shedding; the item can be refused |
| **Drop oldest** | Overwrite the head | Telemetry, metrics, latest-value-wins feeds |
| **Fail fast** | Return an error immediately | The caller has a fallback |

This table is the whole reason queues are a systems topic and not just a data-structure one. See [backpressure & rate limiting](../../architecture-patterns/backpressure-and-rate-limiting/learning.md) — the choice made here *is* the service's overload behaviour.

### The priority queue is a different animal

"Take the most important item" rather than oldest or newest is a **heap**, not a ring buffer — Θ(log n) rather than Θ(1), covered in Stage 4. Worth naming here because "queue" in a job system usually means priority queue, and the complexity is different.

## Complexity

| Operation | `Vec` (stack) | `VecDeque` | Space |
| --- | --- | --- | --- |
| Push back | Θ(1) amortized | Θ(1) amortized | — |
| Pop back | Θ(1) | Θ(1) | — |
| Push front | Θ(n) | **Θ(1) amortized** | — |
| Pop front | Θ(n) | **Θ(1)** | — |
| Peek either end | Θ(1) | Θ(1) | — |
| Index | Θ(1) | Θ(1) (one extra add + mask) | — |
| Contiguous slice | free | Θ(n) `make_contiguous` | — |
| Whole structure | — | — | Θ(cap) ≤ 2n |

**Where the table misleads:** `VecDeque`'s Θ(1) indexing carries a wrap computation and its iteration crosses a discontinuity, so a full scan is measurably slower than `Vec`'s and won't vectorize as readily. For a pure stack, `Vec` is the better choice for that reason alone — the deque's extra capability isn't free even when unused.

**The amortized spike applies here too:** both types double, so a `push` can trigger a full reallocation and copy. For latency-sensitive queues, `with_capacity` up front, or a fixed-capacity ring that never grows.

## Rust Implementation

```rust
// Stack: just a Vec.
let mut stack = Vec::with_capacity(expected_depth);
stack.push(item);
while let Some(top) = stack.pop() { /* ... */ }

// Queue: VecDeque.
use std::collections::VecDeque;
let mut q = VecDeque::with_capacity(cap);
q.push_back(item);
while let Some(front) = q.pop_front() { /* ... */ }

// The two-slice reality — this is NOT one contiguous run.
let (a, b) = q.as_slices();               // e.g. ([0], [1, 2]) after a wrap
let all: &mut [T] = q.make_contiguous();  // Θ(n) rotate, then it is

// Fixed-capacity ring with drop-oldest semantics — the telemetry buffer.
if ring.len() == CAP { ring.pop_front(); }
ring.push_back(sample);
```

**The BFS/DFS equivalence, in code** — worth internalizing before Stage 5:

```rust
// Change VecDeque→Vec and pop_front→pop and this becomes DFS. Nothing else changes.
let mut frontier = VecDeque::from([start]);
while let Some(u) = frontier.pop_front() {
    for &v in &g.adj[u] { if visit(v) { frontier.push_back(v); } }
}
```

**Crates and std beyond the basics:**

| Need | Use |
| --- | --- |
| Bounded async queue with backpressure | `tokio::sync::mpsc::channel(n)` — `send().await` blocks when full |
| Unbounded async queue | `tokio::sync::mpsc::unbounded_channel()` — **choosing OOM as your overload policy** |
| Bounded sync channel | `std::sync::mpsc::sync_channel(n)`, `crossbeam-channel` |
| Fixed-capacity, no allocation | `arraydeque`, `heapless::Deque` |
| Lock-free MPMC | `crossbeam::queue::ArrayQueue` (bounded), `SegQueue` (unbounded) |
| Priority ordering | `BinaryHeap` (Stage 4) |

## Use Cases

- **Graph traversal.** BFS wants a queue, DFS wants a stack. Same loop, one line different.
- **Recursion elimination.** Any recursive algorithm can be rewritten with an explicit stack; the reason to do it is depth (Stage 0's ~250k-frame ceiling), not elegance. Parsers on deeply nested input and tree walks over user-supplied data are the standard cases — user-controlled nesting depth is a denial-of-service vector if you recurse on it.
- **Expression evaluation and parsing.** The shunting-yard algorithm and every recursive-descent parser's operator handling are stacks; matched-bracket checking is the canonical exercise.
- **Undo/redo.** Two stacks, and the interaction between them (a new action clears the redo stack) is the whole design.
- **Producer/consumer pipelines.** Work queues between stages, where the bounded-vs-unbounded choice is the system's overload behaviour.
- **Sliding windows and monotonic structures.** A deque is the backing structure for Θ(n) sliding-window minimum/maximum — see the Stage 6 monotonic-queue topic.
- **Ring buffers for telemetry.** Fixed-capacity, drop-oldest: the last N log lines, the last N latency samples. Bounded memory by construction.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`Vec`** | Pure stack. Smaller, contiguous, faster to scan. |
| **`VecDeque`** | FIFO, or both ends. The default queue. |
| Fixed-capacity ring (`arraydeque`, manual) | Bounded memory required; drop-oldest telemetry; no allocator |
| `tokio::sync::mpsc::channel(n)` | Async producer/consumer where backpressure must propagate |
| `crossbeam::queue::ArrayQueue` | Lock-free, bounded, multi-producer multi-consumer |
| `BinaryHeap` | "Most important" rather than oldest/newest |
| `Vec::remove(0)` | **Never** — this is the Θ(n²) queue |

## Pitfalls in Depth

### Pitfall: The unbounded queue

- **What goes wrong:** A work queue is created unbounded because it's one fewer parameter to pick. Under normal load it holds 0–3 items. During an incident — a slow downstream, a GC pause, a deploy — the consumer falls behind, the queue grows to millions of entries, and the process is OOM-killed. The postmortem blames the downstream service, but the *unrecoverable* failure was the queue: a bounded queue would have shed load and stayed alive.
- **Why it happens (the mechanism):** An unbounded queue hasn't avoided the capacity decision, it has answered it with "grow until the allocator fails." It also silently converts a throughput problem into a latency problem: by Little's Law, a queue holding L items at arrival rate λ adds L/λ seconds of latency to *every* item. A deep queue isn't absorbing a burst — it's making every request slow while hiding that the system is over capacity.
- **How to handle it in production, and why that works:** Bound every queue, and choose the full-policy explicitly: block (propagates backpressure to the producer, which is usually right), drop-newest (load shedding), or drop-oldest (telemetry). Bounding turns an invisible unbounded latency growth into a visible, measurable rejection you can alarm on. Then monitor queue *depth* and *age of oldest item* — the two numbers that reveal a queue being used as a buffer for a permanent mismatch.
- **Trade-offs of the fix:** A bound is a number you can get wrong: too small and you reject during normal bursts, too large and you're back to hidden latency. Blocking propagates backpressure, which is correct but means the producer must be able to handle being slowed — if it's an HTTP handler, that's a timeout, and the failure has to be designed rather than discovered.

### Pitfall: `Vec::remove(0)` as a queue

- **What goes wrong:** A work list is drained with `while !v.is_empty() { let job = v.remove(0); … }`. Every removal shifts the remaining n−1 elements down one slot, making the drain Θ(n²). At 1,000 jobs it's 500k moves and invisible; at 200,000 it's 20 billion and the service appears hung with one core pinned.
- **Why it happens (the mechanism):** `remove(0)` is the most natural spelling of "take the oldest", and its cost is invisible at the call site — the method looks like `pop`. It shifts because `Vec` must keep elements contiguous starting at index 0; the ring buffer's whole purpose is to avoid exactly this by moving the *start index* instead of the data.
- **How to handle it in production, and why that works:** `VecDeque::pop_front` — Θ(1), because it increments `head` rather than moving n elements. If the order doesn't matter, `Vec::pop()` from the back is Θ(1) and keeps the contiguous slice.
- **Trade-offs of the fix:** `VecDeque` is 32 bytes rather than 24, indexing costs an extra add and mask, and you lose the single contiguous slice — code doing SIMD, FFI, or `&[T]` handoff needs `make_contiguous()` at Θ(n). If the collection is genuinely LIFO, staying with `Vec` and popping the back is better than switching.

### Pitfall: Assuming `VecDeque` is contiguous

- **What goes wrong:** Code calls `as_slices()`, uses `.0`, and ignores `.1` — or worse, indexes into `.0` assuming it holds everything. It works in every test, because a deque that has never wrapped returns everything in the first slice and an empty second. The first time the ring wraps in production, the code silently processes a prefix of the data.
- **Why it happens (the mechanism):** The occupied region is `len` slots starting at `head`, modulo `cap` — so once `head + len > cap`, the elements are physically two runs. Measured: `push_back(1); push_back(2); push_front(0)` yields `as_slices() == ([0], [1, 2])`. Whether a wrap has occurred depends on the *history* of pushes and pops, not on the current contents, which is why it's untestable by inspection.
- **How to handle it in production, and why that works:** Never use `.0` alone. Either handle both slices (`chain` them), iterate with `q.iter()` which handles the wrap for you, or call `make_contiguous()` once and take the single `&mut [T]`. If a contiguous slice is needed on the hot path, that's a signal the structure should have been a `Vec` with a different access pattern.
- **Trade-offs of the fix:** `make_contiguous()` is Θ(n) and needs `&mut`, so calling it per iteration turns an Θ(1) queue into an Θ(n) one. Handling two slices is more code and easy to get subtly wrong in reverse iteration. Iterating with `.iter()` is the safe default and gives up the ability to hand a slice to a slice-taking API.

### Pitfall: Recursion where the depth is user-controlled

- **What goes wrong:** A recursive JSON/XML/expression parser, or a recursive tree walk, runs on input from a user. Deeply nested input — `[[[[[[…]]]]]]` — drives recursion past the stack limit and the process **aborts**. Not a panic, not catchable, no error response: the whole process dies, taking every other in-flight request with it. It's a trivially triggered denial of service.
- **Why it happens (the mechanism):** The call stack is a fixed-size resource set at thread creation: 8 MB for the main thread, 2 MB default for spawned threads. Overflow is detected by a guard page and aborts, because unwinding from a stack overflow isn't safe. Nothing in the type system marks a function as depth-unbounded, so the risk is invisible in review.
- **How to handle it in production, and why that works:** Either convert to an explicit stack (depth then bounded by RAM, and you can check the stack's length and return a clean error), or keep recursion but enforce an explicit depth limit checked on entry — a `depth > MAX_DEPTH` guard returning `Err` turns an abort into a rejected request. `serde_json` does exactly this, and it's why it has a recursion limit.
- **Trade-offs of the fix:** The explicit-stack rewrite is significantly less readable than recursion for tree algorithms and is easy to get wrong (the mark-on-push-vs-pop question above). A depth limit is a magic number that will eventually reject legitimate input. For non-adversarial input where depth is provably logarithmic (balanced trees), recursion is correct and the rewrite is wasted complexity — the trigger is *user-controlled depth*, not recursion itself.

### Pitfall: Treating queue depth as a healthy buffer

- **What goes wrong:** A dashboard shows the work queue sitting at 50,000 items. It's been that way for weeks and nothing is failing, so it's treated as normal — the queue is "absorbing bursts". In reality every item now waits behind 50,000 others, and end-to-end latency includes that wait; when the queue does finally start growing, there's no headroom left to absorb anything.
- **Why it happens (the mechanism):** A queue that is *persistently* non-empty is not buffering, it's the standing evidence that arrival rate exceeds service rate at least some of the time and the system never catches up. Little's Law makes it quantitative: L = λW, so at 1,000 items/sec a standing depth of 50,000 is 50 seconds of added latency per item. A buffer that absorbs bursts should return to near-empty between them.
- **How to handle it in production, and why that works:** Alert on **queue depth** and **age of the oldest item**, with the target being "returns to near zero between bursts", not "stays below the bound". Age-of-oldest is the more honest signal because it's directly the latency being added. If depth doesn't drain, the answer is capacity or shedding, not a bigger queue.
- **Trade-offs of the fix:** Some workloads legitimately keep a deep queue — batch pipelines where throughput is the only goal and latency is irrelevant. The alert threshold is therefore per-queue, and getting it wrong generates noise that trains people to ignore it. Distinguish latency-sensitive queues from throughput-only ones explicitly, and only alarm on the former.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if pop returned a new queue? | The **banker's queue**: two stacks, front and reversed back — amortized Θ(1) persistent queue |
| Batch it | What if you pushed/popped k at a time? | Chunked queues; `SegQueue`'s segments; batched channel sends amortizing the sync cost |
| Approximate it | What if it could lose items when full? | Drop-oldest ring buffers — bounded memory by construction |
| Randomize it | What if you popped a random element? | A "bag" — the third traversal order, giving randomized search |
| Externalize it | What if it lived on disk? | Durable message queues; the [outbox pattern](../../architecture-patterns/outbox-pattern/learning.md) is exactly a queue in a database table |
| Parallelize it | Where's the contention? | Both ends contend — hence MPMC ring buffers (`ArrayQueue`), LMAX Disruptor, per-worker deques with **work stealing** (steal from the *opposite* end to minimize contention) |
| Invert it | What if you popped the newest instead of the oldest? | Stack ↔ queue — and BFS ↔ DFS, one line apart |
| Augment it | What does a priority per item buy? | The heap, and the whole of Stage 4 |
| Specialize it | What if capacity were a power of two? | `% cap` becomes `& (cap-1)` — one instruction instead of a division |
| Amortize it | What if one push could be terrible? | Doubling ring buffer; or two-stack queue where one reversal pays for n pops |

**Questions:**

1. BFS and DFS differ by one line. What does that say about where an algorithm's identity actually lives — and can you name a third traversal you get by swapping in a different container?
2. The banker's queue makes a queue from two stacks with amortized Θ(1) pops. Explain where the amortization comes from, then explain why it *breaks* under persistence unless you add laziness.
3. Under "parallelize it", work-stealing deques have the owner push/pop one end and thieves steal from the other. Why that end specifically, and what does it do to the number of atomic operations on the common path?
4. A ring buffer with power-of-two capacity replaces `%` with `&`. What did you give up, and at what capacity does the wasted space start to matter?
5. `tokio::sync::mpsc::unbounded_channel()` exists and is widely used. State the overload policy it implicitly chooses, and name the one situation where it's the correct choice.
6. Little's Law says L = λW. Your queue holds a steady 50,000 items at 1,000/sec. Give the added latency, then argue whether a *bigger* queue could ever help.
7. Under "externalize it", the outbox pattern is a queue in a database table. Which property of a ring buffer does it keep, which does it give up, and what does it gain in return?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the ring-buffer invariant, and explain why `head == tail` is ambiguous and the three ways implementations resolve it.
2. Why does `VecDeque` have `as_slices()` rather than `as_slice()`? Give the exact push sequence that makes the difference observable.
3. Convert a recursive DFS to an explicit stack. Name the two behavioural differences you must consciously decide about.
4. Your queue is unbounded and the consumer is slow. Describe the failure, then give the three bounded policies and a workload for each.
5. Why is `Vec::remove(0)` in a loop Θ(n²) while `VecDeque::pop_front` is Θ(1)? Answer in terms of what moves.
6. When is `Vec` the better stack than `VecDeque`, given `VecDeque` can do everything `Vec` can at the back?

Build exercises:

- Implement a fixed-capacity ring buffer over `[MaybeUninit<T>; N]` with `push_back`, `pop_front`, `is_full`, and correct `Drop` for the occupied region only. Run it under Miri. This forces the wrap arithmetic and the initialized-region invariant to become concrete, and it's the foundation for the lock-free queues in Stage 9.
- Write one graph traversal function parameterized over the frontier container, and show BFS and DFS falling out of the same code. Then add a third: swap in a `BinaryHeap` keyed by distance and observe Dijkstra appear (Stage 5, previewed).
- Build the recursion-depth DoS: write a recursive parser for nested brackets, feed it 1M nested `[`, and watch it abort. Then fix it twice — once with an explicit stack, once with a depth limit — and compare the failure modes (clean error vs process death).
- Measure the unbounded-queue failure: producer at 10k/sec, consumer at 5k/sec, unbounded channel. Plot RSS and end-to-end latency over time. Then bound it at 1,000 and plot the same. The two graphs are the argument for bounding, in a form you can show someone.

## Open Questions

- What does `VecDeque`'s wrap arithmetic actually cost on a full scan versus `Vec` on this machine — and does the compiler ever eliminate it when the deque provably hasn't wrapped?
- Does std's `VecDeque` use a power-of-two capacity (mask) or a general modulus now? Read the source and confirm rather than assume.
- `crossbeam::ArrayQueue` vs `tokio::sync::mpsc` bounded, for the same producer/consumer shape: measure throughput and p99 rather than reasoning about it.
- Two-stack (banker's) queue vs `VecDeque` in Rust: is there any workload where the amortized version wins, or is the ring buffer strictly better outside a persistence requirement?
- What's a defensible default bound for a work queue when the arrival rate is unknown — is there a principled starting point, or is it always measure-then-set?

## References

- [`VecDeque` documentation](https://doc.rust-lang.org/std/collections/struct.VecDeque.html) — read `as_slices` and `make_contiguous` carefully; they encode the wrap invariant.
- Chris Okasaki, *Purely Functional Data Structures*, ch. 5–6 — the banker's and physicist's queues; the clearest treatment of amortization interacting with persistence, and the source for the "persist it" lens above.
- LMAX Disruptor technical paper — a ring buffer engineered for mechanical sympathy: power-of-two masking, false-sharing padding on the cursors, batching. The best single case study of a queue as a performance artifact.
- Related topics in this repo: [Arrays & Dynamic Arrays](../arrays-and-dynamic-arrays/learning.md) (the backing structure), [Linked Lists](../linked-lists/learning.md) (what a queue is *not* usually built from), [Backpressure & Rate Limiting](../../architecture-patterns/backpressure-and-rate-limiting/learning.md) (bounded-vs-unbounded as a system property, Little's Law in anger), [Outbox Pattern](../../architecture-patterns/outbox-pattern/learning.md) (a durable queue in a database), [Parallelism & Work Stealing](../../performance-optimization/parallelism-and-work-stealing/learning.md) (the deque as the scheduler's core structure), [Async & I/O](../../performance-optimization/async-and-io/learning.md) (bounded channels and cancellation).
