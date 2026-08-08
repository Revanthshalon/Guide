# Profiling & Measurement — Learning Notes

## The Hardware Mechanism

Everything in performance measurement rides on three machine facilities — knowing them tells you what each tool can and cannot see.

**The timestamp counter (TSC).** The CPU increments a 64-bit counter every cycle (`rdtsc` on x86; `cntvct_el0` on ARM/Apple Silicon), readable in ~10–30 cycles. Every "how long did this take?" ultimately bottoms out here (via `Instant::now()` → `clock_gettime` → TSC on Linux). Two facts matter: reading it is cheap but *not free* — timing a 5 ns operation with a 20 ns clock read measures the clock, not the operation (harnesses batch iterations for exactly this reason); and modern TSCs are "invariant" (tick at a constant rate regardless of frequency scaling), which makes them reliable for *wall time* but means cycles-of-work and nanoseconds diverge as the CPU clocks up and down — one reason repeated runs of identical work vary.

**Performance monitoring counters (PMU).** Beside the TSC, each core has a handful of programmable hardware counters that can count *events*: instructions retired, cache misses per level, branch mispredictions, TLB misses, stalled cycles. `perf stat` programs them and reads totals — this is how you learn *why* code is slow, not just that it is (IPC — instructions per cycle — is the single most diagnostic ratio: ~4 means compute-bound and healthy; ~0.5 means the core is mostly waiting, usually on memory). The PMU can also fire an interrupt every N events — which enables the next mechanism.

**Sampling via interrupts.** A sampling profiler (`perf record`, samply) programs the PMU to interrupt, say, 999 times/second of on-CPU time; each interrupt grabs the program counter and call stack. Over seconds, the histogram of stacks *is* the time distribution — with two built-in blind spots: overhead proportional to sampling rate but independent of code shape (unlike instrumenting profilers, which tax every function call and distort exactly what they measure — the observer effect); and **sampling only sees on-CPU time**. A thread blocked on a lock, disk, or socket takes no samples — it's invisible. That's not a flaw to work around but a categorization to exploit: *on-CPU problems* (computing too much) are found by sampling profilers; *off-CPU problems* (waiting too much) need different instruments — off-CPU profiling, async runtime introspection, or tracing. Asking "is the time on-CPU or off?" is the first fork in every investigation.

## Mental Model

**You are not allowed to have an opinion about performance — only measurements have opinions.** Every optimization in this category's other docs is a hypothesis until this doc's machinery confirms it on your workload, your data shape, your hardware. Programmer intuition about *where* time goes is reliably wrong (the famous hot spot is innocent; the "trivial" serialization step is 40%); the discipline exists because of that unreliability.

The working model is a **funnel of instruments**, coarse to fine, each answering one question:

1. **Is there a problem, and how big?** Macro measurement: end-to-end latency/throughput of the real system under realistic load — production metrics, or a load generator + `hyperfine` for CLIs. This sets the *baseline* — the number every later change is judged against. No baseline, no optimization: without it you cannot distinguish improvement from noise.
2. **Where does the time go?** Profile the macro workload: flamegraph from sampled stacks (on-CPU), off-CPU analysis if the time isn't there. Output: the *one or two* places worth touching — and, via **Amdahl's law**, the ceiling on caring: a function taking 10% of runtime caps your total win at 10% no matter how brilliant the fix. Amdahl is the arithmetic that kills most optimization projects before they waste a week — apply it *before* getting attached.
3. **Why is that code slow?** PMU counters on the hot region (`perf stat`, cachegrind): compute-bound (high IPC — fewer instructions needed: algorithm, SIMD) vs. memory-bound (low IPC, high miss rate: [cache locality](../cache-locality/learning.md), [memory layout](../memory-layout/learning.md)) vs. mispredict-bound ([branch prediction](../branch-prediction/learning.md)). The counter profile *routes you to the right chapter of this repo*.
4. **Did the fix work?** Microbenchmark the isolated change (criterion/divan) for fast iteration — then **re-measure the macro number**, because a microbenchmark win that doesn't move the end-to-end baseline is a decoration, not an optimization (the isolated loop had warm caches and trained branch predictors the real workload doesn't — micro and macro legitimately disagree, and macro is the one that's true).

