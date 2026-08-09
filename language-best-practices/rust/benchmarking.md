# Rust — Benchmarking

## What This Is For

Testing asks "is it correct?" — a yes/no question with a stable answer. Benchmarking asks "how fast is it?", which is a *distribution* under conditions you only partly control, and where **the optimizer is actively working against you**.

That last point is the one that makes Rust benchmarking distinctive. LLVM will happily delete the code you're trying to measure, because from its point of view a computation whose result is unused is dead code, and it is correct to remove it. Measured on this machine:

```
discarded result          :       0.04 ns/iter     ← the computation was deleted
accumulated, transparent  :    5020.88 ns/iter
black_box in AND out      :    7345.71 ns/iter
```

The first row claims a 10,000-element sum-of-squares took 0.04 nanoseconds — about a **tenth of one clock cycle**. That's not a fast loop; it's an empty one. Recognizing physically-impossible results is the primary benchmarking skill, and the reflex to build is: *convert the number into work-per-unit-time and ask whether the hardware could do that.*

The other half of the discipline is knowing what a benchmark can and cannot tell you. A microbenchmark measures one function under ideal conditions — warm cache, trained branch predictor, no competing working set. Those are privileges the function will never have in situ, which is why a 5× microbenchmark win routinely fails to move the end-to-end number. **Micro results compare variants of one function; only a macro measurement claims system impact.** See [profiling & measurement](../../performance-optimization/profiling-and-measurement/learning.md) for the full funnel.

## The Decisions

| Decision | Guidance |
| --- | --- |
| Which harness? | `criterion` by default (statistics, regression detection); `divan` if you want lighter and faster to write; `iai-callgrind` for CI gates |
| Wall time or instruction counts? | Wall time for real answers; **instruction counts for CI**, because they're immune to machine noise |
| `black_box` where? | Around **inputs** always; around outputs only if your harness doesn't do it (criterion's `iter` does) |
| One size or a sweep? | **Always a sweep.** One data point hides the curve, and the curve is the finding |
| Micro or macro? | Micro to iterate, macro to conclude. Never ship a claim from micro alone |
| Debug or release? | Release, always — debug is **13× slower** and optimizes nothing |

## Setup

```toml
[dev-dependencies]
criterion = { version = "0.8", features = ["html_reports"] }

[[bench]]
name = "parse"
harness = false        # REQUIRED: disables libtest's harness so criterion's main() runs

[profile.bench]
# Inherits `release`. Add symbols so you can profile the benchmark binary itself:
debug = true           # costs binary size, not speed
```

`harness = false` is not optional and is the most common setup mistake — without it, cargo runs the built-in test harness, your `criterion_main!` never executes, and the bench appears to do nothing.

## The Workflow

```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;

fn bench_parse(c: &mut Criterion) {
    // 1. A SWEEP, not a point — this is what reveals the complexity curve.
    let mut group = c.benchmark_group("parse");
    for size in [64usize, 1_024, 16_384, 262_144] {
        let input = make_input(size);
        group.throughput(Throughput::Bytes(input.len() as u64));   // reports MiB/s, not just ns
        group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, input| {
            b.iter(|| parse(black_box(input)))                     // black_box the INPUT
        });
    }
    group.finish();
}

fn bench_with_setup(c: &mut Criterion) {
    // 2. Per-iteration setup that must NOT be timed (e.g. you mutate the input).
    c.bench_function("sort_1k", |b| {
        b.iter_batched(
            || make_random_vec(1_000),        // setup — excluded from the measurement
            |mut v| { v.sort_unstable(); v }, // measured
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_parse, bench_with_setup);
criterion_main!(benches);
```

```sh
cargo bench                          # run all
cargo bench -- parse/1024            # filter by name
cargo bench -- --save-baseline main  # record a baseline
cargo bench -- --baseline main       # compare against it — this is the regression workflow
```

Criterion prints a change estimate against the previous run and flags whether it's statistically significant, which is the feature that makes it worth its weight: it distinguishes "3% faster" from "3% of noise."

