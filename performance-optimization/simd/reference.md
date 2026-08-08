# SIMD — Quick Reference

Core model: one instruction, 4–64 lanes — divides instruction count by lane width, so it pays exactly where instructions were the bottleneck (compute-bound, L1/L2-resident; memory-bound loops gain ~nothing). Prerequisites: data-parallel shape, contiguous SoA data, branches→masks. Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| Data-parallel map/filter/reduce over contiguous primitives | Loop already at bandwidth ceiling — fix working set/layout first |
| Compute-dense, cache-resident kernels | Gather-shaped access (scattered lanes) |
| Data-random conditions (masks are entropy-immune) | Shuffle/horizontal-dominated algorithms (often AoS's fault) |
| Byte/parse/scan work (u8 = 16–64 lanes) | Small N — setup + tail dominate; keep a scalar path |

## The Routes (escalation order)

| Route | What | Fragility / ceiling |
| --- | --- | --- |
| 0. A crate did it | `memchr`, `bytecount`, `simdutf8`, `simd-json` | Check first; losing to memchr is the standard lesson |
| 1. Autovectorization | Shape the loop, verify assembly | Free but silently reversible — no marker, no test failure |
| 2. Portable SIMD | `std::simd` (nightly) / `wide` (stable): `f32x8`, masks, `select` | **Right default** — explicit, safe, cross-ISA |
| 3. Intrinsics | `core::arch` + `#[target_feature]` + runtime dispatch (`multiversion`) | Last 20%; unsafe, per-ISA, price the maintenance |

## Rules of Thumb

- Stay vertical: per-lane accumulators, one horizontal fold at the end.
- Branches → compare-to-mask → `select`/arithmetic; SIMD is mandatory branchlessness.
- SoA flat primitives are the prerequisite (`Vec3` → three `Vec<f32>` for sweeps).
- Smaller lanes = double dividend: more lanes/register + less bandwidth (u8 over u32).
- `chunks_exact` + `remainder()` — the tail is real code with real bugs.
- `f32` reductions don't autovectorize (associativity); multiple accumulators or explicit SIMD — and rounding order changes: a semantics sign-off.
- Bounds check inside the body kills autovectorization — hoist (`&data[..n]`) or use iterators.
- `target-cpu=native` for local benches only; ship runtime dispatch.
- Verify assembly before believing any SIMD benchmark (`cargo asm`, godbolt: packed regs).
- Load-bearing vectorized loops get an iai instruction-count gate (catches silent-scalar regressions).

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Register widths | 128 (SSE/NEON, all Apple M) / 256 (AVX2) / 512 (AVX-512) |
| u8 lanes per 128/256/512 | 16 / 32 / 64 |
| M-series | 128-bit NEON × 4 pipes — throughput rivals AVX2 |
| Byte-threshold kernel, L2-resident | ~15× over branchless scalar |
| Same kernel, DRAM-resident | ~2–4× — the roofline observed |
| Unaligned loads within a line | ~free on modern cores |

## Diagnostic Signatures

| Signature | Meaning | Action |
| --- | --- | --- |
| Packed instructions absent in asm | Autovec didn't fire | Hoist checks, remove branches/calls, or route 2 |
| Instructions ÷ lane-width, time barely moves | Memory-bound | Cache/layout docs first |
| Fast at 1 MB, flat at 1 GB | Residency-dependent multiplier | Report both; fix working set |
| Win vanished after refactor | Silent-scalar regression | iai gate; move to explicit SIMD |

## Benchmark Checklist

- [ ] Assembly verified vectorized before timing
- [ ] GB/s reported against measured bandwidth ceiling (roofline check)
- [ ] Working-set sweep: multiplier at L1 / L3 / DRAM sizes
- [ ] N-sweep includes small and odd sizes (tail path exercised)
- [ ] Dispatch overhead measured through the real call pattern

## Key References

- [`std::simd` docs](https://doc.rust-lang.org/std/simd/index.html) / `wide` crate.
- Agner Fog instruction tables — when a kernel underperforms its math.
- Lemire's blog + simdjson paper; `memchr` source as route-3 craft.
