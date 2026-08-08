# False Sharing — Learning Notes

## The Hardware Mechanism

The [cache-locality doc](../cache-locality/learning.md) treated the cache line as the unit of *transfer*. On a multicore machine it is also the unit of **coherence** — and that second role is this topic.

Cores have private L1/L2 caches, yet the machine must behave as if there's one memory. The **MESI protocol** (Modified/Exclusive/Shared/Invalid, per line, per core) enforces it: many cores may hold a line **Shared** for reading, but to *write* a line a core must own it **Exclusive/Modified** — which requires **invalidating every other core's copy**. The next core that wants the line must fetch it *from the writer's cache* (a cross-core transfer, **~40–100+ cycles**, worse across sockets/[NUMA nodes](../numa-awareness/learning.md)) before it can read or write. Writes to shared lines are, mechanically, a game of stealing the line back and forth.

Now the failure mode this doc exists for: **the protocol tracks lines, not variables.** Two *completely independent* variables that happen to sit in the same 64-byte line (128 on Apple Silicon) are, to the coherence hardware, *the same object*:

```
counters: [AtomicU64; 8]        // thread i increments counters[i] — no logical sharing at all
// 8 × 8 bytes = 64 bytes = ONE line: every increment by any core invalidates all other cores
```

Each thread's increment steals the line, invalidates seven caches, and forces the next thread's "private" counter access into a ~50-cycle remote fetch. That is **false sharing**: coherence traffic between threads that share *no data* — only an address neighborhood. The effect is invisible in the code (each thread touches only its own element!), absent single-threaded, and grows with thread count — the classic "we added threads and throughput went *down*" mystery.

