# Bridge — Quick Reference

## One-Liner

Separates a high-level abstraction (policy) from its low-level implementation (mechanism) into two independent trait/struct hierarchies, linking them via composition.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| You have two orthogonal dimensions of variability (e.g., HTTP Clients and TLS Backends) | The abstraction and implementation are tightly coupled and won't change independently |
| You want to swap backend engines without changing the domain logic | You only have one dimension of variation (standard trait implementation is sufficient) |

## Structure Sketch

```rust
// 1. Implementor (Mechanism)
pub trait StorageBackend {
    fn store_bytes(&self, key: &str, data: &[u8]);
}

// 2. Concrete Implementors
pub struct RedisBackend;
impl StorageBackend for RedisBackend {
    fn store_bytes(&self, key: &str, data: &[u8]) {}
}

// 3. Abstraction (Policy)
pub struct UserStore<B: StorageBackend> {
    backend: B,
}

impl<B: StorageBackend> UserStore<B> {
    pub fn save_user(&self, user_id: u64, name: &str) {
        // High-level logic delegating to low-level primitives
        let key = format!("user:{}", user_id);
        self.backend.store_bytes(&key, name.as_bytes());
    }
}
```

## Rust Idiom

- **Generics (`<T: Trait>`):** The default Rust bridge. Provides zero-cost static dispatch via monomorphization.
- **Trait Objects (`Box<dyn Trait>`):** Use when you need dynamic dispatch, heterogeneous collections, or to hide complex types at API boundaries.
- **The pattern dissolves:** Rust's core design (structs + traits) naturally forces the Bridge pattern, making it the default way to write reusable Rust code rather than a specialized design pattern.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Adapter** | Adapter reconciles incompatible interfaces *after* the fact. Bridge is designed *up front* to decouple layers. |
| **Strategy** | Strategy swaps a specific algorithm inside a class. Bridge separates the entire structural abstraction from the implementation mechanism. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Fat Implementors** | Keep the trait focused strictly on universal primitives. Configure specific backends before bridging. | Traits accumulating backend-specific methods (`set_openssl_cipher()`). |
| **Lifetime Hell** | Make the Abstraction own the Implementor (`<T: Backend>`) or use `Arc<dyn Backend>`. | Defining the bridge with `&'a dyn Backend`. |
| **Trait Object Virality** | Prefer generics by default. Trait objects force allocations and kill inlining. | Hiding types via `Box` out of sheer convenience. |

## Rules of Thumb

- If you have an `N x M` matrix of types (e.g., 2 client types × 3 storage backends), you need a Bridge.
- The implementor provides the "atoms"; the abstraction builds the "molecules".
- Always configure a concrete implementor completely before injecting it into the abstraction.
