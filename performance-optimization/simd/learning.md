# SIMD — Learning Notes

## The Hardware Mechanism

Beside the scalar ALUs, every modern core has **vector units**: wide registers and execution pipes that apply *one instruction to multiple data lanes at once* — Single Instruction, Multiple Data. The widths that matter:

| ISA | Register width | f32 lanes | u8 lanes | Where |
| --- | --- | --- | --- | --- |
| SSE2/NEON | 128-bit | 4 | 16 | Baseline x86-64; **all ARM incl. Apple M-series** |
| AVX2 | 256-bit | 8 | 32 | Near-universal x86 since ~2015 |
| AVX-512 | 512-bit | 16 | 64 | Server x86, recent AMD; adds first-class masks |

Apple Silicon note: M-series is NEON-only (128-bit) but with **four vector pipes** — so peak vector throughput per cycle rivals AVX2 machines; width isn't the whole story, *issue width × register width* is.

The economics: a vector add costs roughly the same latency and throughput as a scalar add — **one instruction does 4–16× the work at ~1× the cost**. That's the entire pitch. The catches, each of which shapes the discipline:

- **Lanes are isolated verticals.** Lane-wise ("vertical") ops — add, multiply, compare, min/max, bitwise — are the cheap ones. **Horizontal** ops (sum *across* lanes, find the max lane) are expensive multi-instruction shuffles; the idiom is to stay vertical as long as possible (per-lane accumulators) and go horizontal exactly once, at the end.
- **Branches don't exist; masks do.** A vector compare produces a *mask* (per-lane all-ones/all-zeros); conditional behavior is `select(mask, a, b)` — both sides computed, per-lane choice. SIMD is the [branchless doctrine](../branch-prediction/learning.md) made mandatory, and the payoff compounds: a data-random condition that cost 8 cycles/element as a branch costs a fraction of a cycle as a mask.
- **Data must be contiguous.** Vector loads want adjacent memory (one line, one instruction). **Gather/scatter** (lanes from scattered addresses) exists but is slow enough to void the exercise on most cores — which is why [SoA layout](../data-oriented-design/learning.md) is SIMD's prerequisite: "one field, adjacent across entities" is exactly a vector load. (Alignment, the historical bugbear, is mostly relaxed on modern cores — unaligned loads within a line are ~free; crossing lines costs a little.)
- **The remainder exists.** N elements ÷ 8 lanes leaves a tail; every SIMD loop has an epilogue (scalar tail, masked final iteration, or overlapping last vector). Boilerplate, and a real cost at small N.

## Mental Model

**SIMD divides the instruction count by the lane width — so it pays exactly where instructions were the bottleneck.** The two-question qualifier for any loop:

1. **Is it data-parallel?** Same operation per element, no cross-element dependencies (or dependencies restructurable into per-lane accumulators). Map/filter/reduce shapes: yes. Pointer-chasing, early-exit searches with rare hits, stateful parsing: mostly no.
2. **Is it compute-bound?** The [roofline logic](../cache-locality/learning.md): if the loop already saturates memory bandwidth, dividing instructions by 8 changes nothing — the lanes just wait faster. Check GB/s against the machine's ceiling *before* vectorizing; SIMD's multiplier is largest for L1/L2-resident data and compute-dense work, and shrinks toward 1× as the working set falls out of cache. (Corollary: [shrinking the data](../memory-layout/learning.md) — u8 over u32 — *is* a SIMD optimization: more lanes per register *and* less bandwidth.)

Given a qualifying loop, Rust offers **three routes, in escalation order**:

