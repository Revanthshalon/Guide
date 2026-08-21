# Facade — Learning Notes

## Mental Model

A Facade solves the problem of **cognitive overload and tight coupling at subsystem boundaries**. When a subsystem consists of dozens of structs, complex initialization sequences, and intricate dependencies, exposing it raw forces every caller to become an expert in that subsystem.

The Facade provides a simplified, higher-level entry point. It hides the orchestration of the internal components and exposes a single API for the 80% use case. The critical distinction is that a Facade is structural: it wires things together and delegates. It does *not* add new business logic. 

If you are using a web framework, the function you call to start the server is a Facade over the TCP listener, the connection pool, the routing tree, and the async runtime. 

## Structure & Participants

- **Facade:** The simplified entry point. It knows which subsystem components to initialize and how to route requests to them.
- **Subsystem:** The collection of low-level structs, traits, and modules that do the actual work. They have no knowledge of the facade.

## Idiomatic Rust Implementation

In Rust, the classical OOP "Facade Class" often dissolves completely into the module system. 

### 1. The Module Boundary and `pub use`
Rust's visibility rules (`pub`, `pub(crate)`) and re-exports (`pub use`) are the native way to build transparent facades. You can have a deeply nested, highly granular internal architecture, but curate a perfectly flat public API.

```rust
// The complex internal subsystem
mod internal {
    pub mod tcp { pub struct Listener; }
    pub mod http { pub struct Router; }
    pub mod tls { pub struct Acceptor; }
}

// The Facade (curated public API)
pub mod server {
    // 1. Re-exporting hides the module hierarchy
    pub use super::internal::tcp::Listener;
    pub use super::internal::http::Router;
    
    // 2. Convenience functions hide the initialization complexity
    pub fn serve(port: u16) {
        let _listener = Listener;
        let _router = Router;
        let _tls = super::internal::tls::Acceptor;
        println!("Listening on {}", port);
    }
}
```

### 2. The `prelude` Pattern
A `prelude` module is a specialized facade designed to be glob-imported (`use my_crate::prelude::*;`). It exposes the absolute minimum set of traits and types required to use a crate effectively, hiding the organizational complexity of where those types actually live.

## When This Pattern Dissolves in Rust

In GoF OOP, a Facade is a dedicated class. In Rust:
- **It dissolves into Free Functions:** Orchestration is usually a standalone `pub fn` (like `std::fs::read_to_string`) rather than an empty struct with static methods.
- **It dissolves into Modules:** `pub use` completely replaces the need for wrapper structs whose only job is passing method calls through.

## Worked Example

Let's look at a notorious subsystem in Rust: setting up OpenTelemetry and Tracing.

**Stage 0 — The Subsystem Bleeds**
Every time you create a new microservice, your `main.rs` starts with 40 lines of wiring formatting layers, OTLP exporters, environmental filters, and the global registry. Your application domain is instantly polluted by infrastructure mechanics.

**Stage 1 — The Structural Facade**
You create a `telemetry` crate. It encapsulates all the tracing crates and exposes a single initialization function.

```rust
// Inside your company's `telemetry` crate
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// The Facade function
pub fn init(service_name: &str) {
    let env_filter = tracing_subscriber::EnvFilter::from_default_env();
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);
    
    // Complex OTLP pipeline setup hidden from caller...
    let otlp_layer = setup_otlp_exporter(service_name); 

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otlp_layer)
        .init();
}

fn setup_otlp_exporter(_name: &str) -> impl tracing_subscriber::Layer<tracing_subscriber::Registry> {
    // ... complex initialization ...
    tracing_subscriber::fmt::layer() // simplified for example
}
```

**Stage 2 — The Client Experience**
Your microservice `main.rs` is now completely decoupled from the tracing subsystem's churn.
```rust
fn main() {
    telemetry::init("checkout-service");
    // start application...
}
```

## Versus

