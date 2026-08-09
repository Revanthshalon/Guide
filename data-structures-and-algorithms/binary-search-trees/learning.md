# Binary Search Trees & Balancing — Learning Notes

## Mental Model

**A BST is binary search made incremental.** Sorting an array gives you Θ(log n) lookup but Θ(n) insertion; a BST keeps the same ordering property in a linked structure so that *both* are Θ(log n). That's the trade: you give up contiguity to buy cheap mutation while staying ordered.

The catch is that the Θ(log n) is not a property of the structure — it's a property of the *shape*, and the shape depends on the insertion order. Insert sorted data into a plain BST and every node becomes a right child: you have built a linked list with extra pointers. Measured on this machine, inserting and then looking up 100,000 keys:

| Insertion order | Time | Max depth |
| --- | --- | --- |
| Sorted | **29,689 ms** | **99,999** |
| Shuffled | 19.2 ms | 38 |
| `BTreeMap`, sorted input | 9.2 ms | — |

**1,546× slower**, and the tree is 99,999 deep instead of 38. Sorted input is not an adversarial edge case — it is the single most common way data arrives. So:

> An unbalanced BST is not a data structure you can ship. Balancing is not an optimization on top of a BST; it is the thing that makes a BST work at all.

The second thing to internalize is that **in Rust you will almost never write one**. `BTreeMap` is std's ordered map and beats a binary tree on real hardware for reasons covered in [B-trees](../b-trees/learning.md). The value of studying BSTs is that they teach the rotation/invariant machinery that every balanced structure reuses, and that *augmentation* — storing extra data per node — turns them into structures nothing else can replace (order-statistic trees, interval trees).

## The Invariant

> For every node, all keys in its left subtree are **less** than its key, and all keys in its right subtree are **greater**. (Duplicates need an explicit policy: forbid, or send consistently one way, or store a count.)

That's the *search* invariant, and it's all a plain BST guarantees. It says nothing about shape, which is exactly the hole the measurement above falls into. Balanced trees add a second, structural invariant:

| Structure | Balance invariant | Height bound |
| --- | --- | --- |
| **AVL** | Heights of a node's subtrees differ by ≤ 1 | ≤ 1.44 log₂ n — the tightest |
| **Red-black** | Every root→leaf path has the same number of black nodes; no two consecutive reds | ≤ 2 log₂ n |
| **Treap** | Heap order on a *random* priority per node | ≤ ~3 log₂ n **expected** |
| **Splay** | None — recently accessed nodes move to the root | Θ(log n) **amortized** only |
| **Scapegoat** | Weight-balanced; rebuild a subtree when it degrades | ≤ log_{1/α} n amortized |

Every one of these is "the search invariant, plus something that bounds the height." The differences are what they optimize: AVL keeps trees shortest (best for read-heavy), red-black rebalances less per write (best for write-heavy), treaps get it from randomness with almost no code, splay trees get *adaptivity* (frequently accessed keys become cheap) at the price of no worst-case guarantee per operation.

## Mechanics

### Rotations — the one primitive

Every balanced BST rebalances with rotations, and a rotation is the only operation that changes shape while preserving the search invariant:

```
    y                             x
   / \      right rotate(y)      / \
  x   C     ───────────────>    A   y
 / \        <───────────────       / \
A   B        left rotate(x)       B   C
```

`A < x < B < y < C` before and after — the in-order sequence is unchanged, which is why the search invariant survives. Rotations are Θ(1) (three pointer updates), so rebalancing costs Θ(log n) per operation at worst: you rotate along the path back to the root.

AVL: after an insert, walk up; if a node's balance factor hits ±2, one or two rotations fix it. Red-black: recolour where possible (cheap) and rotate only when recolouring can't resolve it — which is why it does fewer rotations per write than AVL and is the classic choice for write-heavy maps.

### Deletion is the hard part

Insertion adds a leaf. Deletion has three cases, and the third is where implementations go wrong:

1. **Leaf** — remove it.
2. **One child** — splice the child into its place.
3. **Two children** — replace the key with its **in-order successor** (leftmost node of the right subtree), then delete that successor, which by construction has at most one child.

Then rebalance up the path. In practice, deletion is 2–3× the code of insertion in every balanced-tree implementation, and it is where the bugs live. This is a large part of why treaps and skip lists are attractive: their deletion is much simpler.

### Augmentation — where BSTs become irreplaceable

