# Memento — Quick Reference

## One-Liner

Capture and restore an object's internal state (for "Undo" or rollback) using an opaque token, preserving encapsulation by hiding the internal fields from the outside world.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| You need to implement Undo/Redo or rollback mechanisms. | The state is so massive that full snapshots cause OOM errors (and `Arc` isn't enough). |
| You want to save state without adding public getters/setters that violate encapsulation. | The object manages external resources (sockets, file handles) that cannot be cleanly "rolled back". |

## Structure Sketch

```rust
pub mod domain {
    use std::sync::Arc;

    // 1. The Memento (Opaque to the outside)
    pub struct Snapshot { state: Arc<String> }

    // 2. The Originator
    pub struct Entity { state: Arc<String> }

    impl Entity {
        pub fn mutate(&mut self, text: &str) { 
            Arc::make_mut(&mut self.state).push_str(text); 
        }
        
        // Expose snapshot creation/restoration, not the data
        pub fn save(&self) -> Snapshot { Snapshot { state: Arc::clone(&self.state) } }
        pub fn restore(&mut self, s: Snapshot) { self.state = s.state; }
    }
}

// 3. The Caretaker (outside the module)
use domain::{Entity, Snapshot};
pub struct Caretaker {
    history: Vec<Snapshot>,
}
```

## Rust Idiom

**Module-level privacy + `Clone` or `serde`.** Place the Originator and Memento in the same module so they can see each other's private fields. To the rest of the application, the Memento is just an opaque token. For persistent mementos, `serde` serialization is the standard. Use `Arc` for Copy-On-Write sharing of large data.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Command** | Command saves the *operation* (diff); Memento saves the *state* (snapshot). |
| **Event Sourcing** | Event Sourcing is a whole-system architecture where history *is* the truth; Memento is a local cache of past states. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Memory Exhaustion (OOM)** | Cap the undo stack history (e.g., 50 items max). Use `Arc` or immutable data structures (the `im` crate) for Copy-On-Write memory sharing. | Creating a 10MB snapshot on every keystroke. |
| **Encapsulation leaks** | Keep Memento fields private. The Caretaker must never be able to read or alter the Memento's contents. | A Caretaker mutating a snapshot before handing it back. |
| **External invalidation** | Only use Memento for isolated, leaf-node data structures. | Rolling back an object that holds an active socket. |

## Rules of Thumb

- If the object has no internal invariants to protect, just derive `Clone` and store the object itself.
- Memento is vastly easier to implement than Command for undo functionality, but uses much more memory.
- An unbounded `undo_stack` is a memory leak waiting to happen. Always enforce a memory budget.

## Key References

- [Rust Book: Privacy Rules](https://doc.rust-lang.org/book/ch07-03-paths-for-referring-to-an-item-in-the-module-tree.html)
- Related: [Command Pattern](../command/learning.md)
