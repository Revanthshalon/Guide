# Factory — Reference

**One-Liner:** Encapsulate the instantiation logic of objects, allowing the concrete type to be determined at runtime or hidden behind an abstraction.

## When to Use
- When you need to decouple the caller from the concrete implementation of a dependency.
- When object creation involves complex conditional logic (e.g., parsing a config to decide which backend to initialize).
- When building open-ended plugin systems.

## Structure Sketch
```rust
// Enum approach (Closed polymorphism)
enum Product {
    VariantA(TypeA),
    VariantB(TypeB),
}

fn create_product(config: Config) -> Product {
    if config.use_a { Product::VariantA(TypeA) }
    else { Product::VariantB(TypeB) }
}

// Trait approach (Open polymorphism)
trait Product: Send + Sync { fn do_work(&self); }

fn create_dynamic_product() -> Box<dyn Product> {
    Box::new(ConcreteProduct)
}
```

## Rust Idiom
- Prefer **Enums** over `Box<dyn Trait>` when the set of products is known at compile-time.
- Prefer **free-standing functions** (`fn create_x()`) over empty Factory structs.
- Always append `+ Send + Sync` to trait objects if they will be used in concurrent contexts.

## Versus
- **Builder:** Factory is a one-shot creation. Builder is multi-step configuration.
- **Default Trait:** `Default` is a zero-argument factory for a single, known concrete type.

## Pitfalls

| Pitfall | Mechanism | Fix | Trade-off |
| :--- | :--- | :--- | :--- |
| **Java-style Factory Classes** | Using empty structs just to hold a `create` method. | Use free-standing functions. | Less visual grouping of factory methods. |
| **Missing Thread Safety** | Returning `Box<dyn Trait>` prevents sharing across threads. | Return `Box<dyn Trait + Send + Sync>`. | Forces concrete types to be thread-safe. |
| **Excessive Dynamic Dispatch** | Overusing `Box<dyn Trait>` causes heap allocations. | Use Enums for closed variants. | Enums limit extensibility by external crates. |

## Rules of Thumb
- Start with a simple `fn`. 
- Upgrade to an Enum if you need to return multiple internal types.
- Upgrade to `Box<dyn Trait>` only if downstream users need to provide their own types.

## Key References
- [Rust Design Patterns - Factory](https://rust-unofficial.github.io/patterns/idioms/pass-and-return.html)
