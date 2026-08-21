# Builder — Learning Notes

## Mental Model

The Builder pattern solves the engineering problem of constructing objects that require many optional parameters, complex invariants, or multi-step initialization. When an object has a large number of configuration flags, constructing it in a single step leads to brittle, hard-to-read code. 

In Rust, the Builder pattern is particularly essential. Rust lacks named parameters, constructor overloading, and default arguments for functions. Therefore, builders are the idiomatic way to configure and instantiate complex structs without forcing the caller to provide `None` for every optional field.

## Structure & Participants

### Product
- **Role:** The final struct being constructed.
- **In Rust:** A struct, often with private fields to enforce invariants during creation.

### Builder
- **Role:** The intermediary struct that accumulates configuration state.
- **In Rust:** A struct typically containing `Option<T>` for required fields and default values for optional fields, or a generic struct leveraging type-state.

## Idiomatic Rust Implementation

Builders in Rust typically follow one of three styles. We'll explore these in the worked example.

## Worked Example: HTTP Client Configuration

Let's look at how we build a configurable HTTP client, starting from the naive approach and evolving to the Type-State builder.

### Stage 0: Without Builder (The Telescoping Anti-pattern)

Without a builder, we are forced to provide all arguments at once, which becomes unwieldy.

```rust
use std::time::Duration;
use std::collections::HashMap;

#[derive(Debug)]
pub struct HttpClientConfig {
    pub timeout: Duration,
    pub retries: u32,
    pub headers: HashMap<String, String>,
    pub proxy: Option<String>,
}

impl HttpClientConfig {
    pub fn new(
        timeout: Duration,
        retries: u32,
        headers: HashMap<String, String>,
        proxy: Option<String>
    ) -> Self {
        Self { timeout, retries, headers, proxy }
    }
}

// Usage requires passing empty maps and None
let config = HttpClientConfig::new(
    Duration::from_secs(30),
    3,
    HashMap::new(),
    None,
);
```

### Stage 1: The Owned Builder

The Owned Builder takes ownership of `self` in each method and returns `Self`. This allows for elegant method chaining and handles optional fields gracefully.

```rust
#[derive(Debug, Default)]
pub struct ClientConfig {
    timeout: Option<Duration>,
    retries: u32,
    headers: HashMap<String, String>,
    proxy: Option<String>,
}

pub struct ClientBuilder {
    config: ClientConfig,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            config: ClientConfig::default(),
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }

    pub fn retries(mut self, retries: u32) -> Self {
        self.config.retries = retries;
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.config.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn proxy(mut self, proxy: &str) -> Self {
        self.config.proxy = Some(proxy.to_string());
        self
    }

    pub fn build(self) -> Result<ClientConfig, &'static str> {
        if self.config.timeout.is_none() {
            return Err("Timeout is required");
        }
        Ok(self.config)
    }
}

// Usage
let config = ClientBuilder::new()
    .timeout(Duration::from_secs(30))
    .retries(3)
    .header("User-Agent", "MyApp/1.0")
    .build()
    .unwrap();
```

### Stage 2: The Type-State Builder

If we want to enforce at compile-time that a required field (like `timeout`) is provided, we use the Type-State pattern. 

```rust
pub struct NoTimeout;
pub struct HasTimeout(Duration);

pub struct TypedClientBuilder<T> {
    timeout: T,
    retries: u32,
}

impl TypedClientBuilder<NoTimeout> {
    pub fn new() -> Self {
        Self {
            timeout: NoTimeout,
            retries: 0,
        }
    }

    pub fn timeout(self, timeout: Duration) -> TypedClientBuilder<HasTimeout> {
        TypedClientBuilder {
            timeout: HasTimeout(timeout),
            retries: self.retries,
        }
    }
}

// `retries` is independent of `timeout`, so it's defined generically over
// every state `T` instead of being gated behind `HasTimeout`. Gating an
// unrelated field behind another field's state is the "State Explosion"
// mistake described in type-state/learning.md — it forces an arbitrary
// call order (`.timeout()` before `.retries()`) that the domain doesn't
// actually require.
impl<T> TypedClientBuilder<T> {
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }
}

// `build()` is ONLY implemented when the timeout has been provided
impl TypedClientBuilder<HasTimeout> {
    pub fn build(self) -> ClientConfig {
        ClientConfig {
            timeout: Some(self.timeout.0),
            retries: self.retries,
            headers: HashMap::new(), // omitted for brevity
            proxy: None,
        }
    }
}

// Usage
// let error = TypedClientBuilder::new().build(); // COMPILE ERROR
let config = TypedClientBuilder::new()
    .timeout(Duration::from_secs(30))
    .retries(3)
    .build();
```

