# Singleton & Shared State — Reference

**One-Liner:** Ensure a class has only one instance and provide a global point of access to it, carefully managing concurrency.

## When to Use
- When exactly one instance of a resource is required globally (e.g., global configuration, logging sink).
- When passing a reference deeply through the call stack (Prop Drilling) becomes architecturally unreasonable.

## Structure Sketch
```rust
use std::sync::OnceLock;

static CONFIG: OnceLock<Config> = OnceLock::new();

fn get_config() -> &'static Config {
    CONFIG.get_or_init(|| load_config_from_file())
}
```

## Rust Idiom
- Use `std::sync::OnceLock` for lazy initialization when the initializer needs runtime arguments (e.g. a path from argv).
- Use `std::sync::LazyLock` (stable since 1.80) when the initializer is a fixed closure with no external inputs — it keeps the init logic at the `static` declaration instead of repeating it at every call site.
- Use `std::sync::atomic` types for simple global counters.
- Use `Mutex` or `RwLock` inside the `OnceLock`/`LazyLock` if the singleton must be mutated, but expect severe testing difficulties.

## Versus
- **Dependency Injection:** DI passes state explicitly, avoiding hidden coupling and making testing trivial. DI is preferred in Rust.
- **Thread-Local Storage:** `std::thread_local!` creates one instance *per thread*, preventing thread contention but duplicating state.

## Pitfalls

| Pitfall | Mechanism | Fix | Trade-off |
| :--- | :--- | :--- | :--- |
| **Hidden Dependencies** | Functions silently rely on global state. | Pass state as explicit arguments. | More verbose function signatures. |
| **Testing State Bleed** | Concurrent tests modify the same global state, failing randomly. | Avoid global mutability or mock the state. | Requires architecture changes to support injection. |
| **Deadlocks** | Multiple global mutexes lock in the wrong order. | Minimize lock scope; never hold across awaits. | Might require cloning data out of the lock. |

## Rules of Thumb
- Always try to pass state explicitly first.
- If you must use a global, make it immutable (`OnceLock<T>`).
- If you must use a mutable global, use atomics if possible.

## Key References
- [Rust standard library - OnceLock](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)
- [Rust API Guidelines: Global State](https://rust-lang.github.io/api-guidelines/future-proofing.html)
