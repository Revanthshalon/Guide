# Newtype & Zero-Cost Abstractions — Learning Notes

## Mental Model

**Primitive obsession** is a code smell where you use basic data types (like `u64`, `f64`, or `String`) to represent domain concepts (like an Order ID, Distance in Meters, or an Email Address). The danger is mixing them up: accidentally passing a `u64` User ID to a function expecting a `u64` Order ID, or adding meters to feet.

The **Newtype** pattern solves this by wrapping the primitive in a single-field tuple struct. Because Rust resolves types statically and eliminates the wrapper during compilation, this is a **zero-cost abstraction**. It guarantees type safety at compile time while producing the exact same machine code as if you had just used the bare primitive.

## Structure & Participants

### The Newtype
- **Role:** A single-field tuple struct that wraps an underlying type, creating a strictly distinct type in the eyes of the compiler.
- **In classic OOP:** Often requires creating a class with an inner field, sometimes incurring heap allocation or pointer indirection.
- **In Rust:** `struct UserId(u64);`

### Trait Implementations
- **Role:** Selectively exposing behaviors of the inner type, or implementing foreign traits.
- **In Rust:** Implementing `From`, `Into`, `Display`, or using `derive`.

## Idiomatic Rust Implementation

```rust
// The Newtypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(pub u64);

fn process_order(user: UserId, order: OrderId) {
    println!("Processing order {} for user {}", order.0, user.0);
}

// Usage:
// let u = UserId(1001);
// let o = OrderId(55);
// process_order(u, o); // Compiles!
// process_order(o, u); // COMPILE ERROR: mismatched types
```

## When This Pattern Dissolves in Rust

This pattern is a native idiom in Rust. In languages like Java, a primitive wrapper (like `Integer`) incurs an object allocation. In Go, you use `type UserId uint64` (a type alias with distinct identity). In Rust, the single-field tuple struct is the canonical way to achieve strong typedefs, with strictly zero runtime overhead.

## Worked Example

### Stage 0: Primitive Obsession

Imagine an application dealing with physics calculations. 

```rust
fn calculate_time(distance: f64, velocity: f64) -> f64 {
    distance / velocity
}

// Usage:
let time = calculate_time(10.0, 50.0); // Wait, was distance in feet or meters?
```
The compiler cannot save you if you accidentally pass velocity where distance is expected.

### Stage 1: The Newtype Enforces Domain Safety

We wrap the primitives in distinct types.

```rust
#[derive(Debug, Clone, Copy)]
pub struct Meters(pub f64);

#[derive(Debug, Clone, Copy)]
pub struct MetersPerSecond(pub f64);

#[derive(Debug, Clone, Copy)]
pub struct Seconds(pub f64);

fn calculate_time(distance: Meters, velocity: MetersPerSecond) -> Seconds {
    Seconds(distance.0 / velocity.0)
}
```
Now, `calculate_time(velocity, distance)` is a compiler error. We've eliminated an entire class of bugs at zero runtime cost.

### Stage 2: Bypassing the Orphan Rule

Rust's **Orphan Rule** states you can only implement a trait for a type if you own either the trait or the type. You cannot implement `std::fmt::Display` (foreign trait) for `Vec<String>` (foreign type). The Newtype pattern is the standard escape hatch.

```rust
// 1. Create a Newtype (which we own)
pub struct CommaSeparated(pub Vec<String>);

// 2. Implement the foreign trait for our owned Newtype
impl std::fmt::Display for CommaSeparated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join(", "))
    }
}

// Usage:
// let items = vec!["apple".to_string(), "banana".to_string()];
// println!("Items: {}", CommaSeparated(items));
```

## Versus

### Type Aliases (`type Id = u64;`)
- **What's the same:** Both give a semantic name to an existing type.
- **What's different:** Type aliases are *not distinct types*. The compiler sees `Id` and `u64` as identical. You can pass a `u64` to a function expecting `Id`. Newtypes are strictly distinct; you must explicitly wrap and unwrap.
- **How to decide:** Use type aliases to shorten long signatures (e.g., `type Result<T> = std::result::Result<T, MyError>`). Use Newtypes for type safety and domain modeling.

