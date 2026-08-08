# Compiler Optimizations — Learning Notes

## The Hardware Mechanism

The "hardware" of this topic is the optimizer pipeline standing between your source and every mechanism in the previous fourteen docs. Rustc lowers through MIR (its own optimizations: inlining of tiny fns, const-prop) into **LLVM**, where the heavy machinery lives:

- **Inlining is the gateway drug.** Almost no optimization works across an opaque function call — the optimizer must *see* the code to transform it. Inlining replaces the call with the body, and then everything else fires: constant propagation (a parameter that's always 8 becomes the literal 8), dead-code elimination (the branch on `if n == 0` vanishes), scalar replacement (that temporary struct never materializes — its fields become registers), loop-invariant code motion, GVN/CSE (recomputed expressions deduplicated), unrolling, and [autovectorization](../simd/learning.md). The visible/invisible boundary — what got inlined — decides more about your generated code than any other single factor.
- **The optimizer works with what it can prove.** Rust hands LLVM an unusually strong proof kit: `&mut` is `noalias` by construction (the aliasing guarantees C programmers annotate with `restrict`, Rust provides *everywhere, verified*), types can't alias arbitrarily, and no UB-by-default means transformations stay legal. This is the mechanical substance behind "Rust is fast": not magic, but *more provable facts per line* than C++ gives its optimizer.
- **What it cannot prove, it cannot do:** it doesn't know your data's values ([PGO](#applying-it) injects measured reality), your deployment CPU (`target-cpu` widens the instruction menu), or anything across crate boundaries unless LTO opens them. And it *cannot* change your algorithm, your [data layout](../memory-layout/learning.md) (it will not convert AoS to SoA, ever), or your [heap placement](../allocation-strategies/learning.md) — the previous fourteen docs exist precisely because those decisions are above the compiler's pay grade. This doc is about **not obstructing what it can do**, and paying for the last few knobs.

## Mental Model

**Zero-cost abstraction is a contract with terms: write clear code, get optimal code — *provided the optimizer can see through the abstraction*. Visibility is the currency; your job is to spend it deliberately.**

1. **The abstraction ledger, by visibility:** iterators, closures passed to generic functions, `Option`/`Result` combinators, newtypes — all compile away *because monomorphization inlines them* (a generic instantiated per type is a concrete function the optimizer sees whole). The opaque citizens: **`dyn Trait` calls** (a runtime jump — nothing inlines through it, [the indirect-branch cost](../branch-prediction/learning.md) plus the lost-optimization cost, which is usually bigger), function pointers, `#[inline(never)]`, and cross-crate calls without LTO. The rule: *generic in the hot path, dyn at the [cold edges](../data-oriented-design/learning.md)* — the same line DoD drew, now with the compiler's reasons.
2. **Bounds checks: cheap to run, expensive to keep.** A slice index emits a check; [predicted-never-taken, it costs ~nothing per execution](../branch-prediction/learning.md) — but its *presence* blocks transformations (the optimizer can't reorder/vectorize a loop whose iterations might panic mid-way). The elision game: iterators (no index, no check), hoisting (`let s = &x[..n];` before the loop — one check, then provably in-bounds), `chunks_exact`, `zip` over separately-indexed arrays, an `assert!(a.len() >= n)` up front that teaches the optimizer the fact it needed. `get_unchecked` is the last resort with a safety comment, rarely worth its risk after the patterns above.
3. **The knobs ladder** (each rung costs build time or deployment flexibility; illustrative gains for a typical service binary):
   - `opt-level = 3` + `debug = true` (symbols cost nothing at runtime — [profiling's](../profiling-and-measurement/learning.md) prerequisite): the baseline.
   - `codegen-units = 1` (one LLVM module per crate: better intra-crate inlining; ~5–10% for slower builds) + **thin LTO** (cross-crate inlining at tolerable link cost; the sweet spot).
   - **Fat LTO** + `panic = "abort"` (whole-program optimization; abort deletes unwinding tables and landing pads — smaller, slightly faster, *changes semantics*: no `catch_unwind`, FFI unwind edges gone): the aggressive tier, a few % more.
   - `target-cpu = native` for **your own machines only** (unlocks AVX2/NEON-level codegen everywhere; ships an illegal-instruction crash to older CPUs — [runtime feature dispatch](../simd/learning.md) is the shipping answer).
   - **PGO** (`cargo-pgo`: build instrumented → run representative load → rebuild with profile): the compiler stops guessing branch weights and block layout and uses *measured reality* — typically 5–15% on branchy services, [the branch doc's](../branch-prediction/learning.md) layout hints industrialized. BOLT reorders the linked binary's code layout post-link for a further few %.
4. **The compiler is an unstable ally.** Inlining heuristics, vectorization cost models, and MIR opts shift across releases; the beautiful codegen you verified in March can quietly regress in June's toolchain ([the silent-scalar regression](../simd/learning.md) generalized). The response is not distrust but *instrumentation*: assembly checks for load-bearing loops, [iai instruction-count gates](../profiling-and-measurement/learning.md) in CI, and pinned toolchains for release builds.
5. **Where this doc sits in the funnel: last.** The compiler amplifies good decisions and cannot rescue bad ones — flags on top of pointer-chasing AoS with a `Mutex` in the loop optimize the wrong program harder. Knobs are the *cheapest* wins per engineering-hour (a Cargo.toml edit) and the *smallest* in magnitude; that's why they're rung one of effort and rung last of the curriculum.

## Worked Example

Three vignettes, each provable on your machine with `cargo asm`/godbolt.

**1. The contract, honored.** Sum of squares of evens:

```rust
// A: the abstraction        // B: the hand loop
xs.iter().filter(|x| *x % 2 == 0).map(|x| x * x).sum()
                              // for x in xs { if x % 2 == 0 { s += x * x } }
```

At `opt-level=3`: **identical assembly** — the iterator chain inlines into the same loop (often both autovectorized). This is the zero-cost claim, *verified rather than believed* — and verifying it once, yourself, permanently changes how you write Rust: you stop avoiding abstractions for imagined costs.

**2. The contract, broken by opacity.** Same logic, but the predicate is a field: `pred: Box<dyn Fn(&u64) -> bool>`:

```
per-element: an indirect call — no inlining through it, so no const-prop,
no vectorization, plus the call overhead itself.  ~8-15× slower on this shape.
```

The fix ladder (generic parameter `F: Fn(...)` → monomorphized and inlined; or `enum` of known predicates → [match, predictable](../branch-prediction/learning.md)) restores vignette 1's codegen. Opacity, not abstraction, was the cost.

**3. The bounds-check tax and its elision.** Summing `a[i] * b[i]` with indices over `0..n` where `n` isn't provably ≤ both lengths: two checks per iteration, **vectorization blocked** (iterations might panic). Three fixes, same codegen result:

```rust
let (a, b) = (&a[..n], &b[..n]);            // hoist: one check each, loop is clean
a.iter().zip(b).map(|(x, y)| x * y).sum()   // iterator: no indices at all
// or: assert!(a.len() >= n && b.len() >= n); before the indexed loop
```

After any of them: checks gone from the loop body, autovectorization returns, ~4–8× on this shape. The general lesson: the optimizer wanted to help and needed *one provable fact*; the code's job was to state it.

**4. The knobs, cumulatively** (illustrative, service-shaped workload): baseline release 100% → `codegen-units=1` + thin LTO ~93% runtime → fat LTO + `panic=abort` ~90% → PGO ~82%. Each rung: measure on the [macro baseline](../profiling-and-measurement/learning.md), pay the build-time bill knowingly (fat LTO can triple link times), and record flags beside results — a benchmark without its build config is not a result.

## Applying It

- **The release-profile recipe** (start here; it's a Cargo.toml paste):
  ```toml
  [profile.release]
  debug = true              # symbols for profiling; free at runtime
  lto = "thin"              # cross-crate inlining, tolerable builds
  codegen-units = 1         # intra-crate inlining; slower builds
  # panic = "abort"         # opt-in: smaller/faster, no unwinding — decide consciously
  ```
- **`#[inline]` discipline:** the attribute *permits* cross-crate inlining for non-generic fns (generics are always candidates); use it on small, hot, cross-crate functions. `#[inline(always)]` is a heuristic override — measure or don't; sprayed everywhere it bloats I-cache and *regresses* ([the front-end stalls](../branch-prediction/learning.md) you were avoiding). `#[cold]`/`#[inline(never)]` push error paths out of hot instruction streams — the more valuable attribute in practice.
- **De-opaque the hot path:** `dyn` → generics or `enum_dispatch` ([the branch doc's ladder](../branch-prediction/learning.md), now for optimization visibility, which usually matters more than the call itself); closures stay generic (`impl Fn`, not `Box<dyn Fn>`) through hot call chains.
- **Bounds-check elision as a habit:** iterators first; hoist-slice or up-front `assert!` where indices are unavoidable; audit any hot loop's assembly for `panic_bounds_check` calls — their presence names both a per-iteration cost and a blocked vectorizer.
- **PGO workflow** (`cargo-pgo` wraps it): instrumented build → drive with *representative* load (the profile is only as good as the workload — [the same representativeness rule as profiling](../profiling-and-measurement/learning.md)) → optimized rebuild; refresh profiles on a schedule, staleness decays the win. BOLT after, if the binary is large and hot.
- **Verification toolkit:** `cargo asm` / godbolt for spot checks; `iai-callgrind` instruction gates on load-bearing kernels; `cargo bloat` for size regressions (monomorphization's bill — [the serialization doc's](../serialization-and-encoding/learning.md) compile-time note has a runtime-size sibling); pin the release toolchain, upgrade deliberately with the gates watching.
- **Build-time economics:** thin LTO + CGU=1 is most teams' ceiling; fat LTO for release artifacts only (CI dev builds keep defaults); remember the compile-time/runtime trade is itself [an F-vs-m decision](../batching-and-amortization/learning.md) — pay big fixed build cost only where the binary's runtime hours amortize it.

## When It Hurts

- **`target-cpu=native` shipped:** the classic — benchmarks on the build machine, `SIGILL` on the customer's older Xeon. Native is for local benchmarking; shipping means baseline + [runtime dispatch](../simd/learning.md), or an explicitly chosen minimum (`target-cpu=x86-64-v3` with documented requirements).
- **`panic=abort` as a silent default:** it changes program semantics — `catch_unwind` returns Err never fires (breaks some test harnesses, some FFI callback contracts, some web-server isolation models). It belongs in a deliberate decision log, not copied from a speed blog.
- **Heuristic fights:** `#[inline(always)]` sprayed until I-cache misses climb; hand-unrolled loops the vectorizer would have handled better; "optimized" code so contorted LLVM no longer recognizes the idiom (the optimizer pattern-matches — *idiomatic* code hits the patterns). Write the obvious thing, check the asm, intervene only on evidence.
- **PGO staleness and skew:** a profile from last quarter's traffic optimizes last quarter's branches; worse, a profile from the *wrong* workload actively pessimizes the right one. Treat profiles as versioned artifacts with expiry.
- **`get_unchecked` for sport:** UB risk permanently, for a win the hoist-or-assert patterns get safely nine times in ten. It needs a benchmark showing the safe patterns failed *and* a `// SAFETY:` comment that survives review.
- **Chasing compiler-version deltas:** a 3% regression on toolchain upgrade is usually heuristic drift, not your bug — burn engineering time only if the gates show a load-bearing kernel deoptimized (then: minimize, report upstream, pin meanwhile).

## Benchmarking Methodology

- **A/B flags on the macro baseline** ([hyperfine](../profiling-and-measurement/learning.md), full workload) — never on microbenchmarks alone (inlining/layout effects are global; micro slices miss them). Record the full flag set with every result.
- **Instruction counts read codegen changes cleanly:** [iai](../profiling-and-measurement/learning.md) deltas on kernels show what the flag actually did (fewer instructions? or same instructions scheduled better — then wall time and IPC tell the rest).
- **Assembly diffs for the load-bearing few:** before/after `cargo asm` on the loops that matter, checked into the PR description when a flag or attribute claims a win — reviewable evidence beats adjectives.
- **Report build cost beside runtime win:** "+4% runtime, +180 s clean build, +90 s incremental" is the honest tuple; teams under-account the daily compile tax against the runtime dividend.
- **PGO needs an extra honesty check:** measure PGO'd binary on a *held-out* workload, not the training load — same overfitting logic as any measured-then-optimized system.
- **Gate against toolchain drift:** the iai/asm checks run in CI on toolchain bumps; a pinned toolchain plus deliberate upgrades converts "the compiler changed something" from a mystery into a diff.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Why is inlining the gateway optimization? List four transformations that only fire after it, and what "visibility" means mechanically.
2. What does Rust's `&mut`-noalias guarantee let LLVM do that C compilers must assume away? Construct a two-pointer loop where it matters.
3. Bounds checks: reconcile "predicted-never-taken ≈ free" with "blocks vectorization." Give the three elision patterns and what fact each proves to the optimizer.
4. `Box<dyn Fn>` in a hot loop vs. generic `F: Fn`: enumerate *both* cost components of the former and rank them.
5. Recite the knobs ladder with each rung's price (build time / semantics / deployment). Which rung changes program behavior, and how?
6. Why is PGO "the branch doc industrialized"? What decays it, and what's the held-out-workload check for?
7. Your toolchain upgrade regressed a kernel 4%. What's the triage sequence, and when do you *not* investigate?

Measurement exercises:

- Verify the contract: vignette 1's iterator-vs-loop on your machine — `cargo asm` both, confirm identity, then break it with `Box<dyn Fn>` and measure the multiple. (This single exercise recalibrates your abstraction instincts permanently.)
- Bounds-check elision lab: the indexed two-array product, then the three fixes; confirm `panic_bounds_check` disappears from the asm and measure the vectorization return.
- Run the knobs ladder on a real project of yours: baseline → CGU=1+thin-LTO → fat+abort → (Linux target) PGO — hyperfine at each rung, build times recorded, the honest tuple tabulated. This is the cheapest real speedup you haven't collected yet.

## Open Questions

- Current MIR-level optimization state: what does rustc optimize *before* LLVM these days (MIR inlining thresholds, const-prop reach) — and does it change the small-function `#[inline]` calculus?
- PGO on macOS/M-series targets: llvm-profdata workflow status via cargo-pgo, and typical gains for an Apple-silicon-deployed service vs the Linux numbers.
- BOLT for Rust binaries in practice: wins on a real service, and the symbol/relocation prerequisites (`--emit-relocs`) worth the pipeline complexity?
- `-Zbuild-std` (rebuilding std with your flags/target-cpu): measurable on top of fat LTO, or marginal?
- Where do the vectorizer's cost models currently give up on Rust idioms — refresh the [SIMD doc's fragility corpus](../simd/learning.md) against the current toolchain and diff yearly.

## References

- Nicholas Nethercote, [The Rust Performance Book — "Build Configuration"](https://nnethercote.github.io/perf-book/build-configuration.html) — the knobs ladder with current syntax and honest caveats; this doc's Applying-It in living form.
- [rustc book: Profile-Guided Optimization](https://doc.rust-lang.org/rustc/profile-guided-optimization.html) + [cargo-pgo README](https://github.com/Kobzol/cargo-pgo) — the PGO workflow end to end.
- Matt Godbolt, "What Has My Compiler Done for Me Lately?" (CppCon 2017) — the check-the-assembly culture, from the person who built the tool; transfers to Rust verbatim.
- [Compiler Explorer](https://godbolt.org/) + `cargo asm` — the verification habit's instruments ([shared with the branch and SIMD docs](../branch-prediction/learning.md)).
- Denis Bakhvalov, *Performance Analysis and Tuning on Modern CPUs* (free online) — ch. on CPU front-end and PGO/BOLT: the layout-optimization story at book depth.
- Related topics in this repo: this doc is the *amplifier* for all fourteen before it — [profiling](../profiling-and-measurement/learning.md) (verify everything), [branch prediction](../branch-prediction/learning.md) (PGO, `#[cold]`, dispatch), [SIMD](../simd/learning.md) (autovectorization's terms), [memory layout](../memory-layout/learning.md) + [DoD](../data-oriented-design/learning.md) (what the compiler can never do for you — the reason the rest of the category exists).
