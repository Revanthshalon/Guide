# Rust — Learning Notes

## Mental Model

**Ownership is not a memory-safety bolt-on — it's a type system for describing who's responsible for what, and the compiler is checking your architecture, not just your pointers.** Every other language lets you write "this function takes a `Thing`" without saying whether the function *keeps* it, *borrows* it briefly, or *shares* it with others — that ambiguity is resolved at runtime (GC) or not at all (C's dangling pointers). Rust makes you say which, in the signature, and then enforces it. The payoff is not "no segfaults" — it's that **the function signature is a complete contract**: `fn f(x: T)` takes ownership (the caller loses access); `fn f(x: &T)` borrows read-only (caller keeps it, many readers coexist); `fn f(x: &mut T)` borrows exclusively (caller keeps it, but nobody else may touch it meanwhile). Reading a signature tells you the data-flow story without reading the body — a property most languages can't offer at any price.

The corollary that reorganizes how you think about every design problem: **most "how do I structure this?" questions are actually "who owns this, and for how long?" questions.** A graph of objects with pointers back and forth is not merely awkward in Rust — the compiler is telling you the ownership story is genuinely ambiguous, and it's making you resolve the ambiguity *before* the bug ships instead of after. The idiomatic responses — arenas with index-based references, single-owner trees with explicit weak back-links, message passing instead of shared mutable state — aren't workarounds for the borrow checker; they're what the design looks like once the ownership question has an honest answer. This is the same lesson the [performance-optimization](../../performance-optimization/data-oriented-design/learning.md) docs teach from the opposite direction (index arenas, SoA) — Rust's type system and Rust's performance idioms point at the same designs because ownership clarity and cache-friendly layout have the same enemy: implicit, tangled, many-owner data.

Three consequences worth internalizing up front, because they explain a lot of what follows:

- **The compiler is a design-review partner, not an obstacle.** A fight with the borrow checker is almost always the compiler correctly reporting that your mental model of the data's lifetime doesn't match what the code does. The fix is rarely a workaround (`.clone()`, `Rc<RefCell<>>>`) — it's usually a clearer design that was available all along.
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

### Practice: Design errors as data, not strings

- **Guideline:** Library/domain code returns a specific `enum` error type (via `thiserror`) whose variants a caller can `match` on and recover from; only the outermost application layer collapses heterogeneous errors into an opaque, context-carrying type (`anyhow::Error`) for logging/reporting.
- **Why (what it prevents / enables):** A `String` or `Box<dyn Error>` error tells the caller nothing they can act on programmatically — recovery logic degenerates into string matching (fragile) or blanket catch-all handling (loses information). A typed enum lets calling code branch on `Err(FetchError::NotFound)` vs. `Err(FetchError::Timeout)` and make different decisions — retry one, surface the other — which is the entire point of `Result` over exceptions: the error is part of the type, not an out-of-band control-flow escape.
- **Example:**

```rust
#[derive(thiserror::Error, Debug)]
enum FetchError {
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("request timed out after {0:?}")]
    Timeout(Duration),
    #[error("transport error")]
    Transport(#[from] reqwest::Error),
}

fn fetch(url: &str) -> Result<Response, FetchError> { /* ... */ }

// caller can actually decide:
match fetch(url) {
    Err(FetchError::Timeout(_)) => retry(),
    Err(FetchError::NotFound(_)) => return Err(NotFoundInCatalog),
    Err(e) => return Err(e.into()),   // anyhow at the app boundary
    Ok(r) => handle(r),
}
```

### Practice: Parse, don't validate — encode invariants in the type

- **Guideline:** Instead of a function that checks a precondition and returns `bool`/panics, write a constructor that *consumes* the raw input and returns a type that can only exist if the invariant holds — then every later use of that type is guaranteed valid, with no re-checking.
- **Why (what it prevents / enables):** "Validate then use" separates the proof from the use by an arbitrary amount of code, and nothing stops a later refactor from using the unvalidated value by mistake. "Parse into a type" fuses the proof to the value: a `NonEmptyString` or `Email` type can only be constructed via a fallible parse, so every function that takes one gets the guarantee for free, permanently, checked by the compiler rather than remembered by the programmer. This is the newtype pattern doing double duty — [zero runtime cost](../../performance-optimization/memory-layout/learning.md) via `#[repr(transparent)]`, and a genuine correctness upgrade.
- **Example:**

```rust
struct Email(String);

impl Email {
    fn parse(raw: String) -> Result<Self, EmailError> {
        if raw.contains('@') { Ok(Self(raw)) } else { Err(EmailError::Invalid(raw)) }
    }
}

fn send_welcome(to: &Email) { /* no need to re-validate — Email can't exist otherwise */ }
```

### Practice: Accept borrowed, generic input; return owned, concrete output

- **Guideline:** Function parameters should be the most general borrowed form the caller might have (`&str` not `&String`, `&[T]` not `&Vec<T>`, `impl AsRef<Path>` for path-like arguments, `impl Iterator<Item = T>` over `Vec<T>` when only iteration is needed); return types should be concrete owned values (or `impl Trait` for opaque-but-concrete return position) unless the API genuinely needs to hand back a borrow.
- **Why (what it prevents / enables):** A parameter of `&String` forces every caller with a `&str` (a string literal, a slice of a larger string) to allocate just to satisfy the signature; `&str` accepts both for free via deref coercion. This is the API-design half of the [zero-copy doctrine](../../performance-optimization/zero-copy/learning.md) — but it's also a usability property independent of performance: the more general the accepted type, the fewer places callers need `.to_string()`/`.to_vec()` calls that exist only to satisfy the compiler.
- **Example:**

```rust
// Forces an allocation from every caller holding a &str:
fn greet(name: &String) -> String { format!("Hello, {name}") }

// Accepts anything string-shaped, allocates only where genuinely needed:
fn greet(name: &str) -> String { format!("Hello, {name}") }
```

### Practice: Make illegal states hard to reach across module boundaries, not just within one function

- **Guideline:** Keep struct fields private by default; expose invariant-preserving methods instead of public fields, and use the module system (`pub(crate)`, private fields with public constructors/accessors) so a type's invariants can only be violated from inside the module that owns them.
- **Why (what it prevents / enables):** A `pub` field is a promise that *any* value of that field's type is valid forever, to every caller in the crate — which forecloses ever adding a new invariant later without a breaking change hunt. Private fields plus a smart constructor mean the module boundary is the only place invariants are checked, and it can be re-verified in one place when the type changes. This is encapsulation, but Rust gives it teeth: there's no reflection or "just cast around it" escape hatch the way there is in many other languages.
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

### Practice: Prefer borrowing and scoped lifetimes over `Rc<RefCell<T>>` — reach for shared mutability last, not first

- **Guideline:** When the borrow checker resists a design, first try: restructuring ownership (does one thing actually own this?), passing `&mut` through the call stack instead of storing a shared handle, or splitting the type so the parts that need independent mutation are genuinely independent. `Rc<RefCell<T>>` (or `Arc<Mutex<T>>` across threads) is the fallback once those are truly unavailable — not the first tool reached for.
- **Why (what it prevents / enables):** `RefCell` moves borrow checking from compile time to runtime (`.borrow_mut()` panics on a conflicting borrow instead of failing to compile) — which converts a category of bug from "impossible to ship" back to "possible to ship, now with a panic instead of a compile error." It's the right tool when genuine graph-shaped shared ownership is unavoidable (a GUI widget tree, an observer registry) — but reaching for it reflexively at the first borrow-checker complaint trades the language's strongest guarantee for convenience, usually to route around a design that a little restructuring would fix outright.
- **Example:**

```rust
// Reflexive reach-for-RefCell:
struct Cache { entries: Rc<RefCell<HashMap<K, V>>> }

// Often the actual fix is ownership restructuring:
struct Cache { entries: HashMap<K, V> }
impl Cache {
    fn get_or_insert(&mut self, k: K, f: impl FnOnce() -> V) -> &V { /* ... */ }
}
```

### Practice: Design traits around behavior, not around inheritance you wish you had

- **Guideline:** A trait should describe a *capability* a type has (`Drawable`, `Serialize`, `Iterator`) with a small, coherent method set — not an attempt to recreate a class hierarchy from another language. Prefer composing multiple small traits (with default methods for shared behavior) over one large trait every implementor must satisfy in full.
- **Why (what it prevents / enables):** Rust has no implementation inheritance, and treating traits as a substitute produces a "god trait" every type technically implements but most types stub out half the methods with `unimplemented!()`. Small, focused traits compose (`T: Read + Write`), can be implemented piecemeal, and let a type opt into exactly the capabilities it actually has — the standard library's own `Iterator`/`Read`/`Write`/`Display` split is the reference example: nobody who wants `Display` is forced to also implement `Iterator`.
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

### Practice: Test the public contract with `#[cfg(test)] mod tests`, and reach for property tests where the domain has invariants

- **Guideline:** Unit tests live in a `tests` submodule beside the code they test (compiled only under `cfg(test)`, zero release cost); integration tests that exercise the crate as a library user does go in `tests/`. For code with algebraic properties (round-trip serialization, sort invariants, "parsing then re-encoding is identity"), add `proptest`/`quickcheck` cases alongside example-based tests — they find the input you didn't think to write by hand.
- **Why (what it prevents / enables):** Colocated unit tests keep the test honest about what it's actually exercising (private helpers included) while costing nothing in the shipped binary; `tests/` integration tests catch the "it compiles inside the crate but the public API is unusable" class of bug that unit tests structurally can't see. Property tests catch the edge case three engineers didn't think of because they shrink failing inputs automatically to the minimal reproduction — a genuinely different bug-finding mechanism than example-based tests, not a fancier version of the same one.
- **Example:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_out_of_range() { assert!(Percentage::new(150.0).is_err()); }
}

