# Linked Lists — Learning Notes

## Mental Model

**A linked list trades locality for splice.** It buys you O(1) insertion and removal *at a position you already hold*, and it pays for that with a pointer dereference per element and an allocation per node. The textbook presents this as an even trade. On modern hardware it is not remotely even.

Measured on this machine, summing 5M `u64` values, all Θ(n):

| Layout | Time per element | Relative |
| --- | --- | --- |
| `Vec<u64>` | **0.13 ns** | 1× |
| `Box` list, nodes in allocation order | 1.30 ns | 10× |
| `Box` list, nodes in scattered order | **84.78 ns** | **641×** |

The third row is the realistic one. A list that has lived — nodes allocated and freed at different times, interleaved with other allocations — has its nodes scattered across the heap, and traversal becomes a dependent-load chain: you cannot fetch node *i+1* until node *i* has arrived, so every step is a full memory latency with **no prefetching and no parallelism possible**. The array scan, by contrast, is predictable, prefetched, and vectorized.

So the honest mental model is:

> A linked list is the right answer when you splice far more often than you traverse, and you get your position for free from somewhere else.

That last clause is where most linked-list decisions actually go wrong. "O(1) insertion" is true *given a cursor*. Finding the cursor is O(n) — and an O(n) traversal at 84 ns/element costs more than the O(n) `memmove` that `Vec::insert` would have done, because `memmove` runs at bus speed. **A linked list you have to search is strictly worse than an array.**

The structure earns its place in exactly two situations: when something else hands you the position (a hash map, an intrusive back-pointer), or when you must splice whole *ranges* in O(1). Otherwise it's a teaching device — which is still a real use, because the invariant discipline it teaches shows up in every pointer-based structure afterward.

## The Invariant

For a singly-linked list:

> Every node is owned by exactly one predecessor (or by the list head); `next` is `None` exactly at the tail; the chain from `head` reaches every node exactly once and terminates.

For doubly-linked, add the one that makes it hard:

> `node.next.prev == node` and `node.prev.next == node`, for every node.

That mutual-consistency requirement is the entire difficulty of doubly-linked lists. Every insertion and removal must update **four** pointers, and any ordering mistake leaves the list traversable in one direction and corrupt in the other — a bug that hides until something iterates backward. It's also precisely the invariant Rust's ownership model cannot express: two nodes each holding a reference to the other is shared mutable aliasing by construction.

## Mechanics

### The three variants

| Variant | Node | Buys | Costs |
| --- | --- | --- | --- |
| Singly | `next` | Simplest; O(1) push/pop front | No backward traversal; removal needs the *predecessor* |
| Doubly | `next`, `prev` | O(1) removal given the node itself; bidirectional | 2× link overhead; 4 pointer updates per edit; needs `unsafe` or an arena in Rust |
| Circular | tail links to head | No special-casing at the ends; natural round-robin | Traversal needs a termination rule; easy to loop forever |

**The subtle one:** removing a node from a *singly*-linked list requires its predecessor, so a "pointer to the node" isn't enough. This is why the classic interview trick (copy the next node's value into this one and delete *that*) exists, and why real singly-linked implementations pass around `&mut Option<Box<Node>>` — a pointer to the *link*, not to the node. That's the same "cursor into the hole" idea from [Rust for data structures](../rust-for-data-structures/learning.md), and it's the cleanest way to think about list surgery in any language.

### The operations, honestly priced

| Operation | Complexity | The catch |
| --- | --- | --- |
| Push/pop front | Θ(1) | Genuinely fast |
| Push/pop back | Θ(1) with a tail pointer, Θ(n) without | Singly-linked without a tail pointer is a common accident |
| Insert/remove **given a cursor** | Θ(1) | The headline claim |
| Insert/remove **at index i** | Θ(i) | And at ~85 ns/step, worse than `Vec`'s `memmove` |
| Search | Θ(n) | ~640× slower per element than an array scan |
| Splice a whole sublist | **Θ(1)** | The genuinely unbeatable operation |
| Concatenate two lists | **Θ(1)** | Same |
| Index random access | Θ(n) | Not a random-access structure at all |

