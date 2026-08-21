# Iterator — Quick Reference

## One-Liner

Provide a lazy, safe, and sequential way to access elements of a collection without exposing its underlying representation, guaranteed at compile-time to prevent concurrent modification.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| You want to traverse a collection sequentially. | You need to traverse a highly heterogeneous tree where types change at each node (use Visitor). |
| You want to process data streams lazily (pipelining). | You are just mutating state (use a standard `for` loop instead of `.map()`). |

## Structure Sketch

```rust
// Core mechanism built into Rust
trait Iterator {
    type Item;
    // Merges hasNext and next to prevent race conditions
    fn next(&mut self) -> Option<Self::Item>;
}

trait IntoIterator {
    type Item;
    type IntoIter: Iterator<Item = Self::Item>;
    fn into_iter(self) -> Self::IntoIter;
}
```

## Rust Idiom

- **DO NOT** write manual `while` loops managing array indices.
- **DO** use `for item in &collection` (which automatically calls `IntoIterator`).
- **DO** use heavily chained iterator adapters: `iter().filter().map().fold()`.
- **DO** return `impl Iterator<Item = T>` from functions to hide complex nested adapter types while preserving zero-cost inlining.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Visitor** | Iterator lets the *caller* pull data and control the loop. Visitor gives the *collection* control to push data to the caller. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Laziness** | `map()` does nothing unless consumed. Use `for_each()` or a `for` loop for side-effects. | Compiler warnings about unused results. |
| **The `dyn Iterator` Perf Cliff** | Return `impl Iterator` instead of `Box<dyn Iterator>`. | `Box` destroys inlining and prevents vectorization in hot loops. |
| **The Eager Allocation Trap** | Write an explicit state machine for complex trees, rather than collecting to a `Vec`. | Eager allocation defeats the laziness guarantee of Iterators. |
| **Lending Iterator Issues** | If yielding references to internal buffers, you need GATs (Streaming Iterator). | The standard Iterator trait cannot yield references to mutating internal state. |

## Rules of Thumb

- **`into_iter()`**: Consumes the collection, yields owned `T`.
- **`iter()`**: Borrows the collection, yields `&T`.
- **`iter_mut()`**: Mutably borrows, yields `&mut T`.
- If an iterator adapter closure doesn't return anything (side-effect only), you should probably be using a `for` loop instead.

## Key References

- [Rust Documentation - std::iter](https://doc.rust-lang.org/std/iter/index.html)
