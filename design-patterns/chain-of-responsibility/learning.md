# Chain of Responsibility — Learning Notes

## Mental Model

Chain of Responsibility decouples the sender of a request from the code that eventually processes it. Instead of a monolith receiving a request and executing a massive `if-else` block to decide what to do (authenticate, rate-limit, parse, route), the request is passed through a sequence of discrete handlers.

Each link in the chain receives the request and makes a choice:
1. **Short-circuit:** Reject the request immediately (e.g., unauthorized) and return early.
2. **Handle:** Completely fulfill the request and return the response.
3. **Pass:** Forward the request to the next link in the chain (often after mutating or inspecting it).

This allows you to construct modular pipelines where each link does exactly one thing, and the order of operations is cleanly separated from the domain logic.

## Structure & Participants

- **Handler Trait:** The interface defining how to process a request (and optionally how to pass it to the next handler).
- **Concrete Handlers:** The individual links in the chain (e.g., `AuthMiddleware`, `RateLimitMiddleware`, `Router`).
- **The Chain:** The structure connecting the handlers. 

## Idiomatic Rust Implementation

In classical OOP, the Chain is usually a linked list of objects holding a `next: Box<dyn Handler>` pointer. In Rust, that dynamic approach is rarely used for request pipelines because it incurs heap allocations and dynamic dispatch on every request.

Instead, Rust ecosystems (most notably the `tower` crate) use **Static Nesting**. The chain is built by nesting structs at compile time. 

### The Ultimate Rust Chain: The `?` Operator

Before looking at structs, realize you already use Chain of Responsibility constantly in Rust. The `?` operator on `Result` types is a built-in Chain of Responsibility!

```rust
// A chain of operations. If any operation fails, it short-circuits (returns Err early).
// If it succeeds, it passes the unwrapped value to the next link.
let user = parse_request(req)?
    .validate()?
    .fetch_db()?
    .sanitize()?;
```
Each `?` inspects the state. If it's `Err`, the chain halts and propagates the error up. If it's `Ok`, it unwraps and passes the data to the next step.

## Worked Example

Let's build an HTTP middleware stack using static nesting to show the performance and compile-time trade-offs.

**Stage 0 — The Monolith**
```rust
fn handle_request(req: Request) -> Response {
    if !req.headers.contains("Bearer") {
        return Response::Unauthorized();
    }
    if rate_limiter.is_throttled(&req.ip) {
        return Response::TooManyRequests();
    }
    // ... actual routing ...
}
```
As the app grows, this function becomes an unmaintainable thousand-line chokepoint.

**Stage 1 — The Static Middleware Chain**
We define a `Service` trait, and wrap services inside each other.

```rust
// The Handler Trait
pub trait Service<Request> {
    type Response;
    fn call(&mut self, req: Request) -> Self::Response;
}

// 1. The Terminal Link (The core application)
pub struct AppHandler;
impl Service<String> for AppHandler {
    type Response = String;
    fn call(&mut self, req: String) -> Self::Response {
        format!("Handled: {}", req)
    }
}

// 2. The Auth Link
pub struct AuthMiddleware<S> {
    inner: S, // The next link in the chain
}

impl<S> Service<String> for AuthMiddleware<S>
where
    S: Service<String, Response = String>,
{
    type Response = String;
    fn call(&mut self, req: String) -> Self::Response {
        if !req.contains("token=123") {
            return "401 Unauthorized".to_string(); // Short-circuit
        }
        self.inner.call(req) // Pass to next
    }
}

// 3. The Logging Link
pub struct LogMiddleware<S> {
    inner: S,
}

impl<S> Service<String> for LogMiddleware<S>
where
    S: Service<String, Response = String>,
{
    type Response = String;
    fn call(&mut self, req: String) -> Self::Response {
        println!("Incoming request: {}", req);
        let res = self.inner.call(req); // Pass to next
        println!("Outgoing response: {}", res); // Post-process
        res
    }
}

fn main() {
    // We compose the chain from inside out: App <- Auth <- Log
    // The type of `chain` is statically known at compile time: 
    // LogMiddleware<AuthMiddleware<AppHandler>>
    let mut chain = LogMiddleware {
        inner: AuthMiddleware {
            inner: AppHandler
        }
    };

    let res = chain.call("GET /data?token=123".to_string());
}
```

**The Scoreboard:**
- **Performance:** Because the chain is statically typed (`LogMiddleware<AuthMiddleware<AppHandler>>`), the Rust compiler can aggressively inline the entire stack. At runtime, there is zero dynamic dispatch overhead. It executes as fast as the Stage 0 monolith.
- **Latency/Compile-time tradeoff:** The runtime latency is absolute minimal. The trade-off is compile time and type complexity. If you stack 20 middlewares, the compiler has to resolve a 20-deep nested generic type, which can slow down compilation and make type-mismatch error messages extremely difficult to read.

