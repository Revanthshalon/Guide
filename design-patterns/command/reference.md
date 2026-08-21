# Command — Quick Reference

## One-Liner

Reify a method call into data (an object or enum), disconnecting *what* to do from *when* and *where* it is executed.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| You need an Undo/Redo stack. | Actions are fire-and-forget and execute immediately. |
| You need to serialize requests to disk or over a network (RPC/Event Sourcing). | The action logic heavily depends on immediate return values. |
| You need to delay execution (task queues, thread pools). | The system is highly concurrent with untracked out-of-band state changes (ruins delta-based undo). |

## Structure Sketch

```rust
// The Receiver (State)
pub struct Document { text: String }

// The Command (Data)
pub enum EditorCmd {
    Insert { pos: usize, text: String },
    Delete { pos: usize, len: usize, deleted: Option<String> },
}

// The Invoker (Queue/History)
pub struct History {
    undo_stack: Vec<EditorCmd>,
    doc: Document,
}
```

## Rust Idiom

- **Enum Commands (Data-driven):** The most idiomatic approach. Use `enum` when commands are known at compile time. Trivial to serialize with `serde` and avoids dynamic dispatch.
- **Closures (Task queues):** If undo and serialization aren't needed, use `Box<dyn FnOnce() + Send>`.
- **Pass Context to Execute:** Never store a `&mut Receiver` inside the Command struct. Always pass `&mut Receiver` as an argument to `execute()`.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Strategy** | Strategy encapsulates an *algorithm* (how to do it, stateless). Command encapsulates a *request* (what to do, stateful). |
| **Event Sourcing** | Command is the *intent* to change state. Event Sourcing is the log of facts that state *has* changed. |
| **Memento** | Command stores a *delta* (inverse operations). Memento stores a *snapshot* of the entire state. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **The `&mut` Self-Containment Trap** | Do not store `&mut Receiver` in the Command. Pass `&mut Receiver` into `execute(context)`. | The Invoker must now know the exact type of the Receiver to pass it along. |
| **Inexact Undo (State Desync)** | Ensure *all* state mutations go through the Command queue. | Background tasks or collaborative syncs that modify state directly will corrupt delta-based undo. |
| **Polymorphic Serialization** | Switch to an `enum` instead of `dyn Trait`, or use a crate like `typetag`. | `dyn Trait` cannot be serialized by `serde` natively because the type is erased. |
| **Half-Executed Failures** | Commands must validate before applying, or apply to a clone, to avoid corrupting state on panic/error. | Destructive actions that fail midway can leave the system unrecoverable. |

## Rules of Thumb

- Prefer `enum` over `dyn Trait` for commands in Rust.
- If a command can be undone, it must store enough historical data to perfectly reconstruct the state (e.g., a Delete command must store the deleted text).
- The Invoker owns the Receiver, the Command owns the Arguments.
- For simple deferred execution without undo (like thread pools), standard closures (`Box<dyn FnOnce()>`) are exactly the Command pattern.

## Key References

- [Rust Design Patterns - Command](https://rust-unofficial.github.io/patterns/patterns/behavioural/command.html)
- [Serde documentation on Enum representations](https://serde.rs/enum-representations.html)
