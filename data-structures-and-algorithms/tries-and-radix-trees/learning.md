# Tries & Radix Trees — Learning Notes

## Mental Model

**A trie stores keys in the *shape of the tree* rather than in the nodes.** The path from the root to a node spells a prefix; the node itself holds no key. Two keys sharing a prefix share the path, so a trie is a structure whose *topology is the data*.

That buys three things nothing else does cheaply:

1. **Prefix queries** — "everything starting with `foo`" is one descent plus a subtree.
2. **Longest-prefix match** — "the most specific rule matching this key", which is IP routing's core operation and something a hash map fundamentally cannot do.
3. **Comparison-free lookup** — you consume the key one symbol at a time, so lookup is Θ(k) in key length with **no comparisons and no dependence on n**. A trie with a billion keys costs the same as one with ten.

Now the part that the textbook framing hides, and that I measured rather than assumed. I built a straightforward trie (one node per character, children in a sorted vec) over 200,000 keys and compared it against the obvious alternatives:

| Operation (200k random words) | Trie | `HashMap` scan | `BTreeMap` range | Sorted `Vec` |
| --- | --- | --- | --- | --- |
| Prefix count `"ab"` | 0.0168 ms | 0.9883 ms | 0.0019 ms | **0.0015 ms** |
| Prefix count `"abc"` | 0.0006 ms | 0.9533 ms | **0.0001 ms** | 0.0002 ms |
| Exact lookup | 119.9 ns | **17.9 ns** | — | — |
| Memory (structure only) | **43.7 MB** (1.28M nodes) | 5.6 MB | — | — |

**The trie lost on every axis.** Against URL-like keys with heavy shared prefixes — supposedly its best case — it got *worse*: 2.46M nodes, ~84 MB, 281.6 ns per exact lookup against `HashMap`'s 27.7 ns.

The lesson isn't "tries are bad." It's this:

> **An uncompressed trie is not a viable structure.** One node per character means one pointer chase and one allocation per character, and for any realistic alphabet the node overhead dwarfs the data. **Path compression is not an optimization on top of a trie — it is the thing that makes a trie work at all**, exactly as balancing is for a [BST](../binary-search-trees/learning.md).

And the second lesson: for pure *prefix range* queries over a static set, a **sorted array is extremely hard to beat**, because all matching keys are contiguous. What survives as a genuine trie advantage is **longest-prefix match** and **incremental/streaming matching**, which sorted arrays can't do — and neither was in the table above.

## The Invariant

> Every node represents the prefix spelled by the path from the root. A node's children are indexed by the *next symbol*. A terminal marker (or a stored value) distinguishes "this prefix is a key" from "this prefix is merely on the way to one."

Two consequences:

- **The terminal marker is mandatory and separate from having children.** `"car"` and `"cart"` both exist means the node at `car` is terminal *and* has a child. Conflating "leaf" with "key" is the classic bug: it breaks whenever one key is a prefix of another.
- **Keys are never compared, only consumed.** This is why lookup is Θ(k) independent of n, and why the structure works on any sequence type — bytes, characters, bits, IP octets, words.

For a **radix (PATRICIA) tree**, add:

> No node has exactly one child unless it is terminal. Chains of single-child nodes are collapsed into one edge labelled with the whole substring.

That single rule is what turns the measured disaster above into a usable structure.

## Mechanics

### The child-storage decision is the whole design

Every trie node needs "given the next symbol, which child?" and this choice dominates memory and speed:

| Child representation | Lookup per level | Memory per node | Use when |
| --- | --- | --- | --- |
| Array `[Option<u32>; 256]` | **Θ(1)** | 1 KB — catastrophic | Tiny alphabet or a dense top level |
| Sorted `Vec<(u8, u32)>` | Θ(log σ) binary search | ~3 B/child | General purpose (what I measured) |
| `HashMap<u8, u32>` | Θ(1) expected | large constant | Rarely worth it |
| **Bitmap + packed array** | Θ(1) via popcount | **1 child slot each + 32 B** | The good answer — ART, HAMT |

