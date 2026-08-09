# Cache-Aware & Cache-Oblivious Structures — Quick Reference

## At a Glance

When two structures share an asymptotic bound, **the one matching the memory hierarchy wins**. Count **block transfers**, not operations.

- **Cache-aware:** knows B, tuned to it (B-trees, tiling). Optimal for one level; retune per machine.
- **Cache-oblivious:** recursive decomposition works at **every** level without knowing B (van Emde Boas, recursive matmul).

## The Number

Binary search, 1,000,000 random `u32` queries (measured):

| n | `std::binary_search` | Branchless | Eytzinger (plain) | **Eytzinger + prefetch** |
| --- | --- | --- | --- | --- |
| 100,000 | 32.37 ms | 36.42 ms | 32.43 ms | **26.81 ms** |
| 1,000,000 | **24.55 ms** | 36.83 ms | 32.20 ms | 28.38 ms |
| 10,000,000 | 128.41 ms | 173.41 ms | 141.02 ms | **76.68 ms** |
| 50,000,000 | 226.87 ms | 322.22 ms | 269.28 ms | **120.68 ms** |

> **The layout alone is not the win — the prefetch is.** Plain Eytzinger *loses* above 100k. Branchless was worst throughout.

## I/O Model Bounds

| Operation | RAM | **I/O model** |
| --- | --- | --- |
| Scan | Θ(n) | **Θ(n/B)** |
| Sort | Θ(n log n) | Θ((n/B) log_{M/B}(n/B)) |
| Binary search, sorted array | Θ(log n) | Θ(log n) — **B doesn't help** |
| **B-tree search** | Θ(log n) | **Θ(log_B n)** |

## The Recurring Pattern

Flat contiguous beats pointer-based at equal asymptotics, measured across this category:

| Comparison | Gap |
| --- | --- |
| `Vec` vs scattered linked list | **641×** |
| Bitset vs `Vec<bool>` | **79×** |
| Sorted `Vec` vs uncompressed trie | 11× |
| `BTreeMap` vs binary BST | ~2× |
| CSR vs `Vec<Vec<_>>` | 1.76× |

One mechanism, six observations.

## Eytzinger

```rust
let mut k = 1usize;
while k < a.len() {
    let p = k * 16;                                     // ← prefetch great-grandchildren
    if p < a.len() { unsafe { std::ptr::read_volatile(a.as_ptr().add(p)); } }
    k = 2 * k + (a[k] < x) as usize;                    // branchless descent
}
let j = k >> ((!k).trailing_zeros() + 1);               // last left-turn = lower bound
```

Layout: `a[1]` is root, children of `a[k]` are `a[2k]`, `a[2k+1]`. **No range scans; rebuild on insert.**

## Choose This When

| Use | For |
| --- | --- |
| **Sorted array + `binary_search`** | Default; hard to beat below ~10⁷ |
| **Eytzinger + prefetch** | Static table, n ≳ 10⁷, lookups dominate (**1.88×**) |
| B-tree layout | Also need updates or range scans |
| van Emde Boas | Multiple levels matter, can't tune per machine |
| Tiling / BLAS | Dense numeric kernels |
| Structure-of-arrays | A pass touches a subset of fields |
| Implicit (pointer-free) | Shape derivable from size (heap, Fenwick) |
| **Nothing** | Working set fits in L2, or it isn't in the profile |

## Rules of Thumb

- Reordering without exploiting predictability buys nothing — **prefetch or don't bother**.
- Profile first: confirm it's hot **and** miss-heavy, and that n exceeds cache.
- Tuned constants don't transfer between machines — detect at runtime or go cache-oblivious.
- **Branchless isn't automatically faster** — it trades a control dependency for a data dependency, killing latency hiding.
- Removing pointers removes both memory *and* dependent loads.
- Prefetch distance must be measured, not derived.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Eytzinger without prefetch | 0.84× — slower than `std::binary_search` |
| Layout work on a cache-resident structure | No change; the costs remain |
| Shipped a machine-tuned fanout/tile | Slower on different hardware |
| Branchless binary search | Worst of four variants at every size |
| Eytzinger where ranges are needed | Range scans impossible — array isn't sorted |

## Key References

- Khuong & Morin, ["Array Layouts for Comparison-Based Searching"](https://arxiv.org/abs/1509.05053) — the definitive measurement
- Frigo, Leiserson, Prokop & Ramachandran (1999) — cache-oblivious algorithms
- Aggarwal & Vitter (1988) — the I/O model