Store extra data per node, maintained through rotations, and you get operations no hash map or flat array can do:

| Store per node | Buys | Name |
| --- | --- | --- |
| Subtree size | `select(k)` — the k-th smallest; `rank(x)` — how many are less than x | **Order-statistic tree** |
| Max endpoint in subtree | "Which stored intervals overlap [a,b]?" in Θ(log n + matches) | **Interval tree** |
| Subtree aggregate (sum/min/max) | Range aggregates over a *changing* ordered set | Augmented BST |

The rule is that an augmentation is maintainable iff a node's value can be recomputed from its children in Θ(1) — then a rotation only needs to fix the two nodes it touched. Order-statistic trees are the answer to "the k-th largest, over a set that keeps changing", which [selection](../selection-and-order-statistics/learning.md) can only answer for a static array.

### Why Rust pushes you to arenas here

A BST with parent pointers is exactly the shape the borrow checker forbids — see [Rust for data structures](../rust-for-data-structures/learning.md). A `Box`-based tree works if links are strictly downward, but rebalancing wants parent access, and deletion wants to move subtrees around. The practical representations:

- **`Box` + no parent pointers**, rebalancing on the recursive way back up. Workable for AVL, awkward for red-black.
- **Arena + `u32` indices** — the default. Parent pointers, rotations, and subtree moves are all just integer assignments.
- Note the `Box` version's other trap: a degenerate tree of depth 100,000 is also a 100,000-deep recursive drop, which aborts. The measurement above needed an iterative `Drop` to even run.

## Complexity

| Operation | Balanced | Unbalanced (worst) | Space |
| --- | --- | --- | --- |
| Search | Θ(log n) | **Θ(n)** | — |
| Insert | Θ(log n) | Θ(n) | — |
| Delete | Θ(log n) | Θ(n) | — |
| Min / max | Θ(log n) | Θ(n) | — |
| Predecessor / successor | Θ(log n) | Θ(n) | — |
| In-order traversal | Θ(n) | Θ(n) | Θ(log n) stack |
| Range query [a,b] | Θ(log n + k) | Θ(n) | — |
| `select(k)` / `rank(x)` | Θ(log n) *if augmented* | — | — |
| Structure | — | — | Θ(n), 2–3 pointers/node |

**Where the table misleads.** Θ(log n) counts *comparisons*; each one is a pointer dereference to a node that is almost certainly not in cache. At n = 10⁶ that's ~20 dependent cache misses, ~2 µs — which is why `BTreeMap`, with the same Θ(log n), is several times faster in practice. The measured numbers above make this concrete: the shuffled BST (depth 38) took 19.2 ms where `BTreeMap` took 9.2 ms on *harder* input. Asymptotically identical, 2× apart, and that gap widens with n.

## Rust Implementation

```rust
// In practice: use BTreeMap. It is the balanced ordered map, and it's faster.
use std::collections::BTreeMap;
let mut m = BTreeMap::new();
m.insert(key, value);
m.range(lo..hi);                        // Θ(log n + k) — the operation HashMap can't do
m.first_key_value();                    // ordered min
let (&k, &v) = m.range(..=target).next_back().unwrap();   // predecessor query
```

When you genuinely need a BST — an augmentation std doesn't provide — the arena shape:

```rust
struct Node { key: K, left: Option<u32>, right: Option<u32>, size: u32 }  // `size` = augmentation
struct Tree { nodes: Vec<Node>, root: Option<u32> }

impl Tree {
    fn rotate_right(&mut self, y: u32) -> u32 {
        let x = self.nodes[y as usize].left.expect("rotate_right needs a left child");
        self.nodes[y as usize].left  = self.nodes[x as usize].right;
        self.nodes[x as usize].right = Some(y);
        self.fix_size(y);                 // children first...
        self.fix_size(x);                 // ...then the new parent
        x                                 // caller relinks this as the new subtree root
    }
    fn fix_size(&mut self, i: u32) {
        let n = &self.nodes[i as usize];
        let s = 1 + n.left.map_or(0, |c| self.nodes[c as usize].size)
                  + n.right.map_or(0, |c| self.nodes[c as usize].size);
        self.nodes[i as usize].size = s;
    }
    /// The k-th smallest key — Θ(log n), impossible without the `size` augmentation.
    fn select(&self, mut i: u32, mut k: u32) -> K { /* descend using subtree sizes */ todo!() }
}
```

The `fix_size(y)` before `fix_size(x)` ordering is the augmentation-maintenance rule: recompute bottom-up, or the parent reads a stale child.

