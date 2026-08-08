# Profiling & Measurement — Quick Reference

Core model: only measurements have opinions. Funnel: macro baseline → profile (where) → counters (why) → micro iterate → **re-verify macro**. Amdahl first: a 10% slice caps the win at 10%. Details in [learning.md](learning.md).

## Which Tool for Which Question (Rust)

| Question | Tool | Note |
| --- | --- | --- |
| How slow, end to end? (baseline) | `hyperfine` (CLI), metrics + load gen (service) | The number all changes are judged against |
| Where does on-CPU time go? | `samply` (macOS+Linux), `cargo flamegraph`, `perf record --call-graph dwarf` | Needs `[profile.release] debug = true`; read width, not height |
| Why is that code slow? | `perf stat` (IPC, miss rates), `cachegrind` (deterministic A/B) | IPC ~4 compute-bound; ~0.5 memory/wait-bound |
| Is variant B faster than A? | `criterion` / `divan` | `black_box` in AND out; realistic sizes; per-size benches |
| CI regression gate | `iai-callgrind` (instruction counts) or criterion baselines | Instructions are noise-immune |
| Who allocates? | `dhat`, `heaptrack`, malloc frames in flamegraph | Top "idiomatic Rust is slow" answer |
| Where does *blocked* time go? | `tokio-console`, off-CPU flamegraphs, `perf sched`, `strace -c` | Sampling profilers can't see waiting |
| Prod reality | Continuous profiler at low Hz | Synthetic load never fully matches |

## Rules of Thumb

- User+Sys ≪ wall → off-CPU problem: skip the CPU profiler entirely.
- No mechanism you can name + win < 5% → assume layout luck / noise.
- Micro win that doesn't move macro = decoration; macro is the truth.
- Variance is a finding, not noise: explain it before averaging it away.
- Latency = distribution: p99 for one hop; fan-out of N ≈ sum of tails.
- Profile inherits the workload's representativeness — curate real data shapes.
- Measure your measurement: overhead-with vs. -without before trusting <10% deltas.

## Numbers to Remember

| Thing | Number |
| --- | --- |
| TSC / `Instant::now()` read | ~10–30 ns — batch iterations when timing < 100 ns work |
| Sampling profiler default | ~1 kHz, negligible overhead; instrumentation distorts hot loops |
| "0.2 ns/iter" result | Dead code elimination, always — fix `black_box` |
| Healthy vs. starving IPC | ~3–4 vs. ≤1 (memory-bound or stalled) |

## Benchmark Checklist

- [ ] Governor pinned / turbo noted / not on battery; thermal drift watched
- [ ] Warmup phase, then statistically meaningful iteration count
- [ ] Distribution reported (mean ± σ minimum; percentiles for latency)
- [ ] `black_box` verified; throughput physically plausible
- [ ] One variable changed; toolchain + environment recorded
- [ ] Regression gate automated in CI
- [ ] Macro baseline re-run before declaring victory

## Key References

- Gregg, *Systems Performance* + [flamegraphs](https://www.brendangregg.com/flamegraphs.html).
- Nethercote, [Rust Performance Book](https://nnethercote.github.io/perf-book/).
- Berger, "Performance Matters" — layout luck; distrust small deltas.
