# Adapter — Learning Notes

## Mental Model

The Adapter pattern solves an **integration constraint**. You have a domain component that dictates a specific interface (the expected trait) and an infrastructure component that provides the right functionality but speaks the wrong interface (the struct). You own neither, or you own both but modifying either violates architectural boundaries. 

An adapter is a stateless (or thin) translation layer that wraps the incompatible component and implements the expected trait, fulfilling the caller's contract without leaking foreign types into the domain.

In Rust, the adapter is not just an organizational choice; it is a structural necessity forced by the compiler. The **Orphan Rule** dictates that you cannot implement a foreign trait on a foreign type. If you want `reqwest::Client` (foreign) to implement `tower::Service` (foreign), the compiler blocks you. The adapter—specifically the Newtype pattern—is the only mechanism to bridge that gap.

## Structure & Participants

- **Target:** The `trait` your application domain expects. It defines the contract.
- **Adaptee:** The existing `struct` (often from a third-party crate or legacy module) that performs the actual work but doesn't implement the Target trait.
- **Adapter:** A wrapper `struct` (usually a Newtype) that holds the Adaptee and implements the Target trait, translating method calls from the Target interface to the Adaptee's interface.

## Idiomatic Rust Implementation

In Rust, adaptation takes a few distinct forms depending on whether you are adapting behavior, data, or references.

### 1. The Newtype Adapter (Behavior)
This is the standard solution to the Orphan Rule. You wrap the foreign type in a local tuple struct, making it a "local type" that can now implement any trait.

```rust
// The Target (Domain)
pub trait EventHandler {
    fn handle(&self, event: &str);
}

// The Adaptee (Foreign infrastructure)
pub struct OpaqueVendorLogger;
impl OpaqueVendorLogger {
    pub fn write_event(&self, level: u8, msg: &str) {
        println!("Level {}: {}", level, msg);
    }
}

// The Adapter
pub struct LoggerAdapter(pub OpaqueVendorLogger);

impl EventHandler for LoggerAdapter {
    fn handle(&self, event: &str) {
        // Translation logic happens here
        self.0.write_event(1, event);
    }
}
```

### 2. `From` / `Into` and `TryFrom` / `TryInto` (Data)
When translating data structures rather than behavior, Rust's standard library provides the ultimate adapter traits: `From` and `Into`. If adaptation can fail (e.g., parsing an enum from an integer), use `TryFrom`.

```rust
use std::convert::TryFrom;

pub struct LegacyDbRow {
    pub status: i32,
}

pub enum DomainStatus {
    Active,
    Inactive,
}

pub struct StatusError;

// Fallible data adapter
impl TryFrom<LegacyDbRow> for DomainStatus {
    type Error = StatusError;
    
    fn try_from(row: LegacyDbRow) -> Result<Self, Self::Error> {
        match row.status {
            1 => Ok(DomainStatus::Active),
            0 => Ok(DomainStatus::Inactive),
            _ => Err(StatusError),
        }
    }
}
```

### 3. `AsRef` and `Borrow` (References)
Sometimes the caller doesn't need to consume the type, but just needs a specific borrowed view of it (like `&str` or `&[u8]`). Implementing `AsRef<T>` allows your type to seamlessly adapt to functions expecting that reference.

```rust
fn process_str(s: impl AsRef<str>) {
    println!("{}", s.as_ref());
}
```

## When This Pattern Dissolves in Rust

The GoF class-based Adapter heavily relies on inheritance, which doesn't exist in Rust. 
- **Class Adapters dissolve:** You cannot inherit from both the Target and the Adaptee.
- **Extension Traits:** If you own the caller but *don't* have a strict Trait boundary yet, you can skip the wrapper and just add the missing methods directly to the foreign type using an [Extension Trait](../extension-traits/learning.md).
- **Blanket Implementations:** You can write an adapter for *any* type that implements a certain trait (e.g., `impl<T: Read> MyTrait for T`).

## Worked Example

Imagine you are building a background job runner. The runner orchestrates jobs and expects an HTTP client that implements a generic `HttpService` trait. 

**Stage 0 — The Integration Wall**
Your domain crate defines the contract:
```rust
pub trait HttpService {
    fn fetch(&self, url: &str) -> String;
}

pub fn run_job(client: &impl HttpService) {
    let data = client.fetch("http://example.com");
    // process data...
}
```
You decide to use a robust `AwsClient` from a vendor crate:
```rust
pub struct AwsClient;
impl AwsClient {
    pub fn get_resource(&self, _uri: &str) -> String {
        "aws_data".to_string()
    }
}
```
You try to wire them up: `impl HttpService for AwsClient { ... }`. 
The compiler halts: **E0117**. You cannot implement a trait you didn't define for a type you didn't define.

**Stage 1 — The Newtype Adapter**
You create a thin wrapper in your application crate (which acts as the composition root):
```rust
pub struct AwsAdapter(pub AwsClient);

impl HttpService for AwsAdapter {
    fn fetch(&self, url: &str) -> String {
        self.0.get_resource(url) // Interface translation
    }
}
```

