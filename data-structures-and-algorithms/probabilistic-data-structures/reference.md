# Probabilistic Data Structures — Quick Reference

## At a Glance

Give up exactness and space collapses from **proportional to the data** to **proportional to the accuracy you asked for**. These store *a function of* the data, not the data — so they can't enumerate, and space stops depending on n.

**Three questions, in order:** (1) which **direction** does the error go? (2) is it **mergeable**? (3) does the bound hold in practice? (Measured: yes, to 0.01 pp.)

## The Numbers (measured)

**Bloom filter, 1,000,000 items:**

| Bits/item | k | Memory | **Measured FP** | Theory |
| --- | --- | --- | --- | --- |
| 8 | 6 | 976 KB | **2.150%** | 2.158% |
| 10 | 7 | 1,220 KB | **0.824%** | 0.819% |
| 16 | 11 | 1,953 KB | **0.045%** | 0.046% |
| exact `HashSet` | — | **~8,928 KB** | 0% | — |

**Rule: every ~4.8 bits/item divides the FP rate by 10.** Size is independent of *item* size.

**HyperLogLog:**

| Precision | Memory | True | Estimate | Error |
| --- | --- | --- | --- | --- |
| p=10 | 1 KB | 10,000,000 | 10,120,580 | 1.21% |
| **p=14** | **16 KB** | **10,000,000** | 10,040,707 | **0.41%** |

**Same 16 KB for 1,000 or 10 billion.** (One p=10 trial hit 4.41% vs ±3.25% theory — 1.04/√m is a *standard error*, not a hard bound.)

## The Family

| Structure | Answers | Error direction | Space | Merge |
| --- | --- | --- | --- | --- |
| **Bloom** | membership | **FP only, never FN** | ~10 bits/item @1% | **OR** |
| Counting Bloom | + delete | FP only | 4× Bloom | yes |
| **Cuckoo filter** | + delete | FP only | ≈Bloom, better <1% | limited |
| **HyperLogLog** | distinct count | ±1.04/√m | **fixed** | **max** |
| **Count-Min** | frequency | **over-estimates only** | w·d | **add** |
| Count sketch | frequency | two-sided, unbiased | w·d | add |
| **t-digest/DDSketch** | quantiles | bounded relative | Θ(1/ε) | **yes** |
| MinHash / SimHash | similarity | ±1/√k | k hashes / 64 b | yes |

## Invariants

- **Bloom:** all k bits set ⇒ *maybe*; any bit clear ⇒ **definitely absent**. Insertion only *sets* bits — which is why **you cannot delete**.
- **HLL:** each register holds a *max* ⇒ idempotent ⇒ counts distinct, and merges by element-wise max.
- **Count-Min:** collisions only add ⇒ the estimate is an **upper bound**.

## Snippets

```rust
// Bloom: k hashes via double hashing (h1 + i*h2) — Kirsch-Mitzenmacher
fn insert(&mut self, x: u64) {
    let (h1, h2) = (hash1(x), hash2(x));
    for i in 0..self.k { let p = (h1.wrapping_add(i as u64 * h2) % self.m) as usize;
                         self.bits[p >> 6] |= 1u64 << (p & 63); }
}
// Merge (identical size + hashes required)
for (a, b) in self.bits.iter_mut().zip(&other.bits) { *a |= b; }
// HLL merge
for (a, b) in self.reg.iter_mut().zip(&other.reg) { *a = (*a).max(*b); }
```

## Choose This When

| Use | For |
| --- | --- |
| **Bloom** | Membership pre-filter, FP tolerable, no deletes, **n known** |
| **Cuckoo filter** | Same + **deletion**, or very low FP |
| **HyperLogLog** | Distinct count, huge/unbounded stream |
| **Count-Min** | Frequency, heavy hitters (over-estimation OK) |
| **t-digest / DDSketch** | Quantiles **merged across hosts** |
| MinHash / SimHash | Near-duplicate detection |
| **Exact** | Answer must be exact, or it fits in budget |

## Rules of Thumb

- Write down **which direction is safe** at every call site.
  Bloom: *negative is certain*. Count-Min: *estimate is an upper bound*.
- **Merge the sketches, then query.** Never average estimates.
- Bloom space is Θ(n·bits) — it is **not** constant; it saturates silently.
- HLL genuinely is constant space regardless of n.
- Compute the exact structure's size first — 7.3× saving only matters if the exact one doesn't fit.
- The Bloom win grows with **item size** (1M URLs is still ~1.2 MB).
- Every LSM engine keeps a Bloom per SSTable — that's the canonical production use.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Undersized / unbounded Bloom | FP → 100%, **no signal**; filter stops filtering |
| Deleting from a plain Bloom | **False negatives** — breaks the contract downstream |
| Count-Min for "is this rare?" | Unbounded error in the unsafe direction |
| Summing per-host HLL estimates | Double-counts items seen on several hosts |
| Averaging per-host p99 | Not a percentile of anything |
| Sketch where exact fits | Complexity + error budget for memory nobody needed |
| Mismatched params on merge | Silent corruption — sizes and hashes must match |

## Key References

- Bloom (1970) · Kirsch & Mitzenmacher (2006) — double hashing
- Flajolet et al. (2007) — HyperLogLog · Heule et al. (2013) — HLL++
- Cormode & Muthukrishnan (2005) — Count-Min
- Fan et al. (2014) — cuckoo filter · Dunning & Ertl — t-digest
