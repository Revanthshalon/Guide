# String Matching — Learning Notes

## Mental Model

**Every string matcher is an answer to one question: after a mismatch, how far can I safely shift?**

The naive algorithm shifts by 1 and rechecks everything — Θ(n·m) worst case. Every improvement is a way of *precomputing* how far you can jump using information you already have:

- **KMP** precomputes, for each prefix of the pattern, the longest proper prefix that is also a suffix. On a mismatch it shifts so that known-matching characters realign — never re-examining a text character. Θ(n + m), guaranteed.
- **Boyer-Moore** scans the pattern **right to left** and uses the mismatched *text* character to shift, often skipping m characters at once. Sublinear in practice; the basis of most fast matchers.
- **Rabin-Karp** compares hashes instead of characters, using a [rolling hash](../hashing-techniques/learning.md) to slide in Θ(1). Θ(n + m) expected, and it generalizes to *multiple* patterns.
- **Aho-Corasick** builds a trie of all patterns plus failure links — KMP generalized to many patterns at once, in one pass.

Now the finding that should reset expectations, measured on a 19 MB haystack with the pattern placed near the end so every algorithm scans nearly everything:

| Pattern | Naive | KMP | Rabin-Karp | **`str::find`** |
| --- | --- | --- | --- | --- |
| 4 B | 263.25 µs | 224.33 µs | 636.46 µs | **87.83 µs** |
| 16 B | 38.43 ms | 42.14 ms | 121.76 ms | **8.79 ms** |
| 64 B | 31.93 ms | 43.80 ms | 130.00 ms | **4.87 ms** |

Two things fall out, both counter to the textbook framing:

1. **std's `find` beats hand-rolled KMP by 2.6–9.0×**, and the gap *widens* with pattern length. It uses a two-way algorithm with a SIMD `memchr` prefilter — it skips ahead in 16- or 32-byte chunks looking for a rare byte, rather than examining every character.
2. **Naive beats KMP on random text** at 16 B and 64 B patterns. On text where mismatches happen at the first character almost every time, naive's inner loop exits immediately and its outer loop is a tight scan, while KMP pays failure-table bookkeeping per character for a guarantee it never needs.

So the honest summary: **KMP's value is its worst-case guarantee, not its speed.** On adversarial input — haystack `aaaa…a`, pattern `aaa…ab` — that guarantee is exactly what you want:

| Adversarial | Naive | KMP | `str::find` |
| --- | --- | --- | --- |
| 2 MB, 1000-char pattern | 49.99 ms | **7.71 ms** (6× faster) | **2.58 ms** |

**Use `str::find` / `memchr::memmem`. Learn KMP for the failure function**, which reappears in Aho-Corasick, in Z-algorithm applications, and in periodicity problems — not because you'll ship it.

## The Invariant

**KMP's failure function:**

> `f[i]` = the length of the longest proper prefix of `p[0..=i]` that is also a suffix of `p[0..=i]`.

That's the whole algorithm. When matching breaks at pattern position `k`, everything in `p[0..k]` matched the text, so the longest prefix-suffix `f[k-1]` is *already* aligned — you resume at pattern position `f[k-1]` without moving the text pointer at all. **The text pointer never goes backwards**, which is what gives Θ(n) and what makes KMP streamable.

The amortization is the same argument as [monotonic stack](../monotonic-stack-and-queue/learning.md): `k` increases at most n times and each `while k > 0 { k = f[k-1] }` step decreases it, so total work is Θ(n).

**Z-algorithm:**

> `z[i]` = the length of the longest substring starting at `i` that is also a prefix of the whole string.

Computing `z` for `pattern + '\0' + text` finds every occurrence (wherever `z[i] == m`). It's often easier to reason about than the failure function and solves the same problems.

**Aho-Corasick:**

> A trie of all patterns, where each node's **failure link** points to the longest proper suffix of that node's string which is also a node in the trie. Plus **output links** so a node reports all patterns ending there.

The output links matter: matching `she` must also report `he` if `he` is a pattern. Forgetting them is the classic Aho-Corasick bug.

## Mechanics

### KMP

