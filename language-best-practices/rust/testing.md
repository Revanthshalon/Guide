# Rust — Testing

## What This Is For

Rust's type system eliminates whole bug classes before a test runs, which changes *what* tests are for. You are not testing for null dereferences, data races, or use-after-free — the compiler did that. You are testing **behaviour the types can't express**: business logic, algebraic properties, error paths, and the usability of your public API.

The central structural fact is that Rust gives you **three test locations with genuinely different reach**, and each catches a class the others structurally cannot:

| Location | Sees | Compiles as | Catches what others can't |
| --- | --- | --- | --- |
| `#[cfg(test)] mod tests` (in-file) | **Private** items | Part of the crate | Internal invariants, edge cases in helpers |
| `tests/*.rs` | **Public API only** | A **separate crate** per file | "Compiles internally but the public API is unusable" |
| `///` doc examples | Public API, as a reader sees it | Separate binaries under `cargo test` | Documentation that has silently rotted |

The middle row is the one people skip and the one that pays. An integration test links your crate exactly as a consumer does, so it catches the missing `pub`, the un-exported type, the trait that isn't in scope, the API that can't actually be called without a private type. Unit tests cannot see any of that, because they're *inside*.

The third row is unusual and underrated: **your documentation examples are compiled and executed by `cargo test`**. Documentation cannot go stale silently, which makes `///` examples the cheapest test you'll ever write and the only ones users read.

## The Decisions

| Decision | Guidance |
| --- | --- |
| Unit or integration? | Test the **public contract** in `tests/`; use unit tests for private helpers with real complexity |
| Test private functions directly? | Rarely — it couples tests to structure. If a private thing is complex enough to need direct tests, it may want to be its own module or crate |
| Example-based or property-based? | Examples for specific known cases and regressions; properties where the domain has algebraic laws (round-trips, invariants, orderings) |
| One `tests/` file or many? | Many files = many crates = slower compiles. Group by feature area, not one file per source file |
| Mock or real? | Prefer real implementations and in-memory fakes; mock at trait boundaries only where the real thing is slow, non-deterministic, or external |
| `--release` for tests? | Debug by default (better panic messages, debug assertions, overflow checks). Use `--release` only for compute-heavy tests — it's **13× faster** (see [releasing](releasing.md)) |
| Coverage target | Use coverage to find *untested* areas, never as a number to hit |

## Setup

```toml
[dev-dependencies]
proptest    = "1"       # property tests with shrinking
insta       = "1"       # snapshot tests
rstest      = "0.23"    # parameterized tests / fixtures
pretty_assertions = "1" # readable diffs on assert_eq! failures
tokio       = { version = "1", features = ["macros", "rt-multi-thread", "test-util"] }

[profile.test]
# Inherits `dev`: debug assertions ON, integer overflow checks ON. Keep it that way —
# these catch bugs that a release build silently permits.
```

Layout:

```
src/
  lib.rs
  parser.rs          ← #[cfg(test)] mod tests { ... } at the bottom
tests/
  common/mod.rs      ← shared helpers. MUST be common/mod.rs, not common.rs
  parsing.rs         ← integration tests, one crate
  api_surface.rs     ← integration tests, another crate
benches/
```

`tests/common/mod.rs` rather than `tests/common.rs` is a real gotcha: every top-level `.rs` file in `tests/` is compiled as its own test crate, so `tests/common.rs` becomes a test binary with no tests, and emits warnings. A subdirectory `mod.rs` is not treated as a test target.

## The Workflow

```rust
// ─── Unit tests: colocated, see private items, zero release cost ───
pub struct Parser { /* ... */ }

impl Parser {
    fn normalize(&self, raw: &str) -> String { /* private */ todo!() }
}

#[cfg(test)]                                  // compiled ONLY for tests
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_whitespace() {
        let p = Parser::new();
        assert_eq!(p.normalize("a   b"), "a b");
    }

    #[test]
    #[should_panic(expected = "capacity must be non-zero")]   // match the MESSAGE, not just "it panicked"
    fn zero_capacity_is_rejected() { Parser::with_capacity(0); }

    #[test]
    fn parse_reports_position() -> Result<(), Box<dyn std::error::Error>> {
        let ast = Parser::new().parse("1 + 2")?;     // `?` works in tests that return Result
        assert_eq!(ast.span().start, 0);
        Ok(())
    }

    #[test]
    #[ignore = "takes ~40s; run with --ignored"]
    fn full_corpus() { /* ... */ }
}
```