## Versus

### Versus Decorator

- **Chain of Responsibility** allows any link to *halt* execution and return early. The focus is on finding the *right* handler or enforcing gates (like auth).
- **Decorator** wraps behavior around a component, but the request almost always reaches the core component. The focus is on *augmenting* behavior, not halting it.
- **In Rust:** They look structurally identical (struct wrapping struct). The difference is purely semantic in how the `call` method behaves.

### Versus Command

- **Command** encapsulates a request as an object to be executed *later*.
- **Chain** routes a request to be executed *now*. (Though they are often combined: a Chain routes a Command).

## Pitfalls in Depth

### Pitfall: The Unhandled Request Drop

- **What goes wrong:** A request traverses the entire chain, no handler claims it, and it silently drops off the end. The caller receives an empty response or the system panics.
- **Why it happens (the mechanism):** The chain lacks a terminal catch-all handler. If every link says "not my problem," and passes the request, there is nothing at the end of the chain to gracefully catch it.
- **How to handle it, and why that works:** The innermost core of the chain *must* be a terminal handler that explicitly returns a "Not Found" or "Unhandled" error. In web frameworks, this is the default 404 handler at the bottom of the routing stack.
- **Trade-offs of the fix:** Requires explicit configuration of the terminal node, rather than relying on implicit fall-through.

### Pitfall: Fat Middleware (Violating Single Responsibility)

- **What goes wrong:** A single middleware link does authentication, rate limiting, *and* payload parsing. The chain becomes a monolith disguised as a pipeline.
- **Why it happens (the mechanism):** Once the plumbing for a middleware chain is set up, it's easier to add ten lines of code to an existing link than to create a new struct and implement the trait boilerplate.
- **How to handle it, and why that works:** Strictly enforce that one link = one reason to reject a request. Auth is one link. Rate limiting is another. Tracing is another. This makes them composable, re-orderable, and testable in isolation.
- **Trade-offs of the fix:** More boilerplate and more deep type nesting, which compounds the compile-time cost.

### Pitfall: Order Dependency and Silent Corruption

- **What goes wrong:** You reorder your middleware, moving `RateLimiter` after `BodyParser`. Suddenly, a slow-loris attack takes down your server because parsing happens before rate limiting. Or, `Auth` requires an IP extracted by `ProxyHeaders`, but they are in the wrong order, so `Auth` fails silently.
- **Why it happens (the mechanism):** Middlewares often communicate implicitly via context attached to the request (e.g., an `Extensions` type map). The type system does not enforce that Link B runs before Link A.
- **How to handle it, and why that works:** Document ordering constraints clearly. Where possible, use the type system: if Auth requires an IP, make the `call` signature require an `IpAddress` token that is only produced by the `ProxyHeaders` middleware.
- **Trade-offs of the fix:** Type-safe middleware ordering is incredibly difficult to design generally (which is why frameworks like `axum` rely on untyped `Extensions` maps and leave ordering to the developer).

## Design Decisions & Trade-offs

**Static Nesting vs Dynamic Vectors:**
- *Static Nesting* (`A<B<C>>`): Zero-cost abstraction, highly optimized, type-safe. The chain structure is fixed at compile time. This is the `tower` way.
- *Dynamic Vectors* (`Vec<Box<dyn Handler>>`): Allows adding/removing middleware at runtime based on config, but incurs heap allocation and dynamic dispatch costs on every request. Use only when the chain *must* be reconfigured dynamically at runtime.

## Exercises & Self-Test

1. How does the structure of a `tower::Service` middleware chain differ from the classic OOP linked-list Chain of Responsibility?
2. Explain how the `?` operator acts as a Chain of Responsibility.
3. Write a static chain of three string filters (Struct A wraps B wraps C). One trims whitespace, one capitalizes, one replaces bad words.
4. If you have 15 nested middlewares, what happens to the type signature of the outermost struct, and why might you use `Box<dyn Service>` at the boundary to mitigate it?

## Open Questions

- When does deep static nesting of middleware cause unacceptable compile-time bloat, and what are the best strategies to break the type chain without losing performance?
- How do you effectively trace the execution flow through a heavily nested, asynchronous middleware chain without getting lost in the boilerplate stack traces?

## References

- [Tower - A library for robust modular network services](https://github.com/tower-rs/tower) — the definitive implementation of this pattern in the Rust ecosystem.
- [Actix Web Middleware](https://actix.rs/docs/middleware/) & [Axum Middleware](https://docs.rs/axum/latest/axum/middleware/index.html)
- Related: [Backpressure & Rate Limiting](../../architecture-patterns/backpressure-and-rate-limiting/learning.md)
