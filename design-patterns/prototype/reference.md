# Prototype — Reference

**One-Liner:** Create new objects by cloning an existing "prototype" instance, optimizing resource sharing via `Arc` or `Cow`.

## When to Use
- When initialization from scratch is expensive (e.g., requires parsing JSON or hitting a database).
- When you have a registry of pre-configured "templates" (like enemy types in a game).

## Structure Sketch
```rust
use std::sync::Arc;

#[derive(Clone)]
struct Prototype {
    shared_heavy_data: Arc<Vec<u8>>,
    unique_state: i32,
}

fn spawn(proto: &Prototype) -> Prototype {
    let mut instance = proto.clone(); // Cheap clone
    instance.unique_state = 0;        // Modify instance
    instance
}
```

## Rust Idiom
- The pattern maps 1:1 with the `Clone` trait.
- Optimize memory by wrapping large, immutable, shared data in `Arc<T>`.
- Modify the clone after instantiation to give it unique state.

## Versus
- **Factory:** Factory creates from scratch. Prototype clones from memory.
- **Builder:** Builder specifies every field explicitly. Prototype copies most fields implicitly.

## Pitfalls

| Pitfall | Mechanism | Fix | Trade-off |
| :--- | :--- | :--- | :--- |
| **Hidden Allocation Costs** | `[derive(Clone)]` deep-copies heap data like `Vec` and `String`. | Wrap read-only heap data in `Arc<T>`. | `Arc` adds slight atomic overhead. |
| **Accidental State Sharing** | Cloning `Arc<Mutex<T>>` shares mutable state with the prototype. | Only share immutable data, deep clone mutable state. | Complicates struct design. |
| **Stale Prototype Data** | Prototypes are accidentally mutated. | Store prototypes in a strictly read-only registry. | Requires a dedicated registry manager. |

## Rules of Thumb
- If `clone()` shows up in your profiler, you need `Arc`.
- Separate your data into "Template Data" (shared, `Arc`) and "Instance Data" (unique, directly inside the struct).

## Key References
- [Rust standard library - Clone trait](https://doc.rust-lang.org/std/clone/trait.Clone.html)
- [Rust standard library - Cow](https://doc.rust-lang.org/std/borrow/enum.Cow.html)
