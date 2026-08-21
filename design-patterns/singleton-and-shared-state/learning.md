# Singleton & Shared State — Learning Notes

## Mental Model

The Singleton pattern ensures that a class has exactly one instance and provides a global point of access to it. It exists to solve the constraint of managing shared, central resources—like a configuration manager, a global metrics counter, or a hardware interface.

In Rust, the strict aliasing rules (you cannot have mutable references aliased) make traditional Singletons extremely difficult and explicitly discouraged. Rust forces you to confront the reality of shared state: if multiple parts of your program can access a global variable concurrently, you must synchronize that access, or you will cause data races. Therefore, in Rust, the Singleton pattern shifts from "how do I make a global instance" to "how do I safely share state?"

## Structure & Participants

### The Global Instance
- **Role:** The singleton object.
- **In Rust:** Usually implemented as a `static` variable wrapped in a synchronization primitive like `OnceLock`, `Mutex`, or `RwLock`.

## Idiomatic Rust Implementation & Worked Example

### Stage 0: Hidden Initialization Graphs (The Danger)

In traditional OOP, Singletons often initialize themselves or depend on other Singletons invisibly. This creates hidden temporal coupling (A must initialize before B, but it's not enforced by the compiler).

```rust
// A hypothetical unsafe or badly designed global state
// If Logger initializes before Config, it crashes.
// There is no compiler guarantee of initialization order.
```

### Stage 1: Safe Global Initialization with `OnceLock`

`std::sync::OnceLock` is the modern, idiomatic way to create global singletons that are initialized exactly once, at runtime, thread-safely.

```rust
use std::sync::OnceLock;

// Global static using OnceLock
static CONFIG: OnceLock<String> = OnceLock::new();

fn get_config() -> &'static String {
    CONFIG.get_or_init(|| {
        // This closure runs exactly once, even if called concurrently by multiple threads.
        println!("Initializing config...");
        "Global Config".to_string()
    })
}

// Usage
fn main() {
    let config = get_config();
    println!("Config: {}", config);
}
```

### Stage 2: Interior Mutability with Atomics

If your global singleton needs to be mutable (like a metrics counter), you cannot use `&mut T`. You must use interior mutability. For counters, `Atomic` types are best. For complex structs, `Mutex` or `RwLock`.

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

// Atomic counter for shared state
static METRICS_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn record_metric() {
    // Relaxed ordering is fine for a simple counter without data dependencies
    METRICS_COUNTER.fetch_add(1, Ordering::Relaxed);
}

fn main() {
    record_metric();
    record_metric();
    println!("Counter: {}", METRICS_COUNTER.load(Ordering::Relaxed));
}
```

### Stage 3: `LazyLock` for Non-`fn`-Pointer Initializers

`OnceLock::get_or_init` requires passing the initializer closure at every call site. `std::sync::LazyLock` (stable since Rust 1.80) inlines the closure into the static declaration itself, so the initialization logic lives in one place instead of being repeated (or duplicated with `unwrap()`-on-panic mismatches) at every call site.

```rust
use std::sync::LazyLock;

static CONFIG: LazyLock<String> = LazyLock::new(|| {
    println!("Initializing config...");
    "Global Config".to_string()
});

fn main() {
    println!("Config: {}", *CONFIG); // Initializes on first access, thread-safely.
}
```
Prefer `LazyLock` when there's exactly one way to build the value; prefer `OnceLock` when the initializer depends on runtime arguments not known at the `static` definition site (e.g. a config path from `main`'s argv).

## When This Pattern Dissolves in Rust

Singletons almost entirely dissolve in Rust in favor of **Dependency Injection**. Passing an `Arc<Config>` or `Arc<Mutex<State>>` down the call stack or through context structs is universally preferred over global `static` variables.

## Versus

- **Dependency Injection:** DI passes the single instance explicitly as an argument. Singleton grabs it implicitly from the global namespace. DI is strongly preferred in Rust.
- **Thread-Local Storage:** Instead of a global singleton, `std::thread_local!` gives each thread its own unique instance. Useful for thread-specific caching.

## Pitfalls in Depth

### 1. Hidden Dependencies and Temporal Coupling
- **What goes wrong:** Function `A` relies on the global state, but it fails or crashes because Function `B` hasn't initialized the state yet.
- **Why it happens:** Global variables disconnect the data dependency from the function signature. The compiler cannot enforce initialization order.
- **How to handle it, and why that works:** Pass the state explicitly as an argument (Dependency Injection). If you must use global state, use `OnceLock` so it initializes lazily and safely on first access.
- **Trade-offs of the fix:** Explicit argument passing requires changing function signatures throughout the call stack.

### 2. Testing Difficulty and State Bleed
- **What goes wrong:** Unit tests pass individually but fail when run in parallel (which `cargo test` does by default).
- **Why it happens:** Tests modify the same global singleton concurrently, polluting the state for other tests.
- **How to handle it, and why that works:** Avoid global mutability entirely. If required, inject the state into the system being tested so each test can create its own isolated instance.
- **Trade-offs of the fix:** Requires architectural changes to support injection.

### 3. Deadlocks in Global Mutexes
- **What goes wrong:** The application completely freezes.
- **Why it happens:** Thread A locks Global Mutex X, then tries to lock Global Mutex Y. Thread B locks Y, then tries to lock X. Or, a function locks the global mutex and then calls another function that tries to lock it again.
- **How to handle it, and why that works:** Keep lock scopes as small as possible. Never hold a global lock across asynchronous `.await` points or while calling external functions.
- **Trade-offs of the fix:** Requires careful auditing of lock scopes and can lead to cloning data out of the lock, increasing memory allocations.

## Design Decisions & Trade-offs

- **OnceLock vs lazy_static:** `std::sync::OnceLock` is in the standard library and should replace the external `lazy_static` crate for most new code.
- **Global vs Local:** Always default to local variables and passing by reference. Reach for globals only for cross-cutting concerns like logging, metrics, or global configuration that genuinely never changes once loaded.

## Exercises & Self-Test

1. **Build Exercise:** Implement a global configuration singleton using `OnceLock`. Write a test that spawns 10 threads, all attempting to access the config simultaneously, and prove it only initializes once.
2. **Build Exercise:** Refactor a function that reads from a global `Mutex<State>` to instead accept an `Arc<Mutex<State>>` as a parameter.
3. Why does `cargo test` expose singleton-heavy architectures as problematic?

## Open Questions

- When is `OnceLock` preferable to passing an `Arc` down from `main()`?
- How do you mock a global `OnceLock` singleton for unit testing?

## References

- [Rust standard library - OnceLock](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)
- [Rust standard library - Atomics](https://doc.rust-lang.org/std/sync/atomic/index.html)
