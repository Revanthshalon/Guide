# Rust — Quick Reference

Core model: a function signature is a complete ownership contract (`T` takes, `&T` borrows shared, `&mut T` borrows exclusive) — most design questions are "who owns this, for how long?" questions. Make illegal states unrepresentable with the type system rather than checking for them at runtime. Details in [learning.md](learning.md).

## Do / Don't

| Do | Don't | Why (one line) |
| --- | --- | --- |
| Model mutually-exclusive state as an `enum` | Use `bool`/`Option` flag combos | Compiler forces exhaustive handling; invalid combos unrepresentable |
| Typed error `enum` (`thiserror`) where callers recover | `String`/`Box<dyn Error>` from domain logic | Caller can `match` and act, not string-compare |
| `anyhow` + `.context()` where nothing recovers (CLIs, app edges) | `anyhow` inside libraries others build on | Rule is about *who consumes the error*, not crate type |
| Smart constructor → wrapper type ("parse, don't validate") | `validate(x)` then keep using raw `x` | Proof fused to the value; can't be forgotten downstream |
| `&str` / `&[T]` params (always right) | `&String` / `&Vec<T>` params | Deref coercion accepts both; no forced allocation |
| `impl AsRef<Path>`/`IntoIterator` **when the ergonomic win is real** | Reflexively generic params | Costs: error messages, inference, no turbofish, mono per caller |
| Channels / owner-task carrying owned values | `Arc<Mutex<T>>` as the default concurrency shape | Ownership transfer sidesteps lock discipline entirely |
| `std::thread::scope` for borrowing workers | `Arc` clones purely to satisfy `'static` | Scoped threads borrow local data directly |
| Private fields + invariant-preserving methods | `pub` fields | Module boundary is the only place invariants are checked |
| Split borrows / `mem::take` / index loops | `.clone()` to silence borrowck | Clones drift silently; design question goes unanswered |
| `?` propagation on fallible paths | `.unwrap()` where prod can fail | Turns recoverable errors into panics |
| Newtypes for domain invariants & confusable ids | Newtype every primitive; `Deref` to inner | Boilerplate + orphan friction; `Deref` undoes encapsulation |
| Small composable traits (a capability someone asks for by name) | God traits *or* a trait per method | Both extremes hurt: stub methods vs. five-bound `where` clauses |
| `///` docs with runnable examples; `# Panics`/`# Errors`/`# Safety` | Prose-only docs that can rot | Doc examples run under `cargo test` — cannot go stale silently |
| Derive `Debug` everywhere public (+`Clone`/`PartialEq`/`Hash` where honest) | Ship a public type without `Debug` | Orphan rule means downstream can't fix it — wide blast radius |
| `#[non_exhaustive]` on extensible public enums/structs | Assume you can add a variant later | Makes adding variants non-breaking (additive-evolution, in the language) |

## Anti-Patterns → Fixes

| Anti-pattern | Fix |
| --- | --- |
| `.clone()` as borrow-checker duct tape | 1) Split borrows (destructure fields — disjointness works in a body, not across `&mut self` calls) 2) `mem::take` + put back 3) index loop |
| Lock/`RefCell` guard held across `.await` or a long call | Scope the guard (`{ ... }`), clone out what's needed, re-acquire to write; `tokio::sync::Mutex` only if it must span; better — owner task |
| `.unwrap()` on fallible-in-prod paths | `?` or explicit `match`; `.expect("why this can't fail")` only when provable (note: `parking_lot` doesn't poison, so no `Result` at all) |
| Primitive obsession | Newtypes per domain concept (with the ceiling above) |
| Stringly-typed status/dispatch | Parse to `enum` at the boundary; exhaustive `match` after |
| Generic-izing before a 2nd real call site | Write concrete; generalize from the second caller's actual needs |
| `unsafe` to route around borrowck | Restructure (index arena, deliberate `Rc<RefCell<T>>`); if genuinely needed: minimal block + `// SAFETY:` + safe wrapper + miri in CI |
| Reflexive `Rc<RefCell<T>>` at first friction | Try `&mut self` instead of `&self`, thread `&mut`, or split the type — shared mutability is the fallback |

## Tooling

| Tool | Command | Purpose |
| --- | --- | --- |
| Formatter | `cargo fmt --check` | Non-negotiable baseline; enforce in CI |
| Linter | `cargo clippy --all-targets -- -D warnings` | Directly catches several anti-patterns above |
| Unsafe docs lint | clippy `undocumented_unsafe_blocks` | Forces the `// SAFETY:` argument to exist |
| Test | `cargo test` / `cargo nextest run` | Doc tests run here too; nextest for larger suites |
| Property testing | `proptest` / `quickcheck` | Finds + shrinks edge cases example tests miss |
| UB detection | `cargo +nightly miri test` | Mandatory alongside any `unsafe` |
| Unsafe survey | `cargo geiger` | `unsafe` usage across the dependency tree |
| Supply chain | `cargo audit` / `cargo deny check` | Vulnerabilities, licenses, duplicate versions |
| Public API drift | `cargo public-api` | Catches unintended breaking changes |
| Missing docs | `#![warn(missing_docs)]` at crate root | Keeps the public surface documented |

## Key References

- *The Rust Programming Language* (the Book) — read the ownership chapter twice.
- Jon Gjengset, *Rust for Rustaceans* — API/trait design depth.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — the derive/naming/error checklist.
- Alice Ryhl, ["Actors with Tokio"](https://ryhl.io/blog/actors-with-tokio/) — ownership-transfer concurrency.