The bitmap approach is what modern implementations use: a 256-bit occupancy mask plus a densely packed array of present children. `popcount(mask & ((1<<c)-1))` gives the index in Θ(1) with no wasted slots. It's the same trick as SwissTable's control bytes from [hash tables](../hash-tables/learning.md): a small dense side-structure making the big sparse one cheap.

### Path compression — from unusable to usable

An uncompressed trie storing `"https://api.example.com/v1/users/00000001"` uses **41 nodes**, 39 of which have exactly one child and exist only to spell out characters. A radix tree stores the whole shared run on one edge.

Measured: 200k URL-like keys produced **2,462,472 nodes** uncompressed — 12 nodes per key, ~84 MB for keys totalling a few MB. Compressing single-child chains would collapse the shared `https://api.example.com/v1/` prefix (27 characters) into one edge for the entire subtree.

The rule: **a node exists only where the key set actually branches, or where a key ends.** So the node count becomes Θ(number of keys), not Θ(total key length).

### Adaptive Radix Tree (ART)

The production design. It combines path compression with **adaptive node sizes**: a node uses one of four layouts (Node4, Node16, Node48, Node256) depending on how many children it currently has, growing and shrinking as needed. Node4 stores 4 keys and 4 pointers; Node16 uses SIMD to compare 16 keys at once; Node256 is a direct-index array. This makes the memory cost proportional to actual branching rather than to alphabet size, which is precisely what my measured trie got wrong.

### Longest-prefix match — the operation nothing else does

Given `192.168.1.42`, find the most specific stored route (`192.168.1.0/24` rather than `192.168.0.0/16`). Descend the trie consuming bits, remembering the deepest terminal node passed. Θ(k) — 32 steps for IPv4, 128 for IPv6 — independent of the routing table size.

**No hash map can do this**, because it requires knowing which prefixes exist without enumerating them. A sorted array can do a variant (predecessor search plus a check), but the trie version is direct and is what real routers implement, usually as a multibit trie consuming several bits per level.

### Where tries genuinely win

Being honest about the measurements above, the surviving cases are:

- **Longest-prefix match** — routing tables, IP geolocation, URL-pattern dispatch, phone-number prefix billing.
- **Incremental / streaming matching** — you have a cursor in the trie and feed symbols as they arrive (autocomplete as the user types, tokenizing a stream). A sorted array must restart its search on every keystroke; a trie just advances one node.
- **Multi-pattern search** — Aho-Corasick is a trie plus failure links, matching thousands of patterns in one pass over the text (Stage 7).
- **Ordered iteration in a dense key space** — where the trie *is* the sorted order, for free.
- **Space when prefixes are highly shared and compression is used** — succinct/compressed tries beat storing keys individually for dictionaries.

## Complexity

| Operation | Trie / radix tree | `HashMap` | Sorted `Vec` |
| --- | --- | --- | --- |
| Lookup | Θ(k), **no comparisons, independent of n** | Θ(k) hash + Θ(k) compare | Θ(k log n) |
| Insert | Θ(k) | Θ(k) amortized | Θ(n) |
| Delete | Θ(k) | Θ(k) | Θ(n) |
| **Prefix enumeration** | Θ(k + matches) | **Θ(n·k)** — must scan | Θ(k log n + matches) |
| **Longest-prefix match** | **Θ(k)** | **impossible** | Θ(k log n), awkward |
| Ordered iteration | Θ(total key length) | Θ(n log n) (sort first) | Θ(n) |
| Space, uncompressed | **Θ(total key length)** | Θ(n·k) | Θ(n·k) |
| Space, radix-compressed | Θ(n) nodes | Θ(n·k) | Θ(n·k) |

