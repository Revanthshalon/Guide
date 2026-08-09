# Edit Distance & Alignment — Learning Notes

## Mental Model

**Edit distance measures how far apart two strings are by counting the cheapest sequence of edits that transforms one into the other.** Levenshtein distance allows insertion, deletion, and substitution, each costing 1.

It's the canonical two-sequence [dynamic programming](../dynamic-programming/learning.md), and the recurrence writes itself once you name the state — "the distance between the first *i* characters of A and the first *j* of B":

```
dp[i][j] = min( dp[i-1][j] + 1,                                  // delete a[i-1]
                dp[i][j-1] + 1,                                  // insert b[j-1]
                dp[i-1][j-1] + (a[i-1] != b[j-1]) as usize )     // substitute or match
```

Θ(n·m), and the three transitions *are* the three edit operations — the DP table is a direct encoding of the problem, which is why this is the standard example for teaching DP.

The second idea is the one that makes it practical at scale: **the DP values in a row change by at most ±1 between adjacent cells**, so a row can be encoded as two bitmasks (where it increased, where it decreased) rather than as integers. **Myers' bit-parallel algorithm** then advances an entire row with a handful of word operations, processing 64 DP cells per instruction. Measured, against a rolling-array classic DP:

| Pattern length | Text length | Classic DP | **Myers bit-parallel** | Ratio |
| --- | --- | --- | --- | --- |
| 32 | 100,000 | 24.02 ms | **1.26 ms** | **19×** |
| 64 | 1,000,000 | 241.85 ms | **3.96 ms** | **61×** |

Both produce identical distances (verified by assertion). The ratio grows with pattern length because bit-parallelism packs more cells per word — at 64 characters you get the full 64-way parallelism, which is the same "a word is 64 lanes" idea as [bit manipulation](../bit-manipulation/learning.md)'s measured 79× on bitsets.

The third framing worth carrying: **`diff` is not edit distance — it's LCS.** Line-based diff finds the longest common subsequence of lines and reports everything else as added or removed. Same DP family, different objective, and Myers' *diff* algorithm (a different algorithm by the same author) exploits the fact that real diffs are small.

## The Invariant

> `dp[i][j]` is the minimum number of edits transforming `a[0..i]` into `b[0..j]`.

Boundary conditions carry real meaning and are where the variants live:

- `dp[i][0] = i` — deleting everything.
- `dp[0][j] = j` — inserting everything.

Change those and you change the problem:

| `dp[0][j]` | Meaning | Use |
| --- | --- | --- |
| `= j` | Global alignment | Whole-string distance |
| `= 0` | **Free start in B** | Approximate *search*: find the best match of A anywhere in B |
| — | Free start *and* end | Local alignment (Smith-Waterman) |

That's not a footnote — **it's the single most common source of wrong answers here**, and it's exactly the bug I hit while writing this doc: a Myers implementation missing the `ph |= 1` that encodes `dp[0][j] = j` computed *semi-global* distance (14) instead of global (99,968). The two differ by orders of magnitude and both are "correct" for their respective problems.

The other structural fact:

> **Adjacent cells in the DP table differ by at most 1**, both horizontally and vertically.

This is what licenses the bit-parallel encoding, and also what makes the **banded** optimization valid: if you only care about distances ≤ k, every relevant cell lies within k of the diagonal, so you can compute a band of width 2k+1 in Θ(n·k).

## Mechanics

### The variants

| Distance | Operations | Notes |
| --- | --- | --- |
| **Levenshtein** | insert, delete, substitute | The default |
| Hamming | substitute only | Equal lengths required; Θ(n) |
| LCS distance | insert, delete (no substitute) | `diff` uses this |
| **Damerau-Levenshtein** | + transposition of adjacent | Typo correction — `teh` → `the` is 1, not 2 |
| Needleman-Wunsch | weighted, global | Bioinformatics: gap penalties per operation |
| **Smith-Waterman** | weighted, **local** | Best-matching *substring* pair; negatives clamped to 0 |
| Affine gaps | gap open ≠ gap extend | Realistic for biology; needs 3 DP layers |

Damerau-Levenshtein adds one transition (`dp[i-2][j-2] + 1` when the two characters are swapped) and matters for spell-checking, where transpositions are among the most common typos.

