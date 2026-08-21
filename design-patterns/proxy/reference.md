# Proxy — Quick Reference

## One-Liner

Structurally replaces a target object to control access, intercepting calls to handle lifecycle management, lazy loading, caching, or remote communication before delegating to the real subject.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| You need to manage the lifecycle of a resource transparently (e.g., Connection Pooling). | The overhead of the proxy outweighs the cost of the real object. |
| Object creation is expensive and should be deferred (Virtual Proxy). | You want to add composable, dynamic behavior (use Decorator). |
| You must restrict access to an object structurally (Protection Proxy). | You need to change the interface to match a client's expectations (use Adapter). |

## Structure Sketch

```rust
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

struct Resource;
struct Pool { conns: Mutex<Vec<Resource>> }

// The Proxy controls the resource's return to the pool
struct PooledResource {
    conn: Option<Resource>,
    pool: Arc<Pool>,
}

impl Deref for PooledResource {
    type Target = Resource;
    fn deref(&self) -> &Self::Target { self.conn.as_ref().unwrap() }
}

impl Drop for PooledResource {
    fn drop(&mut self) {
        if let Some(c) = self.conn.take() {
            self.pool.conns.lock().unwrap().push(c);
        }
    }
}
```

## Rust Idiom

Rust handles most proxy needs natively without custom traits:
- **Virtual Proxies:** `OnceLock`, `LazyLock`, or `once_cell`.
- **Smart Pointers:** `Box`, `Rc`, `Arc`, `MutexGuard` (which acts as a locking proxy).
- Transparent custom proxies use `Deref` to seamlessly impersonate the underlying type.

## Versus

| Confused with | Key difference |
| --- | --- |
| Decorator | Decorator *adds behavior* dynamically; Proxy *controls access* structurally. |
| Adapter | Adapter *changes the interface*; Proxy *maintains the exact same interface*. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| `Deref` Security Leak | Do not implement `Deref` on Protection Proxies. | Exposing `&RealSubject`, allowing clients to bypass proxy checks. |
| Remote Lifetime conflicts | Have the trait return owned data (`String`) or `Cow` instead of `&T`. | The borrow checker rejecting remote proxies returning temporary values. |
| Proxy Bloat | Use middleware/decorators for stacking distinct behaviors. | A single proxy that caches, logs, and authenticates all at once. |

## Rules of Thumb

- If the proxy is purely lifecycle/loading, use `Deref`.
- If the proxy enforces security, *never* use `Deref`.
- A proxy should ideally have the same size footprint as the overhead of what it manages (e.g., just an `Arc` pointer).

## Key References

- [std::sync::OnceLock](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)
- [std::ops::Deref](https://doc.rust-lang.org/std/ops/trait.Deref.html)
