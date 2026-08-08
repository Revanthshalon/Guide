# Rust — Learning Notes

## Mental Model

**Ownership is not a memory-safety bolt-on — it's a type system for describing who's responsible for what, and the compiler is checking your architecture, not just your pointers.** Every other language lets you write "this function takes a `Thing`" without saying whether the function *keeps* it, *borrows* it briefly, or *shares* it with others — that ambiguity is resolved at runtime (GC) or not at all (C's dangling pointers). Rust makes you say which, in the signature, and then enforces it. The payoff is not "no segfaults" — it's that **the function signature is a complete contract**: `fn f(x: T)` takes ownership (the caller loses access); `fn f(x: &T)` borrows read-only (caller keeps it, many readers coexist); `fn f(x: &mut T)` borrows exclusively (caller keeps it, but nobody else may touch it meanwhile). Reading a signature tells you the data-flow story without reading the body — a property most languages can't offer at any price.

The corollary that reorganizes how you think about every design problem: **most "how do I structure this?" questions are actually "who owns this, and for how long?" questions.** A graph of objects with pointers back and forth is not merely awkward in Rust — the compiler is telling you the ownership story is genuinely ambiguous, and it's making you resolve the ambiguity *before* the bug ships instead of after. The idiomatic responses — arenas with index-based references, single-owner trees with explicit weak back-links, message passing instead of shared mutable state — aren't workarounds for the borrow checker; they're what the design looks like once the ownership question has an honest answer. This is the same lesson the [performance-optimization](../../performance-optimization/data-oriented-design/learning.md) docs teach from the opposite direction (index arenas, SoA) — Rust's type system and Rust's performance idioms point at the same designs because ownership clarity and cache-friendly layout have the same enemy: implicit, tangled, many-owner data.

Three consequences worth internalizing up front, because they explain a lot of what follows:

- **The compiler is a design-review partner, not an obstacle.** A fight with the borrow checker is almost always the compiler correctly reporting that your mental model of the data's lifetime doesn't match what the code does. The fix is rarely a workaround (`.clone()`, `Rc<RefCell<T>>`) — it's usually a clearer design that was available all along.
- **`unsafe` doesn't turn off the rules — it moves the proof obligation to you.** Safe Rust's guarantees (`&mut` is exclusive, no dangling references, no data races) are still *true* in a correct `unsafe` block; you've just told the compiler you'll maintain them yourself instead of having them checked. This is a promissory note, not a permission slip.
- **The type system is for making illegal states unrepresentable, not just for catching typos.** `Option<T>` instead of a nullable pointer, an enum instead of a struct with three mutually-exclusive optional fields, a newtype instead of a bare `u64` — each of these deletes a category of bug by making the invalid state impossible to construct, which is strictly stronger than checking for it at runtime.

## Practices in Depth

### Practice: Model state with enums, not booleans and optional fields

- **Guideline:** When a type has fields that are only meaningful in combination (`is_connected: bool` + `socket: Option<TcpStream>` + `last_error: Option<Error>`), replace the flags with an enum whose variants each carry exactly the data valid in that state.
- **Why (what it prevents / enables):** A struct with three independent optional fields has up to 2³ representable combinations but usually far fewer *valid* ones — `is_connected: true, socket: None` is a state your code must remember never to construct and never to mishandle, forever, at every call site. An enum has exactly as many states as are valid, and the compiler forces every `match` to handle all of them — a new variant added later causes a compile error everywhere it isn't handled, which is the illegal-states-unrepresentable principle doing real work during a refactor six months from now.
- **Example:**

```rust
// Before: which combinations are even valid? The type doesn't say.
struct Connection {
    is_connected: bool,
    socket: Option<TcpStream>,
    last_error: Option<Error>,
}

// After: every state is exactly what it says, nothing more is representable.
enum Connection {
    Disconnected,
    Connecting { started_at: Instant },
    Connected { socket: TcpStream },
    Failed { error: Error },
}
```

- **Counter-case:** When variants share most of their data and differ in one field, an enum forces that shared data to be repeated per variant (or hoisted into a wrapper struct: `struct Conn { common: Meta, state: State }` — usually the right shape). And enums with a large variant tax every instance with its size — [box the fat variant](../../performance-optimization/memory-layout/learning.md) when the profile says so.

### Practice: Design errors as data, not strings

- **Guideline:** Library/domain code returns a specific `enum` error type (via `thiserror`) whose variants a caller can `match` on and recover from; only the outermost application layer collapses heterogeneous errors into an opaque, context-carrying type (`anyhow::Error`) for logging/reporting.
- **Why (what it prevents / enables):** A `String` or `Box<dyn Error>` error tells the caller nothing they can act on programmatically — recovery logic degenerates into string matching (fragile) or blanket catch-all handling (loses information). A typed enum lets calling code branch on `Err(FetchError::NotFound(..))` vs. `Err(FetchError::Timeout(..))` and make different decisions — retry one, surface the other — which is the entire point of `Result` over exceptions: the error is part of the type, not an out-of-band control-flow escape.
- **Example:**

```rust
#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("request timed out after {0:?}")]
    Timeout(Duration),
    #[error("transport failure")]
    Transport(#[from] reqwest::Error),   // `?` converts reqwest errors automatically
}

fn fetch(url: &str) -> Result<Response, FetchError> { /* ... */ }

// The caller makes different decisions per variant — the whole point of typed errors:
fn load(url: &str) -> Result<Response, FetchError> {
    match fetch(url) {
        Ok(response) => Ok(response),
        Err(FetchError::Timeout(_)) => fetch(url),   // transient: one retry
        Err(other) => Err(other),                    // NotFound/Transport propagate
    }
}
```

- **Counter-case:** In application binaries where nothing recovers programmatically — a CLI that prints the error and exits — a typed enum per module is ceremony; `anyhow::Result` with `.context("reading config")` at each layer produces better diagnostics for less code. The rule is about *who consumes the error*, not about crate type per se.

### Practice: Parse, don't validate — encode invariants in the type

- **Guideline:** Instead of a function that checks a precondition and returns `bool`/panics, write a constructor that *consumes* the raw input and returns a type that can only exist if the invariant holds — then every later use of that type is guaranteed valid, with no re-checking.
- **Why (what it prevents / enables):** "Validate then use" separates the proof from the use by an arbitrary amount of code, and nothing stops a later refactor from using the unvalidated value by mistake. "Parse into a type" fuses the proof to the value: a `NonEmptyString` or `Email` type can only be constructed via a fallible parse, so every function that takes one gets the guarantee for free, permanently, checked by the compiler rather than remembered by the programmer. This is the newtype pattern doing double duty — [zero runtime cost](../../performance-optimization/memory-layout/learning.md) via `#[repr(transparent)]`, and a genuine correctness upgrade.
- **Example:**

```rust
pub struct Email(String);   // private field: the only way in is `parse`

impl Email {
    pub fn parse(raw: String) -> Result<Self, EmailError> {
        if raw.contains('@') { Ok(Self(raw)) } else { Err(EmailError::Invalid(raw)) }
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

fn send_welcome(to: &Email) { /* no re-validation — an Email can't exist otherwise */ }
```

- **Counter-case:** Newtype-everything is its own anti-pattern. Each wrapper costs conversion boilerplate at boundaries, `From`/`Display`/`Serialize` impls, and orphan-rule friction when a third-party trait needs implementing for it. Wrap values that carry a **domain invariant or a confusable identity** (`Email`, `AccountId`, `Cents`); don't wrap a `u32` that is simply a count. And resist blanket `Deref` to the inner type to dodge the boilerplate — it re-exposes the raw value everywhere and quietly undoes the encapsulation.

### Practice: Accept the most general borrowed input that's still honest; return concrete owned output

- **Guideline:** Take `&str` not `&String`, `&[T]` not `&Vec<T>` — these are unconditionally right, since deref coercion means callers with either type pass them for free. Generic parameters (`impl AsRef<Path>`, `impl IntoIterator<Item = T>`) are a *considered* choice, not a default. Return concrete owned values, or `impl Trait` in return position when the type is an implementation detail.
- **Why (what it prevents / enables):** A parameter of `&String` forces every caller holding a `&str` (a literal, a slice of a larger string) to allocate purely to satisfy the signature. `&str` accepts both. This is the API-design half of the [zero-copy doctrine](../../performance-optimization/zero-copy/learning.md), and it's a usability property independent of performance: the more general the accepted type, the fewer `.to_string()`/`.to_vec()` calls exist in callers only to appease the compiler.
- **Example:**

```rust
// Forces an allocation from every caller holding a &str:
fn greet(name: &String) -> String { format!("Hello, {name}") }

// Accepts both &String and &str via deref coercion — strictly better, no trade-off:
fn greet(name: &str) -> String { format!("Hello, {name}") }
```

- **Counter-case (the part usually stated too flatly):** `impl AsRef<Path>` / `impl Into<String>` / `impl IntoIterator` parameters have real costs — worse error messages when a caller passes the wrong thing, type-inference failures at call sites, no turbofish (`impl Trait` in argument position forbids explicit type arguments), and a fresh monomorphized copy per caller type ([code size and compile time](../../performance-optimization/compiler-optimizations/learning.md)). Use them where the ergonomic win is real and repeated (a path-taking constructor called with literals, `PathBuf`s, and `&Path`s alike); prefer plain `&Path`/`&str` when there's one obvious caller shape.

### Practice: Move data between threads rather than sharing it — and read `Send`/`Sync` as the compiler telling you which you're doing

- **Guideline:** Structure concurrent code as **ownership transfer** first: channels (`std::sync::mpsc`, `crossbeam-channel`, `tokio::sync::mpsc`) carrying owned values, or an actor-shaped task that *owns* a piece of state and receives commands. Reach for `Arc<Mutex<T>>` when several threads genuinely need the same mutable thing, and keep the critical section as small as the work allows. Use `std::thread::scope` when worker threads need to borrow non-`'static` local data instead of forcing `Arc` clones for lifetime reasons alone.
- **Why (what it prevents / enables):** `Send` ("safe to move to another thread") and `Sync` ("`&T` is safe to share across threads") are auto traits — the compiler derives them structurally, so a type is thread-safe exactly when its parts are, with no annotation and no trust. That makes them a *design readout*: if a type isn't `Send`, something inside it (an `Rc`'s non-atomic refcount, a raw pointer) is telling you the design assumes single-thread residency. Message passing sidesteps the whole shared-mutability question — one owner at a time, transferred explicitly — which is why it composes better than locks as systems grow, and why the "one task owns the state, everyone else sends it messages" shape is the highest-leverage concurrency idiom in Rust.
- **Example:**

