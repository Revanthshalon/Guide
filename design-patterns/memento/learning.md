# Memento — Learning Notes

## Mental Model

The Memento pattern solves a conflict between two requirements:
1. You need to save and restore the internal state of an object (e.g., for an "Undo" feature).
2. You cannot expose the object's internal fields, because doing so violates encapsulation and allows other objects to corrupt the state.

The solution is the **Memento**: an opaque token that captures the state. The object itself creates the token, hands it to a caretaker (which holds onto it but cannot look inside it), and later accepts the token back to restore itself.

## Structure & Participants

- **Originator:** The object whose state needs to be saved and restored. It creates the Memento.
- **Memento:** A snapshot of the Originator's state. Its fields are strictly private to the Originator.
- **Caretaker:** The object that holds the Memento (e.g., an Undo stack). It knows *when* to save and *when* to restore, but it does not know *what* is inside the Memento.

## Idiomatic Rust Implementation

In Java or C++, Memento is often implemented using nested classes or `friend` declarations. In Rust, we use **Module Privacy**. 

By placing the Originator and the Memento in the same module, the Originator can access the Memento's private fields, while the Caretaker (which lives outside the module) sees the Memento as a completely opaque type.

```rust
// 1. The Module Boundary creates the privacy shield
pub mod editor {
    use std::sync::Arc;

    // The Memento (Opaque to the outside world)
    pub struct Snapshot {
        // Private fields! Only `editor` module can access these.
        // Using Arc for Copy-On-Write memory efficiency
        text: Arc<String>,
        cursor_position: usize,
    }

    // The Originator
    #[derive(Default)]
    pub struct TextEditor {
        text: Arc<String>,
        cursor_position: usize,
    }

    impl TextEditor {
        pub fn type_text(&mut self, new_text: &str) {
            // Unshare the Arc to mutate it safely
            let text_mut = Arc::make_mut(&mut self.text);
            text_mut.push_str(new_text);
            self.cursor_position += new_text.len();
        }

        // Create the Memento
        pub fn save(&self) -> Snapshot {
            Snapshot {
                // Arc cloning is cheap (just bumps a counter)
                text: Arc::clone(&self.text),
                cursor_position: self.cursor_position,
            }
        }

        // Consume the Memento to restore state
        pub fn restore(&mut self, snapshot: Snapshot) {
            self.text = snapshot.text;
            self.cursor_position = snapshot.cursor_position;
        }

        pub fn display(&self) -> String {
            format!("{} (cursor at {})", self.text, self.cursor_position)
        }
    }
}

// 2. The Caretaker (Lives outside the module)
use editor::{TextEditor, Snapshot};

pub struct History {
    // Memory budget: limit to 50 snapshots
    max_capacity: usize,
    undo_stack: Vec<Snapshot>,
}

impl History {
    pub fn new(max_capacity: usize) -> Self {
        History { max_capacity, undo_stack: Vec::new() }
    }

    pub fn backup(&mut self, editor: &TextEditor) {
        if self.undo_stack.len() >= self.max_capacity {
            self.undo_stack.remove(0); // Evict oldest
        }
        // We hold the snapshot, but cannot access `snapshot.text`!
        self.undo_stack.push(editor.save());
    }

    pub fn undo(&mut self, editor: &mut TextEditor) {
        if let Some(snapshot) = self.undo_stack.pop() {
            editor.restore(snapshot);
        }
    }
}
```

## When This Pattern Dissolves in Rust

For many structs, if there are no complex internal invariants to protect, you don't need a separate Memento type. You just derive `Clone`, and the Originator *is* its own Memento.

If you need persistent snapshots (saving to disk), the Memento pattern merges entirely with **Serialization**. The `serde` crate acts as the mechanism to generate the snapshot (JSON/Bincode), and the file system acts as the Caretaker.

## Versus

### Versus Command

