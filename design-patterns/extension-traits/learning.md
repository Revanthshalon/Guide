# Extension Traits — Learning Notes

## Mental Model

**You have a type from another crate (like `std::vec::Vec` or `f64`), and you want to add a method to it (e.g., `my_vec.my_custom_method()`).** 

Because of Rust's **Orphan Rule**, you cannot implement a trait you didn't write on a type you don't own. The **Extension Trait** pattern solves this. You define a brand new trait (which you own), define your methods in it, and then implement that trait for the foreign type. 

The mental model is: **Extend foreign types ergonomically by injecting a locally-owned trait.** It allows you to maintain fluent method chaining (`val.clamp().log().process()`) rather than wrapping calls in nested free functions (`process(log(clamp(val)))`).

## Structure & Participants

### The Foreign Type
- **Role:** The struct or primitive you want to add behavior to.
- **In Rust:** `String`, `Vec<T>`, `f64`, or even a trait like `Iterator`.

### The Extension Trait
- **Role:** A new trait defined specifically to hold the new methods. Conventionally named with an `Ext` suffix.
- **In Rust:** `trait VecExt { ... }`

### The Blanket Implementation (Optional)
- **Role:** Implementing the extension trait for all types that satisfy certain bounds.
- **In Rust:** `impl<T: Iterator> IteratorExt for T { ... }`

## Idiomatic Rust Implementation & When It Dissolves

This pattern is deeply idiomatic in Rust and is the direct equivalent of C# Extension Methods. It effectively replaces the classical **Decorator** pattern in cases where you want to add behavior without needing to hold new state.

The pattern is everywhere in the ecosystem:
- `Itertools` from the `itertools` crate extends `Iterator`.
- `StreamExt` from `futures` extends `Stream`.

## Worked Example

Let's build a logging extension for iterators, a common requirement when debugging complex data pipelines.

**Stage 0 — The Problem (Free Functions break chaining)**
```rust
let iter = vec![1, 2, 3].into_iter().filter(|x| *x > 1);
// I want to log the items here before mapping!
let mapped = iter.map(|x| x * 2);
```

**Stage 1 — The Extension Trait**

We define our trait and implement it for anything that implements `Iterator`.

```rust
// 1. Define the trait
pub trait IteratorExt: Iterator {
    // Return a custom iterator adapter
    fn log_items(self) -> LogIterator<Self> 
    where 
        Self: Sized 
    {
        LogIterator { iter: self }
    }
}

// 2. Blanket implementation for all Iterators
impl<T: Iterator> IteratorExt for T {}

// 3. The new iterator adapter struct
pub struct LogIterator<I> {
    iter: I,
}

impl<I: Iterator> Iterator for LogIterator<I> 
where 
    I::Item: std::fmt::Debug 
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next();
        if let Some(ref val) = item {
            println!("Yielding: {:?}", val);
        }
        item
    }
}
```

**Stage 2 — Usage (Fluent Chaining)**

Now we can seamlessly inject our logging step directly into the pipeline, as if `log_items` was part of `std::iter::Iterator`.

```rust
fn main() {
    let sum: i32 = vec![1, 2, 3]
        .into_iter()
        .filter(|x| *x > 1)
        .log_items() // Our extension method!
        .map(|x| x * 2)
        .sum();
}
```

## Versus

### Versus Newtype Pattern
- **What's the same:** Both allow adding methods to foreign types.
- **What's different:** Newtype wraps the type (`MyF64(f64)`) and implements methods on the wrapper, forcing you to wrap/unwrap the value. Extension Trait adds the method directly to the original type (`f64`).
- **How to decide:** If you want to restrict the API or define domain identity (e.g., `UserId(String)`), use Newtype. If you just want to add utility methods directly to the original type, use Extension Traits.

### Versus Free Functions
- **What's the same:** Both accomplish the same logic (`clamp(val)` vs `val.clamp()`).
- **What's different:** Free functions interrupt method chaining.
- **How to decide:** If it sits in the middle of a pipeline, use an Extension Trait. Otherwise, a free function is often simpler.

