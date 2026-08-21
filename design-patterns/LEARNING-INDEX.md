# Design Patterns — Learning Index

The order to read this category in. It's derived from what the docs actually depend on: each entry's prerequisites are all *above* it, so no doc sends you forward for vocabulary you don't have yet.

Read `learning.md` top-to-bottom for each; the matching `reference.md` is for later, when you're implementing.

## The order

| # | Topic | Depends on | Why here |
| --- | --- | --- | --- |
| 1 | [Strategy](strategy/learning.md) | — | Simplest behavioral pattern; introduces "program to a trait" — the idea every subsequent pattern leans on. |
| 2 | [Enum Dispatch](enum-dispatch/learning.md) | 1 | The Rust-native alternative to Strategy. Read immediately after: when enums beat trait objects, and the performance/extensibility trade-off that governs the choice. |
| 3 | [Builder](builder/learning.md) | 1 | First creational pattern; uses the trait-based thinking from #1. Introduces compile-time construction guarantees via type state (a preview of #11). |
| 4 | [Factory Method & Abstract Factory](factory/learning.md) | 1, 3 | Generalizes Builder's creation concern: who decides *which* concrete type to build, and how to defer that decision. |
| 5 | [Newtype & Zero-Cost Abstractions](newtype-and-zero-cost/learning.md) | 1 | Rust-specific; wrapping a type for safety with no runtime cost. Used pervasively from here on — knowing it early makes every subsequent Rust example cleaner. |
| 6 | [Iterator](iterator/learning.md) | 1, 5 | Rust's `Iterator` trait *is* the pattern. Grounds the trait-based approach in something you already use daily, and shows how Newtype wrappers compose with it. |
| 7 | [Observer & Publish-Subscribe](observer/learning.md) | 1, 6 | Introduces the callback/subscription model. Connects to [Event-Driven Architecture](../architecture-patterns/event-driven-architecture/learning.md) at the distributed scale. |
| 8 | [Command](command/learning.md) | 1, 7 | Reifies a request as an object — combines Strategy's dispatch with Observer's decoupling. The local-process version of [Event Sourcing](../architecture-patterns/event-sourcing/learning.md). |
| 9 | [Memento](memento/learning.md) | 8 | Captures and restores state — pairs with Command for undo/redo. The local version of snapshots in [Event Sourcing](../architecture-patterns/event-sourcing/learning.md). |
| 10 | [State Machine](state-machine/learning.md) | 1, 2, 8 | Reifies state transitions. Heavy Rust coverage with enums; the [Circuit Breaker](../architecture-patterns/circuit-breaker/learning.md) is an architecture-scale instance. |
| 11 | [Type State](type-state/learning.md) | 3, 5, 10 | Compile-time state enforcement — the Rust version of the State pattern. Moves the state machine from runtime to the type system: invalid transitions don't compile. |
| 12 | [Decorator](decorator/learning.md) | 1, 6 | Wrapping to add behavior. `tower` layers are decorators; this is the pattern behind middleware stacks in [Async & I/O](../performance-optimization/async-and-io/learning.md). |
| 13 | [Adapter](adapter/learning.md) | 1 | Structural translation between incompatible interfaces. In Rust: trait impls over foreign types, `From`/`Into`. |
| 14 | [Extension Traits](extension-traits/learning.md) | 1, 6, 13 | The idiomatic Rust way to add methods to types you don't own — the Adapter pattern as Rust actually practices it. |
| 15 | [Facade](facade/learning.md) | 13 | Simplifying interface over a complex subsystem. In Rust: the `pub` boundary of a module is the facade. |
| 16 | [Composite](composite/learning.md) | 1, 6 | Recursive tree structures: treat a group the same as an individual. Ownership and `Box<dyn Trait>` vs enum are the Rust decisions. |
| 17 | [Proxy](proxy/learning.md) | 12, 13 | Controlled access — lazy initialization, access control, caching. Shares Decorator's wrapping structure but with a different intent. |
| 18 | [Bridge](bridge/learning.md) | 1, 13 | Separating an abstraction's interface from its implementation so both can vary independently. In Rust: trait + generic impl vs trait object. |
| 19 | [Template Method](template-method/learning.md) | 1, 14 | Default trait methods in Rust — the superclass hook pattern without inheritance. |
| 20 | [Chain of Responsibility](chain-of-responsibility/learning.md) | 1, 12 | Middleware pipelines: pass a request along a chain of handlers. The code-level pattern behind [Backpressure & Rate Limiting](../architecture-patterns/backpressure-and-rate-limiting/learning.md) middleware. |
| 21 | [Visitor](visitor/learning.md) | 2, 16 | Double dispatch — and why Rust's `enum` + `match` usually replaces it entirely. Understanding the pattern explains why you don't need it, and when you still do (plugin architectures). |
| 22 | [Mediator](mediator/learning.md) | 7, 8 | Centralized coordination replacing direct object-to-object coupling. |
| 23 | [Flyweight](flyweight/learning.md) | 5 | Shared immutable data: interning, `Rc`/`Arc`, arena allocation. Connects to [Allocation Strategies](../performance-optimization/allocation-strategies/learning.md). |
| 24 | [Singleton & Shared State](singleton-and-shared-state/learning.md) | 5, 23 | `OnceCell`, `LazyLock`, why global mutable state hurts, and the Rust-idiomatic alternatives. |
| 25 | [Prototype](prototype/learning.md) | 5 | `Clone` trait — Rust makes this trivial. Worth understanding when deep vs shallow clone matters and the `Clone` vs `Copy` distinction. |
| 26 | [RAII & Drop Guards](raii-and-drop-guards/learning.md) | 5, 11 | Resource management via ownership: `MutexGuard`, file handles, scope guards. The pattern that makes Rust's safety guarantees work. Connects to [Lock-Free Concurrency](../performance-optimization/lock-free-concurrency/learning.md). |
| 27 | [Marker Traits & Phantom Types](marker-traits-and-phantom-types/learning.md) | 5, 11, 26 | Compile-time constraints with zero runtime cost: `Send`, `Sync`, `PhantomData`. The type-system toolkit that enforces the contracts other patterns rely on. |
| 28 | [Repository & Unit of Work](repository-and-unit-of-work/learning.md) | 1, 4, 13 | Boundary patterns bridging domain logic and persistence. Connects to [Outbox Pattern](../architecture-patterns/outbox-pattern/learning.md) at the architecture scale. |
| 29 | [Dependency Injection](dependency-injection/learning.md) | 1, 4, 13, 28 | Last — synthesizes interface-based design from everything above. In Rust: generics + traits, not a framework. |

