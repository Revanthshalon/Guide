# Marker Traits & Phantom Types — Quick Reference

## One-Liner

Use empty traits and `PhantomData<T>` to tag types, categorize data, and enforce logical boundaries entirely at compile time, with zero runtime memory or performance cost.

## When to Use

| Use it when | Avoid it when |
| --- | --- |
| Creating type-safe domain identifiers (e.g., `Id<User>`, `Id<Product>`) to prevent mixing `u64` values. | The tag determines behavior that must be checked via `match` at runtime (use `enum`s instead). |
| You want to prevent a public trait from being implemented outside your crate (Sealed Trait pattern). | You just need one or two distinct types with different methods (use the Newtype pattern). |
| You are writing `unsafe` data structures and need to inform the borrow checker about ownership and variance. | |

## Structure Sketch

```rust
use std::marker::PhantomData;

// Marker Types (Zero Sized Types)
#[derive(Copy, Clone)]
pub struct Usd;
#[derive(Copy, Clone)]
pub struct Eur;

// Struct with Phantom Type
pub struct Money<Currency> {
    amount: f64,
    _marker: PhantomData<Currency>,
}

impl<C> Money<C> {
    pub fn new(amount: f64) -> Self {
        Money { amount, _marker: PhantomData }
    }
}
// Signature ensures Money<Usd> cannot be added to Money<Eur>
```

## Rust Idiom

Zero-sized types (ZSTs) are heavily optimized by Rust; they disappear entirely during compilation. Use `PhantomData<T>` whenever a struct has a generic type parameter `T` that is not used in any of its fields. Hide the `PhantomData` field in a private module and expose it via a constructor.

## Versus

| Confused with | Key difference |
| --- | --- |
| Newtype Pattern | Newtype is `struct Usd(f64)`. Phantom is `Money<Usd>`. Phantoms scale better when you have many tags sharing the exact same inner logic and implementation blocks. |
| Type State Pattern| Type State actively transitions types (`State<A> -> State<B>`). Marker/Phantom types often just passively tag data, though they form the building blocks of Type State. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Over-constrained Derived Traits** | Manually implement `Clone`/`PartialEq` for the wrapper, OR derive them on the marker types. | `#[derive(Clone)]` on `Id<T>` requires `T: Clone`, even though `T` is only phantom. |
| **Drop Check / Unsafe bugs** | Match the `PhantomData` strictly to the ownership semantics (`T` vs `&'a T`). | If writing custom smart pointers, read the Rustonomicon on subtyping and variance. |
| **Noise in API Signatures** | Keep `_marker` private and provide a `.new()` constructor. | Forcing users to type `_marker: PhantomData` bleeds implementation details. |

## Rules of Thumb

- A trait with no methods is a **Marker Trait**. It is used purely for generic bounds (`T: PiiSafe`).
- If you want a trait to be public to *use* but private to *implement*, use the **Sealed Trait** pattern (require a private marker trait as a supertrait).
- `PhantomData` physically costs nothing. Use it liberally to create compiler-enforced constraints.

## Key References

- [Rustonomicon: PhantomData](https://doc.rust-lang.org/nomicon/phantom-data.html)
- [Rustonomicon: Subtyping and Variance](https://doc.rust-lang.org/nomicon/subtyping.html)
- [API Guidelines: Sealed Traits](https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed)
