# System Architecture Reference

A personal reference library covering:

- **Architecture patterns** — what they are, where they break in production, and how to handle it.
- **Open source tools** — especially open-source alternatives to vendor/licensed tools, how they compare, and what to watch for when adopting them.
- **Language best practices** — per-language conventions, idioms, and anti-patterns. Rust is the primary focus.
- **Performance optimization** — techniques for building high-performance applications, applied primarily through Rust.

Every topic folder has two docs, scaffolded from its category's templates:

- **learning.md** — study material: mental models, mechanisms, worked examples, the why behind everything.
- **reference.md** — cheat sheet: tables, checklists, rules of thumb — scannable in under a minute.

## Architecture Patterns

Templates: [_template-learning.md](architecture-patterns/_template-learning.md) · [_template-reference.md](architecture-patterns/_template-reference.md)

- Event Sourcing & CQRS — [learning](architecture-patterns/event-sourcing/learning.md) · [reference](architecture-patterns/event-sourcing/reference.md)
- Event-Driven Architecture — [learning](architecture-patterns/event-driven-architecture/learning.md) · [reference](architecture-patterns/event-driven-architecture/reference.md)
- Saga Pattern — [learning](architecture-patterns/saga-pattern/learning.md) · [reference](architecture-patterns/saga-pattern/reference.md)
- Outbox Pattern — [learning](architecture-patterns/outbox-pattern/learning.md) · [reference](architecture-patterns/outbox-pattern/reference.md)
- Circuit Breaker — [learning](architecture-patterns/circuit-breaker/learning.md) · [reference](architecture-patterns/circuit-breaker/reference.md)
- Strangler Fig — [learning](architecture-patterns/strangler-fig/learning.md) · [reference](architecture-patterns/strangler-fig/reference.md)
- Sharding — [learning](architecture-patterns/sharding/learning.md) · [reference](architecture-patterns/sharding/reference.md)
- Caching Strategies — [learning](architecture-patterns/caching-strategies/learning.md) · [reference](architecture-patterns/caching-strategies/reference.md)

## OSS Tools

Templates: [_template-learning.md](oss-tools/_template-learning.md) · [_template-reference.md](oss-tools/_template-reference.md)

- OpenBao (alternative to Vault) — [learning](oss-tools/openbao/learning.md) · [reference](oss-tools/openbao/reference.md)
- OpenTofu (alternative to Terraform) — [learning](oss-tools/opentofu/learning.md) · [reference](oss-tools/opentofu/reference.md)

## Language Best Practices

Templates: [_template-learning.md](language-best-practices/_template-learning.md) · [_template-reference.md](language-best-practices/_template-reference.md)

- Rust — [learning](language-best-practices/rust/learning.md) · [reference](language-best-practices/rust/reference.md)

## Performance Optimization

Templates: [_template-learning.md](performance-optimization/_template-learning.md) · [_template-reference.md](performance-optimization/_template-reference.md)

Roughly in study order — measurement first, then hardware fundamentals, then techniques built on them:

- Profiling & Measurement — [learning](performance-optimization/profiling-and-measurement/learning.md) · [reference](performance-optimization/profiling-and-measurement/reference.md)
- Cache Locality — [learning](performance-optimization/cache-locality/learning.md) · [reference](performance-optimization/cache-locality/reference.md)
- Memory Layout — [learning](performance-optimization/memory-layout/learning.md) · [reference](performance-optimization/memory-layout/reference.md)
- Branch Prediction — [learning](performance-optimization/branch-prediction/learning.md) · [reference](performance-optimization/branch-prediction/reference.md)
- Data-Oriented Design — [learning](performance-optimization/data-oriented-design/learning.md) · [reference](performance-optimization/data-oriented-design/reference.md)
- Allocation Strategies — [learning](performance-optimization/allocation-strategies/learning.md) · [reference](performance-optimization/allocation-strategies/reference.md)
- SIMD — [learning](performance-optimization/simd/learning.md) · [reference](performance-optimization/simd/reference.md)
- Zero-Copy — [learning](performance-optimization/zero-copy/learning.md) · [reference](performance-optimization/zero-copy/reference.md)
- False Sharing — [learning](performance-optimization/false-sharing/learning.md) · [reference](performance-optimization/false-sharing/reference.md)
- Lock-Free Concurrency — [learning](performance-optimization/lock-free-concurrency/learning.md) · [reference](performance-optimization/lock-free-concurrency/reference.md)
- Async & I/O — [learning](performance-optimization/async-and-io/learning.md) · [reference](performance-optimization/async-and-io/reference.md)
- Compiler Optimizations — [learning](performance-optimization/compiler-optimizations/learning.md) · [reference](performance-optimization/compiler-optimizations/reference.md)
