# Mediator — Learning Notes

## Mental Model

**When components in a system are communicating directly with too many other components (a spiderweb of dependencies), introduce a central hub to coordinate them.** 

Imagine an air traffic control tower. If planes had to talk to every other plane to figure out who lands next, the sky would be chaos. Instead, planes only talk to the tower (the Mediator), and the tower tells the planes what to do. The planes don't know about each other; they only know the tower.

In software, if a Button click needs to disable a TextField, update a Progress Bar, and send a Network Request, wiring all those objects directly to the Button creates tight coupling. A Mediator sits in the middle: the Button tells the Mediator "I was clicked," and the Mediator orchestrates the TextField, ProgressBar, and Network Request. **Critically, the Mediator does not perform the work itself; it acts as a message router between independent colleagues.**

## Structure & Participants

### The Mediator
- **Role:** Centralizes complex communication and control logic between components.
- **In Rust:** An event loop running over a `Receiver`, holding `Sender`s to various sub-components.

### The Colleagues (Components)
- **Role:** The independent pieces of the system. They do not know about each other, and they do not know the Mediator explicitly—they just know how to send and receive messages.
- **In Rust:** Independent structs holding a `Sender` (to emit events) and optionally a `Receiver` (to receive commands).

## Idiomatic Rust Implementation & When It Dissolves

**The classical OOP Mediator (where colleagues hold references to the mediator, and the mediator holds references to the colleagues) is extremely hostile to Rust's ownership model.** It creates circular references requiring `Rc<RefCell<T>>` everywhere.

Instead, Rust implements Mediator via **Message Passing (Channels)** or **Actor Systems**. If you are using `actix` or `tokio::mpsc`, you are already using the idiomatic Rust Mediator. The classical pattern "dissolves" into standard asynchronous concurrency patterns.

## Worked Example

A checkout UI interacting with a Payment system and a UI Controller. In a Facade, the central object calls methods on dependencies directly. In a true Mediator, components are completely decoupled and run independently.

```rust
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

// 1. The Language of the System
pub enum Event {
    CheckoutClicked,
    PaymentSucceeded(u64),
    PaymentFailed,
}

pub enum Command {
    ChargePayment(u64),
    UpdateUI(String),
}

// 2. Independent Colleagues
// They only know about their channels. They do NOT know about the Mediator.
pub struct CheckoutButton {
    pub tx: Sender<Event>,
}
impl CheckoutButton {
    pub fn click(&self) {
        println!("Button clicked!");
        let _ = self.tx.send(Event::CheckoutClicked);
    }
}

pub struct PaymentProcessor {
    pub tx: Sender<Event>,
    pub rx: Receiver<Command>,
}
impl PaymentProcessor {
    pub fn run(self) {
        for cmd in self.rx {
            if let Command::ChargePayment(amount) = cmd {
                println!("Processing payment of ${}...", amount);
                // Simulate success
                let _ = self.tx.send(Event::PaymentSucceeded(amount));
            }
        }
    }
}

pub struct UIController {
    pub rx: Receiver<Command>,
}
impl UIController {
    pub fn run(self) {
        for cmd in self.rx {
            if let Command::UpdateUI(msg) = cmd {
                println!("UI Updated: {}", msg);
            }
        }
    }
}

// 3. The Mediator
// A pure router coordinating the flow of messages.
pub struct AppMediator {
    pub rx: Receiver<Event>,
    pub payment_tx: Sender<Command>,
    pub ui_tx: Sender<Command>,
}
impl AppMediator {
    pub fn run(self) {
        for event in self.rx {
            match event {
                Event::CheckoutClicked => {
                    let _ = self.payment_tx.send(Command::ChargePayment(100));
                }
                Event::PaymentSucceeded(amount) => {
                    let msg = format!("Success: ${}", amount);
                    let _ = self.ui_tx.send(Command::UpdateUI(msg));
                    break; // End app for example
                }
                Event::PaymentFailed => {
                    let _ = self.ui_tx.send(Command::UpdateUI("Error".to_string()));
                }
            }
        }
    }
}

fn main() {
    let (event_tx, event_rx) = channel();
    let (payment_tx, payment_rx) = channel();
    let (ui_tx, ui_rx) = channel();
    
    let button = CheckoutButton { tx: event_tx.clone() };
    let payment = PaymentProcessor { tx: event_tx.clone(), rx: payment_rx };
    let ui = UIController { rx: ui_rx };
    
    let mediator = AppMediator { rx: event_rx, payment_tx, ui_tx };

    // Colleagues can run on entirely separate threads
    thread::spawn(move || payment.run());
    thread::spawn(move || ui.run());
    thread::spawn(move || button.click());

    // Mediator coordinates them
    mediator.run();
}
```

