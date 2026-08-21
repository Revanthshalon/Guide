# Decorator (Wrapper) — Learning Notes

## Mental Model

The Decorator pattern is about augmenting an object's capabilities dynamically, without modifying its source code and without the caller knowing it happened. By wrapping an inner object with an outer object that implements the *exact same interface*, the outer object can intercept calls, perform its added behavior (like logging, caching, or retries), and then delegate the core work to the inner object.

Forget Russian dolls; think of an I/O pipeline or a networking middleware stack. An HTTP request flows through a series of filters. Each filter conforms to the `Service` interface. The timeout filter starts a timer, calls the next layer, and aborts if it takes too long. The metrics filter records the start time, calls the next layer, and records the duration. None of these layers care about the actual HTTP routing; they only care about their specific orthogonal concern.

The critical engineering constraint this solves is the **composition of orthogonal concerns**. If you try to bake logging, caching, and retries into the core HTTP client, you violate the Single Responsibility Principle and create an unmaintainable monolith. Decorators keep these concerns isolated and composable.

## Structure & Participants

- **Component:** The shared `trait` that defines the contract. 
- **Concrete Component:** The core implementer (e.g., `TcpStream`, `File`, `RpcClient`) where the actual domain or I/O work happens.
- **Decorator:** A wrapper struct containing `inner: T` where `T: Component`. It implements `Component` by performing some action and delegating to `inner`.

## Idiomatic Rust Implementation

In Rust, the Decorator pattern is not just idiomatic—it is the foundational architecture of the two most important ecosystems: I/O and async networking.

### 1. I/O Decorators
The standard library uses decorators extensively to add buffering and decompression to raw file streams.
```rust
use std::io::{self, Read, BufReader};
use std::fs::File;
// flate2::read::GzDecoder is a decorator

fn read_data() -> io::Result<()> {
    // Concrete Component
    let file = File::open("data.gz")?; 
    
    // Decorator 1: Adds buffering behavior
    let buffered = BufReader::new(file); 
    
    // Decorator 2: Adds decompression behavior
    // let mut decompressed = GzDecoder::new(buffered); 
    
    // The client just talks to the `Read` trait.
    // decompressed.read_to_string(&mut contents)?; 
    Ok(())
}
```

### 2. Tower Middleware (The Ultimate Rust Decorator)
In the async web ecosystem, `tower::Service` is the Component trait. Middleware layers are simply Decorators. This is a simplified look at how `tower` stacks behaviors.

```rust
pub trait Service<Request> {
    type Response;
    fn call(&mut self, req: Request) -> Self::Response;
}

// 1. Concrete Component
pub struct RpcClient;
impl Service<String> for RpcClient {
    type Response = String;
    fn call(&mut self, req: String) -> Self::Response {
        format!("Response to {}", req)
    }
}

// 2. Decorator: Metrics
pub struct MetricsService<S> {
    inner: S,
}

impl<S, Request> Service<Request> for MetricsService<S>
where
    S: Service<Request>,
{
    type Response = S::Response;
    fn call(&mut self, req: Request) -> Self::Response {
        println!("Recording start time...");
        let res = self.inner.call(req);
        println!("Recording duration...");
        res
    }
}
```

## When This Pattern Dissolves in Rust

Unlike classic OOP (which relies on a shared abstract base class), Rust enforces decorators at compile time using generics and trait bounds (`struct Decorator<T> { inner: T }`).
- **Zero-Cost Composition:** Because the wrapped types are known at compile time, the compiler can inline the entire stack of decorators. A stack of 5 middlewares compiles down to a single state machine.
- **No Class Hierarchies:** You don't inherit from a `BaseDecorator`. You just implement the trait.

## Worked Example

Let's walk through building a resilient Key-Value store progressively. 

**Stage 0 — The Bare Component**
You start with a simple memory store. It implements your trait perfectly, but in production, you realize you have no visibility into its usage.
```rust
pub trait KvStore {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&mut self, key: &str, value: &str);
}

pub struct MemoryStore; // implements KvStore
```

**Stage 1 — Adding Metrics via Decoration**
Instead of polluting `MemoryStore` with Prometheus counters, you create a wrapper.
```rust
pub struct MetricsStore<T> { inner: T }

impl<T: KvStore> KvStore for MetricsStore<T> {
    fn get(&self, key: &str) -> Option<String> {
        println!("metric: get {}", key);
        self.inner.get(key)
    }
    fn set(&mut self, key: &str, value: &str) {
        println!("metric: set {}", key);
        self.inner.set(key, value)
    }
}
```

**Stage 2 — Adding Tenancy via Decoration**
Later, you need to support multi-tenancy. You want to prefix all keys for a specific tenant. Another decorator!
```rust
pub struct PrefixStore<T> {
    inner: T,
    prefix: String,
}

impl<T: KvStore> KvStore for PrefixStore<T> {
    fn get(&self, key: &str) -> Option<String> {
        self.inner.get(&format!("{}:{}", self.prefix, key))
    }
    // ... same for set
}
```

