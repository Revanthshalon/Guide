# Chain of Responsibility — Quick Reference

## One-Liner

Pass a request along a sequence of handlers where each link can completely handle, short-circuit reject, or mutate-and-pass the request — decoupling the sender from the specific receiver.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| Applying middleware (auth, logging, rate limiting) to requests. | Execution order doesn't matter (use Observer). |
| You need to short-circuit processing on failure (e.g., validations). | You have a static, simple 3-step process with no variations. |
| You are structuring a pipeline of sequential checks. | |

## Structure Sketch

```rust
// Static nesting (Tower-style) is the Rust idiom for Middleware
pub trait Service<Request> {
    type Response;
    fn call(&mut self, req: Request) -> Self::Response;
}

pub struct AuthLayer<S> {
    inner: S,
}

impl<S, Req> Service<Req> for AuthLayer<S>
where
    S: Service<Req>, // Requires the inner service to also implement Service
{
    type Response = S::Response;
    
    fn call(&mut self, req: Req) -> Self::Response {
        if is_unauthorized(&req) {
            return reject(); // Short-circuit early return
        }
        self.inner.call(req) // Pass to next in chain
    }
}
```

## Rust Idiom

1. **For request pipelines:** Nested structs implementing a `Service` trait (static dispatch). This avoids runtime overhead but creates complex types at compile time.
2. **For simple logic flows:** The `?` operator. The `Result` chain *is* a built-in Chain of Responsibility that short-circuits on `Err` and passes on `Ok`.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Decorator** | Decorator augments behavior to a successful call; Chain focuses on the *decision* to handle, pass, or short-circuit. |
| **Command** | Command is the *object* being passed; Chain is the *pipeline* processing it. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Silent drop-off** | Ensure the innermost link of the chain is a terminal handler (e.g., a 404 response). | Requests disappearing into the void with no log or error. |
| **Fat middleware** | Strictly separate concerns: one layer = one reason to reject. | An Auth layer that also parses JSON and checks the database. |
| **Order dependency corruption** | Document ordering carefully; avoid implicitly coupled extensions. | Placing Rate Limiting after Body Parsing, opening DOS vectors. |
| **Unreadable nested types** | Use opaque types (`impl Service`) or `Box<dyn Service>` at boundaries if nesting gets too deep. | Compile errors featuring a 400-character nested type signature (`Log<Auth<Db<...>>>`). |

## Rules of Thumb

- Prefer **static nesting** (`Inner<Outer>`) over dynamic lists (`Vec<Box<dyn Handler>>`) unless the chain *must* be modified dynamically at runtime.
- A handler should do exactly one of three things: Handle completely, Reject completely, or Mutate and pass to `inner`.
- If a link passes to `inner`, it should return whatever `inner` returns, unless it is specifically designed as a post-processing link (like a logger or metric tracker).

## Key References

- [Tower Architecture](https://github.com/tower-rs/tower)
- Related: [Backpressure & Rate Limiting](../../architecture-patterns/backpressure-and-rate-limiting/learning.md)