// tests/roundtrip.rs (integration) or inline with proptest:
proptest! {
    #[test]
    fn encode_decode_roundtrip(v in any::<MyStruct>()) {
        prop_assert_eq!(decode(&encode(&v)), v);
    }
}
```

## Anti-Patterns in Depth

### Anti-Pattern: `.clone()` as borrow-checker duct tape

- **What goes wrong:** Every "cannot borrow as mutable because also borrowed as immutable" error gets silenced with `.clone()` at the call site. The code compiles, but now carries redundant copies that drift — two clones of "the same" data mutated independently become two different values with no error, no warning, just quietly wrong behavior somewhere downstream. At scale it's also a straightforward [allocation-strategies](../../performance-optimization/allocation-strategies/learning.md) cost multiplied across every hot path this pattern touches.
- **Why it's tempting:** It's a one-token fix that makes the red squiggle disappear *right now*, and unlike most compiler errors it requires zero understanding of why the borrow checker objected — which is exactly why it's dangerous: the underlying design question (who actually owns this?) goes unanswered and resurfaces later as a data-consistency bug that doesn't come with a compiler error pointing at it.
- **Bad → good example:**

```rust
// Bad: clone to dodge the borrow, now `items` and `snapshot` can drift.
let snapshot = items.clone();
process(&mut items);
report(&snapshot);