```rust
// Shared-state version: every caller contends on one lock, and lock discipline
// (ordering, hold time, poisoning) becomes everyone's problem.
let counts = Arc::new(Mutex::new(HashMap::new()));

// Ownership-transfer version: one task owns the map; others send owned messages.
let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(1024);   // bounded = backpressure
tokio::spawn(async move {
    let mut counts = HashMap::new();          // owned outright — no lock, no Arc, no Sync bound
    while let Some(event) = rx.recv().await {
        *counts.entry(event.key).or_insert(0) += 1;
    }
});

// Borrowing workers without Arc, when the data outlives the threads:
std::thread::scope(|s| {
    for chunk in data.chunks(1024) {
        s.spawn(move || process(chunk));      // borrows `data` — no 'static requirement
    }
});
```

- **Counter-case:** Message passing costs a channel hop and serializes work through one consumer; for a small read-mostly value shared by many readers, `Arc<RwLock<T>>` (or [`arc-swap`](../../performance-optimization/lock-free-concurrency/learning.md) for read-mostly-reload) is simpler and faster than routing reads through an owner task. And bounded channels are a design decision — an unbounded channel converts overload into memory exhaustion ([backpressure](../../architecture-patterns/backpressure-and-rate-limiting/learning.md)).

### Practice: Make illegal states hard to reach across module boundaries, not just within one function

