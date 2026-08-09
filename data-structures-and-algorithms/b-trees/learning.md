# B-Trees — Learning Notes

## Mental Model

**A B-tree is a search tree redesigned around the fact that memory moves in blocks, not in bytes.**

A binary tree asks one question per node and follows one pointer — which fetches a 64-byte cache line (or a 4 KB page) to read a single key and a single pointer, discarding the rest. A B-tree puts *many* keys in each node, so one fetched block answers many comparisons at once. Same Θ(log n), completely different number of memory transfers:

| Structure | n = 10⁹ | Transfers |
| --- | --- | --- |
| Binary search tree | log₂(10⁹) | ~30 dependent misses ≈ 3 µs |
| B-tree, fanout 100 | log₁₀₀(10⁹) | ~4.5 block reads ≈ 0.5 µs |

The base of the logarithm becomes the fanout, and **the fanout is chosen to fill a block**. That's the entire idea, and it is why:

- Every database index on disk is a B+ tree, where the block is a 4–16 KB page and a random read costs ~100 µs on spinning disks or ~100 µs of syscall+SSD latency — so reducing 30 transfers to 4 is the difference between usable and not.
- **Rust's `BTreeMap` is std's ordered map and there is no red-black tree in std.** The same argument applies one level down: the block is a cache line, and the "disk" is DRAM.

The generalizable lesson, and the reason this topic sits where it does in the curriculum: **when two structures have the same asymptotic complexity, the one that matches the memory hierarchy wins.** This is the [complexity analysis](../complexity-analysis/learning.md) I/O model made concrete, and it recurs in Stage 9.

## The Invariant

For a B-tree of minimum degree *t* (order 2t):

> Every node holds between **t−1 and 2t−1** keys (the root may hold fewer), and an internal node with *k* keys has exactly *k+1* children. Keys within a node are **sorted**, and all keys in child *i* lie between key *i−1* and key *i*. **All leaves are at the same depth.**

Three consequences:

- **Perfect balance is structural, not maintained by rotations.** A B-tree grows at the *root*, not at the leaves: when a node overflows it splits, pushing its median key up; when the root splits, the tree gets one level taller everywhere at once. That's why all leaves stay at equal depth without any rebalancing machinery — and it's a genuinely different mechanism from the [BST](../binary-search-trees/learning.md) rotation approach.
- **The minimum-occupancy rule (t−1) is what bounds the height.** Without it, a tree of mostly-empty nodes would be as tall as a binary tree. Deletion must therefore *merge* or *borrow* to restore it, which is why B-tree deletion is the fiddly part.
- **Fanout is a tuning parameter, not a constant.** It's chosen so a node fills one block of whatever level you're optimizing for.

## Mechanics

### Insertion — split on the way down or up

1. Descend to the leaf that should hold the key, binary-searching (or linearly scanning) within each node.
2. Insert into the leaf's sorted key array.
3. If the leaf now has 2t keys, **split**: the median key moves up into the parent, and the node becomes two nodes of t−1 keys each.
4. If the parent overflows, repeat. If the *root* splits, allocate a new root — this is the only way the tree gets taller.

Because a split pushes exactly one key up, the parent grows by one — so a B-tree never needs to rebalance sideways.

### Deletion — merge or borrow

Deleting from a leaf may drop it below t−1 keys ("underflow"). Then either **borrow** a key from an adjacent sibling (via the parent), or **merge** with a sibling and pull the separating key down from the parent — which may cascade upward and can shrink the tree's height. Deleting from an internal node first replaces the key with its predecessor/successor from a leaf, as in a BST.

### B-tree vs B+ tree

| | B-tree | **B+ tree** |
| --- | --- | --- |
| Values stored | In every node | **Leaves only** |
| Internal nodes | Keys + values + children | Keys + children only → **higher fanout** |
| Leaves linked? | No | **Yes, in a linked list** |
| Range scan | Tree traversal | **Walk the leaf list — sequential** |

Essentially every database index is a **B+ tree**, for two reasons: internal nodes hold no values so they pack more keys per block (higher fanout, shallower tree, and the upper levels fit in cache); and linked leaves turn a range scan into a sequential walk rather than a tree traversal. Rust's `BTreeMap` is a B-tree (values in all nodes), which is the right choice in memory where there's no page-read asymmetry to exploit.

### Fanout: the actual calculation

Fanout is chosen so one node ≈ one block:

- **On disk**, a 4 KB page with 16-byte keys and 8-byte child pointers holds roughly 170 keys → `log₁₇₀(10⁹) ≈ 4` levels. With the top two levels cached, a lookup costs about **two disk reads**.
- **In memory**, `BTreeMap` uses **B = 6 (up to 11 keys per node)**. That's much smaller than a cache line would suggest, and the reason is the trade the disk case doesn't face: a larger node means fewer cache misses on the descent but a longer *in-node* search, plus more data movement on insert and delete (shifting a sorted array). std settled on 11 by measurement — a good demonstration that "fill the cache line" is a starting hypothesis, not the answer.

### Why this beats a binary tree in RAM too

Measured on this machine, 100,000 sorted keys inserted then looked up: the unbalanced BST took 29,689 ms (degenerate), a *shuffled* BST took 19.2 ms, and `BTreeMap` took **9.2 ms on the harder sorted input**. Even against a well-shaped binary tree, the B-tree is ~2× faster at the same asymptotics — and the gap grows with n, because the binary tree's ~log₂ n dependent misses grow faster than the B-tree's ~log₁₁ n.

## Complexity

| Operation | Cost | I/O model (block transfers) |
| --- | --- | --- |
| Search | Θ(log_B n) comparisons ≈ Θ(log n) | **Θ(log_B n)** |
| Insert | Θ(log_B n) amortized | Θ(log_B n) |
| Delete | Θ(log_B n) amortized | Θ(log_B n) |
| Range query [a,b] | Θ(log_B n + k) | Θ(log_B n + k/B) |
| Min / max / pred / succ | Θ(log_B n) | Θ(log_B n) |
| In-order traversal | Θ(n) | **Θ(n/B)** — sequential |
| Space | Θ(n) | ≥ 50% node occupancy guaranteed |

**Where the table misleads:** the comparison count is *higher* than a binary tree's (you search within nodes too — roughly log₂ n comparisons either way), but the **transfer count** is log_B n. B-trees trade more comparisons for fewer memory transfers, which is the correct trade on every machine built in the last thirty years. Counting comparisons makes B-trees look pointless; counting transfers explains why they're everywhere.

The Θ(n/B) traversal row is the underrated one: a full scan of a B+ tree moves through leaves sequentially, so it's prefetcher-friendly in memory and sequential-I/O-friendly on disk. That's why `SELECT ... ORDER BY indexed_column` is cheap.

## Rust Implementation

You will use `BTreeMap`, not write one:

```rust
use std::collections::BTreeMap;

let mut m: BTreeMap<Key, Value> = BTreeMap::new();

// The operations that justify choosing it over HashMap:
for (k, v) in m.range(lo..hi) { }                  // Θ(log n + k)
for (k, v) in m.range(lo..) .take(10) { }          // "next 10 after lo"
m.range(..=target).next_back();                    // predecessor
m.range(target..).next();                          // successor
m.first_key_value();  m.last_key_value();          // ordered min/max
let upper = m.split_off(&pivot);                   // split into two maps

// Entry API works here too.
*m.entry(k).or_insert(0) += 1;

// Iteration is sorted — no sorting pass needed, and it's deterministic
// (unlike HashMap, whose order differs per instance).
```

**Range syntax is the whole point.** `m.range(a..b)` is the operation a `HashMap` fundamentally cannot perform without scanning everything. Measured over 1M entries:

| Range width | `BTreeMap::range` | `HashMap` filter-scan | Sorted `Vec` |
| --- | --- | --- | --- |
| 100 | **0.0002 ms** | 2.9145 ms | 0.0001 ms |
| 10,000 | 0.0114 ms | 3.0791 ms | 0.0034 ms |

**~14,500× at width 100.** Note also that the sorted `Vec` beats `BTreeMap` — 3.4× at width 10,000 — because the results are contiguous. That's consistent with the framing in [arrays](../arrays-and-dynamic-arrays/learning.md): a sorted `Vec` is the right choice for static ordered data, and `BTreeMap` earns its place when the data *changes*.

**Requirements:** keys need `Ord`, and it must be a genuine total order. A non-total `Ord` makes `BTreeMap` lose entries silently — insert then `get` returns `None` — which is the trap from [Rust for data structures](../rust-for-data-structures/learning.md). Floats need `total_cmp` or `ordered_float::NotNan`.

