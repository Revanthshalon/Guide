# Memory Layout — Quick Reference

Core model: a type is a shape stamped into cache lines — size sets entities-per-line, padding is guaranteed-wasted bandwidth, hot-byte fraction sets line efficiency. `repr(Rust)` reorders fields for you (and is unstable); `repr(C)` is manual largest-first. Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| A type dominates the heap/dhat profile (instance count × access freq) | Type lives once on a stack frame — below the noise floor |
| Memory-bound sweep with low line efficiency (big struct, few hot fields) | Compute-bound, or working set fits L1 anyway |
| Enum with a fat rare variant taxes every instance | Shrinking adds conversion churn faster than it removes bytes |
| Preparing for SIMD/SoA sweeps | The "win" requires `packed` (it's a trap, not a tool) |

## Rules of Thumb

- Struct size = fields + padding, rounded to max alignment. `repr(C)`: order largest-first yourself. `repr(Rust)`: free, but never transmute/FFI it.
- Box the fat enum variant (clippy `large_enum_variant`, `result_large_err` on).
- `NonZero*` ids: donate a niche → `Option<Id>` costs nothing extra.
- Shrink where instances are millions, not on principle.
- Hot/cold split when line efficiency is low; full SoA when sweeps dominate.
- Assert `size_of` in tests — the cheapest layout regression gate.
- `packed` → misaligned-reference UB; reorder/shrink instead; wire formats get copies at the edge.
- Immutable-after-build: `Box<str>`/`Box<[T]>` (16 B) over `String`/`Vec` (24 B).

## Numbers to Remember (64-bit)

| Thing | Size |
| --- | --- |
| `&T`, `Box<T>`, `Option<Box<T>>` | 8 B (niche) |
| `&[T]`, `&str`, `&dyn Trait` | 16 B (fat pointer) |
| `String`, `Vec<T>` handle | 24 B + heap block |
| `Arc<T>` | 8 B handle + 16 B counts by the data |
| Arena index | 4 B (`u32`) — half a pointer |
| `bool` | 1 B, but two placed badly = padding twice |
| Alignments | `u32`:4 `u64`/ptr:8 SIMD:16/32/64; line 64 B (128 M-series) |

## Tools

| Question | Tool |
| --- | --- |
| What is this type's real layout? | `-Zprint-type-sizes` (padding + niches shown), `top-type-sizes` |
| Did someone fatten the struct? | `assert_eq!(size_of::<T>(), N)` in tests |
| Which types dominate the heap? | `dhat` (also shows allocator size-class rounding) |
| Did the niche fire? | `size_of::<Option<T>>() == size_of::<T>()` |
| Safe reinterpret-cast | `bytemuck` on `repr(C)` + `Pod` |
| SoA generation | `soa_derive` / `soa-rs` |

## Benchmark Checklist

- [ ] Size asserted before/after; `-Zprint-type-sizes` diffed
- [ ] Win measured at collection scale (size sweep, v0 vs v1 same axes)
- [ ] Counters confirm mechanism: LLC-misses/entity down at same instructions
- [ ] `black_box` on field reads — dead fields fake wins
- [ ] Allocator size-class quantization checked (dhat requested vs actual)

## Key References

- Rustonomicon, ["Data Layout"](https://doc.rust-lang.org/nomicon/data.html).
- Nethercote, [perf-book "Type Sizes"](https://nnethercote.github.io/perf-book/type-sizes.html).
- Drepper §6 for the C-era craft.