**Where the table misleads, and this is the important part.** "Θ(k), independent of n" reads like a decisive win over a hash map's Θ(k). It isn't, because the constants differ enormously: a trie does **k dependent pointer chases** — each a potential cache miss — where a hash map does one hash pass over k bytes (sequential, prefetched) and one probe. Measured: **119.9 ns for a trie lookup versus 17.9 ns for `HashMap`** on the same 200k keys, a 6.7× loss for the structure with the "better" bound.

Likewise the prefix-enumeration row: the trie's Θ(k + matches) beat the hash map's Θ(n·k) by ~60× as expected — but *lost* to the sorted `Vec`'s Θ(k log n + matches), because the sorted array's matches are contiguous while the trie's require a subtree walk through scattered nodes.

## Rust Implementation

The arena representation, with the bitmap child index that makes it viable:

```rust
/// Radix-tree node: an edge label (the compressed run) plus children indexed by a 256-bit mask.
struct Node {
    label: (u32, u32),          // (offset, len) into one shared key buffer — no per-node String
    mask: [u64; 4],             // 256-bit occupancy: which next-bytes exist
    children: Vec<u32>,         // packed, only present children — index via popcount
    terminal: Option<u32>,      // value index; separate from "has children"
}

struct RadixTree { nodes: Vec<Node>, keys: Vec<u8> }

impl RadixTree {
    #[inline]
    fn child(&self, n: u32, byte: u8) -> Option<u32> {
        let node = &self.nodes[n as usize];
        let (w, b) = (byte as usize / 64, byte as usize % 64);
        if node.mask[w] >> b & 1 == 0 { return None; }
        // Rank = number of set bits before this one → index into the packed array.
        let rank: u32 = node.mask[..w].iter().map(|x| x.count_ones()).sum::<u32>()
                      + (node.mask[w] & ((1u64 << b) - 1)).count_ones();
        Some(node.children[rank as usize])
    }
}
```

Three things this gets right that my measured version didn't: **path compression** (label spans a run, not one byte), **packed children** (no 256-slot array, no per-node allocation growth), and **a shared key buffer** (labels are offsets, so there's no `String` per node).

**Before writing any of that, try the alternatives:**

```rust
// Prefix queries on a static set: a sorted Vec is hard to beat and is 5 lines.
let i = keys.partition_point(|k| k.as_str() < prefix);
let matches = keys[i..].iter().take_while(|k| k.starts_with(prefix));

// Prefix queries on a changing set: BTreeMap range does it with no new code.
for (k, v) in map.range(prefix.to_string()..).take_while(|(k, _)| k.starts_with(prefix)) { }
```

**Crates:** `radix_trie`, `qp-trie` (QP-trie — a compact, fast radix variant), `patricia_tree`, `fst` (finite-state transducer — a *succinct* structure that stores a large dictionary in remarkably little memory with fast prefix and fuzzy search; usually the right answer for large static key sets), `ip_network_table` / `treebitmap` for routing, `aho-corasick` for multi-pattern search.

**`fst` deserves special mention**: for a large static dictionary it beats both a trie and a sorted `Vec` on memory by a wide margin while supporting prefix, range, and fuzzy queries. If your key set is static, look there before building anything.

## Use Cases

- **IP routing tables** — longest-prefix match, the irreplaceable case.
- **Autocomplete** — but note that a sorted `Vec` or `BTreeMap` range handles the query; the trie's advantage is the *incremental* cursor as the user types, plus easy top-k-by-score-per-subtree if you augment nodes.
- **Multi-pattern search** — Aho-Corasick over a trie of patterns, one pass over the text.
- **URL/route dispatch** — matching `/users/:id/posts` style patterns by longest prefix.
- **Dictionary compression** — `fst`, succinct tries, and DAWGs for spell-checkers and IME.
- **Key-value stores** — ART is used as an in-memory index in several database engines.
- **HAMTs** — the persistent-map structure in `im`/Clojure/Scala is a trie over hash bits, which is the "persist it" lens applied to [hash tables](../hash-tables/learning.md).

## When to Use Which