```rust
fn failure(p: &[u8]) -> Vec<usize> {
    let mut f = vec![0; p.len()];
    let mut k = 0;
    for i in 1..p.len() {
        while k > 0 && p[k] != p[i] { k = f[k - 1]; }   // fall back along the chain
        if p[k] == p[i] { k += 1; }
        f[i] = k;
    }
    f
}

fn search(h: &[u8], p: &[u8]) -> Option<usize> {
    let f = failure(p);
    let mut k = 0;
    for i in 0..h.len() {                                // i NEVER decreases
        while k > 0 && p[k] != h[i] { k = f[k - 1]; }
        if p[k] == h[i] { k += 1; }
        if k == p.len() { return Some(i + 1 - p.len()); }
    }
    None
}
```

Note the two loops are structurally identical — building the table is KMP matching the pattern against itself.

### Why Boyer-Moore and std's matcher are fast

Boyer-Moore compares the pattern **right to left**. If the text character aligned with the pattern's last position doesn't occur in the pattern at all, you can shift by the entire pattern length — examining 1 character to skip m. That's why it's *sublinear* on typical text: longer patterns mean bigger skips, which is exactly the trend visible in the measured table (std got faster as the pattern grew: 8.79 ms → 4.87 ms).

std's `find` uses the **two-way algorithm** (linear worst case, constant space) with a `memchr` prefilter that uses SIMD to scan for a candidate byte 16–32 bytes at a time. The prefilter is where most of the win comes from: most positions are rejected without ever entering the comparison loop.

### Aho-Corasick — k patterns in one pass

```
build a trie of all patterns
BFS the trie; for each node, failure[node] = the trie node for the longest proper
    suffix of node's string; inherit output lists along failure links
scan the text once, following goto/failure edges; report outputs at each node
```

Θ(n + total pattern length + occurrences) — **independent of the number of patterns**. Running k separate searches is Θ(k·n). For a spam filter with 10,000 keywords over a 1 MB document, that's the difference between one pass and ten thousand.

Use the `aho-corasick` crate — it's the same one that powers `regex`'s literal optimizations, and it has SIMD-accelerated variants for small pattern sets.

### Choosing

| Situation | Use |
| --- | --- |
| Single pattern, Rust | **`str::find` / `memchr::memmem`** — measured 2.6–9.0× faster than KMP |
| Many patterns, one pass | **`aho-corasick`** — Θ(n) regardless of k |
| Streaming, can't buffer | **KMP** — text pointer never rewinds |
| Multiple patterns, hashing available | Rabin-Karp with a hash set of pattern hashes |
| 2-D pattern matching | Rabin-Karp generalizes naturally to rectangles |
| Need all occurrences repeatedly, static text | **[Suffix structures](../suffix-structures/learning.md)** — build once, query in Θ(m log n) |
| Fuzzy / approximate | [Edit distance](../edit-distance-and-alignment/learning.md), or Myers bit-parallel |

That last row is the important cross-reference: if you're searching the *same* text many times, the right move is to preprocess the **text** (suffix array) rather than the pattern.

## Complexity

| Algorithm | Preprocess | Search | Space | Worst case |
| --- | --- | --- | --- | --- |
| Naive | — | Θ(n·m) worst, ~Θ(n) typical | Θ(1) | Θ(n·m) |
| **KMP** | Θ(m) | **Θ(n)** | Θ(m) | **Θ(n)** guaranteed |
| Z-algorithm | Θ(n+m) | Θ(n+m) | Θ(n+m) | Θ(n+m) |
| Boyer-Moore | Θ(m + σ) | **sublinear typical**, Θ(n·m) worst | Θ(m + σ) | Θ(n·m) |
| Boyer-Moore-Horspool | Θ(m + σ) | sublinear typical | Θ(σ) | Θ(n·m) |
| Two-way (std) | Θ(m) | **Θ(n)** | **Θ(1)** | Θ(n) |
| Rabin-Karp | Θ(m) | Θ(n) expected | Θ(1) | Θ(n·m) on collisions |
| **Aho-Corasick** | Θ(Σ\|pᵢ\|) | **Θ(n + occ)** | Θ(Σ\|pᵢ\|·σ) | Θ(n + occ) |

**Where the table misleads, badly.** KMP's Θ(n) and naive's Θ(n·m) suggest KMP always wins. Measured, naive was *faster* than KMP on random 16 B and 64 B patterns, because the worst case essentially never occurs in non-adversarial text while KMP's per-character bookkeeping always does. The Θ notation captures the guarantee and misses the common case entirely.

