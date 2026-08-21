# Dependency Injection — Quick Reference

## One-Liner

Instead of components constructing their own dependencies or fetching them globally, dependencies are passed in from the outside (usually via the constructor), decoupling business logic from infrastructure and enabling testability.

## When to Use

| Use it when | Avoid it when |
| --- | --- |
| A component relies on external boundaries (databases, HTTP APIs, file systems). | Passing pure data structs or simple deterministic helper functions. |
| You need to test a component in isolation using mocks. | You are tempted to abstract internal pure-logic components just for the sake of abstraction. |

## Structure Sketch

```rust
use std::sync::Arc;

trait NotificationService: Send + Sync {
    fn notify(&self, msg: &str);
}

// The Client
struct BillingSystem {
    // Dynamic dispatch using Arc is standard for multi-threaded services
    notifier: Arc<dyn NotificationService>,
}

impl BillingSystem {
    // Constructor Injection
    fn new(notifier: Arc<dyn NotificationService>) -> Self {
        Self { notifier }
    }
    
    fn run(&self) {
        self.notifier.notify("Invoice generated");
    }
}
```

## Rust Idiom

Rust does not need DI containers. The idiomatic approach is **Pure DI**: manual assembly of the dependency graph via simple constructor functions (`fn new()`) in `main.rs`. 

When defining the dependency, prefer `Arc<dyn Trait>` for application services (to avoid generic viral spread and support web framework threading models), and prefer Generics (`impl Trait`) for high-performance library code.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Service Locator** | Locator hides dependencies inside the function body; DI declares them visibly in the signature. |
| **Factory Pattern** | Factory is responsible for *creating* things at runtime; DI is a pattern for *providing* long-lived shared services at startup. |
| **Strategy Pattern** | Identical mechanics. Strategy swaps algorithms at runtime; DI swaps architectural boundaries during application startup. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **DI Frameworks** | Stick to Pure DI (manual wiring in `main`). | Adding heavy macro-based crates that slow compile times and hide logic. |
| **The `SystemContext` Struct** | Pass explicit dependencies directly. | Bundling dependencies into a god-object to shorten signatures ruins testability. |
| **Constructor Explosion** | Refactor the component into cohesive domain units. | A `fn new` taking 8+ arguments means the struct violates the Single Responsibility Principle. |
| **Over-abstraction** | Only create traits for things that cross system boundaries or must be mocked. | A 1:1 ratio of traits to concrete structs across the whole codebase. |

## Rules of Thumb

- If a struct establishes its own database connection internally, it's impossible to test in isolation.
- Push the wiring of dependencies as far up the call stack as possible (usually `main`).
- Don't inject things that change per request (like an Order ID); pass those as method arguments. Inject long-lived stateless services.

## Key References

- [Strategy Pattern](../strategy/learning.md) - relies on identical mechanisms (injecting traits).
- [Repository & Unit of Work](../repository-and-unit-of-work/learning.md) - the primary candidates for dependency injection.