Two distribution rules complete the model. **Averages lie**: latency is a distribution; the mean hides the p99 tail your unluckiest-and-often-biggest users live in — always look at percentiles, and remember a user touching 100 servers per page experiences roughly the *sum of tails*, which is why tail latency is a systems obsession. **Variance is information**: a benchmark that won't sit still is telling you about frequency scaling, a noisy neighbor, or allocation jitter — treat "why does it vary?" as a finding to explain, not noise to average away.

Where the model breaks down: measurement itself perturbs (observer effect — heavy instrumentation, or even adding a counter, changes code layout and cache behavior); and micro-level effects (alignment of code in memory, link order) can swing small benchmarks ±10% for reasons unrelated to your change — which is why methodology (below) exists as its own discipline.

## Worked Example

A Rust log-analysis CLI feels slow on a 2 GB file. The funnel, applied:

**1. Baseline (macro).**

```sh
hyperfine --warmup 2 './target/release/logscan big.log'
#  Time (mean ± σ):  14.31 s ± 0.12 s    [User: 13.9 s, System: 0.4 s]
```

Two immediate reads: 14.3 s ± 0.12 — tight enough variance to detect a 5% change; and User ≫ System with User ≈ wall time → the time is **on-CPU in our code** (not I/O wait, not the kernel). A sampling profiler will see it.

**2. Where (profile).** Build with debug symbols in release (`[profile.release] debug = true`), then:

```sh
cargo flamegraph -- big.log       # perf record + fold + SVG, one step
```

