# Composite — Quick Reference

## One-Liner

The Composite pattern composes heterogeneous objects into tree structures, allowing clients to query and manipulate single items and collections of items uniformly.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| Modeling part-whole hierarchies (File systems, UI DOMs, ASTs) | The structure is a graph with cycles, not a tree |
| Clients shouldn't care if they're holding a leaf or a branch | You need rigid rules about which nodes can hold which types of children |

## Structure Sketch

```rust
// The domain interface
pub trait FileSystemNode {
    fn size(&self) -> u64;
}

// The Leaf
pub struct File { pub size_bytes: u64 }
impl FileSystemNode for File {
    fn size(&self) -> u64 { self.size_bytes }
}

// The Composite
pub struct Directory {
    pub children: Vec<Box<dyn FileSystemNode>>,
}
impl FileSystemNode for Directory {
    fn size(&self) -> u64 {
        // Recursive aggregation
        self.children.iter().map(|child| child.size()).sum()
    }
}
```

## Rust Idiom

- **Enums for Closed Trees:** If the tree types are fixed, `enum FsNode { File, Dir(Vec<FsNode>) }` is much faster and safer than trait objects.
- **Arena Allocation:** If your tree requires `parent` pointers, do not use `Box` or `Rc`. Use a flat `Vec` of nodes and link them via `usize` indices.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Decorator** | Has exactly 1 child and adds behavior vs. has N children and aggregates them structurally. |
| **Iterator** | Used to traverse an existing collection vs. defines the hierarchical structure itself. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Reference Cycles** | Never use `&'a Parent` in a child. Use Arena indices. | The borrow checker blocking parent pointers. |
| **Stack Overflow** | Convert deep recursive methods to iterative ones using a heap-allocated `Vec` as a stack. | Deeply nested ASTs panicking on `eval()`. |
| **Polluted Traits** | Keep `add_child()` on the Composite concrete type, not the base Component trait. | Adding `add_child()` to a `File` and panicking. |

## Rules of Thumb

- A `Directory` shouldn't care what kind of `FileSystemNode` it holds.
- Rely on indirection (`Box`, `Vec`, or Arena indices) to define recursive types.
- Let Interface Segregation override strict OOP Composite rules: don't force leaves to implement collection methods.

## Key References

- [Arena Allocation in Rust](https://manishearth.github.io/blog/2021/03/15/arenas-in-rust/)
