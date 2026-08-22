# Makefile — Quick Reference

## Quick Facts

- **Does:** Rebuilds stale targets from prerequisites via timestamp comparison; runs a shell recipe per out-of-date target.
- **Reach for it when:** You need a dependency graph over files (build artifacts) or a simple, dependency-free task runner already available on every Unix box.
- **Don't use it for:** Complex cross-language build graphs at scale (prefer Bazel/Buck), or pure task-running with no file semantics if the timestamp model keeps causing bugs (prefer `just`).

## Flags That Matter

| Flag | Does |
| --- | --- |
| `-n` / `--dry-run` | Print recipes without running them |
| `-j[N]` | Run up to N recipes in parallel (unlimited if N omitted) |
| `-k` | Keep going after an error in unrelated targets |
| `-B` / `--always-make` | Rebuild everything, ignore timestamps |
| `-C dir` | `cd dir` before reading the Makefile |
| `-f file` | Use `file` instead of `Makefile`/`makefile` |
| `-d` | Full dependency-resolution debug trace |
| `--debug=b` | Basic debug info (why each target ran/didn't) |
| `-e` | Environment variables override Makefile assignments |
| `-s` / `--silent` | Don't echo recipe lines before running them |
| `-p` | Dump the full database of rules/variables (incl. built-in) |
| `-t` / `--touch` | Mark targets up to date without running recipes |

## Syntax

```make
# Variable assignment
SIMPLE  := value          # evaluated once, immediately
LAZY    =  $(SIMPLE)ish   # evaluated every time it's used
APPEND  += more            # append to either kind

# Basic rule
target: prereq1 prereq2
	recipe-line-1
	recipe-line-2   # each line = new subshell

# Phony (non-file) target — always required for task-style targets
.PHONY: all clean test

# Pattern rule
%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

# Order-only prerequisite (must exist, doesn't trigger rebuild if newer)
target: normal-prereq | dir-that-must-exist

# Automatic variables (recipe context only)
$@   # target name
$<   # first prerequisite
$^   # all prerequisites, deduped
$*   # stem matched by a pattern rule
$?   # prerequisites newer than target

# Conditionals
ifeq ($(OS),Darwin)
CC := clang
else
CC := gcc
endif

# Include another makefile
include common.mk
-include optional.mk   # '-' suppresses error if missing

# Multi-line recipe as one shell (GNU Make >=3.82 only)
.ONESHELL:
target:
	cd subdir
	./run.sh   # same shell as the cd above

# Default goal (first target in file, or override explicitly)
.DEFAULT_GOAL := build
```

## Gotchas

| Gotcha | Fix |
| --- | --- |
| `*** missing separator. Stop.` | Recipe line indentation must be a literal tab, not spaces |
| Phony target silently no-ops | Declare it in `.PHONY:` — a same-named file/dir will otherwise "satisfy" it |
| `-j` build is flaky, serial isn't | Missing prerequisite edge between two targets; add it or use `|` order-only prereq |
| `cd` on one recipe line doesn't affect the next | Each line is a new subshell — chain with `&&`, or use `.ONESHELL:` (GNU Make ≥3.82) |
| Recursive `$(MAKE)` subdir builds miss cross-dir staleness | Flatten to one Makefile / `include` fragments so there's one dependency graph |
| `.ONESHELL:` does nothing | Stock macOS ships GNU Make 3.81; needs ≥3.82 — install `gmake` via Homebrew |
| Variable expands to something unexpected later in the file | Check `=` (recursive, lazy) vs `:=` (simple, immediate) |
| "It worked yesterday, now everything rebuilds" | Clock skew (Docker layer, NFS, `git clone` timestamps) — Make trusts mtimes only |

## Key References

- [learning.md](learning.md) — mental model and pitfall mechanisms
- [recipes.md](recipes.md) — task-oriented commands
- GNU Make Manual: https://www.gnu.org/software/make/manual/make.html
