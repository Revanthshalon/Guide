# Makefile — Recipes

> Concepts are in [learning.md](learning.md); flag lookup is in [reference.md](reference.md).

## Writing a task-runner Makefile

### I want to define tasks that always run (build/test/clean, not real files)

```make
.PHONY: all build test clean

all: build test

build:
	cargo build --release

test:
	cargo test

clean:
	rm -rf target/
```

`.PHONY` stops Make from skipping the recipe if a file with that name ever exists in the directory.

### I want a target to depend on other tasks running first

```make
.PHONY: ci
ci: lint test build
```

`ci` has no recipe of its own — it exists purely to sequence its prerequisites. Runs `lint`, then `test`, then `build`, each only if not already satisfied (irrelevant here since all three are phony, so all three always run).

### I want a default target when someone just runs `make`

```make
.DEFAULT_GOAL := build
```

Without this, Make uses the *first* target defined in the file as the default — an easy footgun if `all` isn't written first.

## Working with real file dependencies

### I want a target that only rebuilds when its source changes

```make
app: main.c utils.c
	$(CC) -o app main.c utils.c
```

`app` only rebuilds if `main.c`, `utils.c`, or `app` itself (if missing) is newer/absent — this is Make's actual value over a plain task runner.

### I want a pattern rule instead of one rule per file

```make
%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@
```

`$<` = the matched `.c` file, `$@` = the `.o` target. One rule compiles every `.c` file in the project.

### I want a directory to exist before a target writes into it, without forcing a rebuild every time

```make
bin/app: main.o | bin
	$(CC) -o $@ $^

bin:
	mkdir -p bin
```

The `|` marks `bin` as *order-only* — Make ensures it exists first but a newer `bin/` mtime (e.g. from an unrelated file added to it) won't mark `bin/app` stale.

## Debugging a Makefile

### I want to see what would run without running it

```sh
make -n target
```

### I want to know *why* Make thinks a target is (or isn't) stale

```sh
make --debug=b target
```

### I want to force a full rebuild ignoring timestamps

```sh
make -B target
```

### I want to see every variable and rule Make actually resolved, including implicit/built-in ones

```sh
make -p | less
```

Useful when a build picks up an unexpected `CC`/`CFLAGS` from a built-in implicit rule rather than the Makefile.

### I want parallel builds but need to check they're safe first

```sh
make -j1 target   # baseline: known-correct serial run
make -j8 target    # compare output/behavior
```

If `-j8` misbehaves and `-j1` doesn't, the Makefile has an undeclared dependency edge somewhere — see [learning.md](learning.md#pitfall-parallel-build--j-breaks-a-makefile-that-worked-serially).

## Recovery — "I did something wrong"

Make itself is non-destructive to source — it only ever *runs recipes*, and recipes are just shell, so the actual damage (if any) comes from what the recipe did (e.g. a `clean:` target with `rm -rf` that matched more than intended), not from Make's own bookkeeping.

- **A recipe half-ran and left a target file in a broken/partial state:** delete the target file and rerun — Make has no notion of a "committed" build, so there's nothing to roll back except the file itself: `rm -f target && make target`.
- **`make clean` deleted something it shouldn't have:** check the `clean:` recipe itself; a too-broad `rm -rf $(BUILD_DIR)` where `$(BUILD_DIR)` evaluated to `.` or empty is the classic cause — this is a shell/variable bug, not a Make bug, and there is no Make-level undo.
- **A stale build artifact keeps getting reused when it shouldn't:** `make -B target` forces a rebuild regardless of timestamps; if the underlying cause is clock skew, fix the clock/mount rather than relying on `-B` as a permanent workaround.
- **`.PHONY` was missing and a task silently didn't run:** add the target to `.PHONY:` — no data is lost, the "damage" is just a build that appeared to succeed without doing anything.

## References

- [learning.md](learning.md)
- [reference.md](reference.md)
- GNU Make Manual: https://www.gnu.org/software/make/manual/make.html