- **Guideline:** Keep struct fields private by default; expose invariant-preserving methods instead of public fields, and use the module system (`pub(crate)`, private fields with public constructors/accessors) so a type's invariants can only be violated from inside the module that owns them.
- **Why (what it prevents / enables):** A `pub` field is a promise that *any* value of that field's type is valid forever, to every caller — which forecloses ever adding an invariant later without a breaking-change hunt. Private fields plus a smart constructor mean the module boundary is the only place invariants are checked, and it can be re-verified in one place when the type changes. This is encapsulation, but Rust gives it teeth: there's no reflection or "cast around it" escape hatch.
- **Example:**

```rust
pub struct Percentage(f64);  // private field

impl Percentage {
    pub fn new(v: f64) -> Result<Self, OutOfRange> {
        if (0.0..=100.0).contains(&v) { Ok(Self(v)) } else { Err(OutOfRange) }
    }
    pub fn value(&self) -> f64 { self.0 }
}
// no code outside this module can construct an invalid Percentage
```

### Practice: Prefer restructuring ownership over `Rc<RefCell<T>>` — shared mutability is the fallback, not the reflex

- **Guideline:** When the borrow checker resists, try in order: (1) does one thing actually own this — can the method take `&mut self` instead of `&self`? (2) can `&mut` be threaded through the call stack instead of stored as a shared handle? (3) can the type be split so independently-mutated parts are genuinely separate? `Rc<RefCell<T>>` (or `Arc<Mutex<T>>` across threads) comes after those.
- **Why (what it prevents / enables):** `RefCell` moves borrow checking from compile time to runtime — `.borrow_mut()` *panics* on a conflicting borrow instead of failing to compile, converting a category of bug from "impossible to ship" back to "shippable, now as a production panic." It's correct for genuinely graph-shaped shared ownership (widget trees, observer registries), but reaching for it at the first complaint trades the language's strongest guarantee for convenience.
- **Example — the same program, before and after:**

