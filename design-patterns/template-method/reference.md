# Template Method — Quick Reference

## One-Liner

Define the immutable orchestration skeleton of an algorithm in a base type, requiring variants to implement specific steps ("holes") without altering the overarching control flow.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| Multiple implementations share identical orchestration (e.g., retries, lifecycle) but differ in details. | The entire algorithm changes per variant (use Strategy). |
| You want to enforce a specific order of operations (setup → execute → teardown). | There is only one deferred step (just pass a closure `FnMut`). |
| You are building an API where users fill in missing lifecycle hooks. | The steps don't share state and don't logically belong together. |

## Structure Sketch

```rust
// The Template Trait
pub trait Lifecycle {
    // Required step (Hole)
    fn start(&mut self);
    // Required step (Hole)
    fn stop(&mut self);
    
    // Hook (Optional, has default)
    fn on_error(&self, _err: &str) {} 
    
    // The Template Method (The Skeleton)
    fn run_with_lifecycle(&mut self) {
        self.start();
        // ... internal orchestration ...
        self.stop();
    }
}
```

## Rust Idiom

- **Traits with default methods.** The standard library is built on this. `Iterator` requires `next()` and provides `map`, `filter`, `fold` as template methods.
- **Extension Traits:** If you must prevent the user from overriding the template method, move it to a blanket-implemented extension trait (`impl<T: Lifecycle> LifecycleExt for T { ... }`).

## Versus

| Confused with | Key difference |
| --- | --- |
| **Strategy** | Strategy swaps the whole algorithm via composition; Template swaps individual steps via trait implementation. |
| **Extension Traits** | Extension traits *prevent* overriding; Template Method traits allow overriding by default. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **The `&mut self` Trap** | Require steps to be idempotent, or provide a `reset()` hook before loops. | State corruption when a retry loop calls a mutated implementor a second time. |
| **Template override** | Move the template method to a blanket-implemented Extension Trait. | Implementors silently replacing the orchestration logic, breaking your backoff/retry algorithms. |
| **Temporal coupling** | Use the type system to enforce order (Step A returns a token required by Step B). | Panics because Step C was called before Step A initialized the data. |

## Rules of Thumb

- If the algorithm has only one "hole" to fill, prefer a higher-order function taking a closure over a full trait.
- Keep required methods to an absolute minimum. The more methods an implementor has to write, the less useful the template is.
- Beware `&mut self`. If steps mutate shared state, the order in which the template calls them becomes a highly fragile, implicit contract.

## Key References

- [Rust API Guidelines - Traits](https://rust-lang.github.io/api-guidelines/interoperability.html#c-common-traits)
- Related: [Extension Traits](../extension-traits/learning.md)
