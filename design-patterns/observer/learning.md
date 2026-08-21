# Observer & Publish-Subscribe — Learning Notes

## Mental Model

**When a central piece of state changes, the components that care about it shouldn't have to constantly ask "Are you done yet?"** The Observer pattern defines a one-to-many dependency: a Subject maintains a list of Observers, and when the Subject's state changes, it iterates through that list and pushes an update to each.

Publish-Subscribe (Pub-Sub) is the decoupled cousin of Observer. Instead of the Subject knowing about the Observers directly, both talk to an intermediary (an Event Bus or Message Broker). Publishers fire events into the void; Subscribers bind to topics. This is the foundation of event-driven programming.

The core engineering constraint this solves is **coupling**: a payment processor shouldn't need to import and invoke the email-sending service, the metrics service, and the inventory service. It should just announce "payment succeeded" and let the relevant systems react.

## Structure & Participants

### Subject / Publisher
- **Role:** The source of truth or events. It manages the list of observers and broadcasts changes.
- **In classic OOP:** A class with `attach(observer)`, `detach(observer)`, and `notify()` methods.
- **In Rust:** A struct holding a `Vec` of closures, or the sending half of a broadcast channel (`broadcast::Sender`).

### Observer / Subscriber
- **Role:** The entity waiting for updates.
- **In classic OOP:** An object implementing an `Observer` interface with an `update()` method.
- **In Rust:** A closure (`Box<dyn FnMut(&Event) + Send + Sync>`), or a receiving loop on a channel (`broadcast::Receiver`).

### The Bus (Pub-Sub only)
- **Role:** The intermediary routing events from publishers to subscribers.
- **In Rust:** Often a `tokio::sync::broadcast` channel or a dedicated struct wrapping it.

## Idiomatic Rust Implementation

The classic OOP Observer pattern (where Subject and Observer hold mutable references to each other) **does not work well in Rust**. It violates the borrow checker's rule of exclusive mutability: you can't have the Subject iterate over Observers and mutate them if the Observers also need to mutate or access the Subject. 

Instead, Rust implementations rely on callback vectors for synchronous cases, and channels for asynchronous cases.

### 1. The Callback Vector (Synchronous)

Use this for simple, synchronous, single-threaded GUI or state updates. Notice the use of `FnMut` (so observers can mutate their own captures) and `Send + Sync` bounds (so the Subject can be safely shared across threads).

```rust
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct Event {
    pub value: i32,
}

// Observers must be Send + Sync to allow the Subject to be thread-safe.
type Observer = Box<dyn FnMut(&Event) + Send + Sync>;

pub struct Subject {
    observers: Vec<Observer>,
    state: i32,
}

impl Subject {
    pub fn new() -> Self {
        Self { observers: Vec::new(), state: 0 }
    }

    pub fn subscribe<F>(&mut self, f: F)
    where
        F: FnMut(&Event) + Send + Sync + 'static,
    {
        self.observers.push(Box::new(f));
    }

    pub fn change_state(&mut self, new_val: i32) {
        self.state = new_val;
        let event = Event { value: self.state };
        self.notify(&event);
    }

    // Mutable self is required to invoke FnMut closures
    fn notify(&mut self, event: &Event) {
        for obs in &mut self.observers {
            obs(event);
        }
    }
}

fn main() {
    let mut subject = Subject::new();
    let counter = Arc::new(Mutex::new(0));
    
    let counter_clone = Arc::clone(&counter);
    subject.subscribe(move |event| {
        // Interior mutability required to mutate shared state from a closure
        let mut val = counter_clone.lock().unwrap();
        *val += event.value;
        println!("Observer saw: {}, total: {}", event.value, *val);
    });
    
    subject.change_state(10);
}
```

### 2. Channels (Asynchronous Pub-Sub)

For concurrent or decoupled systems, channels are the idiomatic choice. `tokio::sync::broadcast` is perfect for one-to-many communication.

