# Cache Locality — Learning Notes

## The Hardware Mechanism

**DRAM is ~100 ns away. The core retires up to ~4–8 instructions per cycle at ~0.3 ns per cycle. Uncached, one memory access costs the same as ~300–1000 instructions.** The entire cache hierarchy exists to hide that gap, and cache locality is the craft of letting it.

The hierarchy (typical modern x86; Apple Silicon in parentheses where it differs meaningfully):

| Level | Size | Latency | Notes |
| --- | --- | --- | --- |
| L1d | 32–48 KB/core (M-series: 128 KB P-core) | ~4–5 cycles ≈ 1–2 ns | Split instruction/data |
| L2 | 512 KB–2 MB/core | ~12–16 cycles ≈ 4–5 ns | Per-core (shared per-cluster on M-series) |
| L3 | 8–96 MB, shared | ~40–60 cycles ≈ 12–20 ns | Shared across cores; a partitioned resource under contention |
| DRAM | GBs | ~60–110 ns | Plus row-buffer and channel effects; further on [NUMA](../numa-awareness/learning.md) |

Treat the numbers as *ratios* to memorize, not specs: **L1 : DRAM ≈ 1 : 50–100.** (Exact values per machine are one of the exercises.)

Four mechanisms determine everything in this topic:

- **The cache line is the unit of everything.** Memory moves in fixed-size lines — **64 bytes** on x86, **128 bytes** on Apple M-series. Touch one byte, pay for the full line; the other 63 (or 127) bytes are free *if you use them*. Spatial locality is just "use what you already paid for."
- **Eviction is by set, decided by address bits.** Caches are set-associative (8–16 ways): each address maps to one small set of slots. Consequence worth knowing: access patterns striding by large powers of two can hammer one set while the rest of the cache idles — the occasional "why is my 4096-column matrix pathologically slow" mystery.
- **The prefetcher watches your address stream.** Sequential and constant-stride patterns are detected after a few accesses, and the hardware starts fetching lines *ahead of you* — sequential scans run at near-DRAM-bandwidth with latency fully hidden. Pointer chasing defeats it totally: the next address is unknowable until the current load returns, so every hop eats the full latency, serialized. This single fact is why arrays beat linked structures by an order of magnitude, not a few percent.
- **The TLB is a second cache you're also filling.** Virtual→physical translations are cached too (~1500–2000 entries covering 4 KB pages ≈ a few MB of reach); scattered access across a large heap misses in the TLB *on top of* the data cache. Huge pages (2 MB) are the lever when profiles show `dTLB-load-misses`.

One more, deferred: when *multiple cores* touch the same lines, coherence traffic enters (MESI) — that story is [false sharing](../false-sharing/learning.md)'s; this doc is single-core locality.

## Mental Model

**Memory is not an array of bytes at uniform cost; it's a hierarchy of neighborhoods, and the price of a byte is the distance to its line.** The model that predicts performance:

1. **Think in lines, not bytes.** A `Vec<f64>` holds 8 values per x86 line: a sequential sum does one memory transaction per 8 elements. A linked list of the same f64s holds one value per line (node + padding), plus a serialized latency chain — 8× the traffic, ~50× the latency exposure. Same algorithm, same O(n), an order of magnitude apart.
2. **Three localities, in order of power:** *spatial* (use neighbors of what you touched — layout's job), *temporal* (touch it again before eviction — scheduling's job: do all work on a datum while it's resident), *predictability* (make the address stream regular so the prefetcher hides what latency remains — traversal order's job).
3. **The working-set cliff.** Performance vs. data size isn't a slope, it's a staircase: flat while the working set fits L1, step at L2, step at L3, cliff at DRAM. This is why microbenchmarks on small data lie about production (the [mirage](../profiling-and-measurement/learning.md)), why "it got 10× slower when the table grew" has a mechanical explanation, and why *shrinking the working set* (smaller types, [tighter layout](../memory-layout/learning.md), splitting hot from cold) is a first-class optimization even when it "does more work."
4. **Bandwidth is the other wall.** Latency you can hide (prefetch, parallelism); bandwidth you can only *not waste*. A core streaming sequentially can pull tens of GB/s; if only 4 bytes of each 64-byte line are useful, effective bandwidth is 1/16th of that. Wasted line fraction is the quantity to minimize — which is the entire argument for structure-of-arrays ([data-oriented design](../data-oriented-design/learning.md)).