```rust
// Before: `&self` methods forced interior mutability, so borrows are runtime-checked.
struct Server { cache: Rc<RefCell<HashMap<Key, Value>>> }
impl Server {
    fn lookup(&self, k: Key) -> Value {
        let mut cache = self.cache.borrow_mut();      // panics if already borrowed elsewhere
        cache.entry(k).or_insert_with(compute).clone()
    }
}

// After: the honest signature is `&mut self` — checked at compile time, no Rc, no panic path.
struct Server { cache: HashMap<Key, Value> }
impl Server {
    fn lookup(&mut self, k: Key) -> &Value {
        self.cache.entry(k).or_insert_with(compute)
    }
}
```

### Practice: Design traits around behavior, not around inheritance you wish you had

- **Guideline:** A trait should describe a *capability* a type has (`Drawable`, `Serialize`, `Iterator`) with a small, coherent method set — not an attempt to recreate a class hierarchy. Prefer composing several focused traits (with default methods for shared behavior) over one large trait every implementor must satisfy in full.
- **Why (what it prevents / enables):** Rust has no implementation inheritance, and treating traits as a substitute produces a "god trait" most types stub out half of with `unimplemented!()`. Small traits compose (`T: Read + Write`), can be implemented piecemeal, and let a type opt into exactly the capabilities it has — the standard library's `Iterator`/`Read`/`Write`/`Display` split is the reference example.
- **Example:**

```rust
// God trait: every shape must fake-implement volume, even 2D ones.
trait Shape { fn area(&self) -> f64; fn volume(&self) -> f64; fn perimeter(&self) -> f64; }

// Composable: opt into exactly what applies.
trait Area { fn area(&self) -> f64; }
trait Volume { fn volume(&self) -> f64; }
impl Area for Circle { fn area(&self) -> f64 { /* ... */ } }
// Sphere implements both; Circle implements only Area — no fake stub methods.
```

- **Counter-case:** Over-splitting is also real — a trait per method produces call sites with five-bound `where` clauses and no coherent concept. The unit is a *capability someone would ask for by name*, not a method.

### Practice: Write documentation that compiles — `///` examples are tests

- **Guideline:** Public items get `///` doc comments, and the doc comment's code block is a **runnable example** — `cargo test` compiles and executes it. Document panics, errors, and safety obligations in the conventional `# Panics` / `# Errors` / `# Safety` sections. For crates meant to stay honest, `#![warn(missing_docs)]` at the crate root.
- **Why (what it prevents / enables):** Doc examples that run as tests are documentation that *cannot rot* — the API change that invalidates the example breaks the build, which is a guarantee no other mainstream language's docs offer by default. They also double as usability review: an example that's awkward to write is an API that's awkward to use, discovered before publication rather than after.
- **Example:**

````rust
/// Parses a percentage from a float.
///
/// # Errors
/// Returns [`OutOfRange`] if `v` is outside `0.0..=100.0`.
///
/// ```
/// # use mycrate::Percentage;
/// let p = Percentage::new(42.0)?;
/// assert_eq!(p.value(), 42.0);
/// assert!(Percentage::new(150.0).is_err());
/// # Ok::<(), mycrate::OutOfRange>(())
/// ```
pub fn new(v: f64) -> Result<Self, OutOfRange> { /* ... */ }
````

