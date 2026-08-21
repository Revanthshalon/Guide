# Template Method — Learning Notes

## Mental Model

Template Method exists to eliminate boilerplate orchestration. When you find yourself writing the exact same `setup() -> do_work() -> teardown()` or `loop { try() -> backoff() }` across ten different modules, you have a shared skeleton (the template) and varying internal steps (the holes).

Instead of forcing every caller to implement the orchestration, you invert the control flow: you define the skeleton centrally and require the implementor to fill in the holes. The template method *calls* the variant steps, rather than the variant *being called by* a higher-level orchestrator. 

## Structure & Participants

- **Template Trait:** Defines the algorithm's skeleton (the Template Method) and the required steps.
- **Required Steps:** Methods with no default implementation (the "holes" the implementor must fill).
- **Hook Methods:** Optional steps with a default implementation (often no-ops or default behaviors).
- **Template Method:** The provided method that orchestrates the steps. It uses the required steps and hooks to execute the full algorithm.

## Idiomatic Rust Implementation

In Rust, this pattern is implemented using traits with **provided methods**. The standard library is built on this. For instance, `Iterator` only requires you to implement `next()` (the required step). Once you do, the trait provides dozens of template methods (`map`, `filter`, `fold`) that orchestrate calls to your `next()` method.

### Worked Example: The Orchestrated Retry Loop

Consider the universal problem of executing fallible operations with exponential backoff.

**Stage 0 — Scattered Orchestration**
Every network call, database query, and file read across the codebase implements its own `loop { match ... }` logic. Bugs emerge: some loops lack jitter, some back off linearly instead of exponentially, and some retry on non-retryable 400-level HTTP errors.

**Stage 1 — Template Method**
We centralize the loop logic into a provided method on a trait. The variant steps (what to attempt, whether an error is retryable) are the holes.

```rust
use std::time::Duration;
use std::thread::sleep;

pub trait RetryableOperation {
    type Output;
    type Error;

    // Required step: The actual work to perform
    fn attempt(&mut self) -> Result<Self::Output, Self::Error>;

    // Hook step: Optional filter for retryable errors
    // Default implementation assumes all errors are retryable
    fn is_retryable(&self, _error: &Self::Error) -> bool {
        true 
    }

    // The Template Method: The invariant orchestration skeleton
    fn execute_with_retries(&mut self, max_attempts: u32) -> Result<Self::Output, Self::Error> {
        let mut attempts = 0;
        let mut backoff = Duration::from_millis(100);

        loop {
            match self.attempt() {
                Ok(val) => return Ok(val),
                Err(e) => {
                    attempts += 1;
                    // Ask the subclass if we should abort early
                    if attempts >= max_attempts || !self.is_retryable(&e) {
                        return Err(e);
                    }
                    sleep(backoff);
                    backoff *= 2; 
                }
            }
        }
    }
}

// Implementor
pub struct NetworkFetch {
    pub url: String,
}

impl RetryableOperation for NetworkFetch {
    type Output = String;
    type Error = String;

    fn attempt(&mut self) -> Result<Self::Output, Self::Error> {
        println!("Fetching {}...", self.url);
        // ... simulated network call ...
        Err("Network timeout".to_string())
    }

    fn is_retryable(&self, error: &Self::Error) -> bool {
        error.contains("timeout") // Only retry on timeouts
    }
}
```

**The Scoreboard:**
- Stage 0: 50 lines of boilerplate loop logic at every call site. Inconsistent backoff algorithms across the codebase.
- Stage 1: 0 lines of loop logic at the call site. The implementor only writes the domain logic (`attempt`) and classification (`is_retryable`). The exponential backoff is flawlessly standardized.

## Versus

### Template Method vs. Strategy
- **Template Method** relies on inheritance/traits. The template *owns* the control flow and calls into the subclass's steps. (Inversion: Template calls Subclass).
- **Strategy** relies on composition. The caller owns the control flow and calls out to a swappable strategy object. (Inversion: Caller calls Strategy).
- **How to decide:** If the orchestration skeleton is what you want to share, use Template Method. If the orchestration is simple but the entire mechanism needs to be swapped dynamically at runtime, use Strategy.