- **Route 1 — autovectorization (free, fragile).** LLVM vectorizes suitable loops at `opt-level=3` automatically. Your job is to write *vectorizable shapes* — slices with known bounds, no branches in the body (use `min`/`max`/arithmetic-on-bools), simple indexing or iterators, `chunks_exact` — and **verify in the assembly** that it happened (`cargo asm`, godbolt: look for `vpadd`/`vfmadd`/`fmla` on packed registers). Fragile because invisible: an innocent edit (a bounds check, a function call, an early return) silently drops the loop back to scalar, and nothing tells you. The counter-intuitive classic: **a plain `f32` sum does not autovectorize** — float addition isn't associative, and the compiler won't reorder it without permission; the fix is explicit multiple accumulators (restructuring the reduction yourself) or route 2.
- **Route 2 — portable SIMD (explicit, safe).** `std::simd` (nightly) or the `wide` crate (stable): `f32x8` types with lane-wise ops, masks, and `select`, compiled to the best available instructions per target. Explicit — the vectorization can't silently vanish — portable across x86/ARM, and safe Rust. **The right default for deliberate SIMD.**
- **Route 3 — intrinsics (maximal, costly).** `core::arch` per-ISA intrinsics (`_mm256_*`, NEON `v*q_*`) for the last 20%: exotic shuffles, `movemask` tricks, instructions portable SIMD can't express. `unsafe`, per-platform code, real maintenance weight. Pair with **runtime feature detection** (`is_x86_feature_detected!` + `#[target_feature(enable=...)]` functions, or the `multiversion` crate) so shipped binaries dispatch to the best ISA without compiling for one machine. (`-C target-cpu=native` is for local benchmarking, not for shipping.)

And **route 0, checked first: someone already did it.** `memchr`, `bytecount`, `simdutf8`, `simd-json`, the `hashbrown` probe loop, `regex`'s literal search — the ecosystem's hot primitives are hand-vectorized by people who read Agner Fog for fun. A hand-rolled byte scan losing to `memchr` by 3× is the standard first SIMD lesson; import before writing.

## Worked Example