**Stage 2 — Ergonomic Escapes**
Later, a specific job needs to access an AWS-specific method on the inner client. You can safely expose the inner Adaptee by implementing `AsRef`:
```rust
impl std::convert::AsRef<AwsClient> for AwsAdapter {
    fn as_ref(&self) -> &AwsClient {
        &self.0
    }
}
// Now callers can do: adapter.as_ref().aws_specific_method()
```

## Versus

- **Adapter vs. Facade:** Both wrap something to change an interface. Adapter translates an interface to *match what the client expects* (driven by a Target trait). Facade *simplifies* a complex subsystem (driven by a desire for a smaller API surface).
- **Adapter vs. Decorator:** Decorator keeps the interface the same but adds behavior (e.g., logging, retries). Adapter changes the interface but keeps the behavior the same.
- **Adapter vs. Bridge:** Bridge is a proactive architectural decision to separate abstraction from implementation. Adapter is a reactive decision applied after-the-fact to make two incompatible things work together.

## Pitfalls in Depth

### Pitfall: Fat Adapters (Adding Business Logic)

- **What goes wrong:** The Adapter starts validating data, caching results, or making secondary database calls to fill in missing fields.
- **Why it happens (the mechanism):** While writing the translation layer `TargetMethod(A) -> AdapteeMethod(B)`, you realize `B` requires an ID that `A` doesn't provide. Since you're already in the adapter, it feels convenient to inject a DB lookup right there.
- **How to handle it, and why that works:** Keep adapters aggressively thin. If translation requires I/O or business decisions, you are building an Orchestrator or a Facade, not an Adapter. Extract the data-fetching step so the caller provides all necessary data, or redefine the Target interface. 
- **Trade-offs of the fix:** Strict separation means more layers. Sometimes a pragmatic "fat adapter" is chosen to avoid refactoring a rigid domain interface.

### Pitfall: Infallible Adaptation of Fallible Operations

- **What goes wrong:** A data adapter implements `From<DatabaseRow> for DomainUser`, but the database row contains an invalid enum integer. The adapter uses `.expect()` or `unreachable!()` and panics in production.
- **Why it happens (the mechanism):** The `From` trait is infallible by contract. Developers reach for it by default when adapting data, forcing them to swallow or panic on edge cases.
- **How to handle it, and why that works:** Use `TryFrom` / `TryInto` whenever adaptation has failure modes (parsing, bounds checking, missing fields). This bubbles the error up to the caller gracefully.
- **Trade-offs of the fix:** The caller must now handle a `Result`, which makes the domain code slightly more verbose but vastly more resilient.

### Pitfall: Trait Object Overhead in Hot Paths

- **What goes wrong:** The application wraps multiple different adapters in `Box<dyn Target>` to store them in a single collection. In a hot loop, the dynamic dispatch overhead degrades performance.
- **Why it happens (the mechanism):** The classic OOP pattern suggests injecting adapters via abstract interfaces (interfaces/trait objects). `Box<dyn Trait>` forces a vtable lookup and prevents the compiler from inlining the adapter's translation logic.
- **How to handle it, and why that works:** Use generics (`impl Target` or `<T: Target>`) for the caller wherever possible. This monomorphizes the function, allowing the compiler to completely inline the adapter's thin wrapper away, making it a zero-cost abstraction.
- **Trade-offs of the fix:** Generics infect the caller's signature, can increase compile times, and prevent heterogeneous collections (e.g., a `Vec` of mixed adapters).

## Design Decisions & Trade-offs

- **Where to put the Adapter:** Does it live in the domain layer or the infrastructure layer? **Infrastructure.** The adapter inherently depends on the concrete Adaptee, which is an external concern. The domain should only own the Target trait.
- **By-value vs By-reference (Ownership):** When wrapping an Adaptee, should the Adapter own it `Adapter(Adaptee)` or borrow it `Adapter<'a>(&'a Adaptee)`? Owning is vastly simpler in Rust, avoiding lifetime proliferation. Reach for borrowing only if the Adaptee is a shared resource (like a database connection pool) that cannot be cloned or wrapped in an `Arc`.
- **Static vs. Dynamic Dispatch:** Default to static dispatch (`impl Target`) for adapters. Because adapters are usually very thin wrappers, the compiler can inline them entirely if statically dispatched.

## Exercises & Self-Test

1. Write a generic adapter that allows any type implementing `std::io::Write` to implement a hypothetical trait `trait ByteSink { fn push_bytes(&mut self, data: &[u8]) -> Result<(), String>; }`. 
2. Why is implementing `From` preferred over `Into`, and how does the standard library handle the relationship between the two?
3. In a multi-threaded web server, you have a `Box<dyn Target + Send + Sync>`. How does wrapping your Adaptee in a Newtype adapter fulfill the `Send + Sync` bounds if the Adaptee already implements them?

## Open Questions

- When multiple distinct third-party crates provide similar functionality (e.g., three different async runtimes), is it better to write an Adapter for each to a single Target trait, or use a Facade to hide them completely?
- How do you effectively test an Adapter if the Adaptee is difficult to mock (e.g., a proprietary database client)?

## References

- [Newtype & Zero-Cost Abstractions](../newtype-and-zero-cost/learning.md)
- [Extension Traits](../extension-traits/learning.md)
- Rust API Guidelines on Interoperability (discussing `From` and `Into`).
