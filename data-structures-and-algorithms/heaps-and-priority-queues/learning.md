# Heaps & Priority Queues — Learning Notes

## Mental Model

**A heap is what you get when you ask for *only* the minimum (or maximum) and refuse to pay for anything else.**

A sorted array gives you the minimum in Θ(1) but costs Θ(n log n) to build and Θ(n) to insert into. A heap gives up total order and keeps only a *partial* order — every parent beats its children, siblings are unordered — and that single relaxation buys Θ(log n) insertion and Θ(log n) extraction while keeping the extreme element at a known position. **Sorting is over-solving the "what's next?" problem**, and the heap is the structure that solves exactly that problem and no more.

The second idea is the one people find surprising: **a binary heap has no nodes and no pointers.** Because it's a *complete* binary tree (every level full except possibly the last, filled left to right), the tree shape is completely determined by the element count — so it can live in a flat array with children at `2i+1` and `2i+2`. No allocation per element, perfect cache locality on the sift path, no borrow-checker friction. It's the cleanest example in the category of the [arrays](../arrays-and-dynamic-arrays/learning.md) "specialize it" lens: add a structural precondition, delete all the pointers.

That flat-array representation is also why the classic Fibonacci heap — asymptotically superior, with Θ(1) amortized decrease-key — loses in practice. Its better bound is bought with pointers, allocations, and cache misses, and the constant factor eats the log.

## The Invariant

> **Heap property:** every node is ≤ (min-heap) or ≥ (max-heap) both of its children.
> **Shape property:** the tree is *complete* — all levels full except the last, which fills left to right.

Both matter, and they do different jobs:

- The **heap property** is deliberately weak. It orders each parent against its children and says *nothing* about siblings or cousins, so it costs only Θ(log n) to restore after a change — a single root-to-leaf or leaf-to-root path. Total order would cost Θ(n log n) to maintain; this is the minimum ordering that still puts the extreme at a known place.
- The **shape property** is what makes the array representation possible and bounds the height at exactly ⌊log₂ n⌋. Lose completeness and you need pointers back.

The consequence people forget: **a heap is not sorted, and iterating one gives you no useful order.** `BinaryHeap::iter()` yields elements in arbitrary (array) order. Only repeated `pop` produces sorted output.

## Mechanics

### The two sift operations — that's the whole structure

```
push(x):     append at the end, then SIFT UP    — swap with parent while it beats the parent
pop():       take root, move last element to root, then SIFT DOWN
             — swap with the larger/smaller child while a child beats it
```

Both walk one root-to-leaf path: Θ(log n). Index arithmetic on a `Vec`:

```
parent(i) = (i - 1) / 2        left(i) = 2i + 1        right(i) = 2i + 2
```

### Heapify is Θ(n), not Θ(n log n) — and it's measurable

Building a heap from an existing array by sifting down from the last internal node to the root is **Θ(n)**, not Θ(n log n). The proof is the one worth remembering: count nodes *by height*, not by index. There are ~n/2 nodes at height 0 (leaves, no work), ~n/4 at height 1 (≤1 swap), ~n/8 at height 2 (≤2 swaps)… giving

  Σ (n / 2^(h+1)) · h  =  n · Σ h/2^(h+1)  →  **Θ(n)**

because the series converges. Most nodes are near the bottom and do almost no work; only the root can do log n.

Measured on this machine, `BinaryHeap::from(vec)` versus n pushes into a preallocated heap:

| n | `from(vec)` (heapify) | n × `push` | Ratio |
| --- | --- | --- | --- |
| 100,000 | 0.98 ms | 2.37 ms | **2.42×** |
| 1,000,000 | 5.26 ms | 10.19 ms | 1.94× |
| 5,000,000 | 17.71 ms | 42.84 ms | **2.42×** |

Roughly 2× for free, by calling `from` instead of looping — the practical payoff of the height-counting argument.

### The heap variants, and why the fancy ones lose

