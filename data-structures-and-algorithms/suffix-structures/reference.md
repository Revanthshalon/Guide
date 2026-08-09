# Suffix Structures — Quick Reference

## At a Glance

**Preprocess the *text*, not the pattern.** All suffixes sorted ⇒ every occurrence of P is the prefix of some suffix ⇒ **all matches form one contiguous block**. Two binary searches find it.

**SA invariant:** `sa` is a permutation with `text[sa[0]..] < … < text[sa[n-1]..]`.
**LCP invariant:** `lcp[i]` = LCP of suffixes `sa[i-1]` and `sa[i]`. **LCP of any two suffixes = RMQ over `lcp[i+1..=j]`.**

## The Numbers (2 M chars, measured)

| | Time |
| --- | --- |
| Build (comparison sort) | 99.24 ms |
| Naive scan per query | 5.03 ms |
| **SA binary search per query** | **2.50 µs** (**2,012×**) |

**Break-even ≈ 20 queries.** Below that, just scan.

## Complexity

| Operation | Suffix array | Suffix tree | Automaton | **FM-index** |
| --- | --- | --- | --- | --- |
| Build | Θ(n) SA-IS | Θ(n) | Θ(n) online | Θ(n) |
| **Space** | **4–8 B/char** | 20–40 B/char | ~2n states | **< 1 B/char** |
| Count | Θ(m log n) | Θ(m) | Θ(m) | Θ(m) |
| Locate | Θ(occ) | Θ(occ) | Θ(occ) | Θ(occ·sample) |

Suffix trees' Θ(m) loses in practice — the array is a flat cache-friendly `Vec<u32>`. **The flat array keeps winning** (cf. CSR, sorted `Vec` vs trie).

## What LCP Buys

| Question | Answer |
| --- | --- |
| Longest repeated substring | `max(lcp)` |
| Number of distinct substrings | `n(n+1)/2 − Σ lcp[i]` |
| Longest common substring of A, B | `A#B`, max `lcp[i]` across the boundary |
| LCP of any two suffixes | RMQ over `lcp` — Θ(1) with a sparse table |

## Snippets

```rust
// Search: one contiguous block, two binary searches
let lo = sa.partition_point(|&i| { let e=(i as usize+pat.len()).min(n); text[i as usize..e] <  pat[..] });
let hi = sa.partition_point(|&i| { let e=(i as usize+pat.len()).min(n); text[i as usize..e] <= pat[..] });
let occurrences = hi - lo;          // positions are sa[lo..hi]

// Kasai's LCP in Θ(n) — h drops by at most 1 per step (the amortization)
for i in 0..n {
    if rank[i] > 0 {
        let j = sa[rank[i]-1] as usize;
        while i+h < n && j+h < n && text[i+h] == text[j+h] { h += 1; }
        lcp[rank[i]] = h;
        if h > 0 { h -= 1; }
    } else { h = 0; }
}
```

## Choose This When

| Use | For |
| --- | --- |
| **`str::find`** | Text changes, or < ~20 queries |
| **Suffix array + LCP** | Static text, many queries — the default index |
| `divsufsort` / `suffix` crate | Θ(n) construction (don't hand-roll SA-IS) |
| **FM-index** | Huge text, memory-bound; compressed self-index |
| Suffix automaton | Online construction; substring counting |
| Suffix tree | Rarely — SA + LCP + RMQ replaces it |
| `fst` | The "text" is a **set of keys**, not one string |
| Aho-Corasick | Many known patterns, text not reused |

## Rules of Thumb

- Compute the break-even before indexing: `build / (scan − indexed)`.
- **Append a sentinel** smaller than every character before building.
- Memory is per **character**: `u32` SA = 4× the text; `u32` caps you at 4 GB.
- Memory-bound → FM-index (< 1 B/char), not a bigger machine.
- Byte offsets on UTF-8 can land mid-character — filter with `is_char_boundary`.
- Don't hand-roll SA-IS; prefix doubling (Θ(n log n)) is the safe DIY option.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Indexed for < 20 queries | Slower than scanning |
| No sentinel | Prefix-suffix ties sort wrong; SA-IS produces garbage |
| Suffix array on 1 GB text | 4 GB index; OOM or page-fault per probe |
| UTF-8 byte offset used to slice | "byte index is not a char boundary" panic |
| Hand-rolled SA-IS | Nearly-sorted array; passes small tests |

## Key References

- Manber & Myers (1993) — suffix arrays and prefix doubling
- Kasai et al. (2001) — LCP in Θ(n)
- Nong, Zhang & Chan (2009) — SA-IS
- Ferragina & Manzini (2000) — FM-index