**The one column that matters:** splice and concatenate are Θ(1) *regardless of size*. Moving a 10-million-element range from one list to another is four pointer writes. No array can do that. If your workload is dominated by that operation, the list wins and nothing else comes close.

### Memory overhead

A `u64` payload in a `Box`-based node:

| Structure | Bytes per element | Overhead |
| --- | --- | --- |
| `Vec<u64>` | 8 | 0% |
| Singly-linked `Box` node | 16 + allocator header (~16) | ~300% |
| Doubly-linked node | 24 + header | ~400% |
| Arena node, `u32` links | 12–16 | 50–100% |

Three to four times the memory, *and* it's scattered, so the effective cache footprint is worse still — a node touched brings in a 64-byte line of which you use 16. This compounds with the pointer chase rather than being a separate cost.

### The four Rust representations

Straight from Stage 0, applied:

1. **`Box` singly-linked** — works, and the standard teaching path. Beware the recursive `Drop`: measured, a `Box` chain survives 250k nodes and aborts with a stack overflow at 300k. An iterative `Drop` is mandatory.
2. **Arena + `u32` indices** — the practical answer for doubly-linked. Links are integers, so `prev`/`next` are trivially expressible, nodes are contiguous (recovering much of the locality), and `Drop` is dropping one `Vec`.
3. **`Rc<RefCell<Node>>` + `Weak` for `prev`** — compiles, and is the worst option: refcount traffic on every traversal step, runtime borrow panics, and mandatory `Weak` back-edges or the whole list leaks.
4. **Raw `NonNull` pointers** — what `std::collections::LinkedList` and the `lru` crate do. Correct, fast, and requires Miri.

### `std::collections::LinkedList`

It exists, and the std docs themselves say to use `Vec` or `VecDeque` instead. It has no cursor API on stable other than `cursor_front`/`cursor_back` (still unstable for mutation as of writing), which means **you cannot actually perform the O(1) splice-at-position that is the list's only reason to exist**. If you need a linked list in Rust, you almost always need to write it (arena) or take a crate that wraps `unsafe` for a specific purpose (`lru`, `intrusive-collections`).

## Complexity

| Operation | Singly | Doubly | Space |
| --- | --- | --- | --- |
| Push/pop front | Θ(1) | Θ(1) | — |
| Push/pop back | Θ(n) / Θ(1) with tail | Θ(1) | — |
| Insert/remove given cursor | Θ(1)* | Θ(1) | — |
| Insert/remove at index | Θ(n) | Θ(n) | — |
| Search | Θ(n) | Θ(n) | — |
| Splice / concat | Θ(1) | Θ(1) | — |
| Whole structure | — | — | Θ(n), 3–4× array |

`*` singly-linked needs the *predecessor's* link, not the node.

**Where the table misleads, badly.** Both `Vec::insert(i)` and list-insert-at-i are "Θ(n)", and they differ by two orders of magnitude in the array's favour: `memmove` runs at ~10 GB/s sequentially, while list traversal is a serial dependent-load chain at ~85 ns per hop. The asymptotic notation is doing real damage here — this is the clearest case in the whole category of a bound that is technically correct and practically inverted.

## Rust Implementation

**Singly-linked with `Box`, with the mandatory iterative `Drop`:**

```rust
pub struct List<T> { head: Link<T> }
type Link<T> = Option<Box<Node<T>>>;
struct Node<T> { value: T, next: Link<T> }

impl<T> List<T> {
    pub fn push_front(&mut self, value: T) {
        self.head = Some(Box::new(Node { value, next: self.head.take() }));
    }
    pub fn pop_front(&mut self) -> Option<T> {
        self.head.take().map(|node| { self.head = node.next; node.value })
    }
    /// Remove the first element matching `pred` — the "cursor into the link" pattern.
    pub fn remove_first(&mut self, pred: impl Fn(&T) -> bool) -> Option<T> {
        let mut cur = &mut self.head;
        loop {
            match cur {
                None => return None,
                Some(node) if pred(&node.value) => {
                    let node = cur.take().unwrap();      // detach
                    *cur = node.next;                     // relink
                    return Some(node.value);
                }
                Some(_) => {
                    cur = &mut cur.as_mut().unwrap().next;
                }
            }
        }
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {                    // WITHOUT this: stack overflow at ~300k nodes
        let mut cur = self.head.take();
        while let Some(mut node) = cur { cur = node.next.take(); }
    }
}
```