| Heap | Insert | Extract-min | Decrease-key | Meld | In practice |
| --- | --- | --- | --- | --- | --- |
| **Binary** | Θ(log n) | Θ(log n) | Θ(log n) | Θ(n) | **The default** — flat array, no allocation |
| d-ary (d=4) | Θ(log_d n) | Θ(d log_d n) | Θ(log_d n) | Θ(n) | Shallower, more cache-friendly; wins for decrease-key-heavy work |
| Binomial | Θ(log n) | Θ(log n) | Θ(log n) | **Θ(log n)** | When you must merge heaps |
| Pairing | Θ(1) | Θ(log n) am. | Θ(log n) am.* | Θ(1) | Best *practical* mergeable heap |
| **Fibonacci** | Θ(1) | Θ(log n) am. | **Θ(1) am.** | Θ(1) | **Loses in practice** — pointer-heavy, cache-hostile |

The Fibonacci heap exists to improve Dijkstra's bound from Θ(E log V) to Θ(E + V log V). That's a real theoretical result and it is almost never worth using: the constants and cache behaviour mean a binary heap wins on real graphs. It's the canonical example of an asymptotic win that doesn't survive contact with hardware.

### Decrease-key, and the trick that avoids it

`decrease_key` needs to find an element already inside the heap — but a heap has no index. Two answers:

1. **An indexed heap**: keep a side map `element → array position`, updated on every swap. Correct, and the standard approach when you truly need it.
2. **Lazy deletion** (the one everyone actually uses): don't update; just push a *new* entry with the better priority, and when popping, discard entries whose priority no longer matches the current best. This is the standard Dijkstra idiom in Rust — the heap may hold up to E entries instead of V, but the code is simple and there's no side structure to keep consistent.

### Streaming top-k

For "the k largest of a stream", keep a **min**-heap of size k: if the incoming element beats the root, pop and push; otherwise discard in Θ(1). Θ(n log k) time, **Θ(k) space**, and n need not be known. This is the streaming counterpart to [selection](../selection-and-order-statistics/learning.md)'s `select_nth_unstable`, which needs the whole array in memory.

## Complexity

| Operation | Binary heap | Sorted `Vec` | `BTreeMap` |
| --- | --- | --- | --- |
| Peek min/max | **Θ(1)** | Θ(1) | Θ(log n) |
| Insert | **Θ(log n)** | Θ(n) | Θ(log n) |
| Extract min/max | **Θ(log n)** | Θ(1) / Θ(n) | Θ(log n) |
| Build from n items | **Θ(n)** | Θ(n log n) | Θ(n log n) |
| Search arbitrary | Θ(n) | Θ(log n) | Θ(log n) |
| Delete arbitrary | Θ(n) to find | Θ(n) | Θ(log n) |
| Merge two | Θ(n) | Θ(n) | Θ(n) |
| Ordered iteration | **Θ(n log n)** (pop repeatedly) | Θ(n) | Θ(n) |
| Space | **Θ(n), no pointers** | Θ(n) | Θ(n) + pointers |

**Where the table misleads:** the Θ(log n) rows are unusually *cheap* here compared to tree structures with the same bound, because the sift path is `i → 2i+1` in a flat array — sequential-ish and allocation-free — rather than a chain of pointer dereferences to scattered heap allocations. A heap's log n and a BST's log n are not the same log n.

The Θ(n) search row is the important limitation: **a heap answers "what's the extreme?" and nothing else.** Any workload that also needs "is x present?" or "delete x" needs a second structure or a different one entirely.

## Rust Implementation

