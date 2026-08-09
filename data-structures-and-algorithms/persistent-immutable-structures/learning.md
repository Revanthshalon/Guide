# Persistent & Immutable Structures — Learning Notes

## Mental Model

**A persistent structure gives you a new version on every update while keeping every old version valid and queryable — and it does so without copying everything.**

The mechanism is **structural sharing**: an update copies only the nodes on the path from the root to the change, and every untouched subtree is shared by reference between the old and new versions. For a balanced tree of depth log n, that's Θ(log n) new nodes per update rather than Θ(n).

```
        root_v1                root_v2          ← two roots, most nodes shared
       /       \              /       \
      A         B            A'        B        ← only A's path was copied
     / \       / \          / \
    C   D     E   F        C   D'                ← D' is new; C, E, F shared
```

The naive alternative — clone the whole structure per version — is Θ(n) time and space per update. Structural sharing makes versioning cost proportional to the *change*, not to the data.

Two distinct motivations, and they're worth separating because they lead to different structures:

1. **Versioning as a feature** — undo/redo, time-travel queries, MVCC in databases, git-style history. You *want* old versions.
2. **Immutability as a discipline** — safe concurrent sharing without locks, referential transparency, cheap "snapshot" semantics. You don't care about old versions but you want the guarantee that nothing changes under you.

The second is why persistent structures matter in Rust specifically: **an immutable structure is `Sync` for free.** No locks, no `&mut` contention, no aliasing question — readers hold an `Arc` to a version and it can never change. That's a real answer to a real problem, and it's the same "copy-on-write B-tree" idea that gives LMDB its crash safety ([LSM trees](../lsm-trees/learning.md)).

The price is a **constant factor**, not an asymptotic one. Every access is a pointer chase through a tree instead of an array index, and every update allocates. Measured elsewhere in this category, that gap is consistently large: [linked lists](../linked-lists/learning.md) showed 641× for scattered pointer chasing versus a flat scan. Persistent structures don't pay *that* much — their trees are shallow and allocation-dense — but they are firmly on the pointer-chasing side of the divide.

## The Invariant

> **Nodes are never mutated after they become reachable.** An update produces new nodes along the modified path and reuses every other node by reference. Every previously-returned root remains a valid, complete structure.

Three levels of persistence, worth distinguishing:

| Level | Can read old versions | Can *update* old versions | Example |
| --- | --- | --- | --- |
| **Partial** | yes | no — only the newest | Versioned arrays |
| **Full** | yes | **yes** — any version branches | `im::Vector`, git |
| **Confluent** | yes | yes, **and versions can merge** | git with merges; rope concatenation |

Most practical structures are fully persistent. Confluent persistence is harder because merging can create sharing patterns that break the Θ(log n) bound.

The consequence that matters for correctness: **because nodes are immutable, they are safe to share across threads.** Rust encodes this precisely — `Arc<Node>` where `Node` contains no interior mutability is `Send + Sync`, so a persistent structure is concurrently readable with zero synchronization. That's not an optimization; it's the property that makes the structure worth its constant factor.

## Mechanics

### Path copying — the base technique

```rust
// Persistent BST insert: Θ(log n) new nodes, everything else shared.
fn insert(node: &Option<Arc<Node>>, key: u32) -> Arc<Node> {
    match node {
        None => Arc::new(Node { key, left: None, right: None }),
        Some(n) if key < n.key => Arc::new(Node {
            key: n.key,
            left: Some(insert(&n.left, key)),      // copy this path
            right: n.right.clone(),                // ← Arc clone: share the whole subtree
        }),
        Some(n) => Arc::new(Node {
            key: n.key,
            left: n.left.clone(),                  // shared
            right: Some(insert(&n.right, key)),
        }),
    }
}
```

`n.right.clone()` is the entire trick — cloning an `Arc` bumps a refcount, so an arbitrarily large subtree is "copied" in constant time.

