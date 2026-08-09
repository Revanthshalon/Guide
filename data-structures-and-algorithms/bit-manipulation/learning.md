# Bit Manipulation — Learning Notes

## Mental Model

**A machine word is 64 parallel booleans, and bitwise operations process all 64 in one instruction.** That's the entire value proposition: not clever tricks, but a 64× data-parallelism you get for free from the ALU.

Measured — intersecting two sets of 1,000,000 elements, 1,000 repetitions:

| Representation | Time | Memory |
| --- | --- | --- |
| `Vec<bool>` (one byte per element) | 245.73 ms | ~1 MB |
| **Bitset (`Vec<u64>`) + `count_ones`** | **3.11 ms** | **122 KB** |
| | **79.1×** | **8×** |

Two separate wins compound: 8× less memory (so more fits in cache) and 64 elements per operation. Neither requires any cleverness — `a & b` and `.count_ones()` are the whole implementation.

The second use is **compact state**. A subset of ≤ 64 elements is a single `u64`, which makes set operations Θ(1) and makes subsets usable as array indices or hash keys. That's what powers bitmask [dynamic programming](../dynamic-programming/learning.md) and the bitmask backtracking that ran 9-queens in 50 µs ([recursion & backtracking](../recursion-and-backtracking/learning.md)).

The honest framing on "bit tricks": **most of them are obsolete.** Modern compilers and CPUs provide intrinsics — `count_ones`, `leading_zeros`, `trailing_zeros` compile to single instructions (`POPCNT`, `LZCNT`, `TZCNT`). Hand-rolled bit-twiddling to compute a population count is slower and less readable than calling `count_ones()`. **Learn the idioms that name an operation the hardware has**; skip the ones that emulate an operation the hardware already does.

## The Invariant

There's no data-structure invariant here, but there are three facts that everything rests on:

> **Bit `i` of `x` is set iff `x & (1 << i) != 0`.** Bit positions run 0..63 for `u64`; **shifting by ≥ the width is undefined-ish** — in Rust it panics in debug and is masked in release, which is a real portability trap.
>
> **Two's complement:** `-x == !x + 1`. Therefore `x & -x` isolates the **lowest set bit**, and `x & (x-1)` **clears** it. These two are the most useful identities in the topic.
>
> **Shifts are not division.** `x >> 1` equals `x / 2` for unsigned and for non-negative signed values, but for negative signed values Rust's `>>` is an *arithmetic* shift (sign-extending), so `-3 >> 1 == -2` while `-3 / 2 == -1`. They round in different directions.

The `x & -x` identity is worth deriving once: `-x` is `!x + 1`, so all bits below the lowest set bit are 1 in `!x`, and adding 1 carries through them, leaving the lowest set bit matching and everything else complemented. AND-ing therefore keeps exactly that bit.

## Mechanics

### The idioms worth memorizing

| Operation | Expression | Rust method |
| --- | --- | --- |
| Test bit i | `x & (1 << i) != 0` | — |
| Set bit i | `x \| (1 << i)` | — |
| Clear bit i | `x & !(1 << i)` | — |
| Toggle bit i | `x ^ (1 << i)` | — |
| **Isolate lowest set bit** | `x & x.wrapping_neg()` | — |
| **Clear lowest set bit** | `x & (x - 1)` | — |
| Population count | — | **`x.count_ones()`** |
| Index of highest set bit | — | `63 - x.leading_zeros()` |
| Index of lowest set bit | — | **`x.trailing_zeros()`** |
| Is a power of two | `x != 0 && x & (x-1) == 0` | `x.is_power_of_two()` |
| Round up to a power of two | — | `x.next_power_of_two()` |
| `x mod 2^k` | `x & ((1 << k) - 1)` | — |
| Swap without a temp | `a ^= b; b ^= a; a ^= b` | **don't — use `swap`** |

**Use the std methods.** `count_ones`, `leading_zeros`, `trailing_zeros`, `is_power_of_two`, `next_power_of_two`, `rotate_left`, `swap_bytes`, `reverse_bits` all compile to single instructions where the hardware has them. Hand-rolled equivalents are slower and obscure the intent.