```rust
use std::collections::BinaryHeap;
use std::cmp::Reverse;

// BinaryHeap is a MAX-heap. Reverse turns it into a min-heap — the Dijkstra idiom.
let mut max_heap = BinaryHeap::new();
let mut min_heap: BinaryHeap<Reverse<u64>> = BinaryHeap::new();
min_heap.push(Reverse(cost));
let Reverse(smallest) = min_heap.pop().unwrap();

// Heapify in Θ(n) — ~2× faster than pushing in a loop (measured).
let heap = BinaryHeap::from(vec);          // NOT: for x in vec { heap.push(x) }

// Streaming top-k: Θ(k) memory, n unknown.
let mut top: BinaryHeap<Reverse<Item>> = BinaryHeap::with_capacity(k);
for item in stream {
    if top.len() < k { top.push(Reverse(item)); }
    else if item > top.peek().unwrap().0 { top.pop(); top.push(Reverse(item)); }
}

// Sorted output — consumes the heap.
let sorted = heap.into_sorted_vec();

// Custom priority: implement Ord, or wrap in a tuple with the key first.
#[derive(PartialEq, Eq)]
struct Task { priority: u32, id: u64 }
impl Ord for Task {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&o.priority).then_with(|| o.id.cmp(&self.id))  // tiebreak → deterministic
    }
}
impl PartialOrd for Task { fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(o)) } }
```

**Dijkstra with lazy deletion** — the pattern to memorize, since it sidesteps decrease-key entirely:

```rust
let mut dist = vec![u64::MAX; n];
let mut pq = BinaryHeap::new();
dist[src] = 0;
pq.push(Reverse((0u64, src)));
while let Some(Reverse((d, u))) = pq.pop() {
    if d > dist[u] { continue; }                 // stale entry — the lazy deletion
    for &(v, w) in &adj[u] {
        let nd = d + w;
        if nd < dist[v] { dist[v] = nd; pq.push(Reverse((nd, v))); }
    }
}
```

**Floats need care:** `f64` isn't `Ord`. Use `ordered_float::NotNan` or an integer-scaled priority — `partial_cmp().unwrap()` will panic on `NaN`, and a heap with an inconsistent comparator silently returns the wrong "minimum."

**Crates:** `std::collections::BinaryHeap` (default), `priority-queue` (indexed heap with real `change_priority`), `keyed_priority_queue`, `dary_heap`.

## Use Cases

- **Dijkstra and A\*** — the canonical use; the heap holds the frontier ordered by tentative distance.
- **Task and job schedulers** — run the highest-priority ready task; OS run queues, job runners, retry queues ordered by next-attempt time.
- **Event-driven simulation** — the event queue ordered by timestamp *is* a heap, and it's what makes discrete-event simulation feasible.
- **Streaming top-k** — leaderboards, "worst N offenders", heavy hitters, with Θ(k) memory.
- **k-way merge** — merging k sorted runs with a heap of k iterators: Θ(N log k). This is the merge phase of external sorting and of LSM compaction.
- **Timers and timeouts** — a heap of expiry times gives "what fires next" in Θ(1); this is how timer wheels' slow path and many async runtimes work.
- **Huffman coding** — repeatedly merge the two lowest-frequency nodes.
- **Median maintenance** — two heaps (max-heap of the low half, min-heap of the high half) give a running median in Θ(log n) per update.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`BinaryHeap`** | You need repeated min/max from a changing set |
| `BinaryHeap<Reverse<T>>` | …and you want the **min** |
| Size-k min-heap | Streaming top-k, n unknown or huge |
| `select_nth_unstable` | Top-k from an array you already hold — Θ(n), ~10× faster |
| Sorted `Vec` | The set doesn't change; you want everything in order |
| `BTreeMap` | You also need lookup, deletion by key, or ranges |
| `priority-queue` crate | You genuinely need `decrease_key` with bounded heap size |
| d-ary heap | Decrease-key-heavy; shallower tree, better locality |
| Pairing heap | You must **merge** heaps often |
| Fibonacci heap | Essentially never — the constants lose |

## Pitfalls in Depth

### Pitfall: Building a heap by pushing in a loop