Where the model breaks down: tiny working sets (everything fits in L1 — layout is then irrelevant, see When It Hurts); compute-bound code (high [IPC](../profiling-and-measurement/learning.md), few loads — the core isn't waiting on memory, so locality buys nothing); and truly random required access (a hash lookup *must* jump — you optimize by making the jump's target compact, not by making it sequential).

## Worked Example

Three traversals of the same 512 MB of `f64`s (64M elements), summing them all. Numbers are from a typical x86 desktop — *illustrative shapes, not specs*; reproducing them on your machine is the first exercise.

```rust
// A. Sequential: the prefetcher's best friend
let sum: f64 = data.iter().sum();

// B. Strided: touch one f64 per cache line (stride 8 × f64 = 64 B)
let sum: f64 = (0..data.len()).step_by(8).map(|i| data[i]).sum();

// C. Random: indices pre-shuffled (pointer-chasing's stunt double)
let sum: f64 = shuffled_idx.iter().map(|&i| data[i]).sum();
```

```
A. sequential   ~0.06 s    ~8 GB/s of useful data, latency fully hidden
B. stride-64    ~0.45 s    same line count as A! but only 1/8 of each line used
C. random       ~4.8 s     every access ≈ full DRAM latency, prefetcher useless
```

Readings: **A vs B** — B fetches the *same number of cache lines* as A (every line of the array) yet is ~8× slower in useful throughput: the cost was the lines, and B wasted 7/8 of each. Wasted-line-fraction made visible. **B vs C** — same "one useful f64 per line," but B's regular stride lets the prefetcher pipeline the fetches while C's unpredictability serializes them: predictability alone is worth another ~10×. **A vs C** — the full spread: ~80× for the same O(n) sum.

The `perf stat` signatures that would route you here from a real profile ([the funnel](../profiling-and-measurement/learning.md), stage 3):

```
A: IPC ~2.5   LLC-misses low     (bandwidth-bound at worst)
C: IPC ~0.15  LLC-misses ~1/access, dTLB-misses high   (latency-bound: the memory wall)
```

And the staircase, from the same benchmark run at increasing sizes (sequential): flat to 128 KB (L1-resident on this M-class L1), small step to ~1 MB (L2), step to ~32 MB (L3), cliff after. Every size a benchmark quotes is implicitly a statement about which step it measured.

## Applying It

Rust-specific practice, roughly in order of leverage:

- **`Vec` (and slices) are the default data structure; everything else must justify itself.** `Vec` iteration is sequential, prefetchable, line-dense. `LinkedList` is a per-element heap hop (the doc's C case) — in Rust it's essentially never right. `HashMap` lookups are necessary jumps, but *iteration* over a HashMap is randomized scatter; if you iterate often, keep a `Vec` beside it (or `IndexMap` for insertion-order density). `BTreeMap` sits between: node-clustered, decent scans, jumpy point lookups.
- **Traverse in memory order.** For nested data: iterate the contiguous axis innermost (row-major means rows inner). For 2-D work over `Vec<Vec<T>>`, note each inner `Vec` is a separate allocation — a flat `Vec<T>` with manual indexing (`y * width + x`, or the `ndarray` crate) restores contiguity across the whole matrix and is the standard fix.
- **Replace pointer graphs with index arenas.** `Box`/`Rc` object graphs scatter nodes across the heap; the arena idiom — nodes in a `Vec<Node>`, edges as `u32` indices — clusters them, shrinks references by half (4-byte index vs 8-byte pointer), and as a bonus dissolves most borrow-checker fights in graph code (`petgraph`, `slotmap`, `typed-arena` are the ecosystem forms). This is the single highest-yield locality refactor in typical Rust code.
- **Shrink the working set.** Smaller types (`u32` where `u64` was habit), `#[repr]`-aware field ordering, and hot/cold struct splitting — the [memory layout](../memory-layout/learning.md) doc's territory; from this doc's view, every byte shaved is line capacity reclaimed.
- **Do all the work while it's hot (temporal locality).** Fuse passes: three separate loops over the same big `Vec` load it three times; one loop loads it once (Rust iterator chains fuse *by construction* — `map(...).filter(...).sum()` is one pass, a genuine zero-cost-abstraction win). For work that can't fuse, **tile**: process in L2-sized chunks and run all phases per chunk before moving on.
- **Batch and sort to convert random into sequential.** If you must hit many scattered keys, sort the keys first and sweep — one pass of sequential-ish access beats N random probes; databases have organized around this for fifty years, and it applies verbatim to in-memory work with a sort costing O(n log n) cheap sequential passes.
- **Pre-size allocations** (`Vec::with_capacity`): reallocation churn scatters what would have been one contiguous block and doubles peak traffic during growth.

## When It Hurts

- **The working set already fits.** If everything lives in L1/L2, layout heroics change nothing measurable — the flat part of the staircase. Confirm which step you're on (size sweep, miss counters) before investing; this is the most common wasted locality effort.
- **The code is compute-bound.** IPC ~3–4 with low miss rates means the core is busy, not waiting; the win lives in [SIMD](../simd/learning.md) or algorithm, not layout.
- **Tiling and flattening cost clarity.** A tiled matrix kernel or hand-flattened structure is real complexity; pay it where the profile says memory-bound, not by default. The readable version with a `// tile this if it shows up` comment is often the right v1.
- **SoA can hurt single-entity access.** Structure-of-arrays wins whole-collection sweeps and loses "load every field of entity #4217" (now N scattered arrays instead of one line) — the trade is workload-shaped; [data-oriented design](../data-oriented-design/learning.md) covers choosing.
- **Algorithms outrank locality.** A hash lookup's O(1) random jump beats a cache-perfect O(n) scan *once n is large enough* — locality bends constants, not asymptotics. (But the crossover sits later than intuition says: linear scan of a small `Vec` beats `HashMap` up to tens of elements — measure at your sizes.)

## Benchmarking Methodology

- **Counters that name the problem:** `perf stat -e cache-references,cache-misses,LLC-load-misses,dTLB-load-misses,instructions,cycles`. Low IPC + high LLC misses = latency-bound (this doc); high bandwidth utilization + moderate IPC = bandwidth-bound (waste less per line); dTLB misses high = add huge pages to the conversation. On macOS, `cachegrind` fills the counter gap with deterministic simulation — ideal for A/B-ing a layout change, blind to prefetching (it models none).
- **The size sweep is the diagnostic instrument:** run the identical benchmark from 16 KB to 1 GB working set, plot ns/element — the staircase locates your cache boundaries and tells you which level any other benchmark was really measuring. Do it once per machine; keep the plot.
- **Defeat the prefetcher when measuring latency** (shuffle the access order — the B-vs-C move); *include* it when measuring your real pattern. Know which one your benchmark is.
- **Beware the warm-cache mirage:** criterion's iteration loop re-touches the same data — a "working set" of 4 MB measured hot in L3 says nothing about production cold paths. Either size the benchmark data past L3 or explicitly flush/rotate buffers between iterations.
- **Watch for conflict-miss artifacts** at power-of-two sizes/strides (the set-associativity mechanism): if 4096 is mysteriously slower than 4100, you found one; pad the leading dimension.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Same number of cache lines fetched, 8× throughput difference — reconstruct the A-vs-B mechanism and name the quantity it isolates.
2. Why does pointer chasing cost ~50–100× rather than the ~8× of stride waste? Which hardware helper is defeated, and what property of the address stream defeats it?
3. Your profile: IPC 0.2, LLC-miss per access ~1, dTLB misses high. Diagnose, list the three fixes in leverage order, and say what you'd check *before* any of them (which staircase step is the workload on?).
4. A `HashMap<u64, Entry>` is iterated fully every frame and point-looked-up rarely. What's wrong with this picture, and what's the fix?
5. Why do Rust iterator chains constitute a locality optimization, mechanically?
6. When is converting a `Box`-graph to an index arena *not* worth it? Two distinct reasons.

Measurement exercises:

- Reproduce the worked example (A/B/C at 512 MB) on your machine; then run A as a size sweep 16 KB → 1 GB and plot the staircase. Label each step with your actual L1/L2/L3 sizes (`sysctl hw.l1dcachesize hw.l2cachesize` on macOS). Keep the plot — it's the calibration card for every future benchmark.
- Measure the arena effect: build a 1M-node binary tree twice — `Box` nodes vs. `Vec<Node>` + `u32` indices — and time identical traversals. Then sort the arena in traversal order and measure again (the layout-matches-access bonus).
- Find your linear-scan/HashMap crossover: `Vec` scan vs `HashMap` lookup at n = 4, 16, 64, 256, 1024. The number you find recalibrates "O(1) is faster" forever.

## Open Questions

- Apple M-series specifics: confirm 128-byte lines' effect on the stride experiment (does the B case need stride-16?), and what prefetcher behavior differs from x86 — measure, don't assume.
- Software prefetch intrinsics (`_mm_prefetch` / `core::arch` equivalents): do they ever beat hardware prefetching on modern cores for real workloads (e.g. hash-join style batched lookups), or is batch+sort always the better answer in Rust?
- Huge pages from Rust on Linux (`madvise`/THP) and on macOS: how much do they move a dTLB-heavy workload in practice?
- cachegrind vs. real counters on the same A/B: how far off is the simulation when prefetching matters, quantified once?
- Where does `BTreeMap`'s node size (B=6 in std) sit relative to line size, and would a line-tuned B-tree (per `abseil`'s btree design) measurably win for hot scans?