### Space and reconstruction

The rolling-array optimization from [dynamic programming](../dynamic-programming/learning.md) applies directly — measured there, an LCS table went from 17 MB to 11 KB and got slightly *faster* — but it destroys the ability to reconstruct the alignment.

**Hirschberg's algorithm** recovers reconstruction in Θ(min(n,m)) space at 2× the time: compute the forward DP for the first half and the reverse DP for the second half, find the column where their sum is minimal (that's where the optimal alignment crosses the midpoint), then recurse on the two halves. It's [divide & conquer](../divide-and-conquer/learning.md) applied to a DP table, and it's the standard answer when you need both the alignment and bounded memory.

### Myers' bit-parallel algorithm

```rust
// Pattern length <= 64. Encodes the DP row as two bitmasks: pv (where it increases),
// mv (where it decreases). One iteration advances a full row.
fn myers(pattern: &[u8], text: &[u8]) -> usize {
    let m = pattern.len();
    let mut peq = [0u64; 256];
    for (i, &c) in pattern.iter().enumerate() { peq[c as usize] |= 1u64 << i; }

    let (mut pv, mut mv) = (!0u64, 0u64);
    let mut score = m;                              // dp[m][0] = m
    let last = 1u64 << (m - 1);
    for &c in text {
        let eq = peq[c as usize];
        let xv = eq | mv;
        let xh = (((eq & pv).wrapping_add(pv)) ^ pv) | eq;
        let mut ph = mv | !(xh | pv);
        let mut mh = pv & xh;
        if ph & last != 0 { score += 1 } else if mh & last != 0 { score -= 1 }
        ph = (ph << 1) | 1;                         // ← the |1 encodes dp[0][j] = j (GLOBAL)
        mh <<= 1;
        pv = mh | !(xv | ph);
        mv = ph & xv;
    }
    score
}
```

Drop the `| 1` and you get **semi-global** alignment — the best match of the pattern anywhere in the text, which is what you want for approximate *search*. Both are useful; conflating them is the pitfall.

For patterns longer than 64, the same technique extends to multiple words with carry propagation, giving Θ(n·m/64).

### Choosing an approach

| Situation | Use |
| --- | --- |
| Both strings short (< 1000) | Classic DP with rolling rows |
| Pattern ≤ 64, long text | **Myers bit-parallel** — measured 19–61× |
| Only need "is distance ≤ k?" | **Banded DP** — Θ(n·k), skip cells outside the band |
| Need the alignment, bounded memory | Hirschberg's — Θ(min(n,m)) space, 2× time |
| Fuzzy search over many strings | **BK-tree** or a Levenshtein automaton |
| Line-based diff | Myers' *diff* algorithm (LCS-based, output-sensitive) |
| Biological sequences | Needleman-Wunsch / Smith-Waterman with affine gaps |

**Early termination** is worth knowing: if you only need to know whether the distance is ≤ k, the length difference `|n − m|` is a lower bound (so reject immediately if it exceeds k), and banding restricts the computation to Θ(n·k).

## Complexity

| Algorithm | Time | Space | Notes |
| --- | --- | --- | --- |
| Classic DP | Θ(n·m) | Θ(n·m) | Full table, reconstruction possible |
| DP, rolling rows | Θ(n·m) | **Θ(min(n,m))** | No reconstruction |
| **Myers bit-parallel** | **Θ(n·⌈m/64⌉)** | Θ(σ + m/64) | Measured **19–61×** faster |
| Banded (distance ≤ k) | **Θ(n·k)** | Θ(k) | Early exit if band exceeded |
| Hirschberg | Θ(n·m) | **Θ(min(n,m))** | 2× time, full reconstruction |
| Ukkonen (k-band + doubling) | Θ(n·k) expected | Θ(k) | k unknown: double until found |
| Myers' diff (LCS) | **Θ((n+m)·D)** | Θ(n+m) | D = size of the diff — output-sensitive |
| Hamming | Θ(n) | Θ(1) | Equal lengths only |

**Where the table misleads.** Myers bit-parallel is Θ(n·m/64) — *the same complexity class* as the classic DP, since 64 is a constant. The measured 19–61× is entirely constant factor, exactly like the bitset result in [bit manipulation](../bit-manipulation/learning.md). Asymptotic notation is blind to precisely the thing that makes it worth using.

