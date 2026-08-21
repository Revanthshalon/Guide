# Adapter — Quick Reference

## One-Liner

The Adapter pattern converts the interface of an existing struct into another trait that the domain expects, bridging incompatible boundaries without modifying the source code of either.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| The compiler blocks you with the Orphan Rule (`E0117`) | You want to add state, logging, or retries (use Decorator) |
| Translating legacy structs to modern generic traits | You need to simplify a massive, complex API (use Facade) |
| Integrating third-party crates into your pure domain | You own both types and can just implement the trait directly |

## Structure Sketch

```rust
// 1. Target (Domain expected)
pub trait Target { fn request(&self); }

// 2. Adaptee (Foreign, incompatible)
pub struct Adaptee;
impl Adaptee { pub fn specific_request(&self) {} }

// 3. Adapter (The bridge)
pub struct Adapter(pub Adaptee);
impl Target for Adapter {
    fn request(&self) {
        self.0.specific_request();
    }
}
```

## Rust Idiom

- **Newtype Pattern:** `struct Wrapper(ForeignType);` is the absolute standard way to bypass orphan rules.
- **`From`/`TryFrom`:** The canonical way to adapt data structures (`impl TryFrom<ForeignData> for LocalData`).
- **`AsRef<T>`:** Adapting references for ergonomic access to the inner wrapped type.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Decorator** | Keeps interface identical, modifies behavior vs. modifies interface, keeps behavior identical. |
| **Facade** | Translates an API to meet a specific *Target trait* vs. simplifies an API to make it *easier to use*. |
| **Bridge** | A proactive design to separate abstraction from implementation vs. a reactive fix for incompatible boundaries. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Fat Adapters** | Keep them thin. Extract business logic or DB calls out of the translation layer. | Adapters hitting the network. |
| **Infallible Panics** | Never use `From` for fallible parsing. Use `TryFrom` and bubble up `Result`. | Using `.expect()` in `From::from`. |
| **Trait Object Overhead** | Use generic adapters (`impl Target`) to allow static dispatch and inlining. | `Vec<Box<dyn Target>>` in hot loops. |

## Rules of Thumb

- **Infrastructure Layer:** Adapters belong in the infrastructure/integration layer. The domain defines the trait; the adapter implements it.
- **Static Dispatch:** Adapters are zero-cost abstractions only if they are statically dispatched.
- **Prefer Owning:** An adapter that takes ownership of its Adaptee (`Adapter(Adaptee)`) is vastly easier to manage than one tracking lifetimes (`Adapter<'a>(&'a mut Adaptee)`).

## Key References

- [Newtype & Zero-Cost Abstractions](../newtype-and-zero-cost/learning.md)
- [Extension Traits](../extension-traits/learning.md)