The flamegraph (hypothetical but typical): 41% `serde_json::from_str`, 22% `String::from_utf8` + allocation inside the line iterator, 18% `regex::Regex::find` (compiled per call!), 11% actual analysis logic. The "obvious" suspect — our clever aggregation algorithm — is 11%. The famous hot spot is innocent, on schedule. Amdahl arithmetic: fixing the regex-per-call bug caps at 18%; killing per-line allocation, ~20%; JSON parsing is the big fish but hardest (it's a library).

**3. Why (counters), for the allocation slice.**

```sh
perf stat -e task-clock,instructions,cycles,cache-misses ./target/release/logscan big.log
#  IPC 1.1, cache-misses high → memory-bound flavor: allocation churn, pointer-chasing
```

Plus `dhat` (heap profiler): 38M allocations — one `String` per line. Diagnosis: allocator traffic, not clever-code deficiency.

**4. Fix and verify.** Hoist the regex (`once_cell::sync::Lazy<Regex>`); switch the parse path to borrow (`&str` slices off a reused buffer, `serde` zero-copy `#[serde(borrow)]` where possible). Microbenchmark the parse function first (criterion: 620 ns → 210 ns per line, CI excludes noise), then the number that counts:

```sh
hyperfine --warmup 2 './target/release/logscan big.log'
#  Time (mean ± σ):  6.02 s ± 0.09 s     → 2.4× end-to-end
```

Macro confirms micro. Commit the `hyperfine` line into a script — that's the **regression baseline** the next change gets judged against. Total investment: ~an hour, most of it reading the flamegraph, none of it guessing.

## Applying It

The Rust toolbox, by question — all assume `--release` with `debug = true` (symbols cost binary size, not speed; profiles without symbols are wallpaper):

- **End-to-end timing:** `hyperfine` (CLIs — handles warmup, statistics, comparison of two binaries out of the box); your metrics stack + load generator (services). This tier owns the baseline.
- **On-CPU profile → flamegraph:** `samply record ./target/release/bin` (works on macOS *and* Linux, browser UI — the right default on your Mac); `cargo flamegraph` or raw `perf record --call-graph dwarf` on Linux. Read flamegraphs by *width* (time), never height (depth); look for wide plateaus you didn't expect.
- **Hardware counters:** `perf stat` first pass (IPC, miss rates), `perf stat -e` for specifics — Linux only; on macOS, Instruments or run inside a Linux container/VM for counter work. `valgrind --tool=cachegrind` simulates cache behavior deterministically — slower but reproducible and cross-platform, good for A/B-ing a layout change.
- **Microbenchmarks:** `criterion` (statistical rigor, regression detection, HTML reports) or `divan` (lighter, faster to write). Non-negotiables: `std::hint::black_box` around inputs *and* outputs (or the optimizer deletes your benchmark — a 0.2 ns result means DCE, not speed), realistic data shapes/sizes, and separate benches per input size so you see the curve, not one point. `iai-callgrind` counts *instructions* instead of time — immune to machine noise, ideal for CI regression gates.
- **Allocation:** `dhat` (in-process heap profiling, call-site attribution), `heaptrack` (Linux), or just `perf` showing `malloc` frames wide. Allocation pressure is the most common "why is idiomatic Rust slow" answer — see [allocation strategies](../allocation-strategies/learning.md).
- **Off-CPU / async:** `tokio-console` (live task introspection — which tasks, how long parked, poll times), `tracing` + spans for structured latency attribution, `perf sched` / off-CPU flamegraphs (Linux) for blocked-time analysis, `strace -c` for a syscall census. If User+Sys ≪ wall time in the baseline, start *here*, not with a CPU profiler.
- **In production:** continuous profilers (parca/pyroscope-style, or `perf` in cron) at low sample rates — the 99-samples/s overhead is negligible and the payoff is profiling the *real* workload, which no synthetic load fully reproduces.

## When It Hurts

- **The microbenchmark mirage.** An isolated loop enjoys warm caches, a trained branch predictor, and no competing working set — conditions the function never sees in situ. Teams "win" 5× in criterion and ship a regression. Micro results are for *comparing variants of one function*, never for claiming system impact; only the macro baseline claims that.
- **Optimizing the profile instead of the problem.** A flamegraph of the wrong workload (dev data: 100 rows; prod: 100M) routes you confidently to the wrong code. The profile inherits the representativeness of the load that produced it — curating realistic workloads is half the craft, and the unglamorous half.
- **Observer distortion.** Instrumenting profilers and over-eager `tracing` spans in hot loops can dominate the measured time; even sampling at 10 kHz perturbs. Rule: measure the overhead of your measurement (run the baseline with and without) before trusting deltas under ~10%.
- **Layout luck.** Code alignment and link order swing small benchmarks several percent; adding an unrelated function can "speed up" your hot loop. Defenses: instruction-count benchmarking (iai) for CI, larger effect-size thresholds before celebrating, and suspicion of any win under ~5% that lacks a mechanism you can name.
- **Metric fixation.** Chasing p50 while p99 burns; maximizing throughput while latency queues explode ([backpressure](../../architecture-patterns/backpressure-and-rate-limiting/learning.md) territory); benchmark-suite scores diverging from user experience. The metric is a proxy — periodically re-check it against the thing it proxies.

## Benchmarking Methodology

The checklist that separates measurements from anecdotes:

1. **Quiet the machine.** Close the browser and IDE-indexer; on Linux pin the CPU governor to `performance` and consider disabling turbo for stable clocks (`no_turbo`); on Apple Silicon be aware of P-core/E-core scheduling (pin with `taskpolicy` or accept higher variance). Don't benchmark on battery. Laptops throttle thermally on long runs — watch for the downward drift mid-suite.
2. **Warm up, then measure many.** First iterations pay cold caches, page faults, JIT-like effects (lazy statics, branch training). `hyperfine --warmup`, criterion's warm-up phase — then enough iterations for the statistics to mean something.
3. **Report distributions, not points.** Mean ± σ minimum; percentiles for latency work. criterion's confidence intervals and outlier flags are the model. A single number without spread is a vibe.
4. **Defeat the optimizer honestly.** `black_box` inputs and outputs; verify the benchmark still computes the real thing (check the reported throughput is physically plausible — "parsing at 400 GB/s" means the parse was deleted).
5. **Change one thing.** Same compiler version, same flags, same data, same machine between A and B. `cargo bench` against a pinned toolchain; record the environment in the results.
6. **Automate the regression gate.** criterion's baseline comparison or iai instruction counts in CI, with thresholds loose enough to survive noise and tight enough to catch real drift. A performance suite that only runs when someone remembers is a suite that runs after the regression ships.
7. **Close the loop at macro.** Every accepted micro win must reproduce in the end-to-end baseline before the task is "done." This single rule prevents most decorative optimization.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Your service's wall time is 20 s; User is 3 s, Sys is 1 s. What class of problem is this, which tools apply, and why will a flamegraph from `perf record` mislead you?
2. A function is 8% of the profile. A colleague proposes a heroic 10× optimization of it. Give the Amdahl ceiling and the counterargument in one sentence.
3. criterion reports your parse function at 0.3 ns/iter. What almost certainly happened, and which two harness mistakes cause it?
4. IPC is 0.4 and LLC misses are high in the hot loop. Which three docs in this repo are now relevant, and which is *not* (yet)?
5. Why can a microbenchmark honestly show 5× while the end-to-end number doesn't move at all? Name two specific microbenchmark privileges.
6. Why is mean latency the wrong target for a page that fans out to 80 backend calls?

Measurement exercises (on your machine — the point is calibration):

- Take any CLI you've written; run the full funnel: hyperfine baseline → samply flamegraph → name the top-3 plateaus → predict the win from fixing #1 via Amdahl → fix it → verify macro. Keep the numbers; that's your first baseline file.
- Measure your measurement: time a trivial loop with `Instant::now()` per iteration vs. batched timing; compute your clock-read cost. Then run the same criterion bench 10 times across an hour of normal laptop use and plot the spread — that distribution is your noise floor; memorize its width before ever trusting a small win.
- Deliberately create the mirage: write a hash-map-lookup microbenchmark with 100 keys (fits in L1) vs. 10M keys (doesn't); compare the per-lookup times and connect the ratio to the cache-latency numbers in [cache locality](../cache-locality/learning.md).

## Open Questions

- Apple Silicon specifics: what PMU access does `samply`/Instruments actually expose on M-series (counter equivalents of `perf stat`), and how much does P/E-core migration pollute microbenchmarks in practice — measure.
- Causal profiling (coz / Emery Berger's work): does a Rust-workable coz setup exist, and does virtual-speedup analysis change any conclusion the flamegraph gave on a real project?
- Continuous profiling in prod for a Rust service: parca vs. pyroscope overhead and symbol handling for stripped release binaries — trial one.
- iai-callgrind in CI: what instruction-count delta threshold gives zero false positives over a month of commits on a real repo?
- tokio-console overhead in production builds — acceptable always-on, or attach-on-demand only?

## References

- Brendan Gregg, *Systems Performance* (2nd ed.) — the field's reference: USE method, off-CPU analysis, the full Linux observability map; ch. 6 (CPUs) and ch. 13 (perf) earn their pages.
- Brendan Gregg, [Flame Graphs](https://www.brendangregg.com/flamegraphs.html) — origin and reading guide; internalize "width, not height."
- Nicholas Nethercote, [The Rust Performance Book](https://nnethercote.github.io/perf-book/) — Rust-specific profiling setup, build configs, and the allocation-pressure catalog; short and dense.
- [criterion.rs documentation](https://bheisler.github.io/criterion.rs/book/) — the statistics chapter doubles as a benchmarking-methodology course.
- Emery Berger, "Performance Matters" (StrangeLoop talk) — layout luck and causal profiling; the best 40-minute vaccination against trusting small deltas.
- [samply](https://github.com/mstange/samply) — the macOS/Linux sampling profiler with the Firefox Profiler UI; the practical default on a Mac.
- Related topics in this repo: every other performance doc consumes this one — [cache locality](../cache-locality/learning.md), [memory layout](../memory-layout/learning.md), and [branch prediction](../branch-prediction/learning.md) are where the counter signatures route you; [allocation strategies](../allocation-strategies/learning.md) for the most common flamegraph finding in idiomatic Rust.