`take()` is the workhorse: it moves the `Option` out and leaves `None`, which is how you get ownership of a node while the borrow checker is watching.

**Doubly-linked over an arena** — the representation to actually reach for:

```rust
struct Node<T> { value: T, prev: Option<u32>, next: Option<u32> }
pub struct DList<T> { nodes: Vec<Node<T>>, head: Option<u32>, tail: Option<u32>, free: Vec<u32> }

impl<T> DList<T> {
    /// O(1) removal given a handle — this is the operation that justifies the structure.
    fn unlink(&mut self, i: u32) {
        let (prev, next) = (self.nodes[i as usize].prev, self.nodes[i as usize].next);
        match prev { Some(p) => self.nodes[p as usize].next = next, None => self.head = next }
        match next { Some(n) => self.nodes[n as usize].prev = prev, None => self.tail = prev }
        self.free.push(i);
    }
}
```

Four pointer updates, no `unsafe`, no borrow-checker fight — because indices aren't borrows. Add generational handles (Stage 0) if handles escape.

**Crates:** `lru` (the canonical hash-map-plus-list LRU, `unsafe` internally), `intrusive-collections` (intrusive lists — nodes embedded in your types, zero allocation per link), `crossbeam` (lock-free queues, which are linked lists where the linking *is* the concurrency mechanism).

## Use Cases

The short, honest list — these are the cases where a linked list is genuinely right:

- **LRU caches.** The canonical justified use. A `HashMap<K, Handle>` gives you the node position in O(1), and then the list does O(1) unlink-and-move-to-front. Note what makes this work: **the hash map supplies the cursor**, so the list never has to search. Remove the map and the design collapses.
- **Intrusive lists in systems code.** The Linux kernel's `list_head`, embedded allocator free lists, scheduler run queues. The node links live *inside* the object, so a struct can be on several lists at once with zero allocation. This is where lists are unambiguously correct.
- **O(1) splice of large ranges.** Text editors' piece tables, gap-buffer alternatives, undo systems that move whole spans between documents. No array can concatenate two 10M-element sequences in four writes.
- **Lock-free queues.** The Michael-Scott queue is a linked list, because a CAS can atomically swing a single `next` pointer — you cannot atomically `memmove` an array. See [lock-free concurrency](../../performance-optimization/lock-free-concurrency/learning.md).
- **Stable addresses.** When other code holds pointers into your elements, a `Vec` is disqualified — reallocation moves everything. A list (or an arena that never moves nodes) is the answer.
- **Teaching pointer discipline.** Genuinely valuable: the invariant reasoning transfers to trees, tries, and every arena-based structure later in this category.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`Vec` / `VecDeque`** | Default — including most cases where you were taught a list |
| Intrusive list | Objects must be on multiple lists; zero allocation per link; systems/embedded |
| Arena doubly-linked | Need O(1) unlink given a handle, and the handle comes from elsewhere (LRU) |
| `lru` crate | You are building an LRU — don't rewrite it |
| Lock-free linked queue | Concurrent producers/consumers, CAS-based progress |
| `Box` singly-linked | Learning; persistent/immutable stacks with shared tails |
| `std::collections::LinkedList` | Essentially never — no stable cursor API, so no splice |

## Pitfalls in Depth

### Pitfall: Choosing a list for "fast insertion" without owning a cursor