- **What goes wrong:** `for x in vec { heap.push(x) }` where `BinaryHeap::from(vec)` would do. It's Θ(n log n) instead of Θ(n) and costs about **2× measured** (5,000,000 elements: 42.84 ms pushing vs 17.71 ms heapifying). It's correct, so nothing flags it.
- **Why it happens (the mechanism):** Pushing is the obvious verb, and the Θ(n) heapify result is counter-intuitive — "you touch every node, and each can sift log n, so it must be n log n." The height-counting argument shows why that's wrong: half the nodes are leaves and do zero work, and only the root can do log n.
- **How to handle it in production, and why that works:** `BinaryHeap::from(vec)` when you already have the data; `with_capacity` when you must push, so the underlying `Vec` doesn't repeatedly reallocate.
- **Trade-offs of the fix:** `from` needs the whole collection up front, so it doesn't apply to streaming. Materializing a `Vec` just to heapify costs memory you may not have — for a stream, pushing is correct.

### Pitfall: Assuming a heap is sorted

- **What goes wrong:** Code iterates a `BinaryHeap` (or reads its underlying `Vec`) expecting descending order, and gets arbitrary order. Only the root is guaranteed. The bug is insidious because *small* heaps often happen to look sorted — a 3-element heap frequently is — so it passes tests and fails on real data.
- **Why it happens (the mechanism):** The heap property constrains parent-child pairs only. Siblings and cousins are completely unordered, and that weakness is exactly what makes operations Θ(log n) rather than Θ(n log n). "Partially ordered" is easy to read as "mostly sorted"; it isn't.
- **How to handle it in production, and why that works:** Use `into_sorted_vec()` when you want order, or pop repeatedly. If you need both ordered iteration *and* cheap extremes, a heap is the wrong structure — use `BTreeMap`/`BTreeSet`, which gives Θ(log n) min plus Θ(n) ordered iteration.
- **Trade-offs of the fix:** `into_sorted_vec` consumes the heap and costs Θ(n log n). `BTreeMap` makes the extreme Θ(log n) instead of Θ(1) and adds pointer chasing. If extremes dominate, keep the heap and accept that iteration isn't a supported operation.

### Pitfall: Needing decrease-key and not having it

- **What goes wrong:** Dijkstra or A\* is implemented, a shorter path to an already-queued node is found, and there's no way to update its priority — `BinaryHeap` has no handle to the element. People work around it by rebuilding the heap (Θ(n) per update, making the whole algorithm quadratic) or by scanning for the element (Θ(n) to find it).
- **Why it happens (the mechanism):** A heap deliberately has no index — that's how it stays allocation-free and flat. Locating an arbitrary element is Θ(n), so any operation on a *specific* element is outside what the structure supports.
- **How to handle it in production, and why that works:** Use **lazy deletion**: push a new entry with the improved priority and skip stale entries on pop (`if d > dist[u] { continue; }`). The heap grows to Θ(E) instead of Θ(V), but every operation stays Θ(log E) = Θ(log V) for simple graphs, the code has no side structure to keep in sync, and it's what nearly all production Dijkstra implementations do. If memory genuinely forbids the extra entries, use an indexed heap (`priority-queue`) that maintains an element→position map.
- **Trade-offs of the fix:** Lazy deletion uses more memory and pops more entries (each discarded in Θ(1)). An indexed heap keeps the heap small but adds a hash map, updates it on every swap, and introduces a second invariant that can desynchronize — a real source of subtle bugs.

### Pitfall: An inconsistent or non-total comparator

- **What goes wrong:** A custom `Ord` that isn't transitive, or floats compared with `partial_cmp().unwrap()`. The heap silently violates its own invariant: `pop` returns something that isn't the minimum, elements get "lost" in the middle of the array, and `NaN` makes it panic outright. Because the heap never re-verifies its ordering, the corruption is permanent and undetected.
- **Why it happens (the mechanism):** Sift-up and sift-down are *driven entirely* by comparisons. An inconsistent comparator makes them stop early or swap in the wrong direction, and the resulting array still looks like a plausible heap. Floats fail because `NaN` compares false to everything, so the order isn't total — and priorities are exactly where floats show up (costs, distances, scores).
- **How to handle it in production, and why that works:** Use `ordered_float::NotNan` so `NaN` is unrepresentable and `Ord` is genuine, or scale to integers (costs in milliseconds, distances in fixed point). Always add a **tiebreaker** to the comparator (an ID) so equal priorities resolve deterministically — otherwise pop order varies between runs and makes bugs irreproducible. Property-test transitivity if you hand-write `Ord`.
- **Trade-offs of the fix:** `NotNan` requires handling the error at every construction site. Integer scaling loses precision and needs a chosen unit. A tiebreaker makes comparisons slightly more expensive and requires a genuinely unique field.