**Crates:** `std::collections::BTreeMap` (use this), `im`/`rpds` (persistent ordered maps), `indexmap` (if you wanted insertion order, not sorted order), `rangemap`, `superslice`. There is deliberately no widely-used red-black-tree crate in Rust — that's a signal.

## Use Cases

- **Ordered iteration and range queries.** The reason to choose an ordered map over a hash map at all. Measured: a width-100 range over 1M entries costs 0.0002 ms with `BTreeMap` versus 2.91 ms for a `HashMap` scan — **~14,500×**, because a hash map has no choice but to look at everything.
- **Predecessor/successor queries** — "the last event before this timestamp", "the next available slot". A hash map cannot answer these at all.
- **Order statistics over a changing set** — leaderboards where you need "what rank is this player" *and* "who is 500th", with continuous updates. Augmented BST; [selection](../selection-and-order-statistics/learning.md) only handles the static case.
- **Interval overlap** — scheduling conflicts, genomic ranges, IP range lookup. Interval trees, i.e. BSTs augmented with subtree max.
- **Sorted output without a sorting pass** — iterate the map.
- **Sweep-line algorithms** — the "active set" ordered by a coordinate, with insertion and deletion as the sweep proceeds.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`BTreeMap`/`BTreeSet`** | Any ordered map or set. The default; do not hand-roll a BST for this |
| `HashMap` | No ordering, no ranges — it's several times faster for pure lookup |
| Sorted `Vec` + `binary_search` | Static or rarely-mutated data; better locality, smaller footprint |
| **Augmented BST (hand-rolled/arena)** | You need `select`/`rank` or interval overlap on a *changing* set |
| Treap or skip list | You need a balanced structure with much simpler code, especially deletion |
| Splay tree | Access pattern is heavily skewed and amortized bounds are acceptable |
| **Plain unbalanced BST** | Never in production. Teaching only |

## Pitfalls in Depth

### Pitfall: Degenerate trees from ordered input

- **What goes wrong:** A plain BST is fed data that arrives sorted — timestamps, auto-increment IDs, an alphabetized import, or the output of a previous sort. Every insert goes down the right spine. The structure becomes a linked list, and every operation becomes Θ(n). Measured at n = 100,000: **29,689 ms and depth 99,999**, versus 19.2 ms and depth 38 for the same keys shuffled — a **1,546×** difference from input order alone.
- **Why it happens (the mechanism):** The search invariant constrains *ordering*, not *shape*. Nothing in a plain BST reacts to becoming lopsided. And sorted input is the most common shape real data has, so the worst case isn't rare — it's the default. Tests with random fixtures never see it.
- **How to handle it in production, and why that works:** Use a structure with a height invariant — `BTreeMap` in Rust, or AVL/red-black/treap if you're implementing. A treap is the cheapest to write: assign each node a random priority and maintain heap order on it by rotation, which makes the expected height Θ(log n) *regardless of insertion order*, with no balance-factor bookkeeping. That's the [complexity analysis](../complexity-analysis/learning.md) average-vs-expected distinction applied structurally.
- **Trade-offs of the fix:** Balancing costs rotations on every write and one to two extra fields per node. Treaps trade a worst-case guarantee for an expected one and need a decent RNG. If your data is genuinely static, none of this applies — sort it once into a `Vec` and binary search, which beats every tree.

### Pitfall: Reaching for a BST when a hash map or sorted array would do

- **What goes wrong:** An ordered map is chosen out of habit for a workload that never iterates in order and never does a range query. It pays Θ(log n) pointer-chasing lookups where a hash map would do Θ(1), and gives up nothing in return. Or a tree is used for data that is written once and read many times, where a sorted `Vec` would be smaller, faster, and simpler.
- **Why it happens (the mechanism):** "Tree" reads as the general-purpose structure and "hash map" as the specialized one, when the reverse is true for lookup. And a tree's per-node allocation and pointer chase are invisible in the asymptotics — Θ(log n) and Θ(1) don't communicate that one is ~20 cache misses and the other is ~1.
- **How to handle it in production, and why that works:** Choose by the *operations you actually perform*. Ordered iteration, range, predecessor/successor, or `select`/`rank` → ordered structure. Otherwise → `HashMap`. Rarely mutated → sorted `Vec`, which measured *faster than `BTreeMap`* on range scans (0.0001 ms vs 0.0002 ms at width 100; 0.0034 vs 0.0114 at width 10,000) because the results are contiguous.
- **Trade-offs of the fix:** A sorted `Vec` has Θ(n) insertion, so it's only right when writes are rare or batched. Switching to `HashMap` forfeits deterministic iteration order, which some code depends on without saying so.