### Iterating set bits, and iterating subsets

```rust
// Iterate the set bits of a mask — one iteration per SET bit, not per bit position.
let mut m = mask;
while m != 0 {
    let i = m.trailing_zeros();
    // ... use bit i ...
    m &= m - 1;                       // clear the lowest set bit
}

// Enumerate ALL submasks of `mask`, in decreasing order. Θ(3ⁿ) over all masks — not 4ⁿ.
let mut sub = mask;
loop {
    // ... use sub ...
    if sub == 0 { break; }
    sub = (sub - 1) & mask;
}
```

The submask enumeration is the non-obvious one and it's the engine of several bitmask DPs (set cover, partition into groups). Summed over all 2ⁿ masks, the total number of (mask, submask) pairs is **3ⁿ**, not 4ⁿ — because each element is in the submask, in the mask but not the submask, or in neither.

### Bitsets

```rust
struct Bitset { words: Vec<u64> }

impl Bitset {
    #[inline] fn get(&self, i: usize) -> bool { self.words[i >> 6] >> (i & 63) & 1 == 1 }
    #[inline] fn set(&mut self, i: usize)     { self.words[i >> 6] |= 1 << (i & 63); }
    fn intersect_count(&self, o: &Bitset) -> u32 {
        self.words.iter().zip(&o.words).map(|(a, b)| (a & b).count_ones()).sum()
    }
}
```

`i >> 6` and `i & 63` are `i / 64` and `i % 64` — the compiler generates these anyway for a power-of-two divisor, so write whichever is clearer. **The `intersect_count` loop is what measured 79.1× faster than `Vec<bool>`**, and it also autovectorizes: LLVM turns it into SIMD ANDs plus vector popcounts.

Use `fixedbitset` or `bit-set` in production rather than hand-rolling — they handle growth, iteration, and the edge cases.

### Bitmask DP and state

A subset of n ≤ 20-ish elements as a `u32` index:

```rust
// TSP: dp[mask][i] = min cost to visit exactly `mask`, ending at i.
for mask in 1..(1u32 << n) {
    for i in 0..n {
        if mask & (1 << i) == 0 { continue; }
        let prev = mask & !(1 << i);
        if prev == 0 { dp[mask][i] = dist[start][i]; continue; }
        let mut m = prev;
        while m != 0 {
            let j = m.trailing_zeros() as usize; m &= m - 1;
            dp[mask][i] = dp[mask][i].min(dp[prev][j] + dist[j][i]);
        }
    }
}
```

Θ(2ⁿ·n²) — the 2ⁿ ceiling means n ≤ ~20, which arrives fast. See [dynamic programming](../dynamic-programming/learning.md).

### XOR properties

`x ^ x == 0` and `x ^ 0 == x`, and XOR is commutative and associative. Consequences:

- **Find the single non-duplicated element:** XOR everything; pairs cancel.
- **Find a missing number in `0..n`:** XOR the range with the array.
- **Prefix XOR** gives range-XOR in Θ(1): `xor(l..r) = p[r] ^ p[l]` ([prefix sums](../prefix-sums-and-difference-arrays/learning.md)).
- **Do not XOR-combine sub-hashes** — commutativity means `(1,2)` and `(2,1)` collide ([hashing techniques](../hashing-techniques/learning.md)).

## Complexity

| Operation | Cost | Note |
| --- | --- | --- |
| Any bitwise op on a word | Θ(1) | One instruction, 64 bits |
| `count_ones` / `leading_zeros` / `trailing_zeros` | Θ(1) | Single instruction (POPCNT/LZCNT/TZCNT) |
| Bitset op over n bits | **Θ(n/64)** | 64× fewer operations than per-element |
| Iterate set bits | Θ(popcount) | Not Θ(width) |
| Enumerate submasks of one mask | Θ(2^popcount) | — |
| Enumerate all (mask, submask) pairs | **Θ(3ⁿ)** | not 4ⁿ |
| Bitmask DP over subsets | Θ(2ⁿ · f) | n ≤ ~20 |