**Balance matters more here than for a mutable tree.** Path copying costs Θ(depth), so a degenerate tree makes updates Θ(n) *and* allocates Θ(n) nodes per update. Measured in [binary search trees](../binary-search-trees/learning.md), sorted insertion into an unbalanced BST reached depth 99,999 — as a persistent structure that would allocate 99,999 nodes per insert. Persistent structures therefore use randomized balancing (treaps, which are naturally persistent) or wide-fanout tries.

### HAMT — the persistent hash map

The **Hash Array Mapped Trie** is what backs persistent maps in Clojure, Scala, and Rust's `im`. It's a [trie](../tries-and-radix-trees/learning.md) over the *bits of the hash*, with 32-way branching:

- Each node holds a 32-bit **bitmap** of which children exist, plus a densely packed array of only those children.
- Finding a child is `popcount(bitmap & ((1 << i) - 1))` — the same bitmap-plus-popcount trick that makes tries viable ([bit manipulation](../bit-manipulation/learning.md), [tries](../tries-and-radix-trees/learning.md)).
- Depth is log₃₂ n ≈ **4 levels for 1 million entries**, so path copying touches ~4 nodes per update.

That shallow depth is why HAMTs are practical: "Θ(log₃₂ n)" is close enough to constant that people describe it as "effectively O(1)". Branching on hash bits rather than key bytes also gives uniform depth regardless of key distribution — the "randomize it" lens applied to a trie.

### RRB-trees — the persistent vector

`im::Vector` and Clojure's vectors use a 32-way branching trie indexed by *position*, giving Θ(log₃₂ n) indexing, update, push, and pop — plus **Θ(log n) concatenation and slicing**, which a flat `Vec` cannot do at all. The **relaxed** part (RRB) allows nodes to be partially full, which is what makes concatenation efficient.

For a `Vec`-shaped workload the trade is stark: a `Vec` indexes in one instruction; an RRB-tree does ~4 dependent loads. But `Vec` clone is Θ(n) and RRB clone is Θ(1).

### The Rust-specific angle

| Need | Structure |
| --- | --- |
| Shared immutable data, no versioning | `Arc<T>` — that's already persistent for a single value |
| Copy-on-write with a single owner | **`Cow<'a, T>`** / `Arc::make_mut` — clones only when actually shared |
| Persistent map / set | `im::HashMap` (HAMT), `rpds::HashTrieMap` |
| Persistent vector | `im::Vector` (RRB), `rpds::Vector` |
| Persistent list (cons) | `rpds::List` — sharing a tail is **free**, the classic functional list |
| Snapshot for concurrent readers | `arc-swap`, or `Arc<Structure>` swapped atomically |

**`Arc::make_mut` deserves attention**: it mutates in place if the refcount is 1 and clones otherwise. That gives persistent semantics with mutable performance whenever no version is actually retained — often the common case, and a genuinely good default.

**A singly-linked list is the one structure where persistence is free.** Prepending shares the entire tail with no copying at all, which is why functional languages use cons lists everywhere — and it's the "persist it" lens from [linked lists](../linked-lists/learning.md).

## Complexity

| Structure | Index/lookup | Update | Clone/snapshot | Space per version |
| --- | --- | --- | --- | --- |
| `Vec` | **Θ(1)** | Θ(1) | **Θ(n)** | Θ(n) |
| `HashMap` | **Θ(1)** | Θ(1) | **Θ(n)** | Θ(n) |
| `BTreeMap` | Θ(log n) | Θ(log n) | Θ(n) | Θ(n) |
| **Persistent BST** | Θ(log n) | Θ(log n) | **Θ(1)** | **Θ(log n)** |
| **HAMT** (`im::HashMap`) | Θ(log₃₂ n) ≈ 4 hops | Θ(log₃₂ n) | **Θ(1)** | **Θ(log₃₂ n)** |
| **RRB tree** (`im::Vector`) | Θ(log₃₂ n) | Θ(log₃₂ n) | **Θ(1)** | Θ(log₃₂ n) |
| Cons list | Θ(n) | Θ(1) prepend | **Θ(1)** | **Θ(1)** |

