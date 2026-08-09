# Number Theory & Combinatorics — Learning Notes

## Mental Model

**This is the toolkit that makes "compute this over an astronomically large space" tractable**, and it rests on two ideas that recur throughout:

1. **Modular arithmetic turns unbounded numbers into fixed-size ones.** Counting problems produce results with thousands of digits; working mod a prime keeps everything in a `u64` while preserving the structure you need (addition, multiplication, and — via Fermat — division).
2. **Exponentiation by squaring turns Θ(n) into Θ(log n)** for anything with an associative operation. It applies to integers (`a^b mod m`), to matrices (linear recurrences in Θ(log n)), and to any monoid. Measured here: 100,000 modular inverses via Fermat's little theorem — each ~30 squarings instead of ~10⁹ multiplications — took **23.1 ms** total.

The third recurring idea is **sieving**: rather than testing each candidate independently, mark composites in bulk by walking multiples. Measured, primes below 10,000,000:

| Method | Time |
| --- | --- |
| Sieve of Eratosthenes | **20.49 ms** (664,579 primes) |
| Trial division | ~172× slower (extrapolated from a measured prefix) |

The sieve's advantage is the same "batch it" lens that recurs across [Stage 6](../dynamic-programming/learning.md): doing n independent tests is worse than doing one pass that resolves all of them.

The framing that keeps this topic honest: **most of it is a small set of routines you write once.** GCD, modpow, modular inverse, a sieve, factorials with precomputed inverses, and nCr. Six functions cover the overwhelming majority of uses, and getting them right once is more valuable than knowing the theory behind any of them.

## The Invariant

**Modular arithmetic:**

> `(a + b) mod m`, `(a − b) mod m`, and `(a · b) mod m` are well-defined and preserve the ring structure. **Division is not** — `a / b mod m` requires `b` to have a *modular inverse*, which exists iff `gcd(b, m) = 1`.