| Reach for | When |
| --- | --- |
| **Sorted `Vec` + `partition_point`** | Static key set, prefix queries — **measured fastest**, five lines |
| **`BTreeMap::range`** | Changing key set, prefix queries — measured competitive, no new code |
| `HashMap` | Exact lookup only — **6.7× faster than a trie**, measured |
| **`fst` crate** | Large *static* dictionary; want prefix + fuzzy + tiny memory |
| **Radix tree / ART** | **Longest-prefix match**, or an incremental cursor, on a changing set |
| `aho-corasick` | Matching many patterns in one pass |
| Uncompressed trie | Teaching only — measured unviable |

## Pitfalls in Depth

### Pitfall: Building an uncompressed trie

- **What goes wrong:** One node per character. Measured on 200k URL-like keys: **2,462,472 nodes, ~84 MB** for a key set of a few MB, with exact lookups at **281.6 ns against `HashMap`'s 27.7 ns**. On 200k random words it was 1.28M nodes and 43.7 MB against the hash map's 5.6 MB. The structure is 8–15× larger and 6–10× slower than the thing it was supposed to beat.
- **Why it happens (the mechanism):** In an uncompressed trie the node count is Θ(total key length), not Θ(number of keys), and the overwhelming majority of nodes have exactly one child — they exist only to spell out a character. Each is a separate allocation, each traversal step is a dependent pointer chase, and per-node bookkeeping (a child container, a terminal flag) dwarfs the single byte of information the node carries. Shared prefixes make this *worse* in absolute terms, because longer keys mean more single-child nodes.
- **How to handle it in production, and why that works:** Path-compress — collapse single-child chains into one edge with a substring label, so nodes exist only where the key set branches or a key ends, making the count Θ(n). Combine with adaptive node sizes (ART) so a node with 3 children costs 3 slots rather than 256. Store labels as offsets into one shared buffer rather than a `String` per node.
- **Trade-offs of the fix:** Compression makes insertion harder — adding a key that diverges mid-edge requires *splitting* that edge into two nodes, which is the fiddliest operation in the structure and where implementations have bugs. Adaptive nodes add growth/shrink transitions between four layouts. This is a real jump in complexity, and it's why the honest first move is to check whether a sorted `Vec` or `fst` solves your problem instead.

### Pitfall: Choosing a trie for prefix queries without measuring the alternatives

- **What goes wrong:** "We need prefix search, so we need a trie" — a week is spent building one, and it's slower than five lines using the sorted `Vec` already in the codebase. Measured on 200k keys, prefix `"ab"`: sorted `Vec` **0.0015 ms**, `BTreeMap` 0.0019 ms, trie 0.0168 ms — the trie is **11× slower** than the trivial option.
- **Why it happens (the mechanism):** "Trie" is the textbook answer for prefix queries, so it's reached for by name rather than by measurement. But a prefix query over sorted data is just a contiguous range: one binary search to find the start, then a linear scan of adjacent memory with perfect prefetching. The trie must descend k pointer chases and then walk a subtree scattered across the heap. The sorted array wins because *the matches are contiguous* — the same contiguity argument as [arrays](../arrays-and-dynamic-arrays/learning.md).
- **How to handle it in production, and why that works:** Try the sorted `Vec` (static) or `BTreeMap::range` (changing) first and measure. Reach for a trie only when you need something they genuinely can't do: longest-prefix match, an incremental cursor across successive queries, per-subtree aggregates, or multi-pattern matching.
- **Trade-offs of the fix:** A sorted `Vec` has Θ(n) insertion, so a write-heavy prefix workload does need a tree — but that's `BTreeMap`, which measured competitively, not a hand-built trie. And if the key set is large and static, `fst` beats all three on memory.

### Pitfall: Conflating "leaf" with "key"

