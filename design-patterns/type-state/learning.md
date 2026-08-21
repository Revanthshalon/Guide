# Type State — Learning Notes

## Mental Model

State machines are everywhere, but traditionally, checking if a state transition is valid happens at *runtime* (e.g., throwing an error if you try to `read()` a closed socket). In Rust, the **Type State** pattern moves these runtime checks to *compile time*. 

The mental model is: **Encode the state of an object into its type. Make invalid operations physically unrepresentable in the type system.** If a connection is closed, there shouldn't be a boolean flag saying `is_closed: true`; instead, the `Connection` type should transform into a `ClosedConnection` type that simply doesn't have a `read()` method. You cannot misuse it because the compiler refuses to compile the code.

## Structure & Participants

### The Base Struct
- **Role:** The data-holding shell. It takes a generic parameter that represents its current state.
- **In classic OOP:** A single class with an `enum State` field.
- **In Rust:** `struct Protocol<State> { ... }`

### The State Markers
- **Role:** Empty structs (Zero-Sized Types) that represent the possible states.
- **In Rust:** `struct Connected;`, `struct Disconnected;`

### The Transitions
- **Role:** Methods that consume `self` (taking ownership) and return a new instance of the struct with a different state type.
- **In Rust:** `fn connect(self) -> Protocol<Connected>`

## Idiomatic Rust Implementation

Let's build a mock HTTP Request Builder that strictly enforces that you cannot set the body before the URI, and cannot call `build()` without setting both.

```rust
use std::marker::PhantomData;

// State Markers
pub struct NoUri;
pub struct HasUri;
pub struct HasBody;

// The Base Struct. The PhantomData tells the compiler we conceptually "own" 
// the state, even though it takes up zero space at runtime.
pub struct RequestBuilder<State> {
    uri: Option<String>,
    body: Option<String>,
    _state: PhantomData<State>,
}

// Initial state constructor
impl RequestBuilder<NoUri> {
    pub fn new() -> Self {
        RequestBuilder {
            uri: None,
            body: None,
            _state: PhantomData,
        }
    }

    // Transition: NoUri -> HasUri
    pub fn uri(self, uri: impl Into<String>) -> RequestBuilder<HasUri> {
        RequestBuilder {
            uri: Some(uri.into()),
            body: self.body,
            _state: PhantomData,
        }
    }
}

// Methods only available when we have a URI
impl RequestBuilder<HasUri> {
    // Transition: HasUri -> HasBody
    pub fn body(self, body: impl Into<String>) -> RequestBuilder<HasBody> {
        RequestBuilder {
            uri: self.uri,
            body: Some(body.into()),
            _state: PhantomData,
        }
    }
}

// Methods only available when we have a URI and a Body
impl RequestBuilder<HasBody> {
    pub fn build(self) -> String {
        format!("URI: {}, Body: {}", self.uri.unwrap(), self.body.unwrap())
    }
}
```

## When This Pattern Dissolves in Rust

This pattern *doesn't* dissolve; it is a profound realization of Rust's unique strengths (ownership + generics). 
- In Java or Go, you can return a new object of a different type, but the caller still holds a reference to the old one and could keep calling methods on it. 
- In Rust, `self` (move semantics) consumes the old state. The caller *literally cannot use the old state anymore*. This is what makes Type State flawless in Rust.

## Worked Example

### Stage 0: The Runtime Checks

In embedded Rust, turning a GPIO pin from an input to an output is a critical operation. Doing it wrong ruins hardware. A naive runtime approach:

```rust
pub struct Pin {
    number: u8,
    is_output: bool,
}

impl Pin {
    pub fn set_high(&mut self) -> Result<(), &'static str> {
        if !self.is_output { return Err("Cannot write to input pin"); }
        // write to hardware
        Ok(())
    }
}
```
This forces the caller to handle errors at runtime for something that should be a static guarantee.

### Stage 1: The Type State Approach

```rust
use std::marker::PhantomData;

pub struct Pin<Mode> {
    number: u8,
    _mode: PhantomData<Mode>,
}

pub struct Input;
pub struct Output;

impl Pin<Input> {
    pub fn new(number: u8) -> Self {
        Pin { number, _mode: PhantomData }
    }
    
    pub fn read(&self) -> bool { true /* read hardware */ }
    
    // Consumes self, changing the type
    pub fn into_output(self) -> Pin<Output> {
        // write to hardware registers to change mode...
        Pin { number: self.number, _mode: PhantomData }
    }
}

impl Pin<Output> {
    pub fn set_high(&mut self) { /* write hardware */ }
    pub fn set_low(&mut self) { /* write hardware */ }
}
```

