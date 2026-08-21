# RAII & Drop Guards — Learning Notes

## Mental Model

Every resource acquisition is a liability. When you open a file, lock a mutex, allocate heap memory, or start a database transaction, you take on a debt that must eventually be paid by releasing the resource. In many languages, you pay this debt manually using `try/finally` or `defer` statements. The problem with manual cleanup is that you must have perfect memory on every single exit path, including error returns and panics.

Resource Acquisition Is Initialization (RAII) forces the compiler to pay the debt. By wrapping the resource inside a struct, you tie its lifecycle directly to the lexical scope of a variable. When the variable goes out of scope—whether through a normal return, an early `?` error propagation, or an unwinding panic—the compiler destroys the variable and runs its `Drop` implementation. The resource is released.

A **Drop Guard** is a specific application of RAII: an object whose primary purpose is to execute cleanup logic (like unlocking a mutex, decrementing a counter, or rolling back a transaction) when it is dropped, rather than just freeing memory.

## Structure & Participants

- **The Resource:** The underlying system construct that needs management (a file descriptor, a mutex lock, a pending database transaction).
- **The Guard:** A struct that typically acquires the resource in its constructor (`new`, `lock`, `begin`) and unconditionally releases it in its destructor (`drop`).
- **The Scope:** The lexical block where the guard is bound to a variable. This defines the exact lifetime of the resource.

## Idiomatic Rust Implementation

In Rust, any struct implementing the `Drop` trait is an RAII guard.

```rust
use std::sync::Mutex;
use std::fs::File;

struct Context {
    // Fields drop in declaration order (top to bottom).
    // Here, `log_file` will drop before `db_conn`.
    log_file: File,
    db_conn: String,
}

fn process_shared_state(m: &Mutex<i32>) {
    // The lock is acquired here. `guard` is a MutexGuard.
    let mut guard = m.lock().unwrap();
    
    *guard += 1;
    
    // No explicit unlock() is needed. When `guard` goes out of scope 
    // at the end of the block, its Drop implementation runs.
}
```

## When This Pattern Dissolves in Rust

This pattern *does not dissolve in Rust* — it is the bedrock of the language. Rust's ownership model is fundamentally built on RAII. You don't need a garbage collector, and you don't need `try/finally` blocks. The language is structured specifically to make RAII foolproof.

## Worked Example

A common scenario in robust systems is ensuring a transaction rolls back if an error occurs mid-flight, but commits if everything succeeds.

**Stage 0: Manual rollback (Bug-prone)**

```rust
fn process_order_manual(id: &str) -> Result<(), &'static str> {
    println!("BEGIN TRANSACTION {}", id);
    
    if let Err(e) = execute("UPDATE inventory") {
        println!("ROLLBACK TRANSACTION {}", id); // Cleanup on error path 1
        return Err(e);
    }
    
    if let Err(e) = execute("UPDATE accounts") {
        println!("ROLLBACK TRANSACTION {}", id); // Cleanup on error path 2
        return Err(e);
    }
    
    println!("COMMIT TRANSACTION {}", id);
    Ok(())
}
```
Every new step requires another manual rollback. If you use `?`, you completely bypass the manual rollback!

**Stage 1: The Drop Guard**

We can create a guard that assumes failure unless explicitly told otherwise.

```rust
pub struct DbTransaction {
    id: String,
    committed: bool,
}

impl DbTransaction {
    pub fn begin(id: &str) -> Self {
        println!("BEGIN TRANSACTION {}", id);
        DbTransaction { id: id.to_string(), committed: false }
    }

    pub fn execute(&self, query: &str) -> Result<(), &'static str> {
        println!("Executing {} in tx {}", query, self.id);
        if query.contains("ERROR") {
            Err("Query failed")
        } else {
            Ok(())
        }
    }

    pub fn commit(mut self) {
        println!("COMMIT TRANSACTION {}", self.id);
        self.committed = true;
    }
}

impl Drop for DbTransaction {
    fn drop(&mut self) {
        // Only rollback if we didn't explicitly commit!
        if !self.committed {
            println!("ROLLBACK TRANSACTION {}", self.id);
        }
    }
}

fn process_order() -> Result<(), &'static str> {
    let tx = DbTransaction::begin("1234");
    
    tx.execute("UPDATE inventory")?;       // Uses ? safely
    tx.execute("UPDATE accounts ERROR")?;  // Fails here, returning early
    
    tx.commit(); // Never reached
    Ok(())
}
```
When `execute` returns an `Err`, the `?` operator immediately returns from the function. The `tx` variable goes out of scope, its `Drop` runs, sees `committed == false`, and executes the rollback automatically.

## Versus

### Garbage Collection (GC)
- **What's the same:** Both free you from manual `free()` calls for memory.
- **What's different:** GC only handles memory, and its timing is non-deterministic. It ignores other resources like file handles or locks, forcing you to use `finally` blocks anyway. RAII manages *all* resources deterministically the exact moment the variable goes out of scope.

