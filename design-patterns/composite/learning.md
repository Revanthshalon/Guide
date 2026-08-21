# Composite — Learning Notes

## Mental Model

The Composite pattern solves the problem of **heterogeneous recursion**. When you have single items (leaves) and collections of those items (branches) that you need to treat interchangeably, branching logic (`if item.is_collection() { ... } else { ... }`) quickly becomes unmaintainable. 

Composite unifies them. You compose objects into tree structures and define a single interface that both leaves and branches implement. When you call an operation on a leaf, it does the work. When you call it on a branch, it aggregates the results from its children. To the caller, there is no difference between querying a single file and querying a folder containing 10,000 files.

## Structure & Participants

- **Component:** The uniform interface (trait or enum) that defines the operations valid for both leaves and branches.
- **Leaf:** The terminal elements of the tree. They perform the actual core work and cannot hold children.
- **Composite (Branch):** Elements that hold a collection of children. Their implementation of the Component interface delegates the work to their children and aggregates the results.

## Idiomatic Rust Implementation

Because tree nodes are inherently recursive data structures, Rust requires you to insert indirection (like `Box` or `Vec`) so the compiler can determine their size.

### 1. The Enum Approach (Closed Trees)
If the set of possible node types is known at compile time and won't be extended by users of your crate, an `enum` is the most performant and idiomatic way to build a composite.

```rust
pub enum FsNode {
    File { name: String, size_bytes: u64 },
    // Indirection provided by Vec
    Directory { name: String, children: Vec<FsNode> }, 
}

impl FsNode {
    pub fn size(&self) -> u64 {
        match self {
            FsNode::File { size_bytes, .. } => *size_bytes,
            FsNode::Directory { children, .. } => {
                // Recursively delegate to children
                children.iter().map(|c| c.size()).sum()
            }
        }
    }
}
```
*Why this is good:* Zero dynamic dispatch. The compiler can inline aggressively. Memory locality is excellent.

### 2. The Trait Object Approach (Open Trees)
If you need an extensible tree—e.g., you are building a UI framework and want users to define their own custom widgets—you must use a trait and `Box<dyn Trait>`.

```rust
// Component
pub trait FileSystemNode {
    fn size(&self) -> u64;
    fn search(&self, keyword: &str) -> Vec<String>;
}

// Leaf
pub struct File {
    pub name: String,
    pub size_bytes: u64,
}

impl FileSystemNode for File {
    fn size(&self) -> u64 { self.size_bytes }
    fn search(&self, keyword: &str) -> Vec<String> {
        if self.name.contains(keyword) { vec![self.name.clone()] } else { vec![] }
    }
}

// Composite
pub struct Directory {
    pub name: String,
    pub children: Vec<Box<dyn FileSystemNode>>,
}

impl FileSystemNode for Directory {
    fn size(&self) -> u64 {
        self.children.iter().map(|child| child.size()).sum()
    }
    fn search(&self, keyword: &str) -> Vec<String> {
        let mut results = Vec::new();
        if self.name.contains(keyword) { results.push(self.name.clone()); }
        // Aggregate results from children
        for child in &self.children {
            results.extend(child.search(keyword));
        }
        results
    }
}
```

## When This Pattern Dissolves in Rust

The classical OOP pattern relies on abstract base classes. In Rust, if your tree is closed, the pattern dissolves entirely into an **algebraic data type (`enum`)**. Enums represent the exact same concept—"a thing that is either a Leaf or a Branch"—but with exhaustive compile-time checking and no heap fragmentation (beyond the `Vec`).

## Worked Example

**Stage 0: The Setup**
You compose a tree dynamically at runtime.
```rust
let root = Directory {
    name: "root".into(),
    children: vec![
        Box::new(File { name: "test.txt".into(), size_bytes: 100 }),
        Box::new(Directory {
            name: "subdir".into(),
            children: vec![
                Box::new(File { name: "data.bin".into(), size_bytes: 500 }),
            ],
        }),
    ],
};
```

**Stage 1: The Execution**
The caller executes a single method on the root. The caller does not know or care how deep the structure is.
```rust
let total_size = root.size(); 
// Returns 600. The recursion handles the rest.
```