- **What goes wrong:** A collection sees frequent insertions and removals in the middle, so it becomes a linked list on the strength of "O(1) insert vs O(n) for `Vec`". Throughput drops. Profiling shows nearly all time in traversal — because every insertion is preceded by a search for the position.
- **Why it happens (the mechanism):** The O(1) claim is conditional on already holding the position, and that condition is almost never stated with the claim. Getting the position is Θ(n) at ~85 ns/hop; the `Vec::insert` it was supposed to beat is Θ(n) at `memmove` speed, roughly 100× faster per element. The list loses on the operation it was chosen for.
- **How to handle it in production, and why that works:** Ask "where does the cursor come from?" before choosing the structure. If the answer is "I search for it", use a `Vec`. If the answer is "a hash map hands it to me" or "the caller kept the handle from insertion", the list is legitimate — that's exactly the LRU shape.
- **Trade-offs of the fix:** Keeping a `HashMap<K, Handle>` alongside the list doubles the bookkeeping and adds a hash per operation, plus the two structures can drift out of sync (a removal from one and not the other). That's real complexity — worth it only when the O(1) unlink is on the hot path, as in an LRU under cache pressure.

### Pitfall: Recursive `Drop` stack overflow

- **What goes wrong:** A `Box`-based list is dropped and the process aborts with `fatal runtime error: stack overflow`. It's not a panic, can't be caught, and fires in a destructor — often at the end of a request, far from the code that built the list.
- **Why it happens (the mechanism):** The derived `Drop` for `Box<Node>` recurses once per link. Measured in release with the default 8 MB main-thread stack: fine at 250k nodes, **aborts at 300k**. Spawned threads default to 2 MB, so a worker thread dies at roughly a quarter of that — code that passes on the main thread crashes in a thread pool.
- **How to handle it in production, and why that works:** Write the iterative `Drop` shown above; it unlinks one node at a time at constant stack depth. Or use an arena, where dropping the list is dropping one `Vec` and recursion never enters the picture.
- **Trade-offs of the fix:** A manual `Drop` must be maintained as the node type changes, and it prevents destructuring the type by move. It's also easy to write one that silently leaks (forgetting to `take()` a field). Balanced trees don't need this; lists and degenerate trees always do.

### Pitfall: Doubly-linked pointer updates in the wrong order

- **What goes wrong:** An unlink or insert updates the four pointers in an order that reads one of them after it has already been overwritten. The list stays traversable forward and is corrupt backward (or vice versa), so tests that only iterate forward pass. The corruption surfaces much later as a lost element or an infinite loop.
- **Why it happens (the mechanism):** The `node.next.prev == node` invariant is mutual, so every edit temporarily breaks it, and the window has to be closed in a specific order. Add the boundary cases — inserting at head, at tail, into an empty list, removing the only element — and there are eight distinct paths, several of which are never exercised by a simple test.
- **How to handle it in production, and why that works:** Read the neighbours into locals *first*, then write (as `unlink` above does), so no read depends on a write. Then write a `#[cfg(test)] fn check_invariants(&self)` that walks forward and backward asserting mutual consistency and the element count, and call it after every operation in tests — ideally driven by `proptest` over random operation sequences. That converts a class of latent corruption into an immediate, minimal failing case.
- **Trade-offs of the fix:** The invariant checker is Θ(n) per call, so it must stay behind `cfg(test)` or a debug flag. Property tests on a doubly-linked list are slow to write. Both costs are small next to debugging a corrupted list in production, which is among the least pleasant debugging experiences available.

### Pitfall: `Rc<RefCell<Node>>` for the doubly-linked case