```rust
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub enum SystemEvent {
    UserLoggedIn(String),
    Shutdown,
}

#[tokio::main]
async fn main() {
    // The channel holds up to 16 unread messages.
    let (tx, mut rx1) = broadcast::channel::<SystemEvent>(16);
    
    // Create a second receiver by subscribing to the sender.
    let mut rx2 = tx.subscribe();

    // Subscriber 1 (e.g., Audit Logger)
    tokio::spawn(async move {
        while let Ok(event) = rx1.recv().await {
            println!("Logger saw: {:?}", event);
        }
    });

    // Subscriber 2 (e.g., Welcome Emailer)
    tokio::spawn(async move {
        while let Ok(event) = rx2.recv().await {
            if let SystemEvent::UserLoggedIn(name) = event {
                println!("Sending welcome email to {}", name);
            }
        }
    });

    // Publisher simply fires into the channel.
    tx.send(SystemEvent::UserLoggedIn("Alice".into())).unwrap();
    tx.send(SystemEvent::Shutdown).unwrap();
}
```

## When This Pattern Dissolves in Rust

The *classic OOP implementation* of Observer completely dissolves—or rather, is actively rejected—by Rust. In Java, an Observer often holds a reference back to the Subject to query state. In Rust, this creates a cyclical reference that the borrow checker forbids without `Rc<RefCell<T>>` overhead. 

Rust forces you toward **message passing** (channels) or purely **value-driven events** (callbacks that receive fully cloned/owned data). This results in vastly safer and less tightly-coupled designs.

## Worked Example

Consider a game engine where an entity taking damage needs to notify an achievement system, a UI health bar, and an audio manager.

**Stage 0: Tight Coupling.** The Entity explicitly owns its dependents:

```rust
// Bad: The core entity logic is coupled to UI and Audio.
pub fn take_damage(entity: &mut Entity, amount: f32, ui: &mut UiSystem, audio: &mut AudioSystem) {
    entity.health -= amount;
    ui.update_health(entity.id, entity.health);
    if amount > 50.0 {
        audio.play_critical_hit();
    }
}
```
Every time a new system cares about damage, `take_damage`'s signature changes.

**Stage 1: A broken attempt with `mpsc`.** 
You might try standard library channels (`std::sync::mpsc`). However, `mpsc` stands for Multi-Producer, **Single-Consumer**. If you send an event to an `mpsc` bus, either the UI *or* the Audio system will receive it, but not both. It does not broadcast.

**Stage 2: The Broadcast Bus.**
We use `tokio::sync::broadcast` (or a similar crate like `crossbeam-channel` or `flume` combined with bus logic, though `tokio` provides broadcast natively).

```rust
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub struct DamageEvent {
    pub entity_id: u32,
    pub amount: f32,
}

pub struct GameBus {
    pub damage_tx: broadcast::Sender<DamageEvent>,
}

#[tokio::main]
async fn main() {
    let (tx, mut ui_rx) = broadcast::channel::<DamageEvent>(16);
    let mut audio_rx = tx.subscribe();

    let bus = GameBus { damage_tx: tx };

    // UI System task
    tokio::spawn(async move {
        while let Ok(event) = ui_rx.recv().await {
            println!("UI: Updating health for entity {}", event.entity_id);
        }
    });

    // Audio System task
    tokio::spawn(async move {
        while let Ok(event) = audio_rx.recv().await {
            if event.amount > 50.0 {
                println!("Audio: Playing critical damage sound!");
            }
        }
    });

    // Core gameplay simply sends events without knowing who is listening.
    bus.damage_tx.send(DamageEvent { entity_id: 42, amount: 75.0 }).unwrap();
    bus.damage_tx.send(DamageEvent { entity_id: 10, amount: 10.0 }).unwrap();
}
```

## Versus

- **Observer vs Mediator:** 
  - *Observer* is one-way broadcast. The subject doesn't care who listens.
  - *Mediator* is central orchestration. Components talk *through* the Mediator to each other, often expecting specific interactions.
- **Observer vs Event-Driven Architecture (EDA):**
  - *Observer* is usually in-memory, within a single process.
  - *EDA* is distributed across networks (Kafka, RabbitMQ), dealing with serialization, network partitions, and persistence.

## Pitfalls in Depth

### Pitfall: Cyclical Ownership / Borrow Checker Hell