### Practice: Derive the common traits eagerly; state evolution intent with `#[non_exhaustive]` and `#[must_use]`

- **Guideline:** Derive `Debug` on essentially every public type, plus `Clone`, `PartialEq`, `Eq`, `Hash`, `Default`, `PartialOrd`/`Ord`, and `Copy` wherever semantically honest ([Rust API Guidelines C-COMMON-TRAITS](https://rust-lang.github.io/api-guidelines/)). Mark public enums/structs you expect to extend with `#[non_exhaustive]`, and mark `#[must_use]` on types and functions whose result being ignored is almost certainly a bug.
- **Why (what it prevents / enables):** A public type missing `Debug` poisons everything downstream — every containing struct's `#[derive(Debug)]` fails, every `dbg!` and error message loses the field, and callers can't do anything about it because of the orphan rule. It's a small omission with a wide blast radius. `#[non_exhaustive]` is the schema-evolution discipline from the [event-sourcing doc](../../architecture-patterns/event-sourcing/learning.md) expressed as a language feature: downstream `match`es must include a wildcard arm, so adding a variant later is a non-breaking change instead of an ecosystem-wide break. `#[must_use]` converts a silently-dropped `Result` or ignored builder into a warning.
- **Example:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]                 // adding a variant later won't break downstream matches
pub enum Status { Pending, Active, Closed }

#[must_use = "a Builder does nothing unless `.build()` is called"]
pub struct Builder { /* ... */ }
```

### Practice: Test the public contract with `#[cfg(test)] mod tests`, and reach for property tests where the domain has invariants

- **Guideline:** Unit tests live in a `tests` submodule beside the code they test (compiled only under `cfg(test)`, zero release cost); integration tests that exercise the crate as a library user does go in `tests/`. For code with algebraic properties (round-trip serialization, sort invariants, parse-then-encode identity), add `proptest`/`quickcheck` cases alongside example-based tests.
- **Why (what it prevents / enables):** Colocated unit tests can reach private helpers while costing nothing in the shipped binary; `tests/` integration tests catch the "compiles inside the crate but the public API is unusable" class of bug that unit tests structurally cannot see. Property tests find the input nobody thought to write and *shrink* failures to a minimal reproduction — a different bug-finding mechanism than example-based tests, not a fancier version of one.
- **Example:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_out_of_range() { assert!(Percentage::new(150.0).is_err()); }
}

// With proptest — requires an `Arbitrary` impl or a custom strategy for MyStruct:
proptest! {
    #[test]
    fn encode_decode_roundtrip(v in any::<MyStruct>()) {
        prop_assert_eq!(decode(&encode(&v)), v);
    }
}
```

## Anti-Patterns in Depth

### Anti-Pattern: `.clone()` as borrow-checker duct tape

- **What goes wrong:** Every "cannot borrow `self` as mutable because it is also borrowed as immutable" gets silenced with `.clone()`. The code compiles, but now carries copies that *drift* — two clones of "the same" data mutated independently become two different values, with no error and no warning, surfacing later as inconsistent behavior. At scale it's also a straightforward [allocation](../../performance-optimization/allocation-strategies/learning.md) cost multiplied across every hot path.
- **Why it's tempting:** It's a one-token fix that makes the error disappear *now* and requires zero understanding of why the borrow checker objected — which is exactly the danger: the design question (who owns this?) goes unanswered and returns later as a data-consistency bug with no compiler error pointing at it.
- **Bad → good example — the shape people actually hit:**

```rust
// Bad: the classic self-borrow conflict, "fixed" by cloning the collection.
impl Processor {
    fn run(&mut self) {
        for item in self.items.clone() {      // clone to release the borrow on self
            self.record(&item);               // needs &mut self
        }
    }
}
```

The real fixes, in the order to try them:

```rust
// 1. Split borrows: destructure so the compiler sees disjoint *fields*.
//    (It understands field-level disjointness inside a body — but not across a
//     method call, since `&mut self` borrows the whole struct.)
impl Processor {
    fn run(&mut self) {
        let Self { items, log, .. } = self;           // two independent borrows
        for item in items.iter() {
            Self::record_into(log, item);             // takes only what it needs
        }
    }
    fn record_into(log: &mut Vec<Entry>, item: &Item) { log.push(Entry::from(item)); }
}

