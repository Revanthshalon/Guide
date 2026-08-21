# Facade — Quick Reference

## One-Liner

The Facade pattern provides a simplified, unified entry point to a complex subsystem, decoupling callers from the internal orchestration mechanics.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| Providing a paved-road API for the 80% use case | The subsystem is already simple enough (1-2 steps) |
| Encapsulating complex initialization (e.g., telemetry, server bootstrap) | You find yourself adding business/domain logic to the facade |
| Flattening a deeply nested module hierarchy for library users | |

## Structure Sketch

```rust
// 1. Complex Subsystem
mod internal {
    pub struct Config;
    pub struct DatabasePool;
    pub struct Cache;
}

// 2. The Facade Module
pub mod app_context {
    // Transparent exposure for power users
    pub use super::internal::*;
    
    // The simplified entry point
    pub fn initialize(url: &str) -> (DatabasePool, Cache) {
        let _config = Config;
        // ... complex wiring ...
        (DatabasePool, Cache)
    }
}
```

## Rust Idiom

- **Free Functions:** If a Facade has no state, use a `pub fn` at the module root, not an empty struct.
- **`pub use` Re-exports:** The native Rust way to flatten complex module structures into a clean API surface.
- **Preludes:** `pub mod prelude { pub use ...; }` is a specialized facade for glob-importing essential traits.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Adapter** | Translates an API to fulfill a specific expected trait vs. simplifies an API to make it easier to use. |
| **Mediator** | Centralizes multidirectional communication *within* a system vs. provides a unidirectional entry point from the *outside*. |
| **Builder** | Simplifies the creation of a *single* object vs. simplifies interaction with a *subsystem*. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **The God Object** | Keep it structural. Move domain logic to Domain Services. | `if / else` branching based on business rules inside the facade. |
| **Opaque Lock-in** | Make the facade transparent using `pub use` so advanced clients can bypass it. | Wrapping a complex struct but hiding its builder/configuration methods. |
| **Premature Wrappers** | Don't write facades for subsystems that are already simple. | Facades with a single method that just delegates to a single inner method. |

## Rules of Thumb

- A Facade should provide the simplest possible API for common tasks, without removing the ability to do complex tasks.
- The subsystem components should never depend on or know about the facade.
- If you are typing `wrapper.method()` just to call `inner.method()`, you don't need a facade; you need a public module.

## Key References

- Rust module system visibility
- `std::fs::read_to_string` as the canonical standard library facade