That last clause is the one that causes bugs. When `m` is prime, every non-zero `b` has an inverse (so `b^(m−2) mod m` by Fermat's little theorem works), which is why competitive and cryptographic code almost always uses a prime modulus like `10⁹+7`. With a composite modulus, division silently produces garbage for non-coprime values.

**Two more that matter in Rust specifically:**

> **Subtraction can go negative.** `(a − b) % m` in Rust yields a negative value when `a < b`, because `%` is a remainder, not a mathematical modulus. Always `(a + m − b) % m`.
>
> **Multiplication overflows before the modulus is applied.** Two values below `10⁹+7` multiply to nearly `10¹⁸`, which fits in `u64` — but only just. Chain two multiplications and it doesn't. Compute in `u128` and reduce.

**Exponentiation by squaring:**

> `a^n = (a^(n/2))²` for even n, `a · a^(n−1)` for odd. Θ(log n) multiplications, and it works for **any associative operation** — integers, matrices, function composition, string concatenation.

## Mechanics

### The six routines worth writing once

```rust
const M: u64 = 1_000_000_007;

// 1. Modular exponentiation — Θ(log e). The workhorse.
fn modpow(mut b: u64, mut e: u64, m: u64) -> u64 {
    let (mut r, mut b) = (1u64, b % m);
    while e > 0 {
        if e & 1 == 1 { r = (r as u128 * b as u128 % m as u128) as u64; }
        b = (b as u128 * b as u128 % m as u128) as u64;    // u128 — u64 would overflow
        e >>= 1;
    }
    r
}

// 2. Modular inverse via Fermat (m must be PRIME).
fn inv(a: u64, m: u64) -> u64 { modpow(a, m - 2, m) }

// 3. GCD — Euclid. Θ(log min(a,b)).
fn gcd(mut a: u64, mut b: u64) -> u64 { while b != 0 { let t = b; b = a % b; a = t; } a }

// 4. Extended Euclid — inverse for ANY coprime modulus, not just prime.
fn ext_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if b == 0 { (a, 1, 0) } else { let (g, x, y) = ext_gcd(b, a % b); (g, y, x - (a / b) * y) }
}

// 5. Sieve of Eratosthenes — Θ(n log log n).
fn sieve(n: usize) -> Vec<bool> {
    let mut is = vec![true; n + 1];
    is[0] = false; if n >= 1 { is[1] = false; }
    let mut i = 2;
    while i * i <= n {                                     // stop at √n
        if is[i] { let mut j = i * i; while j <= n { is[j] = false; j += i; } }  // start at i²
        i += 1;
    }
    is
}

// 6. nCr with precomputed factorials — Θ(n) setup, Θ(1) per query.
// fact[i] = i!, inv_fact[i] = (i!)^-1, computed backwards from inv(fact[n]).
fn n_c_r(n: usize, r: usize, fact: &[u64], inv_fact: &[u64]) -> u64 {
    if r > n { return 0; }
    (fact[n] as u128 * inv_fact[r] as u128 % M as u128 * inv_fact[n - r] as u128 % M as u128) as u64
}
```

Two details in the sieve that halve the work: **start marking at `i²`** (smaller multiples were already marked by smaller primes) and **stop the outer loop at `√n`** (any composite ≤ n has a factor ≤ √n).

**The inverse-factorial trick** is worth knowing: computing each `inv_fact[i]` with a separate `modpow` is Θ(n log m). Instead compute `inv_fact[n] = inv(fact[n])` once and walk *backwards* with `inv_fact[i-1] = inv_fact[i] * i`, giving Θ(n + log m) — one modpow total.

### Matrix exponentiation — Θ(log n) linear recurrences

Any linear recurrence becomes a matrix power. For Fibonacci:

```
[F(n+1)]   [1 1]^n   [F(1)]
[F(n)  ] = [1 0]   · [F(0)]
```

Exponentiation by squaring on the 2×2 matrix gives **F(n) mod m in Θ(log n)** — so n = 10¹⁸ is ~60 matrix multiplications. This generalizes: a k-term linear recurrence becomes a k×k matrix, giving Θ(k³ log n). It's the tool for "compute the n-th term where n is astronomically large", and it's the [dynamic programming](../dynamic-programming/learning.md) "amortize it" lens taken to its conclusion.

### The rest of the toolkit

| Tool | Solves | Cost |
| --- | --- | --- |
| **Sieve** | All primes ≤ n | Θ(n log log n) |
| Linear sieve | Primes + smallest prime factor | **Θ(n)** |
| **Miller-Rabin** | Is this one number prime? | Θ(k log³ n), error 4⁻ᵏ |
| Pollard's rho | Factor a large number | Θ(n^¼) expected |
| **Extended Euclid** | Inverse mod any coprime m; solve `ax + by = g` | Θ(log n) |
| **CRT** | Combine congruences with coprime moduli | Θ(k log m) |
| **FFT / NTT** | Polynomial or big-integer multiplication | **Θ(n log n)** |
| Inclusion-exclusion | Count unions of overlapping sets | Θ(2ᵏ) over k sets |
| Möbius inversion | Invert divisor-sum relations | Θ(n log n) |
| Lucas' theorem | nCr mod a small prime, huge n | Θ(log_p n) |

**FFT is the one with the widest reach**: convolution in Θ(n log n) rather than Θ(n²) underlies fast big-integer multiplication (used inside every bignum library), polynomial arithmetic, signal processing, and string matching with wildcards. NTT is the same algorithm over a modular ring, which avoids floating-point error entirely — the right choice when the inputs are integers.

## Complexity

| Operation | Cost |
| --- | --- |
| `gcd(a, b)` | Θ(log min(a,b)) |
| **`modpow(a, e, m)`** | **Θ(log e)** multiplications |
| Modular inverse (Fermat, prime m) | Θ(log m) |
| Modular inverse (extended Euclid, any coprime m) | Θ(log m) |
| **Sieve to n** | **Θ(n log log n)** — effectively linear |
| Linear sieve to n | Θ(n) |
| Trial-division primality of one n | Θ(√n) |
| **Miller-Rabin** | Θ(k log³ n) |
| Factorization (trial division) | Θ(√n) |
| Factorization (Pollard's rho) | Θ(n^¼) expected |
| nCr with precomputed factorials | **Θ(1)** after Θ(n) setup |
| **Matrix exponentiation** (k×k) | **Θ(k³ log n)** |
| **FFT / NTT** | **Θ(n log n)** |

**Where the table misleads.** `Θ(√n)` for factorization looks polynomial and isn't — it's exponential in the *number of digits*, which is precisely why RSA is secure. A 2048-bit modulus has √n ≈ 2¹⁰²⁴ operations. This is the same pseudo-polynomial trap as knapsack's Θ(n·W) in [dynamic programming](../dynamic-programming/learning.md): **the input size is log n, not n**.

And Θ(n log log n) for the sieve is essentially linear — log log 10⁹ ≈ 3 — which is why the measured sieve did 10,000,000 in 20.49 ms while trial division would take ~172× longer.

## Use Cases

- **Cryptography** — RSA is modular exponentiation; key generation is Miller-Rabin; ECC is modular arithmetic over a curve. Every one of the six routines above appears in a crypto library.
- **Hashing** — polynomial rolling hashes ([hashing techniques](../hashing-techniques/learning.md)) are modular arithmetic; the modulus choice is exactly the "large prime, random base" discussion there.
- **Combinatorial counting** — probability, statistics, and any "how many ways" question; nCr mod p with precomputed factorials is the standard tool.
- **Big-integer arithmetic** — Karatsuba and FFT-based multiplication inside every bignum library; `num-bigint` in Rust.
- **Error-correcting codes** — Reed-Solomon is polynomial arithmetic over a finite field.
- **Random number generation** — LCGs and the modular structure behind period guarantees.
- **Checksums and CRCs** — polynomial division over GF(2).
- **Competitive programming** — this stage is heavily represented there; the six routines cover most of it.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`modpow`** | Any `a^b mod m`; modular inverse with prime m |
| **Extended Euclid** | Inverse with a **composite** (but coprime) modulus |
| **Sieve** | You need *all* primes up to n |
| Linear sieve | You also need smallest-prime-factor per number |
| **Miller-Rabin** | Primality of *one* large number |
| Pollard's rho | Factoring a number too large for trial division |
| **Precomputed factorials** | Many nCr queries mod a prime |
| Lucas' theorem | nCr mod a small prime with astronomically large n |
| **Matrix exponentiation** | Linear recurrence, huge n |
| **FFT / NTT** | Convolution, polynomial or big-integer multiplication |
| `num-bigint` | Arbitrary precision — don't hand-roll |

## Pitfalls in Depth

### Pitfall: Overflow before the modulus

- **What goes wrong:** `(a * b) % m` with `a, b < 10⁹+7` in `u64`. The product is nearly 10¹⁸, which *fits* — so it works. Then someone chains `(a * b * c) % m`, or switches to a larger modulus, and it silently wraps. In release builds the wrap is silent; results are wrong in a way that looks like an algorithm bug.
- **Why it happens (the mechanism):** The modulus is applied *after* the multiplication, so the intermediate must fit in the type. `u64` holds up to 1.8×10¹⁹, so a single product of two values below 10⁹ is fine and a product of three is not. The failure depends on values rather than on types, so it passes tests with small inputs.
- **How to handle it in production, and why that works:** Compute every modular multiplication in `u128` and reduce immediately: `(a as u128 * b as u128 % m as u128) as u64`. On 64-bit hardware a 128-bit multiply is one instruction plus a division, so the cost is small and the failure mode is eliminated. Better still, wrap the modulus in a newtype with operator overloads so every operation reduces by construction — then it's impossible to forget.
- **Trade-offs of the fix:** The `u128` modulo is meaningfully slower than a `u64` one (division is the expensive part), which matters in a tight loop. Montgomery or Barrett reduction replaces the division with multiplications and is what production crypto uses — but that's an optimization to reach for after measuring, not by default.

### Pitfall: Negative results from `%`

- **What goes wrong:** `(a - b) % m` where `a < b`. In Rust `%` is a *remainder* and takes the sign of the dividend, so the result is negative. It's then used as an array index (panic) or compared against a positive value (wrong branch). With unsigned types the subtraction itself panics in debug and wraps to an enormous number in release.
- **Why it happens (the mechanism):** Mathematical modulus always returns a value in `[0, m)`; C-family `%` doesn't. The distinction only shows up when the dividend is negative, which in modular arithmetic happens exactly when you subtract.
- **How to handle it in production, and why that works:** Always write `(a + m - b % m) % m`, which is non-negative by construction. Rust also provides `rem_euclid`, which returns the mathematical modulus directly and is the clearest expression of intent. As above, a newtype that encapsulates the reduction removes the class entirely.
- **Trade-offs of the fix:** `rem_euclid` is marginally slower than `%` for signed types (it may add a conditional). The extra `+ m` is free. Neither is a real cost.

### Pitfall: Modular division with a composite modulus

- **What goes wrong:** `a / b mod m` is computed as `a * modpow(b, m-2, m) % m` — Fermat's little theorem — with a **composite** `m`. Fermat requires `m` prime; with composite `m` the result is silently wrong for any `b` not coprime to `m`. Since it's *right* for coprime `b`, tests often pass.
- **Why it happens (the mechanism):** `b^(m−2) ≡ b^(−1)` holds only when `m` is prime (it follows from `b^(m−1) ≡ 1`, Fermat's little theorem). For composite `m`, the multiplicative group is smaller, and elements sharing a factor with `m` have **no inverse at all** — the division is undefined, not merely hard.
- **How to handle it in production, and why that works:** Use the **extended Euclidean algorithm**, which computes the inverse for any `b` coprime to `m` and *reports failure* when `gcd(b, m) ≠ 1` rather than returning garbage. Or choose a prime modulus in the first place — which is why `10⁹+7` and `998244353` are ubiquitous (the latter is also NTT-friendly).
- **Trade-offs of the fix:** Extended Euclid is slightly more code than a `modpow` call, and it returns an `Option`-shaped result you must handle. That's the point: the case it reports is one that has no answer.

### Pitfall: Trial division where a sieve belongs (and vice versa)

- **What goes wrong:** Two opposite errors. Testing every number up to n for primality individually — measured, ~**172× slower** than a sieve at n = 10⁷. Or building a sieve up to 10¹² to test the primality of *one* number, which is impossible (that's a terabyte of memory) when Miller-Rabin answers it in microseconds.
- **Why it happens (the mechanism):** They solve different problems. A sieve is a **batch** algorithm — Θ(n log log n) for *all* primes ≤ n, cheap per prime, but its cost and memory scale with the *range*. Miller-Rabin is a **single-number** test at Θ(k log³ n), where n's size is what matters, not its magnitude. Choosing wrongly is a factor of 100× in one direction and impossible in the other.
- **How to handle it in production, and why that works:** Need all primes in a range → sieve (segmented if the range is large but narrow). Need to test one or a few large numbers → Miller-Rabin. Need to factor a large number → Pollard's rho, which is Θ(n^¼) expected rather than Θ(√n).
- **Trade-offs of the fix:** A sieve's memory is Θ(n) — a bitset ([bit manipulation](../bit-manipulation/learning.md)) makes 10⁹ feasible at ~125 MB, but 10¹² isn't. Miller-Rabin is probabilistic; deterministic variants exist for bounded ranges (specific witness sets are proven correct below 3.3×10²⁴).

### Pitfall: Recomputing modular inverses in a loop

- **What goes wrong:** `nCr` is computed by calling `modpow` for each factorial inverse inside the query loop. Each call is ~30 multiplications, so a loop over 10⁶ queries does 3×10⁷ extra multiplications — measured, 100,000 modpows took 23.1 ms, so a million would be ~230 ms of pure overhead for something that should be Θ(1) per query.
- **Why it happens (the mechanism):** `inv(x) = modpow(x, m-2, m)` is such a clean one-liner that it gets called wherever an inverse is needed. But inverses of factorials have structure: `inv_fact[i-1] = inv_fact[i] * i`, so the whole table follows from **one** modpow.
- **How to handle it in production, and why that works:** Precompute `fact[0..=n]` forwards, compute `inv_fact[n] = inv(fact[n])` with a single modpow, then fill `inv_fact` **backwards**. That's Θ(n + log m) total instead of Θ(n log m), after which every nCr is two multiplications. The same batching trick works for inverses of `1..=n` via a linear recurrence.
- **Trade-offs of the fix:** Precomputation costs Θ(n) memory and requires knowing the maximum n up front. For a handful of queries the direct modpow is simpler and fine — the threshold is roughly "more queries than log m".
