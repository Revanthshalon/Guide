# String Matching — Quick Reference

## At a Glance

Every matcher answers one question: **after a mismatch, how far can I safely shift?**

**KMP invariant:** `f[i]` = longest proper prefix of `p[0..=i]` that is also a suffix. The **text pointer never rewinds** ⇒ Θ(n) and streamable.
**Aho-Corasick invariant:** trie + failure links to the longest suffix that is a trie node, **plus output links** so suffix patterns are reported too.

## The Numbers (19 MB haystack, measured)

| Pattern | Naive | KMP | Rabin-Karp | **`str::find`** |
| --- | --- | --- | --- | --- |
| 4 B | 263.25 µs | 224.33 µs | 636.46 µs | **87.83 µs** |
| 16 B | 38.43 ms | 42.14 ms | 121.76 ms | **8.79 ms** |
| 64 B | 31.93 ms | 43.80 ms | 130.00 ms | **4.87 ms** |

**std beats hand-rolled KMP by 2.6–9.0×**, and **naive beats KMP on random text** — mismatches occur at char 1, so KMP's bookkeeping buys a guarantee it never needs.

Adversarial (`aaaa…a` / `aaa…ab`, 2 MB): naive 49.99 ms · KMP **7.71 ms** (6×) · `str::find` **2.58 ms**.

> **KMP's value is the worst-case guarantee, not speed.** Learn the failure function; ship `str::find`.

## Complexity

| Algorithm | Preprocess | Search | Space |
| --- | --- | --- | --- |
| Naive | — | Θ(n·m) worst, ~Θ(n) typical | Θ(1) |
| **KMP** | Θ(m) | **Θ(n)** guaranteed | Θ(m) |
| Z-algorithm | Θ(n+m) | Θ(n+m) | Θ(n+m) |
| Boyer-Moore | Θ(m+σ) | **sublinear typical** | Θ(m+σ) |
| Two-way (std) | Θ(m) | Θ(n) | **Θ(1)** |
| Rabin-Karp | Θ(m) | Θ(n) expected | Θ(1) |
| **Aho-Corasick** | Θ(Σ\|pᵢ\|) | **Θ(n + occ)**, *any* k | Θ(Σ\|pᵢ\|·σ) |

The table ranks algorithms almost **inversely** to measured performance. Measure, or use the library.

## Choose This When

| Use | For |
| --- | --- |
| **`str::find` / `memchr::memmem`** | Single literal — **the default** |
| **`aho-corasick`** | Many patterns, one pass (Θ(n) regardless of k) |
| `regex` | Genuinely regular, not literal |
| **KMP hand-written** | Streaming input that can't rewind |
| Z-algorithm | Periodicity/border problems — often clearer than KMP |
| Rabin-Karp | Many equal-length patterns; 2-D; fingerprinting |
| **Suffix array / FM-index** | **Static text**, many queries (amortizes after ~20) |

## KMP

```rust
fn failure(p: &[u8]) -> Vec<usize> {
    let mut f = vec![0; p.len()]; let mut k = 0;
    for i in 1..p.len() {
        while k > 0 && p[k] != p[i] { k = f[k-1]; }
        if p[k] == p[i] { k += 1; }
        f[i] = k;
    }
    f
}
// Search is the same loop against the text; `i` never decreases.
```

## Rules of Thumb

- Default to `str::find`; use `memchr::memmem::Finder` for a repeated needle.
- k patterns → Aho-Corasick, never k passes.
- Rabin-Karp: a hash match is a **candidate** — always verify.
- Random base + large prime modulus makes collisions unpredictable to an attacker.
- Repeatedly searching the *same* text → preprocess the **text**, not the pattern.
- Boyer-Moore gets faster with longer patterns; KMP doesn't.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Hand-rolled KMP | 2.6–9.0× slower than `str::find`, more code |
| Aho-Corasick without output links | Misses patterns that are suffixes of others |
| Rabin-Karp without verification | False positives on hash collisions |
| Weak/known modulus | Attacker forces Θ(n·m) |
| Online matcher on static text | Θ(q·n) instead of Θ(q·m log n) — 2,012× per query |
| Byte offsets into UTF-8 | Panic on non-char-boundary slicing |

## Key References

- Knuth, Morris & Pratt (1977) · Boyer & Moore (1977) · Aho & Corasick (1975)
- Crochemore & Perrin, "Two-way string matching" (1991) — what std uses
- [`memchr`](https://docs.rs/memchr/) · [`aho-corasick`](https://docs.rs/aho-corasick/) — read why they beat the classics