## Measured Effects

### `black_box` — what it actually buys, and what it costs

Hand-rolled timing loop, `sum_squares` over 10,000 `u64`, release build, this machine:

| Variant | ns/iter | What happened |
| --- | --- | --- |
| Result discarded | **0.04** | Entire computation deleted as dead code |
| Result accumulated, input transparent | 5020.88 | Real work, but LLVM can hoist and vectorize across iterations |
| `black_box` on input **and** output | 7345.71 | Optimizer barrier at both ends |

Two conclusions, and the second is the one that isn't in the folklore:

1. **Discarding the result deletes the work.** 0.04 ns/iter is the signature. Any benchmark reporting sub-nanosecond time for non-trivial work has been optimized away.
2. **`black_box` is not free and not automatically "more correct."** It cost **46%** here (5020 → 7345 ns) by forcing the input pointer to be re-read each iteration and blocking optimizations across the loop. Some of those optimizations are ones your *real* caller would also get. So `black_box` doesn't reveal the true cost — it pins the measurement to a specific, pessimistic assumption about what the compiler knows. Use it deliberately: around inputs, to stop constant-folding; not scattered everywhere in the hope of accuracy.

### Criterion already black-boxes the output

Running the same function through criterion three ways — result discarded, result returned without `black_box`, and fully `black_box`ed:

```
no_blackbox_result      time:   [3.2075 µs 3.2092 µs 3.2108 µs]
no_blackbox_input       time:   [3.2116 µs 3.2272 µs 3.2503 µs]
correct                 time:   [3.2075 µs 3.2092 µs 3.2110 µs]
```

**All three identical.** Criterion's `b.iter()` already applies a `black_box` to the closure's return value, so the "always wrap your result in `black_box`" advice is redundant *when using criterion*. It remains essential for hand-rolled `Instant::now()` loops, which is exactly where the 0.04 ns result came from. Know which your harness does rather than cargo-culting the wrapper.

### Debug builds measure nothing

`cargo bench` uses the `bench` profile (inheriting `release`) — but a hand-rolled benchmark run under `cargo run` or `cargo test` does not. Debug is **13× slower** on a realistic workload ([releasing](releasing.md)), and more importantly it's a different program: no inlining, no vectorization, every abstraction paid for. A debug benchmark ranks implementations differently from the code you ship.

## Pitfalls

### Pitfall: The optimizer deleted your benchmark

- **What goes wrong:** A benchmark reports a suspiciously wonderful number — sub-nanosecond, or unchanged when you double the input size. The optimization being "measured" looks like a triumph and is a measurement of nothing. Worst case it's believed and shipped, and the real system doesn't move.
- **Why it happens (the mechanism):** LLVM removes computations whose results are unused (dead code elimination) and hoists computations whose inputs don't change out of loops (loop-invariant code motion). A benchmark loop is the ideal target for both: the result is usually discarded, and the input is usually a constant the optimizer can see. Measured: 0.04 ns/iter for a 10,000-element reduction.
- **How to handle it, and why that works:** Sanity-check every result against physics before believing it — convert to work per second and compare against memory bandwidth (~10 GB/s sequential) and clock rate. Then make the input opaque with `black_box`, and ensure the result is consumed (criterion does this for you; a hand-rolled loop does not). Vary the input size and confirm the time scales the way the algorithm says it should — a flat line across 10× input size is the same signal.
- **Trade-offs of the fix:** `black_box` costs a measured 46% here by blocking optimizations your real caller might legitimately get, so an over-black_boxed benchmark is pessimistic rather than accurate. There is no perfect answer — the honest position is that a microbenchmark measures the function under *specific* assumptions about compiler knowledge, and you should know which assumptions you've pinned.

### Pitfall: One input size