**Where the table misleads.** The Θ(1) clone column is the headline and it's genuine, but the Θ(log₃₂ n) lookup hides a large constant: four dependent pointer loads to scattered heap allocations versus a `Vec`'s single indexed access. Typical measured gaps for persistent collections against their mutable equivalents are **2–10× on read-heavy workloads** — not catastrophic, and not free either.

The space column is per *version*, so a workload retaining many versions accumulates Θ(versions × log n) nodes. That's excellent compared to Θ(versions × n) for full copies, but persistent structures are not a memory optimization in absolute terms — they carry per-node overhead (an `Arc` header is 16 bytes of refcounts) that a flat array doesn't.

## Use Cases

- **Undo/redo** — keep a stack of roots; undo is popping one. Editors, CAD, design tools.
- **MVCC in databases** — readers see a consistent snapshot without blocking writers; this is how Postgres, and copy-on-write B-trees like LMDB, provide isolation.
- **Time-travel and audit** — "what did this look like at time T"; the same lineage as [event sourcing](../../architecture-patterns/event-sourcing/learning.md), where history is the source of truth.
- **Concurrent read-mostly state** — publish a new `Arc<Config>` and swap it atomically; readers never lock. `arc-swap` is built for this.
- **Functional programming and interpreters** — persistent environments let closures capture bindings safely without defensive copies.
- **Incremental computation** — build systems and reactive frameworks compare old and new versions to find what changed; cheap snapshots make that practical.
- **Git and content-addressed stores** — a commit is a root; unchanged subtrees are shared by hash. Git is a confluently persistent tree.
- **Backtracking search** — persistent state means "undo" is free, which sidesteps the make/unmake bug class from [recursion & backtracking](../recursion-and-backtracking/learning.md).

## When to Use Which

| Reach for | When |
| --- | --- |
| **`Vec` / `HashMap`** | Single owner, no versioning — **the default** |
| **`Arc<T>` + `Arc::make_mut`** | Usually one owner; clone only when actually shared |
| `Cow<'a, T>` | Usually borrowed, occasionally modified |
| **`im::HashMap` / `im::Vector`** | Genuine versioning, or many concurrent readers of a changing structure |
| `rpds` | Same, with a lighter dependency and no `Arc` requirement on values |
| Cons list (`rpds::List`) | Prepend-heavy, tail sharing — persistence is genuinely free |
| **`arc-swap`** | Read-mostly shared state, atomically republished |
| Copy-on-write B-tree | Persistent *and* on disk (LMDB, `redb`) |
| `RwLock<HashMap>` | Mutation is frequent and versions aren't needed — but see the contention data in [concurrent structures](../concurrent-data-structures/learning.md) |

## Pitfalls in Depth

### Pitfall: Reaching for persistence when a clone would do

- **What goes wrong:** `im::HashMap` is adopted for a structure that's mutated by one owner and never versioned. Every lookup becomes ~4 dependent pointer loads instead of one hash probe, every insert allocates ~4 nodes, and the measured cost is typically 2–10× a `HashMap` — for a Θ(1) clone nobody performs.
- **Why it happens (the mechanism):** "Immutable data structures" carries a correctness connotation that makes them feel like the safer default. But their entire cost model is designed around *cheap versioning*, and if you never take a second version you pay the constant factor for a capability you don't use. Rust's ownership already gives you the aliasing guarantees that immutability provides in other languages.
- **How to handle it in production, and why that works:** Ask whether more than one version is ever alive simultaneously. No → `Vec`/`HashMap`. Occasionally → `Arc<T>` with `Arc::make_mut`, which mutates in place when the refcount is 1 and copies only when a snapshot is actually held — persistent semantics at mutable speed for the common path.
- **Trade-offs of the fix:** `Arc::make_mut` has a Θ(n) cliff on the first write after a snapshot is taken, so a workload that snapshots frequently gets the worst of both. When versions genuinely are retained, the real persistent structure's Θ(log n) update beats repeated full clones by a wide margin.

