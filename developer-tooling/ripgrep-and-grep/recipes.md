# ripgrep & grep — Recipes

> Concepts are in [learning.md](learning.md); flag lookup is in [reference.md](reference.md).

## Finding code

```sh
rg 'fn \w+_handler'                       # regex
rg -t rust -w 'unwrap'                    # whole word, Rust files only
rg -F 'config.get("key")'                 # literal — no escaping needed
rg -S 'TodoItem'                          # smart case: matches TodoItem, not todoitem
rg -g '*.{rs,toml}' 'version'             # multiple extensions
rg -g '!tests/**' -g '!benches/**' 'pub fn'
```

### Where is this defined vs used?

```sh
rg 'fn parse_config'                      # definition
rg 'parse_config\('                       # call sites
rg -l 'parse_config' | rg -v 'parse_config.rs'   # files that use but don't define it
```

### Find TODOs with author and context

```sh
rg -n --heading -C 1 'TODO|FIXME|XXX|HACK'
rg -n 'TODO' -g '!vendor/**' --sort path
```

## Filtering the search space

```sh
rg --type-list                            # what -t knows about
rg -t py -t js 'apiKey'                   # several types
rg -g '*.rs' -g '!**/generated/*' 'Deserialize'
rg --files | rg 'migrations'              # which files WOULD be searched
rg --files-without-match 'license' -g '*.rs'   # files MISSING something
```

## Context and output shaping

```sh
rg -C 3 'panic!'                          # 3 lines around
rg -A 10 'fn main'                        # 10 lines after
rg --heading --line-number --color always 'x' | less -R
rg -o '"[a-z_]+":' data.json | sort -u    # just the matches, deduped
rg -c 'unwrap' -g '*.rs' | sort -t: -k2 -rn | head    # count per file, ranked
```

## Multi-line and structured

```sh
rg -U 'fn \w+\([^)]*\n[^)]*\)'            # signature spanning lines
rg -U --multiline-dotall 'struct Config \{.*?\}'

# For structure, stop using regex:
jq '.dependencies | keys[]' package.json
yq '.services | keys' compose.yaml
ast-grep --pattern 'unwrap()' --lang rust  # syntax-aware, formatting-independent
```

## Bulk edits (safely)

```sh
# 1. Preview — rg -r does NOT write
rg 'old_name' -r 'new_name'

# 2. Confirm the file list
rg -l 'old_name'

# 3. Apply. `sd` is the safe modern choice (no BSD/GNU sed divergence)
rg -l --null 'old_name' | xargs -0 sd 'old_name' 'new_name'

# With sed, note the macOS/BSD -i quirk:
rg -l --null 'old' | xargs -0 sed -i '' 's/old/new/g'      # BSD/macOS: -i ''
rg -l --null 'old' | xargs -0 sed -i    's/old/new/g'      # GNU: bare -i
```

Always run steps 1–2 before 3, and have the change staged in git so `git diff` is your review.

## Searching things that aren't plain files

```sh
rg -z 'error' logs/*.gz                   # compressed
rg --pre pdftotext 'invoice' docs/        # preprocessor for PDFs
rg -uuu 'magic' target/                   # binaries and ignored dirs
git log -S 'removed_function' --oneline   # search HISTORY, not the tree
git grep 'pattern' $(git rev-list --all)  # every commit
```

## Portable `grep` (for scripts that run anywhere)

```sh
grep -RIn --include='*.rs' 'pattern' .    # -I skip binary, -n numbers, -R recurse
grep -E 'foo|bar' file                    # ALWAYS -E; BRE needs \| \+ \?
grep -Fq 'needle' file && echo found      # -q: exit status only, no output
grep -c '' file                           # line count (portable wc -l)
grep -v '^#' conf | grep -v '^$'          # strip comments and blanks
grep -o 'pat' file | sort | uniq -c | sort -rn   # frequency of matches
```

`-q` with `&&` is the idiom for "does this file contain X" in a script — it exits early and prints nothing.

## Debugging "why didn't it match?"

```sh
rg --debug 'pattern' path/ 2>&1 | head -30    # which ignore rule fired
rg --files path/ | head                        # is the file even considered?
rg -uuu 'pattern'                              # ignore all filtering
echo 'test string' | rg 'pattern'              # test the pattern in isolation
rg -F 'literal'                                # is it a regex-escaping problem?
```

Work down that list in order — it resolves nearly every case.

## References

- [ripgrep GUIDE](https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md)
- [`sd`](https://github.com/chmln/sd) — sane find-and-replace, no sed portability traps
- [`ast-grep`](https://ast-grep.github.io/) — structural search for code
