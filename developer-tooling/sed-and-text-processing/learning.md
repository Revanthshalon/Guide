# sed & Text Processing — Learning Notes

## Mental Model

**`sed` is a loop you don't write.** It reads a line into the *pattern space*, runs your script against it, prints the pattern space, and repeats. Everything confusing about `sed` follows from that implicit loop:

```
for each line:
    load line into pattern space (without the newline)
    apply every command in the script, in order
    print pattern space (unless -n)
```

So `sed 's/a/b/'` is really "for every line, substitute the first `a` with `b`, then print". The `-n` flag suppresses the automatic print, which is why `sed -n '5p'` prints line 5 exactly once instead of printing every line and line 5 twice.

The second idea: **`sed` commands are `[address]command`**, where the address selects which lines the command applies to — a line number, a range, a regex, or nothing (meaning every line). `sed '/^#/d'` is "on lines matching `^#`, delete". Once you read commands that way, the syntax stops being cryptic.

The third, and the one that decides which tool to reach for:

> **`sed` is for line-local transformations. The moment you need memory across lines, fields, or arithmetic, you want `awk`. The moment you need structure, you want a parser.**

That boundary is the whole of "which tool". `sed` has a hold space and can do multi-line work, but a `sed` script that uses it is almost always harder to read than the three-line `awk` equivalent — and the `awk` version will still be there in six months.

## The Model in Detail

### Addresses select; commands act

- **What it is:** `sed '2,5s/foo/bar/'` — address `2,5`, command `s/foo/bar/`. Addresses can be `N`, `$` (last line), `/regex/`, `N,M`, `/start/,/end/`, `0~3` (every third, GNU), and `!` negates.
- **Why it matters in practice:** Most tasks that look like they need a script are one address plus one command. `sed -n '/BEGIN/,/END/p'` extracts a block; `sed '/^$/d'` strips blank lines; `sed '$d'` drops the last line.

### The `s` command's flags carry the semantics

- **What it is:** `s/pattern/replacement/flags` — `g` (all occurrences on the line, not just the first), `N` (the Nth occurrence), `i` (case-insensitive, GNU), `p` (print, pairs with `-n`), `w file` (write matches).
- **Why it matters in practice:** **`s///` without `g` replaces once per line**, which is the single most common `sed` surprise. And the delimiter is arbitrary — `s|/usr/local|/opt|` avoids escaping every slash in a path, which is why `|` or `#` is standard for path substitutions.

### Regex flavour is BRE, and that's the trap

- **What it is:** `sed` uses POSIX **Basic** Regular Expressions by default, where `+`, `?`, `|`, `(`, `)`, `{`, `}` are **literal characters** and need backslashes to become metacharacters. `sed -E` (or `-r` on old GNU) switches to Extended, where they behave as expected.
- **Why it matters in practice:** `sed 's/a+/X/'` matches a literal `a+`, not "one or more a". This silently does nothing rather than erroring, so it looks like the pattern didn't match. **Habitually using `-E` removes an entire class of confusion.**

### The hold space (and why you probably shouldn't)

- **What it is:** A second buffer. `h`/`H` copy pattern→hold, `g`/`G` copy hold→pattern, `x` exchanges them, `N` appends the next line to the pattern space.
- **Why it matters in practice:** It's what makes `sed` Turing-complete and what makes `sed` scripts unreadable. If you're reaching for `h`/`G`/`N`, that's the signal to switch to `awk` or a real script. The classic `tac` implementation in `sed` (`sed -n '1!G;h;$p'`) is a good demonstration of exactly why not to.

### awk is the right tool more often than people think

- **What it is:** `awk 'pattern { action }'` over records (lines) split into fields (`$1`, `$2`, …, `$NF`), with variables, arrays, and arithmetic.
- **Why it matters in practice:** Anything involving *columns*, *sums*, *counting*, or *state across lines* is an awk one-liner and a sed nightmare. `awk '{s+=$3} END {print s}'` sums the third column. `awk '!seen[$0]++'` deduplicates while preserving order. Neither has a reasonable `sed` equivalent.

## Portability & Variants

**This machine has BSD sed** (macOS) — confirmed by `sed --version` failing, since BSD sed has no `--version`. This is the single biggest portability trap in shell scripting, and it bites in three specific places:

| Feature | GNU sed (Linux) | **BSD sed (macOS)** |
| --- | --- | --- |
| **In-place edit** | `sed -i 's/a/b/'` | **`sed -i '' 's/a/b/'`** — the empty arg is mandatory |
| Extended regex | `-r` or `-E` | `-E` only |
| Case conversion | `\U`, `\L`, `\u`, `\l` | **not supported** |
| `\+`, `\?`, `\|` in BRE | supported | **not supported** — use `-E` |
| `a`/`i`/`c` (append/insert) | `sed '2a text'` | needs a backslash-newline |
| Word boundary | `\b`, `\<`, `\>` | `[[:<:]]`, `[[:>:]]` |
| `-z` (NUL-separated) | yes | no |

**`sed -i` is the one that corrupts things.** On macOS, `sed -i 's/a/b/' file` interprets `s/a/b/` as the *backup suffix* and then treats `file` as the script — producing an error or, worse, unexpected behaviour. On Linux, `sed -i '' 's/a/b/' file` treats `''` as the script and creates a file named after the pattern.

Practical resolutions, in order of preference:

1. **Use [`sd`](https://github.com/chmln/sd)** — same job, one syntax everywhere, sane regex (Rust `regex`), no `-i` divergence.
2. Write to a temp file and move: `sed -E 's/a/b/' f > f.tmp && mv f.tmp f` — portable, and atomic on the same filesystem.
3. Detect: `sed --version >/dev/null 2>&1 && SED_I=(-i) || SED_I=(-i '')`.
4. Install GNU coreutils on macOS (`brew install gnu-sed`, then `gsed`) and use `gsed` explicitly in scripts.

## Pitfalls in Depth

### Pitfall: `sed -i` portability

- **What goes wrong:** A script tested on macOS runs on a Linux CI runner (or vice versa) and either errors, creates a stray file named `s/old/new/g`, or leaves `.bak` files everywhere. In the worst case a loop over many files corrupts them before anyone notices.
- **Why it happens (the mechanism):** BSD `sed -i` requires an argument for the backup suffix (empty string for no backup); GNU `sed -i` takes the suffix *attached* (`-i.bak`) and treats a following bare argument as the script. The two calling conventions are incompatible, and neither errors clearly on the other's form.
- **How to handle it, and why that works:** Prefer `sd`, which has one behaviour everywhere. If you must use `sed`, avoid `-i` entirely in scripts — redirect to a temp file and `mv`, which is portable *and* atomic (the original is never in a half-written state). Reserve `-i` for interactive one-offs where you can see the result.
- **Trade-offs of the fix:** The temp-file form is three tokens longer and needs care with permissions (`mv` preserves the temp file's mode, not the original's — use `cp -p` first if that matters). `sd` is another tool to install, which is fine on a dev machine and not in a minimal container.

### Pitfall: BRE vs ERE silently not matching

- **What goes wrong:** `sed 's/foo+/bar/'` doesn't replace anything, and there's no error. The same pattern works in `grep -E`, `rg`, and every programming language, so the assumption is that the input is wrong.
- **Why it happens (the mechanism):** In POSIX BRE, `+` is a literal character. `foo+` matches the four-character string `foo+`. The pattern is valid, it simply means something else — so `sed` does exactly what was asked and reports nothing unusual.
- **How to handle it, and why that works:** Use `-E` by default (`sed -E 's/foo+/bar/'`). It's supported on both BSD and GNU, unlike `-r`, and it makes `sed` agree with every other regex tool you use.
- **Trade-offs of the fix:** In ERE, `{` and `}` and `+` now need escaping when you want them *literally* — the inverse problem, but a much rarer one. Very old systems may lack `-E`, though anything current has it.

### Pitfall: Reaching for `sed` when the data has structure

- **What goes wrong:** A regex is written to extract a field from JSON, YAML, XML, or HTML. It works on the sample and breaks on: different key order, whitespace changes, nested objects, escaped quotes, or a value containing the delimiter. The failure is silent — a wrong value rather than an error.
- **Why it happens (the mechanism):** These formats are *nested and context-sensitive*; regular expressions cannot express arbitrary nesting (it's the pumping lemma, not a limitation of your pattern). A regex can only match a fixed textual shape, and any reformatting of semantically identical data breaks it.
- **How to handle it, and why that works:** Use a parser: `jq` for JSON, `yq` for YAML/XML, `htmlq`/`pup` for HTML, `ast-grep` for source code. These operate on the parsed structure, so key order and whitespace are irrelevant by construction and the query says what you mean (`.services[].image` rather than a regex that hopes).
- **Trade-offs of the fix:** Another tool and another query language, and `jq`'s syntax has a real learning curve. For a genuinely line-oriented log file `sed`/`awk` remain correct — the trigger is *nesting*, not *complexity*.

### Pitfall: `s///` replacing only the first match per line

- **What goes wrong:** `sed 's/,/;/'` on a CSV line converts only the first comma. The output looks plausible and is wrong, and on a one-comma test line it appears to work perfectly.
- **Why it happens (the mechanism):** `s` replaces the *first* occurrence on each line by default; `g` is what makes it global. This differs from most languages' `replace_all` defaults, so the expectation transfers wrongly.
- **How to handle it, and why that works:** Add `g` unless you specifically want the first: `sed 's/,/;/g'`. Test with a line containing several occurrences — a single-match test cannot distinguish the two behaviours, which is exactly why this survives review.
- **Trade-offs of the fix:** None. Occasionally you *want* only the first (fixing a leading field), and then `s///` without `g` — or `s///2` for the second — is correct and deliberate.

### Pitfall: Unescaped delimiters and injected variables

- **What goes wrong:** `sed "s/$path/$new/"` where `$path` is `/usr/local/bin` produces `s//usr/local/bin/...` — a syntax error, or worse, a valid-but-wrong command. If the variable comes from user input, it's a command-injection vector into the `sed` script.
- **Why it happens (the mechanism):** The delimiter is just the character after `s`, so any occurrence of it inside the pattern or replacement terminates the field early. Variables interpolated by the shell are inserted textually with no escaping.
- **How to handle it, and why that works:** Change the delimiter to something absent from the data — `sed "s|$path|$new|"` for paths, or `#`, or `,`. For untrusted input, don't build `sed` scripts by interpolation at all; use `sd` (which takes pattern and replacement as separate arguments, so there's no delimiter to break) or a real programming language.
- **Trade-offs of the fix:** Choosing a delimiter is still a guess about what the data contains. Passing pattern and replacement as arguments — as `sd` does — removes the class entirely, which is the strongest argument for it.
