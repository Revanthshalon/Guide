# Suffix Structures — Learning Notes

## Mental Model

**Preprocess the *text* instead of the pattern.** Online matchers ([string matching](../string-matching/learning.md)) build a table from the pattern and stream the text — correct when the text is new each time. When the text is **fixed and queried repeatedly**, that's exactly backwards: build an index over the text once, then answer every query in time proportional to the *pattern*, not the text.

The core object is the **suffix array**: the starting positions of all n suffixes, sorted lexicographically. That single sorted array has a remarkable property:

> **Every occurrence of a pattern P is the prefix of some suffix — and since the array is sorted, all suffixes beginning with P form one contiguous block.**

So searching becomes two binary searches for the block's boundaries, and counting occurrences is a subtraction. Measured on a 2,000,000-character text:

| | Time |
| --- | --- |
| Build suffix array (comparison sort) | 99.24 ms |
| Naive scan per query | 5.03 ms |
| **Suffix-array binary search per query** | **2.50 µs** |
| | **2,012× per query** |

The build **amortizes after about 20 queries** (99.24 ms ÷ 5.03 ms). That ratio is the practical decision rule for the whole topic: below ~20 queries, scan; above it, index.

The second idea is that a sorted suffix array plus **LCP** (longest common prefix between adjacent suffixes) answers a surprising range of questions that have nothing to do with search — longest repeated substring, number of distinct substrings, longest common substring of two strings — because *adjacent suffixes in sorted order are the most similar ones*, so the LCP array captures all the repetition structure in Θ(n) space.

## The Invariant

**Suffix array:**

> `sa` is a permutation of `0..n` such that `text[sa[0]..] < text[sa[1]..] < … < text[sa[n-1]..]` lexicographically.

**LCP array:**

> `lcp[i]` = length of the longest common prefix of `text[sa[i-1]..]` and `text[sa[i]..]`.

Two consequences that generate most applications:

- **The LCP of any two suffixes `sa[i]` and `sa[j]` (i < j) is `min(lcp[i+1..=j])`** — a range-minimum query over the LCP array. So [sparse tables](../range-query-structures/learning.md) turn arbitrary suffix-pair LCP into Θ(1), which is the bridge that makes suffix arrays a general tool rather than just a search index.
- **`max(lcp)` is the longest repeated substring**, and `n(n+1)/2 − Σ lcp[i]` is the number of **distinct** substrings. Both fall out with no extra work.

**Suffix automaton (DAWG):**

> The minimal deterministic automaton recognizing all suffixes of the string. It has at most `2n − 1` states and `3n − 4` transitions — **linear**, despite representing all Θ(n²) substrings.

That size bound is the surprise: every substring corresponds to a path, and there are quadratically many substrings, yet the automaton is linear because substrings with identical "right contexts" share states.

## Mechanics

### Building the suffix array

| Method | Time | Notes |
| --- | --- | --- |
| Sort suffixes directly | Θ(n² log n) worst | What the measured 99.24 ms used — fine for moderate n |
| Prefix doubling (Manber-Myers) | Θ(n log² n) or Θ(n log n) | Sort by first 2^k characters, doubling k |
| **SA-IS / DC3** | **Θ(n)** | What production libraries use |

**Prefix doubling** is the one to understand: rank each suffix by its first character, then repeatedly combine — the rank by first 2^k characters is the pair `(rank_k[i], rank_k[i + 2^k])`, so one sort per doubling gives Θ(n log n) with `log n` rounds. It's the classic "combine two halves of information you already have" trick.

In Rust, use the `suffix` or `divsufsort` crates for Θ(n) construction rather than hand-rolling SA-IS, which is genuinely intricate.

### Searching

```rust
// All suffixes starting with `pat` form ONE contiguous block. Two binary searches.
let lo = sa.partition_point(|&i| {
    let e = (i as usize + pat.len()).min(text.len());
    text[i as usize..e] < pat[..]
});
let hi = sa.partition_point(|&i| {
    let e = (i as usize + pat.len()).min(text.len());
    text[i as usize..e] <= pat[..]
});
let occurrences = hi - lo;                 // and sa[lo..hi] are the positions
```

