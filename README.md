# System Architecture Reference

A personal reference library covering:

- **Architecture patterns** — what they are, where they break in production, and how to handle it.
- **Open source tools** — especially open-source alternatives to vendor/licensed tools, how they compare, and what to watch for when adopting them.
- **Language best practices** — per-language conventions, idioms, and anti-patterns. Rust is the primary focus.
- **Performance optimization** — techniques for building high-performance applications, applied primarily through Rust.
- **Data structures & algorithms** — the structures and paradigms themselves: invariants, complexity, Rust implementations, and when to reach for which.

Every topic folder has two docs, scaffolded from its category's templates:

- **learning.md** — study material: mental models, mechanisms, worked examples, the why behind everything.
- **reference.md** — cheat sheet: tables, checklists, rules of thumb — scannable in under a minute.

## Architecture Patterns

Templates: [_template-learning.md](architecture-patterns/_template-learning.md) · [_template-reference.md](architecture-patterns/_template-reference.md)

**Study order: [LEARNING-INDEX.md](architecture-patterns/LEARNING-INDEX.md)** — the sequence to read these in, with prerequisites. The grouping below is by theme, not by order.

Distributed-systems fundamentals:

- Replication & Consistency Models — [learning](architecture-patterns/replication-and-consistency/learning.md) · [reference](architecture-patterns/replication-and-consistency/reference.md)
- Consensus & Leader Election — [learning](architecture-patterns/consensus-and-leader-election/learning.md) · [reference](architecture-patterns/consensus-and-leader-election/reference.md)
- Idempotency & Delivery Semantics — [learning](architecture-patterns/idempotency-and-delivery-semantics/learning.md) · [reference](architecture-patterns/idempotency-and-delivery-semantics/reference.md)
- Sharding — [learning](architecture-patterns/sharding/learning.md) · [reference](architecture-patterns/sharding/reference.md)

Integration & data-flow patterns:

- Event-Driven Architecture — [learning](architecture-patterns/event-driven-architecture/learning.md) · [reference](architecture-patterns/event-driven-architecture/reference.md)
- Event Sourcing & CQRS — [learning](architecture-patterns/event-sourcing/learning.md) · [reference](architecture-patterns/event-sourcing/reference.md)
- Saga Pattern — [learning](architecture-patterns/saga-pattern/learning.md) · [reference](architecture-patterns/saga-pattern/reference.md)
- Outbox Pattern — [learning](architecture-patterns/outbox-pattern/learning.md) · [reference](architecture-patterns/outbox-pattern/reference.md)
- Change Data Capture — [learning](architecture-patterns/change-data-capture/learning.md) · [reference](architecture-patterns/change-data-capture/reference.md)
- Caching Strategies — [learning](architecture-patterns/caching-strategies/learning.md) · [reference](architecture-patterns/caching-strategies/reference.md)

Security:

- Encryption & Key Management — [learning](architecture-patterns/encryption-and-key-management/learning.md) · [reference](architecture-patterns/encryption-and-key-management/reference.md)

Resilience & traffic management:

- Circuit Breaker — [learning](architecture-patterns/circuit-breaker/learning.md) · [reference](architecture-patterns/circuit-breaker/reference.md)
- Backpressure & Rate Limiting — [learning](architecture-patterns/backpressure-and-rate-limiting/learning.md) · [reference](architecture-patterns/backpressure-and-rate-limiting/reference.md)
- Load Balancing & Service Discovery — [learning](architecture-patterns/load-balancing-and-service-discovery/learning.md) · [reference](architecture-patterns/load-balancing-and-service-discovery/reference.md)

Migration:

- Strangler Fig — [learning](architecture-patterns/strangler-fig/learning.md) · [reference](architecture-patterns/strangler-fig/reference.md)

## OSS Tools

Templates: [_template-learning.md](oss-tools/_template-learning.md) · [_template-reference.md](oss-tools/_template-reference.md) · [_template-runbook.md](oss-tools/_template-runbook.md)

Tools you actually operate also get a **runbook**: annotated configs, ordered ceremonies, day-2 procedures, and a dev→production checklist.

- OpenBao (alternative to Vault) — [learning](oss-tools/openbao/learning.md) · [reference](oss-tools/openbao/reference.md) · [runbook](oss-tools/openbao/runbook.md)
- OpenTofu (alternative to Terraform) — [learning](oss-tools/opentofu/learning.md) · [reference](oss-tools/opentofu/reference.md) · [runbook](oss-tools/opentofu/runbook.md)

## Language Best Practices

Templates: [_template-learning.md](language-best-practices/_template-learning.md) · [_template-reference.md](language-best-practices/_template-reference.md)

