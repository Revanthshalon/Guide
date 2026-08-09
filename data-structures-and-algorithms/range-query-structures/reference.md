# Range Query Structures — Quick Reference

## At a Glance

All answer "aggregate over `[l, r]`" and differ only in the update/query mix they'll pay for. Precompute over a **hierarchy of blocks**, so a range decomposes into Θ(log n) pieces and a point update touches Θ(log n) blocks.

**Fenwick invariant:** `tree[i]` covers the `i & (-i)` elements ending at `i` — each index owns a range the length of its **lowest set bit**.
**Segment tree invariant:** each node stores the aggregate of a contiguous range; children split it.
**Sparse table invariant:** `st[k][i]` covers `2^k` elements from `i`; queries use two **overlapping** blocks — safe only if idempotent.

## The Number

100k point updates interleaved with 100k prefix queries, n = 10⁶:

| Approach | Time |
| --- | --- |
| **Fenwick** | **3.9 ms** |
| Naive array rescan | ~5,623 ms (**~1,400×**) |

Ratio grows linearly with n.

## Complexity

| Structure | Build | Point upd | Range query | Range upd | Space |
| --- | --- | --- | --- | --- | --- |
| Plain array | Θ(n) | **Θ(1)** | Θ(n) | Θ(n) | n |
| **Prefix sums** | Θ(n) | Θ(n) | **Θ(1)** | Θ(n) | n |
| **Fenwick** | Θ(n) | Θ(log n) | Θ(log n) | Θ(log n)* | **n** |
| Segment tree | Θ(n) | Θ(log n) | Θ(log n) | Θ(n) | 2n |
| Seg tree + lazy | Θ(n) | Θ(log n) | Θ(log n) | **Θ(log n)** | 2n+ |
| Sparse table | Θ(n log n) | rebuild | **Θ(1)** † | — | n log n |
| Sqrt decomp | Θ(n) | Θ(1) | Θ(√n) | Θ(√n) | n+√n |

`*` via difference array, point queries only · `†` idempotent ops only

## Decision Order

1. **Does the data change?** No → **prefix sums** (invertible) or **sparse table** (min/max/gcd). Stop.
2. Point updates + **invertible** op (sum, xor) → **Fenwick**.
3. Non-invertible op (min, max, gcd, matrix) → **segment tree**.
4. **Range** updates + range queries → **segment tree + lazy**.
5. Range update, point query only → **Fenwick over a difference array**.
6. No clean structure → **sqrt decomposition**.

## Snippets

```rust
// Fenwick — the whole structure
fn add(&mut self, mut i: usize, d: i64) { i += 1;
    while i < self.0.len() { self.0[i] += d; i += i & i.wrapping_neg(); } }
fn prefix(&self, mut i: usize) -> i64 { let mut s = 0;
    while i > 0 { s += self.0[i]; i -= i & i.wrapping_neg(); } s }
fn range(&self, l: usize, r: usize) -> i64 { self.prefix(r) - self.prefix(l) }  // NEEDS INVERSE

// Range update + point query, no lazy needed
fen.add(l, v); fen.add(r, -v);
let at_i = fen.prefix(i + 1);

// Segment tree query: SEPARATE accumulators (non-commutative ops)
if l & 1 == 1 { res_l = op(res_l, t[l]); l += 1; }
if r & 1 == 1 { r -= 1; res_r = op(t[r], res_r); }
```

## Rules of Thumb

- Static data → prefix sums. Three lines, Θ(1), *faster* than a tree.
- Fenwick when both apply: half the memory, smaller constant, no recursion.
- **Min/max cannot use Fenwick** — no inverse.
- Keep 1-indexing **inside** the type; expose 0-indexed half-open ranges.
- Non-commutative ops need separate left/right accumulators.
- Funnel all lazy access through `push_down`/`pull_up` — never touch nodes directly.
- Sparse table only for static + idempotent.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Fenwick for range-min | Plausible-looking wrong answers; passes casual tests |
| `i -= i & (-i)` at i = 0 | **Infinite loop** |
| Mixed 0-/1-indexing | Correct in the middle, wrong at boundaries |
| One accumulator, non-commutative op | Silent garbage; invisible for sum/min |
| Missed `push_down` | Stale or doubly-applied updates; order-dependent |
| Segment tree on static data | Slower than a 3-line prefix array |

## Key References

- Fenwick (1994) — the original BIT paper
- Al.Cash, ["Efficient and easy segment trees"](https://codeforces.com/blog/entry/18051) — the iterative form
- [CP-Algorithms](https://cp-algorithms.com/data_structures/fenwick.html) — lazy propagation, 2-D variants
