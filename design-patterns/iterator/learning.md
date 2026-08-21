# Iterator — Learning Notes

## Mental Model

**Collections hold data in wildly different ways (arrays, trees, linked lists, hash maps), but the act of *visiting every element* shouldn't require knowing those internal details.** The Iterator pattern extracts the traversal logic out of the collection and into a dedicated state machine.

In classic OOP (like Java), iterators have two methods: `hasNext()` and `next()`. This design fundamentally conflicts with Rust's ownership and safety goals. If another thread mutates the collection between `hasNext()` and `next()`, the iterator crashes or yields garbage. 

Rust's mental model is different: **An Iterator is a state machine that yields an `Option<T>` via a single `next()` method, and the borrow checker completely locks down the collection during iteration.** By merging the check and the retrieval, Rust eliminates race conditions. By locking the collection, Rust mathematically proves concurrent modification is impossible.

## Structure & Participants

### Iterator (Trait)
- **Role:** Defines the standard interface for traversing.
- **In classic OOP:** An interface with `hasNext()` and `next()`.
- **In Rust:** The `std::iter::Iterator` trait (`fn next(&mut self) -> Option<Self::Item>`).

### Concrete Iterator
- **Role:** Keeps track of the current position in the specific collection.
- **In Rust:** Structs like `std::vec::IntoIter` or a custom state machine.

### Iterable (Trait)
- **Role:** The collection that can produce an Iterator.
- **In Rust:** The `std::iter::IntoIterator` trait, which powers `for` loops.

## Idiomatic Rust Implementation

This pattern is deeply, fundamentally baked into Rust. You rarely write an Iterator from scratch for standard arrays; you use them via adapters.

Rust has three distinct forms of iteration driven by ownership:
1. `into_iter()`: Takes ownership (`T`). The collection is consumed and destroyed.
2. `iter()`: Borrows immutably (`&T`).
3. `iter_mut()`: Borrows mutably (`&mut T`).

```rust
let nums = vec![1, 2, 3, 4, 5];
// Iterators are lazy. This creates a state machine but executes nothing.
let evens_squared = nums.iter()
    .filter(|x| *x % 2 == 0)
    .map(|x| x * x);

// Consumes the iterator, driving the state machine
let result: Vec<i32> = evens_squared.collect(); 
```

## When This Pattern Dissolves in Rust

The Iterator pattern doesn't dissolve in Rust—**it conquered the language.** Rust's `for` loops are purely syntactic sugar over `IntoIterator`. Because iterators in Rust are strictly typed and heavily optimized (compiling down to the exact same vector instructions as a manual `while` loop), you use them everywhere. 

## Worked Example

Let's say we have a complex tree structure and we want to iterate over its leaves.

### Stage 0: The Eager Allocation

A common mistake for beginners is to collect everything into a `Vec` and just return the vector's iterator. 

```rust
enum Node<T> {
    Leaf(T),
    Branch(Box<Node<T>>, Box<Node<T>>),
}

impl<T> Node<T> {
    // Bad: Allocates memory and does all the work upfront!
    pub fn eager_leaves(&self) -> impl Iterator<Item = &T> {
        let mut leaves = Vec::new();
        self.collect_leaves(&mut leaves);
        leaves.into_iter()
    }

    fn collect_leaves<'a>(&'a self, acc: &mut Vec<&'a T>) {
        match self {
            Node::Leaf(val) => acc.push(val),
            Node::Branch(left, right) => {
                left.collect_leaves(acc);
                right.collect_leaves(acc);
            }
        }
    }
}
```
This defeats the entire purpose of iterators: **laziness**. If the caller only wants the first leaf and then stops, we still traversed the entire tree and allocated a massive vector.

### Stage 1: The True Lazy State Machine

To be truly lazy, we must extract the traversal state (the call stack) into our own Iterator struct.

```rust
// The Concrete Iterator holds its traversal state manually
pub struct TreeIter<'a, T> {
    stack: Vec<&'a Node<T>>, // A small stack to track where we are
}

impl<'a, T> Iterator for TreeIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        // Pop the next node off our manual stack
        while let Some(node) = self.stack.pop() {
            match node {
                Node::Leaf(val) => return Some(val), // Yield immediately!
                Node::Branch(left, right) => {
                    // Push right first so left is popped next
                    self.stack.push(right);
                    self.stack.push(left);
                }
            }
        }
        None
    }
}

impl<T> Node<T> {
    pub fn iter(&self) -> TreeIter<'_, T> {
        TreeIter { stack: vec![self] } // Zero-cost setup
    }
}
```
Now, calling `tree.iter().next()` does exactly the minimal work to find the first leaf, requires no upfront heap allocations of all elements, and is extremely cache-efficient.

