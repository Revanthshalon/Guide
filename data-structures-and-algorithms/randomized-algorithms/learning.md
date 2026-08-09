# Randomized Algorithms — Learning Notes

## Mental Model

**Randomness converts a promise about your *input* into a promise about your *coin flips* — and you control the coins.**

That single sentence is the whole reason randomization is a tool rather than a gimmick. A deterministic quicksort is Θ(n log n) *on average over random inputs*, which is a hope: it says nothing about the input you actually receive, and an adversary who knows your pivot rule can hand you the Θ(n²) case. A randomized quicksort is Θ(n log n) **expected on every input**, because the randomness lives in the algorithm. The bad case still exists, but nobody — including an attacker — can steer you into it.

This is the **average vs expected** distinction from [complexity analysis](../complexity-analysis/learning.md), and it recurs everywhere in this repo: Rust's `HashMap` seeds SipHash randomly so [hash flooding](../hash-tables/learning.md) can't be precomputed (measured there: 186× degradation when collisions are forced); `sort_unstable` randomizes pivot selection; [treaps](../binary-search-trees/learning.md) get Θ(log n) expected height with no balancing code at all.

The second idea is **randomness replaces bookkeeping**. A treap maintains balance with a random priority per node instead of rotation-counting logic; a skip list replaces a balanced tree's rebalancing with coin flips. You trade a worst-case guarantee for expected behaviour, and get a large reduction in code complexity — which for a [BST](../binary-search-trees/learning.md), where deletion is the bug-prone part, is a genuinely good trade.

The third thing, and the one that bites hardest in practice: **getting randomness right is subtle**. The classic demonstration is shuffling. Measured over 2,000,000 shuffles of 4 elements (24 possible permutations, expected 83,333 each):

| Algorithm | Deviation from uniform |
| --- | --- |
| **Fisher-Yates** (`j` in `0..=i`) | **−0.72% … +0.60%** |
| Naive (`j` in `0..n`) | **−24.83% … +40.37%** |

The naive version — swap each position with a *uniformly random* position — looks obviously fair and isn't. The proof is a counting argument, not a subtle statistical one: it makes `n` independent choices from `n` options, so it has `nⁿ = 256` equally-likely execution paths mapping onto `4! = 24` permutations. **256 is not divisible by 24**, so a uniform distribution is arithmetically impossible. No amount of better entropy fixes it.

## The Invariant

The two families, and they are genuinely different contracts:

> **Las Vegas:** always correct, **runtime is random**. Randomized quicksort, treaps, skip lists. The guarantee is on the expected time.
>
> **Monte Carlo:** always fast, **the answer may be wrong** with bounded probability. Miller-Rabin primality, Karger's min-cut, [Bloom filters](../probabilistic-data-structures/learning.md). The guarantee is on the error rate.

The obligations:

- **For Las Vegas, the expectation must hold on *every* input.** That's what distinguishes it from average-case analysis, and it requires the randomness to be independent of the input — hence random pivots rather than "pick the middle element", and a per-process random seed rather than a fixed one.
- **For Monte Carlo, know which direction the error goes.** One-sided error is far more useful than two-sided: a Bloom filter can say "maybe present" when the item is absent, but **never** "absent" when it's present. That asymmetry is what makes it safe as a pre-filter, and it's a design property, not luck.
- **Error amplification:** repeating a Monte Carlo algorithm with fresh randomness drives the error down exponentially. Miller-Rabin with error ≤ 1/4 per round is ≤ 4⁻ᵏ after k rounds — 40 rounds puts you below the probability of a cosmic-ray bit flip.

## Mechanics

### Fisher-Yates, and why the obvious version fails

```rust
// CORRECT: j is drawn from 0..=i, so element i is placed among the remaining i+1 slots.
for i in (1..n).rev() {
    let j = rng.gen_range(0..=i);
    v.swap(i, j);
}
```

The invariant: after processing position `i`, the suffix `v[i..]` is a uniformly random selection, correctly ordered, of the elements. There are exactly `n!` execution paths (n choices, then n−1, …), matching the `n!` permutations one-to-one.

The naive version draws `j` from `0..n` every time. It has `nⁿ` paths, and since `nⁿ` is generally not divisible by `n!`, some permutations must be more likely than others — measured above at **up to 40% over-representation** for n = 4. This bias has shipped in real systems: a browser vendor's "random" ballot ordering was measurably skewed by exactly this bug.