- **What goes wrong:** The obvious translation — `next: Option<Rc<RefCell<Node>>>`, `prev: Option<Rc<RefCell<Node>>>` — compiles and leaks every node, because each adjacent pair holds each other's refcount above zero. Fixing that with `Weak` for `prev` then produces `BorrowMutError` panics whenever a traversal holds a borrow while an edit re-enters.
- **Why it happens (the mechanism):** `Rc<RefCell>` is the closest syntactic analogue to a C pointer, so it's the first thing that compiles — but the doubly-linked invariant is *inherently* a reference cycle, which is the exact case reference counting cannot collect. The borrow checker didn't approve this design; it was switched off.
- **How to handle it in production, and why that works:** Use the arena. Indices are not references, so cycles are trivially expressible and cost nothing — no refcount traffic, no runtime borrow state, no leaks, and better locality because the nodes are contiguous.
- **Trade-offs of the fix:** The arena makes node lifetime your responsibility (free list, stale handles → generational indices), and the API hands out `(handle, &Arena)` pairs rather than owned node references, which is a heavier interface for callers. Accept it — it eliminates two whole bug classes.

### Pitfall: Benchmarking a list that was built sequentially

- **What goes wrong:** A microbenchmark builds a list by pushing n nodes in a loop, then times traversal, and reports the list as "only ~2× slower than `Vec`". The conclusion gets used to justify a design, and production is 50× worse than the benchmark predicted.
- **Why it happens (the mechanism):** A list built in one tight loop gets its nodes from the allocator in near-consecutive addresses, so traversal is nearly sequential and the prefetcher works. Measured here: **1.30 ns/element in allocation order vs 84.78 ns/element scattered** — a 65× difference between the same structure benchmarked two ways. Real lists are scattered: nodes are inserted and removed over time, interleaved with unrelated allocations.
- **How to handle it in production, and why that works:** Benchmark the *aged* structure — build it, then perform a realistic churn of random insertions and removals, then measure. Or explicitly build in shuffled order, as the measurement in this doc does. The number you want is the steady-state one, not the freshly-built one.
- **Trade-offs of the fix:** Aged benchmarks are slower to set up and noisier, and "realistic churn" is a judgment call you can get wrong in either direction. The discipline generalizes — it's the same representativeness problem as [profiling & measurement](../../performance-optimization/profiling-and-measurement/learning.md)'s warning about profiling the wrong workload.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if push returned a new list sharing the tail? | The immutable cons list — persistence is *free* for singly-linked, which is why functional languages use it |
| Batch it | What if nodes held k elements instead of 1? | The unrolled linked list — amortizes the pointer chase over k elements, recovering most of the array's locality |
| Approximate it | What if `prev` were only *usually* right? | XOR linked lists (`prev ^ next` in one word) — halves the links, forbids random entry |
| Randomize it | What if nodes had a random number of forward links? | **The skip list** — Θ(log n) search from a list, no rotations |
| Externalize it | What if `next` were a file offset? | On-disk free lists; LSM tree segment chains |
| Parallelize it | Where's the contention? | Michael-Scott queue: a single CAS on `next` — lists are the *natural* lock-free structure precisely because one pointer swings atomically |
| Invert it | What if traversal were O(1) and splice O(n)? | You've derived the array — and its 641× traversal advantage |
| Augment it | What does one more link per node buy? | Doubly-linked (backward traversal); a `jump` pointer (skip list); an index (order statistics) |
| Specialize it | What if nodes lived inside the elements? | Intrusive lists — zero allocation, one object on many lists at once |
| Amortize it | What if one operation could be terrible? | Periodic compaction: relink the list in memory order to restore locality |

**Questions:**

1. The unrolled linked list (k elements per node) sits between array and list. Derive the k that makes traversal cost within 2× of an array, using the measured 0.13 ns and 84.78 ns per element. What did you give up to get there?
2. Persistence is nearly free for a singly-linked list and expensive for an array. Explain why in terms of *which direction the pointers point*, then say what that implies about persistent queues.
3. A CAS can atomically swing one pointer but cannot atomically `memmove`. Explain why that single fact makes linked lists the default shape for lock-free structures despite the 641× traversal penalty.
4. Under "randomize it" you get a skip list — Θ(log n) search with no rotations. What did randomness buy that balancing code would otherwise have to provide, and what guarantee did it downgrade?
5. An XOR linked list stores `prev ^ next` in one word, halving the link overhead. Name the two capabilities it destroys, and explain why it's essentially unimplementable in safe Rust.
6. You have an LRU cache. Remove the `HashMap` and keep the list. Give the new complexity of `get`, and explain in one sentence why the entire design existed to avoid that.
7. Under "amortize it", periodic compaction restores locality. What does that make the list equivalent to, and at what point should you have just used that thing instead?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the full doubly-linked invariant, and say why it's the reason `Rc<RefCell>` leaks by default here.
2. "Insertion is O(1)." State the missing precondition, and give the actual cost when it isn't met.
3. Why does removing a node from a *singly*-linked list need the predecessor, and what's the standard trick to avoid needing it? When does the trick fail?
4. Give the measured ratios for `Vec` vs list traversal, built sequentially and built scattered. Explain the 65× gap between the two list numbers.
5. Name three workloads where a linked list genuinely beats `Vec`, and for each, name what supplies the cursor.
6. Why does `std::collections::LinkedList` fail to deliver the linked list's main advantage?