// Good: ask what actually needs to happen — usually, order the borrows.
report(&items);       // read first
process(&mut items);  // then mutate — no clone, no drift, borrow checker is satisfied
                       // because the borrows genuinely don't overlap in time.
```

### Anti-Pattern: `unwrap()`/`expect()` on paths that are not actually infallible

- **What goes wrong:** `.unwrap()` on a `Result`/`Option` that *can* legitimately fail in production (a network call, a user-supplied index, a `HashMap` lookup that assumes a key exists) turns a recoverable error into an unconditional process panic. In a server, one bad request panics the handling task (or the process, depending on the panic strategy) — an availability bug shipped by a single misplaced method call.
- **Why it's tempting:** `unwrap()` is the fastest way to get code compiling while prototyping, and it's genuinely correct in the cases where the value truly cannot be `None`/`Err` (a regex compiled from a string literal, a `Mutex` lock that's never poisoned by design) — the anti-pattern is failing to distinguish "provably infallible here" from "I don't want to write the error-handling code right now" before shipping.
- **Bad → good example:**

```rust
// Bad: this WILL be None sometimes, in production, for real users.
let user = users.get(&id).unwrap();

// Good: propagate or handle explicitly.
let user = users.get(&id).ok_or(Error::UserNotFound(id))?;

