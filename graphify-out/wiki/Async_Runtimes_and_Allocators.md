# Async Runtimes and Allocators

> 11 nodes · cohesion 0.18

## Key Concepts

- **Readiness Models — epoll / kqueue** (4 connections) — `performance-optimization/async-and-io/learning.md`
- **The Fixed-Cost Ladder** (4 connections) — `performance-optimization/batching-and-amortization/learning.md`
- **Blocking the Runtime — The Cardinal Sin** (3 connections) — `performance-optimization/async-and-io/learning.md`
- **async fn as a State-Machine Enum** (3 connections) — `performance-optimization/async-and-io/learning.md`
- **Allocator Size Classes and Per-Thread Caches** (2 connections) — `performance-optimization/allocation-strategies/learning.md`
- **Cancellation Safety (Drop at Any Await)** (2 connections) — `performance-optimization/async-and-io/learning.md`
- **Executor, Reactor, and Waker** (2 connections) — `performance-optimization/async-and-io/learning.md`
- **Completion Model — io_uring** (2 connections) — `performance-optimization/async-and-io/learning.md`
- **The Cardinal Hazards of Async** (2 connections) — `performance-optimization/async-and-io/reference.md`
- **Poll-Time Histogram (tokio-console) as the Flamegraph of Async** (1 connections) — `performance-optimization/async-and-io/learning.md`
- **spawn_blocking and the Rayon Bridge** (1 connections) — `performance-optimization/async-and-io/learning.md`

## Relationships

- [Performance Doc Templates](Performance_Doc_Templates.md) (2 shared connections)
- [Connection Pooling and C10K](Connection_Pooling_and_C10K.md) (2 shared connections)
- [Allocation Levers and Batching](Allocation_Levers_and_Batching.md) (1 shared connections)
- [PostgreSQL Storage and Planner](PostgreSQL_Storage_and_Planner.md) (1 shared connections)

## Source Files

- `performance-optimization/allocation-strategies/learning.md`
- `performance-optimization/async-and-io/learning.md`
- `performance-optimization/async-and-io/reference.md`
- `performance-optimization/batching-and-amortization/learning.md`

## Audit Trail

- EXTRACTED: 12 (75%)
- INFERRED: 4 (25%)
- AMBIGUOUS: 0 (0%)

---

*Part of the graphify knowledge wiki. See [index](index.md) to navigate.*