Two amplifiers worth knowing: **the adjacent-line prefetcher** on Intel cores can pull line pairs, making the effective conflict granularity 128 bytes even where the line is 64; and **atomic RMW operations** (`fetch_add`) always demand exclusive ownership — but note that *plain non-atomic writes do too*: false sharing needs no atomics, just writes from different cores landing in one line. True sharing (threads genuinely contending on one variable — an `Arc`'s refcount, a global counter, a mutex) pays the identical ping-pong cost; the difference is that false sharing is *free* to fix (move the data) while true sharing requires *redesign* (reduce the sharing) — distinguishing them is the diagnostic skill.

## Mental Model

**On a multicore machine, a written-to cache line is a token that only one core may hold; every other core's access to *anything* on that line queues behind stealing it back.** The design rules that fall out:

1. **Writes are the poison; reads are safe together.** Any number of cores can hold a line Shared and read it forever at full speed. The rule targets *writes*: a line written by one core must not hold data any *other* core touches (read **or** write — readers of a written line pay the ping-pong too). Read-mostly data can pack densely; frequently-written data needs isolation.
2. **Per-thread state must be per-line.** The `counters[thread_id]` array is the canonical bug because it does the right thing logically (no sharing) and the wrong thing physically (one line). Fix by *padding* each slot to a line (`crossbeam_utils::CachePadded<T>` — which knows the 128-byte platforms), or better, by making the state genuinely thread-local ([the batching move](../batching-and-amortization/learning.md): accumulate privately, merge rarely).
3. **Layout separates the hot-written from everything else.** Inside a struct shared across threads, a frequently-written field (a counter, a state flag) adjacent to read-mostly fields (config, a name) drags every reader of the cold fields into the ping-pong. [Memory-layout](../memory-layout/learning.md)'s hot/cold splitting, with a new criterion: split by *writer*, not just by temperature — `#[repr(align(64))]` / `CachePadded` around the written field, or move it out of the shared struct entirely.
4. **The cost model:** a falsely-shared write path runs at cross-core-transfer speed (~50–100 cycles/op) instead of L1 speed (~1–4 cycles/op) — a **10–50× per-operation tax** — and it *scales negatively*: more threads = more stealing = lower total throughput. This inverts every intuition from the single-core docs: there, packing data tightly was the goal; here, written-by-different-cores data must be *spread out*. Same hardware, opposite prescription, resolved by asking *who writes it*.
5. **True sharing wears the same costume.** An `Arc<T>` cloned/dropped on every task hammers one refcount line (the [zero-copy doc's](../zero-copy/learning.md) contended-`Bytes` warning); a global `AtomicU64` metric incremented by every request is a deliberate single line taken by all cores. Padding fixes neither — the *algorithm* shares. Fixes are structural: shard the counter (per-thread cells, merged on read), clone `Arc`s once per thread instead of per operation, replace the shared accumulator with [rayon's](../parallelism-and-work-stealing/learning.md) fold/reduce (accumulate privately, combine at the end).

Where the model stops: if threads don't write shared lines at meaningful rates, none of this matters — padding cold data wastes cache for nothing. This is a *profile-confirmed* fix (scaling curves, coherence counters), not a default posture.

## Worked Example

Eight threads, each incrementing its own `u64` counter 100M times. Three layouts; illustrative numbers (M-series/x86 desktop class — reproducing is exercise one).

```rust
// A. Adjacent: [AtomicU64; 8] — one cache line for all eight "private" counters
// B. Padded:   [CachePadded<AtomicU64>; 8] — one line each
// C. Local:    plain u64 per thread on its own stack; one atomic merge at the end
```

```
             1 thread    8 threads    scaling
A. adjacent   0.9 s       11.8 s      NEGATIVE — 8× the cores, ~13× the wall time
B. padded     0.9 s        0.95 s     flat ≈ perfect
C. local      0.25 s       0.26 s     flat, and ~3.5× faster than B per op
```

Readings: **A's negative scaling** is the signature — identical code, one thread fast, eight threads *slower than one*, because 800M increments became 800M line-steal events (~50+ cycles each) instead of L1 hits. **B fixes it with 448 wasted bytes** — padding buys back perfect scaling; this is the cheapest 12× in this repo. **C beats B** because even a padded *atomic* pays the RMW cost (~10–20 ns uncontended, plus preventing some compiler optimizations); a plain local variable pays ~nothing, and the sharing happens once at merge time. The escalation A→B→C mirrors the model: isolate the lines → then question whether the sharing was needed at all.

The counter-side view (Linux `perf c2c`, or `perf stat -e cache-references,cache-misses` per variant): A shows a flood of HITM events ("hit in another core's modified line" — the smoking gun); B and C show none. On macOS, where `perf c2c` doesn't exist, the *scaling curve itself* is your instrument: per-thread throughput vs. thread count, flat = healthy, falling = coherence traffic (false or true — then check the code for who writes what).

## Applying It

- **`crossbeam_utils::CachePadded<T>`** around any per-thread slot in a shared array (queue heads/tails, per-worker counters, shard locks): it sizes to 128 bytes on the platforms that need it (Apple Silicon, modern x86 accounting for the adjacent-line prefetcher). Roll-your-own is `#[repr(align(128))] struct Padded<T>(T)`.
- **Prefer thread-local accumulate + merge** over padded-shared wherever the read side tolerates lag: metrics via per-thread cells swept periodically ([batching](../batching-and-amortization/learning.md)'s per-thread pattern), rayon `fold`/`reduce` instead of a shared atomic in parallel loops, `thread_local!` scratch state. Padding makes sharing *cheap*; locality-of-writes makes it *free*.
- **Split shared structs by writer:** hot-written fields (`AtomicUsize` counters, seqlocks, state flags) get their own padded line or move out of the struct; read-mostly config packs densely and stays `Shared` in every cache happily. Audit any `struct Shared { config: …, hits: AtomicU64 }` shape — that counter is taxing every config reader.
- **Watch the Arc traffic:** `Arc::clone` per item in a hot pipeline = all cores hammering one refcount line (true sharing). Clone once per thread/task at spawn, pass `&T` inside; or restructure ownership so the hot path holds a direct reference.
- **Queue and pool internals:** ring-buffer head and tail indices *must* live on separate lines (every SPSC/MPMC implementation does this — `crossbeam`'s source is the reference reading); a mutex protecting hot data wants the mutex and the data co-located (*same* line is good here — one steal gets both: the exception that proves you're reasoning, not pattern-matching).
- **Verify the fix mechanically:** thread-scaling sweep before/after (the curve is the macOS instrument); `perf c2c` on Linux for HITM attribution when available; and a regression guard — the scaling benchmark in CI, since a refactor that reorders one struct can silently reintroduce the line-share.

## When It Hurts

- **Padding everything wastes the cache you optimized.** `CachePadded` turns an 8-byte counter into 128 bytes — 16× the footprint; padding a 64-slot array costs 8 KB of L1 (a quarter of it) for data that may never be contended. Padding is for *demonstrated* write-shared lines, not decoration; the single-core docs' density rules still govern everything only one thread writes.
- **Padding can't fix true sharing.** If all threads increment *the same* counter, isolation changes nothing — the line is contended because the *algorithm* contends. The fix is sharding/local-merge (accepting read-side lag) or accepting the cost consciously (some global sequence numbers are genuinely necessary — [lock-free doc's](../lock-free-concurrency/learning.md) territory).
- **The diagnosis is easy to get backwards:** falling scaling curves also come from lock contention, memory-bandwidth saturation ([the roofline](../cache-locality/learning.md) — all cores share the DRAM pipe), NUMA remote access, and work imbalance ([parallelism doc](../parallelism-and-work-stealing/learning.md)). False sharing's differentiators: it appears with *writes* to compact shared structures, vanishes with padding (cheap to test!), and shows HITM events where counters exist. Try-the-padding is a legitimate five-minute experiment; leaving speculative padding in without the experiment isn't.
- **Over-aligned types propagate:** `#[repr(align(128))]` on a type infects every struct containing it and every `Vec` of it (each element padded). Contain the padding at the *usage site* (the array of workers) rather than the type itself, unless the type's only job is being a padded slot.
- **macOS tooling gap:** no `perf c2c` means attribution (which line? which fields?) is inference from scaling curves plus code reading. Keep the counters-array and struct-layout patterns in your head — on this platform, the *catalog of shapes* is your profiler.

## Benchmarking Methodology

- **The thread-scaling sweep is the primary instrument:** total and per-thread throughput at 1/2/4/8/… threads. Healthy: flat per-thread. False sharing: per-thread throughput *falls* as threads rise, often below the single-thread line in aggregate. Run it before and after padding; the delta is the effect, isolated.
- **Pin threads for stable numbers** (core affinity) — on macOS, QoS classes and P/E-core migration add variance the sweep must average over ([profiling doc's](../profiling-and-measurement/learning.md) noise discipline); report the spread.
- **Linux: `perf c2c record`/`report`** attributes coherence misses to data addresses and shows HITM counts per cacheline with the offending fields — the definitive instrument; use it when a Linux box is available even if production is macOS.
- **Beware benchmark-layout luck:** heap allocators may *accidentally* pad your test's slots into separate lines (size classes!) while production's arena packs them — control layout explicitly in the benchmark (one array, verified adjacent via address math: print `&slots[i] as *const _ as usize` deltas).
- **Separate the atomic cost from the sharing cost:** benchmark C (thread-local plain) vs B (padded atomic) isolates RMW overhead; B vs A isolates coherence. Three variants, two subtractions, full attribution.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Walk MESI through two cores alternately writing the same line: state transitions per write, and where the ~50 cycles go.
2. Why does false sharing need no atomics and no logical sharing? Construct the minimal buggy layout.
3. A/B/C in the worked example: which comparison isolates coherence cost, which isolates RMW cost, and why is C's merge safe despite being unsynchronized during accumulation?
4. Distinguish false from true sharing by fix: give one example of each and why padding helps exactly one of them.
5. Your 8-thread service scales negatively. List the five alternative diagnoses and the five-minute experiment that isolates false sharing.
6. Why is mutex-next-to-its-data *good* line sharing? What principle does the exception encode?
7. Why should `CachePadded` wrap the array slot rather than the payload type — what propagates otherwise?

Measurement exercises:

- Reproduce A/B/C at 1/2/4/8 threads on your machine (verify slot adjacency via address printing); plot per-thread throughput. On the M-series, test both 64- and 128-byte padding — does 64 suffice, or does the platform need 128? (You're measuring the effective coherence granularity.)
- Build the struct version: `struct Shared { config: [u8; 48], hits: AtomicU64 }` with 7 reader threads (reading config) + 1 writer (incrementing hits); measure reader throughput with the counter in-struct vs. `CachePadded` vs. moved out entirely. The reader tax is the lesson most people haven't internalized.
- Find the ring-buffer head/tail padding in `crossbeam`'s source (`ArrayQueue` or the deque); write down which fields are padded and why — reading production-grade padding decisions calibrates yours.

## Open Questions

- Apple Silicon coherence granularity: is the effective conflict size 128 bytes uniformly, and do the E-cores change the ping-pong cost profile? (The measurement exercise answers the first half.)
- macOS attribution tooling: does Instruments' "Cache" template or `powermetrics` expose anything HITM-like, or is Linux-under-VM (with PMU passthrough?) the only real option?
- Cross-socket vs cross-CCX (AMD) vs cross-cluster (M-series) transfer costs: get real numbers per topology — the "~40–100 cycles" range is wide because the topology matters ([NUMA doc](../numa-awareness/learning.md) preview).
- `CachePadded`'s 128-byte choice: which platforms does crossbeam pad to 128 vs 64 today, and what's the detection logic? (Read the source; it's short.)
- Seqlock and RCU-style read-mostly schemes as the structural escape from reader-tax problems — where do they enter Rust practice (`arc-swap`, `left-right`)? ([Lock-free doc](../lock-free-concurrency/learning.md) hand-off.)

## References

- Ulrich Drepper, *What Every Programmer Should Know About Memory*, §3.3.4 & §6.4.2 — MESI and the multiprocessor-optimization section; the mechanism from the source.
- [`crossbeam_utils::CachePadded` docs + source](https://docs.rs/crossbeam-utils/latest/crossbeam_utils/struct.CachePadded.html) — the platform-granularity table encoded as `cfg` attributes; five minutes, permanently useful.
- Joe Duffy / Herb Sutter's classic false-sharing writeups ("Eliminate False Sharing", DDJ) — the counters-array pathology, named and measured, from the era that discovered it at scale.
- `perf c2c` documentation (Linux) — the attribution workflow: record, report, read the HITM table.
- Related topics in this repo: [Cache Locality](../cache-locality/learning.md) (the line as transfer unit — this doc is its coherence sequel; density rules invert for written data), [Memory Layout](../memory-layout/learning.md) (split-by-writer as a layout criterion; `repr(align)`), [Batching & Amortization](../batching-and-amortization/learning.md) (accumulate-locally-merge-rarely), [Parallelism & Work Stealing](../parallelism-and-work-stealing/learning.md) (fold/reduce as the structural fix), [Lock-Free Concurrency](../lock-free-concurrency/learning.md) (contended atomics as deliberate single-line sharing), [NUMA Awareness](../numa-awareness/learning.md) (ping-pong across memory domains).
