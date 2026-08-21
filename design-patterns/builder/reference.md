# Builder — Reference

**One-Liner:** Separate the construction of a complex object from its representation, often enabling compile-time validation or fluid configuration.

## When to Use
- When a struct has many optional fields.
- When an object requires complex validation before it can be safely instantiated.
- When you want to enforce that certain parameters are provided at compile-time (via Type-State).

## Structure Sketch
```rust
struct Product { /* ... */ }
struct ProductBuilder { /* ... */ }

impl ProductBuilder {
    fn new() -> Self { /* ... */ }
    fn with_field(mut self, value: T) -> Self { /* ... */ self }
    fn build(self) -> Result<Product, Error> { /* ... */ }
}
```

## Rust Idiom
- Use the **Owned Builder** (`mut self -> Self`) by default for clean chaining.
- Return `Result<Product, Error>` from `build()` if validation can fail.
- Use the **Type-State Pattern** for strictly required fields to move runtime errors to compile time.

## Versus
- **Factory:** Factory produces a fully formed object in one step (often hiding the concrete type). Builder exposes the step-by-step configuration of a concrete type.
- **Struct Update Syntax:** `..Default::default()` is simpler and should be preferred over builders for structs without complex invariants.

## Pitfalls

| Pitfall | Mechanism | Fix | Trade-off |
| :--- | :--- | :--- | :--- |
| **Builder Boilerplate** | Writing manual builders for large structs clutters the codebase. | Use `derive_builder` macro. | Adds a dependency and slight compile-time overhead. |
| **Incomplete State at Runtime** | `build()` returns `Result::Err` because a required field was omitted. | Use Type-State builder. | State matrix explodes if there are many required fields. |
| **Borrow Checker Battles** | `&mut self` chaining causes lifetime issues in complex scopes. | Use Owned (`mut self`) builders. | Cannot reuse the builder instance without cloning. |

## Rules of Thumb
- If your struct is mostly public data with defaults, just derive `Default` and use struct update syntax.
- If your struct enforces invariants, make fields private and provide an Owned builder.

## Key References
- [Rust API Guidelines: Builder Pattern](https://rust-lang.github.io/api-guidelines/type-safety.html#builders-enable-construction-of-complex-values-c-builder)
