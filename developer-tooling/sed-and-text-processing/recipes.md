# sed & Text Processing — Recipes

> Concepts are in [learning.md](learning.md); flag lookup is in [reference.md](reference.md).
> **This machine has BSD sed.** In-place edits below use the portable temp-file form so they work everywhere.

## Substitution

```sh
sed -E 's/old/new/g' file                   # all occurrences, every line
sed -E 's/old/new/' file                    # FIRST occurrence per line only
sed -E 's/old/new/2' file                   # second occurrence per line
sed -E '10,20s/old/new/g' file              # only in lines 10–20
sed -E '/^ERROR/s/foo/bar/g' file           # only on lines starting with ERROR
sed -E 's|/usr/local|/opt|g' file           # alternate delimiter for paths
sed -E 's/(\w+)@(\w+)/\2 at \1/' file       # capture groups: \1 \2
sed -E 's/[[:space:]]+$//' file             # strip trailing whitespace
```

### In-place, portably

```sh
# Preferred: no sed portability question at all
sd 'old' 'new' file

# Portable sed: temp file + move (also atomic)
sed -E 's/old/new/g' file > file.tmp && mv file.tmp file

# Many files
rg -l --null 'old' | xargs -0 -I{} sh -c 'sed -E "s/old/new/g" "{}" > "{}.tmp" && mv "{}.tmp" "{}"'
```

## Extracting

```sh
sed -n '5p' file                            # line 5
sed -n '5,10p' file                         # lines 5–10
sed -n '$p' file                            # last line
sed -n '/BEGIN/,/END/p' file                # a block
sed -n '/ERROR/{p;q}' file                  # first match, then STOP (fast on big files)
sed -n 's/.*version="([^"]+)".*/\1/p' f     # -n + s///p = extract only what matched
sed -n '0~3p' file                          # every 3rd line (GNU only)
```

## Deleting

```sh
sed '/^#/d' file                            # comment lines
sed '/^$/d' file                            # blank lines
sed '/^#/d; /^[[:space:]]*$/d' file         # both — this is the "strip a config" idiom
sed '1d' file                               # header
sed '$d' file                               # last line
sed '1,10d' file                            # first 10
sed '/BEGIN/,/END/d' file                   # a block
sed -n '/pat/!p' file                       # keep only NON-matching (same as grep -v)
```

## Inserting

```sh
sed '1i\
#!/bin/bash' file                           # prepend (BSD needs the backslash-newline)

sed '/^\[server\]/a\
port = 8080' config.ini                     # append after a matching line

sed 's/$/;/' file                           # append to every line
sed 's/^/> /' file                          # prefix every line (quote it)
```

## Columns, sums, and state — this is awk

```sh
awk '{print $1, $3}' file                   # fields 1 and 3
awk '{print $NF}' file                      # last field
awk -F',' '{print $2}' data.csv             # comma-separated
awk -F: '$3 >= 1000 {print $1}' /etc/passwd # condition on a field

awk '{s+=$3} END {print s}' file            # sum column 3
awk '{s+=$1} END {print s/NR}' file         # mean
awk '$3 > max {max=$3; line=$0} END {print line}' file    # row with max column 3

awk '!seen[$0]++' file                      # dedupe, ORDER PRESERVED (uniq needs sorting)
awk 'NR==FNR{a[$1]=$2; next} $1 in a {print $0, a[$1]}' map.txt data.txt   # join
awk '/START/{f=1} f{print} /END/{f=0}' file # stateful block extraction
awk 'length > 100' file                     # long lines
awk '{c[$1]++} END {for (k in c) print c[k], k}' file | sort -rn   # frequency
```

**`awk '!seen[$0]++'`** is the one to memorize — deduplication without sorting, which `sort -u` cannot do while preserving order.

## Log processing

```sh
# Top 10 IPs in an access log
awk '{print $1}' access.log | sort | uniq -c | sort -rn | head

# Requests per minute
awk '{print substr($4,2,17)}' access.log | uniq -c

# 5xx only, with the path
awk '$9 ~ /^5/ {print $9, $7}' access.log | sort | uniq -c | sort -rn

# Time window
sed -n '/2025-08-09T14:00/,/2025-08-09T15:00/p' app.log

# p95 of a latency column (field 8)
awk '{print $8}' app.log | sort -n | awk '{a[NR]=$1} END {print a[int(NR*0.95)]}'
```

That last one is exact-percentile-by-sorting — fine offline; for a live stream use a mergeable sketch instead ([probabilistic structures](../../data-structures-and-algorithms/probabilistic-data-structures/learning.md)).

## Structured data — use a parser

```sh
jq -r '.items[] | "\(.id)\t\(.name)"' data.json
jq -r 'to_entries[] | select(.value > 10) | .key' counts.json
jq -s 'add' part*.json                       # merge files
yq '.services | keys' compose.yaml
yq -i '.version = "3.9"' compose.yaml        # in-place, structure-aware
```

Never regex JSON/YAML — key order and whitespace changes will break it silently.

## Whitespace and encoding

```sh
sed -E 's/[[:space:]]+$//' f > f.tmp && mv f.tmp f     # trailing whitespace
sed -E 's/\r$//' f > f.tmp && mv f.tmp f               # CRLF → LF
sed -E 's/\t/    /g' f                                 # tabs → spaces
awk '{$1=$1; print}' f                                 # squeeze internal whitespace
printf '%s\n' "$(cat f)"                               # ensure trailing newline
file f && wc -l f                                      # check encoding / line count
```

## Safety checklist for bulk edits

1. `rg -l 'pattern'` — see which files will change
2. `rg 'pattern' -r 'replacement'` — preview the result (writes nothing)
3. Ensure the tree is clean or staged: `git status`
4. Apply
5. `git diff` — **this is the actual review step**
6. Run the tests

Skipping step 5 is how bulk edits cause outages.

## References

- [GNU sed manual](https://www.gnu.org/software/sed/manual/sed.html) · [AWK one-liners explained](https://catonmat.net/awk-one-liners-explained-part-one)
- [`sd`](https://github.com/chmln/sd) · [`jq` manual](https://jqlang.github.io/jq/manual/)
