# Hashing Techniques — Quick Reference

## At a Glance

Compresses arbitrary data to fixed bits, destroying information on purpose. **"Hash function" is five different jobs** with incompatible requirements — picking the wrong family is the core failure mode.

**Invariant:** `a == b` ⟹ `hash(a) == hash(b)`. The implication runs **one way only** — unequal values may collide, and that's fine.

## The Five Jobs

| Job | Needs | Use |
| --- | --- | --- |
| Table index | Uniformity, speed | FxHash, aHash |
| DoS-resistant index | + **keyed** | SipHash-1-3 (Rust default) |
| Integrity / dedup | Collision resistance | BLAKE3, SHA-256 |
| Passwords | **Slowness**, memory-hard, salted | Argon2, bcrypt |
| Sharding / partitioning | **Stability across runs & languages** | xxHash, consistent hashing |

**Decision order:** (1) does an adversary gain from a collision? → cryptographic. (2) is the value read in another process/language/run? → stability-documented. (3) otherwise → fastest.

## Measured Cost (this machine)

| Key bytes | SipHash ns | Fx ns | Ratio | SipHash ns/byte |
| --- | --- | --- | --- | --- |
| 4 | 12.17 | 2.35 | 5.18× | 3.043 |
| 16 | 21.08 | 3.18 | **6.63×** | 1.317 |
| 64 | 41.43 | 7.48 | 5.54× | 0.647 |
| 256 | 107.60 | 26.78 | 4.02× | 0.420 |
| 1024 | 321.29 | 133.36 | 2.41× | 0.314 |

**SipHash has a large fixed cost** (~12 ns for 4 B) and runs ~0.31 ns/byte (≈3.2 GB/s) asymptotically. The "~1 ns/byte" folklore is wrong at both ends. The Fx advantage shrinks with length — which is why the swap matters for small integer keys and barely for long strings.

## Complexity

| Technique | Cost |
| --- | --- |
| Hash a k-byte key | Θ(k), large constant when k is small |
| **Rolling hash slide** | **Θ(1)** regardless of window |
| Rabin-Karp | Θ(n+m) expected, Θ(n·m) worst |
| Consistent hashing lookup | Θ(log V) |
| Consistent hashing, add node | **~1/N keys move** (vs ~all for `% N`) |
| Perfect hash | **Θ(1) worst case**, fixed key set |
| Cryptographic | Θ(k), ~10× slower |

## Snippets

```rust
#[derive(PartialEq, Eq, Hash)] struct Key { a: u32, b: String }   // derive keeps them in sync

// Hand-written: feed fields into the SAME hasher, in order. NEVER XOR sub-hashes.
impl Hash for Point {
    fn hash<H: Hasher>(&self, s: &mut H) { self.x.hash(s); self.y.hash(s); }
}

// Rolling hash — Θ(1) per slide
h = (h + M - out as u64 * pow % M) % M;
h = (h * base + incoming as u64) % M;
```

## Rules of Thumb

- **Never persist `DefaultHasher::finish()`** — `RandomState` reseeds per `HashMap` *instance*, not just per process.
- Derive `Hash`; if hand-writing, delegate to `state` and hand-write `Eq` over the same fields.
- Feed a length before variable-length sequences, or `["ab","c"]` collides with `["a","bc"]`.
- Take bits from the **strong** end of a weak hash, or apply a finalizer.
- A rolling-hash match is a **candidate** — always verify.
- Passwords never share a hash choice with anything else.
- Intern long keys before optimizing the hasher.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Persisted a randomized hash | Total cache miss after restart; records unfindable; wrong shard |
| `h(x) ^ h(y)` combining | `(1,2)` == `(2,1)`; `(a,a)` hashes to 0; mysteriously slow map |
| No length prefix on sequences | `["ab","c"]` collides with `["a","bc"]` |
| Fast hash for untrusted dedup | Forgeable fingerprint — attacker overwrites others' data |
| SHA-256 for passwords | Leaked table cracked at billions/sec |
| Randomized hash for partitioning | Same key → different partitions; per-key ordering breaks |
| Low bits of a multiply-only hash | Clustering; long probes; invisible in tests |

## Key References

- Aumasson & Bernstein, ["SipHash"](https://www.aumasson.jp/siphash/siphash.pdf)
- Karger et al. (1997) — consistent hashing
- [`std::hash::Hash`](https://doc.rust-lang.org/std/hash/trait.Hash.html) — the contract and the stability warning