// Fine, because it's provably infallible and says so:
let re = Regex::new(r"^\d+$").expect("hardcoded regex is valid");
```

### Anti-Pattern: Primitive obsession — bare `String`/`u64`/`bool` where the domain has a type

- **What goes wrong:** Function signatures fill up with `fn transfer(from: u64, to: u64, amount: u64, currency: String)` — nothing stops a caller from swapping `from`/`to`, passing a currency code where an account id was expected, or passing cents where the function expected dollars. The compiler, which could catch every one of these at the call site, is disabled because everything is the same underlying type.
- **Why it's tempting:** Primitives require no new type definitions, no `From`/newtype boilerplate, and every stdlib function already works with them — the friction of defining `AccountId(u64)` feels like ceremony for what "is just a number" until the first `transfer(to, from, amount, currency)` argument-order bug ships to production.
- **Bad → good example:**

```rust
// Bad: five same-shaped parameters, all interchangeable to the compiler.
fn transfer(from: u64, to: u64, amount: u64, currency: String) { /* ... */ }

// Good: the compiler now rejects transposed arguments at the call site.
struct AccountId(u64);
struct Cents(u64);
enum Currency { Usd, Eur }
fn transfer(from: AccountId, to: AccountId, amount: Cents, currency: Currency) { /* ... */ }
```

### Anti-Pattern: Stringly-typed state and control flow

- **What goes wrong:** Status fields, event types, or dispatch keys represented as `String`/`&str` (`status: "pending"`, `match kind.as_str() { "create" => ..., "update" => ... }`) instead of enums. Typos compile (`"pendign"` silently never matches), the compiler can't warn about an unhandled case when a new status is added, and every comparison pays a string comparison instead of an integer one — the runtime cost mirrors the [branch-prediction](../../performance-optimization/branch-prediction/learning.md) and locality cost of the analogous CRUD anti-pattern at the architecture layer.
- **Why it's tempting:** Strings feel natural coming from JSON/config/database columns that are strings on the wire — converting at the boundary into an enum feels like an extra step, until the fifth place in the codebase does its own ad hoc string comparison against a slightly different spelling.
- **Bad → good example:**

```rust
// Bad: "pending" typo'd anywhere silently fails to match, forever.
if status == "pending" { /* ... */ }

// Good: parse at the boundary, exhaustive-match everywhere after.
#[derive(PartialEq)]
enum Status { Pending, Active, Closed }
impl FromStr for Status { /* parse once, at the edge */ }
match status {
    Status::Pending => { /* ... */ }
    Status::Active => { /* ... */ }
    Status::Closed => { /* ... */ }
    // compiler errors if a new variant is added and not handled here
}
```

### Anti-Pattern: Generic-izing before there are two call sites

- **What goes wrong:** A function is written `fn process<T: Serialize + Clone + Send + 'static>(item: T)` on the first call site, speculating about future flexibility. The generic bound set grows to satisfy whatever the implementation happens to touch, trait objects and `where` clauses proliferate, compile times climb (monomorphization per instantiation — the [compiler-optimizations](../../performance-optimization/compiler-optimizations/learning.md) cost made concrete), and the abstraction usually doesn't even fit the *second* real call site when one shows up, requiring a redesign anyway.
- **Why it's tempting:** Generics feel like "doing it right" — avoiding duplication before it exists — and Rust's generics are genuinely zero-cost at runtime, which removes the usual performance argument against premature abstraction and leaves only the (easy to ignore) complexity argument.
- **Bad → good example:**

```rust
// Bad, at the first call site, no second caller yet:
fn process<T: Serialize + Clone + Send + Sync + 'static>(item: T) -> Result<(), Error> { ... }

// Good: concrete until a second, real use case reveals the actual shared shape.
fn process(item: &Order) -> Result<(), Error> { ... }
// generalize later, from evidence, to the bounds the second call site actually needs
```

### Anti-Pattern: Fighting the borrow checker with `unsafe` instead of restructuring

- **What goes wrong:** A design that genuinely has ambiguous or cyclic ownership gets "fixed" by reaching for raw pointers and `unsafe impl Send`/`unsafe { &mut *ptr }` to route around the compiler's objection — reintroducing the exact class of bug (dangling references, aliased mutation, data races) the borrow checker exists to prevent, except now unchecked and undocumented.
- **Why it's tempting:** `unsafe` genuinely does make the compiler stop complaining, and for someone under deadline pressure who hasn't yet internalized that the borrow checker error is *information about the design*, it looks identical to any other stubborn compiler error that "just needs to be worked around."
- **Bad → good example:**

