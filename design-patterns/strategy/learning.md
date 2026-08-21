# Strategy — Learning Notes

## Mental Model

Software often needs to perform a specific task (like parsing payloads, routing requests, or evicting cache entries) in multiple different ways. If you hardcode every possible method into a single component using large `match` or `if/else` blocks, that component becomes a bottleneck. It must be modified every time a new method is added, breaking the Open-Closed Principle and causing merge conflicts in busy codebases.

The Strategy pattern solves this by extracting the varying behavior behind a strict contract (an interface). The core logic (the Context) only knows about the contract, delegating the actual work. This lets you swap implementations at runtime or add new ones without touching the core code.

## Structure & Participants

### Context
- **Role:** The driver. It maintains a reference to a Strategy and executes it when needed.
- **In Rust:** A struct holding a trait object (`Box<dyn Strategy>`), a generic struct bounded by a trait (`struct Context<S: Strategy>`), or a struct holding a closure (`F: Fn()`).

### Strategy (Interface)
- **Role:** The contract that all algorithms must fulfill.
- **In Rust:** A `trait` (usually requiring `Send + Sync` in multi-threaded contexts), or simply a function signature.

### Concrete Strategies
- **Role:** The specific implementations of the algorithm.
- **In Rust:** Structs implementing the trait, or specific closures.

## Idiomatic Rust Implementation

Rust provides three distinct ways to implement this pattern.

### 1. Dynamic Dispatch (Trait Objects)

Use this when you need to swap strategies at *runtime* or store different strategies in the same collection. In a multi-threaded Rust application, you often need `Send + Sync` bounds on your trait objects.

```rust
// 1. The Strategy Trait
pub trait EvictionPolicy: Send + Sync {
    fn evict(&self, cache_keys: &[String]) -> Option<String>;
}

// 2. Concrete Strategies
pub struct LruPolicy;
impl EvictionPolicy for LruPolicy {
    fn evict(&self, keys: &[String]) -> Option<String> {
        keys.first().cloned() // Oversimplified LRU
    }
}

pub struct RandomPolicy;
impl EvictionPolicy for RandomPolicy {
    fn evict(&self, keys: &[String]) -> Option<String> {
        keys.last().cloned() // Assume some random selection
    }
}

// 3. The Context
pub struct Cache {
    // Box<dyn Trait> gives us dynamic dispatch
    policy: Box<dyn EvictionPolicy>,
    keys: Vec<String>,
}

impl Cache {
    pub fn new(policy: Box<dyn EvictionPolicy>) -> Self {
        Self { policy, keys: Vec::new() }
    }

    pub fn set_policy(&mut self, new_policy: Box<dyn EvictionPolicy>) {
        self.policy = new_policy; // Runtime swapping!
    }

    pub fn evict_one(&mut self) {
        if let Some(evicted) = self.policy.evict(&self.keys) {
            println!("Evicted {}", evicted);
        }
    }
}
```

### 2. Static Dispatch (Generics)

Use this when the strategy is known at *compile-time*. This is faster (no vtable lookups, allows inlining) but the context's type is now bound to the strategy type.

```rust
pub struct FastCache<P: EvictionPolicy> {
    policy: P,
    keys: Vec<String>,
}

impl<P: EvictionPolicy> FastCache<P> {
    pub fn new(policy: P) -> Self {
        Self { policy, keys: Vec::new() }
    }
}
```

### 3. Closures (Function Pointers)

Use this when the strategy is simple enough that a full struct and trait is overkill.

```rust
pub struct SimpleCache<F> 
where 
    F: FnMut(&[String]) -> Option<String> + Send + Sync
{
    policy_fn: F,
    keys: Vec<String>,
}
```

## When This Pattern Dissolves in Rust

The Strategy pattern does not dissolve in Rust—it is fundamental to it. However, the *ceremony* dissolves. 

In Java, a single-method Strategy requires an interface file, a class file, and an instantiation. In Rust, you can just pass a closure. Furthermore, Rust's `enum` dispatch often replaces Strategy when the set of algorithms is closed and known at compile-time, providing the speed of static dispatch with the ergonomics of dynamic dispatch.

## Worked Example

Imagine an HTTP routing layer that needs to parse different payload formats (JSON, XML).

### Stage 0: The Naive Match

Initially, we might hardcode the parsing into the route handler.

```rust
pub struct RouteHandler;

impl RouteHandler {
    pub fn handle_request(&self, content_type: &str, body: &[u8]) {
        match content_type {
            "application/json" => {
                // Parse JSON...
                println!("Processed JSON");
            }
            "application/xml" => {
                // Parse XML...
                println!("Processed XML");
            }
            _ => println!("Unsupported"),
        }
    }
}
```

This works, but every new content type requires modifying the handler. The `handle_request` method becomes massive and fragile.

### Stage 1: Defining the Strategy

We extract the parsing logic behind a `PayloadParser` trait.

```rust
pub trait PayloadParser: Send + Sync {
    fn parse(&self, raw: &[u8]) -> Result<String, String>;
}
```