**Use `rand::seq::SliceRandom::shuffle`.** It's correct, and it's one line.

### Reservoir sampling — k items from an unknown-length stream

```rust
// Keep k items uniformly at random from a stream of unknown length, in Θ(k) space.
for (i, item) in stream.enumerate() {
    if i < k { reservoir.push(item); }
    else {
        let j = rng.gen_range(0..=i);
        if j < k { reservoir[j] = item; }     // replace with probability k/(i+1)
    }
}
```

The proof is a neat induction: item `i` enters with probability `k/(i+1)`, and each already-present item survives each subsequent step with probability `1 − 1/(i+1)`; the product telescopes to exactly `k/n` for every item. **Θ(k) memory regardless of stream length**, and it needs only one pass — which is why it's the standard tool for sampling logs, sampling from a database cursor, or sampling telemetry at scale.

### Randomness replacing balance logic

| Structure | Random element | Replaces |
| --- | --- | --- |
| **Treap** | A random priority per node; maintain heap order by rotation | AVL/red-black balance factors and case analysis |
| **Skip list** | Coin flip per level | Tree rebalancing entirely; also far easier to make concurrent |
| Randomized BST | Random choice of which subtree gets the new root | Explicit balancing |
| Randomized quickselect | Random pivot | Median-of-medians' bad constant |

The treap is the highest-leverage instance: measured in [binary search trees](../binary-search-trees/learning.md), an unbalanced BST fed sorted input reached **depth 99,999 and took 29,689 ms** where a balanced structure took 9.2 ms. A treap fixes that with a random priority and the rotation code you'd already have — about 20 lines — instead of red-black's case analysis.

### Monte Carlo classics

**Miller-Rabin primality:** for a composite `n`, at least 3/4 of the possible witnesses prove compositeness. Test `k` random witnesses → error ≤ 4⁻ᵏ. This is how every cryptographic library generates primes; deterministic AKS primality exists and is far too slow.

**Karger's min-cut:** repeatedly contract a random edge until two vertices remain; the remaining edges are *a* cut. A specific min-cut survives with probability ≥ 2/(n(n−1)), so `Θ(n² log n)` repetitions make failure negligible. It's the clearest example of "an absurdly simple algorithm that works because you run it enough times."

**Randomized rounding:** solve a linear-programming relaxation, then round each fractional variable to 1 with probability equal to its value. Gives provable approximation ratios for set cover and related problems.

### Random number generation

| Need | Use |
| --- | --- |
| General simulation, shuffling, sampling | `rand::thread_rng()` (ChaCha-based, seeded from the OS) |
| Reproducible tests / benchmarks | `StdRng::seed_from_u64(seed)` — **and log the seed** |
| Speed over quality, non-adversarial | `SmallRng`, xorshift, PCG |
| Cryptographic keys, tokens, nonces | `OsRng` / `getrandom` — **never** a general PRNG |
| Hash-table seeds | Handled by `RandomState`; don't roll your own |

**The distinction that matters:** a fast PRNG (xorshift, PCG) is fine for simulation and *fatal* for security — its state is recoverable from a few outputs, so an attacker can predict every subsequent value. Nothing in the type system marks this, which is why the rule is per-use-site.

Also beware **modulo bias** in range generation: `rng() % n` is non-uniform unless `n` divides the generator's range. `gen_range` handles it via rejection sampling; hand-rolled `% n` does not.

## Complexity

| Algorithm | Type | Bound | Holds on |
| --- | --- | --- | --- |
| Randomized quicksort | Las Vegas | Θ(n log n) **expected** | **every** input |
| Randomized quickselect | Las Vegas | **Θ(n) expected** | every input |
| Fisher-Yates | — | Θ(n) | — |
| Reservoir sampling | — | Θ(n) time, **Θ(k) space** | one pass, unknown length |
| Treap | Las Vegas | Θ(log n) expected height | every insertion order |
| Skip list | Las Vegas | Θ(log n) expected | every insertion order |
| Miller-Rabin | Monte Carlo | Θ(k log³ n), error ≤ 4⁻ᵏ | — |
| Karger min-cut | Monte Carlo | Θ(n² log n) for high probability | — |
| Randomized min-cut (Karger-Stein) | Monte Carlo | Θ(n² log³ n) | — |

**Where the table misleads.** "Expected Θ(n log n)" does not mean "usually Θ(n log n) and occasionally terrible in a way you should plan for" — the concentration is extremely tight. Randomized quicksort exceeds `2× its expectation` with probability that falls exponentially in n, so at n = 10⁶ the practical worst case is indistinguishable from the expectation. The *tail* matters far less than the word "expected" suggests.

