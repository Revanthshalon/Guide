# RAII & Drop Guards — Quick Reference

## One-Liner

Tie resource management to lexical scope by acquiring the resource upon initialization and releasing it in the `Drop` implementation, guaranteeing cleanup even on error paths.

## When to Use

| Use it when | Avoid it when |
| --- | --- |
| Managing OS resources (File descriptors, network sockets). | The cleanup can fail and the caller must explicitly handle that failure (`Drop` cannot return a `Result`). |
| Managing synchronization primitives (Mutexes, RwLocks). | The resource cleanup requires async operations (async `Drop` is not currently in stable Rust). |
| You need guaranteed rollback/cleanup on early returns (`?`) or panics (e.g. Database Transactions). | You are passing ownership of the resource to a C API via FFI (use `ManuallyDrop` instead). |

## Structure Sketch

```rust
struct ResourceGuard {
    resource_id: String,
}

impl ResourceGuard {
    fn acquire(id: &str) -> Self {
        // Acquire resource here
        Self { resource_id: id.to_string() }
    }
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        // Release resource here unconditionally
    }
}
```

## Rust Idiom

Rust fundamentally relies on RAII; it replaces `try/finally` blocks entirely. Any type needing cleanup implements `Drop`. The compiler guarantees `drop()` is called when the variable goes out of scope, whether via normal execution, `?` propagation, or a panic.

## Versus

| Confused with | Key difference |
| --- | --- |
| Garbage Collection | GC cleans up memory non-deterministically. RAII cleans up *any* resource (memory, files, locks) synchronously and deterministically. |
| `try/finally` blocks | `finally` forces the programmer to manually write cleanup code at every call site. RAII encodes the cleanup in the type itself, making it impossible to forget. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Deadlocks from long scopes** | Create an explicit `{ let guard = m.lock(); }` block or call `drop(guard)` early. | `let _guard = m.lock()` holds the lock until the very end of the function block. |
| **Panicking in Drop** | Never use `.unwrap()` or `panic!()` inside `drop()`. Log failures instead. | A panic in a drop while the thread is *already* panicking causes an immediate process abort. |
| **FFI dangling pointers** | Wrap the resource in `std::mem::ManuallyDrop` or use `std::mem::forget`. | By default, Rust drops variables exiting scope; if C expects to own it, Rust will corrupt it. |
| **Silent immediate drops** | Always bind guards to a named variable: `let _guard = ...`. | `let _ = m.lock();` drops the lock *immediately*, offering no protection for the next line. |

## Rules of Thumb

- **Struct field drop order:** Local variables drop LIFO (last declared, first dropped). Struct fields drop LIFO? NO. Struct fields drop in **declaration order (top to bottom)**. Order your struct fields carefully if they depend on each other.
- **Fail-safe by default:** If a guard manages a transaction, it should rollback in `Drop` unless an explicit `commit()` was called.
- **Async caution:** Do not hold a standard library `MutexGuard` across an `.await` point.

## Key References

- [Rust Book: The Drop Trait](https://doc.rust-lang.org/book/ch15-03-drop.html)
- [Rust Reference: Destructors](https://doc.rust-lang.org/reference/destructors.html)
- [std::mem::ManuallyDrop](https://doc.rust-lang.org/std/mem/struct.ManuallyDrop.html)