## Versus

- **Composite vs. Decorator:** Both use recursive composition and delegate to inner objects. However, a Decorator typically has exactly *one* child and exists to add new behavior. A Composite has *many* children and exists to aggregate results structurally.
- **Composite vs. Iterator:** An Iterator traverses a structure sequentially, producing one item at a time. A Composite defines the hierarchical structure itself and executes domain operations directly across it.

## Pitfalls in Depth

### Pitfall: Parent Pointers and Ownership Constraints

- **What goes wrong:** You try to add a `parent: Option<&Directory>` field to each node so you can traverse upwards (e.g., to resolve absolute paths). The borrow checker aggressively rejects this.
- **Why it happens (the mechanism):** Trees with parent pointers are graphs containing reference cycles. A child cannot borrow its parent while the parent owns the child; that violates Rust's strict single-ownership rules.
- **How to handle it, and why that works:** 
  1. **Pass context down:** Instead of a child asking for its parent, the caller passes the parent path down as an argument: `fn get_path(&self, parent_path: &str)`.
  2. **Arena Allocation:** If you absolutely need cross-references, don't use `Box` or `Rc`. Store all nodes in a flat `Vec` and use indices to link them: `struct Node { parent: usize, children: Vec<usize> }`.
- **Trade-offs of the fix:** Context-passing pollutes method signatures. Arenas require you to pass the `Arena` object around to resolve any index.

### Pitfall: Stack Overflow on Deep Recursion

- **What goes wrong:** You build a deeply nested UI widget tree (10,000 nodes deep) and call `render()`. The application panics with a stack overflow.
- **Why it happens (the mechanism):** The elegant `self.children.iter().map(|c| c.size()).sum()` is a recursive function call. Each branch consumes a frame on the thread's call stack. Rust does not guarantee tail-call optimization, and tree traversal isn't tail-recursive anyway.
- **How to handle it, and why that works:** Write iterative evaluators using a heap-allocated stack (a `Vec`) instead of relying on the call stack. 
- **Trade-offs of the fix:** Iterative tree traversal is vastly more verbose and harder to read than the declarative recursive map/sum, sacrificing code clarity for runtime safety.

### Pitfall: Polluting the Trait with Tree Management

- **What goes wrong:** You define your trait as `trait Component { fn add(&mut self, node: Box<dyn Component>); fn size(&self) -> u64; }`. Since `File` is a leaf, its `add()` implementation just panics or does nothing.
- **Why it happens (the mechanism):** The original GoF book recommends maximizing uniformity by putting child-management methods on the base Component interface, causing runtime errors if you try to add a child to a leaf.
- **How to handle it, and why that works:** Embrace Interface Segregation. Keep `add_child()` and `remove_child()` off the trait. Define them only on the concrete `Directory` struct. 
- **Trade-offs of the fix:** You lose some uniformity. If a client is holding a `Box<dyn Component>`, they cannot add a child to it without downcasting it to a `Directory` first. This is usually the right choice in Rust.

## Design Decisions & Trade-offs

- **Enum vs Trait Object:** Default to `enum`. It is safer, faster, and easier to exhaustively match on. Only use `Box<dyn Trait>` when building a framework where external consumers need to inject their own custom leaf types.
- **Immutability:** Operations on a Composite should preferably be `&self`. Mutating a deeply nested tree via `&mut self` is possible but makes borrowing subsets of the tree difficult. 

## Exercises & Self-Test

1. Rewrite the `Directory::size` method iteratively. Use a `Vec<&dyn FileSystemNode>` as a manual stack to prevent stack overflow on deep trees.
2. In a UI Widget framework, why is it better to put the `add_widget(child)` method on a concrete `Panel` struct rather than on the `Widget` trait itself?
3. What is the fundamental ownership conflict when adding a `parent` reference to a `Box<dyn FileSystemNode>`? How does Arena allocation solve this?

## Open Questions

- When using an Arena for a Composite tree, how do you handle garbage collection (removing dead nodes) without leaking indices or invalidating existing references?

## References

- [Arena Allocation in Rust](https://manishearth.github.io/blog/2021/03/15/arenas-in-rust/)
- [Iterators](../iterator/learning.md)