### Stage 2: Concrete Strategies

We implement the trait for our specific formats.

```rust
pub struct JsonParser;
impl PayloadParser for JsonParser {
    fn parse(&self, raw: &[u8]) -> Result<String, String> {
        // Simplified JSON parsing
        Ok("Parsed JSON".to_string())
    }
}

pub struct XmlParser;
impl PayloadParser for XmlParser {
    fn parse(&self, _raw: &[u8]) -> Result<String, String> {
        // XML parsing logic...
        Ok("Parsed XML".to_string())
    }
}
```

### Stage 3: The Context

The handler now takes a parser strategy. The core request pipeline remains oblivious to XML vs JSON differences. When building the route tree, the framework wires up the correct parser for each route.

```rust
pub struct ConfiguredRouteHandler {
    parser: Box<dyn PayloadParser>,
}

impl ConfiguredRouteHandler {
    pub fn handle_request(&self, body: &[u8]) {
        match self.parser.parse(body) {
            Ok(parsed) => println!("Processed: {}", parsed),
            Err(e) => println!("400 Bad Request: {}", e),
        }
    }
}
```

## Versus

- **Strategy vs Template Method:** 
  - *Strategy* uses composition (delegates the *entire* algorithm to an object). 
  - *Template Method* uses inheritance/traits (defines the skeleton of an algorithm in a base method, deferring specific steps to subclasses).
- **Strategy vs State:** 
  - *Strategy* swaps algorithms based on configuration or input. The strategy doesn't usually change its own type.
  - *State* swaps behaviors because the internal state of the context has transitioned.
- **Strategy vs Enum Dispatch:** 
  - *Strategy* (Trait objects) is Open-Closed: external crates can add new strategies without modifying your code.
  - *Enum Dispatch* is Closed: you must modify the `enum` to add a variant. It's faster but less extensible.

## Pitfalls in Depth

### Pitfall: Fat Interfaces

- **What goes wrong:** You define a `Strategy` trait with 10 methods because different algorithms need slightly different hooks. Concrete strategies end up implementing methods with `unimplemented!()` or no-ops.
- **Why it happens (the mechanism):** The context is trying to tightly couple with the strategy, dictating too much of the *how* rather than just the *what*.
- **How to handle it, and why that works:** Segregate the interfaces. Keep the strategy trait focused on a single responsibility (one or two methods). If a strategy needs more context, pass it in via the method arguments or the strategy's constructor.
- **Trade-offs of the fix:** You may need multiple traits, which makes the context setup slightly more verbose.

### Pitfall: Generic Infection

- **What goes wrong:** You choose static dispatch (`<S: Strategy>`) for performance, but now every struct that holds your context also needs to be generic over `S`, cascading generics all the way up to your `main.rs`.
- **Why it happens (the mechanism):** Rust's monomorphization resolves generics at compile time, meaning the exact types must be known everywhere in the call stack.
- **How to handle it, and why that works:** Default to dynamic dispatch (`Box<dyn Strategy>`) unless you are in a tight inner loop where vtable overhead (a few nanoseconds) actually matters. Trait objects stop generic infection at the boundary.
- **Trade-offs of the fix:** Slight runtime cost for the pointer dereference; inability to use generic methods inside the trait.

### Pitfall: Object Safety Violations

- **What goes wrong:** You try to use `Box<dyn Strategy>` but the compiler complains that the trait is not "object safe".
- **Why it happens (the mechanism):** A trait cannot be turned into a trait object if it contains methods that return `Self` (other than the receiver), have generic type parameters, or don't take `self` by reference or value. The vtable cannot handle methods where the exact size or type isn't known.
- **How to handle it, and why that works:** Refactor the trait to remove `Self` returns or generics from the methods. Alternatively, use `where Self: Sized` for methods you don't need to call via dynamic dispatch.
- **Trade-offs of the fix:** You lose some flexibility in the trait's API, potentially requiring workarounds like type-erased parameters (`Box<dyn Any>`).

## Design Decisions & Trade-offs

- **Static vs Dynamic:** Choose `dyn Trait` for application-level boundaries (routing, config-driven behavior). Choose generics for low-level libraries (hashing algorithms) where performance is critical.
- **Trait vs Closure:** If the strategy only requires a single function and holds no complex internal state, `Fn` or `FnMut` is much more ergonomic for the caller.

## Exercises & Self-Test

1. Explain a scenario where using static dispatch (`impl Strategy`) would be a poor design choice despite its performance benefits.
2. Refactor a simple `if/else` block that calculates shipping costs based on region into a Strategy pattern using closures.
3. Write a small benchmark comparing the performance of `Box<dyn Strategy>` vs a generic `<S: Strategy>`. What is the actual overhead in nanoseconds?

## Open Questions

- When using closures as strategies, how do we elegantly handle async traits in modern Rust?

## References

- [Rust Design Patterns Book - Strategy](https://rust-unofficial.github.io/patterns/patterns/behavioural/strategy.html) - Standard community resource.