```rust
// Bad: silences the error, reintroduces the exact bug class Rust prevents.
let ptr: *mut Node = &mut *self.head as *mut _;
unsafe { (*ptr).next = Some(new_node); }

// Good: restructure with an idiom built for graph-shaped data.
struct Nodes { arena: Vec<Node>, head: Option<usize> }  // index-based, no lifetimes to fight
// or: Rc<RefCell<Node>> if genuinely shared, chosen deliberately (see the practice above)
```

## Exercises / Things to Try

1. **The clone hunt.** In a real project of yours, `grep -n '.clone()'` every hot-path clone found by [the profiling doc's](../../performance-optimization/profiling-and-measurement/learning.md) dhat pass, and for each one ask: was this satisfying the borrow checker, or genuinely needed? Fix the borrow-checker ones by reordering/restructuring; leave the genuine ones, now documented as deliberate.
2. **Enum-ify a boolean-flag struct.** Find a struct in your code with 2+ `bool`/`Option` fields whose validity depends on each other, and convert it to an enum with per-variant data. Notice which `match` arms the compiler now forces you to handle that the old code silently mishandled.
3. **Parse-don't-validate one boundary.** Take one function that currently does `if !valid(x) { return Err(...) }` followed later by uses of the still-raw `x`, and convert it to a smart constructor returning a wrapper type. Delete the now-redundant re-checks downstream.
4. **Write one property test.** Pick a function with a round-trip property (serialize/deserialize, encode/decode, sort-then-check-sorted) and add a `proptest` case. Let it find an edge case; add that case as a named regression test too.
5. **Trait-split a god trait.** Find (or write) a trait with 5+ methods where at least one implementor stubs several with `unimplemented!()` or a no-op default — split it into two or three focused traits and re-implement.
6. **Read one `unsafe` block you (or a dependency) wrote,** and write out the safety argument as a `// SAFETY:` comment if one isn't there. If you can't write the argument convincingly, that's a finding, not an exercise failure.

## Open Questions

- Where exactly does "generic enough" stop and "premature abstraction" start in practice — is there a good heuristic beyond "wait for the second call site," or does it vary too much by domain to generalize?
- `thiserror` vs. hand-rolled `impl std::error::Error` vs. `snafu`: current ecosystem consensus, if any, and what changes the recommendation for a library crate vs. an application binary.
- Async trait ergonomics post-stabilization (native `async fn` in traits): does this change the trait-design guidance above for async-heavy codebases, and where do the remaining rough edges (dyn-compatibility) still push toward `async-trait`?
- How much of the "prefer borrowing" guidance changes once GATs and more advanced lifetime patterns are common in application code rather than just library internals — worth a follow-up read once a concrete need arises.
- Workspace/crate-splitting heuristics: at what point does a growing binary crate's compile time justify splitting into a workspace, and does that interact with the monomorphization-cost material in the compiler-optimizations doc enough to change the answer?

## References

- *The Rust Programming Language* (the official book, "the Book") — the canonical ownership/borrowing explanation; read the ownership chapter twice, once before writing real Rust and once after a few weeks of borrow-checker fights, since the second read lands completely differently.
- Jon Gjengset, *Rust for Rustaceans* — the best intermediate-to-advanced book once the Book's material is comfortable; strong on API design, trait design, and the "why" behind idioms this doc compresses.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — the community checklist for public-facing crate design (naming, trait implementations to derive, error type conventions); worth applying to any crate meant for other people (including future you).
- Alexis King, ["Parse, don't validate"](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/) — not Rust-specific, but the clearest statement of the practice this doc names; the origin of the phrase.
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/) — a community-maintained catalog of idioms and anti-patterns with runnable examples; a good second opinion against this doc's specific choices.
- Related topics in this repo: [Data-Oriented Design](../../performance-optimization/data-oriented-design/learning.md) (ownership clarity and cache-friendly layout share a root cause), [Allocation Strategies](../../performance-optimization/allocation-strategies/learning.md) (the runtime cost of the clone-as-duct-tape anti-pattern), [Lock-Free Concurrency](../../performance-optimization/lock-free-concurrency/learning.md) (what `unsafe` shared-mutability actually has to prove), [Compiler Optimizations](../../performance-optimization/compiler-optimizations/learning.md) (why premature generics have a real, measurable cost).
