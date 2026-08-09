# Hash Tables — Learning Notes

## Mental Model

**A hash table is an array you index by *computing* the position instead of being told it.** That's the whole idea: `bucket = hash(key) % capacity` converts an arbitrary key into an array index, so lookup becomes an array access — Θ(1) — instead of a search.

Everything else in the topic is consequence management for the two problems that creates:

1. **Collisions are not an edge case, they're the normal case.** You're compressing an enormous key space into a small index space, so collisions are guaranteed by pigeonhole. Worse, they arrive far earlier than intuition suggests: by the birthday paradox, a table with 1 million slots sees its first collision after about **1,200** insertions. Collision *resolution* is the actual design of a hash table.
2. **The Θ(1) is a statistical claim, not a structural one.** It holds when the hash spreads keys evenly. Nothing in the data structure guarantees that — the hash function does, and if an adversary controls the keys they control the distribution. Measured below: with every key colliding, inserting 16,000 entries goes from 0.69 ms to **127 ms — 186× slower**, and it degrades quadratically.

So the honest framing:

> A hash table trades **space** (you keep the array bigger than the data) and a **hash computation** for the ability to skip searching. Its guarantee is *expected* Θ(1), and it holds exactly as long as your hash function's uniformity does.

The practical corollary, which surprises people: **the hash table's asymptotic advantage arrives much earlier than folklore claims.** Measured on this machine, `HashSet::contains` overtakes a linear scan of a `Vec<u32>` at about **n = 12**, and `HashMap` beats a sorted `Vec` + binary search from **n = 32** — with the gap *widening* out of cache. The "contiguity wins for small collections" instinct is calibrated far too high.

## The Invariant

> Every key present in the table is reachable from `hash(key)` by following the probe sequence, without encountering an "empty" marker first. The number of occupied slots never exceeds `load_factor × buckets`.

Three consequences that explain most hash-table behaviour:

- **Deletion cannot simply blank a slot.** If key A probed past slot 7 to land at slot 8, and you empty slot 7, the probe for A now stops at the empty slot and reports "not found." Open-addressed tables therefore use **tombstones** (a "deleted, keep probing" marker) or **backward-shift deletion** (move a later element into the hole). This is the single most common bug in a hand-written hash table.
- **Exceeding the load factor forces a full rehash.** Every key must be re-placed, because the bucket index depends on the capacity. That's an Θ(n) operation, and it's why growth costs far more here than for a `Vec` (measured below: **2.67×** the total insert time when you don't preallocate — versus a `memcpy` for `Vec`).
- **The invariant says nothing about order.** Iteration order is whatever the bucket layout happens to be, and Rust deliberately randomizes it.

## Mechanics

### Collision resolution — the actual design decision

| Strategy | How | Pros | Cons |
| --- | --- | --- | --- |
| **Separate chaining** | Each bucket holds a list/vec of entries | Simple; tolerates load factor > 1; easy deletion | A pointer chase per collision; an allocation per bucket; poor locality |
| **Linear probing** | On collision, try slot+1, slot+2, … | Excellent locality — probes stay in the same cache line | **Primary clustering**: runs merge and grow |
| Quadratic probing | slot+1, slot+4, slot+9, … | Breaks up clustering | Worse locality; may not visit all slots |
| Double hashing | Step size from a second hash | Best theoretical distribution | Two hashes; poor locality |
| **Robin Hood** | On probe, the entry with the greater distance-from-home wins the slot | Bounds the variance of probe length | More work per insert |

Modern tables use **linear probing** despite clustering, because on real hardware locality beats probe count — the same lesson as arrays-vs-linked-lists, one level down.

### SwissTable — what Rust's `HashMap` actually is

Rust's `HashMap` is `hashbrown`, a port of Google's SwissTable. The key idea is a **separate control byte array** alongside the entries:

- The 64-bit hash splits into **h1** (which bucket group) and **h2** (a 7-bit tag stored in the control byte).
- A probe loads **16 control bytes at once** and uses SIMD (`_mm_cmpeq_epi8` on x86, NEON equivalents on ARM) to compare all 16 tags against h2 in one instruction, producing a bitmask of candidate matches.
- Only candidates that match the 7-bit tag are compared for real key equality.

That's why the constant factor is so good: the common case touches one cache line of control bytes and does one vector comparison, rejecting 16 slots at a time without touching the (much larger) key/value array. It's a good example of a data structure designed around the memory hierarchy rather than around operation counts.

### Load factor and growth — measured

`HashMap<u64,u64>`, this machine — capacity reported after each resize:

| Resize triggered at `len` | 1 | 4 | 8 | 15 | 29 | 57 | 113 | 225 | 449 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| New `capacity()` | 3 | 7 | 14 | 28 | 56 | 112 | 224 | 448 | 896 |

The bucket arrays are powers of two (4, 8, 16, 32, …) and the reported capacity is **7/8 of that** — SwissTable's load factor. Two things follow:

- **`HashMap::with_capacity(100)` reports capacity 112**, not 100 — it rounds up to 128 buckets so 100 entries fit under the load factor. Contrast `Vec::with_capacity(100)`, which is exactly 100. A hash table always keeps slack; that slack *is* the mechanism.
- Growth doubles, so the amortized argument from [complexity analysis](../complexity-analysis/learning.md) applies — but each resize **rehashes every key**, which is much more expensive than `Vec`'s `memcpy`.

**Preallocation matters more than for `Vec`** (inserting 1M `u64` keys):

| | Time |
| --- | --- |
| `HashMap::new()`, grown | 93.5 ms |
| `HashMap::with_capacity(1_000_000)` | **35.0 ms** |
| | **2.67× faster** |

### Iteration order is random *per map instance*

```
map1 = [6, 1, 3, 4, 7, 5, 0, 2]
map2 = [2, 1, 5, 7, 3, 0, 4, 6]     // identical contents, same process, same run
```

Both maps were built from `(0..8)` in the same process. `RandomState` draws a **fresh seed per `HashMap` instance** (from a thread-local seeded once per thread), so it isn't merely "random per process" — two maps in the same function iterate differently. Any logic that depends on iteration order is not just non-reproducible across runs, it's non-reproducible across *maps*.

### The `Entry` API — one hash instead of two

`contains_key` then `get_mut`/`insert` hashes the key twice and probes twice. `entry()` does it once:

| Counting 1M keys over a 50k key space | Time |
| --- | --- |
| `contains_key` + `get_mut`/`insert` | 28.1 ms |
| `*map.entry(k).or_insert(0) += 1` | **20.3 ms** |
| | **1.38× faster** |

Less dramatic than it looks in theory (the second probe hits a warm cache line), but it's free to adopt and it's also less code.

## Complexity

| Operation | Average | **Worst** | Amortized | Space |
| --- | --- | --- | --- | --- |
| Lookup | Θ(1) | **Θ(n)** | — | — |
| Insert | Θ(1) | Θ(n) | Θ(1) | — |
| Delete | Θ(1) | Θ(n) | — | — |
| Iterate | Θ(capacity) | Θ(capacity) | — | — |
| Whole structure | — | — | — | Θ(n/α) ≈ 1.14n entries + control bytes |

**Where the table misleads:**

- **The Θ(n) worst case is reachable on purpose.** It's not a theoretical footnote — it's an attack. See the pitfall below.
- **Iteration is Θ(capacity), not Θ(n).** A map that held 10M entries and now holds 3 still iterates 10M slots, because capacity never shrinks. `shrink_to_fit` exists for this.
- **Θ(1) hides the key length.** `HashMap<String, V>` is Θ(1) in the *number of entries* and Θ(k) in the key's length — you hash k bytes and, on a hit, compare k bytes. Interning keys to `u32` is what makes it genuinely Θ(1).
- **The constant is not small.** Measured: ~8 ns per `u32` lookup cache-resident, rising to ~35 ns at 10M entries where the probe misses cache. Compared against a ~1 ns L1 array access, the hash table's "Θ(1)" is roughly 8–35 array accesses.

## Rust Implementation

```rust
use std::collections::HashMap;

// Preallocate — 2.67× measured on a 1M-insert workload.
let mut m: HashMap<u64, Stats> = HashMap::with_capacity(expected);

// One hash, not two.
*m.entry(key).or_insert(0) += 1;
m.entry(key).or_insert_with(Vec::new).push(item);      // lazy default
m.entry(key).and_modify(|v| v.hits += 1).or_insert(Stats::new());

// Borrow lets you look up by the borrowed form — no allocation to query.
let m: HashMap<String, V> = ...;
m.get("literal");                                       // works: String: Borrow<str>

// Deterministic output: sort on the way out, never rely on iteration order.
let mut entries: Vec<_> = m.iter().collect();
entries.sort_unstable_by_key(|(k, _)| *k);

// Hasher choice is PER MAP, by key provenance.
use std::hash::BuildHasherDefault;
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<rustc_hash::FxHasher>>;
let internal: FastMap<NodeId, Node> = FastMap::default();   // self-generated keys → 4.6–6.0× faster
let external: HashMap<String, Session> = HashMap::new();    // user input → keep SipHash

// Capacity never shrinks on its own.
m.retain(|_, v| v.is_live());
m.shrink_to_fit();
```

**Neighbours worth knowing:**

| Need | Use |
| --- | --- |
| Insertion-order iteration | `indexmap::IndexMap` — order-preserving, still Θ(1) |
| Concurrent access | `dashmap` (sharded locks), or `Mutex<HashMap>` when contention is low |
| Small maps built once | `phf` (perfect hash, compile-time), or a sorted `Vec` for the ordering |
| Bounded cache | `lru`, `moka` |
| Faster hasher | `rustc-hash` (FxHash), `ahash` (fast *and* DoS-resistant) |

## Use Cases

- **Deduplication and membership** — the default answer above n ≈ 12.
- **Counting and grouping** — `entry().or_insert(0) += 1` is the canonical Rust one-liner.
- **Indexing by ID** — the arena's companion: `HashMap<ExternalId, Handle>` mapping the outside world's keys to internal indices.
- **Caches** — with an eviction policy layered on, which is where the [LRU](../linked-lists/learning.md) design comes from.
- **Interning** — `HashMap<String, u32>` built once at the boundary so everything internal compares integers.
- **Joins and set operations** — hash join is the database technique that turns a Θ(n·m) nested loop into Θ(n + m).
- **Memoization** — cache the result of an expensive pure function keyed by its arguments.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`HashMap`/`HashSet`** | Default for keyed lookup above n ≈ 12–32 |
| `BTreeMap`/`BTreeSet` | Need ordered iteration, range queries, or a deterministic order |
| Sorted `Vec` + `binary_search` | Want compact footprint, one allocation, ordered iteration — **not** lookup speed |
| Linear scan of a `Vec` | n below ~12, or you iterate far more than you look up |
| Direct array indexing | Keys are dense small integers — no hashing at all, genuinely Θ(1) |
| `IndexMap` | Need `HashMap` speed *and* insertion order |
| `phf` | Fixed key set known at compile time |
| Arena + `u32` handles | The keys are yours to assign — skip the map entirely |

## Pitfalls in Depth

### Pitfall: Hash flooding (HashDoS)

- **What goes wrong:** An attacker submits keys chosen to collide — form fields, JSON object keys, HTTP headers, cache keys. Every insertion probes the entire occupied run, so building the map becomes Θ(n²). Measured with a deliberately colliding hasher:

  | n inserted | Normal | All colliding | Ratio |
  | --- | --- | --- | --- |
  | 1,000 | 119.9 µs | 499.3 µs | 4× |
  | 4,000 | 165.0 µs | 7.64 ms | 46× |
  | 16,000 | 686.0 µs | **127.4 ms** | **186×** |

  Note the *shape*: 16× more keys, 255× more time — quadratic. A single request with a few tens of thousands of crafted keys occupies a core for seconds.
- **Why it happens (the mechanism):** Θ(1) is an average over a *random* hash function. If the hasher is deterministic and public, an attacker computes colliding keys offline once and reuses them forever. The 2011 disclosure broke PHP, Java, Python, Ruby, ASP.NET and Node simultaneously for exactly this reason.
- **How to handle it in production, and why that works:** Rust's default already fixes it — `RandomState` seeds SipHash-1-3 with a per-instance random key, so an attacker cannot precompute collisions without knowing a secret that differs per map. **The pitfall is therefore not "use the default" but "don't swap it away on the wrong map."** Classify each map by key provenance: keys you generate (indices, interned IDs, enum discriminants) can use `FxHashMap`; anything reachable from user input, the network, or files keeps the default. `ahash` is the middle path — much faster than SipHash while retaining DoS resistance.
- **Trade-offs of the fix:** SipHash costs real time, and the size dependence is stark. Measured, swapping to an Fx-style hasher is **4.6–6.0× faster for `u32` keys** but only **1.19–1.29× for 16-char `String` keys** — for short keys the hash *is* the lookup, for longer ones it's amortized against comparison and memory access. So the security cost you're paying is largest exactly where the DoS risk is usually smallest (internal integer keys), which is a happy accident worth exploiting deliberately.

### Pitfall: Depending on iteration order

- **What goes wrong:** Code collects a map's entries into a `Vec` and writes them to a file, returns them in an API response, or hashes them for a cache key. Output differs between runs — sometimes between two maps in the same run. Tests are flaky, diffs are noisy, caches miss, and "reproducible builds" aren't.
- **Why it happens (the mechanism):** Order is a side effect of bucket layout, which depends on the seed. Rust draws a **fresh seed per `HashMap` instance**, so this is stronger than "random per process." Measured: two maps built from `(0..8)` in the same function iterated as `[6,1,3,4,7,5,0,2]` and `[2,1,5,7,3,0,4,6]`. And crucially, order is *stable enough within one map* that a test can pass a hundred times before failing.
- **How to handle it in production, and why that works:** Sort explicitly whenever order is observable — `let mut v: Vec<_> = m.iter().collect(); v.sort_unstable_by_key(...)`. If insertion order is the semantic you want, use `IndexMap`, which preserves it by design. If sorted order is what you want, use `BTreeMap` and stop paying for sorting on every read.
- **Trade-offs of the fix:** Sorting on the way out is Θ(n log n) per call, which is wasteful if it happens on a hot path — that's the signal to switch container rather than sort repeatedly. `IndexMap` costs extra memory (it keeps an index vector) and `BTreeMap` gives up the Θ(1) lookup. The randomization itself is not a misfeature to work around; it's the DoS defence.

### Pitfall: Breaking the `Hash`/`Eq` contract

- **What goes wrong:** `a == b` but `hash(a) != hash(b)`. Entries become unfindable — you insert a key and `get` with an equal key returns `None`. A `HashSet` contains two "equal" elements. The bug is silent and looks like memory corruption.
- **Why it happens (the mechanism):** The contract is a *semantic* requirement the compiler cannot check. The usual causes: deriving `Hash` but hand-writing `PartialEq` (or vice versa) so they consider different fields; a cached or interior-mutable field that participates in `Hash` but not `Eq`; or **mutating a key after insertion** through interior mutability — the entry stays in the bucket its old hash chose, and nothing will ever probe there again.
- **How to handle it in production, and why that works:** Derive `PartialEq`, `Eq`, and `Hash` together so they cannot drift. When you must hand-write them, write both and property-test the law directly: `a == b ⟹ hash(a) == hash(b)` over random pairs — three lines of `proptest` that catch every instance. Never put a `Cell`/`RefCell`/`Mutex` field inside a key type.
- **Trade-offs of the fix:** Deriving forces every field to participate, which is wrong when a field is a cache or a timestamp that shouldn't affect identity. That's precisely when to hand-write both and test them — the cost is the test, and it's small next to the debugging.

### Pitfall: Not preallocating, and never shrinking

- **What goes wrong:** Two opposite failures. Building a large map without `with_capacity` pays repeated full rehashes — measured **2.67×** on a 1M-key insert (93.5 ms vs 35.0 ms). And a long-lived map that once held 10M entries and now holds 3 still occupies the 10M-slot allocation *and iterates all of it*, because capacity never shrinks on its own.
- **Why it happens (the mechanism):** Each resize rehashes every key — Θ(n) work with a random access pattern, much worse than `Vec`'s sequential `memcpy`, which is why the penalty here is larger than the array case. On the other side, `remove` and `retain` reduce `len` but deliberately keep the allocation for reuse.
- **How to handle it in production, and why that works:** `with_capacity` whenever the size is known or boundable — it converts n rehashes into zero. For long-lived maps that shrink, call `shrink_to_fit()` (or `shrink_to(reasonable)`) after a bulk removal, on a threshold rather than unconditionally.
- **Trade-offs of the fix:** `with_capacity` over-commits when the estimate is high, and a hash table already carries 8/7 slack by design, so over-estimating wastes more memory here than it would for a `Vec`. Shrinking is itself a full rehash, so shrinking in a steady-state loop reintroduces exactly the cost you were avoiding.

### Pitfall: `String` keys where an integer would do

- **What goes wrong:** `HashMap<String, V>` keyed by paths, user IDs, or symbol names in a hot loop. Every lookup hashes the whole string and, on a hit, compares it byte for byte. Additionally, constructing a `String` key just to query allocates. The map is Θ(1) in entries and Θ(k) in key length, and k dominates.
- **Why it happens (the mechanism):** The RAM model's unit-cost hashing assumption is false for variable-length keys. The effect is visible in the hasher measurement: for 16-char `String` keys, switching to a 6×-faster hash function bought only **1.19–1.29×** overall, because the hash was never the dominant cost — the memory access and the comparison were.
- **How to handle it in production, and why that works:** **Intern at the boundary.** Convert each distinct string to a `u32` once (`HashMap<String, u32>` plus a `Vec<String>` back), then key everything internal by `u32`. Comparisons become one instruction, hashing becomes trivial, and the Fx-hasher swap now buys the full 4.6–6.0×. This is what `rustc`'s `Symbol` is, and it composes with the arena representation from [Rust for data structures](../rust-for-data-structures/learning.md). Where interning is overkill, at least use `Borrow` so you can query with `&str` without allocating.
- **Trade-offs of the fix:** An intern table is bidirectional state that must be threaded through the program, and IDs are meaningless in logs without a reverse lookup. In a long-lived process interning *unbounded* distinct strings is a memory leak by another name — it's right for a bounded symbol space (identifiers, field names, enum-like values), wrong for arbitrary user text.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if insert returned a new map sharing structure? | **HAMT** — hash array mapped trie; `im`/`rpds`; the basis of Clojure/Scala maps |
| Batch it | What if you inserted a million at once? | Presize then insert; or sort-by-bucket first for locality; hash join's build phase |
| Approximate it | What if membership could be wrong 1% of the time? | **Bloom filter** — Θ(1) bits per element instead of storing keys at all (Stage 8) |
| Randomize it | What if the hash were secretly keyed? | `RandomState` — exactly this, and it's why HashDoS doesn't work on Rust |
| Externalize it | What if buckets were disk pages? | **Extendible / linear hashing** — used by on-disk hash indexes |
| Parallelize it | Where's the contention? | Shard by `hash(key) % shards` and lock per shard — `dashmap`; the same partitioning idea as [sharding](../../architecture-patterns/sharding/learning.md) |
| Invert it | What if the key set were fixed and known? | **Perfect hashing** — zero collisions, no probing (`phf`) |
| Augment it | What does one more array buy? | SwissTable's control bytes: 1 byte/slot buys a 16-way SIMD probe; `IndexMap`'s order vector buys iteration order |
| Specialize it | What if keys were dense small integers? | Drop the hash — direct array indexing. Θ(1) with a constant of 1 |
| Amortize it | What if one insert could be terrible? | Doubling + full rehash; or **incremental resizing** — keep both tables and migrate k entries per operation |

**Questions:**

1. With 1M slots, the first collision appears around 1,200 insertions. Derive that, then explain why it means collision *resolution*, not collision *avoidance*, is the real design problem.
2. Deleting from an open-addressed table can't just blank the slot. Construct a three-key example that breaks, then give both standard fixes and say which one costs you on iteration.
3. SwissTable spends one extra byte per slot on control bytes. Under which lens is that, and what exactly does that byte buy that justifies ~14% more memory?
4. Rust seeds each `HashMap` *instance* separately, not just each process. What attack does per-instance seeding stop that per-process seeding wouldn't?
5. Under "amortize it", incremental resizing spreads the rehash across operations. What does it cost, and why do most general-purpose libraries not do it while some real-time systems must?
6. A Bloom filter answers membership in Θ(1) bits per element without storing keys. What did it give up, and why is the error one-sided (false positives but never false negatives)?
7. The measured Fx-vs-SipHash win is 6× for `u32` and 1.2× for 16-char strings. Derive the key length at which you'd expect the win to halve, then say what that implies about where to spend your interning effort.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the hash-table invariant and use it to explain why tombstones exist.
2. Why does `HashMap::with_capacity(100)` report 112 while `Vec::with_capacity(100)` reports exactly 100?
3. Give the measured HashDoS numbers at n = 1,000/4,000/16,000 and identify the complexity class from the shape.
4. Two `HashMap`s with identical contents, built in the same function, iterate differently. Why — and name a bug this has caused in code you've seen.
5. When is `FxHashMap` correct and when is it a vulnerability? Give the classification rule in one sentence.
6. `HashMap<String, V>` lookup is "Θ(1)". Give two independent reasons that's misleading and the fix for each.

Build exercises:

- Implement an open-addressed hash table with linear probing: `insert`, `get`, `remove` (with tombstones), and resize-on-load-factor. Then write the test that fails without tombstones — insert three colliding keys, remove the middle one, and look up the third. That failing test is the invariant made concrete.
- Reproduce the HashDoS measurement with your own colliding hasher, then plot insertion time against n and confirm the quadratic. Then re-run with `RandomState` and with `ahash` to see the defence work.
- Implement backward-shift deletion instead of tombstones, and measure both on a delete-heavy workload — the trade-off between them is a real one and the numbers make it stick.
- Build an interning layer (`HashMap<String,u32>` + `Vec<String>`) and measure a realistic symbol-heavy workload keyed by `String` vs by interned `u32`, with both hashers. You should see the Fx win go from ~1.2× to ~5×.

## Open Questions

- Where exactly does `ahash` land between SipHash and Fx for `u32` and `String` keys on this machine? If it's close to Fx, the "classify every map" advice mostly collapses into "use ahash everywhere."
- `dashmap` vs sharded `Mutex<HashMap>` vs a single `RwLock<HashMap>` under realistic read/write mixes — at what contention level does each win?
- How much does `HashMap` iteration cost when capacity greatly exceeds len? Measure a map that held 10M and now holds 100.
- Does `IndexMap` actually cost meaningfully more than `HashMap` for lookups, or is the order vector nearly free?
- Robin Hood vs SwissTable: is there a workload where the bounded-variance guarantee beats the SIMD probe, or is SwissTable strictly better in practice now?

## References

- [SwissTable / `hashbrown`](https://github.com/rust-lang/hashbrown) — the implementation behind `HashMap`; the README explains the control-byte and SIMD-probe design.
- Matt Kulukundis, "Designing a Fast, Efficient, Cache-friendly Hash Table, Step by Step" (CppCon 2017) — the original SwissTable talk; the clearest explanation of why the control bytes pay for themselves.
- Crosby & Wallach, "Denial of Service via Algorithmic Complexity Attacks" (2003) — the paper that named hash flooding, eight years before the ecosystem-wide 2011 disclosure.
- Aumasson & Bernstein, "SipHash: a fast short-input PRF" (2012) — the function Rust adopted and why a *keyed* hash is the right defence.
- CLRS ch. 11 — chaining, open addressing, and the universal-hashing analysis behind the expected Θ(1) claim.
- Related in this repo: [Hashing Techniques](../hashing-techniques/learning.md) (the function itself, in depth), [Arrays & Dynamic Arrays](../arrays-and-dynamic-arrays/learning.md) (the measured crossovers), [Complexity Analysis](../complexity-analysis/learning.md) (expected vs average, and why that distinction is a security property), [Cache Locality](../../performance-optimization/cache-locality/learning.md) (why linear probing beats chaining), [Sharding](../../architecture-patterns/sharding/learning.md) (the same partitioning idea one scale up).
