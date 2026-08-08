# Branch Prediction — Learning Notes

## The Hardware Mechanism

**The core doesn't execute your program; it executes its guess about your program.** A modern pipeline is ~15–20 stages deep and 4–8 instructions wide: by the time a conditional branch actually *resolves* (its condition computed, deep in the pipeline), the front-end has already fetched, decoded, and started executing dozens of instructions from *somewhere* — it had to pick a path the moment it fetched the branch. That pick is the **branch predictor's** job, and the two outcomes define this topic:

- **Predicted correctly:** the branch costs roughly nothing — the speculative work was the right work; the pipeline never hiccuped. A well-predicted branch is ~1 cycle, often fused with the comparison.
- **Mispredicted:** everything fetched after the branch is wrong-path work. The pipeline **flushes** and restarts from the correct target: **~15–20 cycles** of throughput gone — at 4–8 wide, that's 60–150 instruction slots per miss. A branch that mispredicts often is one of the most expensive single constructs you can put in a hot loop.

What the predictor actually is: a set of on-core tables tracking branch history. Modern predictors (TAGE-class) correlate each branch's outcome with a long *history of recent branch outcomes* (global history), which lets them learn astonishingly deep patterns — not just "usually taken" but alternations, short repeating sequences, loop trip counts, and correlations *between different branches*. Real-world prediction rates on ordinary code run 95–99%+. Three sub-mechanisms worth naming because they show up as distinct problems:

- **Direction prediction** — taken/not-taken for conditionals: the star of this doc.
- **Indirect branch prediction (BTB)** — for jumps whose *target* is data (`match` jump tables, `dyn Trait` calls, function pointers): the predictor must guess an address, not a bit. Many hot targets alternating unpredictably = repeated mispredicts; this is the hardware face of "virtual dispatch in a hot loop."
- **The return stack buffer** — a small hardware stack predicting `ret` targets; deep or irregular recursion overflows it and every return past that depth mispredicts.

The one-line cost model everything below uses: **branch cost ≈ miss-rate × ~17 cycles.** A branch taken the same way 99% of the time costs ~0.17 cycles amortized — free. A 50/50 data-dependent branch costs ~8–9 cycles *every iteration* — often more than the loop's entire real work.

## Mental Model

**Branches aren't slow. *Surprising* branches are slow.** The predictor is a pattern learner, so the operative question about any hot-path branch is not "is there an `if`?" but **"is the outcome learnable from history?"** Sorted by learnability:

1. **Structurally predictable — free.** Loop back-edges (taken N times, not-taken once), error/bounds checks that never fire, invariant flags. This is why Rust's bounds checks are usually ~free (a never-taken branch predicts perfectly — their real cost is inhibited vectorization, a [compiler-optimizations](../compiler-optimizations/learning.md) story), and why "avoid ifs" as a blanket rule is cargo cult.
2. **Pattern-predictable — cheap.** Regular alternations, correlated conditions ("if A fired, B fires"), short repeating sequences. Modern history-based predictors learn these; don't "optimize" them away without a counter saying they miss.
3. **Data-random — expensive.** The branch condition depends on high-entropy data: `if value >= threshold` over unsorted values, `match` over shuffled type tags, hash-dependent paths. No history helps; miss rate approaches the data's entropy; the 8-cycles-per-iteration tax lands. **This — and only this — is the case the techniques below exist for.**

The two escape routes from case 3, and the trade between them:

- **Make the data predictable** — sort/partition/group so equal-outcome items are contiguous: the branch still exists but now predicts (the data-side fix, and the same batch+sort move as [cache locality](../cache-locality/learning.md) — the two benefits usually arrive together).
- **Delete the branch** — compute both sides and select (**branchless**): conditional moves (`cmov`/`csel`), arithmetic on `bool as i64`, masks, lookup tables. Cost model flips: you pay a *constant* small overhead (both sides executed + select) every iteration, in exchange for *zero* mispredicts. Branchless wins when `miss_rate × 17 > constant_overhead` — i.e. on random data — and **loses on predictable data**, where the branch was free and you added work. Neither is universally better; entropy decides, which is why measurement (and A/B on both sorted and shuffled inputs) decides.