Θ(m log n) — each of the `log n` comparison steps compares up to m characters. With the LCP array and a little bookkeeping this improves to Θ(m + log n).

### Kasai's algorithm — LCP in Θ(n)

```rust
// Compute LCP in Θ(n) by walking suffixes in TEXT order, not sorted order.
let mut rank = vec![0usize; n];
for i in 0..n { rank[sa[i] as usize] = i; }
let (mut h, mut lcp) = (0usize, vec![0usize; n]);
for i in 0..n {
    if rank[i] > 0 {
        let j = sa[rank[i] - 1] as usize;
        while i + h < n && j + h < n && text[i + h] == text[j + h] { h += 1; }
        lcp[rank[i]] = h;
        if h > 0 { h -= 1; }                // ← the key: h drops by at most 1 per step
    } else { h = 0; }
}
```

The insight that makes it linear: if suffix `i` shares `h` characters with its predecessor, suffix `i+1` shares at least `h−1`. So `h` decreases at most n times total and increases at most n times — the same amortization argument as [two pointers](../two-pointers-and-sliding-window/learning.md).

### The structure family

| Structure | Space | Build | Strengths |
| --- | --- | --- | --- |
| **Suffix array** | 4–8 B/char | Θ(n) with SA-IS | Simple, cache-friendly, small |
| SA + LCP | 8–12 B/char | Θ(n) | Repeats, distinct substrings, pair LCP |
| Suffix tree | 20–40 B/char | Θ(n) (Ukkonen) | Θ(m) search; conceptually clearest |
| **Suffix automaton** | ~2n states | Θ(n) | Online construction; substring counting |
| **FM-index / BWT** | **< 1 B/char** | Θ(n) | Compressed self-index — the text itself is discardable |

**Suffix trees are rarely built in practice** — they're 3–5× the memory of a suffix array and much harder to implement, and a suffix array plus LCP plus an RMQ structure simulates one. The usual advice "use a suffix tree" is a teaching artifact.

**FM-indexes** are the production answer at scale: built on the Burrows-Wheeler transform, they store the index in *less* space than the original text while supporting count and locate queries. Every modern DNA aligner (bowtie, BWA) is an FM-index.

### What the LCP array buys, concretely

| Question | Answer |
| --- | --- |
| Longest repeated substring | `max(lcp)` |
| Number of distinct substrings | `n(n+1)/2 − Σ lcp[i]` |
| Longest common substring of A and B | Concatenate `A#B`, find max `lcp[i]` where `sa[i-1]` and `sa[i]` come from different sides |
| LCP of any two suffixes | RMQ over `lcp[i+1..=j]` — Θ(1) with a sparse table |
| k-th smallest substring | Walk the SA using LCP to skip counted prefixes |

## Complexity

| Operation | Suffix array | Suffix tree | Suffix automaton | FM-index |
| --- | --- | --- | --- | --- |
| Build | Θ(n) (SA-IS) | Θ(n) | Θ(n) online | Θ(n) |
| Space | **4–8 B/char** | 20–40 B/char | ~2n states | **< 1 B/char** |
| Count occurrences | Θ(m log n), Θ(m+log n) with LCP | **Θ(m)** | Θ(m) | Θ(m) |
| Locate occurrences | Θ(occ) | Θ(occ) | Θ(occ) | Θ(occ · sample rate) |
| Longest repeated substring | Θ(n) via LCP | Θ(n) | Θ(n) | — |
| Distinct substring count | Θ(n) via LCP | Θ(n) | Θ(n) | — |

**Where the table misleads.** The suffix tree's Θ(m) search beats the array's Θ(m log n) on paper and loses in practice: the array is a flat, cache-friendly `Vec<u32>` while the tree is a pointer-chasing structure 3–5× larger. This is the same contiguity argument that made CSR beat `Vec<Vec>` in [graph representations](../graph-representations/learning.md) and made the sorted `Vec` beat the trie in [tries & radix trees](../tries-and-radix-trees/learning.md) — **the flat array keeps winning**.

