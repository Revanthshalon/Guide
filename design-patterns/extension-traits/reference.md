# Extension Traits — Quick Reference

## One-Liner

Add new methods to foreign types (types you don't own) by defining a locally-owned trait and implementing it for them, bypassing the Orphan Rule and enabling fluent method chaining.

## When to Use

| Use it when | Avoid it when |
| --- | --- |
| You want to add convenience methods to standard library types (e.g., `String`, `Vec`). | You just need a simple utility function that isn't part of a method chain (use a free function). |
| You want to extend a trait like `Iterator` or `Stream` with custom combinators. | You want to hide the original API or enforce strict domain constraints (use Newtype instead). |

## Structure Sketch

```rust
// 1. Define the trait (must be locally owned)
pub trait StringExt {
    fn is_capitalized(&self) -> bool;
}

// 2. Implement it for the foreign type
impl StringExt for String {
    fn is_capitalized(&self) -> bool {
        self.chars().next().map_or(false, |c| c.is_uppercase())
    }
}

// 3. User must bring trait into scope to use the method
// use crate::StringExt;
// let b = "Hello".to_string().is_capitalized();
```

## Rust Idiom

Conventionally name the trait `TargetExt` (e.g., `StringExt`, `IteratorExt`). If building a library, export these traits in a `prelude` module so users can `use my_lib::prelude::*;` to get all the extension methods at once.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Newtype Pattern** | Newtype wraps the type (`MyString(String)`), forcing wrap/unwrap logic. Extension Traits attach methods directly to the original type (`String`). |
| **Free Functions** | Free functions break method chaining (`b(a(val))`). Extension Traits allow fluent chains (`val.a().b()`). |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Methods Not Found** | Remind users to `use` the trait. Expose a `prelude` module. | The compiler will complain the method doesn't exist if the trait isn't in scope. |
| **Name Collisions (Shadowing)** | Use domain-specific method names. Use fully qualified syntax as a last resort. | The standard library adding the same method to the type, silently stealing method resolution. |
| **Compile-Time Spikes** | Restrict blanket implementations (`impl<T>`) carefully using specific bounds. | Massive compile times from the compiler evaluating bounds across the entire dependency tree. |
| **Object Safety Loss** | Add `where Self: Sized` to extension methods that return `Self` or take `self` by value. | Accidentally breaking `dyn Trait` usage for the entire base trait. |

## Rules of Thumb

- Blanket implementations (`impl<T: Iterator> IterExt for T`) are incredibly powerful for extending whole families of types.
- Always add an `Ext` suffix to the trait name to clearly signal its purpose.
- Remember that `where Self: Sized` is required if your extension method returns an adapter struct, otherwise the trait becomes non-object-safe.

## Key References

- `itertools::Itertools` trait in the ecosystem is the most famous example of extending `Iterator`.
