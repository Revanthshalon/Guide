# Rust — Releasing Libraries and Binaries

## What This Is For

Shipping Rust artifacts that don't create problems later. **A library and a binary have almost nothing in common at release time**, and conflating them is the root of most release mistakes:

- A **library**'s product is its *public API and its compatibility promise*. Once published to crates.io, a version is immutable and permanent — you cannot unpublish it, only yank it. Every public item is a commitment, and every dependency you add becomes your consumers' dependency. The failure mode is a breaking change shipped as a patch bump that fractures the ecosystem, or a bloated dependency tree that everyone downstream now compiles.
- A **binary**'s product is *an artifact that runs on a target machine*. Nobody depends on its internals, so you're free to break anything — the concerns are size, startup, portability, reproducibility, and getting it onto machines. The failure mode is a binary that won't run on the deployment target, or one you can't reproduce when you need to debug what shipped.

The build profile is the one place they overlap, and it's the cheapest large win available. Measured below: **the default `dev` profile runs 13× slower than `release`**.

## The Decisions

| Decision | Library | Binary |
| --- | --- | --- |
| Public API size | Minimize — every `pub` is forever | Irrelevant |
| Semver discipline | **Mandatory**, enforced with tooling | Version is a label for humans |
| Dependencies | A tax on every consumer; keep minimal, default features off | Yours alone; add freely |
| MSRV (`rust-version`) | A compatibility promise; bumping it is semver-relevant to many | Whatever your CI uses |
| `Cargo.lock` | **Commit it** (modern guidance — it governs your CI only, not consumers) | Commit it |
| Build profile | Consumers pick their own | **You** pick — see the table below |
| Binary size | N/A | Real: cold-start, container layers, download |
| `panic = "abort"` | Never set it for consumers | Often correct |
| Distribution | `cargo publish` | Release artifacts, containers, package managers |

## Setup

### Library `Cargo.toml`

```toml
[package]
name         = "my-lib"
version      = "0.4.2"
edition      = "2024"
rust-version = "1.85"                 # MSRV. Bumping this breaks builds for pinned users.
license      = "MIT OR Apache-2.0"    # The Rust ecosystem default; permissive dual licence.
description  = "One line — this is what crates.io search shows."
repository   = "https://github.com/you/my-lib"
documentation = "https://docs.rs/my-lib"
categories   = ["data-structures"]    # Must match crates.io's fixed list.
keywords     = ["parser", "zero-copy"]
readme       = "README.md"
exclude      = ["/benches", "/fuzz", "/.github", "/testdata"]  # Keep the published tarball small.

[features]
default = []                          # Default to NOTHING. Every default feature is a cost
                                      # every consumer pays whether they use it or not.
serde  = ["dep:serde"]                # Optional dependency, gated behind a feature of the same name.
std    = []

[dependencies]
serde = { version = "1", optional = true, default-features = false }

[package.metadata.docs.rs]
all-features = true                   # docs.rs builds with every feature so gated items appear.
rustdoc-args = ["--cfg", "docsrs"]     # Pairs with #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
```

### Binary `Cargo.toml` — the profiles that matter

```toml
[profile.release]
opt-level        = 3        # Default. "z"/"s" optimize for size instead — measurably slower.
lto              = "fat"    # Cross-crate inlining. Costs build time; shrinks and can speed up.
codegen-units    = 1        # One unit = best optimization, no parallelism within the crate.
panic            = "abort"  # No unwinding: smaller, slightly faster. See pitfalls before setting.
strip            = "symbols"# Drop symbols from the shipped artifact.

[profile.dist]              # Alternative: keep `release` fast to build, add a slow "ship" profile.
inherits         = "release"
lto              = "fat"
codegen-units    = 1

[profile.profiling]         # Release speed WITH symbols — what you profile and flamegraph.
inherits         = "release"
debug            = true
strip            = "none"
```

## The Workflow

### Publishing a library

1. `cargo semver-checks check-release` — mechanically detects API breakage against the last published version. This is the single highest-value tool here; humans miss breakage constantly.
2. `cargo test --all-features` and `cargo test --no-default-features` — feature combinations are where library builds break for consumers.
3. `cargo doc --all-features --no-deps --open` — read the rendered docs; they are the product.
4. `cargo package --list` — see exactly what will be uploaded. Look for accidentally included test data or huge fixtures.
5. `cargo publish --dry-run` — builds from the packaged tarball, catching "works in my repo, missing from the package" (a file excluded but still `include!`d).
6. Update `CHANGELOG.md`, tag the commit, `cargo publish`.
7. Verify docs.rs built successfully — a docs.rs build failure is invisible until someone complains.