Myers' *diff* algorithm's Θ((n+m)·D) is the important one for tooling: real diffs are small (D ≪ n), so it's nearly linear in practice — which is why `git diff` is fast on large files with small changes and slow on files that were rewritten.

## Use Cases

- **Spell checking and autocorrect** — Damerau-Levenshtein against a dictionary, usually with a BK-tree or a Levenshtein automaton to avoid comparing against every word.
- **`diff`, `git`, code review** — LCS over lines via Myers' diff algorithm; the output-sensitivity is what makes it usable.
- **Fuzzy search** — "did you mean?", record linkage, deduplicating customer records with typos.
- **Bioinformatics** — sequence alignment is the foundational operation: Needleman-Wunsch for global, Smith-Waterman for local, with affine gap penalties and substitution matrices (BLOSUM/PAM) replacing unit costs.
- **OCR and speech post-processing** — word error rate *is* edit distance over tokens.
- **Plagiarism detection** — alignment between documents, often after fingerprinting to find candidate regions.
- **Version control merges** — three-way merge is built on pairwise diffs.
- **Data cleaning** — matching addresses, names, and product titles across systems where exact keys don't exist.

## When to Use Which

| Reach for | When |
| --- | --- |
| Classic DP, rolling rows | Both strings short; you need only the distance |
| **Myers bit-parallel** | Pattern ≤ 64 chars, long text — **19–61× measured** |
| **Banded DP** | You only care whether distance ≤ k |
| Hirschberg | Need the alignment *and* bounded memory |
| BK-tree / Levenshtein automaton | Fuzzy lookup over a **dictionary**, not two strings |
| `fst` crate | Fuzzy search over a large static key set |
| Myers' diff | Line-based diff of files |
| Smith-Waterman + affine gaps | Biological sequences |
| Hamming | Equal-length strings, substitutions only — Θ(n) |

## Pitfalls in Depth

### Pitfall: Global vs semi-global boundary conditions

- **What goes wrong:** The DP or bit-parallel implementation initializes `dp[0][j] = 0` instead of `j` (or, in Myers, omits the `ph |= 1` after the shift). The result is the best match of A *anywhere in* B rather than the distance between A and B. The two answers can differ by orders of magnitude — measured while writing this doc, a 32-character pattern against a 100,000-character text gave **14** (semi-global) versus **99,968** (global).
- **Why it happens (the mechanism):** The boundary row *is* the problem specification, and it's one line buried in initialization. Both variants are legitimate and widely used — approximate search wants free start-in-text, whole-string distance doesn't — so neither result looks obviously wrong. In the bit-parallel form the distinction is a single `| 1`, which is easy to omit when transcribing from a paper that presents the search variant.
- **How to handle it in production, and why that works:** State which problem you're solving in a comment, then **verify any optimized implementation against the classic DP** on random inputs with an assertion — that's exactly what caught this bug here, immediately, before any of it reached the doc. Test with strings of very different lengths, where global and semi-global diverge maximally; equal-length test strings hide the difference.
- **Trade-offs of the fix:** Keeping a reference DP around is a few lines and a test dependency. There's no runtime cost since it lives in `#[cfg(test)]`, and it's the only reliable way to validate a bit-parallel implementation whose intermediate state is unreadable.

### Pitfall: Θ(n·m) on long strings

- **What goes wrong:** Edit distance is computed between two 100,000-character documents: 10¹⁰ cells. Even at 1 ns per cell that's 10 seconds, and the full table would be 20 GB. It works fine on the test fixtures (a few hundred characters) and hangs in production.
- **Why it happens (the mechanism):** The DP is quadratic in a way that's invisible from the call site — `edit_distance(a, b)` looks like a cheap utility. And the growth is in the *product*, so doubling both inputs quadruples the work.
- **How to handle it in production, and why that works:** Pick the tool by what you actually need. Only need to know whether the distance is ≤ k → **banded DP**, Θ(n·k), with an immediate reject when `|n − m| > k`. One string ≤ 64 characters → **Myers**, measured 19–61× faster. Comparing documents → don't use character-level edit distance at all; use line-level LCS (Myers' diff) or fingerprint-then-align.
- **Trade-offs of the fix:** Banding only answers the bounded question — if the true distance exceeds k it tells you "> k", not the value. Myers is capped at 64 characters per word (extendable with carries, at proportional cost). Line-level diff gives coarser output than character-level alignment.

