# Connection Pooling and C10K

> 6 nodes · cohesion 0.33

## Key Concepts

- **The C10K Problem (Thread-Per-Connection Rent)** (4 connections) — `performance-optimization/async-and-io/learning.md`
- **Process-Per-Connection Model** (3 connections) — `oss-tools/postgres/learning.md`
- **Connection Pooling with PgBouncer (Not Optional)** (3 connections) — `oss-tools/postgres/runbook.md`
- **Pitfall: Connection Exhaustion** (2 connections) — `oss-tools/postgres/learning.md`
- **Hoisting — Pay the Fixed Cost Once** (2 connections) — `performance-optimization/batching-and-amortization/learning.md`
- **When Threads Win Anyway** (1 connections) — `performance-optimization/async-and-io/learning.md`

## Relationships

- [Async Runtimes and Allocators](Async_Runtimes_and_Allocators.md) (2 shared connections)
- [PostgreSQL Storage and Planner](PostgreSQL_Storage_and_Planner.md) (1 shared connections)
- [Performance Doc Templates](Performance_Doc_Templates.md) (1 shared connections)
- [Allocation Levers and Batching](Allocation_Levers_and_Batching.md) (1 shared connections)

## Source Files

- `oss-tools/postgres/learning.md`
- `oss-tools/postgres/runbook.md`
- `performance-optimization/async-and-io/learning.md`
- `performance-optimization/batching-and-amortization/learning.md`

## Audit Trail

- EXTRACTED: 7 (70%)
- INFERRED: 3 (30%)
- AMBIGUOUS: 0 (0%)

---

*Part of the graphify knowledge wiki. See [index](index.md) to navigate.*