# Hashing Techniques — Learning Notes

## Mental Model

**A hash function compresses arbitrary data into a fixed number of bits, destroying information on purpose.** Because it destroys information, collisions are inevitable; because it's deterministic, the same input always lands in the same place. Those two properties together are what make it useful — and every technique in this topic is an application of one or the other.

The thing to stop doing is treating "hash function" as one concept. There are at least **five different jobs**, with incompatible requirements:

| Job | Needs | Must NOT need | Example |
| --- | --- | --- | --- |
| Hash-table index | Uniformity, speed | Security (unless keys are hostile) | FxHash, aHash |
| DoS-resistant table index | Uniformity, speed, **keyed** | Full cryptographic strength | SipHash-1-3 |
| Integrity / dedup | Collision resistance | Speed | BLAKE3, SHA-256 |
| Password storage | **Slowness**, memory-hardness, salting | Speed (speed is the vulnerability) | Argon2, bcrypt |
| Sharding / placement | Uniformity, **stability across runs** | Security | xxHash, consistent hashing |

Using the wrong one is a real failure mode in both directions: SHA-256 for a hash map wastes an order of magnitude; a fast non-cryptographic hash for deduplicating user uploads is forgeable; and *any* general-purpose hash for passwords is a breach waiting to be published.

The second idea worth carrying: **the "expected Θ(1)" of a hash table is a property of the hash function, not of the table.** The table's structure is an array. The uniformity that makes lookups constant-time is entirely borrowed from the function, which is why [hash tables](../hash-tables/learning.md) and this topic are really one subject split in two.

## The Invariant

The contract every hash implementation must satisfy, and the one Rust states explicitly:

> **`a == b` ⟹ `hash(a) == hash(b)`.** Equal values must hash equally.

Note what is *not* required: unequal values may hash equally (that's a collision, and it's fine — equality resolves it). The implication runs one way only.

Two additional properties that aren't required but that you almost always want:

- **Avalanche.** Flipping one input bit should flip about half the output bits, independently. Without it, similar keys produce similar hashes, which produces clustering, which produces long probe sequences. This is why a hash function is not just "mix the bytes somehow."
- **Determinism within a run** — but explicitly **not** across runs. Rust's `RandomState` reseeds per `HashMap` instance, so hash values are not stable between two maps in the same process, let alone between processes or versions. **Never persist a hash value produced by a `Hasher` whose stability isn't documented.**

## Mechanics

### Cost, measured

