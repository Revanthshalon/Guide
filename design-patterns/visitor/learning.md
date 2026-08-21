# Visitor — Learning Notes

## Mental Model

**When you have a heterogenous tree or graph of objects, you often want to run an operation across the entire structure where the logic depends strictly on the exact type of each node.** A Visitor decouples the operation from the data types, allowing you to add new operations without changing the data structures.

In single-dispatch languages (like Java or C++), calling `node.process(operation)` resolves based on the runtime type of `node`, but not `operation`. To solve this, the Visitor pattern uses **Double Dispatch**: the caller asks the node to accept a visitor (`node.accept(visitor)`), and the node, now knowing its exact type, calls back to the visitor (`visitor.visit_specific_node(self)`). 

In Rust, this structural dance is largely unnecessary for closed type universes because `match` on an `enum` provides exhaustive, type-safe destructuring out of the box. But when the set of types is open (users can define their own nodes), the trait-based Visitor pattern remains the only way to retain type-specific dispatch without downcasting.

## Structure & Participants

### The Visitor
- **Role:** Declares a visiting method for every concrete element type.
- **In Rust (Classical):** A trait with `visit_paragraph(&mut self, p: &Paragraph)`.
- **In Rust (Idiomatic):** A function with a `match` statement over an enum.

### The Element (Visitable)
- **Role:** Declares an `accept` method.
- **In Rust (Classical):** A trait requiring `fn accept(&self, v: &mut dyn Visitor)`.
- **In Rust (Idiomatic):** The enum variants themselves.

## Idiomatic Rust Implementation

**In 95% of cases, the Visitor pattern completely dissolves in Rust into `enum` and `match`.** 

When you define an `enum`, you close the set of types. A simple `match` exhaustively checks every variant, exactly mirroring the compiler guarantee of an OOP Visitor interface. The `match` statement replaces the `accept` method, and the arms of the `match` replace the `visit` methods.

### When This Pattern Dissolves: The Enum Way

```rust
use std::fmt::{self, Write};

pub enum Expr {
    Literal(i32),
    Add(Box<Expr>, Box<Expr>),
    Multiply(Box<Expr>, Box<Expr>),
}

// The "Visitor" is just a function!
pub fn evaluate(expr: &Expr) -> i32 {
    match expr {
        Expr::Literal(val) => *val,
        Expr::Add(left, right) => evaluate(left) + evaluate(right),
        Expr::Multiply(left, right) => evaluate(left) * evaluate(right),
    }
}
```

This renders the classical pattern obsolete for closed types. You don't "implement" a Visitor; you just write a function.

## Worked Example

Consider building an AST (Abstract Syntax Tree) toolchain. We have a set of core types and want to implement multiple passes: a linter, an optimizer, and a formatter.

**Stage 1 — The Classic Visitor (Extensible Types)**

If users can plug in their own AST nodes, an enum won't work. We need the classical pattern.

```rust
use std::fmt::{self, Write};

pub trait Visitor {
    fn visit_literal(&mut self, val: i32);
    fn visit_add(&mut self, left: &dyn Element, right: &dyn Element);
}

pub trait Element {
    fn accept(&self, visitor: &mut dyn Visitor);
}

pub struct Literal(pub i32);
impl Element for Literal {
    fn accept(&self, visitor: &mut dyn Visitor) {
        visitor.visit_literal(self.0);
    }
}

pub struct Add {
    pub left: Box<dyn Element>,
    pub right: Box<dyn Element>,
}
impl Element for Add {
    fn accept(&self, visitor: &mut dyn Visitor) {
        visitor.visit_add(&*self.left, &*self.right);
    }
}

// Adding an operation: Code Formatter
pub struct Formatter<'a> {
    out: &'a mut String,
}

impl<'a> Visitor for Formatter<'a> {
    fn visit_literal(&mut self, val: i32) {
        let _ = write!(self.out, "{}", val);
    }
    
    fn visit_add(&mut self, left: &dyn Element, right: &dyn Element) {
        let _ = write!(self.out, "(");
        left.accept(self);
        let _ = write!(self.out, " + ");
        right.accept(self);
        let _ = write!(self.out, ")");
    }
}
```

**Stage 2 — The Idiomatic Shift (Closed Types)**

If the AST is strictly defined by us, we drop the traits entirely.

```rust
// Using the Expr enum from earlier
pub fn format_ast(expr: &Expr, output: &mut String) -> fmt::Result {
    match expr {
        Expr::Literal(val) => write!(output, "{}", val),
        Expr::Add(left, right) => {
            write!(output, "(")?;
            format_ast(left, output)?;
            write!(output, " + ")?;
            format_ast(right, output)?;
            write!(output, ")")
        }
        Expr::Multiply(left, right) => {
            write!(output, "(")?;
            format_ast(left, output)?;
            write!(output, " * ")?;
            format_ast(right, output)?;
            write!(output, ")")
        }
    }
}
```
Notice how `format_ast` writes directly to `&mut String` (or `&mut fmt::Formatter`) instead of allocating transient `String`s via `format!()` at every node, avoiding massive allocation overhead during traversal.