### Releasing a binary

1. Decide the profile (below) and record the choice with its reason.
2. Build per target, from a **clean checkout at a tag** — not from your working tree.
3. Embed provenance: version, git SHA, build date, so a running binary can identify itself (`--version` should print the SHA).
4. Verify it runs on the *deployment* target, not just the build host — glibc version differences are the classic Linux failure; `x86_64-unknown-linux-musl` gives a static binary that sidesteps it.
5. Publish artifacts with checksums.

## Measured Effects

Real project (a 200k-line-processing binary using `regex`), this machine, clean builds. Runtime is the program's own reported work time:

| Profile | Binary size | Clean build | Runtime | vs `release` |
| --- | --- | --- | --- | --- |
| `dev` (default) | 5953 K | 3.0 s | **593.9 ms** | **13.0× slower** |
| `release` | 2163 K | 5.5 s | 45.7 ms | 1.00× |
| `release` + `lto="thin"` | 2170 K | 6.8 s | 46.5 ms | ~same |
| `release` + `lto="fat"`, `cgu=1` | 1632 K | 13.2 s | 45.4 ms | ~same, **25% smaller** |
| … + `panic="abort"` | 1487 K | 12.7 s | 43.2 ms | ~same, **31% smaller** |
| … + `strip="symbols"` | 1297 K | 12.4 s | 41.3 ms | ~same, **40% smaller** |
| `opt-level="z"` + all of the above | **1058 K** | 9.6 s | **91.2 ms** | **2.0× slower**, 51% smaller |

What this actually says:

- **`dev` → `release` is the only order-of-magnitude knob.** 13×. Everything else is percentages. Never benchmark, profile, or judge performance in a debug build.
- **LTO and `codegen-units=1` bought size, not speed, here** — 25% smaller for 2.4× the build time, with runtime inside noise. On this workload the hot code is inside `regex`, already optimized within its own crate. LTO's speed benefit grows with cross-crate generic and inlining opportunities, so **this result does not generalize** — measure your own binary.
- **Thin LTO did nothing** for this crate (+24% build time, no size or speed change). It's the better default for large workspaces where fat LTO's build cost is prohibitive, but it earns nothing on a small dependency graph.
- **`opt-level="z"` is a real trade, not a free win**: half the size for half the speed. Correct for embedded or a cold-start-dominated function; wrong for a throughput service.
- **`strip` is free** — 13% smaller with no runtime cost. Just keep the unstripped binary (or a separate debug artifact) so you can symbolize a crash later.

## Pitfalls

### Pitfall: A breaking change published as a patch bump

- **What goes wrong:** `0.4.1 → 0.4.2` adds a field to a public struct, a variant to a public enum, or a method to a public trait. Consumers' builds break on `cargo update`, which they ran expecting a bugfix. Because versions are immutable, the only remedy is yanking and publishing a corrected version — and everyone who already updated has a broken build in the meantime.
- **Why it happens (the mechanism):** Rust's semver rules are subtler than they look, and several breaking changes are invisible at the call site you're editing. Adding an enum variant breaks exhaustive `match`. Adding a struct field breaks struct literals and exhaustive destructuring. Adding a trait method breaks implementors without a default. Adding an inherent method can break callers via ambiguity with a trait method. And under `0.x`, the **minor** position is the breaking one: `0.4 → 0.5` is the breaking bump, `0.4.1 → 0.4.2` is not.
- **How to handle it, and why that works:** Run `cargo-semver-checks` in CI on every PR — it diffs the actual API surface against the published version and names the breakage, which is a mechanical check where human review is unreliable. Mark enums `#[non_exhaustive]` and structs with private fields plus constructors *from the first release*, so adding variants and fields is non-breaking forever after. Give new trait methods default implementations.
- **Trade-offs of the fix:** `#[non_exhaustive]` forces every consumer to write a `_ =>` arm, which is friction they'll notice and some will resent — and it means they *lose* the compile error that would tell them a new variant exists. Private fields plus accessors is more code than a plain data struct. Both are worth it for a widely-used library and over-engineering for an internal crate with one consumer you control.

