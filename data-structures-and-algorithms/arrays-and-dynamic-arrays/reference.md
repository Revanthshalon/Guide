# Arrays & Dynamic Arrays — Quick Reference

## At a Glance

Contiguity, not indexing, is the feature. `Vec<T>` = `(ptr, len, cap)` with geometric growth, giving Θ(1) amortized push without giving up the flat scan.

**Invariant:** `cap ≥ len`; `ptr[0..len]` initialized and valid; `ptr[len..cap]` allocated but uninitialized; one allocation, correctly aligned.

## Complexity

| Operation | Average | Worst | Space |
| --- | --- | --- | --- |
| Index | Θ(1) | Θ(1) | — |
| Push / pop back | Θ(1) amortized | Θ(n) realloc | — |
| `insert(i)` / `remove(i)` | Θ(n − i) | Θ(n) | — |
| `swap_remove(i)` | **Θ(1)** | Θ(1) | — |
| `retain` / `drain` | Θ(n) | Θ(n) | — |
| Search unsorted | Θ(n) | Θ(n) | — |
| `binary_search` | Θ(log n) | Θ(log n) | — |
| Whole structure | — | — | Θ(cap), ≤ 2n |

## Choose This When

| Use it when | Use something else when |
| --- | --- |
| Default — any growable sequence | Need both ends → `VecDeque` |
| Scanning, slicing, SIMD, FFI | Keyed lookup at scale → `HashMap` |
| Built once, read many | Frequent middle insertion → `BTreeMap`/list |
| Want ordered iteration, range queries, compact footprint | **Lookup speed** at n ≳ 32 → `HashMap` |

## Rust

| Need | Use | Handle size |
| --- | --- | --- |
| Growable | `Vec<T>` | 24 B |
| Built once, frozen | `Box<[T]>` via `into_boxed_slice` | 16 B |
| Function parameter | **`&[T]`**, never `&Vec<T>` | 16 B |
| Compile-time N | `[T; N]` | inline |
| Usually tiny, many instances | `smallvec` / `tinyvec` | inline + tag |
| No allocator | `arrayvec` | inline |

## Growth (measured)

| Type | Capacity sequence |
| --- | --- |
| `Vec<u8>` | 8, 16, 32, 64, … |
| `Vec<u64>` | 4, 8, 16, 32, … |

First cap: size 1 → 8, size ≤ 1024 → 4, larger → 1. Then doubling.
`with_capacity(100).capacity() == 100` (exact). `Vec<()>` capacity = `usize::MAX`.

## Snippets

```rust
let mut v = Vec::with_capacity(n);        // known size → no spikes
v.retain(|x| x.is_valid());               // filter in place, order kept
let x = v.swap_remove(i);                 // O(1), order destroyed
let v: Vec<_> = iter.collect();           // sizes via size_hint
let frozen: Box<[T]> = v.into_boxed_slice();
match v.binary_search(&k) {               // Err(i) IS the insertion point
    Ok(i) => v[i] = new,
    Err(i) => v.insert(i, new),
}
v.sort_unstable(); v.dedup();             // dedup alone = CONSECUTIVE only
```

## Rules of Thumb

- `Vec` is the default; departing from it needs a stated reason.
- Take `&[T]` in signatures, not `&Vec<T>`.
- Know n → `with_capacity`. About to add k → `reserve`.
- `remove(0)` in a loop is Θ(n²) — use `VecDeque`, `swap_remove`, `retain`, or push-then-`reverse`.
- `clear()` never frees; only `shrink_to_fit`/`shrink_to` does.
- Linear scan beats `binary_search` only below n≈24 (measured, `u32`) — not the "few hundred" folklore claims.
- Sorted `Vec` as a map is for ordering/footprint/one-allocation — **not** speed. Measured (`u32`): `HashMap` beats it from n ≈ 32, `HashSet` beats a linear scan from n ≈ 12, and the gap **widens** out of cache (10⁷ entries: 217.8 ns vs 35.1 ns).

## Implementation Checklist

- [ ] Size known or boundable? `with_capacity`
- [ ] Removing in a loop? `retain` / `drain` / `swap_remove`
- [ ] Order actually required, or inherited by accident?
- [ ] Long-lived reused buffer? shrink on a threshold
- [ ] Done growing? `into_boxed_slice`
- [ ] `dedup` preceded by a sort (or use a `HashSet` pass)

## Common Bugs

| Bug | Symptom |
| --- | --- |
| `remove(0)` loop | Appears hung at 100k items, fine at 1k |
| No `with_capacity` on a big build | Periodic p99 spikes that grow with n |
| `clear()` on a huge reused buffer | Permanent RSS step; no leak found |
| `dedup` unsorted | Some duplicates silently survive |
| `swap_remove` where order mattered | Downstream output shuffles intermittently |
| `&Vec<T>` parameter | Callers can't pass arrays or sub-slices |

## Key References

- [`Vec` docs — "Guarantees"](https://doc.rust-lang.org/std/vec/struct.Vec.html)
- [Nomicon: Implementing Vec](https://doc.rust-lang.org/nomicon/vec/vec.html)
- std `RawVec::grow_amortized` — the real growth policy