## Versus

### Versus Iterator
- **Iterator** pulls data sequentially, flattening structure and erasing topology.
- **Visitor** walks the exact topology of the data (like a tree), allowing the operation to react differently based on depth and node type.

### Versus Enum Dispatch
- **Enum Dispatch** places the behavior *inside* the data (`expr.evaluate()`). Use this if operations are fundamental and rarely change.
- **Visitor (Match)** places behavior *outside* the data. Use this if the data is dumb and operations are added frequently (linting, formatting).

## Pitfalls in Depth

### Pitfall: The Expression Problem
- **What goes wrong:** You design a system using Visitor, then realize you frequently add new *types of elements* (not just new operations). Every time you add a type, you must update the `Visitor` trait and every single struct that implements it.
- **Why it happens (the mechanism):** The Visitor makes it easy to add new *operations* (just add a struct), but hard to add new *types* (modifies the core trait). This orthogonal scalability is the fundamental trade-off of the Expression Problem.
- **How to handle it, and why that works:** If your domain adds types frequently (UI widgets), put behavior on the types themselves (Traits/Enum Dispatch). If your domain adds operations frequently but types are stable (AST passes), Visitor or Match is perfect.
- **Trade-offs of the fix:** Choosing the wrong side requires sweeping refactors of core traits.

### Pitfall: Virtual Dispatch Overhead
- **What goes wrong:** Traversing a massive AST using `&mut dyn Visitor` and `&dyn Element` becomes a performance bottleneck.
- **Why it happens (the mechanism):** `dyn Trait` forces dynamic dispatch. A double-dispatch call (`accept` then `visit`) means two indirect function calls per node, breaking branch prediction and preventing compiler inlining.
- **How to handle it, and why that works:** If performance is critical, use Enums where dispatch is a simple jump table (Match). If you must use traits, consider making the Visitor a generic parameter instead of a trait object: `fn accept<V: Visitor>(&self, visitor: &mut V)`, allowing monomorphization and inlining.
- **Trade-offs of the fix:** Generics can cause code bloat and increase compile times, especially for deeply nested structures.

### Pitfall: Mutation and Ownership Battles
- **What goes wrong:** You try to write a mutating Visitor that modifies the AST in place. The borrow checker rejects it because traversing a mutable tree while passing a mutable reference to the visitor causes aliasing conflicts.
- **Why it happens (the mechanism):** Classical Visitor assumes aliasable, mutable object graphs (like in Java). Rust enforces XOR mutability.
- **How to handle it, and why that works:** Write a "Folder" or "Rewriter". Instead of taking `&mut self` and mutating in place, the Folder consumes the tree (`self`) and returns a new reconstructed tree (`Self`). This works harmoniously with Rust's ownership model.
- **Trade-offs of the fix:** Allocates memory for a new tree instead of mutating in place, though memory can often be reused if `String`s and `Vec`s are moved.

## Design Decisions & Trade-offs

**Enums vs Traits:** Always default to Enums + Match in Rust. Only reach for the trait-based Visitor if you are writing a framework where the user provides the types (e.g., `serde`), or if you need to decouple crates completely.

**Avoid `format!()` in Tree Traversals:** When writing formatters or serializers, pass a `&mut String` or `&mut fmt::Formatter` down the recursive calls. Constructing and dropping strings at every node yields polynomial allocation overhead.

## Exercises & Self-Test

1. Explain Double Dispatch. Why does Java need it, and why does Rust's `match` bypass it completely?
2. What is the Expression Problem? If you are building a plugin system where users add new UI components every day, should you use a Visitor?
3. Rewrite the classic `Formatter` example above using generics (`<V: Visitor>`) instead of trait objects (`dyn Visitor`). Measure or predict the compilation differences.
4. Read the source code for `serde::de::Visitor`. Identify why `serde` cannot possibly use an enum for this.

## Open Questions

- How do you implement a Visitor that can return errors (e.g., `Result`) without polluting the generic trait bounds or hardcoding an error type?
- When does an enum jump table (`match`) become less efficient than virtual dispatch for ASTs with hundreds of variants?

## References

- [Serde documentation on Deserializer/Visitor](https://serde.rs/impl-deserialize.html) — the most famous, sophisticated use of Visitor in Rust.
- Related: [Enum Dispatch](../enum-dispatch/learning.md)