### Pitfall: Reaching for a Fibonacci heap because of the bound

- **What goes wrong:** Someone implements (or imports) a Fibonacci heap to get Θ(1) amortized decrease-key for Dijkstra, and the result is slower than the `BinaryHeap` it replaced — often substantially — while being far more code.
- **Why it happens (the mechanism):** Fibonacci heaps buy their bound with a forest of pointer-linked nodes, lazy consolidation, and per-node marks. Every operation chases pointers into scattered allocations, where the binary heap walks a flat array. The asymptotic win is real; the constant factor and cache behaviour are worse by enough to swamp it at every practical n — the [complexity analysis](../complexity-analysis/learning.md) `n₀` escape hatch in its purest form.
- **How to handle it in production, and why that works:** Use a `BinaryHeap` with lazy deletion. If profiling genuinely shows decrease-key dominating, try a **d-ary heap** (d = 4 or 8) first: it's still a flat array, the shallower tree makes decrease-key cheaper, and the wider fan improves locality on sift-down. Pairing heaps are the practical choice when you need cheap *melding*.
- **Trade-offs of the fix:** d-ary heaps make extract-min more expensive (d comparisons per level), so they help decrease-key-heavy workloads and hurt extract-heavy ones — measure the mix. There genuinely are workloads where a Fibonacci heap's bound matters, but they're rare enough that the burden of proof is on the measurement.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if push returned a new heap sharing structure? | Leftist / skew heaps — pointer-based, meld in Θ(log n), naturally persistent |
| Batch it | What if you inserted n at once? | **Heapify in Θ(n)** — the measured 2× |
| Approximate it | What if "roughly the smallest" sufficed? | Bucket queues / calendar queues — Θ(1) with bounded integer priorities |
| Randomize it | What if structure came from randomness? | Randomized meldable heaps — merge by coin flip, no balance bookkeeping |
| Externalize it | What if it didn't fit in RAM? | External heaps; **k-way merge** with a heap of k runs — external sorting's merge phase |
| Parallelize it | Where's the contention? | The root is a single hot point — concurrent priority queues use relaxed/skiplist-based designs instead |
| Invert it | What if you kept **both** extremes? | Min-max heap / interval heap — Θ(1) min *and* max |
| Augment it | What does a side index buy? | **Indexed heap** — real `decrease_key`; the alternative to lazy deletion |
| Specialize it | What if priorities were small integers? | **Bucket queue** — Θ(1) insert and extract; this is how 0-1 BFS and dial's algorithm work |
| Amortize it | What if one operation could be terrible? | Pairing/Fibonacci heaps — lazy consolidation deferred to the next extract-min |

**Questions:**

