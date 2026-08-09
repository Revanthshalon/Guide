# sed & Text Processing — Quick Reference

## Quick Facts

- **Does:** line-by-line stream editing (`sed`); field/record processing (`awk`)
- **Reach for it when:** line-local transforms (`sed`), columns/sums/state (`awk`)
- **Don't use it for:** JSON/YAML/XML/HTML/code — use `jq`/`yq`/`htmlq`/`ast-grep`
- **This machine:** **BSD sed** (macOS) — `sed -i` needs `''`

## The Loop

```
for each line: load into pattern space → run script → print (unless -n) → repeat
```

Command form: `[address]command` — address selects, command acts.

## Addresses

| Address | Selects |
| --- | --- |
| `5` | line 5 |
| `$` | last line |
| `2,5` | lines 2–5 |
| `/re/` | lines matching |
| `/start/,/end/` | block between |
| `2,$` | line 2 to end |
| `/re/!` | lines **not** matching |

## Flags That Matter

| Flag | Does |
| --- | --- |
| `-n` | suppress auto-print (pairs with `p`) |
| **`-E`** | **extended regex — use this always** |
| `-i` | in-place (**GNU: `-i`; BSD: `-i ''`**) |
| `-e` | multiple script fragments |
| `-f file` | script from file |

`s///` flags: `g` (all on line) · `2` (2nd occurrence) · `p` (print) · `i` (case-insensitive, GNU) · `w f` (write)

## GNU vs BSD

| Feature | GNU | **BSD (here)** |
| --- | --- | --- |
| In-place | `-i` | **`-i ''`** |
| ERE | `-r` or `-E` | `-E` only |
| `\U \L \u \l` | yes | **no** |
| `\b \< \>` | yes | `[[:<:]] [[:>:]]` |
| `-z` | yes | no |

**Portable in-place:** `sed -E 's/a/b/' f > f.tmp && mv f.tmp f` — or just use [`sd`](https://github.com/chmln/sd).

## Syntax

```sh
sed -E 's/old/new/g' file            # substitute all
sed -E 's|/usr/local|/opt|g' file    # alternate delimiter for paths
sed -n '5p' file                     # print line 5
sed -n '/BEGIN/,/END/p' file         # print a block
sed '/^#/d; /^$/d' file              # delete comments and blanks
sed '$d' file                        # drop last line
sed -n '$=' file                     # count lines
sed '2i\
new line' file                       # insert before line 2 (BSD needs the backslash-newline)

# awk — when you need fields, sums, or state
awk '{print $2, $NF}' file                    # 2nd and last field
awk -F: '$3 >= 1000 {print $1}' /etc/passwd   # custom separator + condition
awk '{s+=$3} END {print s}' file              # sum a column
awk '!seen[$0]++' file                        # dedupe, preserving order
awk 'NR==FNR{a[$1];next} $1 in a' k.txt d.txt # join on a key
awk '{print NR": "$0}' file                   # number lines

# Structured data — stop using regex
jq -r '.services | keys[]' compose.json
yq '.services.api.image' compose.yaml
```

## Gotchas

| Gotcha | Fix |
| --- | --- |
| `sed -i` fails on the other OS | Temp file + `mv`, or `sd` |
| `s/foo+/x/` doesn't match | BRE — use `-E` |
| Only the first match replaced | Add `g` |
| Delimiter appears in the data | Use `s|…|…|` or `s#…#…#` |
| Variable breaks the script | Change delimiter; or `sd` (separate args) |
| Regex on JSON/YAML breaks | `jq` / `yq` |
| Reaching for hold space (`h`,`G`,`N`) | Switch to `awk` |
| `\U` doesn't work | GNU-only; not on BSD |

## Key References

- [GNU sed manual](https://www.gnu.org/software/sed/manual/sed.html) — note GNU-only features
- [`sd`](https://github.com/chmln/sd) · [`jq`](https://jqlang.github.io/jq/manual/) · [AWK one-liners](https://catonmat.net/awk-one-liners-explained-part-one)
