# Arrays & Dynamic Arrays — Learning Notes

## Mental Model

**The array's feature is not indexing — it's contiguity.** O(1) random access is what textbooks lead with; it's the least valuable thing an array gives you. What actually decides outcomes is that element *i+1* sits 
immediately after element *i* in memory, which means:

- A scan moves at the speed of the memory bus, not the speed of pointer dereferences. Measured on this machine: summing a `Vec<u64>` of 5M elements runs at **0.13 ns/element**; the same 5M values in a heap-allocated linked list, chased in scattered address order, take **84.8 ns/element** — 641× slower for the same Θ(n) work.
- The hardware prefetcher can predict the next access and have it in L1 before you ask.
- One allocation holds everything, so building and freeing are single allocator calls, not n of them.
- The whole thing is `&[T]`, which is the interface every other Rust API speaks.

The dynamic array (`Vec<T>`) then answers "but I don't know n in advance" without giving contiguity up: **over-allocate, and when you run out, move everything to a bigger allocation.** Moving everything sounds catastrophic and isn't, because the growth is *geometric* — that's the entire trick, and the amortized argument in [complexity analysis](../complexity-analysis/learning.md) is its proof.

The working stance for this whole category: **`Vec<T>` is the default container, and departing from it requires a reason you can state.** Most "I need a fancier structure" instincts dissolve once you price the fancy structure's pointer chasing against a flat scan.

## The Invariant

A `Vec<T>` is three words — `(ptr, len, cap)` — maintaining:

> `cap ≥ len`; the elements at `ptr[0..len]` are **initialized and valid `T`**; the memory at `ptr[len..cap]` is allocated but **uninitialized**; and `ptr` points into a single allocation of `cap * size_of::<T>()` bytes with the alignment of `T`.

Three consequences worth pulling out:

- **`len` and `cap` are different questions.** `len` is how many values exist; `cap` is how much room was bought. `clear()` sets `len = 0` and leaves `cap` untouched — measured: a `Vec<u64>` grown to 1000 elements still reports `capacity() == 1000` after `clear()`, holding 8 KB. Only `shrink_to_fit()` releases it (capacity → 0).
- **Reallocation moves the data.** Any operation that grows past `cap` may return a *different* `ptr`. This is why Rust won't let you hold a `&T` into a `Vec` while pushing to it — that reference would dangle. The borrow checker isn't being pedantic; this is a genuine use-after-free in C++ and one of the most common bugs there.
- **The uninitialized tail is why `Vec` needs `unsafe` internally.** `push` writes to memory that holds no valid `T` yet. That's the whole reason `Vec` is a std type rather than something you'd casually reimplement.

## Mechanics

### Growth — the measured behaviour

`Vec` doubles, but the *first* capacity depends on element size (measured on this machine):

| Element | Capacity sequence |
| --- | --- |
| `Vec<u8>` | 8, 16, 32, 64, 128, 256, … |
| `Vec<u64>` | 4, 8, 16, 32, 64, 128, … |

The rule std uses: for a first allocation, elements of size 1 get capacity 8, elements up to 1024 bytes get 4, and larger get 1 — a heuristic to avoid a pointless 1-element allocation without over-committing for large types. After that it's strict doubling.

**Why geometric matters.** Growing by a *constant* (say +16) makes n pushes cost 1+2+…+n/16 = Θ(n²) copies. Growing by a *factor* makes the copies a geometric series summing to < 2n, hence Θ(1) amortized. Factor 2 is the common choice; 1.5 is used by some implementations because it can allow reusing previously freed blocks. Either works; constants do not.

**`with_capacity` is exact** — measured: `Vec::with_capacity(100).capacity() == 100`, not 128. So `with_capacity` is the tool for "I know n" and `reserve` for "I know I'm about to add k more." `reserve` may over-allocate (it applies the growth strategy); `reserve_exact` doesn't.

### The operations, and which ones are traps