### `try / finally` (or Go's `defer`)
- **What's the same:** Guarantees cleanup code runs.
- **What's different:** `finally` and `defer` require the programmer to remember to write the cleanup at every use site. RAII embeds the cleanup inside the type itself.

## Pitfalls in Depth

### Pitfall: `ManuallyDrop` and Memory Leaks

- **What goes wrong:** You need to pass a resource (like a heap-allocated buffer) to a C API via FFI. Rust drops and frees it at the end of the block, leaving the C code holding a dangling pointer.
- **Why it happens (the mechanism):** The compiler unconditionally calls `drop()` on variables exiting scope, regardless of whether you passed a raw pointer of its internals elsewhere.
- **How to handle it, and why that works:** Wrap the value in `std::mem::ManuallyDrop` (or use `std::mem::forget`). This tells the compiler *not* to generate the `drop()` call, transferring ownership (and cleanup responsibility) to the C side.
- **Trade-offs of the fix:** You are explicitly leaking memory from Rust's perspective. If the C code doesn't free it, you have a permanent leak.

### Pitfall: Deadlocks from temporary Drop Guards

- **What goes wrong:** You write `let _ = mutex.lock().unwrap();` to acquire a lock, but the lock doesn't protect the code on the next line. Or, you write `let _guard = mutex.lock().unwrap();` and hold it across a long-running HTTP request, starving other threads.
- **Why it happens (the mechanism):** Binding a guard to `_` drops it *immediately* at the end of that statement. Binding it to `_guard` keeps it alive until the end of the lexical block. If the block contains slow operations, the lock is held unnecessarily long.
- **How to handle it, and why that works:** For tight scoping, create explicit inner scopes: `{ let guard = m.lock().unwrap(); guard.update(); } // drops here`. Or explicitly call `drop(guard)` as soon as the critical section is done.
- **Trade-offs of the fix:** Extra block scopes can make code deeply indented and harder to read. Manual `drop()` calls re-introduce a bit of manual resource management.

### Pitfall: Panicking inside a Drop implementation

- **What goes wrong:** Your program aborts instantly without unwinding, taking down the entire process—even if you had a `catch_unwind` set up.
- **Why it happens (the mechanism):** If a thread is *already* panicking (unwinding the stack), it drops variables as it goes. If one of those `Drop` implementations panics, Rust is faced with a double-panic. Unwinding twice simultaneously is impossible, so Rust immediately aborts the process (`std::process::abort()`).
- **How to handle it, and why that works:** `Drop` implementations must never panic. Use `.unwrap_or()` or log the error instead of using `.unwrap()` or `panic!()` inside a `drop()` method.
- **Trade-offs of the fix:** Silent failures in destructors. If a file fails to flush on close, you might just have to log it and move on, meaning the caller won't get a proper error code.

## Design Decisions & Trade-offs

**Drop Order:** In Rust, local variables are dropped in reverse order of declaration (LIFO). However, **fields within a struct are dropped in declaration order (top to bottom)**. If your struct contains a `File` and a `Connection`, and the connection requires the file to be open during its own teardown, you must declare them in the correct order.

**Silent overhead:** RAII can hide expensive operations. When a variable goes out of scope, a massive cascade of destructors might run, potentially causing unexpected I/O or delays at a simple closing brace `}`.

**Async contexts:** Holding an RAII guard (like a standard library `MutexGuard`) across an `.await` point is extremely dangerous and often fails to compile. The guard might be sent to another thread, but OS mutexes must be unlocked on the same thread that locked them. Use async-aware mutexes (e.g., `tokio::sync::Mutex`) if you must hold a lock across `.await`.

## Exercises & Self-Test

1. In a struct with fields `a: File`, `b: Socket`, and `c: MutexGuard`, in what exact order are they dropped when the struct goes out of scope?
2. Write a `Stopwatch` struct that records `Instant::now()` in its constructor and prints the elapsed time in its `Drop` implementation. Use it to measure a `std::thread::sleep`.
3. Why does writing `let _ = my_mutex.lock().unwrap();` usually indicate a bug, and what is the fix?
4. In the `DbTransaction` example, what happens if `execute` panics? Does the transaction commit or rollback? Why?

## Open Questions

- What is the idiomatic way to handle fallible cleanup in Rust, given that `drop` cannot return a `Result`?
- How does `Pin` interact with `Drop` for self-referential structs in async Rust?

## References

- [Rust Book: The Drop Trait](https://doc.rust-lang.org/book/ch15-03-drop.html)
- [Rust Reference: Destructors](https://doc.rust-lang.org/reference/destructors.html)
- Cross-ref: Lock-Free Concurrency (`../../performance-optimization/lock-free-concurrency/learning.md`)
