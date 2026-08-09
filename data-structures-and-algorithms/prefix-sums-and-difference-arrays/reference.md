# Prefix Sums & Difference Arrays — Quick Reference

## At a Glance

Precompute once, query Θ(1). **Try this first for any range-query problem.**

```
prefix[i] = a[0] + … + a[i-1]        prefix[0] = 0,  length n+1
sum(l..r) = prefix[r] - prefix[l]     half-open, no special case
```

**The dual — difference array:**
```
range_update(l, r, +v):  diff[l] += v;  diff[r] -= v      // Θ(1)
a = prefix_sum(diff)                                       // Θ(n) once
```

Prefix sums: Θ(1) **queries**, Θ(n) updates. Difference arrays: Θ(1) **updates**, Θ(n) queries. They are discrete integration and differentiation.

**Invariant:** `prefix[i]` aggregates `a[0..i]`; **n+1 entries with `prefix[0] = 0`**; the operation must be **invertible**.

## Complexity

| Structure | Build | Point upd | Range query | Range upd |
| --- | --- | --- | --- | --- |
| Plain array | Θ(n) | **Θ(1)** | Θ(n) | Θ(n) |
| **Prefix sums** | Θ(n) | Θ(n) rebuild | **Θ(1)** | Θ(n) |
| **Difference array** | Θ(n) | Θ(1) | Θ(n) | **Θ(1)** |
| 2-D prefix | Θ(nm) | Θ(nm) | **Θ(1)** | — |
| Fenwick | Θ(n) | Θ(log n) | Θ(log n) | Θ(log n)* |

Static data: prefix sums beat a Fenwick tree **by a large constant too** (2 reads vs ~20 scattered). With updates the ranking inverts — measured, Fenwick **3.9 ms** vs naive rescan **~5,623 ms**.

## Which Operations Work

| Op | Range query | Works? |
| --- | --- | --- |
| Sum / count | `p[r] - p[l]` | ✅ |
| XOR | `p[r] ^ p[l]` | ✅ self-inverse |
| Product | `p[r] / p[l]` | ⚠️ zeros |
| **Min / max / gcd** | — | ❌ **not invertible** → sparse table |

## Snippets

```rust
// Build — n+1 entries, prefix[0] = 0
let mut p = vec![0i64; a.len() + 1];
for i in 0..a.len() { p[i+1] = p[i] + a[i] as i64; }

// 2-D build and query — inclusion–exclusion
p[i][j] = a[i-1][j-1] + p[i-1][j] + p[i][j-1] - p[i-1][j-1];
let s   = p[r2][c2] - p[r1][c2] - p[r2][c1] + p[r1][c1];

// Difference array: u range updates in Θ(u), materialize in Θ(n)
for &(l, r, v) in &updates { diff[l] += v; diff[r] -= v; }
let mut run = 0; for i in 0..n { run += diff[i]; a[i] = run; }

// Prefix + HashMap: subarrays summing to k — WORKS WITH NEGATIVES
let mut seen = HashMap::from([(0i64, 1usize)]);
for &x in a {
    run += x;
    count += seen.get(&(run - k)).copied().unwrap_or(0);
    *seen.entry(run).or_insert(0) += 1;
}
```

## Choose This When

| Use | For |
| --- | --- |
| **Prefix sums** | Static + many range queries |
| **Difference array** | Many range updates, read once |
| Both | Range update + point query |
| **Prefix + `HashMap`** | Subarray sums with **negatives**, divisibility, balance |
| Sparse table | Static **min/max/gcd** |
| Fenwick | Interleaved point updates + range sums |
| Segment tree | Range updates + range queries, or non-invertible |

## Rules of Thumb

- **n+1 entries, `prefix[0] = 0`, half-open ranges.** Matches `&a[l..r]`.
- **Widen the type** — `i64` prefix over `i32` data. The prefix grows with n, not with the element range.
- Min/max are **not** invertible — sparse table, not prefix sums.
- Data changes between queries? → Fenwick/segment tree, not rebuilds.
- Modular: `(p[r] - p[l] + MOD) % MOD`.
- 2-D: the `- p[i-1][j-1]` and `+ p[r1][c1]` are the same inclusion–exclusion correction.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Prefix in the element type | Silent wrap in release; plausible negative sums |
| Prefix min/max | Wrong whenever the min is in the excluded prefix |
| n-length inclusive prefix | Underflow/panic at `l = 0`; special case forgotten |
| Rebuild per update | Θ(n·u) — ~1,400× vs a Fenwick tree |
| 2-D missing inclusion–exclusion | Corner counted twice |
| Difference array with inclusive `r` | Off-by-one on every range |

## Key References

- Viola & Jones (2001) — integral images: 2-D prefix sums in real-time vision
- CP-Algorithms — prefix sums, difference arrays, 2-D variants