### Template Method vs. Extension Traits
- Template methods are provided methods within the *same* trait. They can be overridden by the implementor.
- Extension traits place the provided method in a *separate*, blanket-implemented trait (`impl<T: BaseTrait> ExtensionTrait for T`). This prevents the implementor from overriding the template orchestration.

## Pitfalls in Depth

### Pitfall: The `&mut self` State Machine Trap

- **What goes wrong:** A retry loop executes step 1, mutates `self`, fails on step 2, and retries. But because `self` was already mutated, the retry uses corrupted or partially-advanced state, causing a panic or logic bug.
- **Why it happens (the mechanism):** When the Template Method requires `&mut self` to call the steps, the implementor often uses that mutable reference to advance an internal state machine (e.g., buffering data). If the orchestration flow aborts and loops back, the implementor's internal state is not automatically reset.
- **How to handle it, and why that works:** If the orchestration involves retries or loops, the required steps must either be strictly idempotent (not mutating `self` in ways that affect retries), or the Template Method must provide an explicit `reset(&mut self)` hook that it calls before looping.
- **Trade-offs of the fix:** Adding `reset()` complicates the trait API. Enforcing idempotency limits what implementors can do.

### Pitfall: Overriding the Orchestration

- **What goes wrong:** An implementor decides their network fetch needs a linear backoff instead of exponential. They implement `execute_with_retries` directly in their `impl` block, silently discarding your standard backoff skeleton.
- **Why it happens (the mechanism):** Rust doesn't have a `final` keyword. Any provided method in a trait can be overridden by the implementor.
- **How to handle it, and why that works:** Use the **Extension Trait** pattern. Put `attempt()` and `is_retryable()` in a `Retryable` trait. Put `execute_with_retries()` in a `RetryExt` trait with a blanket implementation (`impl<T: Retryable> RetryExt for T`). The compiler strictly forbids overriding methods in blanket implementations.
- **Trade-offs of the fix:** Adds a layer of indirection and requires users to bring two traits into scope to use the method.

### Pitfall: Temporal Coupling (The Fragile Base Class)

- **What goes wrong:** A change to the template's orchestration flow (e.g., calling `format()` before `fetch()`) compiles perfectly but panics at runtime.
- **Why it happens (the mechanism):** Implementors implicitly rely on the order in which the template calls their steps, communicating via hidden state mutations in `&mut self`.
- **How to handle it, and why that works:** Use the type system to enforce order. Instead of sharing state via `&mut self`, have `fetch()` return a strongly-typed `DataToken` that is required as an argument to `format(token: DataToken)`. If the template changes the order, it won't compile.

## Design Decisions & Trade-offs

**Trait vs Closure:** If your template method only needs *one* step filled in, a trait is overkill. Just write a standalone function that takes a closure: `fn execute_with_retry<F>(mut f: F) where F: FnMut() -> Result<...>`. Template Method traits shine when there are multiple distinct steps (e.g., `setup`, `attempt`, `teardown`, `is_retryable`) that share contextual state.

## Exercises & Self-Test

1. Write the Extension Trait structure that completely prevents a user from overriding the `execute_with_retries` method in the worked example.
2. Design a `Transaction` trait using Template Method. It should have required steps `begin`, `commit`, and `rollback`, and a template method `run(work)` that automatically rolls back if `work` returns an error. 
3. *Design Challenge:* How do you design the `Transaction` trait to avoid the temporal coupling pitfall? (Hint: how do you ensure the user's `work` closure actually executes *inside* the transaction context?)
4. What is the fundamental difference in control flow between Template Method and Strategy?

## References

- [Rust API Guidelines: Extension Traits](https://rust-lang.github.io/api-guidelines/future-proofing.html) — for preventing template overrides.
- GoF Book: Template Method chapter.