**Crates:** `std::collections::BTreeMap` (in-memory), `sled`/`redb` (embedded on-disk B-tree stores), `rocksdb` (LSM — the write-optimized alternative, Stage 9).

## Use Cases

- **Every relational database index.** B+ trees, tuned to the page size. Primary keys, secondary indexes, and the reason `WHERE created_at BETWEEN ...` is fast.
- **Filesystems.** ext4's HTree, Btrfs (it's in the name), NTFS, APFS, XFS — directory indexes and extent maps.
- **Key-value stores** where reads dominate: LMDB, BoltDB, `sled`, `redb`. The alternative is an LSM tree, which flips the trade toward writes.
- **In-memory ordered maps** — `BTreeMap` in Rust, `std::map` in C++ (a red-black tree, and a decision that predates the cache-hierarchy argument).
- **Range and time-series queries** — "all events between two timestamps" is a leaf-list walk.
- **Anywhere you need sorted order plus mutation.** If it's static, a sorted array beats it.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`BTreeMap`/`BTreeSet`** | Ordered map/set in memory that changes |
| `HashMap` | No ordering, no ranges — several times faster for pure lookup |
| Sorted `Vec` + `binary_search` | Static or batch-built; faster ranges, smaller footprint |
| **B+ tree** (on disk) | Read-heavy persistent index; range scans matter |
| **LSM tree** | Write-heavy persistent store; you can pay read amplification (Stage 9) |
| Binary balanced BST | Essentially never — B-trees dominate on real hardware |
| Trie / radix tree | Keys are strings **and** you need prefix or longest-prefix-match |

## Pitfalls in Depth

### Pitfall: Assuming a shallower tree is automatically faster

- **What goes wrong:** Someone reasons "higher fanout = fewer levels = faster" and builds a B-tree with 1,000-key nodes in memory. It gets slower. The descent does touch fewer nodes, but each node now requires searching a 1,000-element sorted array, and every insert or delete shifts up to 1,000 elements to keep it sorted.
- **Why it happens (the mechanism):** Fanout trades *between* two costs: the number of block transfers (falls as log_B n) and the work *inside* each node (rises linearly for insert/delete, logarithmically for search). The optimum is where they balance, and it depends entirely on the transfer cost. On disk, a transfer is ~10⁵ ns and the in-node work is free by comparison, so fanout should be large. In DRAM, a miss is ~100 ns and shifting 1,000 elements is not free — so the optimum collapses. std's `BTreeMap` settled on **11 keys per node**, far smaller than "fill the cache line" would predict.
- **How to handle it in production, and why that works:** Derive fanout from the *actual* transfer cost of the level you're optimizing (page size on disk, cache line in memory) and then **measure around that estimate**, because the in-node cost is workload-dependent — a delete-heavy workload wants smaller nodes than a read-only one. Don't reason your way to a constant.
- **Trade-offs of the fix:** Tuning fanout to one workload de-tunes it for another, and it's baked into the on-disk format for persistent stores, so it's not changeable later without a migration. For in-memory work, `BTreeMap`'s choice is already measured — the pitfall only matters if you're implementing.

### Pitfall: Using `BTreeMap` where `HashMap` belongs (and vice versa)

- **What goes wrong:** `BTreeMap` chosen by default for a workload that only does point lookups, paying Θ(log n) comparisons and a pointer chase per level where a hash map does one probe. Or `HashMap` chosen for something that needs ranges, so "find all events in this window" becomes a full scan — measured at **2.91 ms versus 0.0002 ms**, a ~14,500× tax, and one that grows with the map rather than with the answer.
- **Why it happens (the mechanism):** The two types look interchangeable — same `insert`/`get`/`remove` surface — so the choice gets made by habit. The distinguishing operations (`range`, `first_key_value`, ordered iteration) aren't in the shared API, so their absence is invisible until someone writes a scan to compensate.
- **How to handle it in production, and why that works:** Choose by the operations you actually perform. Any of ordered iteration, range, predecessor/successor, or "the smallest/largest" → `BTreeMap`. Otherwise → `HashMap`. A `HashMap` plus a manual scan is the specific antipattern to watch for in review; it's a `BTreeMap` that hasn't been recognized yet.
- **Trade-offs of the fix:** `BTreeMap` lookups are genuinely slower than `HashMap`'s, so a workload doing millions of point lookups and one occasional range may legitimately want the hash map plus a separate sorted structure. And `BTreeMap` requires `Ord` where `HashMap` needs `Hash + Eq` — for some key types one is much easier to provide correctly.