## Pitfalls in Depth

### Pitfall: Ergonomic Degradation (Boilerplate)

- **What goes wrong:** You wrap `String` in `struct Username(String)`. Now you can't call `.len()`, `.is_empty()`, or `.contains()` without typing `.0.len()`.
- **Why it happens (the mechanism):** The Newtype strictly hides all methods of the inner type.
- **How to handle it, and why that works:** 
  1. Provide explicit delegate methods (`pub fn len(&self) -> usize { self.0.len() }`).
  2. Implement `AsRef<str>` for read-only access.
  3. Use the `derive_more` crate to auto-derive specific traits like `Display`, `Add`, `From`.
- **Trade-offs of the fix:** Manually delegating is tedious. Proc-macros increase compile time.

### Pitfall: The `Deref` Anti-Pattern

- **What goes wrong:** To avoid boilerplate, you implement `std::ops::Deref` for your Newtype so it behaves exactly like the inner type. Later, someone modifies the inner type, breaking invariants. For instance, `Email(String)` derefs to `String`, allowing a caller to use `String::clear()` or append invalid characters.
- **Why it happens (the mechanism):** `Deref` is designed for smart pointers (`Box`, `Rc`), not for interface inheritance. It implicitly exposes *every* method of the target type.
- **How to handle it, and why that works:** Deliberately implement only the methods you need, or implement `AsRef` for read-only viewing. Never use `DerefMut` for domain Newtypes that enforce invariants.
- **Trade-offs of the fix:** You have to write more boilerplate to expose the safe methods.

### Pitfall: FFI ABI Mismatch

- **What goes wrong:** You pass a `struct UserId(u64)` across a C Foreign Function Interface (FFI). The C side expects a standard 64-bit integer, but receives garbage or crashes.
- **Why it happens (the mechanism):** While a single-field tuple struct has the same *size* as its inner type, the Rust compiler does not guarantee it has the same *ABI (Application Binary Interface)*. The calling convention might pass structs differently than primitive integers in registers.
- **How to handle it, and why that works:** Apply the `#[repr(transparent)]` attribute to the struct. This strictly guarantees that the struct has exactly the same memory layout and ABI as its single non-zero-sized field.
- **Trade-offs of the fix:** None, it's just easily forgotten when defining FFI boundaries.

## Design Decisions & Trade-offs

- **Cost of Wrapping:** It is zero. At runtime, `UserId(u64)` is exactly a `u64` residing in a register or on the stack. The wrapper exists purely in the compiler's semantic analysis.
- **Construction Ergonomics:** Implement `From<InnerType>` for your Newtype so callers can use `.into()`. However, if the inner type must be validated (e.g. `Email(String)`), DO NOT implement `From`; instead, implement `TryFrom` or a `fn new(inner) -> Result<Self, Error>` to enforce the invariant.

## Exercises & Self-Test

1. Define a Newtype `Meters(f64)` and `Feet(f64)`. Write a function that converts `Meters` to `Feet`. Verify it compiles.
2. Why is `type UserId = u64;` insufficient to prevent passing an `OrderId` to a user function?
3. What is the Orphan Rule, and exactly how does the Newtype pattern bypass it?
4. Explain why implementing `DerefMut` on a `Password(String)` newtype could be a critical security flaw.
5. When should you use `#[repr(transparent)]` on a Newtype?

## Open Questions

- What is the performance impact of `From`/`Into` conversions in Newtypes when deep nesting is involved? (Answer: still zero, due to inlining, but how to verify via godbolt?)
- Should domain ID newtypes implement `Copy`? (Generally yes, if the inner type is `u64` or `uuid::Uuid`.)

## References

- [Rust Book: Using the Newtype Pattern](https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#using-the-newtype-pattern-to-implement-external-traits-on-external-types)
- `derive_more` crate — standard tool for removing newtype boilerplate.