### Pitfall: `panic = "abort"` set without checking what depends on unwinding

- **What goes wrong:** `panic="abort"` is added for the 8% size win. Then: a server that used `catch_unwind` to contain a panic in one request handler now takes down the whole process; a test suite using `#[should_panic]` fails to build under a profile that inherits it; `criterion` benchmarks stop working.
- **Why it happens (the mechanism):** Aborting removes the unwinding machinery entirely — that's where the size saving comes from. Anything that relies on stack unwinding stops working, and the dependency is usually indirect (a framework catching panics per-task, a test harness) rather than something you wrote.
- **How to handle it, and why that works:** Set it only in the profile that ships (`[profile.release]` or a dedicated `dist` profile), never in `dev` or `test`. Then verify: does anything in the tree call `catch_unwind`? Does your async runtime rely on it to isolate tasks (tokio does, for task panics)? If a panic in one request must not kill the process, keep unwinding — 8% of binary size is not worth converting a contained failure into a total outage.
- **Trade-offs of the fix:** Keeping unwinding costs the size and a small amount of speed. Splitting profiles means the thing you test is not byte-identical to the thing you ship, which is its own (smaller) risk — mitigate by running the full test suite once against the shipping profile in CI.

### Pitfall: Default features that drag in the world

- **What goes wrong:** A library defaults `features = ["serde", "tokio", "tracing"]` because that's the common case. Every consumer — including a `no_std` embedded user who needs only the core parsing — compiles all of it. Build times and binary sizes inflate across the whole dependency graph, and the crate quietly becomes unusable in constrained environments.
- **Why it happens (the mechanism):** Default features feel like helpful ergonomics, and the author only ever measures their own build. Worse, feature unification means that if *any* crate in the graph enables a feature, it's enabled for everyone — so a consumer cannot opt out of your default just by setting `default-features = false` if some other crate in their tree didn't.
- **How to handle it, and why that works:** Default to `default = []` and make integrations opt-in. Test `--no-default-features` and `--all-features` in CI (`cargo hack --feature-powerset` for the combinations that matter). Depend on things with `default-features = false` yourself so you don't force *their* defaults downstream.
- **Trade-offs of the fix:** Zero defaults means every consumer writes a feature list to get anything useful, and your README has to explain it — a real ergonomic cost that generates issues from people who expected it to just work. A middle ground is a small, genuinely-core default set with everything heavy opt-in.

### Pitfall: A binary that won't run on the deployment target

- **What goes wrong:** Built on a developer machine or a newer CI image, deployed to an older Linux host: `version 'GLIBC_2.34' not found`. Or built for `aarch64` and deployed to `x86_64`. Or it runs but a dynamically-linked OpenSSL is missing.
- **Why it happens (the mechanism):** Rust statically links Rust code but dynamically links the system C library by default, so the binary carries a *minimum* glibc requirement inherited from the build host. glibc is backward- but not forward-compatible: build on new, run on old, fail.
- **How to handle it, and why that works:** Build in a container matching (or older than) the deployment target, or target `x86_64-unknown-linux-musl` for a fully static binary with no libc dependency at all. For TLS, prefer `rustls` over `native-tls` to remove the OpenSSL linkage entirely. Then actually run the artifact on a target-matched image in CI — the only check that's conclusive.
- **Trade-offs of the fix:** musl's allocator is significantly slower than glibc's for allocation-heavy workloads (often enough to matter — measure, and consider swapping in `mimalloc`/`jemalloc`). Building in an old container slows CI and complicates toolchain management.

### Pitfall: An unreproducible release

- **What goes wrong:** A binary is behaving badly in production. You check out what you believe is the tag, rebuild, and get something that doesn't reproduce the bug — because the original was built from a dirty working tree, with a different toolchain, or with `Cargo.lock` regenerated so dependency versions differ.
- **Why it happens (the mechanism):** Nothing in the default workflow ties an artifact to its inputs. `cargo build` happily builds uncommitted changes, `cargo update` silently moves dependencies, and a floating toolchain moves under you. Absent an explicit record, "what shipped" is unknowable after the fact.
- **How to handle it, and why that works:** Commit `Cargo.lock` (yes, for libraries too — it pins *your* CI, and consumers ignore it entirely). Pin the toolchain with a `rust-toolchain.toml`. Build only from tagged, clean checkouts in CI. Embed the git SHA and toolchain version into the binary so `--version` reports exactly what's running. That last step is what turns a production mystery into a two-second check.
- **Trade-offs of the fix:** A committed lockfile means dependency updates arrive as deliberate PRs rather than automatically — more churn, though tools like Dependabot/Renovate absorb it, and the determinism is worth it. A pinned toolchain means you upgrade Rust explicitly, which is a chore that is also the point.

