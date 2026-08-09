# Rust for Data Structures — Learning Notes

## Mental Model

**Every textbook data structure assumes a capability Rust removes: many mutable pointers into one graph of nodes.** CLRS pseudocode says `x.parent.left = y` without a second thought. That single line requires aliasing *and* mutation, which is precisely what the borrow checker exists to forbid.

So learning data structures in Rust is really learning **five representation strategies** and knowing which one a given structure needs. That choice is made once, early, and it determines everything downstream — API shape, performance, whether you'll need `unsafe`, and how painful the code is to write:

1. **Ownership tree with `Box`** — unique ownership, strictly downward links.
2. **Arena + index handles** — one `Vec<Node>`, links are integers. The workhorse.
3. **`Rc<RefCell<T>>`** — shared mutable nodes, checked at runtime.
4. **Slices + split borrows** — no nodes at all; disjoint `&mut` into one buffer.
5. **Raw pointers behind a safe API** — what std itself does, when nothing else works.

The right instinct, learned once and reused for the whole category: **when a structure has back-pointers, cross-links, or shared nodes, reach for the arena, not for `Rc<RefCell>`.** Newcomers reach for `Rc<RefCell>` because it's the closest thing to a pointer; it is almost always the worse answer, and the "Pitfalls" section explains why in detail.

The second instinct: **fighting the borrow checker over a data structure is usually a signal that the representation is wrong, not that Rust is being obstinate.** The exception is real — some structures (doubly-linked lists, splay trees, intrusive lists) genuinely need `unsafe`, and knowing which ones is part of the skill rather than a failure.

## The Invariant

Everything in this doc descends from one rule:

> **Aliasing XOR mutability.** At any moment a value may have *either* any number of `&T`, *or* exactly one `&mut T`. Never both.

The five strategies are five different answers to "how do I build a linked structure under that rule":

| Strategy | How it satisfies the invariant | Cost |
| --- | --- | --- |
| `Box` tree | Proves uniqueness statically — each node has exactly one owner | No back-links, no sharing |
| Arena + indices | **Sidesteps it** — an index isn't a reference, so it can't alias | Bounds checks; manual lifetime discipline for handles |
| `Rc<RefCell>` | Moves the check to runtime | Panics instead of compile errors; refcount; cycles leak |
| Split borrows | Proves disjointness statically | Only works for contiguous, statically-partitionable data |
| Raw pointers | You assert it; the compiler stops checking | Full `unsafe` obligation; needs Miri |

The arena's trick is worth stating plainly because it's the one that unlocks the whole category: **an index is not a borrow.** `nodes[a].next = b` mutates one `Vec`, briefly, through one `&mut`. There is no graph of references for the borrow checker to reason about, so arbitrary topology — cycles, back-pointers, cross-links — becomes trivially expressible.

## Mechanics

### 1. `Box` — the ownership tree

The natural fit for BSTs, tries, expression trees: strictly downward links, one owner each.

```rust
struct Node { key: i32, left: Link, right: Link }
type Link = Option<Box<Node>>;
```

`Option<Box<T>>` is **8 bytes, not 16** — the null-pointer niche encodes `None`, so there's no tag. (Verified: `size_of::<Option<Box<Node>>>() == size_of::<Box<Node>>() == 8`.) This is why `Option<Box<T>>` is the idiomatic nullable link and costs nothing.

Mutation by descent is the pattern to memorize — a cursor that walks *into the hole* where the new node goes:

```rust
fn insert(&mut self, key: i32) {
    let mut cur = &mut self.root;                 // &mut Link
    while let Some(node) = cur {
        cur = if key < node.key { &mut node.left } else { &mut node.right };
    }
    *cur = Some(Box::new(Node { key, left: None, right: None }));
}
```

This compiles because each iteration *reassigns* `cur` to a borrow derived from the previous one — the old borrow ends exactly when the new one begins, so only one `&mut` is ever live. Deletion is harder (you need to move a node out while keeping the tree valid); `Option::take()` is the tool, and it's why `Link` is `Option`-shaped rather than a bare `Box`.

**The trap:** `Box` chains drop *recursively*. Measured on this machine (release build, 8 MB main-thread stack): a singly-linked list of `Box` nodes survives 250,000 nodes and **aborts with `fatal runtime error: stack overflow` at 300,000**. That's a crash in production from a structure that worked in every test. Any `Box` structure that can get deep needs a manual iterative `Drop`:

```rust
impl Drop for List {
    fn drop(&mut self) {
        let mut cur = self.head.take();
        while let Some(mut node) = cur {
            cur = node.next.take();               // node dropped here, depth 1
        }
    }
}
```

A balanced tree is safe (depth log n); a list, a degenerate BST, or a path-shaped graph is not.

### 2. Arena + index handles — the workhorse

```rust
struct Node<T> { value: T, next: Option<u32>, prev: Option<u32>, parent: Option<u32> }
struct Arena<T> { nodes: Vec<Node<T>> }
```

Everything the borrow checker made hard is now trivial: back-pointers, cycles, cross-links, multiple "references" to one node. Plus four benefits people don't expect:

- **Half the memory.** `u32` indices instead of 8-byte pointers. A node with 3 links drops from 24 to 12 bytes — which is a [cache locality](../../performance-optimization/cache-locality/learning.md) win, not just a footprint one.
- **Contiguity.** Nodes sit in one allocation, so traversal order can be made sequential and the prefetcher can help. A `Box`-per-node tree is a pointer chase through the whole heap.
- **`Copy` handles.** Passing a `u32` around has none of the ownership friction of passing `&mut Node`.
- **Trivially serializable and snapshottable** — it's a `Vec`, so cloning the arena clones the whole graph.

The cost is that you've taken over memory management: freeing individual nodes needs a free list, and **a handle can outlive the node it named**. When slot 7 is freed and reused, an old `Handle(7)` now silently points at a different node — a use-after-free that the type system was supposed to prevent, reintroduced as a logic bug. The fix is **generational indices**:

```rust
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Handle { idx: u32, gen: u32 }

pub struct Arena<T> { slots: Vec<(u32, Option<T>)>, free: Vec<u32> }

impl<T> Arena<T> {
    pub fn insert(&mut self, value: T) -> Handle {
        if let Some(idx) = self.free.pop() {
            let slot = &mut self.slots[idx as usize];
            slot.0 += 1;                                  // bump generation
            slot.1 = Some(value);
            Handle { idx, gen: slot.0 }
        } else {
            self.slots.push((0, Some(value)));
            Handle { idx: (self.slots.len() - 1) as u32, gen: 0 }
        }
    }

    pub fn get(&self, h: Handle) -> Option<&T> {
        let slot = self.slots.get(h.idx as usize)?;
        if slot.0 == h.gen { slot.1.as_ref() } else { None }   // stale handle → None
    }
}
```

A stale handle now resolves to `None` instead of the wrong node. This is exactly what `slotmap` and `generational-arena` provide, and what every ECS and every graph library in Rust converges on — `petgraph` is an arena with a nicer API.

### 3. `Rc<RefCell<T>>` — shared mutable nodes

```rust
type Shared = Rc<RefCell<Node>>;
struct Node { value: i32, children: Vec<Shared>, parent: Option<Weak<RefCell<Node>>> }
```

Three costs, all real: **runtime panics** (two overlapping `borrow_mut()` aborts the thread — a compile error converted into a production incident), **8 bytes of refcount plus a heap allocation per node plus a pointer chase**, and **leaks on cycles** — a parent→child `Rc` with a child→parent `Rc` means neither count ever reaches zero. `Weak` for the back-edge is mandatory, not stylistic.

Legitimate uses: an observer/callback graph where nodes are genuinely shared with outside code, GUI widget trees, and interop where an external API hands you shared ownership. For a data structure you own end-to-end, the arena is better on every axis.

### 4. Split borrows — no nodes at all

Many structures don't need nodes. Heaps, union-find, Fenwick trees, and hash tables are arrays with arithmetic, and the borrow checker never objects. When you *do* need two `&mut` into one buffer, prove disjointness:

```rust
let (left, right) = slice.split_at_mut(mid);      // two disjoint &mut [T]
for chunk in data.chunks_mut(1024) { /* ... */ }  // disjoint windows
let [a, b] = arr.get_disjoint_mut([i, j]).unwrap();    // Result; Err on overlap
std::mem::swap(a, b);
```

`get_disjoint_mut` (stable since 1.86; formerly `get_many_mut`) is the clean answer to "I need two elements of one `Vec` mutably" — the case that used to force `split_at_mut` gymnastics or indices-and-copies.

### 5. Raw pointers behind a safe API

What std does. `LinkedList` and `VecDeque` use `NonNull<T>` internally; `HashMap` is `hashbrown`, which is dense `unsafe`. The rule is **contain it**: `unsafe` inside the module, a fully safe public API, an invariant comment on every `unsafe` block, and Miri in CI:

```sh
cargo +nightly miri test        # catches UB: aliasing violations, use-after-free, uninit reads
```

Miri is not optional for hand-written `unsafe` data structures. It catches the aliasing mistakes that pass tests, pass code review, and then miscompile under a new optimizer.

### Trait contracts you must not break

- **`Ord` must be a total order.** Break it and `BTreeMap` silently loses entries and `sort` may panic or produce garbage — no error, just wrong data. The classic breakage is floats: `f64` is `PartialOrd` but not `Ord` because `NaN` compares false to everything. `sort_by(|a, b| a.partial_cmp(b).unwrap())` panics on `NaN`. Use `f64::total_cmp` or the `ordered-float` crate.
- **`Hash` and `Eq` must agree**: `a == b` ⟹ `hash(a) == hash(b)`. Derive both or implement both; deriving one and hand-writing the other is how entries become unfindable.
- **Keys must not mutate while in a map.** Rust mostly enforces this by never handing out `&mut K` — but interior mutability (a `Cell` inside a key) defeats it, and the entry becomes permanently unreachable.
- **`Borrow` is what makes ergonomic lookup work.** `HashMap<String, V>::get(&str)` and `BTreeMap<String, V>::range("a".."b")` work because `String: Borrow<str>`. Implement `Borrow` for your own key wrappers and the same ergonomics follow.

### The std facts that change your constants

- **`HashMap` is SwissTable** (`hashbrown`), with SIMD probing of 16 control bytes at a time — its constant is far better than a textbook chaining table.
- **The default hasher is SipHash-1-3 with a random per-instance seed.** That's DoS resistance, and it isn't free. Measured: for `u32` keys a Fx-style hasher is **4.6–6.0× faster** end-to-end on lookup; for 16-char `String` keys only **1.2–1.3×**, because the hash is amortized against the comparison and the memory access. So the swap matters most exactly where arenas put you — maps keyed by small integer handles. For anything attacker-reachable, keep the default regardless. See [complexity analysis](../complexity-analysis/learning.md) for the full table and why this is a security property, not a tuning knob.
- **`BinaryHeap` is a max-heap.** `BinaryHeap<Reverse<T>>` gives you a min-heap — the Dijkstra idiom.
- **The `Entry` API exists to avoid double lookup**: `*map.entry(k).or_insert(0) += 1` hashes once, where `contains_key` + `insert` hashes twice.
- **`Vec<T>` is 24 bytes** (ptr, len, cap); `Box<[T]>` is 16 — worth it for immutable-after-build arrays stored in bulk.

## Complexity

Representation choice, priced:

| Strategy | Node overhead | Traversal | Alloc per node | `unsafe` | Compile-time safe |
| --- | --- | --- | --- | --- | --- |
| `Box` tree | 8 B/link | pointer chase, scattered | yes | no | yes |
| Arena + `u32` | 4 B/link | sequential-friendly | no (amortized) | no | handles unchecked |
| Generational arena | 8 B/link + 4 B/slot | same | no | no | stale → `None` |
| `Rc<RefCell>` | 16 B refcounts + 8 B borrow flag | chase + refcount traffic | yes | no | runtime panic |
| Raw pointers | 8 B/link | chase | yes | **yes** | you assert it |

## Use Cases

- **Compilers and interpreters:** arena everything. `rustc` interns strings to `Symbol` (a `u32`) and allocates AST/HIR nodes in arenas — both moves come straight from this doc.
- **ECS / game state:** generational arenas *are* the architecture (`slotmap`, `hecs`). Entity handles are exactly the `Handle` above.
- **Graphs:** `petgraph`'s `NodeIndex`/`EdgeIndex` are arena handles; CSR representations go further and drop the arena for two flat arrays.
- **Parsers and text editors:** `Cow<'a, str>` and borrowed slices over one input buffer — strategy 4, no nodes at all.
- **LRU caches:** the canonical "needs a doubly-linked list" structure. The `lru` crate uses raw pointers; the arena version (indices for prev/next) is safe and nearly as fast.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`Box` tree** | Strictly downward links, one owner, no back-pointers — BSTs, tries, ASTs. Depth is bounded or you write an iterative `Drop`. |
| **Arena + indices** | Any graph, back-pointers, cross-links, cycles, or many nodes of uniform type. **The default for this category.** |
| **Generational arena** | The arena case *and* handles are stored by callers or outlive removals. |
| **`Rc<RefCell>`** | Nodes genuinely shared with code you don't control, or an interop boundary demands shared ownership. |
| **Slices / split borrows** | The structure is really an array: heaps, union-find, Fenwick trees, hash tables. |
| **Raw pointers** | Intrusive or self-referential structures, or a measured hot path where indices' bounds checks provably matter. Miri mandatory. |

