# Tries & Radix Trees — Quick Reference

## At a Glance

Keys live in the **shape of the tree**, not in the nodes: root→node path spells a prefix. Lookup is Θ(k) in key length, **independent of n**, with no comparisons.

**Invariant:** each node = the prefix on its path; children indexed by the *next symbol*; a **terminal marker independent of having children** (`car` and `cart` both exist).
**Radix tree adds:** no single-child non-terminal nodes — chains collapse to one labelled edge.

## The Measured Warning

200k keys, my uncompressed trie vs the obvious alternatives:

| Operation | Trie | `HashMap` | `BTreeMap` range | Sorted `Vec` |
| --- | --- | --- | --- | --- |
| Prefix `"ab"` | 0.0168 ms | 0.9883 ms (scan) | 0.0019 ms | **0.0015 ms** |
| Exact lookup | 119.9 ns | **17.9 ns** | — | — |
| Memory | **43.7 MB** (1.28M nodes) | 5.6 MB | — | — |

URL-like keys (shared prefixes — supposedly the best case) were **worse**: 2.46M nodes, ~84 MB, 281.6 ns/lookup vs 27.7 ns.

> **An uncompressed trie is not viable. Path compression is what makes a trie work at all** — like balancing for a BST.
> For prefix queries on static data, a **sorted `Vec` is hard to beat**.

## Complexity

| Operation | Trie/radix | `HashMap` | Sorted `Vec` |
| --- | --- | --- | --- |
| Lookup | Θ(k), independent of n | Θ(k) | Θ(k log n) |
| Insert / delete | Θ(k) | Θ(k) | Θ(n) |
| **Prefix enumeration** | Θ(k + matches) | Θ(n·k) | Θ(k log n + matches) |
| **Longest-prefix match** | **Θ(k)** | **impossible** | awkward |
| Space uncompressed | Θ(total key length) | Θ(n·k) | Θ(n·k) |
| Space radix | **Θ(n) nodes** | Θ(n·k) | Θ(n·k) |

Θ(k) trie = **k dependent pointer chases**; Θ(k) hash = one sequential pass. Not the same Θ(k).

## Child Storage — the whole design

| Representation | Lookup | Memory/node | Use |
| --- | --- | --- | --- |
| `[Option<u32>; 256]` | Θ(1) | **1 KB — catastrophic** | tiny alphabet only |
| Sorted `Vec<(u8,u32)>` | Θ(log σ) | ~3 B/child | general |
| **Bitmap + packed array** | **Θ(1) via popcount** | 32 B + present children | **the good answer** (ART, HAMT) |

## Choose This When

| Use | For |
| --- | --- |
| **Sorted `Vec` + `partition_point`** | Static set, prefix queries — measured fastest, 5 lines |
| **`BTreeMap::range`** | Changing set, prefix queries — no new code |
| `HashMap` | Exact lookup only — **6.7× faster than a trie** |
| **`fst` crate** | Large *static* dictionary; prefix + fuzzy + tiny memory |
| **Radix tree / ART** | **Longest-prefix match** or an incremental cursor |
| `aho-corasick` | Many patterns, one pass |
| Uncompressed trie | Teaching only |

## What Actually Survives as a Trie Advantage

- **Longest-prefix match** (IP routing) — nothing else does it
- **Incremental cursor** — advance per keystroke instead of restarting the search
- **Multi-pattern search** (Aho-Corasick)
- Per-subtree aggregates (top-k autocomplete)

## Snippets

```rust
// Try these BEFORE building a trie.
let i = keys.partition_point(|k| k.as_str() < prefix);          // static
let m = keys[i..].iter().take_while(|k| k.starts_with(prefix));

map.range(prefix.to_string()..)                                  // changing
   .take_while(|(k, _)| k.starts_with(prefix));

// Bitmap child index: Θ(1), memory ∝ actual children
let rank = mask[..w].iter().map(|x| x.count_ones()).sum::<u32>()
         + (mask[w] & ((1u64 << b) - 1)).count_ones();
```

## Rules of Thumb

- Measure the sorted `Vec` first. It usually wins.
- "Trie" in production means **radix tree** — always compress.
- Terminal marker ≠ leaf. `car`/`cart` is the test case.
- Never a 256-slot array per node; bitmap + popcount instead.
- Labels are offsets into one shared buffer, not a `String` per node.
- Static key set → reach for `fst` before writing anything.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Uncompressed | 8–15× memory, 6–10× slower than `HashMap` |
| Leaf == key | `contains("car")` false once `"cart"` exists |
| No upward pruning on delete | Dangling empty chains inflate memory |
| 256-slot child arrays | GB of zeros; cache miss per level |
| Chose trie without measuring | 11× slower than a 5-line sorted-`Vec` scan |

## Key References

- Leis et al., "The Adaptive Radix Tree" (2013) — what makes tries competitive
- Morrison, "PATRICIA" (1968) — path compression
- Gallant, ["Index 1,600,000,000 Keys with Automata and Rust"](https://blog.burntsushi.net/transducers/) — `fst`
