# Hash Tables — Quick Reference

## At a Glance

An array you index by *computing* the position. Trades space and a hash computation for skipping the search. Guarantee is **expected** Θ(1) — it holds exactly as long as the hash's uniformity does.

**Invariant:** every key is reachable from `hash(key)` along its probe sequence without hitting an "empty" marker first; occupied ≤ `load_factor × buckets`.

## Complexity

| Operation | Average | **Worst** | Space |
| --- | --- | --- | --- |
| Lookup / insert / delete | Θ(1) | **Θ(n)** | — |
| Iterate | Θ(**capacity**) | Θ(capacity) | — |
| Structure | — | — | ~1.14n + control bytes |

Θ(1) is in *entries*; a `String` key is also Θ(k) in key length.

## Measured (this machine)

| Thing | Number |
| --- | --- |
| Load factor (SwissTable) | **7/8** |
| Capacity sequence | 3, 7, 14, 28, 56, 112, 224, 448, 896 |
| `with_capacity(100).capacity()` | **112** (vs `Vec`'s exact 100) |
| Insert 1M: grown vs preallocated | 93.5 ms vs 35.0 ms — **2.67×** |
| `contains_key`+`insert` vs `entry()` | 28.1 ms vs 20.3 ms — **1.38×** |
| `u32` lookup, cache-resident | ~8 ns |
| `u32` lookup, 10M entries, cold | ~35 ns |
| Beats `Vec::contains` from | **n ≈ 12** |
| Beats sorted `Vec` + binary search from | **n ≈ 32** |

## HashDoS — all keys colliding

| n | Normal | Colliding | Ratio |
| --- | --- | --- | --- |
| 1,000 | 119.9 µs | 499.3 µs | 4× |
| 4,000 | 165.0 µs | 7.64 ms | 46× |
| 16,000 | 686.0 µs | **127.4 ms** | **186×** |

16× the keys, 255× the time — **quadratic**. Rust's `RandomState` (per-**instance** seed) is the defence.

## Hasher Choice — Per Map, By Key Provenance

| Keys come from | Use | Speed vs default |
| --- | --- | --- |
| User input, network, files | **`RandomState`** (default) | 1× — keep it |
| You generate them (indices, interned IDs) | `FxHashMap` / `rustc-hash` | **4.6–6.0×** (`u32`) |
| Want both | `ahash` | fast *and* DoS-resistant |

Note: for 16-char `String` keys the Fx win is only **1.19–1.29×** — the hash isn't the bottleneck there. Intern first.

## Choose This When

| Use | For |
| --- | --- |
| **`HashMap`/`HashSet`** | Default keyed lookup above n ≈ 12–32 |
| `BTreeMap` | Ordered iteration, range queries, determinism |
| Sorted `Vec` | Footprint / one allocation / ordering — **not** speed |
| Direct array index | Dense small integer keys — no hashing at all |
| `IndexMap` | `HashMap` speed **+** insertion order |
| `phf` | Fixed key set known at compile time |
| `dashmap` | Concurrent, sharded |
| Arena + `u32` handle | Keys are yours to assign — skip the map |

## Snippets

```rust
let mut m = HashMap::with_capacity(expected);      // 2.67× on bulk insert
*m.entry(k).or_insert(0) += 1;                     // one hash, not two
m.entry(k).or_insert_with(Vec::new).push(x);       // lazy default
m.get("literal");                                  // Borrow: no alloc to query a String-keyed map

let mut v: Vec<_> = m.iter().collect();            // NEVER rely on iteration order
v.sort_unstable_by_key(|(k, _)| *k);

m.retain(|_, v| v.is_live());
m.shrink_to_fit();                                 // capacity never shrinks on its own

type FastMap<K,V> = HashMap<K,V,BuildHasherDefault<FxHasher>>;  // self-generated keys only
```

## Collision Resolution

| Strategy | Locality | Note |
| --- | --- | --- |
| Separate chaining | poor | Pointer chase + alloc per bucket |
| **Linear probing** | **best** | Primary clustering, but locality wins on real hardware |
| Quadratic / double hashing | worse | Breaks clustering, costs locality |
| Robin Hood | good | Bounds probe-length variance |

**SwissTable** = linear probing + a separate control-byte array; a probe SIMD-compares **16 tags at once**.

## Rules of Thumb

- Preallocate — rehashing is far costlier than `Vec`'s memcpy.
- `entry()` over `contains_key` + `insert`, always.
- Sort before any observable iteration; or use `IndexMap`/`BTreeMap`.
- Hasher is a **per-map** decision by key provenance, never a global one.
- Intern `String` keys to `u32` before optimizing the hasher.
- Derive `PartialEq`/`Eq`/`Hash` together; never put interior mutability in a key.
- Deleting from open addressing needs tombstones or backward-shift.
- Capacity never shrinks — `shrink_to_fit` after bulk removal, on a threshold.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| `Hash`/`Eq` disagree | Insert then `get` returns `None`; duplicate set entries |
| Mutated key (interior mutability) | Entry permanently unreachable |
| Relied on iteration order | Flaky tests, noisy diffs, cache misses — differs *per map instance* |
| Swapped to `FxHashMap` on user keys | 186× DoS vector at 16k keys |
| No `with_capacity` on bulk build | 2.67× slower |
| Never `shrink_to_fit` | Iteration stays Θ(old capacity); memory retained |
| Blanked slot on delete | Later keys become unfindable |

## Key References

- [`hashbrown`](https://github.com/rust-lang/hashbrown) — the SwissTable behind `HashMap`
- Kulukundis, "Designing a Fast, Efficient, Cache-friendly Hash Table" (CppCon 2017)
- Crosby & Wallach (2003) — algorithmic complexity attacks