### Pitfall: Comparing against every dictionary entry

- **What goes wrong:** A spell-checker computes edit distance from the misspelled word to all 100,000 dictionary entries on every keystroke. Each comparison is cheap, but 100,000 of them per keystroke is not, and it scales with dictionary size rather than with the answer.
- **Why it happens (the mechanism):** Edit distance is a *pairwise* operation, so the obvious lookup is a scan. But the vast majority of dictionary words are nowhere near the query, and computing their exact distance is wasted work — the same "preprocess the corpus" insight as [suffix structures](../suffix-structures/learning.md).
- **How to handle it in production, and why that works:** Index the dictionary. A **BK-tree** exploits the triangle inequality (edit distance is a metric) to prune whole subtrees: if `d(query, node) = x` and you want distance ≤ k, only children at distance `[x−k, x+k]` can qualify. A **Levenshtein automaton** compiles the query into an NFA accepting all strings within k edits and intersects it with a dictionary automaton — which is what the `fst` crate does, and it's fast enough to run per keystroke over millions of keys.
- **Trade-offs of the fix:** A BK-tree's pruning degrades as k grows (at large k almost everything qualifies) and it's sensitive to insertion order. A Levenshtein automaton has to be constructed per query, and its size grows with k. Both add a real index to maintain when the dictionary changes.

### Pitfall: Character-level distance on the wrong unit

- **What goes wrong:** Two source files are compared with character-level edit distance, and the result is a large number that says nothing useful — a reformatted file has enormous character-level distance and zero semantic change. Or edit distance is computed over UTF-8 *bytes*, so a single accented character counts as 2 edits and an emoji as 4.
- **Why it happens (the mechanism):** Edit distance operates on whatever sequence you hand it, and the choice of element is a modelling decision that the function signature doesn't surface. Bytes, `char`s, grapheme clusters, words, and lines are all legitimate units producing wildly different answers — the same four-level problem as [strings & text](../strings-and-text/learning.md).
- **How to handle it in production, and why that works:** Choose the unit to match the question. Human-visible similarity → **grapheme clusters** (`unicode-segmentation`), so `é` counts as one edit whether it's NFC or NFD. Code comparison → **lines** (that's what `diff` does) or tokens. Fuzzy name matching → `char`s after normalization and case folding. Then normalize to NFC first, or the same rendered text compares as different.
- **Trade-offs of the fix:** Grapheme segmentation costs a Unicode table and a pass over the input, and it makes the DP operate over a `Vec<&str>` rather than bytes — slower per cell, and Myers' bit-parallel trick no longer applies directly since the alphabet isn't a byte. For ASCII-dominant data the byte-level version is fine and much faster.

### Pitfall: Rolling rows, then needing the alignment

- **What goes wrong:** The space optimization is applied — two rows instead of the full table — and then the actual alignment is needed for display: which characters were inserted, deleted, substituted. The information was in the discarded rows and can't be recovered.
- **Why it happens (the mechanism):** Rolling rows keep only what's needed to compute the *next* row. Reconstruction requires walking backwards through the decisions from `dp[n][m]` to `dp[0][0]`, which needs the whole table. The optimization and the requirement are structurally opposed, and the requirement usually arrives after the optimization.
- **How to handle it in production, and why that works:** Decide up front whether you need the value or the witness. Value only → roll the rows. Witness needed and the table fits → keep it (a `u16` table for two 3,000-character strings is 17 MB, which is nothing). Witness needed and it doesn't fit → **Hirschberg's**, which gets Θ(min(n,m)) space at 2× the time by finding the midpoint crossing and recursing.
- **Trade-offs of the fix:** Hirschberg's is genuinely more complex — a forward pass, a reverse pass, a midpoint search, and recursion — and doubles the runtime. Storing a compact *decision* table (2 bits per cell rather than a full value) is a middle ground at 8× less memory than the value table.
