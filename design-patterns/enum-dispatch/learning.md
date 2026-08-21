# Enum Dispatch — Learning Notes

## Mental Model

When you need polymorphism—a single type that could be one of several different implementations at runtime—classic object-oriented languages reach for interfaces and virtual dispatch. Rust offers trait objects (`dyn Trait`) for this, but more often, idiomatic Rust reaches for **Enum Dispatch**: wrapping a known, closed set of types in an `enum` and using pattern matching to dispatch method calls. 

The mental model is: **If you know all possible variants at compile time, encode them as an enum instead of hiding them behind a dynamic trait.** This changes the polymorphism from *open and dynamic* (anyone can add a type, dispatch happens via vtable at runtime) to *closed and static* (the compiler knows all variants, dispatch is a match statement resolved by standard branching).

## Structure & Participants

### The Dispatch Enum
- **Role:** Acts as the unified type containing one of several specific implementations. It exposes the common interface by matching on itself and delegating to the inner types.
- **In classic OOP:** This is the abstract base class or interface.
- **In Rust:** An `enum` where each variant holds a concrete struct.

### The Concrete Variants
- **Role:** The actual implementations of the behavior.
- **In classic OOP:** Concrete subclasses implementing an interface.
- **In Rust:** Standard structs, often completely unaware of the enum that wraps them.

## Idiomatic Rust Implementation

```rust
// 1. The common behavior
trait MessageHandler {
    fn handle(&self, payload: &str);
}

// 2. Concrete implementations
struct EmailHandler { address: String }
impl MessageHandler for EmailHandler {
    fn handle(&self, payload: &str) {
        println!("Sending email to {}: {}", self.address, payload);
    }
}

struct SmsHandler { phone_number: String }
impl MessageHandler for SmsHandler {
    fn handle(&self, payload: &str) {
        println!("Sending SMS to {}: {}", self.phone_number, payload);
    }
}

// 3. The Dispatch Enum
enum Handler {
    Email(EmailHandler),
    Sms(SmsHandler),
}

// 4. Implementing the trait on the enum itself
impl MessageHandler for Handler {
    fn handle(&self, payload: &str) {
        // Static dispatch via pattern matching
        match self {
            Handler::Email(h) => h.handle(payload),
            Handler::Sms(h) => h.handle(payload),
        }
    }
}
```

## When This Pattern Dissolves in Rust

This pattern *is* Rust's replacement for classic GoF patterns like **Strategy**, **State**, and **Visitor** in many codebases. 
- **Strategy:** Instead of injecting an interface, you inject an enum containing the strategies.
- **State:** Instead of state classes implementing a state interface, the state machine holds an enum of state structs.
- **Visitor:** Classical Visitor relies on double-dispatch to recover lost type information. Enum dispatch makes Visitor largely unnecessary because pattern matching intrinsically knows the concrete type (though Visitor is still needed if you want cross-crate extensibility without `dyn Trait`).

## Worked Example

Consider a text processing pipeline. We want to apply a series of filters to a string. 

### Stage 0: The Trait Object Approach

Typically, developers coming from OOP languages use trait objects. 

```rust
trait Filter {
    fn apply(&self, input: &mut String);
}

// Implementations...
struct LowercaseFilter;
struct TrimFilter;

// Usage:
fn process_dyn(filters: &[Box<dyn Filter>], text: &mut String) {
    for filter in filters {
        filter.apply(text);
    }
}
```

Every filter invocation here incurs a pointer indirection, a vtable lookup, and thwarts the instruction cache and inliner. Furthermore, `Box` means heap allocation for every filter.

### Stage 1: The Enum Dispatch Approach

By switching to an enum, we eliminate the boxing and dynamic dispatch.

```rust
// The Dispatch Enum
enum TextFilter {
    Lowercase(LowercaseFilter),
    Trim(TrimFilter),
}

impl Filter for TextFilter {
    fn apply(&self, input: &mut String) {
        match self {
            TextFilter::Lowercase(f) => f.apply(input),
            TextFilter::Trim(f) => f.apply(input),
        }
    }
}
```

### Stage 2: Usage and Memory Benefits

```rust
fn process_enum(filters: &[TextFilter], text: &mut String) {
    for filter in filters {
        filter.apply(text);
    }
}
```

The difference is physical: a `Vec<Box<dyn Filter>>` is an array of pointers to scattered heap allocations. A `Vec<TextFilter>` is a tightly packed array of values. Traversing the latter is extremely cache-friendly.

