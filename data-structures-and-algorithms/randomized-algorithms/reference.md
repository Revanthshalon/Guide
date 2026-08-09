# Randomized Algorithms — Quick Reference

## At a Glance

**Randomness converts a promise about your *input* into a promise about your *coin flips* — and you control the coins.**

| Family | Guarantee | Examples |
| --- | --- | --- |
| **Las Vegas** | always correct, **random runtime** | randomized quicksort, treap, skip list |
| **Monte Carlo** | always fast, **may be wrong** (bounded) | Miller-Rabin, Karger, Bloom filter |

**Average ≠ expected.** Average-case quantifies over a *distribution of inputs* you assumed; expected quantifies over the algorithm's *own coins* and holds on **every** input.

## The Number

2,000,000 shuffles of 4 elements (24 permutations, expected 83,333 each):

| Algorithm | Deviation |
| --- | --- |
| **Fisher-Yates** (`j ∈ 0..=i`) | **−0.72% … +0.60%** |
| Naive (`j ∈ 0..n`) | **−24.83% … +40.37%** |

`nⁿ = 256` execution paths onto `4! = 24` permutations — **256 isn't divisible by 24**, so uniformity is arithmetically impossible. Better entropy cannot fix it.

## Snippets

```rust
// Fisher-Yates — the range is the whole difference
for i in (1..n).rev() { let j = rng.gen_range(0..=i); v.swap(i, j); }
// Better: rand::seq::SliceRandom::shuffle

// Reservoir sampling: k from an unknown-length stream, Θ(k) space
for (i, item) in stream.enumerate() {
    if i < k { reservoir.push(item); }
    else { let j = rng.gen_range(0..=i); if j < k { reservoir[j] = item; } }
}
```

## Complexity

| Algorithm | Type | Bound | Holds on |
| --- | --- | --- | --- |
| Randomized quicksort | LV | Θ(n log n) expected | **every** input |
| Randomized quickselect | LV | **Θ(n) expected** | every input |
| Reservoir sampling | — | Θ(n) time, **Θ(k) space** | one pass |
| Treap / skip list | LV | Θ(log n) expected height | every insertion order |
| Miller-Rabin | MC | Θ(k log³ n), error ≤ **4⁻ᵏ** | — |
| Karger min-cut | MC | Θ(n² log n) whp | — |

Monte Carlo errors are **independent per run** ⇒ k rounds give εᵏ. 40 Miller-Rabin rounds ≈ 10⁻²⁴.

## RNG Selection

| Need | Use |
| --- | --- |
| Simulation, shuffling, sampling | `rand::thread_rng()` |
| Reproducible tests/benchmarks | `StdRng::seed_from_u64` — **log the seed** |
| Speed, non-adversarial | `SmallRng`, PCG, xorshift |
| **Keys, tokens, nonces** | **`OsRng` / `getrandom`** — never a general PRNG |
| Range | `gen_range(0..n)` — **never `% n`** (modulo bias) |

## Rules of Thumb

- Adversarial input possible? Put the randomness in the **algorithm**, seeded unpredictably.
- Never hand-roll a shuffle.
- Monte Carlo: know which direction the error goes. **One-sided** error is what makes pre-filters safe.
- Amplify by repeating with **fresh** randomness.
- A fast PRNG's state is recoverable from a few outputs — statistical quality ≠ unpredictability.
- Randomness replaces balance logic: a treap is ~20 lines vs red-black's case analysis.
- Seed tests explicitly; `proptest` persists failing seeds automatically.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| `j ∈ 0..n` shuffle | Up to 40% skew; looks shuffled |
| Deterministic pivot / fixed hash seed | Attacker forces Θ(n²) — 186× measured for HashDoS |
| `SmallRng` for tokens | State recovered from a few outputs; all tokens predictable |
| `rng() % n` | Modulo bias toward low values |
| Unseeded randomized test | 1-in-500 failure that can't be reproduced |
| Timestamp seed | ~2³⁰ possible seeds — brute-forceable |

## Key References

- Motwani & Raghavan, *Randomized Algorithms* — the standard text
- Knuth TAOCP Vol. 2 §3.4.2 — Fisher-Yates and the bias analysis
- Karger (1993) — min-cut by random contraction
- Seidel & Aragon (1996) — treaps · Pugh (1990) — skip lists