| Operation | Cost | Note |
| --- | --- | --- |
| `push` / `pop` | O(1) amortized | The spike is real — see pitfalls |
| `v[i]`, `get(i)` | O(1) | Bounds-checked; `get_unchecked` is `unsafe` |
| `insert(i, x)` | O(n − i) | Shifts the tail |
| `remove(i)` | O(n − i) | Shifts the tail; `remove(0)` is the classic quadratic |
| `swap_remove(i)` | **O(1)** | Moves the last element into slot i — **destroys order** |
| `retain(pred)` | O(n) | One pass; the right way to filter in place |
| `drain(range)` | O(n) | Removes and yields; keeps capacity |
| `extend_from_slice` | O(k) amortized | Better than a `push` loop — one capacity check |
| `dedup` | O(n) | **Only removes *consecutive* duplicates** — sort first |
| `split_off(i)` | O(n − i) | Allocates a new `Vec` for the tail |
| `into_boxed_slice` | O(n) if cap > len | Drops capacity, yields `Box<[T]>` |
| `binary_search` | O(log n) | Requires sorted; returns `Result` — `Err(i)` is the insertion point |
| `sort` / `sort_unstable` | O(n log n) | Stable = merge-ish (allocates); unstable = pdqsort (in place, faster) |

`swap_remove` and `retain` are the two most under-used: together they cover nearly every case where people reach for `remove` in a loop.

### `Vec` vs the fixed-size relatives

| Type | Size of the handle | Storage | Use |
| --- | --- | --- | --- |
| `[T; N]` | N × size_of::<T> | inline/stack | N known at compile time |
| `&[T]` | 16 B (ptr + len) | borrowed | The universal interface — take this in function args |
| `Box<[T]>` | **16 B** | heap | Built once, never resized — saves 8 B vs `Vec` |
| `Vec<T>` | **24 B** | heap | Grows |
| `SmallVec<[T; N]>` | N inline + tag | inline until it spills | Usually-small collections, avoids allocation |

Two measured details: `Option<Vec<T>>` is also 24 bytes (the null-pointer niche means `None` is free), and `Vec<()>` reports `capacity() == usize::MAX` — zero-sized types never allocate, which is why `Vec<()>` is a legitimate (if odd) counter.

## Complexity

| Operation | Average | Worst | Amortized | Space |
| --- | --- | --- | --- | --- |
| Index | Θ(1) | Θ(1) | — | — |
| Push (back) | Θ(1) | Θ(n) *(realloc)* | **Θ(1)** | — |
| Pop (back) | Θ(1) | Θ(1) | — | — |
| Insert/remove at i | Θ(n − i) | Θ(n) | — | — |
| `swap_remove` | Θ(1) | Θ(1) | — | — |
| Search (unsorted) | Θ(n) | Θ(n) | — | — |
| Search (sorted) | Θ(log n) | Θ(log n) | — | — |
| Whole structure | — | — | — | Θ(cap), ≤ 2n after growth |

**Where the table misleads.** The Θ(n) scan and the Θ(log n) binary search are not comparable at face value: a linear scan is prefetched and branch-predictable, while a binary search does ~log₂ n *unpredictable* jumps. But the crossover is much earlier than folklore suggests — measured on this machine, `Vec<u32>` lookups (ns per lookup, ~50% hit rate):

| n | 8 | 16 | **32** | 128 | 1024 | 4096 |
| --- | --- | --- | --- | --- | --- | --- |
| Linear (`iter().any`) | **11.0** | **21.6** | 52.1 | 108.6 | 416.4 | 1040.9 |
| `binary_search` | 13.2 | 25.7 | **31.2** | **28.2** | **25.2** | **21.8** |

**Binary search wins from about n = 24.** The linear scan's advantage is real but narrow — it applies to tiny lookup tables, not to the "few hundred elements" the rule of thumb usually claims. What *does* survive at larger n is the sorted-`Vec`-versus-`BTreeMap`/`HashMap` comparison, which is about allocation and pointer chasing rather than about scanning. Measure your own crossover for your element type; this is the n₀ escape hatch from [complexity analysis](../complexity-analysis/learning.md) made concrete, and it cuts both ways.