// 2. `mem::take`: own the collection briefly, put it back — no allocation, no drift.
impl Processor {
    fn run(&mut self) {
        let items = std::mem::take(&mut self.items);  // self.items is now empty
        for item in &items { self.record(item); }     // self is free to borrow mutably
        self.items = items;
    }
}

// 3. Index loop: each iteration re-borrows briefly, when the collection is indexable
//    and `record` doesn't need to hold a reference across the call.
for i in 0..self.items.len() { self.record_at(i); }
```

### Anti-Pattern: Holding a lock (or a `RefCell` borrow) across an `.await` or a long call

- **What goes wrong:** A `MutexGuard` held across `.await` travels with the future when the task is parked, so the lock stays held while the task waits on I/O — throughput collapses, and with `std::sync::Mutex` on a multithreaded runtime it deadlocks outright when another task on the same worker needs it. The `RefCell` analogue: a `borrow_mut()` held across a call that re-enters the same object panics at runtime. Both are the correctness face of the [async doc's](../../performance-optimization/async-and-io/learning.md) blocking-the-runtime hazard.
- **Why it's tempting:** The scope-based guard is exactly what makes locks pleasant in synchronous Rust — `let guard = m.lock()` and forget about it — and that reflex carries into async code where the guard's *lifetime* now spans an arbitrary suspension.
- **Bad → good example:**

```rust
// Bad: the guard lives across the await point.
let mut state = self.state.lock().unwrap();
let response = fetch(&state.url).await;      // lock held for the whole network round trip
state.last = response;

// Good: take what's needed, drop the guard, re-acquire to write.
let url = { self.state.lock().unwrap().url.clone() };   // guard dropped at the brace
let response = fetch(&url).await;
self.state.lock().unwrap().last = response;
// If a lock genuinely must span an await, use tokio::sync::Mutex — it's designed for it
// and costs more; better still, give the state to an owner task (see the concurrency practice).
```

### Anti-Pattern: `unwrap()`/`expect()` on paths that are not actually infallible

- **What goes wrong:** `.unwrap()` on a `Result`/`Option` that can legitimately fail in production (a network call, a user-supplied index, a map lookup assuming a key exists) turns a recoverable error into a panic. In a server that's an availability bug shipped by one method call.
- **Why it's tempting:** It's the fastest way to get prototyping code compiling, and it *is* correct where the value provably cannot be `None`/`Err` — the anti-pattern is failing to distinguish "provably infallible" from "I don't want to write error handling right now" before shipping. (Note the poisoning special case: `std::sync::Mutex::lock` returns `Result` only because a panic while holding it poisons it; `parking_lot::Mutex` doesn't poison and returns the guard directly, removing the question.)
- **Bad → good example:**

```rust
// Bad: this WILL be None sometimes, in production, for real users.
let user = users.get(&id).unwrap();

// Good: propagate or handle explicitly.
let user = users.get(&id).ok_or(Error::UserNotFound(id))?;

// Fine, because it's provably infallible and the message says why:
let re = Regex::new(r"^\d+$").expect("hardcoded regex is valid");
```

### Anti-Pattern: Primitive obsession — bare `String`/`u64`/`bool` where the domain has a type

- **What goes wrong:** Signatures fill up with `fn transfer(from: u64, to: u64, amount: u64, currency: String)` — nothing stops a caller swapping `from`/`to`, or passing cents where dollars were meant. The compiler, which could catch every one of these at the call site, is disabled because everything is the same underlying type.
- **Why it's tempting:** Primitives need no definitions and work with every stdlib function; defining `AccountId(u64)` feels like ceremony for "just a number" — until the first transposed-argument bug reaches production.
- **Bad → good example:**

```rust
// Bad: four same-shaped parameters, all interchangeable to the compiler.
fn transfer(from: u64, to: u64, amount: u64, currency: String) { /* ... */ }