## Pitfalls in Depth

### Pitfall: Reaching for `Rc<RefCell<T>>` as the default pointer

- **What goes wrong:** A tree with parent pointers gets written as `Rc<RefCell<Node>>` because it's the first thing that compiles. Then: a `BorrowMutError` panic in production the first time a traversal holds a borrow across a callback that re-enters; a memory leak because parent and child both hold `Rc`; and a traversal 3–5× slower than the arena version from refcount traffic and scattered allocation.
- **Why it happens (the mechanism):** `Rc<RefCell<T>>` is the closest syntactic analogue to a C pointer, so it's what a C/Java/Python instinct reaches for. It compiles immediately, which feels like validation — but it has converted *compile-time* errors into *runtime* ones. The borrow checker didn't approve the design; it was turned off.
- **How to handle it in production, and why that works:** Default to the arena. An index cannot alias, so the entire class of borrow conflicts disappears at the representation level rather than being deferred to runtime. Keep `Rc<RefCell>` for genuine external sharing, and always `Weak` for back-edges so cycles can't leak.
- **Trade-offs of the fix:** The arena makes node lifetime your problem (free list, stale handles) and adds a bounds check per access. It also makes "one node" hard to hand out as an owned value — callers get handles plus an `&Arena`, which is a slightly heavier API. Accept the API cost; it buys away a whole panic class.

### Pitfall: Recursive `Drop` overflowing the stack

- **What goes wrong:** A `Box`-linked list, an unbalanced BST built from sorted input, or a deep parse tree is dropped and the process aborts with `fatal runtime error: stack overflow`. It's not catchable, not a panic, and it fires at drop time — often far from the code that built the structure, in a destructor at the end of a request.
- **Why it happens (the mechanism):** The derived `Drop` for `Box<Node>` recurses one stack frame per link. Measured here in release with the default 8 MB main-thread stack: **fine at 250k nodes, aborts at 300k** (~30 bytes of frame per node). Spawned threads default to 2 MB, so the threshold there is roughly 4× lower — code that survives on the main thread dies in a worker.
- **How to handle it in production, and why that works:** Implement `Drop` iteratively with `Option::take()` (shown above) — it unlinks one node at a time at constant stack depth. Better still, use an arena, where dropping the structure is dropping one `Vec` and no recursion exists at all.
- **Trade-offs of the fix:** A hand-written `Drop` must be kept in sync as the structure gains fields, and it's easy to write one that leaves nodes reachable-but-unfreed. It also blocks some niceties (a type with a manual `Drop` can't be destructured by move). Balanced trees don't need it — the depth is log n — so apply it where degeneracy is possible, not everywhere.

### Pitfall: Stale arena handles — use-after-free, reinvented

- **What goes wrong:** Node 7 is removed; its slot goes on the free list; a new node is allocated into slot 7. Code still holding the old `Handle(7)` now reads and mutates a completely unrelated node. No crash, no panic — just quietly corrupted data, exactly the bug class Rust is supposed to have eliminated.
- **Why it happens (the mechanism):** The arena bought freedom from the borrow checker by making links plain integers — and integers carry no lifetime information. You reintroduced manual memory management and, with it, dangling references wearing a safe-looking type.
- **How to handle it in production, and why that works:** Generational handles (`{ idx, gen }`, bump `gen` on reuse, compare on access). A stale handle fails the generation check and resolves to `None`, converting silent corruption into a visible, testable failure at the point of use. `slotmap` and `generational-arena` are the off-the-shelf versions.
- **Trade-offs of the fix:** 4 extra bytes per handle and per slot, plus a comparison per access. If handles never escape a single tightly-scoped pass (build then traverse, no removals), a plain `u32` is genuinely fine — the generation is insurance against a pattern you may not have.

### Pitfall: Breaking the `Ord` / `Hash` contracts