Likewise Boyer-Moore's Θ(n·m) worst case looks worse than KMP's Θ(n), yet it's the family that real matchers derive from — because *sublinear typical* beats *linear guaranteed* when the input isn't adversarial.

**The lesson generalizes:** for string matching, the asymptotic table ranks algorithms almost inversely to how they perform. Measure, or use the library that already did.

## Use Cases

- **`grep`, ripgrep, editors** — literal search is Boyer-Moore/two-way with SIMD prefilters; ripgrep's speed comes from `memchr` and `aho-corasick`, not from a cleverer asymptotic algorithm.
- **Regex engines** — literal prefixes and required substrings are extracted and matched with Aho-Corasick to prefilter, so the automaton only runs on candidate positions.
- **Intrusion detection / antivirus** — thousands of signatures scanned in one pass: Aho-Corasick is the canonical application (Snort, ClamAV).
- **Spam and content filtering** — keyword lists over documents; again Aho-Corasick.
- **Bioinformatics** — exact matching of reads is usually done with [suffix structures](../suffix-structures/learning.md) or FM-indexes rather than online matchers, because the reference text is fixed.
- **Log processing** — `memchr` for delimiters is the single highest-leverage optimization in a line-oriented parser ([strings & text](../strings-and-text/learning.md)).
- **Plagiarism / near-duplicate detection** — Rabin-Karp fingerprinting with winnowing.
- **Streaming protocol parsing** — KMP's never-rewind property matters when you can't buffer the whole stream.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`str::find` / `memchr::memmem`** | Single literal pattern — **the default**, measured 2.6–9.0× over KMP |
| **`aho-corasick`** | Many patterns, one pass over the text |
| `regex` | Patterns are genuinely regular expressions, not literals |
| **KMP (hand-written)** | Streaming input that can't rewind; or you need the failure function itself |
| Z-algorithm | Periodicity, borders, prefix-function problems — often clearer than KMP |
| Rabin-Karp | Multiple patterns of equal length; 2-D matching; fingerprinting |
| [Suffix array / FM-index](../suffix-structures/learning.md) | **Static text**, many different queries |
| [Edit distance](../edit-distance-and-alignment/learning.md) | Approximate matching |

## Pitfalls in Depth

### Pitfall: Hand-rolling KMP instead of using the library

- **What goes wrong:** KMP is implemented because it's Θ(n + m) and naive is Θ(n·m). The result is **2.6–9.0× slower than `str::find`** (measured), and on random text it's slower than the naive loop it replaced. It's also ~30 lines of index-manipulation with an easy-to-get-wrong failure function, versus one method call.
- **Why it happens (the mechanism):** The asymptotic comparison is real and it points the wrong way. Real matchers win by *skipping* rather than by never-rewinding: a SIMD `memchr` prefilter rejects 16–32 bytes per instruction, and Boyer-Moore-style shifts skip up to m characters per examined byte. KMP examines every text character exactly once, which is optimal in the *comparison* model and mediocre on hardware that can compare 32 bytes at a time.
- **How to handle it in production, and why that works:** Use `str::find`, or `memchr::memmem::Finder` when searching the same needle repeatedly (it precomputes the prefilter). These implement the two-way algorithm, so you keep the Θ(n) worst-case guarantee *and* get the SIMD skip — measured 2.58 ms on the adversarial case where naive took 49.99 ms.
- **Trade-offs of the fix:** You give up control over the algorithm, which matters in `no_std` contexts or when matching over a non-slice source (a stream, a rope). KMP's never-rewind property is genuinely required for streaming input, and that's the case where hand-writing it is correct.

### Pitfall: Assuming Θ(n·m) worst case means naive is unusable

- **What goes wrong:** The naive matcher is rejected on sight because of its quadratic bound, and replaced with something more complex. Measured on random text, naive was **faster than KMP** at 16 B (38.43 vs 42.14 ms) and 64 B (31.93 vs 43.80 ms) patterns.
- **Why it happens (the mechanism):** The Θ(n·m) worst case requires the pattern to *repeatedly almost match* — long runs of the same character, or highly periodic text. In natural text, code, or random data, a mismatch occurs at the first or second character virtually always, so the inner loop runs ~1 iteration and the algorithm is effectively Θ(n) with a tiny constant.
- **How to handle it in production, and why that works:** Ask whether the input can be adversarial or highly repetitive. Attacker-supplied patterns and text (a search box, a log filter) → use the library, which has a linear guarantee. Internal, non-repetitive data → naive is fine and the library is still better. The point isn't "use naive", it's that the quadratic bound alone isn't the reason to avoid it.
- **Trade-offs of the fix:** Reasoning about whether your input is adversarial is easy to get wrong, and the failure mode (a request that pins a core for seconds) is severe. Since the library is both faster *and* has the guarantee, there's no real reason to take the risk.