**Stage 3 — The Stack**
The client receives a fully decorated, capable object but only knows it as a `KvStore`.
```rust
let store = MemoryStore;
let metrics = MetricsStore { inner: store };
let tenant_store = PrefixStore { inner: metrics, prefix: "tenant1".into() };

tenant_store.get("user_1"); 
// Prints: metric: get tenant1:user_1
```

## Versus

- **Decorator vs. Proxy:** Identical structure, different intent. A Proxy controls access (lazy loading, security) and usually manages the lifecycle of its inner object. A Decorator adds behavior to an object given to it.
- **Decorator vs. Adapter:** Adapter changes the interface to make things compatible. Decorator keeps the exact same interface.
- **Decorator vs. Chain of Responsibility:** In Chain, handlers can decide to halt the request. In Decorator, the request is typically intercepted but ultimately delegated down to the core component.

## Pitfalls in Depth

### Pitfall: Type Signature Explosion

- **What goes wrong:** As you stack decorators, the compiler infers the exact concrete type: `Timeout<Retry<Metrics<Prefix<MemoryStore>>>>>`. Returning this from a function requires writing out the entire nested nightmare.
- **Why it happens (the mechanism):** Rust resolves generics at compile time to ensure zero-cost abstraction (monomorphization). Every wrapper adds a generic layer.
- **How to handle it, and why that works:** At API boundaries, hide the stack. Return `-> impl KvStore`. If the stack changes dynamically at runtime (e.g., optional metrics based on config), type-erase it behind a trait object: `Box<dyn KvStore>`.
- **Trade-offs of the fix:** `impl Trait` restricts you to a single compile-time stack. `Box<dyn Trait>` allows dynamic stacks but forces heap allocation and dynamic dispatch.

### Pitfall: Execution Order Confusion

- **What goes wrong:** You add a `CacheDecorator` and a `MetricsDecorator`. You notice your metrics report 0 requests, even though the app is serving traffic.
- **Why it happens (the mechanism):** Decorator order is critical. If `Metrics` is the inner wrapper and `Cache` is the outer wrapper, a cache hit returns immediately—the request never reaches the `Metrics` inner layer. 
- **How to handle it, and why that works:** Map out the "onion" explicitly. Outer layers execute first on the way in, and last on the way out. Put cross-cutting telemetry (Metrics, Tracing) on the absolute outermost layer so it observes everything. Put Caching inside telemetry but outside expensive operations.
- **Trade-offs of the fix:** Changing decorator order can sometimes change type signatures, breaking brittle code that depends on the exact nested type.

### Pitfall: Delegation Boilerplate

- **What goes wrong:** You have a `DatabasePool` trait with 15 methods. You want to decorate just `query()` to add a timeout. You must manually implement the other 14 methods in your decorator just to call `self.inner.method()`.
- **Why it happens (the mechanism):** Rust does not have inheritance or automatic trait delegation. If you implement a trait, you must satisfy its entire contract.
- **How to handle it, and why that works:** For small traits, bite the bullet and write the boilerplate. For massive traits, use a macro crate like `delegate`, or rethink your trait boundaries. If you only want to decorate `query`, perhaps `query` should be its own smaller trait (Interface Segregation Principle).
- **Trade-offs of the fix:** Adding macro dependencies slows down compile times and obscures the code. Splitting traits requires refactoring the core domain.

## Design Decisions & Trade-offs

- **Static vs. Dynamic:** Always default to static stacking via generics (`inner: T`). This allows the compiler to inline the entire pipeline, turning decorators into a zero-cost abstraction. Reach for `Box<dyn Trait>` only when the decorator stack is dynamic or when compile times / binary bloat from monomorphization become a problem.
- **Identity Loss:** When you wrap an object, you hide its unique methods. If `MemoryStore` has a `clear_cache()` method not in the `KvStore` trait, you cannot call it once wrapped in `MetricsStore`. You either have to downcast (difficult/anti-pattern in Rust) or expose that functionality through the shared trait.

## Exercises & Self-Test

1. In the `tower::Service` example, how would you write a `TimeoutMiddleware` that aborts the inner call if it takes too long? What happens to the inner task in Rust's async model when the outer wrapper drops its future?
2. Why is a generic decorator stack `A<B<C>>` faster at runtime than a `Vec<Box<dyn Trait>>` of decorators? What is the compile-time cost?
3. Design an order for the following decorators on an HTTP client: `Retry`, `AuthTokenInjector`, `RateLimiter`, `Metrics`. Explain why each goes where it does.

## Open Questions

- Without a built-in delegation feature, large traits remain hostile to the Decorator pattern. Will a native delegation syntax ever land in stable Rust?
- How do you effectively unit test a deep decorator stack without mocking every single layer?

## References

- [tower::Service](https://docs.rs/tower/latest/tower/trait.Service.html)
- [Async & I/O](../../performance-optimization/async-and-io/learning.md)
- [Chain of Responsibility](../chain-of-responsibility/learning.md)