- **What goes wrong:** You try to implement a Java-style Observer where the Subject owns a `Vec<Rc<RefCell<dyn Observer>>>`, and the Observer holds an `Rc<RefCell<Subject>>`. You end up with runtime panics (`BorrowMutError`) because the Subject tries to borrow itself while iterating over observers, which in turn try to borrow the Subject.
- **Why it happens (the mechanism):** Rust guarantees exclusive access for mutation. You cannot have two active mutable references to the same data at runtime. When the Subject calls `notify(&mut self)`, it is exclusively borrowed. If the Observer then tries to borrow the Subject to read its state, it panics.
- **How to handle it, and why that works:** Stop passing `self` to the observers. Pass a distinct event payload (a value or clone) that contains all the data the observer needs. Alternatively, use channels so the Publisher and Subscriber share no state at all, only a message queue.
- **Trade-offs of the fix:** You may have to clone data to send it down channels, which incurs a slight performance cost compared to raw pointers.

### Pitfall: Lapsed Listeners (Memory Leaks)

- **What goes wrong:** A short-lived object registers a callback with a long-lived Subject. The object is logically destroyed, but the Subject still holds the closure in its `Vec`, preventing memory from being freed and executing dead logic forever.
- **Why it happens (the mechanism):** The Subject takes ownership of the callback closure via `Box::new()`, pinning its captures in memory for the Subject's lifetime. Because closures are anonymous types, you can't easily find and remove the specific closure from the `Vec`.
- **How to handle it, and why that works:** Have `subscribe` return a unique ID or a "drop-guard" struct. When the drop-guard goes out of scope, its `Drop` implementation removes the callback from the Subject. Or, with channels, when the `Receiver` is dropped, the connection is automatically severed.
- **Trade-offs of the fix:** Callbacks require complex bookkeeping to manage IDs and removal. Channels handle this natively but incur async/channel overhead.

### Pitfall: Slow Consumers / Lagging Receivers

- **What goes wrong:** A fast publisher continuously fires events. One slow observer (e.g., writing to a slow disk) can't keep up. Depending on the channel, either the entire system runs out of memory, or the slow observer silently misses events.
- **Why it happens (the mechanism):** If using unbounded channels, the queue grows infinitely. If using bounded channels like `tokio::sync::broadcast`, the channel avoids OOM by dropping the oldest messages. The slow receiver gets a `RecvError::Lagged(u64)` when it finally tries to read.
- **How to handle it, and why that works:** Use bounded channels to protect memory. The slow consumer must explicitly match on `RecvError::Lagged` and know how to recover—usually by querying the Subject for the absolute current state, effectively resetting itself.
- **Trade-offs of the fix:** Forcing the observer to know how to "resync" from scratch re-couples it to the Subject's API, defeating some of the purism of event-driven decoupling.

## Design Decisions & Trade-offs

- **Callbacks vs Channels:** Callbacks are synchronous (they block the publisher until all observers finish). This is great for strict ordering (like UI rendering). Channels are asynchronous, allowing the publisher to fire-and-forget, making them vastly superior for high-throughput or concurrent systems.
- **Event Payload Granularity:** Should you send `StateChanged` and force the observer to query the new state, or send `StateChanged { new_val: 5 }`? In Rust, **always send the data in the event**. Forcing observers to query the subject leads to the borrow checker hell mentioned above.

## Exercises & Self-Test

1. Explain why `Vec<Box<dyn FnMut(&Event)>>` is preferred over traits with `&mut self` when implementing synchronous observers.
2. In a `tokio::sync::broadcast` channel, what happens to the publisher if one receiver stops reading events? What happens to the receiver?
3. What is the fundamental difference between `std::sync::mpsc` and `tokio::sync::broadcast` that makes the latter suitable for the Observer pattern?
4. **Design Exercise:** Modify the synchronous `Subject` example to return a `SubscriptionId` on subscribe, and add an `unsubscribe(id)` method. Then, write a `SubscriptionTicket` struct that implements `Drop` to automatically unsubscribe.

## Open Questions

- What is the most idiomatic crate for synchronous event buses in ECS (Entity Component System) architectures like Bevy? (Bevy uses its own custom event queue mechanism, how does it handle lapsed listeners?)

## References

- [Tokio Broadcast Channel Docs](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html) - The gold standard for async pub-sub in Rust, explicitly addressing lag and bounding.
- [Rust Design Patterns Book - Observer](https://rust-unofficial.github.io/patterns/patterns/behavioural/observer.html)