### Pitfall: Aho-Corasick without output links

- **What goes wrong:** The automaton reports a match only at nodes marked as pattern ends, so overlapping patterns are missed: with patterns `{she, he, hers}`, scanning `"shers"` reports `she` and `hers` but not `he`, because the `he` node isn't on the path taken. Occurrences are silently dropped, and the gap only appears when one pattern is a suffix of another.
- **Why it happens (the mechanism):** When the automaton is at the node for `she`, the string `he` is a *suffix* of the current match and is reachable via failure links — but only if you follow them looking for outputs. The node itself isn't marked, so a naive "is this node terminal?" check misses it.
- **How to handle it in production, and why that works:** During the BFS that builds failure links, propagate an **output list**: a node's outputs are its own pattern (if any) plus the outputs of its failure target. Then reporting at each position is a walk of that precomputed list. Use the `aho-corasick` crate, which handles this plus overlapping/leftmost/leftmost-longest match semantics — which are themselves a decision you must make explicitly.
- **Trade-offs of the fix:** Output lists cost memory proportional to the total overlap among patterns, and following them makes reporting Θ(occurrences) rather than Θ(1) per position — which is correct but means a pathological pattern set (all suffixes of one string) produces quadratic *output*, not quadratic *search*.

### Pitfall: Rabin-Karp without verification, or with a weak modulus

- **What goes wrong:** Hash equality is treated as a match without comparing the actual characters, so hash collisions produce false positives. Or a small modulus (or a power of two) is used, making collisions frequent — and an attacker who knows the modulus can construct inputs that collide on every position, degrading the search to Θ(n·m).
- **Why it happens (the mechanism):** Rabin-Karp's Θ(n) is *expected*, conditional on the hash spreading well. A hash match is a *candidate*, exactly as in [hashing techniques](../hashing-techniques/learning.md) — the verification step is what makes it correct, and it's cheap because it rarely runs.
- **How to handle it in production, and why that works:** Always verify a hash match with a real comparison. Use a large prime modulus and a random base chosen at runtime, which makes collisions unpredictable to an attacker — the same expected-vs-average distinction as randomized quicksort in [complexity analysis](../complexity-analysis/learning.md). Measured, Rabin-Karp was the slowest of the four algorithms anyway (130 ms vs `str::find`'s 4.87 ms), so single-pattern use is rarely justified.
- **Trade-offs of the fix:** Verification costs Θ(m) per candidate, which is fine at a low collision rate and terrible at a high one — so the modulus choice still matters. Rabin-Karp earns its place for *multiple equal-length patterns* and 2-D matching, where its structure is genuinely advantageous, not for the single-pattern case.

### Pitfall: Searching the same text repeatedly with an online matcher

- **What goes wrong:** A document is searched for hundreds of different patterns, each time with a fresh Θ(n) scan — Θ(q·n) total. Measured in [suffix structures](../suffix-structures/learning.md): a naive scan of a 2 M-character text took 5.03 ms per query, while a suffix-array binary search took **2.50 µs — 2,012×** — after a one-time 99.24 ms build.
- **Why it happens (the mechanism):** Online matchers preprocess the **pattern**; the text is streamed. That's the right trade when the text is new each time and wrong when the text is fixed — then the text is the thing worth preprocessing, and the cost amortizes across queries.
- **How to handle it in production, and why that works:** Build a [suffix array](../suffix-structures/learning.md) (or an FM-index for compressed search) once, then answer each query in Θ(m log n). Measured, the build amortized after roughly **20 queries** (99.24 ms build ÷ 5.03 ms per naive query). For many *known* patterns against one text, Aho-Corasick in a single pass is simpler still.
- **Trade-offs of the fix:** A suffix array costs 4–8 bytes per input character in memory and a real build step, and it's static — a changed text means a rebuild. Below the amortization threshold, online matching is genuinely better.