Count bytes ≥ 128 in 1 MB of random bytes — the [branch doc's](../branch-prediction/learning.md) four-cell benchmark, completed with its missing row. Illustrative numbers (M-series/AVX2-class; reproducing is exercise one):

```rust
// A. Branchy scalar                          ~6.0 ns/elem shuffled, ~1.0 sorted
if x >= 128 { count += 1 }

// B. Branchless scalar                       ~1.3 ns/elem, entropy-immune
count += (x >= 128) as u64;

// C. SIMD (std::simd), 16-lane u8            ~0.09 ns/elem, entropy-immune
use std::simd::{u8x16, cmp::SimdPartialOrd};
let threshold = u8x16::splat(128);
let mut acc = u8x16::splat(0);
for chunk in data.chunks_exact(16) {
    let v = u8x16::from_slice(chunk);
    let mask = v.simd_ge(threshold);           // per-lane compare → mask
    acc -= mask.to_int().cast::<u8>();         // vertical: accumulate per lane (all-ones = -1)
    // (flush acc to a wider counter every 255 iterations — u8 lanes saturate)
}
let count: u64 = horizontal_sum(acc) + scalar_tail(data.chunks_exact(16).remainder());
```

```
                shuffled     sorted
A. branchy      ~6.0         ~1.0     ns/elem
B. branchless   ~1.3         ~1.3
C. SIMD         ~0.09        ~0.09    ← ~15× over B, ~70× over A-shuffled
```

The teaching points, in order: **C is entropy-immune like B** — masks are branchless by construction — but divides B's instruction count by 16. **The accumulator is vertical**: 16 per-lane counters, summed across lanes *once* at the end (the horizontal-ops-are-expensive rule shaping the code); the u8-saturation flush is the kind of correctness detail SIMD makes you own. **The remainder is explicit** (`chunks_exact` + tail). And the honest caveat: at 1 MB the data is L2-resident and the loop is compute-light — run the same code DRAM-resident at 1 GB and the multiplier compresses toward the bandwidth ceiling (~2–4×, not 15×): *the roofline, observed*. Both numbers are true; which one you get is decided by the working set, which is why the [cache doc](../cache-locality/learning.md) precedes this one.

Second vignette, one line, for the float-reduction trap: `data.iter().sum::<f32>()` runs scalar (associativity); four manual accumulators (`sum0 += chunk[0]; sum1 += chunk[1]; …` then fold) or `f32x8` reduction unlocks 4–8× — *and changes the rounding order*, which is a semantics decision you sign off on, not an optimization detail.

## Applying It

- **Sequence the routes:** check route 0 (crate exists?) → shape the loop and check autovectorization → `std::simd`/`wide` when it must be deliberate → intrinsics when profiling proves the portable version leaves real margin. Most code should stop at route 1 or 2.
- **Write vectorizable shapes for route 1:** iterate slices (`chunks_exact` beats manual indexing), hoist bounds checks (`let d = &data[..n]` before the loop, or iterators — a bounds check *inside* the body kills the transform), no calls/branches/early-exits in the body, masks-as-arithmetic for conditions. Then **verify**: the [profiling doc's](../profiling-and-measurement/learning.md) check-the-assembly habit is load-bearing here — autovectorization is the optimization most often *believed present and actually absent*.
- **Layout is the prerequisite:** SoA arrays of flat primitives ([DoD stage 3](../data-oriented-design/learning.md)); `Vec3` as three `Vec<f32>` beats `Vec<Vec3>` for vector math sweeps; smaller lane types = more lanes (the u8-over-u32 dividend).
- **Reductions:** per-lane accumulators, horizontal fold once at the end; multiple *independent* accumulators even in scalar code (breaks the dependency chain — the same trick serves ILP); watch lane-width saturation (the u8 flush).
- **Dispatch for shipping:** `multiversion` (or hand-rolled `is_x86_feature_detected!` + `#[target_feature]`) compiles the hot function per-ISA and picks at runtime — AVX2 on machines that have it, SSE2 baseline elsewhere, NEON on ARM, one binary. `target-cpu=native` only for your own-machine benchmarks.
- **Masks replace filters end-to-end:** compare → mask → select/compress; `filter().sum()` becomes mask-accumulate ([the branch doc's](../branch-prediction/learning.md) mask spelling, now 16 lanes wide). For "collect the matching elements," masked compaction (`compress`-style) exists but is where intrinsics often enter — consider whether a count/sum/bitmap serves instead.
- **Budget the tail:** small-N calls (< a few vectors' worth) are overhead-dominated — provide a scalar path or accept it; mid-loop, `chunks_exact` + `remainder()` is the clean idiom.

## When It Hurts

- **Memory-bound loops: the multiplier evaporates.** Bandwidth-saturated sweeps gain ~nothing from wider math — check GB/s first ([roofline](../cache-locality/learning.md)); fix the working set or the layout before the instruction count.
- **Horizontal/shuffle-heavy algorithms:** if the inner loop is mostly cross-lane rearrangement, the vector units are fighting the algorithm, not the data — AoS math without layout change lands here (that's the *layout's* fault; fix it first).
- **Gather-shaped access:** lanes needing scattered addresses (hash probes, indexed lookups) mostly void SIMD on current cores; restructure to sorted/batched access or don't vectorize.
- **Float semantics:** vectorized reductions reorder additions — different rounding, different result, sometimes different *tests*. It's usually fine and *always* a decision; document it where numerical code has stakeholders.
- **Intrinsics debt:** per-ISA `unsafe` code that two people can review is a real liability against a 15% win over `std::simd` — price the maintenance, not just the speedup. Route 3 wants a benchmark gate *and* a comment explaining why route 2 wasn't enough.
- **The silent-scalar regression:** autovectorized code has no marker; refactors drop it to scalar without a test failing. For load-bearing loops, either move to route 2 (explicit types can't silently unvectorize) or pin an instruction-count/throughput gate ([iai](../profiling-and-measurement/learning.md)) that fails when the transform vanishes.

## Benchmarking Methodology

- **Verify the code shape before timing it:** assembly first (packed instructions present? loop actually vectorized or just unrolled?); otherwise the benchmark measures a belief.
- **Report GB/s next to ns/elem** and compare against the machine's bandwidth ceiling (measure it once with a plain memcpy/sum sweep — it's the roofline's flat top; results within ~80% of it mean memory-bound, and the SIMD comparison is void).
- **Sweep the working set across cache levels** ([the staircase](../cache-locality/learning.md)): the SIMD multiplier is a *function of residency* — report it at L1, L2/L3, and DRAM sizes, not one flattering point.
- **Instruction counts read the mechanism:** iai/`instructions` should drop by ~lane-width when vectorization engages; time-vs-instructions divergence localizes whether the win was width or something else (unrolling, bounds-check elision) riding along.
- **Include the tail and small-N:** benchmark at N = 10, 1 000, 1 000 000 — the crossover where SIMD starts paying is a number to know per kernel; odd sizes exercise the remainder path (and its bugs).
- **Dispatch overhead:** if using runtime detection, benchmark through the dispatch (function-pointer call per invocation vs. per-batch) — per-element dispatch can eat the win.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Why are horizontal operations expensive while vertical ones are cheap, and how does that asymmetry shape every SIMD reduction?
2. Why doesn't `iter().sum::<f32>()` autovectorize while the same over `u32` does? Name both fixes and what each signs you up for.
3. Complete the four-cell → five-row story: why is the SIMD row entropy-immune, and what did it inherit from the branchless doctrine?
4. A 16-lane byte kernel shows 15× at 1 MB and 2.5× at 1 GB. Explain with the roofline; which doc's fix applies at 1 GB?
5. Rank the three routes by fragility and by ceiling; state the rule for when route 3 is justified.
6. What is the silent-scalar regression, why does no test catch it by default, and what are the two structural defenses?
7. Why is `-C target-cpu=native` wrong for shipped binaries, and what replaces it?

Measurement exercises:

- Reproduce the five-row table (A/B/C × shuffled/sorted collapses to three distinct behaviors) at 1 MB *and* 256 MB; compute the SIMD multiplier at each size and mark your measured bandwidth ceiling on the plot. This one exercise touches four docs' mechanisms.
- Take an `f32` dot product: measure `iter().zip().map().sum()`, then 4-accumulator manual, then `wide`/`std::simd` — verify the first is scalar in the assembly, and explain the gap between the last two (if any) via instruction counts.
- Race `memchr::memchr` against your best hand-rolled byte finder at three sizes; read memchr's source afterward to see what route-3 craft looks like. Losing is the lesson.

## Open Questions

- `std::simd` stabilization status and API drift — re-check; `wide` as the stable bridge meanwhile.
- M-series specifics: measure the four-pipe NEON claim — does a 128-bit `u8x16` kernel hit 4 ops/cycle, and where does it sit vs. AVX2 on the same kernel normalized per GHz?
- Masked tail handling (`std::simd` masked loads) vs. scalar tail vs. overlapping-last-vector: measurable difference at awkward N?
- `multiversion` dispatch cost and code-size effect on a real multi-kernel binary.
- Where does auto-vectorization in current rustc/LLVM actually give up: build the small corpus (early exit, bounds check, call, f32 reduce, stride) and check yearly — the fragility list should be *measured*, not folklore.

## References

- [`std::simd` (portable SIMD) docs](https://doc.rust-lang.org/std/simd/index.html) + the [Rust portable-SIMD guide](https://github.com/rust-lang/portable-simd) — route 2's home; the mask/select vocabulary.
- Agner Fog, [instruction tables & optimization manuals](https://www.agner.org/optimize/) — latencies/throughputs per instruction per core; the reference when a kernel underperforms its paper math.
- Daniel Lemire's blog + [simdjson paper](https://arxiv.org/abs/1902.08318) — what industrial-strength SIMD design looks like (masks, movemask tricks, branchless parsing at GB/s).
- [memchr](https://docs.rs/memchr) / [simdutf8](https://docs.rs/simdutf8) source — readable route-3 craft in Rust, with dispatch done right.
- ARM NEON Programmer's Guide — the intrinsic vocabulary for the machine on your desk.
- Related topics in this repo: [Branch Prediction](../branch-prediction/learning.md) (masks are its doctrine, mandatory), [Cache Locality](../cache-locality/learning.md) (the roofline that caps the multiplier), [Memory Layout](../memory-layout/learning.md) (lane count dividends), [Data-Oriented Design](../data-oriented-design/learning.md) (SoA as prerequisite — this doc is its stage 4), [Profiling & Measurement](../profiling-and-measurement/learning.md) (verify-the-assembly, iai gates).