```rust
// ─── Integration test: tests/parsing.rs — sees ONLY the public API ───
mod common;                                   // tests/common/mod.rs

use my_crate::{Parser, ParseError};           // exactly how a consumer imports

#[test]
fn error_variants_are_matchable_by_consumers() {
    // This is the test that catches a non-`pub` error type or an un-exported variant —
    // a unit test inside the crate can never fail this way.
    let err = Parser::new().parse("1 +").unwrap_err();
    assert!(matches!(err, ParseError::UnexpectedEof { .. }));
}
```

```rust
/// Parses an expression.
///
/// # Examples
/// ```
/// # use my_crate::Parser;                    // `#` hides the line from rendered docs
/// let ast = Parser::new().parse("1 + 2")?;
/// assert_eq!(ast.eval(), 3);
/// # Ok::<(), my_crate::ParseError>(())
/// ```
///
/// # Errors
/// Returns [`ParseError::UnexpectedEof`] if input ends mid-expression.
pub fn parse(&self, input: &str) -> Result<Ast, ParseError> { todo!() }
```

Doc-test fence annotations worth knowing: ` ```no_run ` compiles but doesn't execute (network, side effects); ` ```ignore ` doesn't even compile (avoid — it rots); ` ```compile_fail ` asserts the example *fails* to compile, which is how you test that your API correctly rejects misuse; ` ```text ` for non-Rust blocks.

### Property tests — a different bug-finding mechanism

```rust
proptest! {
    #[test]
    fn encode_decode_roundtrip(v: Vec<u64>) {
        prop_assert_eq!(decode(&encode(&v))?, v);
    }

    #[test]
    fn sort_preserves_multiset(mut v: Vec<i32>) {
        let before = v.iter().copied().collect::<std::collections::BTreeMap<_,_>>();
        v.sort_unstable();
        prop_assert!(v.is_sorted());
        // ... same elements, just reordered
    }
}
```

Properties worth reaching for: **round-trips** (`decode(encode(x)) == x`), **invariants** (output is sorted; the tree is balanced; the count is preserved), **oracles** (the fast implementation agrees with the obvious slow one), and **idempotence** (`f(f(x)) == f(x)`). The oracle pattern is especially strong for the data-structure work in [data-structures-and-algorithms](../../data-structures-and-algorithms/LEARNING-INDEX.md): property-test your arena-based structure against a `Vec`-based reference.

The real payoff is **shrinking**: when proptest finds a failure it automatically reduces the input to a minimal case, so you get `vec![0, 0]` rather than a 400-element random vector. That converts "something is wrong somewhere" into a debuggable case.

## Measured Effects

- **`cargo test` runs in debug by default, and debug is 13× slower than release** (measured in [releasing](releasing.md) on a 200k-line-processing workload: 593.9 ms vs 45.7 ms). That's why compute-heavy tests belong behind `#[ignore]` or `--release`, and why a test suite that feels slow is often just a debug build doing real work.
- Keep it debug anyway for the default path: debug builds enable `debug_assert!` and **integer overflow checks**, which are compiled out in release. A test suite run only in release silently loses overflow detection — a real bug class.

## Pitfalls

### Pitfall: Tests that share process state

- **What goes wrong:** `cargo test` runs tests **in parallel threads within one process** by default. Tests that set environment variables, change the working directory, write to a fixed path like `/tmp/test.db`, or mutate a global/`static` interfere with each other. Symptom: passes alone, fails in the suite; passes locally, fails in CI; fails only sometimes. Teams "fix" it with `--test-threads=1`, hiding the problem and making the suite slow.
- **Why it happens (the mechanism):** Parallelism is per-thread, not per-process, so all tests share one process's globals, environment, and current directory. In Rust 2024, `std::env::set_var` is `unsafe` precisely because it is not thread-safe — the language now flags this rather than letting you discover it at 3 a.m.
- **How to handle it, and why that works:** Make tests independent by construction: unique temporary paths per test (`tempfile::TempDir`), dependency-inject configuration rather than reading the environment inside the code under test, and no mutable global state. Where genuine process-level state is unavoidable, `cargo-nextest` runs **each test in its own process**, which makes isolation the default instead of a discipline.
- **Trade-offs of the fix:** Dependency injection means threading a config object through code that would otherwise read a global — real plumbing. `nextest` is another tool to install and doesn't run doc tests (you need `cargo test --doc` alongside it). Process-per-test also has higher per-test overhead, which shows up on suites with thousands of tiny tests.

