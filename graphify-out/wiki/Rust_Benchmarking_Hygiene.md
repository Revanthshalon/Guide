# Rust Benchmarking Hygiene

> 22 nodes · cohesion 0.10

## Key Concepts

- **criterion Harness (harness = false, baselines)** (5 connections) — `language-best-practices/rust/benchmarking.md`
- **Build Profile Measurements (13× dev, 25%/31%/40% size, opt-level z 2× slower)** (5 connections) — `language-best-practices/rust/releasing.md`
- **Three Test Locations With Different Reach** (5 connections) — `language-best-practices/rust/testing.md`
- **black_box — Cost and Correct Placement** (3 connections) — `language-best-practices/rust/benchmarking.md`
- **Breaking Change Published as a Patch Bump** (3 connections) — `language-best-practices/rust/releasing.md`
- **The Microbenchmark Mirage (Amdahl, cache privileges)** (2 connections) — `language-best-practices/rust/benchmarking.md`
- **The Optimizer Deletes Your Benchmark (0.04 ns/iter tell)** (2 connections) — `language-best-practices/rust/benchmarking.md`
- **Always Sweep, Never One Input Size** (2 connections) — `language-best-practices/rust/benchmarking.md`
- **Numbers to Remember (measured)** (2 connections) — `language-best-practices/rust/reference.md`
- **Rust Default Tooling Table** (2 connections) — `language-best-practices/rust/reference.md`
- **cargo-semver-checks** (2 connections) — `language-best-practices/rust/releasing.md`
- **Default Features That Drag In the World** (2 connections) — `language-best-practices/rust/releasing.md`
- **A Library and a Binary Have Nothing in Common at Release Time** (2 connections) — `language-best-practices/rust/releasing.md`
- **Doc Examples Run Under cargo test** (2 connections) — `language-best-practices/rust/testing.md`
- **Miri for UB Detection in unsafe** (2 connections) — `language-best-practices/rust/testing.md`
- **Language Practice Doc Template** (1 connections) — `language-best-practices/_template-practices.md`
- **iter_batched — Excluding Setup From the Measurement** (1 connections) — `language-best-practices/rust/benchmarking.md`
- **Practice: Derive Hygiene, #[non_exhaustive], #[must_use]** (1 connections) — `language-best-practices/rust/learning.md`
- **Practice: Documentation That Compiles (/// examples are tests)** (1 connections) — `language-best-practices/rust/learning.md`
- **unsafe Moves the Proof Obligation to You** (1 connections) — `language-best-practices/rust/learning.md`
- **panic = "abort" Without Auditing Unwinding** (1 connections) — `language-best-practices/rust/releasing.md`
- **Pitfall: Testing Implementation Instead of Behaviour** (1 connections) — `language-best-practices/rust/testing.md`

## Relationships

- No strong cross-community connections detected

## Source Files

- `language-best-practices/_template-practices.md`
- `language-best-practices/rust/benchmarking.md`
- `language-best-practices/rust/learning.md`
- `language-best-practices/rust/reference.md`
- `language-best-practices/rust/releasing.md`
- `language-best-practices/rust/testing.md`

## Audit Trail

- EXTRACTED: 22 (92%)
- INFERRED: 2 (8%)
- AMBIGUOUS: 0 (0%)

---

*Part of the graphify knowledge wiki. See [index](index.md) to navigate.*