Build exercises:

- Implement the singly-linked list with `push_front`, `pop_front`, `remove_first`, `IntoIterator` (all three flavours), and the iterative `Drop`. Then build 1M nodes and drop it *without* the manual `Drop` to see the abort, and with it to see it work. That contrast is the whole lesson in one run.
- Implement the arena doubly-linked list, then build an LRU cache on top of it with a `HashMap<K, u32>`. Benchmark against the `lru` crate. This is the exercise that makes the "who supplies the cursor" argument concrete.
- Reproduce the 641× measurement: build a list sequentially, build another in shuffled allocation order, and time traversal of both against a `Vec`. Then write the one-paragraph explanation you'd give a colleague proposing a linked list for a hot read path.
- Derive the unrolled list: implement one with k = 16 elements per node and find where it lands between the two extremes. Compare against your answer to creative question 1.

## Open Questions

- What k makes an unrolled linked list match `VecDeque` for a queue workload on this machine — and at that k, is there any reason left to prefer it?
- Does `intrusive-collections` actually beat an arena-based list for the multi-list case, or does the `unsafe` buy less than the locality of a contiguous arena?
- Skip list vs `BTreeMap` in Rust for an ordered map: the skip list's advantage is concurrency, so is there a case for it single-threaded at all? (Revisit after Stage 4.)
- How much locality does periodic compaction of an arena-based list actually restore, and how long does it stay restored under realistic churn?
- Cursor APIs: what's the current stabilization status of `LinkedList::cursor_front_mut`, and does it change the "essentially never" verdict?

## References

- Aria Beingessner, [Learn Rust With Entirely Too Many Linked Lists](https://rust-unofficial.github.io/too-many-lists/) — the definitive treatment. Builds the list seven ways (`Box`, `Rc`, `RefCell`, arena, `unsafe`) and shows exactly where each representation breaks. Essential for this topic specifically.
- Bjarne Stroustrup, "Why you should avoid Linked Lists" — the talk that popularized measuring rather than reciting; the 641× number in this doc is the same experiment in Rust.
- [`std::collections::LinkedList` docs](https://doc.rust-lang.org/std/collections/struct.LinkedList.html) — read the opening paragraph, which recommends against itself and explains why.
- Michael & Scott, "Simple, Fast, and Practical Non-Blocking and Blocking Concurrent Queue Algorithms" (1996) — the lock-free linked queue; why lists are the natural concurrent shape.
- Linux kernel `include/linux/list.h` — intrusive lists done right; the macro trick for embedding links in arbitrary structs is worth understanding once.
- Related topics in this repo: [Arrays & Dynamic Arrays](../arrays-and-dynamic-arrays/learning.md) (the inverted trade), [Rust for Data Structures](../rust-for-data-structures/learning.md) (the five representations, and the 300k drop threshold), [Stacks & Queues](../stacks-and-queues/learning.md) (what you actually wanted, usually), [Cache Locality](../../performance-optimization/cache-locality/learning.md) (why the pointer chase costs 85 ns), [Lock-Free Concurrency](../../performance-optimization/lock-free-concurrency/learning.md) (where lists become unavoidable).
