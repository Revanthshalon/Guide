# Number Theory & Combinatorics — Quick Reference

## At a Glance

Two ideas underpin everything: **modular arithmetic** keeps astronomically large results in a `u64`, and **exponentiation by squaring** turns Θ(n) into Θ(log n) for any associative operation.

**Invariants:** `+`, `−`, `×` mod m are well-defined; **division is not** — it needs an inverse, which exists iff `gcd(b,m)=1`. `%` in Rust is a **remainder** (signed), not a modulus. Multiplication **overflows before** the modulus is applied.

## The Numbers (measured)

| Task | Result |
| --- | --- |
| Primes below 10⁷: sieve | **20.49 ms** (664,579 primes) |
| Primes below 10⁷: trial division | **~172× slower** |
| 100k modular inverses via Fermat | **23.1 ms** (~30 squarings each) |

## The Six Routines

```rust
const M: u64 = 1_000_000_007;

fn modpow(b: u64, mut e: u64, m: u64) -> u64 {              // Θ(log e)
    let (mut r, mut b) = (1u64, b % m);
    while e > 0 {
        if e & 1 == 1 { r = (r as u128 * b as u128 % m as u128) as u64; }
        b = (b as u128 * b as u128 % m as u128) as u64;     // u128 — u64 overflows
        e >>= 1;
    }
    r
}
fn inv(a: u64, m: u64) -> u64 { modpow(a, m - 2, m) }        // m must be PRIME
fn gcd(mut a: u64, mut b: u64) -> u64 { while b != 0 { let t = b; b = a % b; a = t; } a }
fn ext_gcd(a: i64, b: i64) -> (i64,i64,i64) {                // inverse for ANY coprime m
    if b == 0 { (a,1,0) } else { let (g,x,y) = ext_gcd(b, a%b); (g, y, x - (a/b)*y) } }

// Sieve: start at i², stop at √n
while i*i <= n { if is[i] { let mut j = i*i; while j <= n { is[j]=false; j+=i; } } i+=1; }

// inv_fact BACKWARDS — one modpow total, not n
inv_fact[n] = inv(fact[n]);
for i in (1..=n).rev() { inv_fact[i-1] = inv_fact[i] * i % M; }
```

## Complexity

| Operation | Cost |
| --- | --- |
| `gcd` | Θ(log min(a,b)) |
| **`modpow`** | **Θ(log e)** |
| **Sieve to n** | **Θ(n log log n)** ≈ linear |
| Linear sieve | Θ(n) |
| **Miller-Rabin** (one number) | Θ(k log³ n), error 4⁻ᵏ |
| Factorization (trial / Pollard) | Θ(√n) / **Θ(n^¼)** expected |
| nCr, precomputed factorials | **Θ(1)** after Θ(n) |
| **Matrix exponentiation** (k×k) | **Θ(k³ log n)** |
| **FFT / NTT** | **Θ(n log n)** |

**Θ(√n) factorization is exponential in the input *size*** (log n digits) — which is why RSA is secure. Same trap as knapsack's Θ(n·W).

## Choose This When

| Use | For |
| --- | --- |
| **`modpow`** | `a^b mod m`; inverse with **prime** m |
| **Extended Euclid** | Inverse with **composite** (coprime) m — and it reports failure |
| **Sieve** | *All* primes ≤ n |
| **Miller-Rabin** | Primality of *one* large number |
| Pollard's rho | Factoring beyond trial division |
| **Precomputed factorials** | Many nCr queries mod a prime |
| Lucas' theorem | nCr mod small prime, astronomically large n |
| **Matrix exponentiation** | Linear recurrence, huge n (n=10¹⁸ ⇒ ~60 mults) |
| **FFT / NTT** | Convolution, polynomial / big-int multiply |
| `num-bigint` | Arbitrary precision — don't hand-roll |

## Rules of Thumb

- Every modular multiply goes through `u128`.
- Subtraction: `(a + m − b) % m`, or `rem_euclid`.
- Prime modulus (`10⁹+7`, `998244353`) makes division always defined.
- Sieve: start at **i²**, stop at **√n**.
- Compute `inv_fact` **backwards** — one modpow, not n.
- Sieve = batch (range-bound). Miller-Rabin = one number (size-bound). Don't swap them.
- NTT over FFT when inputs are integers — no floating-point error.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| `u64` modular multiply | Silent wrap once you chain three factors |
| `(a - b) % m` | Negative index panic / wrong branch |
| Fermat inverse with composite m | Silently wrong for non-coprime b |
| `modpow` per query for inverses | Θ(n log m) where Θ(n + log m) suffices |
| Sieve to 10¹² for one primality test | Impossible — use Miller-Rabin |
| Trial division for all primes ≤ n | ~172× slower than a sieve |

## Key References

- CLRS ch. 31 — number-theoretic algorithms
- Cormen/Knuth on FFT · Cooley & Tukey (1965)
- [`num-bigint`](https://docs.rs/num-bigint/) · Shoup, *A Computational Introduction to Number Theory and Algebra* (free)
