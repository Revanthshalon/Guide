# Allocation Levers and Batching

> 10 nodes · cohesion 0.20

## Key Concepts

- **Four Levers: Allocate Less, Once, Together, Elsewhere** (7 connections) — `performance-optimization/allocation-strategies/learning.md`
- **N × (F + m) vs. F + N × m** (4 connections) — `performance-optimization/batching-and-amortization/learning.md`
- **with_capacity — Allocate Once** (2 connections) — `performance-optimization/allocation-strategies/learning.md`
- **The Knee: N ≈ F/m Sizing Formula** (2 connections) — `performance-optimization/batching-and-amortization/learning.md`
- **Size-or-Time Flush Shape (N items or T ms)** (2 connections) — `performance-optimization/batching-and-amortization/learning.md`
- **Buffer Reuse via clear() Keeping Capacity** (1 connections) — `performance-optimization/allocation-strategies/learning.md`
- **Swapping the Global Allocator (jemalloc / mimalloc)** (1 connections) — `performance-optimization/allocation-strategies/learning.md`
- **Inline Small Storage (SmallVec / ArrayVec / CompactStr)** (1 connections) — `performance-optimization/allocation-strategies/learning.md`
- **Accidental-Allocation Lint List (Hot Loops)** (1 connections) — `performance-optimization/allocation-strategies/reference.md`
- **A Batch Is a Shared Fate** (1 connections) — `performance-optimization/batching-and-amortization/learning.md`

## Relationships

- [Performance Doc Templates](Performance_Doc_Templates.md) (1 shared connections)
- [Cache Lines and Working Sets](Cache_Lines_and_Working_Sets.md) (1 shared connections)
- [Connection Pooling and C10K](Connection_Pooling_and_C10K.md) (1 shared connections)
- [Async Runtimes and Allocators](Async_Runtimes_and_Allocators.md) (1 shared connections)

## Source Files

- `performance-optimization/allocation-strategies/learning.md`
- `performance-optimization/allocation-strategies/reference.md`
- `performance-optimization/batching-and-amortization/learning.md`

## Audit Trail

- EXTRACTED: 11 (85%)
- INFERRED: 2 (15%)
- AMBIGUOUS: 0 (0%)

---

*Part of the graphify knowledge wiki. See [index](index.md) to navigate.*