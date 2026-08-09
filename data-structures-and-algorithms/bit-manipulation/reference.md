# Bit Manipulation — Quick Reference

## At a Glance

**A machine word is 64 parallel booleans.** Not clever tricks — free 64× data parallelism from the ALU.

**Facts everything rests on:** bit i set iff `x & (1<<i) != 0` · two's complement `-x == !x + 1`, so **`x & -x`** isolates and **`x & (x-1)`** clears the lowest set bit · `>>` on signed is *arithmetic*, **not** division.

## The Number

Intersect two 1,000,000-element sets, 1,000 reps (measured):

| Representation | Time | Memory |
| --- | --- | --- |
| `Vec<bool>` | 245.73 ms | ~1 MB |
| **Bitset + `count_ones`** | **3.11 ms** | **122 KB** |
| | **79.1×** | **8×** |

Θ(n/64) is still Θ(n) — this is **entirely constant factor**, and one of the largest available.

## Idioms

| Operation | Expression / method |
| --- | --- |
| Test / set / clear / toggle bit i | `x & (1<<i)` · `x \| (1<<i)` · `x & !(1<<i)` · `x ^ (1<<i)` |
| **Isolate lowest set bit** | `x & x.wrapping_neg()` |
| **Clear lowest set bit** | `x & (x - 1)` |
| Population count | **`x.count_ones()`** |
| Lowest / highest set bit index | `x.trailing_zeros()` · `63 - x.leading_zeros()` |
| Power of two? / round up | `x.is_power_of_two()` · `x.next_power_of_two()` |
| `x mod 2^k` | `x & ((1 << k) - 1)` |

**Use std methods** — they compile to `POPCNT`/`LZCNT`/`TZCNT`. Hand-rolled SWAR is slower and less clear.

## Iteration

```rust
// Set bits: one iteration per SET bit
let mut m = mask;
while m != 0 { let i = m.trailing_zeros(); /* use i */ m &= m - 1; }

// All submasks of `mask`, decreasing. Over all masks: Θ(3ⁿ), not 4ⁿ.
let mut sub = mask;
loop { /* use sub */ if sub == 0 { break; } sub = (sub - 1) & mask; }

// Bitset
fn get(&self, i: usize) -> bool { self.words[i >> 6] >> (i & 63) & 1 == 1 }
fn intersect_count(&self, o: &Self) -> u32 {
    self.words.iter().zip(&o.words).map(|(a,b)| (a & b).count_ones()).sum()   // autovectorizes
}
```

## Complexity

| Operation | Cost |
| --- | --- |
| Any bitwise op | Θ(1), 64 bits |
| `count_ones` etc. | Θ(1) single instruction |
| Bitset op over n bits | **Θ(n/64)** |
| Iterate set bits | Θ(popcount), not Θ(width) |
| All (mask, submask) pairs | **Θ(3ⁿ)** |
| Bitmask DP | Θ(2ⁿ·f) — **n ≤ ~20** |

## Choose This When

| Use | For |
| --- | --- |
| **Bitset** (`fixedbitset`) | Dense boolean sets, set algebra, visited marks |
| **Roaring bitmap** | **Sparse** over a huge universe |
| `u64` as a subset | n ≤ 64; subsets as indices/keys |
| Bitmask DP | n ≤ ~20 |
| `HashSet` | Sparse, unbounded, or non-integer |
| **Plain arithmetic** | Anything the compiler already optimizes |

## Rules of Thumb

- Bitset cost is proportional to the **universe**, not the set — dense only.
- `x / 2`, not `x >> 1`, on signed values.
- `1u64 << n` (wider intermediate) or `checked_shl` when n can reach the width.
- Name positions and masks differently, or use a newtype.
- Never XOR-combine sub-hashes — commutativity collides `(1,2)` and `(2,1)`.
- `i >> 6` / `i & 63` = `/64` / `%64`; write whichever is clearer.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| `1 << 32` on a `u32` | Panics in debug, silently `== 1` in release |
| `>>` as division on signed | Off by one for negatives only |
| Bitset on sparse data | 125 MB of zeros; slower than `HashSet` |
| Hand-rolled popcount | Slower than `count_ones()` |
| `mask \| i` instead of `mask \| (1<<i)` | Plausible for i ∈ {0,1}; wrong after |
| Bitmask DP at n = 30 | 10⁹ states — infeasible |

## Key References

- Warren, *Hacker's Delight* — the canonical catalogue (know which entries the hardware now does)
- [`fixedbitset`](https://docs.rs/fixedbitset/) · Roaring bitmaps — the production forms
- Rust primitive int docs — `count_ones`, `leading_zeros`, `rotate_left`, `reverse_bits`
