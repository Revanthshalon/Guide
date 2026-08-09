# Performance Optimization — Learning Index

The order to read this category in, derived from what the docs actually depend on. The shape is: **measure first, then the hardware mechanisms, then the techniques built on them, then concurrency, then the compiler.**

One rule the whole category assumes: you never apply anything here without a profile pointing at it. That's why #1 is #1.

## The order

| # | Topic | Depends on | Why here |
| --- | --- | --- | --- |
| 1 | [Profiling & Measurement](profiling-and-measurement/learning.md) | — | The gate. Every other doc consumes it, and its counter signatures are what route you to #2–4. Reading anything else first means optimizing by folklore. |
| 2 | [Cache Locality](cache-locality/learning.md) | 1 | The cache line as the unit of transfer — the single mechanism most of this category is downstream of. |
| 3 | [Memory Layout](memory-layout/learning.md) | 1, 2 | Shrinking and shaping what fills those lines: field order, padding, niches, AoS vs. SoA. |
| 4 | [Branch Prediction](branch-prediction/learning.md) | 1, 2 | The other hardware mechanism a profile routes to. Batch-and-sort serves this and #2 at once. |
| 5 | [Data-Oriented Design](data-oriented-design/learning.md) | 2, 3, 4 | Not a new mechanism — #2 + #3 + #4 composed at program scale. Read it once the three below it are instinct, not before. |
| 6 | [Allocation Strategies](allocation-strategies/learning.md) | 1, 2, 3, 5 | The most common finding in an idiomatic-Rust flamegraph. Arenas and handles are #5 as architecture; size classes quantize #3's structs. |
| 7 | [SIMD](simd/learning.md) | 3, 4, 5 | Stage 4 of data-oriented design. Masks are branch prediction's doctrine; SoA and alignment are prerequisites, not optimizations. |
| 8 | [Batching & Amortization](batching-and-amortization/learning.md) | 6, 7 | Fixed-cost-vs-marginal-cost as a general lever. Forward-refers to #12 and #15, but they read better with this in hand. |
| 9 | [Zero-Copy](zero-copy/learning.md) | 2, 6, 8 | Borrow-don't-own: copies carry allocations and burn the bandwidth #2 priced. |
| 10 | [Serialization & Encoding](serialization-and-encoding/learning.md) | 4, 6, 7, 8, 9 | The boundary where all five above land at once — why text parsing is slow, and what zero-copy formats actually buy. |
| 11 | [False Sharing](false-sharing/learning.md) | 2, 3, 8 | The coherence sequel to #2: the same cache line, now written by two cores. Note the density rules from #2 *invert* for written data. |
| 12 | [Parallelism & Work Stealing](parallelism-and-work-stealing/learning.md) | 2, 5, 8, 11 | Scaling out only after you know what #11 does to naive parallel sweeps. Granularity is the knee; fold/reduce is the structural fix. |
| 13 | [Lock-Free Concurrency](lock-free-concurrency/learning.md) | 8, 11, 12 | The deep end. Read only after #11–12 — the coherence economics are the whole cost model, and the work-stealing deque is the flagship win. |
| 14 | [NUMA Awareness](numa-awareness/learning.md) | 2, 6, 11, 12 | DRAM was never one place. Pinning vs. stealing is the trade #12 set up; arena-per-worker from #6 ages well here. |
| 15 | [Async & I/O](async-and-io/learning.md) | 3, 8, 9, 12 | The other half of #12: waiting well instead of computing fast. Futures are state-machine enums (#3); io_uring is batched syscalls (#8). |
| 16 | [Compiler Optimizations](compiler-optimizations/learning.md) | all of 1–15 | Last by design: it's the amplifier for everything before it, and its most useful lesson is what the compiler can *never* do for you — which is why the other fifteen exist. |

## Shorter paths

- **Single-threaded hot loop is slow:** 1 → 2 → 3 → 4, then 6 if the profile says allocation, 7 if it's a tight numeric kernel.
- **Throughput doesn't scale with cores:** 1 → 2 → 11 → 12, then 14 on a multi-socket box.
- **Service is I/O-bound, not CPU-bound:** 1 → 8 → 9 → 15 (and [Backpressure & Rate Limiting](../architecture-patterns/backpressure-and-rate-limiting/learning.md) for the architecture-side half).
- **Serialization is on the flamegraph:** 1 → 9 → 10.

## Pairs that should be read together

- [Cache Locality](cache-locality/learning.md) + [False Sharing](false-sharing/learning.md) — the same cache line, read-shared vs. write-shared; the second inverts the first's advice.
- [Parallelism & Work Stealing](parallelism-and-work-stealing/learning.md) + [Async & I/O](async-and-io/learning.md) — computing fast vs. waiting well; the runtime boundary between them is where real systems break.
- [Memory Layout](memory-layout/learning.md) + [SIMD](simd/learning.md) — SoA and alignment are the prerequisite, not the payoff.
- [Zero-Copy](zero-copy/learning.md) + [Serialization & Encoding](serialization-and-encoding/learning.md) — the borrow regime only pays off at a boundary.

## Where this category meets architecture

Same ideas, different scale — see the matching table in [architecture-patterns/LEARNING-INDEX.md](../architecture-patterns/LEARNING-INDEX.md).

| Performance | Architecture counterpart | The shared idea |
| --- | --- | --- |
| [NUMA Awareness](numa-awareness/learning.md), [False Sharing](false-sharing/learning.md) | [Sharding](../architecture-patterns/sharding/learning.md) | Partition so owners don't contend |
| [Cache Locality](cache-locality/learning.md) | [Caching Strategies](../architecture-patterns/caching-strategies/learning.md) | Same policies, silicon instead of config |
| [Batching & Amortization](batching-and-amortization/learning.md) | [Outbox](../architecture-patterns/outbox-pattern/learning.md), [Backpressure](../architecture-patterns/backpressure-and-rate-limiting/learning.md) | Relay batches, bounded queues, batch-failure semantics |
| [Lock-Free Concurrency](lock-free-concurrency/learning.md) | [Event Sourcing & CQRS](../architecture-patterns/event-sourcing/learning.md) | CAS and `expected_version` are one idea |
| [Serialization & Encoding](serialization-and-encoding/learning.md) | [Event-Driven Architecture](../architecture-patterns/event-driven-architecture/learning.md) | Schema evolution as the real contract |