## Shorter paths

- **Just need Rust-specific patterns:** 1 → 2 → 5 → 11 → 14 → 26 → 27 — the patterns that originate from or are transformed by Rust's type system.
- **Just need the GoF behavioral patterns:** 1 → 7 → 8 → 9 → 10 → 20 → 21 → 22 — the communication and state patterns.
- **Just need to structure a Rust codebase:** 1 → 3 → 4 → 5 → 6 → 15 → 28 → 29 — the patterns that govern module and API shape.
- **Just need the wrapping/composition patterns:** 1 → 12 → 13 → 14 → 17 → 18 → 20 — Decorator, Adapter, Proxy, Bridge, Chain.

## Pairs that should be read together

- [Strategy](strategy/learning.md) + [Enum Dispatch](enum-dispatch/learning.md) — the trait-object way vs the enum way; the same problem, two Rust solutions.
- [Command](command/learning.md) + [Memento](memento/learning.md) — reified operations + captured state = undo/redo.
- [State Machine](state-machine/learning.md) + [Type State](type-state/learning.md) — runtime vs compile-time state enforcement.
- [Adapter](adapter/learning.md) + [Extension Traits](extension-traits/learning.md) — the classical pattern and its Rust-idiomatic replacement.
- [Decorator](decorator/learning.md) + [Proxy](proxy/learning.md) — same structure, different intent.
- [Visitor](visitor/learning.md) + [Enum Dispatch](enum-dispatch/learning.md) — the pattern and the Rust feature that replaces it.
- [Newtype](newtype-and-zero-cost/learning.md) + [Marker Traits & Phantom Types](marker-traits-and-phantom-types/learning.md) — wrapping for type safety at the value level and the type level.

## Where this category meets architecture patterns

Several design patterns are the same idea at a different scale; read the pair once you have both sides.

| Design Pattern | Architecture counterpart | The shared idea |
| --- | --- | --- |
| [Observer](observer/learning.md) | [Event-Driven Architecture](../architecture-patterns/event-driven-architecture/learning.md) | Pub-sub: in-process callbacks vs cross-service topics |
| [State Machine](state-machine/learning.md) | [Circuit Breaker](../architecture-patterns/circuit-breaker/learning.md) | State machines controlling behavior |
| [Command](command/learning.md) | [Event Sourcing](../architecture-patterns/event-sourcing/learning.md) | Reified operations as first-class data |
| [Memento](memento/learning.md) | [Event Sourcing](../architecture-patterns/event-sourcing/learning.md) | Snapshot & restore (Memento is the local version) |
| [Chain of Responsibility](chain-of-responsibility/learning.md) | [Backpressure & Rate Limiting](../architecture-patterns/backpressure-and-rate-limiting/learning.md) | Middleware pipelines |
| [Decorator](decorator/learning.md) | [Async & I/O](../performance-optimization/async-and-io/learning.md) | `tower` layers — wrapping to add behavior |
| [Repository](repository-and-unit-of-work/learning.md) | [Outbox Pattern](../architecture-patterns/outbox-pattern/learning.md) | Transaction boundary management |

## Where this category meets performance

| Design Pattern | Performance counterpart | The shared idea |
| --- | --- | --- |
| [Enum Dispatch](enum-dispatch/learning.md) | [Serialization & Encoding](../performance-optimization/serialization-and-encoding/learning.md) | Static vs dynamic dispatch cost |
| [Flyweight](flyweight/learning.md) | [Allocation Strategies](../performance-optimization/allocation-strategies/learning.md) | Sharing to reduce allocation |
| [RAII & Drop Guards](raii-and-drop-guards/learning.md) | [Lock-Free Concurrency](../performance-optimization/lock-free-concurrency/learning.md) | Resource lifetime guarantees |
| [Marker Traits](marker-traits-and-phantom-types/learning.md) | [Parallelism & Work Stealing](../performance-optimization/parallelism-and-work-stealing/learning.md) | `Send`/`Sync` as safety contracts |
