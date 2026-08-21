# State Machine — Learning Notes

## Mental Model

**When an object's behavior changes dramatically based on its current condition, `if/else` flags become a fragile, bug-prone mess.** The State pattern (and the broader concept of a State Machine) formalizes this by representing each "condition" as a distinct State. 

Instead of an object holding boolean flags like `is_connected` or `has_error` and checking them in every method, the object transitions between entirely different State representations. By binding the available actions directly to the current state, invalid actions are either structurally prevented or caught systematically. 

## Structure & Participants

### Context

- **Role:** The interface to the outside world. It tracks the current State and delegates operations to it.
- **In classic OOP:** A class holding a reference to a `State` interface.
- **In Rust:** A struct holding an `enum`, an `enum` itself, or a struct parameterized by a generic state type.

### State

- **Role:** Defines the data and behavior associated with a particular condition of the Context.
- **In classic OOP:** An interface or abstract base class.
- **In Rust:** An `enum` with variants for each state (for runtime state machines), or a trait/empty structs (for compile-time typestates).

### Concrete States

- **Role:** Implement the behavior for a specific state. Crucially, they dictate valid transitions to *other* states.
- **In Rust:** The data payloads inside the `enum` variants, or the concrete marker structs.

## Idiomatic Rust Implementation

Rust's `enum` is the ultimate tool for Runtime State Machines. Unlike C-style enums, Rust enums can hold data payload. Combined with pattern matching, Rust guarantees **exhaustiveness**: you cannot forget to handle a state, because the compiler won't let you.

To avoid borrow checker issues when transitioning state, the most idiomatic runtime approach is to take ownership (`self`), returning the new state. When integrating this back into a Context struct, the `Option::take` pattern allows us to temporarily move the state out of the struct.

```rust
pub struct CartData { pub items: Vec<String> }
pub struct PaidData { pub items: Vec<String>, pub receipt_id: String }

pub enum OrderState {
    Cart(CartData),
    Paid(PaidData),
}

impl OrderState {
    // Takes ownership of `self`, returning the new state
    pub fn checkout(self, receipt_id: String) -> Self {
        match self {
            OrderState::Cart(data) => {
                OrderState::Paid(PaidData {
                    items: data.items,
                    receipt_id,
                })
            }
            // Invalid transition: remain in current state
            other => other, 
        }
    }
}

pub struct Order {
    // Option allows us to take ownership of the state temporarily during transitions
    state: Option<OrderState>,
}

impl Order {
    pub fn new() -> Self {
        Self {
            state: Some(OrderState::Cart(CartData { items: vec![] })),
        }
    }

    pub fn checkout(&mut self, receipt_id: String) {
        // 1. Take state out, leaving None temporarily
        if let Some(state) = self.state.take() {
            // 2. Transition state and put it back
            self.state = Some(state.checkout(receipt_id));
        }
    }
}
```

## When This Pattern Dissolves in Rust

The classic OOP State pattern (where every state is a class implementing a State interface and transitioning via dynamic dispatch) is entirely replaced by **Enums and Match statements** in Rust. 

Furthermore, Rust takes this a step further with the **Typestate Pattern** (see [Type State](../type-state/learning.md)), which elevates the State Machine from a runtime check to a *compile-time guarantee*. In Typestate, attempting an invalid transition is a compilation error, not a runtime no-op.

## Worked Example

Consider a TCP Connection. We want to ensure we don't send data before connecting, and we don't connect if we're already connected.

**Stage 0: The Boolean Flag Mess**
```rust
struct Connection {
    is_connected: bool,
    is_closed: bool,
    ip: Option<String>,
}
// Methods must check `if self.is_connected && !self.is_closed` everywhere.
// `ip` might be accessed before it's set.
```

**Stage 1: Runtime Enum State Machine**
We group data into the state it belongs to.
```rust
enum ConnectionState {
    Disconnected,
    Connected { ip: String, bytes_sent: usize },
    Closed,
}

impl ConnectionState {
    fn connect(self, ip: String) -> Self {
        match self {
            ConnectionState::Disconnected => ConnectionState::Connected { ip, bytes_sent: 0 },
            other => other, // Ignore invalid transition
        }
    }
    
    fn send(self, bytes: usize) -> Self {
        match self {
            ConnectionState::Connected { ip, bytes_sent } => {
                ConnectionState::Connected { ip, bytes_sent: bytes_sent + bytes }
            }
            other => other,
        }
    }
}
```
This is safe at runtime: it's impossible to access `ip` when `Disconnected`. However, invalid transitions (like calling `send` on a `Disconnected` socket) compile fine but do nothing at runtime.

**Stage 2: Typestate (Compile-Time State Machine)**
We encode the states as distinct types, turning invalid transitions into compiler errors.
```rust
struct Connection<State> {
    state: State,
}

struct Disconnected;
struct Connected { ip: String, bytes_sent: usize }
struct Closed;

impl Connection<Disconnected> {
    fn connect(self, ip: String) -> Connection<Connected> {
        Connection { state: Connected { ip, bytes_sent: 0 } }
    }
}

impl Connection<Connected> {
    fn send(mut self, bytes: usize) -> Self {
        self.state.bytes_sent += bytes;
        self
    }
    fn close(self) -> Connection<Closed> {
        Connection { state: Closed }
    }
}
// A Disconnected connection literally does not have a `send` method.
```

## Versus