**Memory overhead.** Right after a doubling, up to half the allocation is unused — a `Vec` holding n elements can occupy 2n slots. For a few large `Vec`s this is invisible; for a million small ones it's a doubling of your memory bill, and `into_boxed_slice`/`shrink_to_fit` is the fix.

## Rust Implementation

The idioms that matter in practice:

```rust
// Know the size → one allocation, no growth spikes, no realloc copies.
let mut v = Vec::with_capacity(n);

// Filter in place: one pass, no allocation, order preserved.
v.retain(|x| x.is_valid());

// Remove when order doesn't matter: O(1) instead of O(n).
let removed = v.swap_remove(i);

// Building from an iterator sizes correctly via size_hint — prefer it to a push loop.
let v: Vec<_> = (0..n).map(f).collect();

// Freeze after building: 24 B → 16 B per handle, capacity slack released.
let frozen: Box<[T]> = v.into_boxed_slice();

// Binary search returns the insertion point on miss — use it, don't recompute.
match v.binary_search(&key) {
    Ok(i) => v[i] = new,
    Err(i) => v.insert(i, new),
}

// dedup only removes CONSECUTIVE duplicates.
v.sort_unstable();
v.dedup();

// Take ownership of a range without dropping capacity.
let tail: Vec<_> = v.drain(k..).collect();
```

**Take `&[T]`, not `&Vec<T>`, in function signatures.** `&Vec<T>` accepts only a `Vec`; `&[T]` accepts arrays, boxed slices, and sub-ranges too, and costs nothing extra (deref coercion does the work at call sites). The same rule as `&str` over `&String`.

**Crates worth knowing:** `smallvec` / `tinyvec` (inline storage until N elements — a genuine win when most instances are tiny and there are many of them), `arrayvec` (fixed capacity, no heap at all, `no_std`-friendly), `bumpalo` (bump arena when many `Vec`s share a lifetime).

## Use Cases

- **Everything, by default.** Every `Vec` in a codebase that doesn't need to be something else is a small win banked.
- **Sorted `Vec` as a map.** For collections under a few hundred entries that are built once and read many times, a sorted `Vec<(K, V)>` beats `BTreeMap` and often `HashMap`: one allocation, perfect locality, binary search or even linear scan. This is what `phf`-style static maps and many compilers do.
- **Struct-of-arrays.** Splitting `Vec<Entity>` into `Vec<Position>` + `Vec<Velocity>` so a system touching only positions doesn't drag velocities through cache — the [data-oriented design](../../performance-optimization/data-oriented-design/learning.md) move, and it's only possible because arrays are contiguous.
- **Arenas.** The arena from [Rust for data structures](../rust-for-data-structures/learning.md) is a `Vec` with indices for links.
- **Ring buffers, heaps, hash tables, Fenwick trees.** All of these are arrays plus arithmetic — no nodes, no pointers, no borrow-checker friction.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`Vec<T>`** | Default. Grows, indexes, scans, hands out `&[T]`. |
| `Box<[T]>` | Built once, never resized, stored in bulk — saves 8 B/handle and the slack. |
| `[T; N]` | N known at compile time; want it on the stack. |
| `VecDeque<T>` | Need efficient push/pop at **both** ends — see [stacks & queues](../stacks-and-queues/learning.md). |
| `SmallVec<[T; N]>` | Many instances, usually tiny — trades a size increase for zero allocation. |
| `ArrayVec<T, N>` | Hard capacity bound, no allocator (embedded, hot paths). |
| Sorted `Vec` as a map | Small (< ~500), read-heavy, build-once. Beats hash maps on locality. |
| `HashMap`/`BTreeMap` | Keyed lookup at scale, or frequent insertion in the middle. |

## Pitfalls in Depth

### Pitfall: `remove(0)` and `insert(0, x)` in a loop

