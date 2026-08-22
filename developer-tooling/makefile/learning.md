# Makefile — Learning Notes

## Mental Model

**Make is a dependency graph evaluator over files, not a task runner.** Every rule declares a node (the target), its edges (prerequisites), and the edge-crossing action (the recipe). Make's only job is: is the target older than any prerequisite, or missing? If so, run the recipe; otherwise skip it. Everything confusing about Make — targets that "don't rerun," phony targets, parallel build races — falls out of taking that literally: **Make reasons about files and timestamps, not about intent.**

```
target: prerequisite1 prerequisite2
	recipe line 1
	recipe line 2
```

- A **target** is (almost always) a filename Make expects the recipe to produce.
- A **prerequisite** is a filename that must exist and be up to date before the recipe runs.
- The **recipe** is shell, run in a fresh subshell per line, only when the target is missing or older than any prerequisite.

Once a target is up to date, Make treats "run the task" and "the file already reflects the task" as the same question — which is exactly why `make test` or `make build` used as task names (rather than filenames) misbehave until declared `.PHONY`.

## The Model in Detail

### Timestamps are the only signal

- **What it is:** Make compares the modification time of the target file to each prerequisite's. Newer prerequisite → stale target → rerun. No content hashing, no semantic understanding of what changed.
- **Why it matters in practice:** Touching a file without changing it (`touch file.c`) forces a rebuild; restoring an old file with the same content but an old timestamp can make Make think it's still up to date. Clock skew (common in Docker builds, NFS mounts, `git clone` re-setting timestamps) causes "changes are being ignored" or "everything rebuilds every time" bugs that have nothing to do with the Makefile's logic.

### Phony targets: names that aren't files

- **What it is:** `.PHONY: test` tells Make that `test` doesn't name a file — always treat it as out of date, i.e. always run its recipe.
- **Why it matters in practice:** Without `.PHONY`, if a file literally named `test` or `clean` ever appears in the directory (a common test-binary output name), Make sees an up-to-date "target" and silently no-ops the recipe. This is the single most common Make footgun for task-runner-style Makefiles (`make build`, `make lint`, `make deploy`) — declare every non-file target phony, no exceptions.

### Prerequisites build the DAG; order within a recipe doesn't

- **What it is:** Make computes a full dependency graph from all rules, then topologically schedules recipes. Prerequisite order in a rule only affects the `$^`/`$<` variable expansion, not build order — build order is entirely determined by the graph.
- **Why it matters in practice:** With `-j` (parallel builds), any two targets without a dependency edge between them may run concurrently, in either order. A Makefile that "happens to work" serially because rule A is written above rule B will break under `-j` if B actually depends on state A produces as a side effect but doesn't declare as a prerequisite.

### Implicit rules and automatic variables