## References

- Ulrich Drepper, *What Every Programmer Should Know About Memory* — the classic deep treatment (2007; the numbers aged, the mechanisms didn't). Parts 2–4 (caches) and 6 (what programmers can do) are the payload.
- ["Latency Numbers Every Programmer Should Know"](https://colin-scott.github.io/personal_website/research/interactive_latency.html) — the interactive per-year version; internalize the ratios.
- Agner Fog, [*The microarchitecture of Intel, AMD and VIA CPUs*](https://www.agner.org/optimize/) — the reference for cache/prefetch specifics per core generation, when a mystery needs the actual hardware manual.
- Nicholas Nethercote, [The Rust Performance Book](https://nnethercote.github.io/perf-book/) — the Rust-side catalog (collections, allocation, iterators) that pairs with this doc's mechanisms.
- Chandler Carruth, "Efficiency with Algorithms, Performance with Data Structures" (CppCon 2014) — the arrays-over-node-structures argument delivered memorably; directly transfers to Rust.
- Related topics in this repo: [Profiling & Measurement](../profiling-and-measurement/learning.md) (the counter signatures that route here), [Memory Layout](../memory-layout/learning.md) (shrinking and shaping what fills the lines), [Data-Oriented Design](../data-oriented-design/learning.md) (organizing whole programs around these mechanisms), [False Sharing](../false-sharing/learning.md) (what happens to lines when cores share them), [NUMA Awareness](../numa-awareness/learning.md) (when DRAM itself has neighborhoods).