- **State vs Strategy:**
  - *Strategy* swaps algorithms based on external configuration. The Context doesn't change its fundamental nature.
  - *State* swaps behavior based on the internal lifecycle of the Context. The Context's available actions change entirely based on its state.
- **State vs Type State:**
  - *State Machine (Runtime)* checks transitions at runtime using `match` on an enum. Invalid transitions are handled silently or with runtime errors. Use this when states depend on runtime I/O or input.
  - *Type State (Compile-time)* encodes states as generic types. Invalid transitions fail to compile. Use this for linear, deterministic workflows where the sequence of states is strictly known at compile-time.

## Pitfalls in Depth

### Pitfall: State Data Bleed

- **What goes wrong:** You define a single struct with all fields for all states, but some fields are `Option<T>` because they only apply to certain states (e.g., `receipt_id` is `None` until checkout).
- **Why it happens (the mechanism):** You separated the *behavior* via methods, but you didn't separate the *data*. You end up continually `.unwrap()`ing Options that you "know" are `Some` in a given state, risking runtime panics.
- **How to handle it, and why that works:** Bind data exclusively to the state that needs it. In Rust, put the data *inside* the enum variant (e.g., `Paid(PaidData)`). This makes it mathematically impossible to access `receipt_id` while in the `Cart` state.
- **Trade-offs of the fix:** Accessing common data shared across all states requires matching on the enum or duplicating the data in every variant. (To mitigate, put shared data in the outer Context struct, and state-specific data in the enum).

### Pitfall: Mutation in Place (The Borrow Checker Trap)

- **What goes wrong:** You try to transition states by mutating `&mut self.state`. You want to take data out of one variant to put it into the next. The borrow checker rejects this because you cannot leave `self.state` in a partially moved, invalid state during the transition.
- **Why it happens (the mechanism):** Rust requires all data to be valid at all times. You can't gut a struct and rebuild it on the same memory location directly, because if a panic occurred midway, the memory would be corrupted.
- **How to handle it, and why that works:** 
  1. Have transition methods take `self` (ownership) and return a new `Self`.
  2. In the Context struct, wrap the state in an `Option<State>` and use `self.state.take()` to temporarily move the state out, transition it, and put the result back.
- **Trade-offs of the fix:** `Option::take` requires wrapping the state in `Option`, which means an extra `unwrap()` or `if let` when you just want to read the current state. 

### Pitfall: Silent Failures on Invalid Transitions

- **What goes wrong:** A bug in the caller logic attempts an invalid transition (e.g. paying an already paid order). The state machine silently ignores it, returning the current state unchanged via a wildcard `_ => self` match arm. The caller proceeds, assuming the transition succeeded.
- **Why it happens (the mechanism):** Lazy matching (`_ => self`) seems convenient to satisfy exhaustiveness, but it masks logical errors by turning invalid commands into no-ops.
- **How to handle it, and why that works:** Make transitions explicit and return a `Result`. If a transition is invalid, return an `Error` alongside the unchanged original state (so the caller can recover it).
  ```rust
  pub fn checkout(self) -> Result<Self, (Self, Error)> { ... }
  ```
- **Trade-offs of the fix:** Increased verbosity for the caller, who must now handle the `Result`.

## Design Decisions & Trade-offs

- **Taking `self` vs `Option::take`:** If your state machine is the *entire* object, taking `self` is cleanest (`let paid_order = cart.checkout();`). If your state machine is *embedded* inside a larger, long-lived Context object, you'll need the `Option::take` dance to mutate it in place.
- **Enums vs Typestate:** Use Typestate for strict, compile-time enforced pipelines (like a builder pattern or strict protocol). Use Enums when the state is driven by unpredictable runtime events (like a game character, or UI states) where storing heterogeneous states in a collection (like a `Vec`) is required.
- **Hand-rolled Enums vs Crates:** For simple state machines, an `enum` is perfect. For complex workflows (dozens of states, hierarchical states, entry/exit actions), reach for a library like `statig` or `machine` which provide macros and DSLs for building formal statecharts.

## Exercises & Self-Test

1. **Design:** Write a runtime state machine for a `VendingMachine` using an enum. States: `Idle`, `CoinInserted(u32)`, `Dispensing(Item)`. Ensure `CoinInserted` tracks the balance, and `Dispensing` consumes it.
2. **Refactor:** Convert the `VendingMachine` from Exercise 1 into a compile-time Typestate pattern. Observe which runtime checks become unnecessary.
3. Why does the `Option::take` pattern satisfy the borrow checker when mutating state in place?
4. Look at the [Circuit Breaker](../../architecture-patterns/circuit-breaker/learning.md) pattern. How does its state machine (Closed, Open, Half-Open) benefit from Rust enums?

## Open Questions

- What is the most actively maintained Rust crate for complex, hierarchical state machines (like Harel statecharts)?
- How do we cleanly share common state (like a database connection pool) across all variants without duplicating it in every enum variant payload?

## References

- [Typestate pattern in Rust](https://cliffle.com/blog/rust-typestate/) by Cliff Biffle
- [Hoverbear - Rust State Machine Patterns](https://hoverbear.org/blog/rust-state-machine-pattern/) - Essential reading on bridging enums and Typestate.
- [Architecture Patterns: Circuit Breaker](../../architecture-patterns/circuit-breaker/learning.md) - A prime real-world example of a state machine governing request flow.
