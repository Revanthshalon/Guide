# Visitor — Quick Reference

## One-Liner

Extract operations from a heterogeneous structure of objects, allowing you to add new operations without modifying the objects themselves.

## When to Use

| Use it when | Avoid it when |
| --- | --- |
| The type hierarchy is stable, but operations change frequently (ASTs, formatters). | New element types are added frequently (The Expression Problem). |
| You need to execute distinct logic depending on the exact concrete type of an object in a tree. | You need to modify/mutate the tree structure heavily (use a Folder/Rewriter). |

## Structure Sketch (The Enum/Match idiom)

```rust
use std::fmt::{self, Write};

pub enum Expr {
    Literal(i32),
    Add(Box<Expr>, Box<Expr>),
}

// The "Visitor" is a function with a Match
pub fn format_expr(expr: &Expr, out: &mut String) -> fmt::Result {
    match expr {
        Expr::Literal(val) => write!(out, "{}", val),
        Expr::Add(l, r) => {
            write!(out, "(")?;
            format_expr(l, out)?;
            write!(out, " + ")?;
            format_expr(r, out)?;
            write!(out, ")")
        }
    }
}
```

## Rust Idiom

**Enums and `match`.** Rust's closed enums provide exhaustive pattern matching, natively solving the exact problem that double-dispatch Visitor solves in classical OOP. Only use the trait-based Double Dispatch Visitor for open/extensible hierarchies (like `serde`).

## Versus

| Confused with | Key difference |
| --- | --- |
| **Iterator** | Iterator yields uniform items linearly; Visitor traverses a topology and reacts to specific concrete types. |
| **Enum Dispatch** | Enum Dispatch puts behavior *inside* the data; Visitor puts behavior *outside* the data. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **The Expression Problem** | Don't use Visitor if you anticipate adding new element types frequently. | Needing to update 20 Visitor implementations because you added a `Video` node. |
| **Mutation battles** | Instead of a mutating visitor, use a Folder/Rewriter that consumes the tree and builds a new one. | Borrow checker errors when a mutable visitor tries to borrow the tree. |
| **Virtual Dispatch Cost** | If using traits, prefer `<V: Visitor>` generics over `&mut dyn Visitor`. | Massive performance overhead from double-indirection at every node in large ASTs. |

## Rules of Thumb

- If you control all the types, use an `enum` and `match`. This renders classical Visitor obsolete.
- If a user of your crate defines the types, and you provide the operations, use a Trait-based Visitor.
- Pass `&mut String` or `&mut fmt::Formatter` for text accumulation; never allocate transient `String`s via `format!()` at every step.

## Key References

- [Rust API Guidelines - Enums](https://rust-lang.github.io/api-guidelines/flexibility.html)
- [Serde Visitor Documentation](https://serde.rs/visitor.html)