**Where the table misleads.** Θ(n/64) is the same complexity class as Θ(n) — asymptotically bitsets change nothing. The measured 79.1× is entirely constant factor, and it's one of the largest constant-factor wins available anywhere in this category. That's a reminder that Θ() is deliberately blind to exactly the thing that decided this comparison.

The 2ⁿ row is the hard ceiling: at n = 20 that's 10⁶ masks (fine), at n = 30 it's 10⁹ (not fine), at n = 40 it's 10¹² (impossible).

## Use Cases

- **Set operations at scale** — inverted indexes in search engines intersect posting lists as bitmaps; column stores use bitmap indexes for filtering. Roaring bitmaps are the production form (compressed, adaptive).
- **Feature flags and permissions** — a `u64` holds 64 independent booleans in one atomic-sized word, which also makes it cheap to store and compare.
- **Visited sets in traversal** — a bitset visited-marker uses 8× less memory than `Vec<bool>`, which can decide whether a large [BFS](../graph-traversal/learning.md)'s working set fits in cache.
- **Bitmask DP** — TSP, assignment, set cover, partition-into-groups.
- **Backtracking state** — the measured 9-queens in 50 µs used three `u32` masks for columns and diagonals.
- **Hash table control bytes** — SwissTable's SIMD probe is a bitmask operation over 16 tags ([hash tables](../hash-tables/learning.md)).
- **Bloom filters** — k hash functions setting k bits (Stage 8).
- **Compression and encoding** — varint, bit-packing, Elias-gamma; anything that packs values below byte granularity.
- **Chess and game engines** — bitboards represent a board as a `u64` per piece type, making move generation a few shifts and masks.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Bitset** (`fixedbitset`) | Dense boolean sets, set algebra, visited marks |
| Roaring bitmap | **Sparse** sets over a huge universe — bitsets waste space |
| `u64` as a subset | n ≤ 64, need subsets as indices or keys |
| Bitmask DP | Subsets matter and n ≤ ~20 |
| std intrinsics | Always — `count_ones` over hand-rolled |
| `HashSet` | Sparse, unbounded, or non-integer elements |
| **Not bit tricks** | Anything the compiler already does — `x * 2` over `x << 1` |

## Pitfalls in Depth

### Pitfall: Shift overflow

- **What goes wrong:** `1u32 << 32` or `1u64 << 64`. In debug builds this **panics** ("attempt to shift left with overflow"); in release it's masked to `1 << 0 == 1`, silently producing wrong results. The classic instance is `(1 << n) - 1` to build an n-bit mask when `n` equals the type width — a boundary that appears exactly when the mask should be "all bits".
- **Why it happens (the mechanism):** Hardware shift instructions mask the shift amount by the register width, so `<< 32` on a 32-bit value is `<< 0`. Rust surfaces this as a debug assertion but preserves the hardware behaviour in release, so debug and release disagree — the worst kind of bug.
- **How to handle it in production, and why that works:** Use a wider type for the intermediate (`1u64 << n` then cast) or the checked forms: `1u32.checked_shl(n)`, or `u32::MAX >> (32 - n)` guarded for `n = 0`. For the "all bits" case just use `u32::MAX` or `!0`. Rust also offers `wrapping_shl`/`overflowing_shl` when you genuinely want the masking behaviour, which documents the intent.
- **Trade-offs of the fix:** A wider intermediate costs nothing on 64-bit hardware. `checked_shl` returns an `Option` you must handle, which is friction at every call site — worth it only where `n` is genuinely variable and unbounded.

### Pitfall: Arithmetic vs logical shift on signed values