Hashing a byte slice, this machine (`RandomState`'s SipHash-1-3 vs an Fx-style multiply-rotate hasher):

| Key bytes | SipHash (ns) | Fx-style (ns) | Ratio | SipHash ns/byte |
| --- | --- | --- | --- | --- |
| 4 | 12.17 | 2.35 | 5.18× | 3.043 |
| 8 | 7.25 | 2.39 | 3.03× | 0.906 |
| 16 | 21.08 | 3.18 | **6.63×** | 1.317 |
| 32 | 26.87 | 4.70 | 5.72× | 0.840 |
| 64 | 41.43 | 7.48 | 5.54× | 0.647 |
| 128 | 67.73 | 14.74 | 4.59× | 0.529 |
| 256 | 107.60 | 26.78 | 4.02× | 0.420 |
| 1024 | 321.29 | 133.36 | 2.41× | 0.314 |

Read the last column: **SipHash's per-byte cost falls from 3.0 to 0.31 ns/byte as keys grow**, which means it has a large *fixed* cost (setup and finalization) that dominates short keys. Asymptotically it runs at roughly 3.2 GB/s. The common "SipHash costs about 1 ns/byte" claim is wrong at both ends — it's ~3 ns/byte for a 4-byte key and ~0.3 for a long one.

The Fx advantage shrinks with key length (6.6× at 16 bytes → 2.4× at 1 KB), because both functions converge toward being memory-bandwidth bound. That's the quantitative reason the hasher swap matters most for small integer keys and barely for long strings, which matches the end-to-end map measurements in [hash tables](../hash-tables/learning.md) (4.6–6.0× for `u32`, 1.19–1.29× for 16-char `String`).

*(The 8-byte row being faster than the 4-byte row is a specialization artifact — a fast path for word-sized writes — not a trend; it's a good reminder that microbenchmarks of tiny operations pick up implementation details.)*

### The families

- **Multiply-shift / multiply-rotate** (FxHash, `rustc-hash`). One multiply and one rotate per word. No avalanche guarantee in the low bits, trivially reversible, zero security — and the fastest thing available. Correct for keys you generate.
- **Fast general-purpose** (xxHash, aHash, wyhash, FNV). Good avalanche, high throughput. `aHash` is keyed and uses AES instructions where available, so it's *both* fast and DoS-resistant — often the right default when you'd otherwise be choosing.
- **Keyed PRFs** (SipHash). Designed specifically for hash-table DoS resistance: fast on short inputs, and without the key an attacker cannot construct collisions. Rust's default.
- **Cryptographic** (SHA-256, BLAKE3). Collision resistance you can rely on against an adversary. BLAKE3 is fast (SIMD, parallelizable) but still ~an order of magnitude off a non-cryptographic hash for short keys. Use for content addressing, integrity, dedup of untrusted data.
- **Password hashes** (Argon2, scrypt, bcrypt). Deliberately slow and memory-hard, always salted. A different category entirely — using SHA-256 here is a vulnerability, not an optimization.

### Rolling hashes

A rolling hash updates in Θ(1) when the window slides, instead of rehashing the whole window. The polynomial (Rabin-Karp) form:

```
H(s[0..m]) = s[0]·b^(m-1) + s[1]·b^(m-2) + … + s[m-1]   (mod p)

slide by one:  H' = (H − s[0]·b^(m-1))·b + s[m]          (mod p)
```

Two multiplications and an addition, regardless of window size. That turns naive Θ(n·m) substring search into Θ(n + m) expected, and it's the basis of:

- **Rabin-Karp** substring search, and multi-pattern search by hashing all patterns into a set.
- **Content-defined chunking** — slide a window over a file and cut a chunk boundary wherever the rolling hash has (say) 13 low zero bits. Boundaries then depend on *content*, not offset, so inserting a byte at the start doesn't shift every subsequent chunk. This is how rsync, borg, restic and most deduplicating backup systems get useful dedup ratios.
- **Plagiarism/near-duplicate detection** via winnowing over document fingerprints.

The correctness caveat: a rolling hash match is a *candidate*, not a match. You must verify with a real comparison, because collisions are possible — and with a weak modulus, constructible.

### Consistent hashing

Plain `hash(key) % N` remaps almost every key when N changes — add one server to ten and ~90% of keys move. Consistent hashing places both keys and nodes on a ring and assigns each key to the next node clockwise, so adding a node moves only ~1/N of the keys. **Virtual nodes** (each physical node placed at many ring positions) fix the load imbalance that a small number of nodes otherwise produces. This is the mechanism behind the [sharding](../../architecture-patterns/sharding/learning.md) topic, and it's also what a distributed cache uses to avoid a full cache invalidation on every deployment.

### Perfect hashing

When the key set is **known in advance and fixed**, you can construct a collision-free hash: zero probing, no comparisons on lookup, no load-factor slack. *Minimal* perfect hashing maps n keys onto exactly 0..n. In Rust this is the `phf` crate, generating the table at compile time — the right answer for keyword tables, static routing tables, and enum-from-string parsing.

### Writing a `Hash` impl — combining is where it goes wrong

```rust
impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);        // feed fields IN ORDER into the same hasher
        self.y.hash(state);
    }
}
```

Feed fields sequentially into the provided `Hasher`; do **not** compute per-field hashes and combine them yourself. `h(x) ^ h(y)` is the classic mistake — XOR is commutative and self-annihilating, so `(1,2)` and `(2,1)` collide, and `(a,a)` hashes to 0 for every `a`. Deriving `Hash` does the right thing; hand-write it only when you must, and then only by delegating to `state`.

Also feed a length or delimiter when hashing variable-length sequences, or `["ab","c"]` and `["a","bc"]` collide. std's `Hash for [T]` writes the length first for exactly this reason.

## Complexity

| Technique | Cost | Note |
| --- | --- | --- |
| Hash a k-byte key | Θ(k) | With a large constant for short keys — measured above |
| Table lookup | Θ(1) expected | The Θ(1) is *borrowed* from the hash's uniformity |
| Rolling hash, slide by 1 | **Θ(1)** | Independent of window size |
| Rabin-Karp search | Θ(n + m) expected, Θ(n·m) worst | Worst case when collisions force verification every position |
| Consistent hashing lookup | Θ(log V) | V = virtual nodes; binary search on the ring |
| Consistent hashing, add node | ~1/N keys move | vs ~all for `mod N` |
| Perfect hash lookup | **Θ(1) worst case** | No probing; requires a fixed key set |
| Cryptographic hash | Θ(k), ~10× slower | Buy it only for adversarial collision resistance |

## Rust Implementation

```rust
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher, RandomState};

// Derive whenever possible — it delegates correctly and keeps Hash/Eq in sync.
#[derive(PartialEq, Eq, Hash)]
struct Key { tenant: u32, name: String }

// Hand-written: feed fields into the SAME hasher, in order. Never XOR sub-hashes.
impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) { self.x.hash(state); self.y.hash(state); }
}

// Choosing a hasher is a per-map decision (see hash-tables).
type FastMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<rustc_hash::FxHasher>>;

// Hashing something once, outside a map.
fn hash_of<T: Hash>(t: &T, build: &RandomState) -> u64 {
    let mut h = build.build_hasher();
    t.hash(&mut h);
    h.finish()                       // NOT stable across runs — never persist this
}

// Rolling hash: Θ(1) per slide.
struct Rolling { h: u64, pow: u64, base: u64, modulus: u64 }
impl Rolling {
    fn slide(&mut self, out: u8, incoming: u8) {
        self.h = (self.h + self.modulus - (out as u64 * self.pow) % self.modulus) % self.modulus;
        self.h = (self.h * self.base + incoming as u64) % self.modulus;
    }
}
```

**Crates:** `rustc-hash` (FxHash), `ahash` (fast + DoS-resistant), `xxhash-rust`, `blake3` (fast cryptographic, content addressing), `argon2` (passwords), `phf` (compile-time perfect hashing), `seahash`, `twox-hash`.

## Use Cases

- **Hash tables** — the dominant consumer; the table borrows its complexity guarantee from here.
- **Deduplication and content addressing** — Git's object store, container image layers, backup systems. Needs a cryptographic hash if the content is untrusted.
- **Sharding and placement** — consistent hashing so scaling doesn't reshuffle everything.
- **Bloom filters and sketches** — k independent hashes over the same key (Stage 8).
- **Substring search and diffing** — Rabin-Karp, and rolling hashes inside `diff`/`rsync`.
- **Cache keys** — hash the request shape to a fixed-size key. Watch stability: a hash that changes between deploys invalidates the whole cache.
- **Partitioning in data pipelines** — `hash(key) % partitions` decides which Kafka partition or worker gets a record, which is why the hash must be **stable across language runtimes**, not just across runs.
- **Integrity checks** — checksums, Merkle trees, signature preimages.

## When to Use Which

| Reach for | When |
| --- | --- |
| `RandomState` (std default) | Keys touch user input, network, or files — **the default is the safe default** |
| `ahash` | Want most of Fx's speed while keeping DoS resistance |
| `FxHash` (`rustc-hash`) | Keys are self-generated: indices, interned IDs, enum discriminants |
| `xxHash` / `wyhash` | Need speed **and** cross-run/cross-language stability (partitioning, checksums) |
| BLAKE3 / SHA-256 | Adversary must not find collisions: content addressing, dedup of untrusted data, integrity |
| Argon2 / bcrypt | Passwords. Never anything else here |
| Consistent hashing | Assigning keys to a changing set of nodes |
| `phf` | Fixed key set known at compile time |
| Rolling hash | Sliding window over a sequence: substring search, chunking |

## Pitfalls in Depth

### Pitfall: Persisting a hash value that isn't stable

- **What goes wrong:** A cache key, a database column, a shard assignment, or a file name is computed with `DefaultHasher`/`RandomState`. It works in testing. In production the value differs between processes — and, in Rust, between two `HashMap` instances in the *same* process, because `RandomState` reseeds per instance. Symptoms: total cache miss after every restart, records that can never be found again, keys landing on the wrong shard.
- **Why it happens (the mechanism):** The randomized seed is a *security feature* (the HashDoS defence), and its whole point is unpredictability. `Hasher::finish()` returns a `u64` that looks like a stable identifier and isn't. Nothing in the type system distinguishes "hash for in-memory bucketing" from "hash as a durable identifier."
- **How to handle it in production, and why that works:** Use an explicitly stability-documented function for anything durable — `xxHash`/`wyhash` for speed, BLAKE3/SHA-256 when it must also be collision-resistant — and pin the algorithm and its version in your own code, so an upgrade can't silently change values. Treat "is this value ever written down, sent over a wire, or compared across processes?" as the deciding question.
- **Trade-offs of the fix:** A stable hash is by definition predictable, so if it's also used to index an in-memory table keyed by user input you've reintroduced the DoS vector — meaning you sometimes need *two* hashes for the same data, one stable and one keyed. Pinning the algorithm also means you own its migration if you ever change it, which for stored values means a rehash of everything.

### Pitfall: Combining sub-hashes with XOR (or addition)

- **What goes wrong:** A hand-written `Hash` computes each field's hash and XORs them. `(1,2)` and `(2,1)` now collide. Any struct with two equal fields hashes to 0. A tuple key with swapped components is indistinguishable. In a hash table this shows up as a mysteriously slow map — a cluster of colliding keys — rather than as incorrect results, so it can persist for a long time.
- **Why it happens (the mechanism):** XOR is commutative and associative, so it discards field *order*, and `x ^ x == 0`, so it discards field *identity* for repeated values. Addition has the same commutativity problem. The `Hasher` API's `write_*` methods are stateful precisely so that order is captured; bypassing them throws that away.
- **How to handle it in production, and why that works:** Derive `Hash`. When you must hand-write it, call `field.hash(state)` for each field in a fixed order so the hasher's internal state mixes them — that's what the sequential design is for. For variable-length sequences, feed the length first (as std's `[T]` impl does) so `["ab","c"]` and `["a","bc"]` differ.
- **Trade-offs of the fix:** Deriving forces every field to participate, which is wrong when a field is a cache, timestamp, or other non-identity data. Then hand-write it — but hand-write `Eq` at the same time, over the same fields, and property-test the implication. The cost is one small test; the alternative is an invisible clustering bug.

### Pitfall: The wrong hash family for the job

- **What goes wrong:** Four distinct failures. SHA-256 chosen for a hash map, costing ~10× for security nobody needed. FxHash chosen to deduplicate user-uploaded files, so a malicious user can produce two different files with the same "fingerprint" and overwrite someone's data. SHA-256 (fast, unsalted) chosen for password storage, so a leaked table is cracked at billions of guesses per second. A randomized hash chosen for Kafka partitioning, so the same key lands on different partitions from different producers and per-key ordering breaks.
- **Why it happens (the mechanism):** "Hash function" reads as one interchangeable concept, and the requirements are invisible from the call site — all five return bytes. The failure is silent in every case: the wrong choice produces plausible values and fails only under adversarial input or across process boundaries.
- **How to handle it in production, and why that works:** Choose by asking two questions in order: *(a) does an adversary benefit from finding a collision?* → cryptographic. *(b) does the value need to be identical in another process, language, or run?* → a stability-documented function. Only if both are "no" is speed the deciding factor. Passwords are their own category and never share an answer with anything else.
- **Trade-offs of the fix:** Cryptographic hashes really do cost an order of magnitude, so applying them by default to internal keys is a measurable waste. And a "just use BLAKE3 everywhere" policy is wrong for passwords specifically, where the *speed* is the vulnerability — a policy that's correct four times out of five is exactly the shape that produces the fifth failure.

### Pitfall: Modulo bias and using the low bits of a weak hash

- **What goes wrong:** `hash(key) % table_size` with a table size that shares factors with a pattern in the hash. Or taking the low bits of a multiply-only hash, where the low bits of a product depend on very few input bits. Keys cluster into a fraction of the buckets, probe sequences lengthen, and the table's performance degrades in a way no test catches because it depends on the key distribution.
- **Why it happens (the mechanism):** Not all bits of a hash are equally good. A multiply-shift hash concentrates its avalanche in the *high* bits — the low bits of `x * K` are determined by the low bits of `x` alone. Taking `% 2^n` selects exactly those weak low bits. Historically this is why prime-sized tables were recommended; modern tables use power-of-two sizes and take the *high* bits instead, or apply a finalizer.
- **How to handle it in production, and why that works:** Use a hash function with good avalanche across all bits, and take bits from the strong end (SwissTable derives its bucket index from the high bits and its 7-bit tag from another part of the hash). If you must use a weak-but-fast hash, apply a finalizer (an xor-shift-multiply mix) before extracting bits. Practically: don't hand-roll the bucket-index computation — the library's choice already accounts for this.
- **Trade-offs of the fix:** A finalizer costs a few instructions, which partly erodes the reason you chose a fast hash. Prime-sized tables avoid the issue without a finalizer but pay a modulo (a division, ~20–40 cycles) on every lookup instead of a mask — which is why essentially all modern implementations chose power-of-two plus good bit-mixing.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if the hash addressed the content forever? | **Content addressing** — Git, IPFS, container layers; the hash *is* the name |
| Batch it | What if you hashed many keys at once? | SIMD-parallel hashing; BLAKE3's tree mode; batched table probes |
| Approximate it | What if you only kept a few bits per item? | Bloom filters, MinHash, SimHash — similarity from hashes alone |
| Randomize it | What if the function were secretly keyed? | SipHash and the entire HashDoS defence |
| Externalize it | What if buckets were disk pages? | Extendible/linear hashing; hash partitioning in databases |
| Parallelize it | Can hashing itself be split? | Tree hashing (BLAKE3) — parallel *and* incrementally verifiable |
| Invert it | What if you hashed the *node set* instead of just keys? | **Consistent hashing** — the ring; ~1/N movement instead of ~all |
| Augment it | What if you kept a few bits of the hash beside the entry? | SwissTable's 7-bit control tags — reject 16 slots per SIMD compare |
| Specialize it | What if the key set were fixed and known? | **Perfect hashing** — Θ(1) worst case, no probing, no slack |
| Amortize it | What if updating the hash were Θ(1) instead of Θ(k)? | **Rolling hash** — Rabin-Karp, content-defined chunking |

**Questions:**

1. Rust reseeds `RandomState` per `HashMap` *instance*. Name a concrete bug that per-instance seeding causes which per-process seeding wouldn't, and one attack it prevents that per-process seeding wouldn't.
2. Under "invert it": derive why `hash(key) % N` moves ~all keys when N changes but the ring moves ~1/N. Then explain what virtual nodes fix and what they cost.
3. Content-defined chunking cuts boundaries where the rolling hash has k low zero bits. Derive the expected chunk size from k, and explain why inserting one byte at the start of a file doesn't shift every later boundary.
4. Measured, SipHash's per-byte cost drops from 3.04 ns/byte at 4 bytes to 0.31 at 1024. What does that shape tell you about the algorithm's structure, and what does it predict about the Fx-vs-SipHash ratio at 1 MB?
5. XOR-combining sub-hashes breaks two distinct ways. Name both, give a colliding pair for each, and explain why feeding fields into one `Hasher` fixes both.
6. Under "augment it", SwissTable stores 7 hash bits per slot. Why 7 and not 8, and what does the spare bit encode?
7. A hash used for Kafka partitioning must be stable across languages; one used for an in-memory map keyed by user input must be unpredictable. What do you do when the same field needs both?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the `Hash`/`Eq` contract as an implication, and say which direction is *not* required.
2. Name the five hashing jobs and the one property each needs that the others don't.
3. Why must you never persist `DefaultHasher::finish()`? Give the Rust-specific reason that's stronger than "it might change between versions."
4. `h(x) ^ h(y)` — give two different collisions it produces and the correct alternative.
5. Give the rolling-hash slide formula and say why it's Θ(1) regardless of window size.
6. When is a fast non-cryptographic hash a security bug? Give two distinct scenarios.

Build exercises:

- Implement Rabin-Karp with a rolling hash, then construct an input that forces the Θ(n·m) worst case (many hash matches that fail verification). This makes "a hash match is a candidate, not a match" permanent.
- Implement content-defined chunking over a file, then insert one byte at the beginning and measure how many chunk boundaries moved. Compare against fixed-size chunking, where all of them move. That contrast is the whole argument for CDC in backup systems.
- Write a deliberately bad hasher (identity, or multiply-only taking low bits), put it behind `HashMap`, and measure the clustering — then apply an xor-shift-multiply finalizer and watch it recover. Ties directly to the modulo-bias pitfall.
- Implement consistent hashing with and without virtual nodes, and measure the key-distribution imbalance across 10 nodes for both. Then add an 11th node and count how many keys moved, against a `% N` baseline.

## Open Questions

- Where does `ahash` land on the measured cost curve above? If it's near Fx at 4–16 bytes while staying keyed, the whole "classify every map by key provenance" discipline mostly collapses into "use ahash."
- The 8-byte SipHash row was faster than the 4-byte row. Confirm this is a word-sized fast path by reading the implementation rather than inferring it.
- BLAKE3 vs SHA-256 for short inputs on this machine — BLAKE3's parallelism should only pay above some size; find it.
- For content-defined chunking, what k (zero-bit count) gives the best dedup-ratio-to-metadata-overhead trade on realistic data?
- Does `xxh3` beat Fx for `u32` keys while also being stable across runs? If so it dominates Fx for most internal uses.

## References

- Aumasson & Bernstein, ["SipHash: a fast short-input PRF"](https://www.aumasson.jp/siphash/siphash.pdf) — why a *keyed* hash is the right hash-table defence, and why short-input speed was the design target.
- Karp & Rabin, "Efficient randomized pattern-matching algorithms" (1987) — the rolling hash, and the reason a match must be verified.
- Karger et al., "Consistent Hashing and Random Trees" (1997) — the ring, from the original distributed-caching motivation.
- [BLAKE3 spec](https://github.com/BLAKE3-team/BLAKE3) — tree hashing: parallel, incremental, and verifiable; a good example of the "parallelize it" lens applied to a hash.
- [`phf`](https://github.com/rust-phf/rust-phf) — compile-time perfect hashing in Rust.
- [`std::hash::Hash` docs](https://doc.rust-lang.org/std/hash/trait.Hash.html) — the contract, and the explicit warning about hash stability.
- Related in this repo: [Hash Tables](../hash-tables/learning.md) (the primary consumer; the HashDoS measurements), [Strings & Text](../strings-and-text/learning.md) (interning, and why key length dominates), [Sharding](../../architecture-patterns/sharding/learning.md) (consistent hashing at system scale), [Encryption & Key Management](../../architecture-patterns/encryption-and-key-management/learning.md) (where cryptographic hashes actually belong), [Caching Strategies](../../architecture-patterns/caching-strategies/learning.md) (stable cache keys).
