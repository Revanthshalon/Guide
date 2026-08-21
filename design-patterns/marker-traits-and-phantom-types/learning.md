# Marker Traits & Phantom Types — Learning Notes

## Mental Model

Sometimes you need to encode constraints into the type system—like "this data is safe to log," "this ID belongs to a User," or "this pointer outlives that reference"—but this metadata requires exactly zero bytes at runtime.

The mental model is: **Categorize types and track state entirely at compile time.** 
- **Marker Traits** are traits with no methods. They don't provide behavior; they exist purely so a type can "opt in" to a category, allowing the compiler to enforce rules (like "only allow `PIISafe` types to be written to the log file").
- **Phantom Types** are generic parameters that aren't actually used by the struct's fields. They exist to track metadata (like state, ownership, or unit of measurement). To satisfy the compiler, you use a Zero-Sized Type (ZST) to "trick" it into thinking the parameter is used, occupying zero bytes in memory.

## Structure & Participants

### Marker Traits
- **Role:** Empty traits. A struct implements it to claim a property.
- **In Rust:** `trait PiiSafe {}` (custom, manually opted into) or `Send`/`Sync` (built-in). `Send` and `Sync` are declared as `unsafe auto trait`s: the compiler implements them automatically for any type whose fields are all `Send`/`Sync`, and you almost never write `impl Send for MyType {}` by hand — you only intervene to *opt out* (e.g. `impl !Send for MyType {}`, nightly-only) or to assert `unsafe impl Send` when a raw pointer or similar makes the auto-derivation too conservative. This is a different mechanism from `PiiSafe`, which requires an explicit `impl` per type.

### PhantomData
- **Role:** A marker struct (`std::marker::PhantomData<T>`) that tells the compiler how to treat a generic parameter `T` that doesn't appear in the struct's real fields. It explicitly models *ownership* and *variance* for the borrow checker.
- **In Rust:** `struct Id<T> { value: u64, _marker: PhantomData<T> }`

## Idiomatic Rust Implementation

### Marker Traits for Security Boundaries

Instead of relying on developer discipline to avoid logging Personally Identifiable Information (PII), use the compiler.

```rust
// A marker trait with no methods.
pub trait PiiSafe {}

#[derive(Debug)]
struct UserId(u64);
impl PiiSafe for UserId {} // Safe to log

#[derive(Debug)]
struct Email(String);
// Email does NOT implement PiiSafe.

// A function that physically cannot accept un-safe types.
fn emit_telemetry<T: PiiSafe + std::fmt::Debug>(field_name: &str, val: &T) {
    println!("telemetry: {} = {:?}", field_name, val);
}

fn main() {
    let uid = UserId(101);
    let email = Email("user@example.com".to_string());

    emit_telemetry("user_id", &uid);
    // emit_telemetry("email", &email); // COMPILE ERROR!
}
```

### Phantom Types for Type-Safe IDs

Passing a `ProductID` into a function expecting a `UserID` is a classic bug when both are just `u64`. Phantom types solve this with zero runtime overhead.

```rust
use std::marker::PhantomData;

// The marker types. They don't even need to be instantiated.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct User;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Product;

// The PhantomData<T> tells the compiler we "own" a T, even though it takes 0 bytes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Id<T> {
    value: u64,
    _marker: PhantomData<T>,
}

impl<T> Id<T> {
    pub fn new(value: u64) -> Self {
        Id { value, _marker: PhantomData }
    }
}

fn fetch_user(id: Id<User>) {
    println!("Fetching user {}", id.value);
}

fn main() {
    let user_id: Id<User> = Id::new(101);
    let product_id: Id<Product> = Id::new(202);

    fetch_user(user_id);
    // fetch_user(product_id); // COMPILE ERROR: expected Id<User>, found Id<Product>
}
```

## When This Pattern Dissolves in Rust

This doesn't dissolve—it is the foundation of Rust's thread-safety guarantees. 
In C++ or Java, thread safety is an external property verified by the programmer. In Rust, `Send` and `Sync` are Marker Traits. If a type isn't `Send`, the compiler physically forbids sending it across thread boundaries. `PhantomData` is similarly mandatory when writing unsafe code to correctly inform the borrow checker about raw pointers.

## Worked Example

### The Sealed Trait Pattern

Sometimes you want an open trait for dynamic dispatch within your library, but you *don't* want downstream crates to implement it (to preserve backward compatibility when adding methods). You enforce this using a marker trait inside a private module.

```rust
// A private module prevents external access
mod private {
    pub trait Sealed {}
}

// Public trait, but requires implementing the private Sealed trait!
pub trait DatabaseDriver: private::Sealed {
    fn connect(&self);
}

pub struct Postgres;
impl private::Sealed for Postgres {}
impl DatabaseDriver for Postgres {
    fn connect(&self) { println!("Connecting to PG..."); }
}
```

If a user tries to `impl DatabaseDriver for MongoDb`, the compiler says they must also implement `Sealed`. But they can't, because `private::Sealed` is inaccessible outside your crate! You have successfully created a closed trait.

## Versus