The other misreading: Monte Carlo error bounds are per-run and independent, so k rounds give εᵏ, not ε. That exponential decay is why a 1/4 error bound is practically useful — 40 rounds is 4⁻⁴⁰ ≈ 10⁻²⁴.

## Use Cases

- **Sorting and selection** — `sort_unstable`'s pivot randomization; `select_nth_unstable`'s expected Θ(n) (measured 10.7× faster than sorting in [selection](../selection-and-order-statistics/learning.md)).
- **Hash tables** — random seeds as the [HashDoS](../hash-tables/learning.md) defence; measured 186× degradation when collisions are forced.
- **Balanced structures without balance code** — treaps and skip lists; skip lists dominate *concurrent* ordered maps because they're far easier to make lock-free.
- **Sampling** — reservoir sampling for logs and streams; weighted sampling via prefix sums plus binary search; A/B test bucketing.
- **Cryptography** — prime generation (Miller-Rabin), key generation, nonces. Note this is the one domain where PRNG quality is a security boundary.
- **Load balancing** — "power of two choices": pick two servers at random and use the less loaded one, which reduces maximum load from Θ(log n / log log n) to Θ(log log n) — a large win for one extra probe.
- **Simulation and Monte Carlo integration** — physics, finance, queueing; also the basis of MCTS in game AI.
- **Approximation** — randomized rounding of LP relaxations; sketch-based estimation ([probabilistic data structures](../probabilistic-data-structures/learning.md)).

## When to Use Which

| Reach for | When |
| --- | --- |
| **Randomized pivot / seed** | Input could be adversarial — converts average into expected |
| **Treap / skip list** | Want balanced-tree behaviour without balance code |
| **Reservoir sampling** | Sample from a stream of unknown length in Θ(k) space |
| `rand::seq::SliceRandom::shuffle` | Shuffling — **never hand-roll** |
| Monte Carlo + repetition | A cheap test with one-sided error; amplify by repeating |
| `OsRng` | Anything security-relevant |
| `StdRng::seed_from_u64` | Reproducible tests and benchmarks |
| **Deterministic algorithm** | You need a hard worst-case bound (real-time, safety-critical) |

## Pitfalls in Depth

### Pitfall: The biased shuffle

- **What goes wrong:** Each position is swapped with a uniformly random position in `0..n` rather than `0..=i`. The result *looks* shuffled and is measurably skewed — measured over 2,000,000 shuffles of 4 elements, permutation frequencies ranged from **24.83% below to 40.37% above** uniform, against Fisher-Yates' ±0.72%.
- **Why it happens (the mechanism):** The naive version makes `n` independent choices from `n` options, giving `nⁿ` equally-likely execution paths. Those must map onto `n!` permutations, and `nⁿ` is generally not divisible by `n!` — for n = 4, 256 paths over 24 permutations. **Uniformity is arithmetically impossible**, regardless of the RNG's quality. The bias is invisible to eyeballing and to most tests because the output is still *a* permutation.
- **How to handle it in production, and why that works:** Use `rand::seq::SliceRandom::shuffle`, or write Fisher-Yates with `j ∈ 0..=i` so the path count is exactly `n!` and the mapping is a bijection. To verify any shuffle implementation, run the frequency test above on n = 4 with a few million trials — a biased shuffle shows up immediately, and a correct one stays within a fraction of a percent.
- **Trade-offs of the fix:** None — same complexity, one character different in the range. The cost is entirely in knowing that the obvious version is wrong.

### Pitfall: Average-case reasoning where expected-case is needed