### Pitfall: Path copying an unbalanced structure

- **What goes wrong:** A persistent BST is built with no balancing and fed sorted keys. Each insert copies the whole root-to-leaf path — which is now Θ(n) — so a sequence of n inserts is Θ(n²) time *and* allocates Θ(n²) nodes. Memory grows quadratically, not just time.
- **Why it happens (the mechanism):** Path copying's cost is exactly the depth, so persistence multiplies the penalty for imbalance by turning a slow traversal into a slow traversal *plus* an allocation per node on the path. Measured in [binary search trees](../binary-search-trees/learning.md), sorted insertion reached depth 99,999 at n = 100,000 — as a persistent structure, that's 99,999 allocations for one insert.
- **How to handle it in production, and why that works:** Use a structure with a bounded depth by construction. Treaps are the natural fit (randomized balance, and rotations compose cleanly with path copying), and wide-fanout tries — HAMT at 32-way branching, ~4 levels for a million entries — bound the copied path to a handful of nodes regardless of key distribution.
- **Trade-offs of the fix:** Wide fanout means each copied node is larger (a 32-entry array), so you copy fewer nodes but more bytes; the net is strongly favourable but not free. Randomized balancing gives an expected rather than worst-case bound.

### Pitfall: Retaining versions accidentally

- **What goes wrong:** Old roots are kept alive — in a history vector, a cache, a closure, a channel — and because every version shares structure, the retained nodes pin *all* the structure reachable from them. Memory grows without bound and profiling shows no obvious leak, since every allocation is genuinely reachable.
- **Why it happens (the mechanism):** Structural sharing makes versions cheap to *create*, which makes it easy to create many, and refcounting frees a node only when the last version referencing it drops. A single retained old root can therefore keep a large amount of superseded data alive. It's the mirror image of the benefit.
- **How to handle it in production, and why that works:** Bound the version history explicitly — an undo stack with a maximum depth, a snapshot registry with a TTL — so old roots are dropped on a schedule. Treat retained versions as a resource with a budget, and monitor the count. This is the same discipline as bounding a queue ([stacks & queues](../stacks-and-queues/learning.md)): unbounded retention is a decision, usually an unintended one.
- **Trade-offs of the fix:** A bounded history means undo has a limit and time-travel has a horizon, which may be a product decision rather than an engineering one. Reference counting also can't reclaim cycles — not an issue for trees, but relevant if the structure admits them.

### Pitfall: Assuming persistent means thread-safe to *build*

- **What goes wrong:** A persistent structure is treated as concurrency-safe in general, and two threads independently derive new versions from the same root, then each publishes theirs. The updates don't merge — one silently overwrites the other, because each derived its version from the same base and neither saw the other's change. Lost updates, with no error.
- **Why it happens (the mechanism):** Immutability makes *reading* safe with no synchronization, which is a genuine and large benefit — so it's easy to extend the assumption to writing. But "derive a new version and store it" is a read-modify-write on the shared root pointer, and that composite operation is not atomic just because the data is immutable.
- **How to handle it in production, and why that works:** Publish new versions with a **compare-and-swap** on the root (`arc-swap`'s `compare_and_swap`, or an `AtomicPtr`), retrying if the base changed — the same optimistic-concurrency pattern as `expected_version` in [event sourcing](../../architecture-patterns/event-sourcing/learning.md) and CAS in [concurrent structures](../concurrent-data-structures/learning.md). Or serialize writers behind a single mutex while readers continue lock-free, which is often the right shape: many readers, one writer.
- **Trade-offs of the fix:** CAS retries waste work under high write contention, so a write-heavy workload may be better served by a lock. And a single-writer lock reintroduces a serialization point — though readers remain free, which is the whole point of the arrangement.