- Rust — [learning](language-best-practices/rust/learning.md) · [reference](language-best-practices/rust/reference.md)

## Performance Optimization

Templates: [_template-learning.md](performance-optimization/_template-learning.md) · [_template-reference.md](performance-optimization/_template-reference.md)

**Study order: [LEARNING-INDEX.md](performance-optimization/LEARNING-INDEX.md)** — the sequence below, with prerequisites and shorter paths for specific symptoms.

In study order — measurement first, then hardware fundamentals, then techniques built on them, then concurrency, then the compiler:

- Profiling & Measurement — [learning](performance-optimization/profiling-and-measurement/learning.md) · [reference](performance-optimization/profiling-and-measurement/reference.md)
- Cache Locality — [learning](performance-optimization/cache-locality/learning.md) · [reference](performance-optimization/cache-locality/reference.md)
- Memory Layout — [learning](performance-optimization/memory-layout/learning.md) · [reference](performance-optimization/memory-layout/reference.md)
- Branch Prediction — [learning](performance-optimization/branch-prediction/learning.md) · [reference](performance-optimization/branch-prediction/reference.md)
- Data-Oriented Design — [learning](performance-optimization/data-oriented-design/learning.md) · [reference](performance-optimization/data-oriented-design/reference.md)
- Allocation Strategies — [learning](performance-optimization/allocation-strategies/learning.md) · [reference](performance-optimization/allocation-strategies/reference.md)
- SIMD — [learning](performance-optimization/simd/learning.md) · [reference](performance-optimization/simd/reference.md)
- Batching & Amortization — [learning](performance-optimization/batching-and-amortization/learning.md) · [reference](performance-optimization/batching-and-amortization/reference.md)
- Zero-Copy — [learning](performance-optimization/zero-copy/learning.md) · [reference](performance-optimization/zero-copy/reference.md)
- Serialization & Encoding — [learning](performance-optimization/serialization-and-encoding/learning.md) · [reference](performance-optimization/serialization-and-encoding/reference.md)
- False Sharing — [learning](performance-optimization/false-sharing/learning.md) · [reference](performance-optimization/false-sharing/reference.md)
- Parallelism & Work Stealing — [learning](performance-optimization/parallelism-and-work-stealing/learning.md) · [reference](performance-optimization/parallelism-and-work-stealing/reference.md)
- Lock-Free Concurrency — [learning](performance-optimization/lock-free-concurrency/learning.md) · [reference](performance-optimization/lock-free-concurrency/reference.md)
- NUMA Awareness — [learning](performance-optimization/numa-awareness/learning.md) · [reference](performance-optimization/numa-awareness/reference.md)
- Async & I/O — [learning](performance-optimization/async-and-io/learning.md) · [reference](performance-optimization/async-and-io/reference.md)
- Compiler Optimizations — [learning](performance-optimization/compiler-optimizations/learning.md) · [reference](performance-optimization/compiler-optimizations/reference.md)

## Data Structures & Algorithms

Templates: [_template-learning.md](data-structures-and-algorithms/_template-learning.md) · [_template-reference.md](data-structures-and-algorithms/_template-reference.md)

**Full curriculum: [LEARNING-INDEX.md](data-structures-and-algorithms/LEARNING-INDEX.md)** — every topic needed for mastery across 11 stages, with prerequisites, shorter paths by goal, and the transformation lenses each doc closes with. Topic folders are scaffolded as they're started; the index is the roadmap.

Nothing scaffolded yet — start at Stage 0 (`complexity-analysis/`, `rust-for-data-structures/`).

## Deferred Topics

Deliberately not scaffolded yet — pick these up after the current sets are learned. When starting one, scaffold it from the category's templates and move it into the index above.

| Topic | Target category | Why deferred | Pick up when |
| --- | --- | --- | --- |
| Distributed Tracing & Observability | architecture-patterns | Leans operations more than architecture | Running a multi-service system and debugging cross-service latency/failures |
| API Gateway & BFF | architecture-patterns | Infrastructure choice more than a pattern | Exposing multiple services to multiple client types |
| Service Mesh | architecture-patterns | Infrastructure choice; assumes microservices maturity | Cross-cutting concerns (mTLS, retries, traffic shifting) outgrow per-service code |
| Formal Methods (TLA+) | architecture-patterns | Only pays off when verifying non-trivial designs | Designing custom consensus/replication logic where bugs are catastrophic |
| GPU Compute | performance-optimization | Different discipline from CPU performance | A workload is provably compute-bound beyond what SIMD + parallelism deliver |
