# ripgrep & grep — Learning Notes

## Mental Model

**Searching a codebase is dominated by the files you *don't* read.** Classic `grep -r` reads everything — `.git/`, `target/`, `node_modules/`, minified bundles, binaries — and then matches. ripgrep's central design decision is to *skip* most of that by default, which is why it's typically an order of magnitude faster on a real repository while doing the same regex work.

The three mechanisms, in order of how much they contribute:

1. **Skip files entirely** — respect `.gitignore`, skip hidden files and binaries. On a typical repo this eliminates most of the bytes before any matching happens.
2. **Reject lines without a full regex match** — a SIMD literal prefilter (`memchr`) finds candidate positions 16–32 bytes per instruction, and the regex engine only runs where a required literal appears. This is exactly the finding measured in [string matching](../../data-structures-and-algorithms/string-matching/learning.md): std's `find` beat hand-rolled KMP by 2.6–9.0× because of this prefilter, and `rg` is built by the same author on the same machinery.
3. **Parallelism** — one thread per file, work-stealing across a directory walk.

The second model worth holding: **`rg` uses a finite-automaton regex engine (the `regex` crate) with linear-time guarantees**, while `grep -P` and most other tools use PCRE2 backtracking. That's not a performance detail — it's a **denial-of-service boundary**. A pattern like `(a+)+b` against a long line of `a`s takes exponential time in a backtracking engine and linear time in `rg`'s default engine. If a pattern comes from user input, the engine choice is a security decision.

The practical consequence of mechanism 1 is the thing that surprises people: **`rg` not finding something is usually correct behaviour, not a bug.** It skipped a file your `.gitignore` excludes. `rg -uuu` disables all of it and makes `rg` behave like `grep -r`.

## The Model in Detail

### The `-u` ladder

- **What it is:** Each `-u` removes one layer of filtering: `-u` = don't respect ignore files; `-uu` = also search hidden files; `-uuu` = also search binary files.
- **Why it matters in practice:** This single flag explains most "rg didn't find it" reports. Reach for `-uu` when searching for something in a dotfile, and `-uuu` when you genuinely need to grep a build artifact.

### Regex engine choice

- **What it is:** `rg` defaults to the Rust `regex` crate — a hybrid NFA/DFA with **guaranteed linear time**, but no backreferences and no lookaround. `rg -P` switches to PCRE2, which has both features and exponential worst-case behaviour.
- **Why it matters in practice:** You only need `-P` for lookaround (`(?<=foo)bar`) or backreferences (`(\w+)\s+\1`). Everything else is expressible in the default engine and is safer and usually faster. If patterns are ever attacker-supplied, `-P` is a ReDoS vector.

### Literal extraction is why it's fast

- **What it is:** Before running the automaton, the engine extracts required literal strings from the pattern and uses SIMD (`memchr`/Teddy) to find candidate positions.
- **Why it matters in practice:** `rg 'fn parse_\w+'` is fast because `fn parse_` is a required literal. `rg '\w+parse'` is much slower because there's no literal prefix to anchor on. **Putting a literal in your pattern is the highest-leverage optimization**, and it explains why anchoring or adding context often makes a search faster rather than slower.

### grep is still the portable answer

- **What it is:** POSIX `grep` exists everywhere — containers, minimal images, remote hosts, CI runners, `busybox`.
- **Why it matters in practice:** Scripts that must run anywhere should use `grep`. `rg` is an interactive and developer-machine tool; assuming it in a shell script that runs in a scratch container is a portability bug.

## Portability & Variants

**This machine: ripgrep 15.2.0, and `grep` is actually `ugrep 7.5.0`** (a drop-in replacement installed via Homebrew), not GNU or BSD grep. That's worth knowing because it means `grep` here supports GNU-style long options that BSD grep does not — **scripts written and tested here may fail on a stock macOS or Alpine host.**

| Variant | Notes |
| --- | --- |
| **GNU grep** | Linux default. `-P` (PCRE), `-r`, `--include`, `-z` |
| **BSD grep** | Stock macOS. No `-P`, different `-r` semantics, fewer long options |
| **ugrep** | GNU-compatible, faster, extra features (`-z` archives, fuzzy) — what's installed here |
| **busybox grep** | Minimal containers. BRE only, very few flags |
| **ripgrep** | Not POSIX; ignore-aware; linear-time regex by default |

**Regex flavour is the other portability trap:** `grep` uses POSIX **BRE** by default, where `+`, `?`, `|`, `(` are *literals* and must be escaped (`\+`). `grep -E` (ERE) and `rg` treat them as metacharacters. This is why the same pattern behaves differently between tools, and why `grep -E` should be the default habit.

## Pitfalls in Depth

### Pitfall: "ripgrep didn't find it"

- **What goes wrong:** A string is definitely in the repository, and `rg` reports nothing. Time is lost doubting the pattern.
- **Why it happens (the mechanism):** `rg` skips files matched by `.gitignore`/`.ignore`/`.rgignore`, hidden files (dotfiles and dot-directories), and files it detects as binary — by design, because that's where the speed comes from. A match inside `target/`, `.env`, or `node_modules/` is invisible by default.
- **How to handle it, and why that works:** Escalate with the `-u` ladder: `rg -uu pattern` (hidden files included) then `rg -uuu pattern` (binaries too). Use `rg --files | rg name` to confirm whether the file is even being considered, and `rg --debug` to see which ignore rule excluded it.
- **Trade-offs of the fix:** `-uuu` searches everything including `.git/` objects and build output, which is slow and noisy — it's a diagnostic, not a default. A better long-term fix is a `.ignore` file that differs from `.gitignore` where the two intents diverge.

