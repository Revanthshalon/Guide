# Strategy — Quick Reference

## One-Liner

Encapsulate interchangeable algorithms or behaviors behind a common interface, allowing the context to swap them without changing its own structure.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| You have multiple variations of an algorithm (e.g., routing, caching, tax calculation). | The algorithm rarely changes and variations are trivial (just use an `if`). |
| You want to configure an object's behavior at runtime. | The variations are a closed set known at compile-time (use enums instead). |
| You want to isolate complex business logic from the context that uses it, allowing independent testing. | |

## Structure Sketch

```rust
// Strategy trait with Send + Sync for thread safety
trait Strategy: Send + Sync {
    fn execute(&self, data: &str) -> String;
}

struct Context {
    // Dynamic dispatch: Flexible, stops generic infection
    strategy: Box<dyn Strategy>,
}

impl Context {
    fn do_work(&self) {
        self.strategy.execute("data");
    }
}
```

## Rust Idiom

Rust offers three distinct flavors of Strategy. Choose based on need:

1. **`Box<dyn Trait>` (Dynamic):** Best default. Stops generic spread, allows runtime swapping. Requires `Send + Sync` in multi-threaded contexts.
2. **`T: Trait` (Static):** Best for hot paths. Fast, allows inlining, but infects the type signature.
3. **`F: Fn() -> T` (Closure):** Best for single-method, simple behaviors without struct state.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Template Method** | Strategy delegates the *whole* algorithm. Template Method dictates the skeleton and delegates *steps*. |
| **State** | Strategy is swapped by the user/config. State swaps itself based on internal transitions. |
| **Enum Dispatch** | Enums are closed (can't add variants externally) and use static dispatch internally via match. Traits are open. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Generic Infection** | Use `Box<dyn Strategy>` instead of generics. | Lifetime bounds on trait objects (e.g., `Box<dyn Strategy + 'a>`). |
| **Fat Interfaces** | Keep the trait to 1-2 core methods. Pass context as args. | Context leaking internal details to the strategy. |
| **Object Safety** | Ensure trait methods don't return `Self` or use generic types. | Loss of type-level precision in the trait's API. |

## Rules of Thumb

- Default to dynamic dispatch (`dyn Trait`) in application code to keep signatures clean.
- Reach for generics (`impl Trait`) in libraries where consumer performance is paramount.
- If it's just one function, just use a closure (`FnMut`). Don't over-engineer a trait.
- Always include `Send + Sync` bounds on trait objects if the context will be shared across threads.