- **Command** for undo records *what happened* (the operation: "insert 'a' at position 5"). To undo, you apply the inverse operation.
- **Memento** for undo records *what the state was* (the snapshot: "the text is now 'hello'"). To undo, you blast the old state over the current state.
- **How to decide:** Command uses less memory (diffs vs full snapshots) but is much harder to implement correctly (you must write perfect inverse math for every operation). Memento is memory-heavy but trivially easy to implement (just copy the state).

### Versus Event Sourcing

- **Event Sourcing** (`../../architecture-patterns/event-sourcing/learning.md`) is a macro-architecture where the history of events is the single source of truth, and current state is derived.
- **Memento** is a micro-pattern where current state is the source of truth, and history is just a cache of previous states for rollback.

## Pitfalls in Depth

### Pitfall: Memory Exhaustion (The Unbudgeted Stack)

- **What goes wrong:** A text editor backs up a 50MB document 200 times. The application uses 10GB of RAM and crashes.
- **Why it happens (the mechanism):** The Memento captures the *full state* of the Originator on every save.
- **How to handle it, and why that works:** 
  1. **Cap the stack:** The Caretaker must enforce a maximum history size memory budget (e.g., only keep the last 50 mementos, dropping the oldest).
  2. **Copy-on-write:** Use Rust's `std::sync::Arc` for large data structures inside the Memento. If a string doesn't change between snapshots, two Mementos share the same `Arc<String>`.
- **Trade-offs of the fix:** `Arc` adds slight overhead to mutation, but drastically reduces memory usage for snapshots. Dropping old mementos limits the user's undo depth.

### Pitfall: Invalidating External References

- **What goes wrong:** You restore a system to a state from 10 minutes ago. Other parts of the system are holding IDs, references, or open network connections that existed in the current state but don't exist in the restored state.
- **Why it happens (the mechanism):** Memento only restores the *internal* state of the Originator. It cannot magically update the rest of the world.
- **How to handle it, and why that works:** Memento is best used for isolated, leaf-node data structures (like a text buffer, a canvas drawing). Do not use Memento to roll back a database connection pool or an active socket listener. 
- **Trade-offs of the fix:** Restricts the pattern to domain models and pure data layers, rather than whole-application state management.

### Pitfall: Leaking the Memento's Internals

- **What goes wrong:** The Caretaker inspects the Memento, reads a value, and makes a business logic decision based on it, or worse, mutates it before handing it back.
- **Why it happens (the mechanism):** The developer made the fields of the Memento `pub` because the Caretaker and the Originator were written in the same file or without strict module boundaries.
- **How to handle it, and why that works:** Keep the fields of the Memento strictly private. Put the Originator and Memento in a module together, and keep the Caretaker outside that module.
- **Trade-offs of the fix:** Forces a specific file/module structure, which might slightly complicate simple scripts but guarantees encapsulation.

## Design Decisions & Trade-offs

**Full Snapshot vs Diff:** 
If state is massive, a pure Memento is too expensive. You must hybridize it with Command by storing a "diff" (what changed). In Rust, crates like `im` (immutable data structures) make full snapshots incredibly cheap because they share memory structurally, giving you the ease of Memento with the memory footprint of Command.

## Exercises & Self-Test

1. How does Rust's module system natively enforce the strict encapsulation rule of the Memento pattern?
2. Compare the memory usage of an Undo system built with Command vs one built with Memento.
3. Modify the `TextEditor` example above to use `Arc<String>` instead of `String` to simulate a memory-efficient Memento for very large documents.
4. When does it make sense for a struct to act as its own Memento (via `Clone`)?

## Open Questions

- How do you handle migrations if a persistent Memento is serialized to disk, but the Originator's schema changes in a newer version of the application?
- Can we use the Typestate pattern to ensure at compile-time that a Caretaker doesn't try to apply a Memento from Editor A onto Editor B?

## References

- [The `im` crate](https://crates.io/crates/im) — Immutable data structures for Rust, which make Memento-style snapshots practically free in terms of memory.
- Related: [Command Pattern](../command/learning.md)