### Pitfall: Shell expansion eating the pattern

- **What goes wrong:** `rg *.rs` or `grep foo *.txt` behaves unpredictably; patterns containing `*`, `?`, `$`, or `!` are mangled or expand to filenames before the tool sees them.
- **Why it happens (the mechanism):** The shell expands globs and variables *before* invoking the command. An unquoted `*.rs` becomes a list of filenames, so it's interpreted as a pattern plus paths. In double quotes, `$` and backticks still expand.
- **How to handle it, and why that works:** **Always single-quote patterns**: `rg 'fn \w+\(' `. Use `-e` for patterns beginning with `-` (`rg -e '-foo'`) and `--` to end option parsing. For file filtering use the tool's own flags — `rg -g '*.rs'` or `--type rust` — not shell globs, so the tool does the walking and the ignore rules still apply.
- **Trade-offs of the fix:** Single quotes prevent variable interpolation, so building a pattern from a variable needs care: `rg "$var"` is correct but then `$var` must be regex-escaped if it's literal text — which is what `rg -F` (fixed string) is for.

### Pitfall: Catastrophic backtracking with `-P`

- **What goes wrong:** A pattern like `^(\w+\s?)+$` runs instantly on most lines and hangs for minutes on one long line. In a CI job or a service that accepts user patterns, this is a denial of service.
- **Why it happens (the mechanism):** PCRE2 backtracks: nested quantifiers create exponentially many ways to split the input, and a failing match explores all of them. `rg`'s default engine is a finite automaton with no backtracking, so it cannot exhibit this — the guarantee is structural, not a matter of tuning.
- **How to handle it, and why that works:** Don't use `-P` unless you specifically need lookaround or backreferences, and never with an untrusted pattern. Most `-P` uses are habit; `(?<=foo)bar` can usually be rewritten as `foo\Kbar`-free alternatives like matching `foobar` and post-processing, or using `-o` with a capture-aware tool.
- **Trade-offs of the fix:** Lookaround genuinely is more expressive, and rewriting without it can be clumsy. When you must use `-P`, set a timeout around the invocation and never let the pattern come from outside.

### Pitfall: Assuming line-oriented tools see multi-line structure

- **What goes wrong:** Searching for a function signature that spans two lines, or a JSON key whose value is on the next line, returns nothing — because both `grep` and `rg` are line-oriented by default.
- **Why it happens (the mechanism):** The default unit is a line, so `.` doesn't match `\n` and the pattern is applied per line. This is what makes streaming and memory use predictable, so it's a deliberate design choice rather than a limitation.
- **How to handle it, and why that works:** `rg -U` (multiline) makes patterns span lines, and `rg -U --multiline-dotall` additionally lets `.` match newlines. For *structured* data, stop using a regex: `jq` for JSON, `yq` for YAML, and **`ast-grep`** for source code, which matches on syntax tree shape rather than text and therefore doesn't care about formatting.
- **Trade-offs of the fix:** Multiline mode is slower and can match enormous spans if the pattern is loose. Structural tools require learning another query language, but for "find all calls to X with a literal second argument" they're the only thing that works reliably.

### Pitfall: Piping into `xargs` and breaking on odd filenames

- **What goes wrong:** `rg -l pattern | xargs sed -i ...` fails or corrupts files when a path contains spaces, quotes, or newlines.
- **Why it happens (the mechanism):** `xargs` splits on whitespace by default, so `my file.rs` becomes two arguments. Filenames may legally contain almost anything including newlines, so any line-based pipeline is fragile.
- **How to handle it, and why that works:** Use NUL separation end to end: `rg -l --null pattern | xargs -0 sed -i ''`. NUL is the one byte that cannot appear in a filename, so the framing is unambiguous. Better still, use the tool's built-in replacement (`rg --replace` for preview, `sd` for in-place) and skip the pipeline.
- **Trade-offs of the fix:** `-0`/`--null` is slightly more to type and not supported by every downstream tool. `find -exec ... +` is the portable alternative when `xargs -0` isn't available.

## Open Questions

- How much of ripgrep's advantage over `ugrep` (installed here as `grep`) is the ignore-awareness versus the regex engine? Worth measuring on this repo.
- Does `rg --pre` (preprocessor) make searching PDFs/archives practical, or is `ugrep -z` better for that?
- `ast-grep` vs `rg` for large-scale Rust refactors — where's the crossover in reliability versus setup cost?

## References

- [ripgrep user guide](https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md) — genuinely worth reading end to end
- Andrew Gallant, ["ripgrep is faster than…"](https://blog.burntsushi.net/ripgrep/) — the definitive benchmark-and-why writeup
- [`regex` crate docs](https://docs.rs/regex/) — the syntax `rg` accepts, and the linear-time guarantee
- Related in this repo: [String Matching](../../data-structures-and-algorithms/string-matching/learning.md) (the SIMD prefilter, measured), [sed & text processing](../sed-and-text-processing/learning.md), [Regular Expressions](../regular-expressions/learning.md) (engine internals and flavor differences in depth)
