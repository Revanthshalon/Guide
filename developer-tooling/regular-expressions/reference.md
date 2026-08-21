# Regular Expressions — Quick Reference

## Quick Facts

- **Does:** Matches/extracts text against a pattern describing a shape, using either a backtracking engine (flexible, exponential worst case) or a finite-automaton engine (restricted, linear guaranteed).
- **Reach for it when:** The data is flat and line/token-oriented (logs, CSV, simple text extraction) and the shape is fixed.
- **Don't use it for:** Anything with real nesting (JSON, HTML, XML, source code) — use a parser. Anything security-sensitive where the pattern or input is attacker-influenced and you're on a backtracking engine.

## Syntax

```
.            any character except newline (unless dotall/(?s))
^  $         start / end of string (or line, in multiline/(?m) mode)
*  +  ?      0+, 1+, 0-or-1 of the preceding token (greedy)
*? +? ??     lazy versions — match as little as possible
*+ ++ ?+     possessive versions (PCRE/Java only) — never backtrack
{n} {n,} {n,m}   exact / at-least / range repetition
[abc] [^abc] [a-z]   character class / negated class / range
(...)        capturing group
(?:...)      non-capturing group
(?<name>...) named capturing group (PCRE/.NET); (?P<name>...) in Python
|            alternation
\1  \2       backreference to capture group N (backtracking engines only)
(?=...) (?!...)      lookahead / negative lookahead (backtracking only)
(?<=...) (?<!...)    lookbehind / negative lookbehind (backtracking only)
(?>...)      atomic group — no backtracking into it (backtracking engines)
\d \w \s     digit / word char / whitespace (scope depends on Unicode mode)
\D \W \S     negated versions
\b \B        word boundary / non-word-boundary
```

## Engine Comparison

| Engine | Used by | Lookaround | Backreferences | Worst case |
| --- | --- | --- | --- | --- |
| POSIX BRE | `grep`, `sed` (default) | No | `\1`-`\9` only | Impl-defined |
| POSIX ERE | `grep -E`, `sed -E`, `awk` | No | No | Impl-defined |
| PCRE/PCRE2 | `grep -P`, `rg -P`, most stdlibs | Yes | Yes | Exponential |
| RE2 / Rust `regex` | `rg` (default), Go | No | No | Linear (guaranteed) |
| JS `RegExp` | browsers, Node | Yes | Yes | Exponential |

## Mode Flags

| Flag (common spellings) | Effect |
| --- | --- |
| `i` / `(?i)` | Case-insensitive |
| `m` / `(?m)` | `^`/`$` match per line, not whole string |
| `s` / `(?s)` | `.` matches newline too ("dotall"/single-line mode) |
| `x` / `(?x)` | Extended/verbose — whitespace and `#` comments ignored in pattern |
| `u` (JS) / default (Python 3, Rust) | Unicode-aware `\d`/`\w`/`\b` |

## Gotchas

| Gotcha | Fix |
| --- | --- |
| `(a+)+` / `(\w+\s?)+` style nesting hangs on certain input (ReDoS) | Remove the nested ambiguity, or use a linear-time engine (`rg` default, RE2) for untrusted input |
| `+`/`?`/`|`/`(` do nothing in `grep`/`sed` default mode | You're in BRE — use `-E` (or escape: `\+`) |
| `<.*>` matches way more than expected | Greedy — use `<.*?>` (lazy) or `<[^>]*>` (negated class, faster) |
| `^`/`$` not matching per-line on multi-line input | Enable multiline mode (`(?m)`, or the tool's line-mode default may already differ — check) |
| `.` not matching across a line break | Enable dotall (`(?s)`) or the tool's multiline flag (`rg -U`) |
| Regex written for JSON/HTML breaks on reformatting | Don't — regex can't express nesting; use `jq`/`yq`/a real parser |
| `\d`/`\w` match unexpected Unicode characters | You're in Unicode mode; use an explicit ASCII class (`[0-9]`, `[A-Za-z0-9_]`) if that's what's meant |
| Renumbered `\1`/`\2` after inserting a group | Use named groups (`(?<name>...)`) instead of positional |
| Interpolating a variable into a pattern breaks or is exploitable | Escape it (`re.escape()` or equivalent) or use the tool's fixed-string mode (`rg -F`, `grep -F`) |

## Key References

- [regular-expressions.info](https://www.regular-expressions.info/)
- [`regex` crate syntax](https://docs.rs/regex/latest/regex/#syntax)
- [OWASP: ReDoS](https://owasp.org/www-community/attacks/Regular_expression_Denial_of_Service_-_ReDoS)
- Related: [ripgrep & grep reference](../ripgrep-and-grep/reference.md), [sed & text processing reference](../sed-and-text-processing/reference.md)