## When This Pattern Dissolves in Rust

For simple structs where all fields are optional or have sensible defaults, the `Default` trait and struct update syntax render builders unnecessary:

```rust
let config = ClientConfig {
    retries: 5,
    ..ClientConfig::default()
};
```

## Versus

- **Factory:** A factory creates a complete object in a single step, often hiding the concrete type behind an interface. A builder creates an object step-by-step and exposes configuration options explicitly.
- **Constructor with Named Parameters:** In languages like Python, named parameters with defaults replace 90% of builders. In Rust, struct update syntax is the closest alternative, but it cannot enforce complex invariants.

## Pitfalls in Depth

### 1. Builder Boilerplate
- **What goes wrong:** You write hundreds of lines of builder code for a struct with many fields, cluttering the codebase.
- **Why it happens:** Rust requires explicitly implementing the builder struct, the `new()` method, setter methods for each field, and the `build()` method.
- **How to handle it, and why that works:** Use a macro crate like `derive_builder` to generate the boilerplate automatically.
- **Trade-offs of the fix:** Adds a macro dependency and slows compile times slightly, but significantly reduces maintenance burden.

### 2. Incomplete State at Runtime
- **What goes wrong:** A required field is forgotten, and `.build()` returns a `Result::Err` at runtime, crashing the application if unwrapped.
- **Why it happens:** The builder uses `Option<T>` for required fields and defers validation to the runtime `build()` step.
- **How to handle it, and why that works:** Use the Type-State pattern to enforce required fields at compile-time by gating the `build()` method behind specific generic types.
- **Trade-offs of the fix:** Type-state builders are complex to write, can result in intimidating compiler errors, and don't scale well to many required fields (the state matrix explodes). 

### 3. Borrow Checker Battles with `&mut self` Builders
- **What goes wrong:** The caller attempts to use a mutable reference builder in a chain and simultaneously borrow data, leading to borrow checker errors.
- **Why it happens:** Chaining `&mut self` requires the caller to manage lifetimes carefully, and sometimes the temporary references drop too early.
- **How to handle it, and why that works:** Prefer the "owned" builder style (taking `mut self` and returning `Self`) unless you specifically need to mutate a builder conditionally inside a loop.
- **Trade-offs of the fix:** Owned builders consume themselves, meaning you cannot easily reuse the same builder instance to spawn multiple slightly different products without adding a `.clone()` capability to the builder.

## Design Decisions & Trade-offs

- **Validation:** If validation can fail, `build()` must return a `Result`. Do not panic inside `build()`.
- **Encapsulation:** Keep the Product's fields private if the builder enforces invariants. If fields are public, users can bypass the builder entirely.

## Exercises & Self-Test

1. **Build Exercise:** Implement an owned builder for a `DatabaseConnection` struct. Include a `Result`-returning `build()` method that errors if the connection string is empty.
2. **Build Exercise:** Convert your `DatabaseConnection` builder to use the Type-State pattern, ensuring the connection string must be provided at compile-time.
3. Compare the generated assembly (conceptually) or memory footprint between a struct update syntax instantiation and an owned builder. Is there a runtime cost to builders in Rust?

## Open Questions

- When should a builder be implemented as a separate struct vs. implemented directly on the target struct?
- How does the `derive_builder` crate handle complex validation logic compared to hand-written builders?

## References

- [Rust API Guidelines - Builder Pattern](https://rust-lang.github.io/api-guidelines/type-safety.html#builders-enable-construction-of-complex-values-c-builder)
- [derive_builder crate](https://crates.io/crates/derive_builder)
