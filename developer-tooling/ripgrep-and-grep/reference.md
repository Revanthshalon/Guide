# ripgrep & grep — Quick Reference

## Quick Facts

- **Does:** regex search across files; `rg` skips ignored/hidden/binary by default
- **Reach for it when:** finding anything in a codebase
- **Don't use it for:** structured data (`jq`/`yq`), syntax-aware refactors (`ast-grep`), portable scripts (use `grep`)
- **This machine:** ripgrep 15.2.0 · `grep` is **ugrep 7.5.0**, not GNU/BSD

## The `-u` Ladder (fixes most "rg found nothing")

| Flag | Also searches |
| --- | --- |
| *(default)* | respects `.gitignore`, skips hidden + binary |
| `-u` | ignored files |
| `-uu` | + hidden files |
| `-uuu` | + binary files (≈ `grep -r`) |

## Flags That Matter

| Flag | Does |
| --- | --- |
| `-i` / `-S` | ignore case / **smart case** (case-insensitive unless pattern has uppercase) |
| `-w` / `-x` | whole word / whole line |
| `-F` | **fixed string** — no regex, for literal text |
| `-l` / `--files-with-matches` | filenames only |
| `--files` | list files rg *would* search (great for debugging ignores) |
| `-c` | count per file |
| `-A/-B/-C n` | context after/before/around |
| `-g '*.rs'` / `-g '!vendor/**'` | glob include / **exclude** |
| `-t rust` / `-T test` | file type include / exclude (`rg --type-list`) |
| `-U` | **multiline** (`--multiline-dotall` for `.` to match `\n`) |
| `-o` | print only the match |
| `-r 'text'` | show replacement (**preview only**, doesn't write) |
| `--null` | NUL-separate output for `xargs -0` |
| `-P` | PCRE2: lookaround/backrefs — **exponential worst case** |
| `-z` | search inside gzip etc. |
| `--debug` | why a file was skipped |

## Syntax

```sh
rg 'pattern'                       # respects .gitignore
rg -uu 'pattern'                   # include hidden files
rg -t rust 'fn main'               # only Rust files
rg -g '!target/**' 'TODO'          # exclude a path
rg -F 'a.b.c'                      # literal, no regex
rg -C 3 'panic!'                   # 3 lines of context
rg -l --null 'old' | xargs -0 sd 'old' 'new'    # safe bulk edit

# grep — the portable form
grep -RIn --include='*.rs' 'pattern' .          # -I skips binary, -n line numbers
grep -E 'foo|bar' file                          # ERE: use -E, BRE needs \| \+ \?
grep -Fx -f wanted.txt haystack.txt             # fixed strings, whole line, from a file
```

## Regex Flavour

| Tool | Default | `+ ? | ( )` |
| --- | --- | --- |
| `grep` | POSIX **BRE** | **literal** — must escape: `\+` |
| `grep -E` | POSIX ERE | metacharacters |
| `grep -P` | PCRE2 | metacharacters, backtracking |
| `rg` | Rust `regex` | metacharacters, **linear time**, no lookaround/backrefs |

## Gotchas

| Gotcha | Fix |
| --- | --- |
| "rg found nothing" | `-uu` / `-uuu`; check `rg --files`, `rg --debug` |
| Shell ate the pattern | **Single-quote it**; `-e` for leading `-`; `-g` not shell globs |
| `grep 'a+b'` doesn't match | BRE — use `grep -E` |
| `-P` hangs on a long line | Catastrophic backtracking; drop `-P` |
| Multi-line pattern fails | `-U`; or use `jq`/`ast-grep` for structure |
| `xargs` breaks on spaces | `rg -l --null … | xargs -0` |
| Script works here, fails in CI | `grep` here is ugrep — test against target's grep |
| Slow pattern | Add a literal prefix — the SIMD prefilter needs one |

## Key References

- [ripgrep GUIDE](https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md)
- [ripgrep is faster than…](https://blog.burntsushi.net/ripgrep/)
- [`regex` syntax](https://docs.rs/regex/latest/regex/#syntax)