## Checklist

**Library, before `cargo publish`:**

- [ ] `cargo semver-checks check-release` passes, and the version bump matches what it says (remember: under `0.x`, minor is the breaking position)
- [ ] `cargo test --all-features` and `--no-default-features` both pass
- [ ] `cargo publish --dry-run` succeeds; `cargo package --list` contains nothing unexpected
- [ ] `default = []` unless a default is genuinely justified
- [ ] `rust-version` (MSRV) set and actually verified in CI
- [ ] Public enums `#[non_exhaustive]`; public structs have private fields or are deliberately frozen
- [ ] `license`, `description`, `repository`, `categories`, `keywords` filled in
- [ ] `[package.metadata.docs.rs] all-features = true`
- [ ] CHANGELOG updated; commit tagged
- [ ] After publishing: docs.rs build succeeded

**Binary, before shipping:**

- [ ] Profile chosen deliberately, with the reason recorded
- [ ] `panic = "abort"` audited against `catch_unwind` and runtime task isolation
- [ ] Built from a clean, tagged checkout in CI — never a working tree
- [ ] `Cargo.lock` committed; `rust-toolchain.toml` pins the toolchain
- [ ] Version + git SHA embedded and printed by `--version`
- [ ] Verified running on a target-matched image, not just the build host
- [ ] Unstripped binary or split debug info archived for symbolizing crashes
- [ ] `cargo deny check` (licences, advisories, duplicates) passes

## Tooling

| Need | Tool | Notes |
| --- | --- | --- |
| Detect API breakage | `cargo-semver-checks` | The highest-value tool for libraries; run in CI |
| Automate version/tag/publish | `cargo-release`, `release-plz` | `release-plz` derives versions from conventional commits |
| Feature-combination testing | `cargo-hack` | `--feature-powerset` catches broken feature combos |
| Licence/advisory/duplicate policy | `cargo-deny` | One config, fails CI on policy violations |
| Vulnerability scan | `cargo-audit` | RustSec advisory database |
| Cross-compilation | `cross` | Container-based; solves the glibc problem |
| Binary distribution | `cargo-dist` | Generates release workflows, installers, checksums |
| Binary size analysis | `cargo-bloat`, `twiggy` | Find *what* is large before optimizing for size |
| Dependency graph review | `cargo-tree`, `cargo-udeps` | `-d` shows duplicate versions; `udeps` finds unused deps |
| Unused/outdated deps | `cargo-machete`, `cargo-outdated` | Keep the tax down |

## Open Questions

- LTO bought size but not speed on this binary. Which workload shape *does* show a runtime win from fat LTO — heavy cross-crate generics? Measure on a workspace with several first-party crates.
- musl vs glibc allocator cost for an allocation-heavy Rust service on this hardware — how much does swapping in `mimalloc` recover?
- Does `opt-level="s"` sit usefully between `3` and `z`, or is it dominated by one of them?
- `codegen-units=1` vs `16` at equal LTO setting: how much of the 25% size win came from each?
- Practical MSRV policy: does "latest stable minus N releases" cause real friction, and what do widely-used crates actually do?

## References

- [The Cargo Book — Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) — every knob in the table above, with defaults.
- [SemVer Compatibility (Cargo Book)](https://doc.rust-lang.org/cargo/reference/semver.html) — the definitive list of what is and isn't breaking in Rust. Read it once end-to-end; several entries are surprising.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — the checklist for public API design; the naming and future-proofing sections earn their length.
- [`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks) — mechanical breakage detection.
- [`cargo-dist`](https://opensource.axo.dev/cargo-dist/) — binary release automation.
- Related in this repo: [Rust learning notes](learning.md) (API design practices this builds on), [testing](testing.md), [benchmarking](benchmarking.md) (which depends on the `release` profile above), [OpenTofu runbook](../../oss-tools/opentofu/runbook.md) (the same clean-checkout, pinned-version discipline for infrastructure).
