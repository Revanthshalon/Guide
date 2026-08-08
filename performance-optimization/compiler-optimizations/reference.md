# Compiler Optimizations — Quick Reference

Core model: zero-cost abstraction is a contract — clear code becomes optimal code *provided the optimizer can see through it*. Inlining is the gateway (everything else fires after it); visibility is the currency (generics/monomorphization = visible; `dyn`/cross-crate-sans-LTO = opaque). The compiler cannot fix algorithms, layout, or allocation — this doc amplifies the other fourteen, it doesn't replace them. Details in [learning.md](learning.md).

## The Knobs Ladder

| Rung | Flags | Price | Typical gain |
| --- | --- | --- | --- |
| Baseline | `opt-level=3`, `debug=true` | none (symbols are runtime-free) | — |
| Sweet spot | `codegen-units=1`, `lto="thin"` | slower builds | ~5–10% |
| Aggressive | `lto="fat"`, `panic="abort"` | long links; **semantics change** (no unwinding/catch_unwind) | few % more |
| Machine-tuned | `target-cpu=native` | **local benches only** — ships SIGILL otherwise; runtime dispatch for shipping | workload-dependent |
| Measured reality | PGO (`cargo-pgo`), then BOLT | profile pipeline + staleness upkeep | ~5–15% branchy services |

## Rules of Thumb

- Generic in the hot path, `dyn` at cold edges — opacity costs the *lost optimizations*, not just the call.
- Bounds checks: ~free to execute, expensive to keep (block vectorization). Elide via iterators / hoisted slice (`&x[..n]`) / up-front `assert!`; grep hot-loop asm for `panic_bounds_check`.
- `#[inline]`: small + hot + cross-crate + non-generic. `#[inline(always)]`: measured or not at all (I-cache bloat regresses). `#[cold]` on error paths is the higher-value attribute.
- Idiomatic code hits the optimizer's patterns — write the obvious thing, check the asm, intervene on evidence.
- `&mut` = noalias everywhere: Rust's structural edge over C; don't launder pointers through raw casts in hot code and forfeit it.
- `get_unchecked`: only after safe elision patterns measurably failed, with a `// SAFETY:` that survives review.
- Pin release toolchains; upgrade with iai/asm gates watching (heuristics drift).
- PGO profiles are versioned artifacts with expiry; validate on held-out load.
- Monomorphization bills: `cargo bloat` watches size; fewer format/type instantiations when it grows.

## Verification Toolkit

| Question | Tool |
| --- | --- |
| Did the abstraction compile away? | `cargo asm` / godbolt (iterator == loop, once, yourself) |
| What did this flag actually change? | iai instruction deltas + hyperfine macro A/B |
| Codegen regression on toolchain bump? | iai/asm gates in CI |
| Binary size creep | `cargo bloat` |
| Honest report | runtime win **and** build-time cost, flags recorded with results |

## Numbers to Remember

| Thing | Number |
| --- | --- |
| `dyn` predicate in a hot filter loop | ~8–15× vs generic (inlining + vectorization lost) |
| Bounds-check elision on vectorizable loop | ~4–8× (vectorizer unblocked) |
| CGU=1 + thin LTO | ~5–10% for slower builds |
| PGO on branchy services | ~5–15%; BOLT a few % more |
| Fat LTO link times | can triple — release artifacts only |

## Benchmark Checklist

- [ ] Flags A/B'd on the macro baseline, never micro alone (inlining/layout are global)
- [ ] Build-time cost reported beside runtime win (the honest tuple)
- [ ] Load-bearing loops: asm diff in the PR when a win is claimed
- [ ] PGO validated on held-out workload
- [ ] Toolchain pinned; upgrade = a diff, not a mystery

## Key References

- Nethercote, [perf-book "Build Configuration"](https://nnethercote.github.io/perf-book/build-configuration.html).
- [rustc PGO chapter](https://doc.rust-lang.org/rustc/profile-guided-optimization.html) + cargo-pgo.
- Godbolt, "What Has My Compiler Done for Me Lately?" — the culture.
