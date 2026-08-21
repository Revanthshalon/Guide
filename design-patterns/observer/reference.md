# Observer & Publish-Subscribe — Quick Reference

## One-Liner

Define a one-to-many relationship so that when a subject changes state, all registered dependents are notified automatically without tight coupling.

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| Multiple parts of an app need to react to a single event (e.g., UI updates, logging, achievements). | The flow is strictly sequential and predictable (just use direct function calls). |
| You want to decouple the sender of an event from the receivers. | You require a response/return value from the event (Observer is fire-and-forget). |

## Structure Sketch

```rust
// Idiomatic Rust usually uses Channels rather than OOP Subject/Observer traits.
use tokio::sync::broadcast;

#[derive(Clone)]
enum Event { Click(i32, i32), Quit }

// Publisher
let (tx, mut rx1) = broadcast::channel::<Event>(16);
let mut rx2 = tx.subscribe(); // Additional listeners subscribe to the sender

// rx1 and rx2 will both receive this event
tx.send(Event::Click(10, 20)).unwrap();
```

## Rust Idiom

Avoid mutual mutable references (Subject owning Observers, Observers holding a ref to Subject). 
1. **Async/Concurrent:** Use `tokio::sync::broadcast` channels. Do **not** use `std::sync::mpsc` as it cannot broadcast to multiple receivers.
2. **Synchronous/UI:** Use a `Vec<Box<dyn FnMut(&Event) + Send + Sync>>`. Pass cloned data *in the event*, do not force observers to query the subject back.

## Versus

| Confused with | Key difference |
| --- | --- |
| **Mediator** | Mediator routes traffic between specific components. Pub-Sub broadcasts to unknown listeners. |
| **Event-Driven Architecture** | EDA is Observer scaled to distributed networks (Kafka, SNS). Observer is usually in-process. |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| **Borrow Checker Hell** | Pass state *inside* the event payload so observers don't need a reference back to the Subject. | Observers that genuinely need to mutate the original Subject. |
| **Lapsed Listeners** | Use drop-guards for callbacks, or channels (when `Receiver` drops, it safely disconnects). | Memory leaks from permanently retained closures in a `Vec`. |
| **Slow Consumers** | Use bounded channels (`broadcast::channel(16)`). Handle the `Lagged` error on the receiver side. | Unbounded channels causing Out-Of-Memory (OOM) crashes. |

## Rules of Thumb

- Prefer **message passing (channels)** over **shared state (callbacks)**.
- Event payloads should be plain data types (`structs` or `enums`), explicitly cloned or passed by reference.
- Never use `std::sync::mpsc` for an Event Bus. It is Multi-Producer Single-Consumer and will starve all but one observer.
- If an observer needs to reply, include a oneshot channel inside the event payload (`struct Request { data: String, reply_to: oneshot::Sender<Response> }`).

## Key References

- [Tokio Broadcast Channel Docs](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)
- [Rust Design Patterns Book - Observer](https://rust-unofficial.github.io/patterns/patterns/behavioural/observer.html)