- **What goes wrong:** A benchmark runs at n = 1,000 and reports 3 µs. The optimization is accepted. In production n is 2 million, the data no longer fits in L2, and the "faster" implementation is slower — because the win came from cache residency the real workload doesn't have.
- **Why it happens (the mechanism):** A single point cannot show a curve, and the interesting behaviour of nearly every data structure is *where the curve bends* — at L1, L2, and LLC boundaries, at allocator size-class thresholds, at the point an algorithm switches strategy. This session's own measurements are full of these: `HashSet` overtakes a linear scan at n ≈ 12; the stable sort has a *pessimal* middle ground at 1,000 perturbations; binary search flattens completely between n = 32 and n = 4096.
- **How to handle it, and why that works:** Always sweep — powers of two spanning at least three orders of magnitude, using criterion's `bench_with_input` and `BenchmarkId`. Add `Throughput` so results come out as bytes/s or elements/s, which makes non-linearity obvious at a glance. Then read the *shape*, not the numbers.
- **Trade-offs of the fix:** A sweep multiplies benchmark runtime, which matters when it's in CI. Mitigate by keeping a small sweep in CI for regression detection and running the full sweep on demand when investigating.

### Pitfall: Trusting a small delta on a noisy machine

- **What goes wrong:** A change shows 4% improvement. It's merged. It was noise — or worse, it was code-layout luck, where adding an unrelated function shifts alignment and changes the number by more than your optimization did.
- **Why it happens (the mechanism):** A developer laptop is a hostile measurement environment: thermal throttling on long runs, frequency scaling, background processes, and on Apple Silicon the scheduler migrating threads between P-cores and E-cores that differ by roughly 2–3× in single-thread speed. On top of that, instruction and data alignment can swing small benchmarks several percent for reasons unrelated to the change.
- **How to handle it, and why that works:** Establish your noise floor first — run the *same* benchmark ten times across an hour of ordinary use and look at the spread. Any delta smaller than that spread is not a result. Use criterion's confidence intervals and its significance flag rather than comparing point estimates. For CI, use `iai-callgrind`, which counts **instructions** via Callgrind rather than measuring time: deterministic, noise-free, and therefore able to gate on a 1% change that wall-clock could never resolve.
- **Trade-offs of the fix:** Instruction counts are not time — they're blind to cache misses, branch mispredictions, and memory-level parallelism, so an optimization that improves locality can look neutral or worse. Use them as a *regression tripwire*, not as the measure of an optimization. And a noise-floor run costs an hour you have to actually spend.

### Pitfall: The microbenchmark mirage

- **What goes wrong:** A function is 5× faster in criterion. It ships. End-to-end latency doesn't move at all, and nobody can explain why.
- **Why it happens (the mechanism):** The microbenchmark grants privileges the real workload doesn't: the input is hot in L1 because it's reused every iteration, the branch predictor is perfectly trained by thousands of identical iterations, and there's no competing working set evicting your data. In situ, the same function runs cold, with the cache full of other work. Amdahl compounds it: a function that is 8% of runtime caps the total win at 8% no matter how much faster it gets.
- **How to handle it, and why that works:** Apply Amdahl *before* optimizing — get the function's share from a profile and compute the ceiling; if it's 8%, decide whether an 8% ceiling justifies the work. Then close the loop: every accepted micro win must reproduce in an end-to-end measurement before the task is done. That single rule eliminates most decorative optimization.
- **Trade-offs of the fix:** End-to-end benchmarks are slower, noisier, and harder to attribute — you learn the total moved without learning why. The practical pattern is micro for fast iteration and attribution, macro for the verdict, and never letting micro alone justify a claim.

### Pitfall: Benchmarking setup instead of the operation

- **What goes wrong:** A sort benchmark times `v.clone()` plus the sort, and the clone dominates. A parse benchmark includes reading the file. The measured difference between two implementations is compressed toward zero because most of the measured time is shared overhead — so a genuine 2× improvement shows as 15%.
- **Why it happens (the mechanism):** Many operations are destructive (sorting mutates, parsers consume), so each iteration needs fresh input, and the obvious way to get it is to build the input inside the timed closure. Criterion's plain `iter()` times everything in the closure.
- **How to handle it, and why that works:** Use `iter_batched` with a setup closure — criterion runs setup outside the timed region and times only the routine. Where setup is expensive relative to the operation, `BatchSize::SmallInput`/`LargeInput` control how many iterations share one setup. For non-destructive operations, build the input once outside the closure and just borrow it.
- **Trade-offs of the fix:** `iter_batched` allocates setup for batches of iterations, which itself perturbs the allocator and cache state, so it isn't free either. And if you subtract too much you can measure something unrealistically favourable — a sort that never pays for cold input. Measure the setup separately so you know how much you excluded.

