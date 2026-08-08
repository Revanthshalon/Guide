# Parallelism & Work Stealing — Learning Notes

## The Hardware Mechanism

The cores are just *there* — 8–16 on a desktop, a P/E mix on Apple Silicon — and using them costs nothing per cycle. What costs is everything *around* them:

- **Thread lifecycle:** spawning an OS thread is ~10s of µs (stack allocation, kernel bookkeeping); a context switch ~1–10 µs plus the invisible tax of [cache/TLB repopulation](../batching-and-amortization/learning.md). This is why thread *pools* exist — spawn once, feed forever — and why per-task threads are an anti-pattern: the pool is [amortization](../batching-and-amortization/learning.md) applied to thread creation.
- **Cores communicate at coherence prices.** Handing work or results between cores moves cache lines: ~40–100+ cycles per transfer, and shared-line writes ping-pong ([false sharing](../false-sharing/learning.md)). Every synchronization point — queue, mutex, atomic counter — is line traffic; parallel design is largely the art of *not* communicating.
- **The memory pipe is shared.** N cores share one DRAM interface: on a machine with ~60 GB/s of bandwidth, one core streaming at 25 GB/s leaves less than 3× headroom *no matter how many cores join* — memory-bound work stops scaling at 2–3 cores while compute-bound work scales to all of them. The [roofline](../cache-locality/learning.md) is per-*machine*, not per-core, and it is the most common reason "we parallelized and got 2.1×" surprises teams with 8 cores.
- **Heterogeneity is here:** M-series P-cores and E-cores differ ~2–3× in single-thread speed; the OS migrates threads between them. Uniform-split strategies inherit the variance; this quietly strengthens the case for dynamic scheduling (below).

## Mental Model

**Parallel speedup is a fight against three limits: the serial fraction (Amdahl), the shared memory pipe (roofline), and the cost of coordination (communication + imbalance). Work stealing is the industry's answer to the third.**

