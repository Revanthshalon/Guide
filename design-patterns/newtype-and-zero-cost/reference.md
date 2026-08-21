# Newtype & Zero-Cost Abstractions — Quick Reference

## One-Liner

Wrap primitive or external types in a single-field tuple struct to enforce domain type safety and bypass the Orphan Rule, with exactly zero runtime performance cost.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| Differentiating primitives (e.g., `UserId` vs `OrderId`). | You just want to shorten a long type signature (use `type` alias). |
| You need to implement a foreign trait on a foreign type. | You need to expose 100% of the inner type's methods without restriction (causes boilerplate). |
| Interfacing with C (FFI) and you need strongly-typed pointers. | |

## Structure Sketch

```rust
// The Newtype enforcing invariants
#[derive(Debug, PartialEq, Eq)]
pub struct Password(String);

impl Password {
    // Explicitly expose only safe methods
    pub fn len(&self) -> usize { self.0.len() }
}

// Bypassing Orphan Rule
pub struct Wrapper(pub Vec<u8>);
impl std::fmt::Display for Wrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

// FFI Safety
#[repr(transparent)]
pub struct SafeId(pub u32);
```

## Rust Idiom

Always use single-field tuple structs for this. Define traits explicitly. Avoid `Deref` for domain-modeling newtypes; use it only if the newtype is strictly acting as a smart pointer. Use `#[repr(transparent)]` for FFI safety.

## Versus

| Confused with | Key difference |
| --- | --- |
| Type Aliases (`type A = B;`) | Aliases do not create a new type, just a synonym. The compiler won't stop you from mixing them up. Newtypes are distinct. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Method Hiding Boilerplate** | Implement `as_inner(&self) -> &T`, `AsRef`, or use `derive_more`. | Avoid `Deref` unless it's semantically a smart pointer. |
| **The Deref Anti-Pattern** | Never implement `DerefMut` for types with invariants (like `Email`). | Callers can mutate the inner value directly, bypassing validation. |
| **FFI ABI mismatch** | Apply `#[repr(transparent)]` to the struct. | Without this, the compiler might pass it differently across FFI than the bare primitive. |

## Rules of Thumb

- If a function takes three `f64` parameters (e.g., `lat`, `lon`, `alt`), they should be Newtypes to prevent argument swapping.
- `std::convert::From` and `Into` are your best friends when using Newtypes.
- Use `TryFrom` instead of `From` if the Newtype enforces constraints (e.g., a string must contain an '@' to be an `EmailAddress`).

## Key References

- [derive_more crate](https://crates.io/crates/derive_more) for reducing boilerplate.