### Pitfall: Testing implementation instead of behaviour

- **What goes wrong:** Tests reach into private functions, assert on internal structure, or verify that a mock was called with specific arguments. Every refactor breaks dozens of tests despite behaviour being unchanged, so the suite becomes a tax on improvement. Eventually people stop refactoring, or stop trusting the tests.
- **Why it happens (the mechanism):** Unit tests inside the module can see everything private, so it's *easy* to assert on internals — the language offers no friction. Mock-heavy designs make it worse by encoding the call sequence into the test, so the test asserts *how* the code works rather than *what* it produces.
- **How to handle it, and why that works:** Default to `tests/`, where only the public API is reachable — the boundary is enforced by the compiler rather than by discipline. Assert on outputs and observable effects, not on interactions. Keep unit tests for genuinely complex private algorithms, and let them be about the algorithm's contract (invariants, edge cases) rather than its structure.
- **Trade-offs of the fix:** Public-API-only testing gives coarser failure localization — you learn something is wrong, not which internal step. Integration tests also compile slower (each file is a crate). The balance is roughly: integration tests for confidence, a smaller number of unit tests for the tricky internals where precise localization pays.

### Pitfall: Chasing a coverage number

- **What goes wrong:** A team sets 80% line coverage as a gate. People write tests that execute code without asserting anything meaningful, test trivial getters, and add `#[cfg(no_coverage)]`-style escapes. Coverage hits 80%; the error paths — the ones that actually matter — remain untested, because they're the hardest to reach.
- **Why it happens (the mechanism):** Line coverage measures *execution*, not *verification*. A test with no assertions covers lines perfectly. And coverage is uniform across code that isn't uniformly important: a getter and a payment-reconciliation branch count the same.
- **How to handle it, and why that works:** Use `cargo-llvm-cov` as a *discovery* tool — read the uncovered report and ask "is this a path that could fail in a way that matters?" Error branches, `Drop` impls, and edge-case arms are usually where the answer is yes. Then write tests for those specifically. That converts coverage from a target (which Goodhart's law destroys) into a checklist.
- **Trade-offs of the fix:** Without a number, coverage becomes a judgment call, and judgment is unevenly applied across a team. A low floor (catching "this module has no tests at all") is defensible; the failure mode is treating a high target as a quality proxy.

### Pitfall: Non-deterministic tests

- **What goes wrong:** Tests using randomness without a fixed seed, real time (`Instant::now()`, sleeping), unordered iteration (`HashMap` order is deliberately randomized per process in Rust), or real network. They fail perhaps 1 in 200 runs. The team retries CI until it's green, and the suite stops being a signal.
- **Why it happens (the mechanism):** Each is a hidden input. `HashMap` iteration order genuinely varies between runs by design (that's the DoS defence from the [complexity analysis](../../data-structures-and-algorithms/complexity-analysis/learning.md) doc), so a test asserting on collected-into-`Vec` order from a map is *correct-looking* and inherently flaky.
- **How to handle it, and why that works:** Make every input explicit: seed RNGs with a constant and log the seed on failure; inject a `Clock` trait rather than calling `Instant::now()` (tokio's `time::pause()` does this for async); sort before asserting on anything derived from a `HashMap`, or use a `BTreeMap` in tests; never touch the network. A flaky test should be quarantined and fixed, never retried — a retry converts a real intermittent bug into an invisible one.
- **Trade-offs of the fix:** Injecting a clock is real design pressure on production code — often a good thing, sometimes over-abstraction for a small program. A fixed seed makes property tests deterministic but also *less* exploratory; proptest's default of a fresh seed per run with a persisted failure file is a better balance for properties specifically.

### Pitfall: Not testing the error paths

- **What goes wrong:** Every test constructs valid input and asserts the happy result. The `Err` arms — malformed input, exhausted resources, partial writes — are never executed. They contain the bugs, because they were never run even once. This is the most common gap in otherwise well-tested Rust code.
- **Why it happens (the mechanism):** `Result` makes errors so ergonomic to *propagate* with `?` that it's easy to never construct one deliberately. And errors are usually the harder input to build — you need a malformed file, a full disk, a closed socket.
- **How to handle it, and why that works:** Test each error variant your public API can produce, asserting with `matches!` on the variant rather than on a message string (messages change; variants are the contract). Use fault injection for I/O — a reader that returns `ErrorKind::Interrupted` on the third call, a writer that accepts 3 bytes then errors. `#[should_panic(expected = "...")]` for the genuinely-panicking paths. If an error variant cannot be triggered by any test, ask whether it can happen at all.
- **Trade-offs of the fix:** Fault injection needs the code to accept an injectable `Read`/`Write` rather than a concrete `File`, which is a design constraint (usually a good one — it's the "accept the most general borrowed input" practice from [learning.md](learning.md)). Exhaustively testing every variant of a large error enum has diminishing returns; prioritize the ones a caller is expected to handle differently.

## Checklist

- [ ] Public API exercised from `tests/` as a consumer would import it
- [ ] Shared test helpers in `tests/common/mod.rs`, not `tests/common.rs`
- [ ] Every public item has a `///` example — they run under `cargo test`
- [ ] `# Panics` / `# Errors` / `# Safety` documented where applicable
- [ ] Each public error variant has a test asserting via `matches!`
- [ ] `#[should_panic(expected = "…")]` matches the message, not just the panic
- [ ] No shared process state: temp dirs, no env mutation, no fixed paths
- [ ] No hidden inputs: seeded RNG, injected clock, no network, no `HashMap` order assertions
- [ ] Slow tests `#[ignore]`d with a reason, and run in CI's nightly job
- [ ] Property tests where the domain has round-trips or invariants
- [ ] `unsafe` code runs under `cargo +nightly miri test`
- [ ] Tests run in debug (overflow + `debug_assert!` checks) at least once per CI run

## Tooling

| Need | Tool | Notes |
| --- | --- | --- |
| Faster runs, per-test process isolation | `cargo-nextest` | Doesn't run doc tests — pair with `cargo test --doc` |
| Property tests with shrinking | `proptest`, `quickcheck` | `proptest` shrinks better; persists failing seeds |
| Snapshot tests | `insta` | `cargo insta review` for accepting diffs |
| Parameterized tests / fixtures | `rstest` | Table-driven cases without macro boilerplate |
| Coverage | `cargo-llvm-cov` | For discovery, not as a gate |
| UB detection in `unsafe` | Miri | Mandatory for hand-written `unsafe` |
| Mocking at trait boundaries | `mockall` | Use sparingly — prefer in-memory fakes |
| Temp files/dirs | `tempfile` | `TempDir` cleans up on drop |
| HTTP mocking | `wiremock` | Real server on a port; better than mocking the client |
| Readable assertion diffs | `pretty_assertions` | Drop-in `assert_eq!` replacement |
| Fuzzing | `cargo-fuzz`, `afl.rs` | For parsers and anything reading untrusted bytes |
| Concurrency permutation testing | `loom` | Exhaustive interleavings for lock-free code |

## Open Questions

- `cargo-nextest` on a suite of this shape: how much wall-clock does process-per-test actually save versus cost?
- Does `insta` snapshot testing pay for itself outside CLI-output testing, or does it mostly encode implementation detail?
- `loom` on the concurrent structures in the DSA Stage 9 topics — is it tractable for a work-stealing deque, or does the state space explode?
- What is a defensible coverage *floor* (as opposed to a target) that catches genuinely untested modules without inviting gaming?
- Fault-injection ergonomics: is there a good Rust crate for injectable I/O failures, or is a hand-rolled `Read`/`Write` wrapper still the norm?

## References

- [The Rust Book, ch. 11 — Writing Automated Tests](https://doc.rust-lang.org/book/ch11-00-testing.html) — the three locations and the mechanics.
- [Rust By Example — Testing](https://doc.rust-lang.org/rust-by-example/testing.html) — doc-test attributes (`no_run`, `compile_fail`, hidden lines) in one place.
- [`proptest` book](https://proptest-rs.github.io/proptest/) — the shrinking chapter is what makes property testing practical rather than theoretical.
- [`cargo-nextest`](https://nexte.st/) — the process-isolation model and what it does and doesn't run.
- [Miri](https://github.com/rust-lang/miri) — UB detection; non-optional for `unsafe`.
- Related in this repo: [Rust learning notes](learning.md) (the API design these tests verify), [releasing](releasing.md) (feature-combination testing, the 13× debug/release number), [benchmarking](benchmarking.md) (why measurement is not testing), [Rust for Data Structures](../../data-structures-and-algorithms/rust-for-data-structures/learning.md) (invariant checkers and oracle-based property tests).
