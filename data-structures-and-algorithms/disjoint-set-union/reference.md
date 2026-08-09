# Disjoint Set Union — Quick Reference

## At a Glance

Answers **one** question — "same group?" — and supports **one** modification — "merge". No split, no delete, no enumerate. In exchange: **effectively Θ(1)** on a flat integer array.

**Invariant:** every element has a `parent`; following parents terminates at a **root**; same set **iff** same root. The tree shape is unconstrained — which is why `find` may rewrite it freely.

## The Number

n elements, 2n random unions, then n finds (measured):

| n | Neither | By rank | Compression | **Both** |
| --- | --- | --- | --- | --- |
| 10,000 | 73.6 ms | 0.5 | 0.5 | **0.2** |
| 50,000 | 2,286.1 ms | 2.0 | 2.2 | **1.1** |
| 200,000 | **66,767 ms** | 9.1 | 10.6 | **4.6** |
| 1,000,000 | >60,000 ms | 50.5 | 67.2 | **25.7** |

**14,500× at n = 200k**, from two one-line changes. Naive scales **quadratically**.

## Complexity

| Operation | Naive | By rank | Compression | **Both** |
| --- | --- | --- | --- | --- |
| `find` / `union` / `same` | Θ(n) | Θ(log n) | Θ(log n) am. | **Θ(α(n))** |
| Space | Θ(n) | Θ(n) | Θ(n) | Θ(n) |

α(n) ≤ 4 for any writable n. ~8 ns/op measured including cache misses.

## The Implementation

```rust
pub fn find(&mut self, x: u32) -> u32 {           // ITERATIVE — recursion can overflow
    let mut root = x;
    while self.parent[root as usize] != root { root = self.parent[root as usize]; }
    let mut cur = x;                               // path compression
    while self.parent[cur as usize] != root {
        let next = self.parent[cur as usize];
        self.parent[cur as usize] = root;
        cur = next;
    }
    root
}

pub fn union(&mut self, a: u32, b: u32) -> bool {  // MUST return bool
    let (ra, rb) = (self.find(a), self.find(b));
    if ra == rb { return false; }                  // already merged — cycle detected
    let (hi, lo) = if self.rank[ra as usize] >= self.rank[rb as usize] { (ra, rb) } else { (rb, ra) };
    self.parent[lo as usize] = hi;                 // union by rank
    if self.rank[hi as usize] == self.rank[lo as usize] { self.rank[hi as usize] += 1; }
    self.count -= 1;
    true
}
```

`rank` fits in a **`u8`** (rank ≤ log₂ n).

## Variants

| Variant | Adds | Cost |
| --- | --- | --- |
| **Union by size** | set sizes for free | same bound — usually preferred over rank |
| **Rollback** | undo unions | **must drop path compression** → Θ(log n) |
| **Weighted** | offset to parent (`a − b = d`) | offsets must be fixed during compression |
| Small-to-large | actual member lists | Θ(n log n) total |

## Choose This When

| Use | For |
| --- | --- |
| **DSU** | Merge-only grouping; "same set?"; never split or enumerate |
| DSU by size | Same + you want set sizes |
| Rollback DSU | Offline dynamic connectivity (handles deletions) |
| Weighted DSU | Relations with offsets, parity constraints |
| BFS/DFS | You need components' contents, paths, or structure |
| `HashMap<K, Vec<V>>` | You need to enumerate or modify members |
| Link-cut / Euler-tour trees | **Online** dynamic connectivity with deletions |

## Rules of Thumb

- Always both optimizations. Either alone is ~14,000× better than neither.
- `find` **iteratively** — recursive can overflow on a pre-compression chain.
- `union` returns `bool`; mark it `#[must_use]`.
- Roots are **not stable** — never cache a root across a union.
- Maintain the set count inside `union`, not at call sites.
- Intern real keys to `0..n` IDs first; grids use `row*w + col`.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| No optimizations | Correct but 14,500× slow; quadratic |
| Recursive `find` | `fatal runtime error: stack overflow` |
| Ignored `union` return | Kruskal builds cycles; set count drifts |
| Cached a root | Stale group identity after a later merge |
| Expected `split`/enumerate | Bolted-on member map goes stale on every union |
| Compression + rollback | Undo silently impossible — history destroyed |

## Key References

- Tarjan (1975) — the Θ(mα(n)) bound, proved tight
- Tarjan & van Leeuwen (1984) — every compression/union combination
- [CP-Algorithms: DSU](https://cp-algorithms.com/data_structures/disjoint_set_union.html) — rollback, weighted, offline connectivity