### Pitfall: A non-total `Ord`, or mutating a key

- **What goes wrong:** A custom `Ord` that isn't transitive, or `partial_cmp().unwrap()` on floats, or a key mutated through interior mutability after insertion. Entries become unreachable: you insert and `get` returns `None`. The map is not corrupt in a way that panics — the search simply descends the wrong branch. It looks like data loss.
- **Why it happens (the mechanism):** A B-tree's search *is* the ordering: it decides which child to descend into by comparing. If the comparison is inconsistent, or if a key's position in the order changed after it was placed, the descent goes somewhere the key isn't. Nothing detects this, because every individual comparison returns a valid answer. Floats are the standard instance — `NaN` compares false to everything, so the order isn't total.
- **How to handle it in production, and why that works:** Derive `PartialOrd`/`Ord` where possible so they can't drift from `PartialEq`/`Eq`. For floats use `f64::total_cmp` or `ordered_float::NotNan`, which restores a genuine total order at the type level. Never put `Cell`/`RefCell` in a key. For hand-written `Ord`, property-test transitivity and antisymmetry — a few lines of `proptest` catches every instance.
- **Trade-offs of the fix:** `NotNan` pushes validation to construction, which means handling the error at every parse boundary — correct, but real plumbing. `total_cmp` places `NaN` at a defined but arbitrary position, so `NaN`s still appear somewhere in your sorted output; filter them at the edge if they're meaningless.

### Pitfall: Ignoring node occupancy on disk

- **What goes wrong:** A persistent B+ tree index that has seen heavy deletion or random insertion sits at ~50% node occupancy. The index is twice the size it needs to be, so twice as many pages are read for a scan and half the page cache is wasted on empty space. Query plans that assumed an index fits in memory stop being true.
- **Why it happens (the mechanism):** The invariant only guarantees nodes are **at least half full**. A split produces two half-full nodes, and random insertion patterns keep them near that floor — while sequential insertion (a monotonically increasing key) leaves a trail of nodes that are exactly half full after every split, which is the common case for auto-increment primary keys.
- **How to handle it in production, and why that works:** Bulk-load indexes when building from sorted data — constructing bottom-up packs nodes to near 100% and produces a shallower tree, which is why `CREATE INDEX` on an existing table is much better than inserting rows one at a time. For live indexes, databases expose rebuild/`REINDEX`/`VACUUM FULL` to repack; some implementations special-case rightmost-leaf splits for sequential keys to avoid the 50% trail.
- **Trade-offs of the fix:** Rebuilding an index takes a lock or a full rewrite and is expensive on a large table. Packing to 100% also means the *next* insert into any node splits immediately, so a fill factor slightly below 100% (Postgres defaults to 90 for B-trees) trades a little space for fewer subsequent splits — another measured constant rather than a derived one.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if updates never overwrote a page? | **Copy-on-write B-trees** — Btrfs, LMDB; the root swap is an atomic commit, giving crash safety for free |
| Batch it | What if writes accumulated before touching the tree? | **LSM tree** / fractal tree — buffer writes, merge later; flips the read/write trade (Stage 9) |
| Approximate it | What if you could skip blocks you know don't match? | Bloom filters per block; **zone maps**/min-max indexes in columnar stores |
| Randomize it | What if balance came from randomness instead of splitting? | Skip lists — B-tree-ish behaviour, far simpler concurrency |
| Externalize it | What if a node were a network round trip? | Distributed B-trees; the fanout goes way up because the transfer cost did |
| Parallelize it | Where's the contention? | Latch coupling (hand-over-hand), B-link trees — a right-link pointer lets readers proceed during a split without locking the parent |
| Invert it | What if leaves held everything and internal nodes were pure routing? | **B+ tree** — higher fanout, linked leaves, sequential range scans |
| Augment it | What does storing a count per node buy? | Order statistics over a B-tree; `select(k)` in Θ(log_B n) |
| Specialize it | What if keys were fixed-width strings? | **Radix/prefix B-trees**; prefix truncation packs more keys per page |
| Amortize it | What if one insert could be terrible? | Root splits — rare, and they're the only thing that increases height |

**Questions:**