## Versus

### Versus Observer
- **Observer** is a distributed broadcast. A subject yells "I changed!" and anyone listening reacts. The observers hold the business logic of how to react.
- **Mediator** is centralized. Colleagues yell "I changed!" to *one* central hub, and the hub holds the business logic of what to do about it.

### Versus Facade
- **Facade** provides a simplified structural interface to a complex subsystem (one-way). The Facade owns and directly calls methods on the subsystem.
- **Mediator** coordinates behavior between active, decoupled components (two-way). The components run independently and emit events back to the Mediator.

## Pitfalls in Depth

### Pitfall: The God Object
- **What goes wrong:** The Mediator absorbs so much logic that it becomes a monolithic "God Object" containing the entire application's business rules.
- **Why it happens (the mechanism):** Because it's so easy to route an event to the Mediator, developers stop putting logic in the colleagues. The colleagues become hollow shells that just emit events, and the Mediator becomes a 10,000-line `match` statement.
- **How to handle it, and why that works:** The Mediator should *coordinate*, not *execute*. When the Mediator receives `CheckoutClicked`, it shouldn't contain the math for calculating taxes; it should tell the `TaxCalculator` component to do it.
- **Trade-offs of the fix:** Requires disciplined boundaries and passing state back and forth, rather than just writing the logic inline.

### Pitfall: Circular Lifetimes (The OOP Trap)
- **What goes wrong:** You try to implement the GoF version: Mediator holds `&mut Colleague`, Colleague holds `&mut Mediator`. The compiler completely rejects it.
- **Why it happens (the mechanism):** Rust forbids mutable aliasing and circular ownership by design to prevent data races and memory unsafety.
- **How to handle it, and why that works:** Use message passing (Channels). `Sender`s inherently decouple the colleagues from the Mediator's lifetime and satisfy the compiler.
- **Trade-offs of the fix:** Introduces slight message-passing overhead and requires designing a protocol (`enum Event`) instead of direct method calls.

### Pitfall: Missing Backpressure
- **What goes wrong:** The Mediator falls behind in processing messages. Memory usage balloons until the process OOMs, or latency spikes astronomically.
- **Why it happens (the mechanism):** Using unbounded channels (`std::sync::mpsc::channel` or `tokio::sync::mpsc::unbounded_channel`) means senders can enqueue messages infinitely fast. If the Mediator's `run` loop blocks or is slower than the producers, the queue grows forever.
- **How to handle it, and why that works:** Always use bounded channels (`sync_channel` in `std`, `mpsc::channel(capacity)` in `tokio`). This forces producers to block (or yield in async) when the Mediator is saturated, applying backpressure naturally.
- **Trade-offs of the fix:** Bounded channels can lead to deadlocks if two components are mutually waiting to send to each other while queues are full.

## Design Decisions & Trade-offs

**`Send + 'static` for Threading:** Notice in the example that we used `thread::spawn(move || ...)`. For a colleague to run independently, its state and its channels must be `Send` (safe to move across threads) and `'static` (owning all their data, no temporary borrows). Channel receivers/senders naturally fulfill this.

**Typed Channels vs Enum Messages:**
If you use a single channel with an `enum Event`, the Mediator must handle every variant in one massive loop. If the system is highly decentralized, having separate channels for distinct subsystems prevents the central `enum` from growing infinitely and allows distributing the Mediator logic into smaller orchestrators.

## Exercises & Self-Test

1. Why does the classical OOP Mediator (with mutual references) fail to compile easily in Rust?
2. Explain the difference between a Mediator and a Facade. Look at the `Worked Example`—how would it differ if it were a Facade?
3. What is the risk of using `mpsc::channel()` instead of `mpsc::sync_channel(100)` for your Mediator's event queue?
4. Modify the worked example so that `PaymentProcessor` runs asynchronously using `tokio` instead of `std::thread`.

## Open Questions

- When building GUI applications in Rust (like with `egui` or `slint`), how much should be handled by a central Mediator vs local widget state?
- How do you test a Mediator that sits at the center of 15 different components without writing massive integration tests?

## References

- [Elm Architecture](https://guide.elm-lang.org/architecture/) — The conceptual basis for the channel-based Mediator in modern UI design (Model-View-Update).
- Related: [Channels in Rust (std::sync::mpsc)](https://doc.rust-lang.org/std/sync/mpsc/index.html)