- **What goes wrong:** `x >> 1` is used as "divide by two" on an `i32`, and for negative values it rounds toward negative infinity while `/` rounds toward zero: `-3 >> 1 == -2` but `-3 / 2 == -1`. Off-by-one errors appear only for negative inputs, so tests with positive data pass.
- **Why it happens (the mechanism):** Rust's `>>` on signed integers is an *arithmetic* shift (it sign-extends), which is the right behaviour for preserving sign but is not the same operation as division. The equivalence `x >> 1 == x / 2` holds only for non-negative values.
- **How to handle it in production, and why that works:** Write `x / 2` and let the compiler choose the instruction — it emits a shift plus a correction for signed types, which is what you actually wanted. Reserve shifts for genuine bit manipulation, where the operation *is* "move bits", not "scale by a power of two".
- **Trade-offs of the fix:** The compiler's signed division-by-power-of-two is two or three instructions rather than one. That difference is irrelevant outside a measured hot loop, and inside one you can use unsigned types where the shift is exact.

### Pitfall: Bitsets on sparse data

- **What goes wrong:** A bitset is used for a set of 1,000 elements drawn from a universe of 10⁹. That's 125 MB of mostly-zero words, and every set operation scans all of it — Θ(universe), not Θ(elements). The 79.1× win inverts into a large loss against a `HashSet`.
- **Why it happens (the mechanism):** A bitset's cost is proportional to the **universe size**, not the set size. That's exactly what makes it fast for dense sets and pathological for sparse ones, and the distinction is about the *ratio*, not the absolute count.
- **How to handle it in production, and why that works:** Estimate density (elements / universe) before choosing. Dense (say > 1%) → bitset. Sparse over a huge universe → `HashSet`, a sorted `Vec`, or a **Roaring bitmap**, which partitions the universe into chunks and stores each chunk as a bitset *or* a sorted array depending on that chunk's density — adaptively getting both behaviours.
- **Trade-offs of the fix:** Roaring adds a dependency and some per-operation branching to dispatch on chunk type. A `HashSet` gives up the SIMD-friendly set algebra entirely. There's no representation that's optimal at both ends, which is why Roaring's adaptivity exists.

### Pitfall: Hand-rolled bit tricks that the compiler already does better

- **What goes wrong:** A hand-written population count (the classic SWAR sequence of masks and multiplies), a manual "round up to power of two", or `x << 1` instead of `x * 2` "for speed". The result is slower than `count_ones()` (which is one `POPCNT` instruction), harder to read, and occasionally wrong at boundaries.
- **Why it happens (the mechanism):** These tricks date from when hardware lacked the instructions and compilers didn't recognize the patterns. Both facts changed: `POPCNT` has been standard since 2008, and LLVM optimizes `x * 2` to a shift automatically. The folklore outlived its justification.
- **How to handle it in production, and why that works:** Use the std methods — `count_ones`, `leading_zeros`, `trailing_zeros`, `is_power_of_two`, `next_power_of_two`, `rotate_left`, `reverse_bits`. They compile to the single instruction where it exists and to a good fallback where it doesn't, and they say what you mean. Write arithmetic as arithmetic and let the optimizer choose.
- **Trade-offs of the fix:** On targets without `POPCNT`, `count_ones` compiles to a software fallback — which is exactly the SWAR sequence you'd have written, so you lose nothing. The only genuine reason to hand-roll is a specific instruction-set feature the std method doesn't expose, and that's rare enough to require measurement first.

### Pitfall: Confusing bit position with bit value

- **What goes wrong:** `mask |= i` where `mask |= 1 << i` was meant, or `if mask & i != 0` instead of `if mask & (1 << i) != 0`. The code compiles, and for `i` in {0, 1} it even behaves plausibly, so small tests pass.
- **Why it happens (the mechanism):** Both the index and the mask are integers of the same type, so the type system offers no protection. The distinction is entirely conventional — `i` is a *position*, `1 << i` is a *value*.
- **How to handle it in production, and why that works:** Name them differently (`bit_index` vs `bit_mask`) or wrap masks in a newtype so the two can't be mixed. Small helper functions (`fn has(mask: u64, i: u32) -> bool { mask >> i & 1 == 1 }`) put the shift in exactly one place. In practice, using `fixedbitset` removes the whole class by never exposing raw masks.
- **Trade-offs of the fix:** A newtype adds `.0` noise; helpers add a call the optimizer will inline anyway. Both cost less than debugging a mask that's off by a factor of two.
