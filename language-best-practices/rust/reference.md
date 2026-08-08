# Rust — Quick Reference

Core model: a function signature is a complete ownership contract (`T` takes, `&T` borrows shared, `&mut T` borrows exclusive) — most design questions are "who owns this, for how long?" questions. Make illegal states unrepresentable with the type system rather than checking for them at runtime. Details in [learning.md](learning.md).

## Do / Don't

| Do | Don't | Why (one line) |
| --- | --- | --- |
| Model mutually-exclusive state as an `enum` | Use `bool`/`Option` flag combos | Compiler forces exhaustive handling; invalid combos unrepresentable |
| Typed error `enum` (`thiserror`) in library code | Return `String`/`Box<dyn Error>` from domain logic | Caller can `match` and recover, not string-compare |
| `anyhow::Error` only at the app boundary | `anyhow` everywhere | Domain code needs typed errors to be actionable |
| Smart constructor → wrapper type ("parse, don't validate") | `validate(x)` then keep using raw `x` | Proof fused to the value; can't be forgotten downstream |
| `&str` / `&[T]` / `impl AsRef<Path>` params | `&String` / `&Vec<T>` params | Accepts more callers for free, no forced allocation |
| Concrete owned return types | Generic bounds speculated before a 2nd call site | Monomorphization cost is real; generalize from evidence |
| Private fields + invariant-preserving methods | `pub` fields | Module boundary is the only place invariants are checked |
| Reorder/restructure borrows first | `.clone()` to silence borrowck | Clones drift silently; the design question goes unanswered |
| `Result` propagation (`?`) on fallible paths | `.unwrap()`/`.expect()` on paths that can fail in prod | Turns recoverable errors into process panics |
| Newtypes (`AccountId(u64)`) for domain values | Bare `u64`/`String` for everything | Compiler catches transposed/mismatched arguments |
| Small composable traits | One "god trait" with stub `unimplemented!()` methods | No implementor forced to fake capabilities it lacks |

## Anti-Patterns → Fixes

| Anti-pattern | Fix |
| --- | --- |
| `.clone()` as borrow-checker duct tape | Reorder borrows (read before mutate); restructure ownership |
| `.unwrap()` on fallible-in-prod paths | `?` propagation or explicit `match`; reserve `.expect()` for provably-infallible cases with a message saying why |
| Primitive obsession (`u64` for ids, amounts, everything) | Newtype wrappers per domain concept |
| Stringly-typed status/dispatch | Parse to `enum` at the boundary; exhaustive `match` everywhere after |
| Generic-izing before a 2nd real call site | Write concrete; generalize from the second caller's actual needs |
| `unsafe` to route around a borrow-checker objection | Restructure (index arena, `Rc<RefCell<T>>` deliberately chosen) — `unsafe` is a proof obligation, not an escape hatch |
| Reflexive `Rc<RefCell<T>>` at first friction | Try ownership restructuring / `&mut` threading first; shared mutability is the fallback, not the default |

## Tooling

| Tool | Command | Purpose |
| --- | --- | --- |
| Formatter | `cargo fmt` | Non-negotiable baseline; run in CI as a check |
| Linter | `cargo clippy --all-targets -- -D warnings` | Catches idiom violations, several anti-patterns above directly |
| Docs lint | `cargo doc --no-deps` (watch warnings) | Public API surface sanity |
| Test | `cargo test` / `cargo nextest run` | nextest for speed + better output on larger suites |
| Property testing | `proptest` / `quickcheck` | Finds edge cases example tests miss, shrinks to minimal repro |
| Unsafe review | `cargo geiger` | Surfaces `unsafe` usage across the dependency tree |
| Miri | `cargo +nightly miri test` | Catches UB in `unsafe` code and some concurrency bugs |
| Dependency audit | `cargo audit` / `cargo deny check` | Known vulnerabilities, license/duplicate-version policy |
| API guideline check | [rust-lang.github.io/api-guidelines](https://rust-lang.github.io/api-guidelines/) checklist | Manual pass for public crates |

## Key References

- *The Rust Programming Language* (the Book) — read the ownership chapter twice.
- Jon Gjengset, *Rust for Rustaceans* — API/trait design depth.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — the community checklist.