1. **Amdahl's law is the ceiling:** speedup = 1/(s + (1−s)/N) for serial fraction s. At s = 10%, infinite cores give you 10×; 8 cores give 4.7×. The serial fraction hides in ambush: the `Mutex` inside the parallel loop (serializing the "parallel" part), the single-threaded merge phase, the allocator behind every task ([contended shared service](../allocation-strategies/learning.md)), the final sort. **Measure s** (one thread vs. many, fit the curve) before promising N×. The optimistic counterweight — Gustafson: bigger problems have smaller serial fractions, so scaling the *workload* often rescues scaling the *machine*. And the [profiling funnel's](../profiling-and-measurement/learning.md) ordering still binds: parallelizing unoptimized code multiplies waste by N — single-thread optimization first, then cores.
2. **Parallelism ≠ concurrency.** Concurrency is *structure* — many logical tasks in flight, possibly on one core (the [async doc's](../async-and-io/learning.md) subject: waiting well). Parallelism is *simultaneous execution* — many cores computing (this doc: computing more). Rust's ecosystem splits cleanly along this line: tokio for concurrency, rayon for parallelism; confusing them puts CPU work on the async runtime (starving I/O tasks) or async waiting on the compute pool (idling cores).
3. **The scheduling problem, and why stealing wins it.** Given P workers and a pile of tasks with *unknown, varying* costs:
   - **Static partitioning** (split N items into P chunks) has zero coordination cost and a fatal flaw: the slowest chunk gates the job ([the tail again](../batching-and-amortization/learning.md)). One expensive item in chunk 3 = seven idle cores waiting.
   - **A central shared queue** balances perfectly and contends horribly: every dequeue is a CAS on one hot line — the [single-line contention](../lock-free-concurrency/learning.md) disaster as *designed architecture*.
   - **Work stealing** takes both wins: each worker owns a **deque**; it pushes/pops its own work at the *bottom* (LIFO — the task just spawned is cache-warm: [locality](../cache-locality/learning.md) exploited by scheduling); an idle worker **steals from the top** of a random victim (FIFO — the *oldest* task, which in divide-and-conquer is the *largest* subtree: one steal buys the most work, minimizing steal frequency). Coordination cost is paid **only when imbalance exists** — balanced runs never touch another worker's deque. The deque itself (Chase-Lev) is [lock-free concurrency's](../lock-free-concurrency/learning.md) flagship production structure: single-owner bottom, CAS-only-on-steal top.
4. **Granularity is the batching knee, again.** Every task pays scheduling overhead F (~ns for a rayon task, but real); per-item work m. Items-per-task below F/m drowns in scheduling; too-coarse re-creates static partitioning's imbalance. Rayon's divide-and-conquer splits *adaptively* — keep dividing while thieves are hungry, stop when they're not — which is why `par_iter` on a million tiny items works at all; `with_min_len`/`par_chunks` are the manual override when the heuristic needs help ([N ≈ F/m](../batching-and-amortization/learning.md), third appearance in this repo).
5. **Communication-free reduction is the shape to aim for:** fork-join over [dense, DoD-shaped data](../data-oriented-design/learning.md), thread-local accumulation, merge at the end — `fold` + `reduce`, the [false-sharing rung-0 fix](../false-sharing/learning.md) as a first-class API. The transform-shaped signatures DoD prescribes (`fn f(items: &mut [T])`) are exactly what `par_iter` wants; the two disciplines were always one.

## Worked Example

Three experiments on an 8-core machine (illustrative shapes; reproducing is exercise one).

**A. The bandwidth wall.** Sum 1 GB of `f64`s (memory-bound) vs. compute a cheap hash per element ×64 rounds (compute-bound):

```
                      1T      2T      4T      8T      efficiency@8
memory-bound sum      1.0×    1.9×    2.6×    2.8×    35%   ← flat at the DRAM ceiling
compute-bound hash    1.0×    2.0×    3.9×    7.6×    95%   ← near-linear
```

Same cores, same `par_iter`, opposite verdicts — the *workload's* arithmetic intensity decided, not the parallel framework. Check GB/s against the machine ceiling before blaming the scheduler; the memory-bound fix lives in [cache](../cache-locality/learning.md)/[layout](../memory-layout/learning.md) docs (shrink and densify), not in more threads.

**B. Imbalance, and stealing fixing it.** 10 000 items where 1% cost 100× (a heavy tail — the realistic case: mixed file sizes, mixed query costs):

```
static 8 chunks:      3.9×    (unlucky chunks; cores idle at the tail — per-worker
                               busy-times: 41 s, 12 s, 11 s, … the slowest gates)
rayon (stealing):     7.4×    (thieves drain the heavy chunk's neighbors' work,
                               then split the heavy items' own subtasks)
```

The per-worker busy-time histogram is the diagnostic: static shows a ragged skyline, stealing shows a flat one. Nothing about the code changed except who decides where work runs.

**C. The accumulator trap, three ways** (count matches in parallel):

```rust
// 1. Mutex<u64> incremented per item:        1.9× — the lock serialized the loop (Amdahl, self-inflicted)
// 2. AtomicU64 fetch_add per item:           3.1× — no lock, but one hot line (single-line contention)
// 3. fold(|| 0u64, ..).reduce(|| 0, +):      7.7× — thread-local partials, merged once
data.par_iter().fold(|| 0u64, |acc, x| acc + is_match(x) as u64)
               .reduce(|| 0, |a, b| a + b)
```

The [counter table](../lock-free-concurrency/learning.md) reborn inside a parallel loop: `fold`/`reduce` *is* rung 0 (don't share) packaged as an iterator adapter. If your `par_iter` closure touches an `Arc<Mutex<_>>`, you've built experiment C-1 and Amdahl is already collecting.

## Applying It

- **Rayon is the default engine:** `par_iter`/`par_iter_mut` on slices and Vecs (dense [DoD layouts](../data-oriented-design/learning.md) parallelize trivially); `rayon::join(a, b)` for two-way fork-join; `rayon::scope` for irregular task trees; `par_sort_unstable`, `par_chunks_mut` for the common bulk shapes. It's a fixed global pool (size = logical cores by default) — configure via `ThreadPoolBuilder` when you must, and prefer *one* pool per process.
- **Granularity control:** trust the adaptive splitter first; `with_min_len(k)` when per-item work is tiny (pick k ≈ F/m, then measure); `par_chunks(k)` when items want batch processing anyway ([batching](../batching-and-amortization/learning.md) and scheduling amortization in one move).
- **Accumulate with `fold`/`reduce`/`map`+`sum`,** never shared mutable state in the closure. For collecting results: `collect()` on parallel iterators is smart (per-thread Vecs, merged); `flat_map` + `collect` covers most "gather results" shapes without a Mutex in sight.
- **Scoped threads for the non-rayon cases:** `std::thread::scope` when you need a *few* long-lived workers with borrowed data (pipeline stages, dedicated I/O thread) rather than data parallelism — channels between them ([flume/crossbeam](../lock-free-concurrency/learning.md)), one owner per stage.
- **Keep the runtimes apart:** CPU-bound work inside an async service goes to rayon (bridge with a oneshot channel) or `spawn_blocking` — never inline on the tokio runtime ([async doc's](../async-and-io/learning.md) boundary). And audit *total* thread count: tokio workers + rayon pool + ad-hoc threads all sized "num_cpus" = 3× oversubscription and context-switch churn.
- **Apple Silicon notes:** logical-core count includes E-cores — for latency-critical parallel work measure with the pool pinned smaller (P-core count); accept scheduler migration variance in benchmarks ([report the spread](../profiling-and-measurement/learning.md)).
- **Check the layout first:** parallel sweeps magnify [false sharing](../false-sharing/learning.md) (adjacent outputs written by different workers — `par_chunks_mut` naturally avoids it; `par_iter_mut` over `&mut [T]` gives each worker disjoint elements, safe but possibly line-sharing at chunk borders for small T — rarely matters, worth knowing).

## When It Hurts

- **Memory-bound work: the wall, not the win.** Experiment A's 2.8× is a *good* outcome for streaming sweeps; expecting 8× and "fixing" the scheduler wastes weeks. Diagnose with GB/s vs. ceiling; fix with arithmetic intensity (fuse passes, shrink data) or accept the ceiling.
- **The hidden serial fraction:** locks in closures (C-1), the allocator under [alloc-heavy tasks](../allocation-strategies/learning.md) (arena-per-worker fixes it), one `println!`/tracing line inside the hot closure (a global stdout lock!), channel send on every item ([batch them](../batching-and-amortization/learning.md)), the final single-threaded `collect`-and-sort. Amdahl-fit your scaling curve: the measured s names the debt.
- **Tiny tasks:** `par_iter` over 10M two-ns items without `with_min_len` can run *slower* than sequential — all scheduling, no work. The knee formula predicts it; the fix is one method call.
- **Nested/recursive parallelism inside libraries:** a rayon-using function called *from* a rayon task is fine (same pool, stealing composes — this is rayon's genius); but two *different* pools, or rayon inside tokio workers, or OpenMP-style libraries with their own pools stack threads badly. One process, one compute pool, enforced by convention.
- **Parallelizing the unprofiled:** N cores × wasteful code = N× waste, and the parallel version's complexity now *obscures* the single-thread fix. [Funnel](../profiling-and-measurement/learning.md) order: optimize serial, then parallelize what remains hot.
- **Determinism leaves quietly:** parallel float reduction reorders additions ([the SIMD doc's sign-off](../simd/learning.md), again); `reduce` order varies run-to-run; tests that snapshot exact floats start flaking. Decide tolerance or fix order (`fold` in index order via `par_chunks` + sequential merge) *consciously*.

## Benchmarking Methodology

- **The scaling curve is the instrument** (third doc running): 1/2/4/…/N threads (`ThreadPoolBuilder::num_threads`), plot speedup and efficiency (speedup/threads). Shapes diagnose: early flattening + high GB/s = bandwidth wall; flattening + low GB/s = serial fraction or contention (then: [false-sharing's padding experiment](../false-sharing/learning.md), lock audit); sawtooth on M-series = P/E migration (pin or report spread).
- **Fit Amdahl to the curve** to extract s — two-parameter fit, and suddenly "it doesn't scale" becomes "s = 8%, and here's the lock it lives in."
- **Per-worker busy/idle histograms** (rayon doesn't expose this directly — instrument with per-worker counters via `fold`, or `tracing` spans) — the imbalance diagnostic from experiment B.
- **Report absolute baselines, not just speedup:** 7× over an unoptimized serial loop can still lose to one optimized core ([SIMD](../simd/learning.md) + [locality](../cache-locality/learning.md) often beat 8 threads of scalar pointer-chasing). Speedup-vs-*best-serial* is the honest metric.
- **Control the pool in criterion:** rayon's global pool persists across benchmark iterations — warm, which is realistic, but size it explicitly per measurement and don't let two benchmarks' pools interleave.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Derive the 8-core speedup at s = 5%, 10%, 25%. At what s does an 8-core machine deliver less than 3×? What's Gustafson's rejoinder?
2. Why does the owner pop LIFO from the bottom while thieves steal FIFO from the top? Name both locality arguments and the steal-frequency argument.
3. Experiment A: same code, 2.8× vs 7.6× — what property of the workload decided, how do you measure it, and which docs own the fix for the slow case?
4. Reconstruct the three accumulator variants and their mechanisms. Why is `fold`/`reduce` exactly the false-sharing doc's rung 0?
5. Where does a central task queue's cost concentrate, mechanically, and when does work stealing pay *zero* coordination cost?
6. Your `par_iter` is slower than `iter`. Give the three most likely causes in testing order, and the one-line fix for the most common.
7. Why must tokio and rayon stay separate, and what goes wrong in each direction of mixing?

Measurement exercises:

- Reproduce experiments A and C on your machine (pool sizes 1/2/4/8; on M-series also try pool = P-core count): plot the four curves, mark your measured DRAM ceiling on A, and Amdahl-fit C-1 to extract the mutex's serial fraction.
- Build experiment B: 10 K items, 1% costing 100× (`spin_loop` a calibrated amount); compare static `chunks(n/8)` via scoped threads against `par_iter`, and produce the per-worker busy-time histogram (fold with a per-worker `(work_done, id)` accumulator).
- Find the knee: `par_iter` over 10M trivial items with `with_min_len` ∈ {1, 64, 1 K, 64 K}; plot and connect to N ≈ F/m — you're measuring rayon's per-task F on your machine.

## Open Questions

- Rayon's actual per-task overhead F on M-series (the third exercise measures it) — and how adaptive splitting decides "small enough" internally (read the plumbing: `Splitter`'s steal-count heuristic).
- P/E-core strategy: does pinning rayon to P-cores beat letting the scheduler use all cores for throughput workloads? Latency workloads? Measure both.
- Chase-Lev in crossbeam-deque: read the implementation against the paper — where does the memory-ordering subtlety concentrate (the `steal` CAS and the bottom load)?
- The USL (Universal Scalability Law) as a richer fit than Amdahl (adds a coherence-penalty term that models *retrograde* scaling) — fit both to experiment C-2's curve; does USL capture the atomic's ping-pong?
- Pipeline parallelism (scoped threads + bounded channels) vs data parallelism for stream processing: where's the crossover, and how does [backpressure](../../architecture-patterns/backpressure-and-rate-limiting/learning.md) map onto bounded channel depth?

## References

- [Rayon docs](https://docs.rs/rayon) + Niko Matsakis, "Rayon: data parallelism in Rust" (blog) — the model and its design rationale from the source.
- Chase & Lev, "Dynamic Circular Work-Stealing Deque" (SPAA 2005) — the deque; readable, and the crossbeam implementation's blueprint.
- Blumofe & Leiserson, "Scheduling Multithreaded Computations by Work Stealing" (JACM 1999) — the theory: why stealing is provably near-optimal; skim for the intuitions.
- Amdahl (1967) / Gustafson (1988) — the two one-page arguments everyone cites; read the originals once.
- Paul McKenney, *Is Parallel Programming Hard, And, If So, What Can You Do About It?* (free online) — the encyclopedic treatment when you outgrow this doc.
- Related topics in this repo: [Lock-Free Concurrency](../lock-free-concurrency/learning.md) (the deque is its flagship; fold/reduce is its rung 0), [False Sharing](../false-sharing/learning.md) (what parallel sweeps magnify), [Batching & Amortization](../batching-and-amortization/learning.md) (granularity = the knee; pools = amortized spawning), [Cache Locality](../cache-locality/learning.md) (the shared roofline; LIFO warmth), [Data-Oriented Design](../data-oriented-design/learning.md) (the layouts par_iter wants), [Async & I/O](../async-and-io/learning.md) (the other half: waiting well).
