# Proxy — Learning Notes

## Mental Model

A Proxy exists to solve a fundamental constraint: you have a caller and a target object, but direct access to the target is either unsafe, excessively expensive, structurally impossible, or requires lifecycle management (like connection pooling). The Proxy structurally replaces the target by implementing the exact same interface, effectively "impersonating" the real object. 

Instead of changing the caller to handle the complexity of checking permissions, tracking reference counts, lazily loading data, or traversing a network, the Proxy intercepts the calls. It performs the necessary auxiliary work, then either forwards the call to the real subject or handles the response itself.

## Structure & Participants

- **Subject Trait / Interface:** Defines the common contract. In Rust, this is often implicit via the `Deref` trait, which allows the proxy to seamlessly expose the subject's methods.
- **Real Subject:** The concrete underlying structure that actually performs the heavy lifting, holds the data, or manages the raw resource.
- **Proxy:** A struct that wraps or points to the Real Subject. It intercepts method calls to enforce rules, handle caching, or manage lifetimes before forwarding execution to the underlying subject.

## Idiomatic Rust Implementation

In Rust, the Proxy pattern is heavily integrated into the language and standard library via smart pointers. `Box`, `Rc`, `Arc`, `MutexGuard`, and `RefCell` are all proxies. Rust uses the `Deref` and `DerefMut` traits to make these transparent.

Here is an example of a **Connection Pool Proxy**, which manages the lifecycle of a resource. Instead of trusting the client to return a connection to the pool, the proxy owns the connection and returns it automatically upon dropping.

```rust
use std::sync::{Arc, Mutex};
use std::ops::{Deref, DerefMut};

pub struct Connection {
    id: usize,
}

impl Connection {
    pub fn execute(&self, query: &str) {
        println!("Conn {}: Executing {}", self.id, query);
    }
}

pub struct Pool {
    conns: Mutex<Vec<Connection>>,
}

impl Pool {
    pub fn new(size: usize) -> Arc<Self> {
        let mut conns = Vec::with_capacity(size);
        for id in 0..size {
            conns.push(Connection { id });
        }
        Arc::new(Self { conns: Mutex::new(conns) })
    }

    pub fn acquire(self: &Arc<Self>) -> Option<PooledConnection> {
        let conn = self.conns.lock().unwrap().pop()?;
        Some(PooledConnection {
            conn: Some(conn),
            pool: Arc::clone(self),
        })
    }
}

pub struct PooledConnection {
    conn: Option<Connection>,
    pool: Arc<Pool>,
}

// Transparently act like the Real Subject
impl Deref for PooledConnection {
    type Target = Connection;
    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().unwrap()
    }
}

impl DerefMut for PooledConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn.as_mut().unwrap()
    }
}

// The core logic of the Proxy: lifecycle management
impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.pool.conns.lock().unwrap().push(conn);
        }
    }
}
```

## When This Pattern Dissolves in Rust

You rarely write custom trait-based proxies in Rust because the ecosystem provides them off-the-shelf:
- **Virtual Proxy (Lazy Loading):** Handled by `std::sync::OnceLock`, `std::sync::LazyLock`, or `once_cell`.
- **Protection Proxy:** Handled by ownership rules, module visibility (`pub(crate)`), or locking primitives (`Mutex`, `RwLock`).
- **Remote Proxy:** Handled by RPC frameworks (like `tonic` for gRPC) that generate client stubs.

When you do write one, implementing `Deref` dissolves the boilerplate of manually mirroring methods.

## Worked Example

Consider building an HTTP client that occasionally needs to cache responses to avoid hammering an external API. 

**Stage 0 — No Proxy (Manual Caching)**
The caller manually checks a cache before making a request.
```rust
// Caller code is cluttered with caching logic
if let Some(cached) = cache.get("user/123") {
    return cached;
}
let res = client.fetch("user/123").await;
cache.insert("user/123", res.clone());
return res;
```
This forces all callers to know about the cache, risking cache misses or redundant requests if someone forgets the pattern.

**Stage 1 — The Caching Proxy**
We introduce a proxy that implements the exact same fetching interface.

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// The trait representing the interface
pub trait Fetcher {
    fn fetch(&self, path: &str) -> String;
}

// The Real Subject
pub struct HttpFetcher;
impl Fetcher for HttpFetcher {
    fn fetch(&self, path: &str) -> String {
        // Expensive network call
        format!("Response from {}", path)
    }
}

// The Proxy
pub struct CachingProxy<T: Fetcher> {
    inner: T,
    cache: Mutex<HashMap<String, String>>,
}

impl<T: Fetcher> CachingProxy<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            cache: Mutex::new(HashMap::new()),
        }
    }
}