1. B-trees do *more* comparisons than a binary tree but fewer transfers. Write both counts, then state the hardware condition under which the binary tree would win — and say whether any real machine satisfies it.
2. `BTreeMap` uses 11 keys per node, far fewer than a cache line would suggest. Give the two competing costs, and predict which direction the optimum moves for a delete-heavy workload.
3. Under "invert it", the B+ tree moves all values to the leaves. Quantify what that does to fanout for 16-byte keys, 8-byte pointers, and 100-byte values in a 4 KB page.
4. A B-tree grows at the root, not the leaves. Explain why that single fact removes the need for rotations, and contrast it with how an AVL tree maintains balance.
5. Under "persist it", copy-on-write B-trees get crash safety from the root swap. What exactly is atomic, and what does that save you compared to a write-ahead log?
6. Sequential (auto-increment) key insertion leaves nodes ~50% full. Explain the mechanism, then propose a split policy that fixes it and say what it costs for random keys.
7. Under "parallelize it", B-link trees add a right-link so a reader can survive a concurrent split. Describe the race it fixes, and why that's cheaper than locking the parent.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the B-tree invariant including the minimum-occupancy rule, and say what breaks without it.
2. Give the transfer counts for a BST vs a fanout-100 B-tree at n = 10⁹, and the rough time for each.
3. Why does a B-tree need no rotations? Name the mechanism that keeps all leaves at equal depth.
4. List three differences between a B-tree and a B+ tree, and say which one makes range scans cheap.
5. Give the measured `BTreeMap::range` vs `HashMap`-scan numbers at width 100 over 1M entries, and say why the sorted `Vec` beat both.
6. Your `BTreeMap` insert-then-`get` returns `None`. Give three possible causes.

Build exercises:

- Implement an in-memory B-tree with configurable *t*: search, insert with node splitting, and in-order iteration. Then sweep *t* from 2 to 512 on the same workload and plot lookup time. Finding your own optimum — and seeing it land nowhere near "one cache line" — is the exercise.
- Add deletion with borrow and merge, and write a `#[cfg(test)]` invariant checker (occupancy bounds, equal leaf depth, sorted keys, child-count = key-count + 1) driven by `proptest` over random insert/delete sequences. Deletion is where the bugs are, and the checker is what finds them.
- Convert it to a B+ tree (values in leaves, leaves linked) and measure range-scan throughput against your B-tree version. Then compare both to `BTreeMap::range` and a sorted `Vec`.
- Reproduce the occupancy problem: insert 1M sequential keys, measure average node occupancy, then insert 1M random keys and measure again. Then bulk-load from sorted input and compare all three, plus the resulting tree heights.

## Open Questions

- Sweep `BTreeMap`-like fanout on this machine for `u64` keys — where is the optimum, and how far is it from std's 11?
- How much does prefix truncation buy for a B-tree over `String` keys with shared prefixes?
- `sled` vs `redb` vs an LMDB binding for an embedded ordered store in Rust — read/write mix where each wins.
- Does `BTreeMap`'s advantage over a well-shaped arena BST grow with n as predicted (log₂ vs log₁₁ misses)? Measure at 10⁴ through 10⁷.
- At what point does a copy-on-write B-tree's write amplification outweigh its crash-safety benefit versus a WAL?

## References

- Bayer & McCreight, "Organization and Maintenance of Large Ordered Indices" (1972) — the original; the motivation is explicitly about block-device access costs.
- Douglas Comer, "The Ubiquitous B-Tree" (1979) — the survey that named the B+ and B* variants; still the clearest overview.
- Graefe, "Modern B-Tree Techniques" (2011) — everything the original paper doesn't cover: concurrency, latch coupling, prefix truncation, bulk loading.
- Lehman & Yao, "Efficient Locking for Concurrent Operations on B-Trees" (1981) — B-link trees; the right-link idea behind most real concurrent implementations.
- [Rust `BTreeMap` source and its module docs](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html) — read the comment explaining the choice of B; it's a good worked example of measurement beating derivation.
- Related in this repo: [Binary Search Trees](../binary-search-trees/learning.md) (what this replaces and why), [Complexity Analysis](../complexity-analysis/learning.md) (the I/O model), [Arrays & Dynamic Arrays](../arrays-and-dynamic-arrays/learning.md) (the sorted-`Vec` alternative and its measured range performance), [Cache Locality](../../performance-optimization/cache-locality/learning.md) (the mechanism), [Caching Strategies](../../architecture-patterns/caching-strategies/learning.md) (where the page cache sits in a real system).