### Pitfall: Getting deletion wrong

- **What goes wrong:** The two-children case is mishandled — the wrong replacement is chosen, or the successor's own child is dropped, or rebalancing is skipped on the deletion path. The tree still *looks* fine (traversal produces sorted output) while having silently lost nodes or grown unbalanced. It surfaces later as a missing key or as gradual performance decay.
- **Why it happens (the mechanism):** Deletion has genuinely more cases than insertion (three structural cases × the rebalancing cases), and the two-children case is the only one that moves a key rather than a node. Several of these paths are never exercised by tests that only delete leaves.
- **How to handle it in production, and why that works:** Use `BTreeMap`. If you must implement: write a `#[cfg(test)] fn check_invariants()` that verifies in-order sortedness, the height/colour invariant, the node count, *and* any augmented values — then drive it with `proptest` over random insert/delete sequences, asserting after every operation. That turns a class of silent corruption into a minimal failing case, which is the same discipline as the doubly-linked list in [linked lists](../linked-lists/learning.md).
- **Trade-offs of the fix:** The invariant checker is Θ(n) per call, so it stays behind `cfg(test)`. Property-testing a balanced tree is genuinely slow to write. Both are small next to debugging a corrupted index in production.

### Pitfall: Augmentation that isn't maintained through rotations