- **What goes wrong:** Draining a work list with `while !v.is_empty() { let job = v.remove(0); … }`, or building a list in reverse with `insert(0, x)`. Each call shifts every remaining element down or up by one, so the loop is Θ(n²). At 1,000 items it's 500k moves and invisible; at 100,000 it's 5 billion and the process appears hung.
- **Why it happens (the mechanism):** `remove(i)` must preserve contiguity and order, so it `memmove`s the tail. The cost is proportional to *the distance from the end*, which is maximal at index 0 — the exact index people reach for when they want a queue.
- **How to handle it in production, and why that works:** If it's a queue, use `VecDeque` — a ring buffer gives O(1) at both ends with the same contiguity benefits. If order is irrelevant, `pop()` from the back or `swap_remove(i)`. If you're building in reverse, push and then `reverse()` once (Θ(n) total instead of Θ(n²)). If you're filtering, `retain` or `drain`.
- **Trade-offs of the fix:** `VecDeque` isn't a single contiguous slice, so `as_slice()` doesn't exist and you get two slices from `as_slices()` — code that assumes contiguity (SIMD, FFI, `&[T]` APIs) needs `make_contiguous()`, which is Θ(n). `swap_remove` destroys ordering, which is fine right up until something downstream silently depended on it.

### Pitfall: The reallocation latency spike

- **What goes wrong:** A `Vec` accumulating request data grows to millions of elements. One unlucky `push` reallocates: allocate a block twice the size, `memcpy` everything, free the old block. At 4M `u64` that's a 32 MB copy — single-digit milliseconds, on a p99 budget that may be smaller than that. The average push time looks perfect.
- **Why it happens (the mechanism):** Amortization is an accounting argument, not a scheduling one. The copies really do happen all at once, and they get *more expensive as the vector grows* while becoming rarer — precisely the worst shape for tail latency, since the biggest spike arrives when the system is most loaded.
- **How to handle it in production, and why that works:** `with_capacity`/`reserve` when the size is known or boundable, which moves the entire cost to a moment you chose. When it isn't boundable, use a chunked structure (`VecDeque`, or a `Vec<Vec<T>>` of fixed-size blocks) that allocates a new block instead of relocating everything — growth becomes O(1) *worst case*, not just amortized.
- **Trade-offs of the fix:** Preallocation wastes memory when the estimate is high and doesn't help when it's low. Chunked storage gives up the single `&[T]`, and with it slicing, SIMD scans, and zero-copy FFI. This is a genuine latency-vs-simplicity trade — make it where the p99 is actually measured, not everywhere.

### Pitfall: Capacity that never comes back

- **What goes wrong:** A long-lived `Vec` (a buffer reused across requests, a cache) sees one huge input, grows to 500 MB, then `clear()` is called and it's reused forever at 100 elements. Measured: after growing to 1000 elements and calling `clear()`, capacity is still 1000. The memory is retained for the process lifetime; RSS shows a permanent step and the leak-hunt finds nothing, because it isn't a leak.
- **Why it happens (the mechanism):** `clear()` and `truncate()` drop the *elements* and set `len`; they deliberately never release the allocation, because reuse without reallocation is the point of a reusable buffer. It's the correct default for the common case and the wrong one for the outlier case.
- **How to handle it in production, and why that works:** For buffers reused across units of work, `shrink_to_fit()` (capacity → 0 when empty) or `shrink_to(reasonable_cap)` on a threshold — "if capacity > 10× typical, shrink." Better where it applies: don't reuse across a size boundary at all; let the big `Vec` drop at the end of the request that needed it.
- **Trade-offs of the fix:** Shrinking is a reallocation with a copy, so shrinking unconditionally on every iteration reintroduces the churn you avoided by reusing the buffer. Only shrink on a threshold, never in the steady-state path.

### Pitfall: `dedup` on unsorted data

