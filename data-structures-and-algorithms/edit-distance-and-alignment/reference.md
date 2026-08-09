# Edit Distance & Alignment — Quick Reference

## At a Glance

Cheapest sequence of edits transforming A into B. The canonical two-sequence DP — the three transitions **are** the three operations.

```
dp[i][j] = min( dp[i-1][j] + 1,                              // delete
                dp[i][j-1] + 1,                              // insert
                dp[i-1][j-1] + (a[i-1] != b[j-1]) )          // substitute/match
```

**Invariant:** `dp[i][j]` = min edits from `a[0..i]` to `b[0..j]`. **Adjacent cells differ by at most 1** — this licenses bit-parallelism and banding.

## Boundary Conditions ARE the Problem

| `dp[0][j]` | Problem |
| --- | --- |
| `= j` | **Global** — whole-string distance |
| `= 0` | **Semi-global** — best match of A anywhere in B |
| free both ends | **Local** (Smith-Waterman) |

Measured divergence: 32-char pattern vs 100k text → **14** semi-global vs **99,968** global. In Myers, the difference is one `| 1`.

## The Number

Classic rolling-row DP vs Myers bit-parallel (measured, identical results):

| Pattern | Text | DP | **Myers** | Ratio |
| --- | --- | --- | --- | --- |
| 32 | 100,000 | 24.02 ms | **1.26 ms** | **19×** |
| 64 | 1,000,000 | 241.85 ms | **3.96 ms** | **61×** |

Θ(n·m/64) is the same class as Θ(n·m) — **entirely constant factor**, like bitsets' 79×.

## Complexity

| Algorithm | Time | Space |
| --- | --- | --- |
| Classic DP | Θ(n·m) | Θ(n·m) |
| Rolling rows | Θ(n·m) | **Θ(min(n,m))** — no reconstruction |
| **Myers bit-parallel** | Θ(n·⌈m/64⌉) | Θ(σ + m/64) |
| **Banded (d ≤ k)** | **Θ(n·k)** | Θ(k) |
| Hirschberg | Θ(n·m), 2× | **Θ(min(n,m))** + reconstruction |
| **Myers' diff (LCS)** | **Θ((n+m)·D)** | Θ(n+m) — output-sensitive |
| Hamming | Θ(n) | Θ(1) |

## Variants

| Distance | Operations |
| --- | --- |
| **Levenshtein** | insert, delete, substitute |
| Hamming | substitute only, equal lengths |
| LCS distance | insert, delete — **what `diff` uses** |
| **Damerau-Levenshtein** | + adjacent transposition (`teh`→`the` = 1) |
| Smith-Waterman | weighted, **local**, negatives clamped to 0 |
| Affine gaps | open ≠ extend; 3 DP layers |

## Myers Core

```rust
let xv = eq | mv;
let xh = (((eq & pv).wrapping_add(pv)) ^ pv) | eq;
let (mut ph, mut mh) = (mv | !(xh | pv), pv & xh);
if ph & last != 0 { score += 1 } else if mh & last != 0 { score -= 1 }
ph = (ph << 1) | 1;          // ← |1 = GLOBAL. Omit it for semi-global search.
mh <<= 1;
pv = mh | !(xv | ph);  mv = ph & xv;
```

## Choose This When

| Use | For |
| --- | --- |
| Rolling-row DP | Both strings short; distance only |
| **Myers bit-parallel** | Pattern ≤ 64, long text |
| **Banded DP** | Only need "is distance ≤ k?" |
| Hirschberg | Alignment **and** bounded memory |
| **BK-tree / Levenshtein automaton / `fst`** | Fuzzy lookup over a **dictionary** |
| Myers' diff | Line-based file diff |
| Smith-Waterman + affine | Biological sequences |

## Rules of Thumb

- State global vs semi-global in a comment; **verify optimized versions against the classic DP**.
- Reject immediately if `|n − m| > k`.
- Pick the **unit** deliberately: bytes / chars / graphemes / words / lines give different answers.
- Normalize to NFC before comparing human text.
- Rolling rows lose reconstruction — decide up front.
- Dictionary lookup ⇒ index it; don't scan all entries.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Wrong `dp[0][j]` / missing `\| 1` | Semi-global answer where global was wanted (14 vs 99,968) |
| Θ(n·m) on 100k-char documents | 10¹⁰ cells; hangs in prod, fine in tests |
| Scanning the whole dictionary | Scales with dictionary, not with the answer |
| Byte-level on UTF-8 | `é` counts as 2 edits, emoji as 4 |
| Un-normalized Unicode | Identical-looking strings have nonzero distance |
| Rolled rows, then needed the path | Reconstruction information gone |

## Key References

- Myers (1999) — bit-vector algorithm (the 19–61×)
- Myers (1986) — the O(ND) **diff** algorithm (different paper, same author)
- Hirschberg (1975) — linear-space reconstruction
- Smith & Waterman (1981) — local alignment
