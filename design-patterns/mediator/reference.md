# Mediator — Quick Reference

## One-Liner

Centralize complex communication and coordination between independent components into a single routing hub, preventing a chaotic web of direct component-to-component dependencies.

## When to Use

| Use it when | Avoid it when |
| --- | --- |
| Components are tightly coupled, making them hard to reuse or test independently. | Components only need simple, 1-to-1 communication. |
| You need to coordinate a workflow across multiple active, independent processes or threads. | The Mediator starts absorbing all computational business logic (God Object). |

## Structure Sketch (The Channel Idiom)

```rust
use std::sync::mpsc::{channel, Sender, Receiver};

// The Vocabulary
pub enum Event { SaveClicked }
pub enum Command { WriteToDisk }

// The Colleague (Independent)
pub struct Button { tx: Sender<Event> }

// The Mediator (Router)
pub struct Orchestrator {
    rx: Receiver<Event>,
    storage_tx: Sender<Command>,
}

impl Orchestrator {
    pub fn run(self) {
        for event in self.rx {
            match event {
                Event::SaveClicked => {
                    let _ = self.storage_tx.send(Command::WriteToDisk);
                }
            }
        }
    }
}
```

## Rust Idiom

**Channels (`mpsc`) and Event Loops.** Do not use the classical OOP approach of storing mutual references (`Rc<RefCell<Mediator>>`). Instead, colleagues hold a `Sender`, and the Mediator runs a loop on the `Receiver`. This perfectly satisfies Rust's ownership and thread-safety (`Send + 'static`) rules.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Observer** | Observer distributes coordination logic (everyone reacts); Mediator centralizes it (one hub decides what happens). |
| **Facade** | Facade is structural (simplifies an API one-way by calling methods directly); Mediator is behavioral (coordinates decoupled active components two-way). |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Circular Ownership** | Use Message Passing (Channels) instead of direct `&self` references. | `Rc<RefCell<T>>` spreading like a virus through the codebase. |
| **The God Object** | The Mediator should route and coordinate, but delegate actual computation back to components. | A single `match` statement that is 5,000 lines long. |
| **Missing Backpressure** | Use bounded channels (`mpsc::sync_channel`) instead of unbounded ones. | OOM crashes because producers enqueue events faster than the Mediator can route them. |

## Rules of Thumb

- If a component knows about more than 3 other distinct components just to notify them of changes, you probably need a Mediator.
- A well-designed Colleague should be entirely testable by passing it a mock `Sender`.
- The Mediator should **not** call `component.do_work()` directly if the component is an active entity; it should send a message to the component's command channel.

## Key References

- [Rust Book: Message Passing](https://doc.rust-lang.org/book/ch16-02-message-passing.html)