Notice: `Pin<Input>` doesn't even have a `set_high()` method. If you pass an input pin to a function requiring `Pin<Output>`, the compiler rejects it. You physically cannot toggle an input pin.

## Versus

### Runtime State Machine
- **What's the same:** Both track states and transitions.
- **What's different:** Runtime state machines use `enum` fields and check `if self.state == State::Foo` on every method call, returning `Result<T, Error>`. Type State uses generics, zero-sized types, and returns raw values, doing the check at compile time.
- **How to decide:** Use Type State when transitions are deterministic and driven by your code (like builders). Use Runtime State Machines when transitions are driven by external inputs that you can't control (like a parser reading bytes, or a TCP socket receiving a remote FIN packet).

### Builder Pattern
- **What's the same:** Chained method calls for construction.
- **What's different:** A classic builder returns `Result<Build, Error>` because you might have forgotten a field. A Type State builder returns `Build` directly, because missing a field makes `build()` un-callable.

## Pitfalls in Depth

### Pitfall: State Explosion / Combinatorial Explosion

- **What goes wrong:** You have 4 independent boolean flags, resulting in 16 state structs, leading to a massive matrix of generic implementations.
- **Why it happens (the mechanism):** Type State requires defining transitions between discrete types. If states are orthogonal (independent), encoding them as a single monolithic type parameter scales exponentially (2^n combinations for n independent flags — 4 flags is 16 states, matching the example above, not 4! = 24).
- **How to handle it, and why that works:** Use multiple independent generic parameters: `struct Builder<HasUri, HasBody>`. Or, if the logic is too dynamic, abandon Type State for that specific boundary and use runtime checks.
- **Trade-offs of the fix:** Signatures become very noisy (`Builder<T, U, V, W>`).

### Pitfall: Monomorphization Bloat

- **What goes wrong:** Your binary size balloons, and compile times become painfully slow.
- **Why it happens (the mechanism):** Rust's compiler generates a brand new copy of generic methods for every distinct type parameter used (monomorphization). If you have generic helper methods inside your `Protocol<State>`, the compiler duplicates them for `Protocol<Connected>` and `Protocol<Disconnected>`.
- **How to handle it, and why that works:** Factor out common, non-state-dependent logic into a non-generic inner struct or free function. Only make the state-dependent wrapper generic.
- **Trade-offs of the fix:** Slightly more boilerplate to split the struct.

### Pitfall: Inability to store heterogeneous states

- **What goes wrong:** You try to put a `Socket<Connected>` and a `Socket<Disconnected>` in the same `Vec`, but the compiler rejects it.
- **Why it happens (the mechanism):** Generics monomorphize into unique, incompatible types.
- **How to handle it, and why that works:** Wrap the generic struct in a unified `enum` (Enum Dispatch!) that hides the type parameter.
- **Trade-offs of the fix:** You are converting the compile-time state back into a runtime state to store them, meaning when you extract them from the enum, you have to pattern match (runtime check) again.

## Design Decisions & Trade-offs

- **Ergonomics vs. Safety:** Type State shifts the burden from testing to compilation. It makes APIs incredibly safe but harder to learn, as users see intimidating compile errors like `expected struct Connected, found struct Disconnected`.

## Exercises & Self-Test

1. **Build Exercise:** Write a `Door` struct using Type State. It starts `Open`. `Open` doors can be `close()`d, returning a `Closed` door. `Closed` doors can be `lock()`ed or `open()`ed. Write a `main` function that proves you cannot lock an `Open` door.
2. How does Rust's ownership system (`self` vs `&self`) make the Type State pattern safer than in languages like Java?
3. Why do we need `PhantomData` in Type State structs? What happens if you remove it?
4. **Build Exercise:** Refactor the `RequestBuilder` to use two independent generic parameters (`UriState` and `BodyState`) instead of a single monolithic state, to avoid combinatorial explosion if you added more fields.

## Open Questions

- What is the best way to document Type State APIs so that Rustdoc clearly shows the required transition graph?
- Is there a macro in the ecosystem that can automatically generate boilerplate for combinatorial Type State parameters?

## References

- [Typestate pattern in Rust](https://cliffle.com/blog/rust-typestate/) by Cliff Biffle.
- Cross-ref: State Machine (`../state-machine/learning.md`), Builder (`../builder/learning.md`), Marker Traits & Phantom Types (`../marker-traits-and-phantom-types/learning.md`)