- **What goes wrong:** The trie marks a node as a key by testing whether it has children. Then `"car"` and `"cart"` are both inserted, and `contains("car")` returns false — the node has a child, so it isn't a leaf. Deleting `"cart"` may also delete `"car"`, or leave a dangling chain of empty nodes that inflate memory and slow traversal.
- **Why it happens (the mechanism):** In many toy examples every key is a leaf, so the two notions coincide and the bug is invisible. They diverge exactly when one key is a prefix of another — which is common in every real key set: paths, URLs, identifiers, dictionary words.
- **How to handle it in production, and why that works:** Store an explicit terminal marker (an `Option<Value>` or a bool) on every node, independent of the child count. On deletion, clear the marker, then prune upward only while a node has no children *and* no marker. That makes the two conditions independent, which is what the data actually requires.
- **Trade-offs of the fix:** One extra field per node, which on an uncompressed trie is a meaningful fraction of node size — another push toward compression. Upward pruning needs parent links or a descent stack, adding code to deletion.

### Pitfall: A 256-slot child array per node

- **What goes wrong:** Each node holds `[Option<u32>; 256]` for Θ(1) child lookup. At 1 KB per node, a trie with a million nodes is a gigabyte, almost entirely zeros. Beyond the memory, traversal now misses cache on every level because each node spans 16 cache lines.
- **Why it happens (the mechanism):** Direct indexing is the obvious way to get Θ(1) child lookup, and it's genuinely optimal for a small alphabet (DNA: 4; binary tries: 2). For a 256-symbol alphabet with typical branching factors under 5, it wastes over 98% of the space — and the waste is what destroys locality.
- **How to handle it in production, and why that works:** Use a **bitmap plus a packed child array**: a 256-bit occupancy mask (32 bytes) and a dense `Vec` of only the present children, indexed by `popcount` of the mask below the target bit. That's Θ(1) lookup with one instruction, memory proportional to actual children, and a node that fits in a cache line or two. ART's adaptive layouts are the tuned version of the same idea.
- **Trade-offs of the fix:** Insertion into the packed array is Θ(children) because it shifts, though children counts are small. The popcount trick needs care across the four `u64` words. Direct indexing remains correct for genuinely small alphabets — don't apply this to a 4-symbol trie.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if insert returned a new trie sharing structure? | **HAMT** — the persistent map behind `im`, Clojure, Scala; a trie over hash bits |
| Batch it | What if all keys were known up front? | **`fst`** / succinct tries / DAWG — build once, minimal memory, no pointers |
| Approximate it | What if you only stored a fingerprint per subtree? | Bloom-filtered tries; hash-based prefix pruning |
| Randomize it | What if you keyed on hash bits instead of key bytes? | HAMT again — uniform branching, no adversarial deep paths |
| Externalize it | What if nodes were disk pages? | Prefix B-trees; the trie/B-tree hybrid used in on-disk indexes |
| Parallelize it | Where's the contention? | Subtrees are independent — partition by first byte; ART supports optimistic lock coupling |
| Invert it | What if edges spelled *runs* instead of symbols? | **Radix/PATRICIA tree** — the compression that makes it viable at all |
| Augment it | What does a per-subtree aggregate buy? | Top-k autocomplete (best score in subtree); counts for prefix frequency |
| Specialize it | What if the alphabet were 2 symbols? | **Binary trie** — IP routing, van Emde Boas layouts, x-fast/y-fast tries |
| Amortize it | What if you rebuilt periodically? | Batch-compress a mutable trie into an `fst`; the LSM-style read/write split |

**Questions:**