- **What goes wrong:** `v.dedup()` is called to remove duplicates and quietly removes only *some* of them. `[1, 2, 1, 2]` stays `[1, 2, 1, 2]`. The bug survives review because the method name promises exactly what the caller wanted, and it survives testing because small fixtures happen to be sorted.
- **Why it happens (the mechanism):** `dedup` removes *consecutive* duplicates — it's a single pass with a one-element window, which is what makes it O(n) with no allocation and no `Hash`/`Ord` requirement beyond `PartialEq`. Removing all duplicates would need sorting or hashing, so std doesn't hide that cost inside a method that looks free.
- **How to handle it in production, and why that works:** `sort_unstable()` then `dedup()` for Θ(n log n) with no allocation and sorted output. Or a `HashSet`-based pass (`v.retain(|x| seen.insert(x.clone()))`) for Θ(n) expected while preserving first-occurrence order. Pick by whether you need order preserved or sorted output.
- **Trade-offs of the fix:** Sorting destroys the original order and requires `Ord`; the `HashSet` pass requires `Hash + Eq`, allocates, and clones keys. On small n, the naive O(n²) `retain` with a linear scan genuinely beats both — the [complexity analysis](../complexity-analysis/learning.md) crossover argument again.

### Pitfall: Reaching past `Vec` too early

- **What goes wrong:** A profile shows a hot linear scan, so the `Vec` becomes a `HashMap` or a `BTreeMap` — and gets *slower*. At a few hundred elements the hash cost, the pointer chase, and the loss of prefetching outweigh the asymptotic win, and the code is now harder to read.
- **Why it happens (the mechanism):** Θ(n) vs Θ(1) is a compelling argument that omits the constants. A contiguous scan of 200 `u32`s is 800 bytes — 13 cache lines, fully prefetched, a handful of nanoseconds. A `HashMap` lookup is a hash (~1 ns/byte with SipHash), a random probe into a table that isn't in cache, and a comparison.
- **How to handle it in production, and why that works:** Measure the crossover for your element type and access pattern before switching, and prefer the sorted-`Vec`-plus-binary-search middle ground for small, read-heavy, build-once collections — it keeps one allocation and full locality while getting the log factor.
- **Trade-offs of the fix:** A sorted `Vec` has Θ(n) insertion, so it's only right when writes are rare or batched (build, sort once, then read). If the collection can grow unboundedly or is write-heavy, the hash map's asymptotics do win and the crossover argument stops applying.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if push returned a new version sharing the old? | Persistent vectors: RRB-trees, `im::Vector` — O(log₃₂ n) "effectively constant" |
| Batch it | What if you appended 10,000 at once? | `extend_from_slice`; one capacity check instead of 10,000 |
| Approximate it | What if the length were approximate? | Ring buffer that overwrites — bounded telemetry buffers |
| Randomize it | What if the removal index were random? | `swap_remove` as uniform sampling-with-removal |
| Externalize it | What if it were a memory-mapped file? | Indices are position-independent; `Vec` of offsets serializes as-is |
| Parallelize it | Where's the contention? | `split_at_mut` / `par_chunks_mut` — arrays are the *ideal* parallel structure |
| Invert it | What if push were O(n) and insert-middle O(1)? | You've derived the linked list — and its 641× traversal cost |
| Augment it | What does one extra array buy? | Parallel arrays (SoA); an index array giving sorted order without moving data |
| Specialize it | What if T were 1 bit? | Bitset — 64× the density, and set operations become word-at-a-time |
| Amortize it | What if one push could be terrible? | Geometric growth — the dynamic array itself |

**Questions:**