## Versus

- **Iterator vs Visitor:**
  - *Iterator* is pulled by the client. The client controls the `for` loop and can stop halfway.
  - *Visitor* is pushed by the collection. The collection controls the traversal. Visitor is better for traversing heterogeneous structures (like ASTs) where types change; Iterator is better for homogeneous sequences where laziness matters.

## Pitfalls in Depth

### Pitfall: Forgetting iterators are lazy

- **What goes wrong:** You write `my_vec.iter().map(|x| do_side_effect(x));` and wonder why `do_side_effect` never executes.
- **Why it happens (the mechanism):** Iterator adapters (`map`, `filter`) only *describe* a computation by building up a nested struct of closures. They do absolutely nothing until a consumer (like `collect`, `fold`, or a `for` loop) repeatedly calls `next()`.
- **How to handle it, and why that works:** Use a `for` loop for side effects, or use `.for_each()`. Do not use `.map()` for side-effects.
- **Trade-offs of the fix:** None. It makes the code's intent clearer.

### Pitfall: The `Box<dyn Iterator>` Performance Cliff

- **What goes wrong:** To simplify a complex return type, you write `fn get_data() -> Box<dyn Iterator<Item = u32>>`. In a tight numeric loop, this can be several times slower than the equivalent statically-dispatched code — the exact factor depends on how much SIMD/inlining the boxed version loses, so benchmark on your actual workload rather than assuming a fixed multiplier.
- **Why it happens (the mechanism):** Dynamic dispatch (`dyn Iterator`) completely blinds the compiler's inliner. Instead of fusing a `map` and `filter` into a tight assembly loop using SIMD instructions, the CPU must do a pointer dereference and virtual table lookup for *every single element*. It also forces a heap allocation for the iterator itself.
- **How to handle it, and why that works:** Return `impl Iterator<Item = u32>`. This "Opaque Return Type" hides the concrete type from the user, but the compiler still knows exactly what it is, allowing 100% inlining and zero-cost abstraction.
- **Trade-offs of the fix:** `impl Iterator` cannot return different underlying iterator types from different branches (e.g., an `if` returning a `std::slice::Iter` and an `else` returning a `std::vec::IntoIter`). If you must do that, use the `either` crate instead of Boxing.

### Pitfall: The Lending Iterator Trap

- **What goes wrong:** You try to write an iterator that yields a sliding window or a reused buffer (e.g., reading a file line-by-line into a single `String` buffer). The compiler gives you impossible lifetime errors.
- **Why it happens (the mechanism):** The standard `Iterator` trait is defined as `fn next(&mut self) -> Option<Self::Item>`. The returned `Item` is entirely independent of the `&mut self` lifetime. You cannot yield a reference to something inside the iterator itself, because the caller could collect all those references into a `Vec`, but they all point to the same mutating buffer!
- **How to handle it, and why that works:** You cannot implement standard `Iterator` for this. You must either yield owned values (allocating new strings), or use a "Lending Iterator" (Streaming Iterator) crate which uses Generic Associated Types (GATs) to tie the yield lifetime to the `next` call.
- **Trade-offs of the fix:** Lending iterators cannot be used in standard `for` loops without a `while let` construct, because `for` loops require standard `IntoIterator`.

## Design Decisions & Trade-offs

- **Static vs Dynamic:** Always use `impl Iterator`. Never use `Box<dyn Iterator>` on a hot path.
- **Eager vs Lazy:** Extracting an iterator into an eager `Vec` is acceptable only for tiny data structures where the cost of writing a manual state machine outweighs the allocation overhead.

## Exercises & Self-Test

1. Why did Rust combine Java's `hasNext()` and `next()` into a single `Option`-returning method? 
2. Explain why returning `Box<dyn Iterator>` destroys performance in a tight loop.
3. Write a custom lazy `Iterator` that yields the Fibonacci sequence infinitely.
4. Try to write an Iterator that yields `&mut [u8]` from a single internal `[u8; 1024]` buffer. Observe the lifetime errors. Why is this structurally impossible with standard `Iterator`?

## Open Questions

- How do GATs (Generic Associated Types) fully solve the Lending/Streaming Iterator problem in modern Rust?

## References

- [Rust Documentation - std::iter](https://doc.rust-lang.org/std/iter/index.html) - One of the best standard library modules ever written.