- **What goes wrong:** A deterministic pivot rule (first element, middle element, median-of-three) is used because it's Θ(n log n) "on average". An attacker who knows the rule constructs input that forces Θ(n²), and a single request pins a core for minutes. The same shape as [hash flooding](../hash-tables/learning.md), where forcing collisions measured **186× degradation at 16,000 keys** with quadratic scaling.
- **Why it happens (the mechanism):** Average-case analysis quantifies over a *distribution of inputs* you assumed; expected-case quantifies over the algorithm's *own coin flips* and therefore holds for every input. If the adversary picks the input and you picked the pivot rule deterministically, the average-case bound provides no protection at all — it was a statement about a distribution the adversary is not drawing from.
- **How to handle it in production, and why that works:** Put the randomness in the algorithm and seed it unpredictably. `sort_unstable` and `RandomState` already do this; hand-rolled quicksort, custom hashers, and load-balancing schemes are where the gap appears. The seed must be per-process (or per-instance) and not derivable from anything the attacker sees.
- **Trade-offs of the fix:** Randomization makes runtimes non-reproducible, which complicates benchmarking and debugging — use a fixed seed in tests and log it on failure. It also costs an RNG call in the hot path, which for pivot selection is negligible and for a per-lookup hash is measurable (measured in [hashing techniques](../hashing-techniques/learning.md): SipHash costs ~12 ns for a 4-byte key against a fast hash's 2.4 ns).

### Pitfall: A fast PRNG where a cryptographic one is required

- **What goes wrong:** Session tokens, password-reset links, or API keys are generated with `SmallRng`, xorshift, or a seeded `StdRng` with a predictable seed (the timestamp). An attacker who observes a few outputs recovers the internal state and can predict every subsequent token — including ones already issued to other users.
- **Why it happens (the mechanism):** Non-cryptographic PRNGs are designed for *statistical* quality, not unpredictability: xorshift's full state is recoverable from a handful of consecutive outputs by construction, and a timestamp seed has only ~2³⁰ realistic values. Both produce output that passes randomness tests and is completely predictable to an adversary — the properties are unrelated.
- **How to handle it in production, and why that works:** Use `OsRng`/`getrandom` for anything an attacker would benefit from predicting — tokens, nonces, keys, salts, password-reset identifiers. It draws from the OS CSPRNG, which is designed so that observing output reveals nothing about future output. Reserve fast PRNGs for simulation, sampling, and testing.
- **Trade-offs of the fix:** `OsRng` is a syscall (or vDSO call) and is meaningfully slower than a userspace PRNG — irrelevant for token generation, potentially relevant if you needed millions of values per second, in which case seed a CSPRNG (`ChaCha20Rng`) from `OsRng` once and draw from that.

### Pitfall: Modulo bias in range generation

- **What goes wrong:** A random index is generated with `rng.next_u64() % n`. When `n` doesn't divide 2⁶⁴, low values are slightly more likely than high ones. For small `n` the bias is negligible; for `n` close to the generator's range — or when the value drives something security-relevant like a lottery or a shard assignment — it's a real skew.
- **Why it happens (the mechanism):** `2⁶⁴ mod n` values at the top of the generator's range map onto the first `2⁶⁴ mod n` outputs, giving those outputs one extra chance. It's the same counting argument as the biased shuffle: you can't partition `2⁶⁴` equally-likely values into `n` equal groups unless `n` divides `2⁶⁴`.
- **How to handle it in production, and why that works:** Use `rng.gen_range(0..n)`, which performs rejection sampling — discard values in the biased tail and redraw, giving exact uniformity at the cost of an occasional extra draw (expected < 2 draws). Or use Lemire's multiply-shift method, which is what `rand` actually implements for speed.
- **Trade-offs of the fix:** Rejection sampling makes the *time* non-deterministic (unbounded in the worst case, though the probability decays geometrically), which matters only in constant-time cryptographic contexts. `gen_range` is otherwise strictly better than `%`.

### Pitfall: Non-reproducible tests and benchmarks

- **What goes wrong:** A randomized algorithm's test uses `thread_rng()`, fails once in 500 runs, and cannot be reproduced. The failure is dismissed as flakiness and the underlying bug ships. Or a benchmark uses fresh randomness per run, so measurements vary by more than the change being measured.
- **Why it happens (the mechanism):** Randomness is a hidden input. The test's outcome depends on it exactly as much as on the code, but nothing records it, so a failing run is unreconstructible — the same hidden-input problem as unseeded time or `HashMap` iteration order in [testing](../../language-best-practices/rust/testing.md).
- **How to handle it in production, and why that works:** Seed explicitly with `StdRng::seed_from_u64(seed)` and **print the seed on failure** so any failing case is reproducible by re-running with that seed. For property tests, `proptest` does this automatically — it persists failing seeds to a file and replays them, which is the single best argument for using it over hand-rolled random testing. For benchmarks, fix the seed so input is identical across runs.
- **Trade-offs of the fix:** A fixed seed makes the test *less* exploratory — it examines one sample forever. The good compromise is `proptest`'s: fresh randomness each run for coverage, plus automatic persistence of any seed that fails, so you get exploration and reproducibility together.