The other misleading part is the build cost. Measured, a comparison-based build took 99.24 ms for 2 M characters — 20 naive queries' worth. Θ(n) SA-IS is several times faster, but the amortization threshold is still the number that decides whether to index at all.

## Use Cases

- **Full-text search over a fixed corpus** — the canonical case; measured 2,012× per query over scanning.
- **Bioinformatics** — read alignment against a reference genome. FM-indexes (bowtie, BWA) index 3 GB of genome in ~1–2 GB and answer millions of queries.
- **Data compression** — the Burrows-Wheeler transform (bzip2) is computed from the suffix array; LZ77 factorization can be derived from it.
- **Plagiarism and clone detection** — longest common substring between documents, via the concatenation trick.
- **Duplicate detection in logs/code** — longest repeated substring is `max(lcp)`, one pass.
- **Autocomplete over a fixed dictionary** — though for pure prefix queries a sorted array or `fst` is simpler (see [tries](../tries-and-radix-trees/learning.md)).
- **Counting distinct substrings** — a surprisingly common subproblem in string analytics.
- **Longest palindromic substring** — via suffix array of `S#reverse(S)` plus LCP (though Manacher's is simpler for that specific problem).

## When to Use Which

| Reach for | When |
| --- | --- |
| **Online matcher (`str::find`)** | Text changes, or fewer than ~20 queries |
| **Suffix array + LCP** | Static text, many queries — **the default index** |
| `suffix` / `divsufsort` crate | You need Θ(n) construction |
| **FM-index** | Text is huge and memory-bound; want a compressed self-index |
| Suffix automaton | Building online (text arrives incrementally); substring counting |
| Suffix tree | Rarely — an SA + LCP + RMQ does the same in less memory |
| **`fst` crate** | The "text" is a *set of keys*, not one long string |
| [Aho-Corasick](../string-matching/learning.md) | Many known patterns, one pass, text not reused |

## Pitfalls in Depth

### Pitfall: Building an index for too few queries

- **What goes wrong:** A suffix array is built for a handful of searches. Measured, the build cost 99.24 ms on a 2 M-character text while a naive scan answered a query in 5.03 ms — so for fewer than ~20 queries the index is a net loss, and for one query it's 20× slower than just scanning.
- **Why it happens (the mechanism):** The index's per-query win is enormous (2,012×), which makes it feel unconditionally better. But the build is Θ(n) with a large constant and the win is per *query*, so the comparison is `build + q·query_fast` versus `q·query_slow` — a break-even at `build / (query_slow − query_fast)`.
- **How to handle it in production, and why that works:** Compute the break-even from your own numbers before indexing. Few queries or a changing text → `str::find`/`memmem`. Many queries against a fixed text → index. Many *known* patterns in one pass → [Aho-Corasick](../string-matching/learning.md), which needs no text index at all.
- **Trade-offs of the fix:** The break-even depends on text size and query count, both of which may be unknown at design time. A reasonable default is to scan until a query counter crosses a threshold, then build the index lazily — though that adds a mode switch and a latency spike on the crossing query.

### Pitfall: Underestimating memory

- **What goes wrong:** A suffix array over a 1 GB text needs a `u32` per character — 4 GB, four times the text — and that's the *cheapest* structure in the family. A suffix tree would need 20–40 GB. The process is OOM-killed, or the index doesn't fit in the cache hierarchy and every binary-search probe is a page fault.
- **Why it happens (the mechanism):** The index stores one entry per *character*, not per word or per document, so its size is proportional to the text and the constant is the index-type width. `u32` caps you at 4 GB of text; beyond that you need `u64` and the index doubles again.
- **How to handle it in production, and why that works:** Use an **FM-index** when memory-bound — it's a *self-index*, storing the text in compressed form and supporting search directly on the compression, typically under 1 byte per character. That's why genomic aligners can index a 3 GB genome in 1–2 GB. Otherwise, partition the text into blocks with a per-block index, or use `u32` and cap block size at 4 GB.
- **Trade-offs of the fix:** FM-indexes are substantially more complex, and *locating* occurrences (as opposed to counting them) requires a sampled suffix array whose sample rate trades memory against locate time. Block partitioning makes cross-block matches need special handling.

### Pitfall: Forgetting the sentinel

- **What goes wrong:** A suffix array is built without appending a unique smallest character (conventionally `$` or `\0`). Suffixes that are prefixes of other suffixes compare ambiguously — `"ana"` vs `"ana..."` — and depending on the comparison, sorting is wrong or the LCP computation walks past the end. Many construction algorithms (SA-IS, DC3) *require* the sentinel and produce garbage without it.
- **Why it happens (the mechanism):** Lexicographic order on suffixes of the same string has a subtlety: a proper prefix should sort *before* the longer string, and a naive character-by-character comparison that runs out of characters must handle that case explicitly. A sentinel smaller than every real character makes the shorter suffix terminate first and compare smaller, removing the special case entirely.
- **How to handle it in production, and why that works:** Append a sentinel byte that cannot appear in the text (`0` if the text is ASCII/UTF-8 without NULs) before building, and remember the array is then `n+1` long with `sa[0]` being the sentinel suffix. Library crates handle this internally — another argument for using one.
- **Trade-offs of the fix:** You must guarantee the sentinel doesn't occur in the text, which requires knowing the alphabet. For arbitrary binary data there is no unused byte, and the standard fix is to widen the alphabet (index as `u16`) or use an algorithm that handles the tie explicitly.

### Pitfall: Byte offsets on UTF-8 text

- **What goes wrong:** A suffix array is built over UTF-8 bytes, and a returned match position lands in the middle of a multi-byte character. Slicing the original `&str` at that offset **panics** with "byte index is not a char boundary", or the reported position is meaningless to a user counting characters.
- **Why it happens (the mechanism):** Suffix structures operate on bytes. UTF-8 is self-synchronizing, so a byte-level match of a valid UTF-8 pattern *is* a real character-level match — but the *suffixes* include ones starting mid-character, which are not meaningful text positions, and they participate in the sort.
- **How to handle it in production, and why that works:** Either filter results to char boundaries (`text.is_char_boundary(pos)` — see [strings & text](../strings-and-text/learning.md)), or build the index only over suffixes starting at char boundaries. For counting occurrences of a valid UTF-8 pattern, the byte-level count is already correct because UTF-8 prevents spurious mid-character matches — the issue is only with *reported positions*.
- **Trade-offs of the fix:** Filtering costs a check per result. Restricting to boundary suffixes changes the structure enough that library builders won't do it for you, so you'd hand-roll. The pragmatic answer is usually to keep the byte index internally and convert to a character index only at the display boundary.

### Pitfall: Hand-rolling SA-IS

- **What goes wrong:** Θ(n) suffix-array construction is implemented from the paper. It's one of the more intricate algorithms in common use — LMS substrings, induced sorting, recursive reduction — and a subtle bug produces a *nearly* sorted array that passes small tests and fails on structured input.
- **Why it happens (the mechanism):** The Θ(n log n) comparison build is easy and obviously correct (measured, 99.24 ms for 2 M characters), so the temptation is to "just make it linear". SA-IS's correctness argument is genuinely difficult, and its failure mode is a wrong permutation rather than a crash.
- **How to handle it in production, and why that works:** Use `divsufsort` or `suffix`. If you must hand-roll, start with the simple comparison sort, then prefix doubling (Θ(n log n), far easier to get right), and property-test any faster version against the simple one on random and structured inputs — including all-same-character, highly periodic, and Fibonacci strings, which are the classic adversaries.
- **Trade-offs of the fix:** A crate dependency, and less control over the memory layout if you're integrating with a custom index. The Θ(n log n) fallback is usually fine — at 2 M characters it was 99 ms, and construction is a one-time cost by definition.