## Checklist

- [ ] `harness = false` under `[[bench]]`
- [ ] Result is physically plausible — converted to work/second and sanity-checked
- [ ] Input passed through `black_box`; output consumed (criterion does this)
- [ ] Sweep across ≥3 orders of magnitude, not a single size
- [ ] `Throughput` set so the report shows bytes/s or elements/s
- [ ] Destructive operations use `iter_batched`
- [ ] Noise floor measured once, and no delta smaller than it is believed
- [ ] Baseline saved (`--save-baseline`) and compared (`--baseline`)
- [ ] `[profile.bench] debug = true` so the bench binary can be profiled
- [ ] Machine quiet: not on battery, browser/IDE-indexer closed
- [ ] CI gate uses instruction counts (`iai-callgrind`), not wall time
- [ ] Every accepted micro win re-verified end to end

## Tooling

| Need | Tool | Notes |
| --- | --- | --- |
| Statistical microbenchmarks | `criterion` | Confidence intervals, baselines, HTML reports; `iter()` black-boxes output |
| Lighter/faster to write | `divan` | Less ceremony, attribute-based; fewer statistics |
| Noise-immune CI gates | `iai-callgrind` | Counts instructions via Callgrind — deterministic, but blind to cache effects |
| Whole-binary timing | `hyperfine` | For CLIs; handles warmup and comparison |
| Where the time goes | `samply`, `cargo-flamegraph` | `samply` works on macOS and Linux |
| Hardware counters | `perf stat`, `cachegrind` | IPC and miss rates; `perf` is Linux-only |
| Allocation profiling | `dhat`, `heaptrack` | Allocation pressure is the most common Rust finding |
| Async task introspection | `tokio-console` | Off-CPU time that a sampling profiler can't see |
| Compile-time analysis | `cargo build --timings` | When the *build* is what's slow |

## Open Questions

- `divan` vs `criterion` on the same benchmarks — how much do the numbers differ, and is divan's lighter statistics a practical problem?
- What is this machine's actual noise floor for a criterion benchmark across an hour of normal use? Needed before trusting any delta under ~5%.
- How much do P-core/E-core migrations pollute results on Apple Silicon, and does pinning with `taskpolicy` measurably tighten the distribution?
- `iai-callgrind` in CI: what instruction-count threshold produces zero false positives over a month of commits?
- The `black_box` cost measured here was 46% for a memory-bound reduction. Is it that large for compute-bound code, or is this specific to forcing pointer reloads?

## References

- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) — the statistics chapter doubles as a benchmarking-methodology course; the baseline/comparison workflow is the part to internalize.
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/) — Rust-specific build configuration and profiling setup; short and dense.
- Emery Berger, "Performance Matters" (StrangeLoop) — layout luck and causal profiling; the best vaccination against trusting small deltas.
- [`std::hint::black_box` docs](https://doc.rust-lang.org/std/hint/fn.black_box.html) — note the explicit caveat that it is best-effort and not a guarantee.
- [`iai-callgrind`](https://github.com/iai-callgrind/iai-callgrind) — instruction-count benchmarking for CI.
- Related in this repo: [Profiling & Measurement](../../performance-optimization/profiling-and-measurement/learning.md) (the funnel this sits inside, and the micro/macro discipline), [releasing](releasing.md) (the profiles, and the 13× debug/release number), [testing](testing.md) (why correctness and measurement need different tools), [Complexity Analysis](../../data-structures-and-algorithms/complexity-analysis/learning.md) (the doubling experiment — a sweep that recovers the exponent).