## Pitfalls in Depth

### Pitfall: Trait Not In Scope
- **What goes wrong:** A user of your library tries to call `.log_items()` on an iterator, but gets a compiler error saying the method doesn't exist, even though your crate implements it.
- **Why it happens (the mechanism):** Extension methods are only visible to the compiler if the *Trait itself* is brought into scope via a `use` statement. Rust does not globally resolve trait methods to avoid chaos.
- **How to handle it, and why that works:** Expose extension traits in a library `prelude` module (`pub mod prelude { pub use super::IteratorExt; }`). Users just add `use my_crate::prelude::*;`.
- **Trade-offs of the fix:** Preludes can pollute the user's namespace if they contain too many items.

### Pitfall: Name Collisions (Shadowing)
- **What goes wrong:** The standard library or another crate adds a method with the exact same name to the underlying type. Your code might suddenly stop compiling or silently change behavior.
- **Why it happens (the mechanism):** Rust prioritizes native inherent methods over trait methods. If `f64` eventually gets a native `clamp_to_zero` method, the compiler will choose the native one. If two traits provide the same method, it's an ambiguous resolution error.
- **How to handle it, and why that works:** Use very specific domain names for your methods. If a collision occurs, use fully qualified syntax to force the trait method: `F64Ext::clamp_to_zero(my_val)`.
- **Trade-offs of the fix:** Fully qualified syntax destroys method chaining, defeating the entire purpose of the extension trait.

### Pitfall: Compile-Time Cost of Blanket Impls
- **What goes wrong:** Adding a massive blanket implementation (`impl<T> MyExt for T`) causes your project's compile times to spike dramatically.
- **Why it happens (the mechanism):** When you implement a trait for literally every type `T`, the Rust compiler has to evaluate this trait bound and potentially monomorphize it for a massive number of types across the entire dependency tree.
- **How to handle it, and why that works:** Be precise with your bounds. Instead of `impl<T> MyExt for T`, implement it only for the specific types you actually need (`impl MyExt for String`), or constrain it heavily (`impl<T: SpecificTrait> MyExt for T`).
- **Trade-offs of the fix:** You lose the universal applicability of the extension, requiring manual `impl` blocks for new types.

## Design Decisions & Trade-offs

**Granularity:** Should you have one massive `Ext` trait or many small ones? Generally, group methods by the type they extend (e.g., `StringExt`, `VecExt`).

**Object Safety:** Extension traits that take `self` by value or return `Self` (like our `log_items` returning `LogIterator<Self>`) are **not object safe**. You cannot use them on a trait object like `dyn Iterator`. If you need an extension method to work on `dyn Trait`, you must ensure it has a `where Self: Sized` bound, which explicitly excludes trait objects from calling that specific method, keeping the rest of the trait object-safe.

## Exercises & Self-Test

1. Explain the Orphan Rule. Why does it exist, and how does the Extension Trait pattern work around it?
2. If `std::vec::Vec` adds a `.my_custom_sort()` method in Rust 1.80, and you already have a `VecExt` trait with `.my_custom_sort()`, how does the compiler resolve `vec.my_custom_sort()`?
3. Design Exercise: Write a `ResultExt` trait that adds a `.log_err()` method to `Result<T, E>`. It should print the error if it is an `Err`, but return the unchanged `Result` in both cases to allow chaining.
4. Why does adding `where Self: Sized` to an extension method allow the trait to remain object-safe for `dyn Trait` usage?

## Open Questions

- What are the exact compile-time implications of adding hundreds of extension traits to a project's prelude?
- Is there a way to prioritize an extension trait method over an inherent method without losing method chaining syntax? (Currently, no).

## References

- [Rust Book: Traits (The Orphan Rule)](https://doc.rust-lang.org/book/ch10-02-traits.html)
- `itertools` documentation (a masterclass in extension traits).
- Cross-ref: Adapter (`../adapter/learning.md`), Template Method (`../template-method/learning.md`)