// Good: the compiler now rejects transposed arguments at the call site.
struct AccountId(u64);
struct Cents(u64);
enum Currency { Usd, Eur }
fn transfer(from: AccountId, to: AccountId, amount: Cents, currency: Currency) { /* ... */ }
```

(See the newtype counter-case above — this principle has a ceiling, and `Deref`-to-inner is how people accidentally undo it.)

### Anti-Pattern: Stringly-typed state and control flow

- **What goes wrong:** Status fields and dispatch keys as `String`/`&str` (`status == "pending"`, `match kind.as_str() { "create" => ... }`) instead of enums. Typos compile and silently never match; the compiler can't flag an unhandled case when a new status appears; and every comparison is a string compare instead of an integer one — the runtime echo of the [branch-prediction](../../performance-optimization/branch-prediction/learning.md) and locality costs the perf docs describe.
- **Why it's tempting:** Strings arrive that way from JSON, config, and database columns, so converting at the boundary feels like an extra step — until the fifth site does its own ad hoc comparison against a slightly different spelling.
- **Bad → good example:**

```rust
// Bad: "pendign" typo'd anywhere silently fails to match, forever.
if status == "pending" { /* ... */ }

// Good: parse at the boundary, exhaustive-match everywhere after.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Status { Pending, Active, Closed }
impl FromStr for Status { /* parse once, at the edge */ }

match status {
    Status::Pending => { /* ... */ }
    Status::Active  => { /* ... */ }
    Status::Closed  => { /* ... */ }
    // adding a variant without handling it here is a compile error
}
```

### Anti-Pattern: Generic-izing before there are two call sites

- **What goes wrong:** A function is written `fn process<T: Serialize + Clone + Send + 'static>(item: T)` at the first call site, speculating about flexibility. Bounds accrete to satisfy whatever the body touches, `where` clauses proliferate, compile times climb (a monomorphized copy per instantiation — [the compiler doc's](../../performance-optimization/compiler-optimizations/learning.md) cost made concrete), and the abstraction usually doesn't fit the *second* real caller anyway.
- **Why it's tempting:** Generics feel like "doing it right," and Rust's are zero-cost at runtime — which removes the usual performance argument against premature abstraction and leaves only the easily-ignored complexity one.
- **Bad → good example:**

```rust
// Bad, at the first call site, with no second caller yet:
fn process<T: Serialize + Clone + Send + Sync + 'static>(item: T) -> Result<(), Error> { /* ... */ }

// Good: concrete until a second, real use case reveals the shared shape.
fn process(item: &Order) -> Result<(), Error> { /* ... */ }
// generalize later, to the bounds the second call site actually needs
```

### Anti-Pattern: Fighting the borrow checker with `unsafe` instead of restructuring

- **What goes wrong:** A design with ambiguous or cyclic ownership gets "fixed" with raw pointers and `unsafe { &mut *ptr }` / `unsafe impl Send`, reintroducing exactly the bug class (dangling references, aliased mutation, data races) the borrow checker exists to prevent — now unchecked and undocumented.
- **Why it's tempting:** `unsafe` does make the compiler stop complaining, and to someone who hasn't yet internalized that a borrow error is *information about the design*, it looks like any other stubborn error to work around.
- **Bad → good example:**

```rust
// Bad: silences the error, reintroduces the exact bug class Rust prevents.
let ptr: *mut Node = &mut *self.head as *mut _;
unsafe { (*ptr).next = Some(new_node); }

// Good: an idiom built for graph-shaped data — indices instead of pointers.
struct Nodes { arena: Vec<Node>, head: Option<usize> }   // no lifetimes to fight
// or: Rc<RefCell<Node>> if the sharing is genuine, chosen deliberately (see above)
```