impl<T: Fetcher> Fetcher for CachingProxy<T> {
    fn fetch(&self, path: &str) -> String {
        let mut cache = self.cache.lock().unwrap();
        if let Some(cached) = cache.get(path) {
            return cached.clone();
        }
        
        let result = self.inner.fetch(path);
        cache.insert(path.to_string(), result.clone());
        result
    }
}
```
**Scoreboard:**
- Without Proxy: Callers manage cache state, leading to scattered logic.
- With Proxy: Callers just use the `Fetcher` trait. The proxy intercepts, checks the cache, and only hits the real `HttpFetcher` if needed.

## Versus

### Proxy vs. Decorator
- **Same:** Both wrap an underlying object and maintain its interface.
- **Different:** *Intent.* A Decorator *adds behavior* (logging, metrics, retries) and allows clients to stack multiple decorators dynamically. A Proxy *controls access* (lazy loading, auth, caching) and is usually instantiated statically by the system, not configured by the client.

### Proxy vs. Adapter
- **Same:** Both act as middlemen.
- **Different:** Adapter *changes the interface* to bridge incompatible things. Proxy *keeps the same interface*.

## Pitfalls in Depth

### Pitfall: `Deref` Leaking Protection

- **What goes wrong:** A Protection Proxy implements `Deref`, allowing the client to access the underlying object directly.
- **Why it happens (the mechanism):** Rust's `Deref` coercion is ergonomic, and it's tempting to use it to avoid writing pass-through methods. But `Deref` returns a reference (`&RealSubject`). Once the client has that, the Proxy's access control is entirely bypassed.
- **How to handle it, and why that works:** Never use `Deref` for Protection Proxies. You must manually implement safe pass-through methods and enforce access logic explicitly. 
- **Trade-offs of the fix:** Higher boilerplate, as you must explicitly mirror the safe parts of the underlying interface.

### Pitfall: Lifetimes in Remote Proxies

- **What goes wrong:** A proxy holding a network connection struggles to return references with the correct lifetimes, causing borrow checker conflicts.
- **Why it happens (the mechanism):** The Proxy pattern in classic OOP assumes the proxy and the real subject can be swapped perfectly. In Rust, a real subject might return `&Data` with a lifetime tied to `&self`. A remote proxy has to fetch data over the network, deserialize it into an owned struct, and then try to return a reference to it—which fails because the owned struct is dropped at the end of the proxy's method.
- **How to handle it, and why that works:** The trait must be designed to return owned data (e.g., `String` or `Vec<u8>`) or `Cow` instead of borrowed references, so the remote proxy can satisfy the signature.
- **Trade-offs of the fix:** Forces the real local subject to also return owned data, potentially adding unnecessary allocations to the fast path.

### Pitfall: Fat Proxies (Overusing Proxies for Behavior)

- **What goes wrong:** A proxy accumulates caching, rate-limiting, logging, and authentication into a single monolithic wrapper.
- **Why it happens (the mechanism):** Once a proxy intercepts a call, it's trivial to add "just one more check" before forwarding it.
- **How to handle it, and why that works:** If you are stacking distinct behaviors, you want a Decorator or a middleware pipeline (like `tower` Layers), which composes discrete behaviors cleanly. 
- **Trade-offs of the fix:** Middleware architectures add some structural complexity and indirection compared to a simple struct wrapper.

## Design Decisions & Trade-offs

**To `Deref` or Not To `Deref`.**
If the proxy is purely for lifecycle management or lazy loading, `Deref` is idiomatic. If the proxy enforces security or business rules, `Deref` is a vulnerability.

**Trait vs. Struct Proxying.**
In Rust, proxies don't always require a trait. If you control the codebase, a proxy struct that wraps the subject and just provides the same method names is often sufficient, avoiding dynamic dispatch overhead. 

## Exercises & Self-Test

1. Design a memory-budgeting proxy for an image processing library. The proxy should enforce a global cap on concurrent memory usage, blocking if the limit is exceeded.
2. Build a proxy around a `std::fs::File` that automatically deletes the underlying file from the filesystem when the proxy is dropped.
3. Why does returning a `Cow<'a, str>` instead of `&'a str` help solve the lifetime pitfall in remote proxies?

## Open Questions

- When using gRPC Remote Proxies in Rust (via `tonic`), how do we elegantly mask network timeout configurations without leaking them into the core business logic?
- Are there patterns to automatically generate proxy boilerplate (like `derive` macros) for protection proxies without using `Deref`?

## References

- "Design Patterns: Elements of Reusable Object-Oriented Software" (GoF)
- Rust Documentation: `std::ops::Deref`
- `tower` crate documentation (for comparing Proxy to Middleware/Decorators)