- **Facade vs. Adapter:** Facade simplifies an interface to make a subsystem easier to use. Adapter translates an interface to make it compatible with a specific Target trait. 
- **Facade vs. Mediator:** Both coordinate multiple objects. A Facade orchestrates a subsystem for an *outside* client (unidirectional). A Mediator orchestrates communication *between* the subsystem components themselves (multidirectional), keeping them decoupled from each other.
- **Facade vs. Builder:** A Builder simplifies the creation of a *single* complex object. A Facade simplifies the interaction with a *subsystem* of many objects.

## Pitfalls in Depth

### Pitfall: The God Object (Adding Business Logic)

- **What goes wrong:** The Facade starts taking on domain logic. A `CheckoutFacade` doesn't just orchestrate the `Cart` and `Payment` subsystems; it starts calculating tax rates and applying discount codes internally.
- **Why it happens (the mechanism):** It starts as a simple delegation script. Then you need to conditionally call a subsystem based on user state, so you add an `if` statement. Over time, the Facade inflates into a 1,000-line controller that tightly couples everything.
- **How to handle it, and why that works:** A Facade must be strictly structural. It routes, it initializes, it wires things together. If you are writing domain rules (`if user.is_admin()`), you need a Domain Service or a Controller, not a Facade.
- **Trade-offs of the fix:** Strict separation means you have both a Facade (for setup) and a Domain Service (for logic), increasing the number of types.

### Pitfall: Opaque Facades (Hiding Essential Levers)

- **What goes wrong:** You wrap `reqwest::Client` in a `SimpleHttpClient` facade. Months later, a caller needs to inject a custom TLS certificate or a proxy. Your facade doesn't expose those options, so the caller is blocked.
- **Why it happens (the mechanism):** The facade over-indexes on the 80% use case and locks out the 20% power users by making the subsystem private.
- **How to handle it, and why that works:** Make the facade *transparent*. Provide the 80% paved road (`pub fn init()`), but use `pub use` to expose the underlying subsystem types so advanced users can bypass the facade. Alternatively, provide an escape hatch like `.into_inner()` to yield the raw subsystem object.
- **Trade-offs of the fix:** Exposing the subsystem means you are committing to its API stability as part of your public contract.

### Pitfall: Premature Facades (Dumb Wrappers)

- **What goes wrong:** You create an `EmailService` facade that contains a single `send()` method, which immediately calls `SmtpClient::send()`. It provides zero simplification.
- **Why it happens (the mechanism):** Anticipating future complexity ("we might change email providers later") or a misapplied desire for architectural purity.
- **How to handle it, and why that works:** Delete the wrapper. A Facade's job is to reduce complexity. If there is no complexity to reduce, the Facade is just boilerplate. In Rust, wait until an initialization sequence requires 3+ steps before hiding it.
- **Trade-offs of the fix:** If the underlying implementation *does* change later, you will have to refactor the call sites. This is usually vastly cheaper than maintaining useless wrappers for years.

## Design Decisions & Trade-offs

- **Opaque vs. Transparent:** Default to transparent facades in Rust libraries. Use `pub use` to flatten the API while keeping the granular types public. Use opaque facades (hiding the subsystem completely) only in application code where you want to strictly enforce architectural boundaries.
- **Struct vs. Module vs. Function:** Don't create an empty `struct Orchestrator` with no state just to hold methods. If the facade has no state, it should be a module containing free functions.

## Exercises & Self-Test

1. How does `std::fs::read_to_string` act as a Facade? List the underlying subsystem components it likely orchestrates.
2. You have a `BillingFacade` that charges a credit card and sends a receipt. A new requirement states that VIP users get a 10% discount. Should you add the VIP check to the Facade? Why or why not?
3. What is the distinction between a `prelude` module and a standard Facade?

## Open Questions

- When a Facade requires configuration (e.g., passing 10 arguments to the `init` function), at what point should it transition from a Facade function to a Builder pattern?
- How do you version a transparent facade if the underlying third-party subsystem introduces a breaking change?

## References

- [Rust API Guidelines: Preludes](https://rust-lang.github.io/api-guidelines/future-proofing.html)
- [Module System in Rust Book](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)
- [Builder Pattern](../builder/learning.md)