- **What goes wrong:** Subtree sizes (or max-endpoints, or sums) are updated on insert and delete but not inside `rotate_left`/`rotate_right`. `select(k)` returns the wrong element, `rank(x)` is off by a few, and the error is proportional to how much rebalancing happened — so it's small, data-dependent, and looks like an off-by-one bug anywhere else in the code.
- **Why it happens (the mechanism):** A rotation changes the parent/child relationship of two nodes, so both of their subtree aggregates change even though no key was inserted or removed. It's easy to think of rotation as "just pointer shuffling" — structurally it is, but any value derived from subtree contents is invalidated for exactly the two nodes involved.
- **How to handle it in production, and why that works:** Make recomputation part of the rotation itself, bottom-up: fix the demoted node first, then the promoted one (as in the snippet above). Encapsulate rotations so no code path can move pointers without going through them. Then assert in the invariant checker that every node's stored aggregate equals the recomputed value.
- **Trade-offs of the fix:** Recomputing on every rotation adds constant work to each rebalance. Only augmentations computable in Θ(1) from the children can be maintained this way at all — anything requiring a subtree scan is not augmentable and needs a different structure.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if insert returned a new tree sharing structure? | Path copying — Θ(log n) new nodes per version; the persistent ordered map (`im`, Clojure's sorted maps) |
| Batch it | What if you inserted a sorted run at once? | Bulk-loading a perfectly balanced tree in Θ(n); join/split-based bulk merge |
| Approximate it | What if the height bound were probabilistic? | **Skip list** — same bounds, far simpler code, and concurrency-friendly |
| Randomize it | What if balance came from a coin flip? | **Treap** — expected Θ(log n) with no balance factors and no case analysis |
| Externalize it | What if a node were a disk page? | **B-tree** — raise the fanout so each transfer carries ~100 keys |
| Parallelize it | Where's the contention? | Lock-free skip lists; RCU trees; per-subtree locks — trees are hard to lock, which is why skip lists dominate concurrent ordered maps |
| Invert it | What if recently-used nodes moved to the root? | **Splay tree** — adaptive, amortized, no per-operation guarantee |
| Augment it | What does one extra field per node buy? | Size → order statistics; max-endpoint → interval tree; sum → range aggregates |
| Specialize it | What if keys were fixed-width integers? | **van Emde Boas** / y-fast tries — Θ(log log U) predecessor, beating the comparison bound |
| Amortize it | What if one operation could be terrible? | **Scapegoat tree** — no per-node balance data; rebuild a whole subtree when it degrades |

**Questions:**

1. Sorted insertion made the tree 99,999 deep. A treap fixes this with a random priority per node and no balance factors. Explain precisely why randomness defeats *adversarial* input here, using the average-vs-expected distinction.
2. Under "externalize it" you get a B-tree. Given that both are Θ(log n), why does `BTreeMap` beat a red-black tree on real hardware — and what would have to change about hardware to reverse that?
3. Rotations preserve the in-order sequence. Prove it for a single right rotation using the `A < x < B < y < C` labelling, then explain why that proof is the entire justification for balancing being safe.
4. Order-statistic trees maintain subtree sizes; interval trees maintain subtree max-endpoints. State the general rule for which augmentations are maintainable, and give an example of one that *isn't*.
5. Skip lists match balanced BSTs asymptotically with much simpler code and dominate in concurrent settings. What does a BST still have that a skip list doesn't?
6. Splay trees have no per-operation guarantee but are amortized Θ(log n) and adapt to access patterns. Name a workload where that's strictly better than AVL, and one where the lack of a worst-case bound disqualifies it.
7. Under "specialize it", van Emde Boas achieves Θ(log log U) predecessor queries. Which assumption of the Ω(log n) comparison bound does it violate, and what does it cost in space?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the BST search invariant, then explain why it alone permits a 99,999-deep tree.
2. Give the measured sorted-vs-shuffled numbers at n = 100,000 and the depths. Which real-world data sources produce the bad case?
3. Draw a right rotation with the `A, x, B, y, C` labelling and show the in-order sequence is unchanged.
4. Give the three deletion cases and say which one moves a *key* rather than a *node*.
5. Name the augmentation for: k-th smallest; how many keys are < x; which intervals overlap [a,b].
6. When is `BTreeMap` the wrong choice, and what replaces it in each case?

Build exercises:

- Implement a plain `Box`-based BST and reproduce the degeneration: insert 100,000 sorted keys, record max depth and time, then repeat shuffled. You'll need an iterative `Drop` for the sorted case or it aborts on a 100,000-deep recursive drop — that failure is itself the lesson from [Rust for data structures](../rust-for-data-structures/learning.md).
- Convert it to a **treap** by adding a random priority and rotating to maintain heap order. Re-run the sorted-input test and watch the depth collapse to ~2 log₂ n. This is the highest ratio of insight to code in the whole topic — about 20 lines buys a 1,500× improvement.
- Augment your treap with subtree sizes and implement `select(k)` and `rank(x)`. Property-test both against a sorted `Vec` reference over random insert/delete/query sequences. Then deliberately remove the size-fixup from one rotation and watch the property test localize it.
- Implement an interval tree (augment with subtree max endpoint) and use it to detect overlapping calendar events. Compare against the Θ(n) scan at 10, 1,000, and 100,000 intervals.

## Open Questions

- At what n does an arena-based AVL/treap start losing to `BTreeMap` on this machine, and is there any n where it wins?
- Treap vs skip list in Rust for a single-threaded ordered map — is the skip list's simplicity free, or does the extra pointer level cost measurably?
- How much does the `Box`-per-node allocation hurt versus the arena version for the same tree? Isolate allocation from pointer-chasing.
- Is there a practical Rust crate for order-statistic trees, or is hand-rolling still the norm?
- Splay trees on a realistically skewed (Zipfian) access pattern — does the adaptivity actually beat `BTreeMap`, or does the constant factor eat it?

## References

- CLRS ch. 12–14 — BSTs, red-black trees, and augmentation (order-statistic and interval trees). Chapter 14's "how to augment a data structure" methodology is the transferable part.
- Sedgewick, "Left-Leaning Red-Black Trees" — a red-black variant designed to be implementable without a page of case analysis; worth reading for how much of the complexity was accidental.
- Seidel & Aragon, "Randomized Search Trees" (1996) — the treap; the clearest demonstration that randomness can replace balance logic entirely.
- Pugh, "Skip Lists: A Probabilistic Alternative to Balanced Trees" (1990) — the argument that the simpler structure is the better engineering choice.
- Sleator & Tarjan, "Self-Adjusting Binary Search Trees" (1985) — splay trees and the amortized analysis that justifies them.
- Related in this repo: [B-Trees](../b-trees/learning.md) (what you should actually use, and why), [Complexity Analysis](../complexity-analysis/learning.md) (average vs expected — the treap's whole argument), [Rust for Data Structures](../rust-for-data-structures/learning.md) (arena representation; the recursive-drop abort), [Selection & Order Statistics](../selection-and-order-statistics/learning.md) (the static counterpart to order-statistic trees), [Cache Locality](../../performance-optimization/cache-locality/learning.md) (why Θ(log n) pointer chasing loses).