## Physical Memory Layout

Understanding Enum Dispatch requires understanding how enums are laid out in memory. The size of a Rust enum is determined by its **largest variant plus a discriminant byte** (often aligned to word size). 

For example:
```rust
enum MyEnum {
    VariantA(u8),           // Needs 1 byte
    VariantB([u64; 100]),   // Needs 800 bytes
}
```
`std::mem::size_of::<MyEnum>()` will be at least 808 bytes! If you have a `Vec<MyEnum>`, every single `VariantA` you store will waste 807 bytes of memory.

## Versus

### Trait Objects (`dyn Trait`)
- **What's the same:** Both allow a collection to hold heterogeneous types.
- **What's different:** Trait objects are *open* (external crates can add implementations) and *dynamic* (runtime vtable). Enums are *closed* (all variants must be known) and *static* (match statement).
- **How to decide:** Use Enum Dispatch when the set of variants is closed or small, and performance/memory layout matters. Use Trait Objects when writing a library that expects users to plug in their own implementations.

### Visitor Pattern
- **What's the same:** Both execute type-specific logic on a collection of different types.
- **What's different:** Visitor requires modifying every class to add an `accept` method. Enum dispatch just pattern matches.
- **How to decide:** Always prefer Enum Dispatch in Rust over Visitor, unless you specifically need open extensibility across crate boundaries without using `dyn Trait`.

## Pitfalls in Depth

### Pitfall: Bloated Enum Size

- **What goes wrong:** You have an enum with variants of vastly different sizes. `Vec<Enum>` wastes memory, and passing the enum by value involves copying massive amounts of padding.
- **Why it happens (the mechanism):** The size of an enum is the size of its largest variant plus a discriminant.
- **How to handle it, and why that works:** Box the large variant: `LargeVariant(Box<LargeStruct>)`. This shrinks the enum size to the size of a pointer, restoring cache density for the common cases.
- **Trade-offs of the fix:** Incurs heap allocation and pointer indirection, but only for the boxed variant.

### Pitfall: Boilerplate Exhaustion

- **What goes wrong:** Adding a new method to the trait requires adding a boilerplate `match self { ... }` delegator to the enum.
- **Why it happens (the mechanism):** Rust does not automatically delegate trait implementations to inner types of an enum.
- **How to handle it, and why that works:** Use the `enum_dispatch` crate or macro tools. It generates the `match` statements for you at compile time.
- **Trade-offs of the fix:** Adds a proc-macro dependency, which increases compile times.

### Pitfall: Inability to Extend

- **What goes wrong:** Downstream users of your library complain they cannot add their own handler to your `enum`.
- **Why it happens (the mechanism):** Enums are strictly closed types.
- **How to handle it, and why that works:** If extensibility is a requirement, you must revert to `dyn Trait`, or provide a `Custom(Box<dyn Trait>)` variant in your enum as an escape hatch.
- **Trade-offs of the fix:** Adding an escape hatch variant bloats the enum size (by the size of a fat pointer) and reintroduces dynamic dispatch overhead for the custom cases.

## Design Decisions & Trade-offs

- **Open/Closed Principle:** Enum dispatch violates the Open/Closed Principle since adding a variant requires modifying the enum. In Rust, this is often a feature because the compiler's exhaustiveness checking guarantees you update all match statements.
- **Performance:** Because the `match` statement resolves to concrete function calls, the optimizer can inline the methods, which it cannot do through a vtable.

## Exercises & Self-Test

1. What dictates the physical memory size of a Rust `enum` in bytes? Write a quick program to test `size_of`.
2. Write an enum `Shape` holding `Circle` and `Square`. Implement an `Area` trait on the enum that delegates to the variants.
3. Compare the memory layout of `Vec<Box<dyn Area>>` versus `Vec<Shape>`.
4. How would you solve the "Bloated Enum Size" problem? Write the before and after structs.
5. Why doesn't Rust need the classical Visitor pattern in most cases?

## Open Questions

- When using Enum Dispatch, how does the branch predictor fare against a virtual function call if there are 20 variants?
- At what threshold (number of variants) does `dyn Trait` become faster than a massive `match` statement?

## References

- [enum_dispatch crate](https://docs.rs/enum_dispatch/latest/enum_dispatch/) — Proc-macro for generating dispatch boilerplate.
