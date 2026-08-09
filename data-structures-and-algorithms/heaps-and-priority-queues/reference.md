# Heaps & Priority Queues — Quick Reference

## At a Glance

The structure for "what's next?" and nothing else. Gives up total order for a **partial** order — parent beats children, siblings unordered — buying Θ(log n) insert/extract with the extreme at a known position.

**Invariants:** *heap property* — every node ≤ (or ≥) both children. *Shape* — complete tree, so it lives in a **flat array with no pointers**: `parent=(i-1)/2`, `children=2i+1, 2i+2`.

## The Number

`BinaryHeap::from(vec)` vs n pushes (measured):

| n | `from()` | pushes | Ratio |
| --- | --- | --- | --- |
| 100,000 | 0.98 ms | 2.37 ms | **2.42×** |
| 1,000,000 | 5.26 ms | 10.19 ms | 1.94× |
| 5,000,000 | 17.71 ms | 42.84 ms | **2.42×** |

Heapify is **Θ(n)** — count nodes by *height*: half are leaves doing zero work.

## Complexity

| Operation | Heap | Sorted `Vec` | `BTreeMap` |
| --- | --- | --- | --- |
| Peek extreme | **Θ(1)** | Θ(1) | Θ(log n) |
| Insert | **Θ(log n)** | Θ(n) | Θ(log n) |
| Extract extreme | **Θ(log n)** | Θ(1)/Θ(n) | Θ(log n) |
| Build from n | **Θ(n)** | Θ(n log n) | Θ(n log n) |
| **Search / delete arbitrary** | **Θ(n)** | Θ(log n) | Θ(log n) |
| Ordered iteration | Θ(n log n) | Θ(n) | Θ(n) |
| Space | **Θ(n), no pointers** | Θ(n) | Θ(n) + ptrs |

A heap's Θ(log n) is cheaper than a tree's — flat array, not a pointer chase.

## Variants

| Heap | Insert | Extract | Decrease-key | Meld | Verdict |
| --- | --- | --- | --- | --- | --- |
| **Binary** | log n | log n | log n | Θ(n) | **default** |
| d-ary (4) | log_d n | d log_d n | log_d n | Θ(n) | decrease-key-heavy |
| Pairing | Θ(1) | log n am. | log n am. | **Θ(1)** | best mergeable |
| Fibonacci | Θ(1) | log n am. | **Θ(1) am.** | Θ(1) | **loses in practice** |

## Snippets

```rust
let mut min: BinaryHeap<Reverse<u64>> = BinaryHeap::new();   // BinaryHeap is a MAX-heap
min.push(Reverse(cost));
let heap = BinaryHeap::from(vec);                            // Θ(n) — ~2× vs a push loop
let sorted = heap.into_sorted_vec();

// Streaming top-k: Θ(k) memory
if top.len() < k { top.push(Reverse(x)); }
else if x > top.peek().unwrap().0 { top.pop(); top.push(Reverse(x)); }

// Dijkstra with LAZY DELETION — sidesteps decrease-key entirely
while let Some(Reverse((d, u))) = pq.pop() {
    if d > dist[u] { continue; }                             // stale entry
    for &(v, w) in &adj[u] {
        if d + w < dist[v] { dist[v] = d + w; pq.push(Reverse((d + w, v))); }
    }
}
```

## Choose This When

| Use | For |
| --- | --- |
| **`BinaryHeap`** | Repeated min/max from a **changing** set |
| `BinaryHeap<Reverse<T>>` | …and you want the min |
| Size-k min-heap | Streaming top-k, n unknown, Θ(k) memory |
| `select_nth_unstable` | Top-k from an array you hold — Θ(n), ~10× faster |
| `BTreeMap` | Also need lookup, delete-by-key, or ranges |
| `priority-queue` crate | Genuinely need `decrease_key` |
| d-ary heap | Decrease-key-heavy |
| Fibonacci heap | Essentially never |

## Rules of Thumb

- `from(vec)`, never a push loop.
- **A heap is not sorted** — only the root is guaranteed. `iter()` order is arbitrary.
- Need decrease-key? Use **lazy deletion**, not a rebuild.
- Always add a tiebreaker to `Ord` so pop order is deterministic.
- Floats: `ordered_float::NotNan` or integer-scale. Never `partial_cmp().unwrap()`.
- A heap answers *one* question. Anything needing "is x present?" needs another structure.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Push loop instead of `from` | ~2× slower |
| Assumed iteration is sorted | Passes on small heaps, fails on real data |
| Rebuild heap for decrease-key | Θ(n) per update → quadratic Dijkstra |
| `partial_cmp().unwrap()` on floats | Panic on `NaN` |
| Inconsistent `Ord` | `pop` silently returns a non-minimum |
| No tiebreaker | Pop order varies between runs; bugs irreproducible |

## Key References

- CLRS ch. 6 — heaps and the Θ(n) build-heap proof
- Larkin, Sen & Tarjan (2014) — empirical priority-queue study; why not Fibonacci heaps
- [`BinaryHeap` docs](https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html)