One subtlety that separates practitioners from folklore: `cmov` turns a *control* dependency into a *data* dependency — the select can't retire until both inputs are ready, which can lengthen the loop's critical path and inhibit the speculation that was *hiding* latency (a mispredicted-but-mostly-right branch sometimes beats cmov for exactly this reason). And a security-adjacent cousin: constant-time cryptographic code is branchless *for a different objective* — making timing independent of secrets — where being slower-but-constant is the *goal*; don't confuse the disciplines.

## Worked Example

The most famous benchmark on the internet ("why is processing a sorted array faster?"), run properly. Sum values ≥ 128 from 32 K random bytes:

```rust
// A. Branchy
let mut sum = 0i64;
for &x in &data {
    if x >= 128 { sum += x as i64 }        // 50/50 on random bytes
}

// B. Branchless: bool → 0/1, multiply as mask
for &x in &data {
    sum += (x >= 128) as i64 * x as i64;   // no control flow in the loop body
}
```

Four cells — the same two loops, on shuffled vs. sorted data (illustrative shape; reproducing it is exercise one):

```
                 shuffled        sorted
A. branchy       ~6.0 ns/elem    ~1.0 ns/elem      ← sorting = ~6× on identical code
B. branchless    ~1.3 ns/elem    ~1.3 ns/elem      ← flat: immune to data order
```

The `perf stat` signature that explains every cell (stage 3 of [the funnel](../profiling-and-measurement/learning.md)):

```
A/shuffled:  branch-misses ≈ 25% of branches   IPC ~0.6    ← the tax
A/sorted:    branch-misses ≈ 0.1%              IPC ~3.5    ← same code, predictor happy
B/either:    branch-misses ≈ 0.1%              IPC ~3.8, more instructions retired
```

Readings: **A-shuffled vs A-sorted** is the entire mechanism in one comparison — identical instructions, 6× apart, only the predictability of the data changed. **B's flatness** is the branchless contract: constant cost, entropy-immune. **A-sorted beats B** — on predictable data the branch is free and branchless is pure overhead: branchless is a *situational* tool, not an upgrade. (In real code, check what the compiler already did before hand-optimizing: LLVM frequently auto-converts small selects to `cmov` — read the assembly via `cargo asm` or Compiler Explorer; the A/B above assumes it didn't.)

Postscript for honest bookkeeping: sorting cost O(n log n) — worth it only if the data is swept repeatedly or was needed sorted anyway. One pass? The branchless form or just eating the mispredicts may win overall — [Amdahl and the macro baseline](../profiling-and-measurement/learning.md) arbitrate, as always.

## Applying It

Rust practice, ordered by how often it's the right move:

- **Confirm the diagnosis before treating.** `perf stat -e branches,branch-misses`: hot loops with miss rates in the double digits are the patient; anything under a few percent is not worth touching. (macOS: Instruments' CPU counters, or cachegrind's `--branch-sim=yes` for deterministic A/B.)
- **Sort/partition first.** The data-side fix is usually cheaper to maintain than clever code and compounds with cache wins: group work items by type/outcome before the loop (`sort_unstable_by_key`, `partition`, or per-category `Vec`s built at ingest — the [DoD](../data-oriented-design/learning.md) shape). A `match` over *grouped* tags predicts nearly perfectly; the same `match` over shuffled tags is an indirect-branch storm.
- **Branchless idioms, when data must stay random:** `(cond) as i64 * val` (the mask-multiply), `if cond { a } else { b }` on simple values (LLVM → `cmov`/`csel`), `i64::max`/`min`/`clamp` (compile to selects), bit tricks (`(x >> 63)` sign masks), and small **lookup tables** indexed by the condition byte when several outcomes exist. Verify in the assembly that you actually got branch-free code — the optimizer both giveth (auto-cmov) and taketh (re-branchifies "clever" code it sees through).
- **Iterator note:** `data.iter().filter(|x| cond).sum()` still branches per element — `filter` is control flow, not magic; `.map(|x| (cond) as i64 * x).sum()` is the branchless spelling. Autovectorization changes the game entirely (SIMD compares produce masks — the [SIMD doc](../simd/learning.md)'s territory, and the real end-state for hot filters).
- **Hint layout, not direction:** stable Rust's tool is `#[cold]` on functions and `#[inline(never)]` for error paths — moving unlikely code out of the hot instruction stream (an I-cache win that also helps the front-end). `core::hint::unlikely` remains unstable; **PGO** (`cargo-pgo`) is the production-grade version — the compiler measures real branch biases and lays out accordingly; BOLT goes further post-link. For a hot service binary, PGO is often a free 5–15%.
- **Indirect-branch hygiene:** `match` on a dense enum compiles to a jump table (one indirect jump, predictable if tags cluster); `dyn Trait` in a hot loop is an indirect call per element — if the profile shows it, the fixes are (in order) group-by-type, `enum_dispatch` (converts dyn calls to a match), or generics/monomorphization (no dispatch at all).
- **Recursion depth:** the return-stack buffer holds ~16–32 entries; hot recursion deeper than that mispredicts every return on the way up — one more reason iterative/explicit-stack rewrites of hot tree walks pay.

## When It Hurts

- **Branchless on predictable data is a self-inflicted tax** — both sides computed, every iteration, to avoid mispredicts that weren't happening. The A-sorted-vs-B cell. Most real-world branches are predictable; counters before surgery.
- **`cmov`'s data-dependency drag:** on latency-critical dependency chains (pointer walks, reductions), the select serializes what speculation was parallelizing. Symptom: branchless version has *fewer* misses but *lower* IPC and worse time. It's real and it's why "always cmov" is as wrong as "never cmov."
- **Evaluating both sides isn't always legal or cheap:** side effects, potential panics/UB (can't speculatively index out of bounds), or an expensive arm make branchless unavailable or counterproductive. Masks work for arithmetic, not for `launch_missile()`.
- **Readability debt compounds:** bit-trick code is write-once; the next reader (you, in a year) re-derives it. Encapsulate branchless kernels behind named functions with the branchy version in a comment or `#[cfg(test)]` oracle.
- **Fighting the compiler:** hand-branchless code the optimizer re-branchifies, or hand-branches it converts to cmov — the source-level `if` is a *request*, not an instruction. The assembly is the only truth; check it or don't bother.

## Benchmarking Methodology