1. My uncompressed trie used 2.46M nodes for 200k keys. Derive the node count for a radix-compressed version and explain why it becomes Θ(n) rather than Θ(total key length).
2. Trie lookup is Θ(k) *independent of n*; `HashMap` lookup is Θ(k) too. Measured, the trie was 6.7× slower. Explain the constant-factor difference in terms of what each does per byte.
3. A sorted `Vec` beat the trie on prefix queries. State the property of sorted storage that makes prefix matches contiguous, and name the two operations that property cannot provide.
4. Under "invert it", radix compression collapses single-child chains. What does that do to *insertion*, and why is edge-splitting the bug-prone operation?
5. Under "specialize it", IP routing uses a binary trie over address bits. Why is longest-prefix match natural there, and why can a hash map not do it at all?
6. A HAMT is a trie over *hash bits* rather than key bytes. What does that buy over a trie over key bytes, and what does it lose?
7. Under "augment it", storing the best score per subtree gives top-k autocomplete. What must be true of the aggregate for it to be maintainable through insertion, and how does that rule compare to the BST augmentation rule?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the trie invariant and explain why the terminal marker must be independent of having children.
2. Give the measured node count and memory for 200k URL-like keys, and say why shared prefixes made it *worse* rather than better.
3. Give the four child-storage options with their lookup cost and memory, and say which one modern implementations use and why.
4. Why did the sorted `Vec` beat the trie at prefix queries, and which two trie operations does it still not replace?
5. Describe longest-prefix match on a binary trie and say why a `HashMap` cannot do it.
6. When should you use `fst` instead of building anything?

Build exercises:

- Build the uncompressed trie and reproduce the measurements: node count, memory, exact-lookup latency, and prefix-query time, all against a `HashMap` and a sorted `Vec` on the same key set. Seeing your own trie lose is the point of this topic.
- Now add path compression and re-measure all four numbers. Then add a bitmap+popcount child index and measure again. The two-step improvement is the argument for why "trie" in production always means "radix tree."
- Implement longest-prefix match over a binary trie of IPv4 CIDR blocks and use it to route a stream of addresses. Then try to implement the same thing with a `HashMap` and articulate precisely where it becomes impossible.
- Build an autocomplete with per-subtree top-k scores augmented into the nodes, then implement the same feature with `BTreeMap::range` plus a sort. Compare code, latency, and memory — and decide honestly which you'd ship.

## Open Questions

- How much do path compression and bitmap children actually recover? My measured trie was 6.7× behind `HashMap` on lookup — does a proper radix tree close that, or just narrow it?
- Where does `fst` land against a sorted `Vec` and a `BTreeMap` for prefix queries on 200k–10M static keys, and how big is its memory advantage really?
- Is `qp-trie` or `radix_trie` competitive with `BTreeMap::range` for prefix work in Rust, or is the ecosystem's answer effectively "use `BTreeMap`"?
- For longest-prefix match on a full IPv4 BGP table (~1M routes), how does a multibit trie compare to `ip_network_table`?
- Does ART's adaptive node sizing matter at these scales, or is plain path compression most of the win?

## References

- Leis, Kemper & Neumann, "The Adaptive Radix Tree: ARTful Indexing for Main-Memory Databases" (2013) — the design that makes tries competitive in memory; the adaptive-node argument is the core contribution.
- Donald Morrison, "PATRICIA" (1968) — the original path-compressed trie.
- Andrew Gallant, ["Index 1,600,000,000 Keys with Automata and Rust"](https://blog.burntsushi.net/transducers/) — the `fst` crate write-up; the best explanation of succinct key structures and why they beat tries for static sets.
- Aho & Corasick, "Efficient string matching: an aid to bibliographic search" (1975) — the trie-plus-failure-links multi-pattern matcher (Stage 7).
- Bagwell, "Ideal Hash Trees" (2001) — HAMTs; the trie-over-hash-bits idea behind persistent maps.
- Related in this repo: [Strings & Text](../strings-and-text/learning.md) (key representation, interning), [Hash Tables](../hash-tables/learning.md) (the measured exact-lookup comparison; the bitmap trick mirrors SwissTable's control bytes), [B-Trees](../b-trees/learning.md) (the other ordered structure, and prefix B-trees), [Arrays & Dynamic Arrays](../arrays-and-dynamic-arrays/learning.md) (why the contiguous sorted `Vec` keeps winning), [Binary Search](../binary-search/learning.md) (`partition_point`, the five-line prefix query).