- **What it is:** Make ships built-in pattern rules (`%.o: %.c` compiles via `$(CC) $(CFLAGS) -c`) and automatic variables that only have meaning inside a recipe: `$@` (the target), `$<` (first prerequisite), `$^` (all prerequisites, deduplicated), `$*` (the stem a pattern rule matched).
- **Why it matters in practice:** Implicit rules are why a bare `make foo.o` can work with no explicit rule for it at all — and why an unexpected compiler flag or wrong compiler shows up "from nowhere" (it's `CC`/`CFLAGS` inherited from the built-in rule or the environment). `$@`/`$<`/`$^` are how idiomatic Makefiles avoid repeating the target/prerequisite names in the recipe, which matters for pattern rules where those names vary per invocation.

### Variables: recursive (`=`) vs simple (`:=`)

- **What it is:** `FOO = $(BAR)` is **recursively expanded** — `$(BAR)` is re-evaluated textually every time `FOO` is used, lazily. `FOO := $(BAR)` is **simply expanded** — `$(BAR)` is evaluated once, immediately, at the point of definition, like a normal variable assignment.
- **Why it matters in practice:** Recursive expansion enables forward references (`FOO = $(BAR)` where `BAR` is defined later in the file) but also causes surprises like infinite loops (`FOO = $(FOO) x`) or a variable silently picking up a different value than the one that existed when it was "assigned," because it was never actually evaluated until use. Modern Makefiles default to `:=` unless the lazy, forward-referencing behavior of `=` is specifically wanted.

### Recipe execution: one subshell per line

- **What it is:** Each recipe line runs in its own `/bin/sh` subprocess, not a persistent shell. `cd`, exported variables, and shell state do not carry over to the next line.
- **Why it matters in practice:** `cd subdir; make` on one line then a bare command on the next silently runs in the original directory, not `subdir` — a classic bug. Fix is either a single line joined with `&&` / `;`, or `.ONESHELL:` (GNU Make extension) to run the whole recipe in one persistent shell invocation.

## Portability & Variants

**This machine has GNU Make 3.81** (the last GPLv2 release Apple ships, frozen since GPLv3 licensing concerns — it predates `.ONESHELL:`, which needs GNU Make ≥3.82, and lacks `$(file ...)`, `$(let ...)`, and other GNU Make 4.x functions). This has concrete consequences:

- **`.ONESHELL:` does not work on stock macOS Make.** Either install GNU Make 4.x via Homebrew (`brew install make`, giving `gmake`) or keep multi-line recipes chained with `&&`/`\`.
- **`$(file >...)` and `$(shell ...)` output tricks from GNU Make 4.x docs won't run.** Check `make --version` before copying a "modern GNU Make" recipe from a blog post.
- **BSD Make (`/usr/bin/bmake` conceptually, though macOS's `/usr/bin/make` is GNU 3.81, not BSD Make)** has different syntax entirely (`.include`, `.if`, `!=` for shell-out) — relevant if a Makefile is meant to be portable to actual BSD/illumos systems, not relevant on stock macOS.
- **POSIX Make** (the lowest common denominator, `#!/usr/bin/env -S make -f`) supports none of: `ifeq`/`ifdef`, `$(shell ...)`, pattern rules with multiple targets, or `.PHONY` extensions beyond the basic form — write to this subset only when a Makefile must run unmodified on minimal/embedded systems.
- Prefer `gmake` (Homebrew GNU Make ≥4.x) for anything using `.ONESHELL:`, `$(file ...)`, grouped targets (`a b: c`, GNU 4.3+), or `!=` shell assignment.

## Pitfalls in Depth

### Pitfall: task-like target silently swallowed by a same-named file

- **What goes wrong:** `make clean` does nothing, no error, no output.
- **Why it happens (the mechanism):** A file or directory named `clean` exists in the working directory (common: a `clean/` build-output folder) and its mtime is newer than nothing needs comparing against, so Make considers the "target" already up to date.
- **How to handle it, and why that works:** Declare every task-style target phony up front: `.PHONY: all clean test build install`. This unconditionally marks them as always-stale, bypassing the file-timestamp check entirely — the fix works because it removes the file check from the equation rather than trying to out-timestamp it.
- **Trade-offs of the fix:** None — there's no legitimate reason for a task target not to be phony. Missing `.PHONY` declarations should be treated as a bug, not a style choice.

### Pitfall: parallel build (`-j`) breaks a Makefile that "worked" serially

- **What goes wrong:** `make -j8` produces flaky failures, corrupted output, or nondeterministic build order — the exact same Makefile passes with plain `make`.
- **Why it happens (the mechanism):** Two recipes with no declared dependency edge between them are scheduling-order-independent by design. If recipe B actually reads a file recipe A produces, but that file isn't listed as B's prerequisite, serial execution masks the bug by accident (A always happened to run first); parallel execution exposes it because Make is free to run them concurrently or in reverse order.
- **How to handle it, and why that works:** Add the missing prerequisite edge so the graph reflects the real dependency, or use order-only prerequisites (`target: normal-prereqs | order-only-prereqs`) when B only needs A's *side effect* (e.g., a directory existing) rather than a rebuild trigger. This works because it makes the implicit dependency explicit to the scheduler, which is the only thing Make actually reasons about.
- **Trade-offs of the fix:** Requires auditing every recipe for undeclared file reads/writes — tedious in a large legacy Makefile, but the alternative is `-j1` forever, which forfeits Make's main practical benefit (fast incremental parallel builds).

### Pitfall: recursive Make loses incrementality ("recursive Make considered harmful")

- **What goes wrong:** A top-level Makefile that does `cd subdir && $(MAKE)` in every subdirectory ends up rebuilding far more than necessary, and misses cross-directory dependencies entirely.
- **Why it happens (the mechanism):** Each recursive `$(MAKE)` invocation gets its own independent dependency graph, scoped to that subdirectory. The parent Make has no visibility into what the child rebuilt, so it can't correctly decide whether *its* targets (which may depend on the child's outputs) are stale — it either always reruns the child (safe but slow) or trusts a subdirectory timestamp proxy that misses real changes.
- **How to handle it, and why that works:** Prefer a single flat Makefile (or `include`d fragments sharing one `make` invocation) so one dependency graph spans the whole project — Make's staleness check is then correct by construction because every file is a node in the same graph. This is Peter Miller's well-known 1997 argument ("Recursive Make Considered Harmful").
- **Trade-offs of the fix:** A flat Makefile for a large multi-component project can get unwieldy and couples build logic across components that might want independent build systems; `include` with per-directory `.mk` fragments and namespaced variables is the usual middle ground.

### Pitfall: tabs vs spaces

- **What goes wrong:** `*** missing separator. Stop.`
- **Why it happens (the mechanism):** Make's recipe-line parser requires a literal tab character as the first character of every recipe line — it's how Make distinguishes "this line is shell to execute" from "this line is a variable/rule definition." An editor that auto-converts tabs to spaces silently breaks every recipe it touches.
- **How to handle it, and why that works:** Configure the editor to preserve literal tabs in Makefiles specifically (most editors detect the filename), or use GNU Make's `.RECIPEPREFIX` (GNU Make ≥3.82, so not on the stock macOS 3.81) to redefine the recipe-line marker to a non-tab character. This works because it addresses the actual parser requirement rather than fighting the editor repeatedly.
- **Trade-offs of the fix:** `.RECIPEPREFIX` is non-obvious to any reader unfamiliar with the file, so it trades one class of pain (editor whitespace mangling) for another (reader confusion) — usually not worth it versus just fixing editor config.

## Open Questions

- Whether this project's Rust-first tooling should move to `just` (a `make`-inspired command runner with none of the file-timestamp semantics) for pure task-runner use cases, keeping `make` reserved for actual build-artifact dependency graphs.

## References

- GNU Make Manual: https://www.gnu.org/software/make/manual/make.html
- Peter Miller, "Recursive Make Considered Harmful" (1997)
- POSIX `make` specification (IEEE Std 1003.1)