- **A/B across entropy, always four cells:** {branchy, branchless} × {sorted/grouped, shuffled} — one axis shows the mechanism, the other shows which variant your *real* data distribution wants. A benchmark on one distribution answers half the question.
- **Counters over time:** `branch-misses/branches` names the mechanism; time alone can't distinguish mispredict pain from cache pain (shuffling data also breaks prefetching — on pointer-free arrays like the worked example it's pure branch effect; on pointer-chasing workloads the two mix, so use counters to apportion).
- **Beware the predictor-training mirage** (this topic's warm-cache equivalent): a criterion loop replaying one small input trains the predictor on *that input's exact sequence* — miss rates near zero that production will never see. Rotate inputs across iterations, size them past what history tables memorize, and match production entropy.
- **Instruction-count divergence is the tell:** iai/`instructions` up but time down = successful branchless trade (more work, no flushes). Time up despite misses down = the cmov-drag case. The two-counter view (instructions + branch-misses) reads the trade directly.
- **Verify code shape in assembly** (`cargo asm`, Compiler Explorer, `perf annotate`): confirm branchless actually compiled branchless, and that the branchy baseline didn't silently become cmov — otherwise the A/B measured nothing.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Why does a mispredict cost ~15–20 *cycles* but 60–150 *instruction slots*? What does the pipeline do with the wrong-path work?
2. Derive the amortized cost of a 99%-predicted branch and a 50/50 branch. At what miss rate does a 3-cycle branchless overhead break even?
3. Rust bounds checks are branches — why are they nearly free, and what's their *actual* performance cost mechanism?
4. `A-sorted` beats `B-branchless` in the worked example. Explain why, and name the general principle about when branchless is a tax.
5. What does `cmov` do to the dependency graph that a predicted branch doesn't? Construct the workload shape where that makes branchless *slower despite fewer misses*.
6. A hot loop calls `dyn Trait::process()` per element over shuffled heterogeneous objects. Name the hardware structure being thrashed and the three escalating fixes.
7. Why can a criterion benchmark show a 0.1% miss rate for a branch that misses 30% in production?

Measurement exercises:

- Reproduce the four-cell experiment on your machine (32 K bytes, threshold 128): all four cells, with `branch-misses` and IPC recorded per cell (Instruments or cachegrind `--branch-sim` on macOS). Compare your ratios to the doc's illustrative ones; explain deviations.
- Find your break-even entropy: run the branchy version at taken-probabilities 50/60/75/90/95/99% (generate accordingly) and plot ns/elem — the curve *is* the miss_rate × penalty model made visible; overlay the branchless flat line and mark the crossover.
- Read the assembly: write `if x > k { a } else { b }` over simple integer arms, check whether LLVM emitted `cmov`/`csel`; then add a function call to one arm and watch it become a real branch. The habit of checking is the lesson.

## Open Questions

- Apple Silicon specifics: mispredict penalty and predictor behavior on M-series vs. x86 (`csel` vs `cmov` characteristics) — the four-cell experiment's ratios on this Mac, measured.
- Where exactly does LLVM's select-vs-branch heuristic sit in current rustc, and which patterns reliably produce cmov across versions (worth a small compile-test corpus)?
- PGO on a real Rust service: measured win, workflow friction with `cargo-pgo`, and how stale a profile can get before it hurts.
- Lookup tables vs. mask arithmetic for multi-way branchless: at how many outcomes does the table's cache footprint beat the arithmetic chain?
- How much does hot-path `#[cold]`/error-path hygiene actually move front-end stalls on a branch-heavy service binary — measure `topdown-fe-bound` before/after.

## References

- The Stack Overflow question ["Why is processing a sorted array faster than processing an unsorted array?"](https://stackoverflow.com/q/11227809) — the canonical demonstration; Mysticial's answer is a complete mini-course.
- Agner Fog, [*The microarchitecture of Intel, AMD and VIA CPUs*](https://www.agner.org/optimize/), branch-prediction chapters — the actual predictor designs per core generation; read once to replace folklore with mechanism.
- Daniel Lemire's blog ([lemire.me](https://lemire.me/blog/)) — a decade of measured branchless/branchy experiments in the exact style this doc prescribes; search "branch".
- Nicholas Nethercote, [The Rust Performance Book](https://nnethercote.github.io/perf-book/) — the `#[cold]`/inlining/PGO toolchain notes.
- [Compiler Explorer](https://godbolt.org/) with `rustc` — the verify-the-assembly habit's home; pair with `cargo asm` locally.
- Related topics in this repo: [Profiling & Measurement](../profiling-and-measurement/learning.md) (the branch-miss signature routes here; the training-mirage is its warm-cache mirage), [Cache Locality](../cache-locality/learning.md) (batch+sort serves both mechanisms at once), [SIMD](../simd/learning.md) (masks as the end-state for hot filters), [Data-Oriented Design](../data-oriented-design/learning.md) (group-by-type as architecture), [Compiler Optimizations](../compiler-optimizations/learning.md) (PGO/BOLT, bounds-check elision).