### Newtype Pattern
- **What's the same:** Both achieve type-safety (e.g., differentiating `UserId` and `ProductId`).
- **What's different:** Newtype defines entirely separate structs (`struct UserId(u64)`). Phantom types use a single generic struct parameterized by markers (`Id<User>`).
- **How to decide:** If the underlying behavior is exactly the same and you just need many distinct tags, Phantom Types (`Id<T>`) scale better than writing boilerplate for 50 Newtypes. If they need distinct methods, use Newtypes.

### Type State Pattern
- **What's the same:** Both use types to represent states or properties.
- **What's different:** Type State actively transitions between generic parameters (e.g., `Builder<Unconfigured> -> Builder<Configured>`). Marker/Phantom types often just passively tag data (e.g., `Id<User>`). They frequently work together.

## Pitfalls in Depth

### Pitfall: Over-constrained Derived Traits

- **What goes wrong:** You `#[derive(Clone, PartialEq)]` on `struct Id<T>`, but then `let id1 = Id::<User>::new(1); let id2 = id1.clone();` fails to compile with "User does not implement Clone".
- **Why it happens (the mechanism):** The `#[derive]` macro is naive. It generates `impl<T: Clone> Clone for Id<T>`. It assumes that for `Id<T>` to be cloned, `T` must be cloned. But `T` is only a phantom type; you don't actually need to clone a `T` to clone the `u64` value!
- **How to handle it, and why that works:** Either aggressively derive the same traits (`Copy, Clone, PartialEq, Eq, Hash`) on your empty marker structs (`struct User;`), OR manually implement `Clone` and `PartialEq` for `Id<T>` without putting the `T: Clone` bound on it.
- **Trade-offs of the fix:** Deriving on the markers is easy but pollutes them. Manual implementation is boilerplate-heavy but keeps the API cleaner.

### Pitfall: Dropped PhantomData and Variance in Unsafe Code

- **What goes wrong:** When writing a custom collection using raw pointers (`*const T`), omitting `PhantomData` or using the wrong one causes the compiler to misunderstand lifetimes, leading to use-after-free bugs or overly restrictive compile errors.
- **Why it happens (the mechanism):** The borrow checker uses generic parameters to calculate **variance** (subtyping rules for lifetimes) and **drop checking**. Raw pointers bypass these checks. `PhantomData<T>` explicitly tells the compiler: "I *own* a `T`, so apply drop-checking to ensure `T` outlives me." `PhantomData<fn() -> T>` says "I *produce* a `T` (covariant), but I don't own it."
- **How to handle it, and why that works:** If you own the data (like `Vec<T>`), use `PhantomData<T>`. If you just hold a reference to it (like an iterator), use `PhantomData<&'a T>`. This perfectly aligns the compiler's safety checks with your unsafe implementation.
- **Trade-offs of the fix:** Requires deep knowledge of Rust's variance rules (covariance, contravariance, invariance) when building foundational data structures.

### Pitfall: Noise in API Signatures

- **What goes wrong:** Users of your library have to construct types like `Id { value: 5, _marker: PhantomData }`, bleeding implementation details into every call site.
- **Why it happens (the mechanism):** Rust requires all fields of a struct to be initialized, even ZSTs like `PhantomData`.
- **How to handle it, and why that works:** Keep the `_marker` field private, and provide a `new()` method that abstracts the `PhantomData` initialization away.
- **Trade-offs of the fix:** None. This is universally best practice.

## Design Decisions & Trade-offs

**Zero Cost:** `PhantomData` and Marker Traits literally do not exist at runtime. `std::mem::size_of::<Id<User>>()` is exactly 8 bytes (the size of `u64`). They are purely compile-time constructs.

**Mental Overhead:** Heavy use of Phantom types makes the codebase look abstract and daunting to beginners. A signature like `fn process<T, S: State>(req: Request<T, S>)` is harder to read than concrete types. Keep them at the boundary layers (like database IDs, security boundaries, or complex state machines).

## Exercises & Self-Test

1. Define a `Currency<T>` struct that wraps an `f64`. Define `Usd` and `Eur` markers. Write an `add` function that only allows adding identical currencies, returning a new `Currency<T>`.
2. What happens if you try to `#[derive(PartialEq)]` on `Id<T>`, but the `User` marker struct does not implement `PartialEq`? How do you fix it without changing `User`?
3. How does the Sealed Trait pattern prevent external implementations, and why is this useful for library authors?
4. If `PhantomData` takes up no memory, why is it required when building a custom `Vec<T>` with a raw pointer?

## Open Questions

- What is the difference between `PhantomData<T>`, `PhantomData<*const T>`, and `PhantomData<fn() -> T>` in terms of variance?
- Can we use const generics (e.g., `Id<"user">`) instead of Phantom types to achieve the same result in modern Rust?

## References

- [Rustonomicon: PhantomData](https://doc.rust-lang.org/nomicon/phantom-data.html)
- [Rustonomicon: Subtyping and Variance](https://doc.rust-lang.org/nomicon/subtyping.html)
- Cross-ref: Type State (`../type-state/learning.md`), Newtype (`../newtype-and-zero-cost/learning.md`)