1. `Vec` doubles. Why not grow by 1.5×, or by 4×? Argue both directions in terms of wasted memory versus number of copies, and say which one lets the allocator reuse previously freed blocks.
2. `swap_remove` is O(1) and `remove` is O(n), and they differ only in whether order is preserved. What does that tell you about the *cost of ordering* as a general principle? Name two other structures where the same trade appears.
3. Under the "invert it" lens you get a linked list. Given the measured 641× traversal gap, construct the workload where the linked list still wins — be precise about the ratio of traversals to splices.
4. A `Vec<u8>` starts at capacity 8 and a `Vec<u64>` at 4. Derive the rule std is using, and predict the first capacity for a 4 KB struct.
5. `Vec<()>` has capacity `usize::MAX`. Why is that *correct* rather than a bug, and what does it tell you about what `capacity` actually means?
6. Under "specialize it", a `Vec<bool>` could be a bitset at 1/8 the size. Rust's `Vec<bool>` is one byte per element anyway. Give the reason (hint: what would `&mut v[i]` return?), and name what you'd give up by switching to `bitvec`.
7. You need a collection that is append-only, read-heavy, and must never move its elements (other code holds references into it). `Vec` is disqualified — why, exactly, and what's the minimal change that fixes it?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the three-part `Vec` invariant, and explain which part is the reason you can't hold `&v[0]` across a `push`.
2. Give the total number of element copies for n pushes under doubling, and under a +16 constant growth. Name the series in each case.
3. When does `clear()` free memory? When does `truncate()`? What actually does?
4. You need to remove element i from a 10M-element `Vec` in a hot loop. Give three approaches and the condition under which each is correct.
5. Why is `&[T]` a better parameter type than `&Vec<T>`? Give two things it accepts that `&Vec<T>` doesn't.
6. `v.dedup()` left duplicates behind. Explain the mechanism in one sentence and give two fixes with different complexity and different guarantees.

Build exercises:

- Implement `MyVec<T>` from raw parts: `RawVec`-style allocation, `push`, `pop`, `Index`, `Drop`, and `Deref<Target=[T]>`. Then run it under `cargo +nightly miri test`. This is the single best exercise in Stage 1 — it forces the initialized/uninitialized boundary to become concrete, and it's the foundation for every array-backed structure later in this category.
- Reproduce the growth spike: push 10M `u64`, timing each push, and plot the distribution. Identify the reallocation spikes, verify they double in cost while halving in frequency, then re-run with `with_capacity` and report p50/p99/max both ways.
- Find your crossover: linear scan of a sorted `Vec<u32>` vs `binary_search` vs `HashSet::contains`, at n from 8 to 100,000. Plot all three. The two crossover points are numbers you'll reuse for the rest of this category — write them down.
- Measure the ordering tax: fill a `Vec` with 1M elements and remove half of them, once with `remove` at random indices and once with `swap_remove`. Predict the ratio from the asymptotics first, then explain the gap.

## Open Questions

- The `u32` crossover is measured (~24). Still open: the same number for `u64`, for a 32-byte struct, and for `String` keys where comparison itself is Θ(k).
- Does `smallvec` actually win for the typical "0–3 elements, millions of instances" case here, or does the larger inline size hurt more than the avoided allocation helps? Measure against `Vec` and `Box<[T]>`.
- How much do bounds checks cost on a hot scan in practice, given that LLVM elides most of them? Compare an indexed loop, an iterator loop, and `get_unchecked`.
- `shrink_to_fit` on a 500 MB `Vec` — does the allocator actually return the pages to the OS, or does RSS stay flat? Test with the system allocator and with `mimalloc`/`jemalloc`.

## References

- [`std::vec::Vec` documentation](https://doc.rust-lang.org/std/vec/struct.Vec.html) — the "Guarantees" section is a precise statement of the invariant above and is worth reading in full once.
- Aria Beingessner, [The Rustonomicon: Implementing Vec](https://doc.rust-lang.org/nomicon/vec/vec.html) — builds `Vec` from raw allocation upward; the reference implementation for the build exercise.
- Bjarne Stroustrup, "Are lists evil?" / the vector-vs-list talks — the original empirical demolition of the "linked lists are good for insertion" intuition; the numbers in this doc reproduce it in Rust.
- Rust std source for `RawVec::grow_amortized` — the actual growth policy, including the size-dependent first capacity measured above.
- Related topics in this repo: [Complexity Analysis](../complexity-analysis/learning.md) (the amortization proof and the crossover argument), [Linked Lists](../linked-lists/learning.md) (the inverted trade, and the 641× number in context), [Stacks & Queues](../stacks-and-queues/learning.md) (`VecDeque` and the ring buffer), [Cache Locality](../../performance-optimization/cache-locality/learning.md) + [Memory Layout](../../performance-optimization/memory-layout/learning.md) (why contiguity is the real feature), [Data-Oriented Design](../../performance-optimization/data-oriented-design/learning.md) (SoA as the array idea at program scale).