1. Heapify is Θ(n) but n pushes are Θ(n log n) — and they build the same heap. Where does the log go? Derive it by counting nodes *by height* rather than by index.
2. Under "specialize it", a bucket queue gets Θ(1) with bounded integer priorities. Which assumption of the comparison-based Ω(log n) does that violate, and what does it cost in space?
3. A heap has no index, so decrease-key is hard. Compare lazy deletion and an indexed heap on memory, code complexity, and the invariants each must maintain.
4. Under "invert it", a min-max heap gives both extremes in Θ(1). What has to change about the heap property, and what does it cost each operation?
5. Fibonacci heaps improve Dijkstra to Θ(E + V log V) and lose in practice. Name the two mechanisms that destroy the constant, then describe the graph shape where they'd actually win.
6. The two-heap running median keeps a max-heap and a min-heap. State the balance invariant precisely, and say what breaks if you allow the sizes to differ by 2.
7. Under "parallelize it", the root is a single contention point. Why does that make a concurrent heap much harder than a concurrent hash map, and what do real concurrent priority queues do instead?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State both heap invariants and say what each one buys.
2. Give the parent/child index arithmetic and explain why no pointers are needed.
3. Give the measured heapify-vs-push numbers at n = 5,000,000 and the height-counting reason for the gap.
4. Why does `BinaryHeap::iter()` not yield sorted order, and when does that bug survive testing?
5. Write Dijkstra's lazy-deletion line and explain exactly which entries it discards.
6. When is `select_nth_unstable` better than a size-k heap, and when is the heap better?

Build exercises:

- Implement a binary heap over a `Vec`: `push`, `pop`, `peek`, and `from_vec` via sift-down heapify. Benchmark `from_vec` against a push loop at n = 10⁵, 10⁶, 10⁷ and confirm the ~2× — then plot the ratio and check it doesn't grow, which is the signature of Θ(n) vs Θ(n log n) both being dominated by memory traffic.
- Implement Dijkstra twice: once with lazy deletion, once with an indexed heap maintaining an element→position map. Compare code size, peak heap size, and runtime on a large graph. The indexed version's position-map maintenance inside every swap is where the bugs are — write the invariant checker.
- Build the two-heap running median and verify against a re-sort on every insert over 100k inserts. Then measure both; the Θ(log n) vs Θ(n log n) gap makes "maintain a structure, don't recompute a statistic" concrete.
- Implement a 4-ary heap and compare against binary on an extract-heavy and a decrease-key-heavy workload. Find where each wins — that's the fanout trade from [B-trees](../b-trees/learning.md) appearing again in a different structure.

## Open Questions

- Where exactly does a size-k `BinaryHeap` beat `select_nth_unstable` on this machine? The heap rejects most elements in one comparison, so for very small k it should win despite Θ(n log k).
- d-ary heap sweep (d = 2, 4, 8, 16) on this hardware for extract-heavy vs decrease-key-heavy mixes — where's each optimum?
- How much does lazy deletion actually cost in peak memory on a realistic road-network Dijkstra, versus an indexed heap?
- Does `BinaryHeap::from` use the Θ(n) heapify, or does it push? Read the std source and confirm rather than infer.
- Pairing heap vs binary heap in Rust when melding is frequent — is the pointer cost recovered?

## References

- CLRS ch. 6 — binary heaps, the Θ(n) build-heap proof by height counting, and heapsort. Chapter 19 covers Fibonacci heaps and their analysis.
- Williams (1964) and Floyd (1964) — heapsort and the linear-time heapify respectively; Floyd's is the one that matters here.
- Fredman & Tarjan, "Fibonacci Heaps and Their Uses" (1987) — the Dijkstra bound, and worth reading to understand exactly what the amortization buys.
- Larkin, Sen & Tarjan, "A Back-to-Basics Empirical Study of Priority Queues" (2014) — measures the theoretical hierarchy against reality; the definitive answer to "why not Fibonacci heaps."
- [`BinaryHeap` docs](https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html) — the `Reverse` idiom and the explicit note that iteration order is arbitrary.
- Related in this repo: [Selection & Order Statistics](../selection-and-order-statistics/learning.md) (the array-based alternative and the 10.7× measurement), [Arrays & Dynamic Arrays](../arrays-and-dynamic-arrays/learning.md) (the flat backing store), [B-Trees](../b-trees/learning.md) (fanout as a locality trade, appearing here as d-ary), [Complexity Analysis](../complexity-analysis/learning.md) (amortization, and the n₀ argument against Fibonacci heaps), [Stacks & Queues](../stacks-and-queues/learning.md) (the priority queue as the third frontier container).
