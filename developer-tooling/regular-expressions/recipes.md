# Regular Expressions — Recipes

> Concepts are in [learning.md](learning.md); flag/syntax lookup is in [reference.md](reference.md).

## Testing and building a pattern

### I want to test a pattern interactively before using it

```sh
rg -e 'your-pattern' --pretty path/to/sample.txt
```

Or use an online tester matched to the right engine — [regex101.com](https://regex101.com) supports switching flavor (PCRE, Python, Go/RE2, JS) in a dropdown; make sure it's set to the flavor of the tool you'll actually run the pattern with, since PCRE-flavor testing gives false confidence about a pattern that will run on `rg`'s (RE2-like) default engine.

### I want to know which regex flavor a command is using

```sh
grep --version   # GNU grep supports -P; BSD/ugrep varies — check the man page
sed --version    # presence of this flag at all implies GNU sed, not BSD
```

If in doubt, test the specific metacharacter: `echo "a+" | grep -E 'a+'` matches "one or more a" only under `-E`/PCRE, not plain BRE `grep`.

## Extraction

### I want to extract just the matched part, not the whole line

```sh
grep -oP '\d+\.\d+\.\d+\.\d+' file.txt   # GNU grep, PCRE mode
rg -o '\d+\.\d+\.\d+\.\d+' file.txt      # ripgrep, default engine
```

`-o` prints only the matched text, one match per line, instead of the whole line it was found on.

### I want to extract a capture group, not the whole match

```sh
rg -or '$1' 'version=(\d+\.\d+)' file.txt
```

`-o` plus `-r '$1'` (replace-and-print) prints just the first capture group. For anything beyond a single group, reach for `awk` or a real scripting language — chained capture-group extraction in a one-liner regex tool gets unreadable fast.

### I want to pull structured values (JSON/YAML) instead of regexing them

```sh
jq '.services[].image' docker-compose.json   # don't regex JSON — see the pitfall in learning.md
```

## Matching across lines

### I want a pattern to match across multiple lines

```sh
rg -U 'fn \w+\([^)]*\)\s*\{' src/          # -U enables multiline mode
rg -U --multiline-dotall 'START.*END' file  # additionally lets . match \n
```

Multiline search is slower and can match unexpectedly large spans if the pattern is loose — anchor it as tightly as you can (a specific closing token, not just `.*`).

### I want `^`/`$` to match every line, not just start/end of the whole input

In most language APIs: pass the multiline flag (`re.M` in Python, `m` flag in JS/PCRE, `(?m)` inline). In `sed`/`awk`, this is already the per-line default since those tools process one line at a time — the multiline question only exists once you've read multiple lines into one buffer (`sed` hold space, or a `slurp`-style read).

## Avoiding the sharp edges

### I want to check a pattern isn't vulnerable to catastrophic backtracking before shipping it

Look for the shape: a quantified group, containing a quantified subexpression, where the subexpression's matches can overlap or be empty — `(a+)+`, `(a*)*`, `(a|a)*`, `(\w+\s?)+`. If you find one:

1. Simplify it — often the outer quantifier is redundant (`(a+)+` → `a+`).
2. Or run it through a linear-time engine instead (Rust `regex`, RE2, Go `regexp`) if the pattern doesn't need backreferences/lookaround.
3. Or, staying in a backtracking engine, make the inner group atomic: `(?>a+)+`.

```sh
# Rust regex crate (linear-time) rejects nothing at compile time for this —
# it simply can't express unbounded backtracking, so there's no separate "test" step.
```

### I want to match a literal string that might contain regex metacharacters

```sh
rg -F 'a.b*c(d)'          # -F: fixed string, no regex interpretation at all
grep -F 'a.b*c(d)' file   # same, POSIX grep
```

```python
import re
re.escape(user_input)     # escape before interpolating into a pattern
```

### I want to replace only the Nth occurrence, or all occurrences, on a line

```sh
sed 's/,/;/'      # first occurrence only (sed's default)
sed 's/,/;/g'     # all occurrences
sed 's/,/;/2'     # only the second occurrence
```

## Recovery — "I did something wrong"

### A pattern is hanging / spinning a CPU core

This is almost always catastrophic backtracking, not an infinite loop in your surrounding code.

1. **Kill it first:** `Ctrl-C`, or `kill <pid>` if it's backgrounded/detached. There is no recovery *from* a hung backtracking match — it will not finish in your lifetime on adversarial input; interrupting is correct, not a workaround.
2. **Identify the shape:** look for nested quantifiers per the pitfall above. Test the suspect subpattern in isolation against a long repeated string with no valid ending (the classic trigger).
3. **Fix before re-running:** simplify the pattern, add an atomic group/possessive quantifier, or switch to a linear-time engine. Don't just re-run the same pattern against smaller input and call it fixed — the exponential blowup means it will resurface the moment input length increases again.
4. **If this pattern ever takes untrusted input** (a search box, an API parameter, a config value from outside your team): treat this as a security bug, not a performance bug. Add a length cap on the input as defense-in-depth even after fixing the pattern.

### A `sed`/`grep` in-place edit corrupted files with a bad pattern

Not a regex-specific issue, but a common companion mistake — see [sed & Text Processing recipes](../sed-and-text-processing/recipes.md#safety-checklist-for-bulk-edits) for the portability trap and recovery steps around `-i`.

## References

- [learning.md](learning.md) — the mechanism behind every recipe above
- [reference.md](reference.md) — syntax and flag lookup