- **What goes wrong:** A custom `Ord` that isn't transitive, or a hand-written `Hash` that disagrees with a derived `Eq`. Symptoms are absurd and hard to attribute: a `BTreeMap` where `insert` then `get` returns `None`; a `HashSet` containing two equal elements; `binary_search` missing a value that's present; occasionally a `sort` panic reporting the comparator is broken.
- **Why it happens (the mechanism):** These are *contracts*, not just trait signatures — the compiler checks the shape, never the semantics. A comparator that sorts by one field while `Eq` compares two is inconsistent but perfectly type-correct. Floats are the standard trap: `NaN` makes every comparison false, so `partial_cmp().unwrap()` panics and any `NaN`-containing sort is meaningless.
- **How to handle it in production, and why that works:** Derive `PartialEq`/`Eq`/`Hash` together whenever possible so they can't drift. For floats use `f64::total_cmp` (a genuine total order, `NaN` included) or the `ordered-float` newtype. For hand-written impls, property-test the laws with `proptest`: transitivity, antisymmetry, and `a == b ⟹ hash(a) == hash(b)` are three short properties that catch every instance of this bug.
- **Trade-offs of the fix:** Deriving forces all fields to participate, which is sometimes wrong (a cached field shouldn't affect equality) — and that's exactly when you must hand-write both and test them together. `total_cmp` orders `NaN` in a defined but arbitrary position, so a `NaN` still gets *somewhere* in your sorted output; filter them at the boundary if they're meaningless.

### Pitfall: Paying SipHash on internal keys (and dropping it on external ones)

- **What goes wrong:** Two mirror-image mistakes. A hot inner loop keys a `HashMap<u32, T>` with the default hasher and spends measurable time hashing 4 bytes with a cryptographic-strength function. Or someone reads that `FxHashMap` is faster, swaps it globally, and now a map keyed by user-supplied strings can be driven into O(n) buckets by an attacker.
- **Why it happens (the mechanism):** The hasher is a global-feeling default, so it gets treated as a global decision. It isn't — it's a per-map security/performance trade, and the correct answer depends entirely on where that map's keys come from.
- **How to handle it in production, and why that works:** Classify each map by key provenance. Self-generated keys (indices, interned symbols, enum discriminants) → `FxHashMap`/`aHash`, because no adversary can choose them. Anything reachable from user input, network, or files → keep std's `RandomState`. The random per-process seed is what makes precomputed collision attacks impossible.
- **Trade-offs of the fix:** Two hasher types in one codebase invites the wrong one being copy-pasted into a new map. A type alias per category (`type FastMap<K,V> = HashMap<K,V,FxBuildHasher>;`) with a comment on each makes the decision reviewable, which is the actual goal.

## Creative & Lateral Thinking

**Transformation lenses**, applied to *representation* rather than to algorithms:

| Lens | Question | What it produces in Rust |
| --- | --- | --- |
| Persist it | What if updates returned a new version? | `Rc`-shared subtrees; `im`/`rpds` HAMTs; structural sharing |
| Batch it | What if nodes were built all at once, then frozen? | Build in a `Vec`, convert to `Box<[T]>`; CSR graphs |
| Approximate it | What if handles could be wrong 1-in-2³² times? | Generation counters — exactly this trade |
| Randomize it | What if balancing were a coin flip? | Treaps, skip lists — no rotation code, no parent pointers |
| Externalize it | What if the arena were a memory-mapped file? | Indices are position-independent; pointers aren't — the arena serializes for free |
| Parallelize it | Where's the contention? | `split_at_mut` for disjoint work; `&Arena` shares readonly across threads |
| Invert it | What if children pointed at parents instead? | Union-find: parent-only links, no children, no allocation |
| Augment it | What does one more `u32` per node buy? | Subtree sizes → order statistics; generation → safe handles |
| Specialize it | What if the tree were complete? | Implicit heap: no links at all, children at 2i+1, 2i+2 |
| Amortize it | What if one operation could be terrible? | `Vec` doubling; arena compaction as a stop-the-world pass |

**Questions:**

1. The arena "sidesteps" the borrow checker by using integers. Has safety been *lost*, or *relocated*? Name precisely which guarantee you gave up and which new mechanism replaces it.
2. `Option<Box<T>>` is 8 bytes but `Option<u32>` is 8 bytes too (4 wasted). Why does the niche optimization apply to one and not the other — and what would you change about your handle type to recover those 4 bytes?
3. Under the "externalize" lens, an arena can be memory-mapped and a `Box` tree cannot. State the one property of indices that makes this true, and name another benefit that falls out of the same property.
4. Union-find stores only parent links and no children. Under which lens is that derived from a tree, and what operation did it give up to get O(α(n))?
5. A binary heap needs no links at all. Which lens produced that, what precondition does it require, and why can't a BST use the same trick?
6. You need a doubly-linked list. Give three implementations (arena, `Rc<RefCell>` + `Weak`, raw pointers) and the *specific* requirement that would force each one — not preference, requirement.
7. `RefCell` moves a compile-time check to runtime. Where else in this list does a check move between compile time and runtime, and is there a case where moving it *back* is possible?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the aliasing-XOR-mutability rule, then explain in one sentence each how the arena and `RefCell` satisfy it — they do so in fundamentally different ways.
2. Why does the `while let Some(node) = cur { cur = &mut node.left }` descent compile, when holding two `&mut` into the same tree does not?
3. Your `Box`-based BST works in tests and aborts in production with a stack overflow at drop. Give the mechanism, the input shape that triggers it, and two independent fixes.
4. You have `HashMap<NodeId, Data>` where `NodeId` is a `u32` you assign. Which hasher, and what would change your answer?
5. A colleague's `sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap())` panics once a week. Diagnose it and give the two-character-ish fix.
6. When is `Rc<RefCell<T>>` the *right* answer? Give a concrete scenario the arena cannot serve.

Build exercises:

- Implement a BST twice — `Box`-based and arena-based — with insert, search, and in-order traversal. Then: build both from **sorted** input of 1M keys (worst case, fully degenerate) and observe the `Box` version abort on drop; fix it with an iterative `Drop`. Benchmark traversal on both with criterion and explain the gap using [cache locality](../../performance-optimization/cache-locality/learning.md).
- Build the generational arena from this doc, then write the bug it prevents: insert, remove, insert again, and assert the old handle returns `None`. Remove the generation field and watch the same test read the wrong node — that failing test is the whole argument for generational indices in one screen.
- Write an LRU cache with a `HashMap` plus an arena-based doubly-linked list. It's the smallest realistic structure that genuinely needs back-pointers, and it forces every idea in this doc at once.
- Take any `unsafe` linked structure you write and run `cargo +nightly miri test`. If it passes first try, deliberately introduce an aliasing violation (hold `&mut` while reading through a raw pointer) and confirm Miri catches what your tests didn't.

## Open Questions

- Bounds-check cost in arenas: what does `get_unchecked` actually buy on a hot traversal, measured — and does the safe version autovectorize equally?
- `slotmap` vs a hand-rolled generational arena: measure the API and performance difference on the LRU exercise before defaulting to the crate.
- Polonius: which of the borrow-checker rejections in this doc does it actually fix, and what's its current status?
- ~~`FxHashMap` vs `RandomState` for `u32` keys~~ **measured**: 4.6–6.0×, far more than the 2–3× folklore. Still open: does `aHash` recover most of that while keeping DoS resistance?
- Is there a clean way to make arena handles type-safe across *multiple* arenas (a `Handle<Node>` that can't index an `Arena<Edge>`) without a phantom-type explosion?

## References

- Aria Beingessner, [Learn Rust With Entirely Too Many Linked Lists](https://rust-unofficial.github.io/too-many-lists/) — the single best resource for this topic. Works through `Box`, `Rc`, `RefCell`, arenas and `unsafe` on one structure and shows exactly where each breaks. Read it before writing any linked structure in Rust.
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/) — the contract you take on with `unsafe`: aliasing rules, variance, `Drop` order, uninitialized memory.
- Catherine West, "Using Rust for Game Development" (RustConf 2018 closing keynote) — the talk that made generational arenas the standard Rust answer for graph-shaped data; the argument generalizes far beyond games.
- [`slotmap`](https://docs.rs/slotmap/) and [`generational-arena`](https://docs.rs/generational-arena/) — the production versions of this doc's `Handle`; read `slotmap`'s docs for the design trade-offs it makes.
- [`hashbrown`](https://github.com/rust-lang/hashbrown) — std's `HashMap`; the SwissTable design and its SIMD probe are worth reading once for the constant-factor lesson.
- [Miri](https://github.com/rust-lang/miri) — mandatory for hand-written `unsafe` structures.
- Related topics in this repo: [Complexity Analysis](../complexity-analysis/learning.md) (the other half of Stage 0), [Rust best practices](../../language-best-practices/rust/learning.md) (the general idioms this specializes), [Cache Locality](../../performance-optimization/cache-locality/learning.md) + [Memory Layout](../../performance-optimization/memory-layout/learning.md) (why arenas win on real hardware), [Lock-Free Concurrency](../../performance-optimization/lock-free-concurrency/learning.md) (where the raw-pointer strategy becomes unavoidable).