If `unsafe` really is required (FFI, a proven-hot data structure), the obligations are: a `// SAFETY:` comment stating the invariant *and why it holds*, the smallest possible `unsafe` block, a safe wrapper API, and [`cargo miri test`](https://github.com/rust-lang/miri) in CI. Clippy's `undocumented_unsafe_blocks` lint enforces the first one.

## Exercises / Things to Try

Self-test — answer from the model, then check against the doc:

1. What three things does a function signature tell you in Rust that most languages leave ambiguous? Why does that make signatures readable as a data-flow story?
2. A struct has `is_connected: bool`, `socket: Option<TcpStream>`, `last_error: Option<Error>`. How many states are representable, roughly how many are valid, and what does converting to an enum buy at *refactor* time specifically?
3. Distinguish `Send` from `Sync` precisely. Why is `Rc<T>` neither, and what does a "not `Send`" diagnostic tell you about a design?
4. You hit "cannot borrow `self` as mutable because it is also borrowed as immutable" in a loop. Give three fixes in the order you'd try them, and say what the borrow checker understands about field disjointness that it doesn't understand across a method call.
5. Why does holding a `std::sync::Mutex` guard across an `.await` deadlock rather than merely slow things down? What are the two correct responses?
6. When is "parse, don't validate" the wrong call — what does newtype-everything cost, and which convenience move quietly undoes the encapsulation?
7. Why is a missing `#[derive(Debug)]` on a public type a wide-blast-radius omission rather than a local inconvenience? What does the orphan rule have to do with it?
8. What does `#[non_exhaustive]` buy, and which architecture-side discipline is it the language-level form of?

Katas — do these in real code:

1. **The clone hunt.** `grep -n '\.clone()'` a project of yours; for each hot-path clone flagged by [dhat](../../performance-optimization/profiling-and-measurement/learning.md), decide: borrow-checker appeasement or genuine need? Fix the former with split borrows or `mem::take`; document the latter as deliberate.
2. **Enum-ify a flag struct.** Convert a 2+ `bool`/`Option` struct to an enum. Note which `match` arms the compiler now forces you to handle that the old code silently mishandled.
3. **Parse-don't-validate one boundary.** Convert a `if !valid(x) { return Err(..) }` + later-raw-use into a smart constructor; delete the downstream re-checks.
4. **Add a doc test to a public function** and break it deliberately (change the signature) to watch `cargo test` catch stale documentation.
5. **Convert one `Arc<Mutex<T>>` to an owner task + channel.** Compare the two for lock-discipline complexity, not just speed.
6. **Write one property test** for a round-trip; let it find an edge case, then add that case as a named regression test.
7. **Read one `unsafe` block** (yours or a dependency's) and write its `// SAFETY:` argument. Failing to write one convincingly is a finding, not a failed exercise.

## Open Questions

- Where exactly does "generic enough" stop and "premature abstraction" start — a better heuristic than "wait for the second call site," or is it too domain-dependent to generalize?
- `thiserror` vs. hand-rolled `impl Error` vs. `snafu`: current ecosystem consensus, and what changes the recommendation between a library crate and an application binary.
- Async trait ergonomics post-stabilization (native `async fn` in traits): what changes in the trait-design guidance above, and where does dyn-compatibility still push toward `async-trait`?
- How much does the "prefer borrowing" guidance shift as GATs and richer lifetime patterns become common in application code rather than library internals?
- Workspace/crate-splitting heuristics: when does a growing binary's compile time justify splitting, and how does that interact with the monomorphization costs in the compiler-optimizations doc?
- Which clippy lint groups (`pedantic`, `nursery`) are worth enabling by default versus cherry-picking — and which of this doc's anti-patterns already have a lint that catches them?

## References

- *The Rust Programming Language* (the Book) — the canonical ownership/borrowing explanation; read the ownership chapter twice, once before writing real Rust and once after a few weeks of borrow-checker fights, since the second read lands completely differently.
- Jon Gjengset, *Rust for Rustaceans* — the best intermediate-to-advanced book once the Book is comfortable; strong on API design, trait design, and the "why" behind idioms this doc compresses.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — the community checklist for public crate design (naming, common traits to derive, error conventions); the source for this doc's derive-hygiene practice.
- Alexis King, ["Parse, don't validate"](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/) — not Rust-specific, but the clearest statement of the practice and the origin of the phrase.
- Alice Ryhl, ["Actors with Tokio"](https://ryhl.io/blog/actors-with-tokio/) — the ownership-transfer concurrency shape in full, and the best argument for it over shared state.
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/) — a community catalog of idioms and anti-patterns; a useful second opinion against this doc's specific choices.
- Related topics in this repo: [Data-Oriented Design](../../performance-optimization/data-oriented-design/learning.md) (ownership clarity and cache-friendly layout share a root cause), [Allocation Strategies](../../performance-optimization/allocation-strategies/learning.md) (the runtime cost of clone-as-duct-tape), [Async & I/O](../../performance-optimization/async-and-io/learning.md) (the runtime-blocking hazard behind the lock-across-await anti-pattern), [Lock-Free Concurrency](../../performance-optimization/lock-free-concurrency/learning.md) (what `unsafe` shared mutability must prove), [Compiler Optimizations](../../performance-optimization/compiler-optimizations/learning.md) (why premature generics cost measurably), [Event Sourcing](../../architecture-patterns/event-sourcing/learning.md) (`#[non_exhaustive]` as its additive-evolution discipline in language form).
