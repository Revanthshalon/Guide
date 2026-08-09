# Strings & Text — Quick Reference

## At a Glance

A Rust string is a `Vec<u8>` plus one invariant: valid UTF-8. No O(1) character indexing exists, by design — a "character" has four different meanings and you must pick one.

**Invariant:** always valid UTF-8; every index must land on a char boundary. `&s[a..b]` **panics** off-boundary; `s.get(a..b)` returns `None`.

## The Four Levels (measured)

| Level | `"café"` | `"👨‍👩‍👧"` | `"🇮🇳"` | How |
| --- | --- | --- | --- | --- |
| Bytes | **5** | **18** | **8** | `s.len()` |
| Scalar values (`char`) | **4** | **5** | **2** | `s.chars()` |
| Grapheme clusters | 4 | **1** | **1** | `unicode-segmentation` |
| Words | 1 | 1 | 1 | `unicode-segmentation` |

A `char` is **not** a character. `chars().count()` is **not** what the user sees.

## Complexity

| Operation | Cost |
| --- | --- |
| `len()`, `as_bytes()`, `&s[a..b]` | Θ(1) |
| `chars().nth(i)` | **Θ(i)** |
| `chars().count()` | Θ(n) |
| `push_str` | Θ(k) amortized |
| `s = s + &t` in a loop | **Θ(n²)** |
| `find` | ~Θ(n) (two-way; `memchr` is SIMD) |
| `to_uppercase` | Θ(n), **length may change** |
| `==` | Θ(n) bytes — **not** Unicode equivalence |
| `split` | Θ(n) lazy, borrows, no alloc |

## Types

| Type | Size | Use |
| --- | --- | --- |
| `&str` | **16 B** | **Every function parameter** |
| `String` | **24 B** | Owned, growable (no SSO — always heap) |
| `Box<str>` | **16 B** | Owned, final, stored in bulk |
| `Cow<'a, str>` | 24 B | Usually borrowed, sometimes modified |
| `char` | **4 B** | One scalar value |
| `OsStr` / `Path` | — | Anything from the OS — not guaranteed UTF-8 |
| `&[u8]` | — | Might not be text at all |

`Option<String>` = 24 B, `Option<&str>` = 16 B (niche).

## Three Separate Problems

| Problem | Symptom | Fix |
| --- | --- | --- |
| **Encoding** | mojibake | UTF-8 (settled in Rust) |
| **Normalization** | `"café" != "café"` (NFC 2 B vs NFD 3 B) | `unicode-normalization` → NFC at the boundary |
| **Collation** | "Ä" sorts after "Z" | `icu`, not `sort()` |

## Case Conversion Is Not a Char Map

| Input | Output | Note |
| --- | --- | --- |
| `"ß".to_uppercase()` | `"SS"` | 1 char → 2 |
| `"ﬁ".to_uppercase()` | `"FI"` | ligature expands |
| `"İ".to_lowercase()` | `"i\u{307}"` | 2 chars; locale-dependent |

## Snippets

```rust
fn parse(input: &str) {}                       // &str, never &String

// Offsets must come FROM the string
if let Some(i) = s.find(':') { let (k, rest) = s.split_at(i); }
match s.char_indices().nth(n) {                 // safe truncation
    Some((byte_idx, _)) => &s[..byte_idx], None => s,
}

let mut out = String::with_capacity(est);       // not `out = out + part`
for p in parts { out.push_str(p); }
// or parts.concat() / parts.join(", ")

let text = std::str::from_utf8(&buf)?;          // validate once, borrow
let lossy = String::from_utf8_lossy(&buf);      // Cow, U+FFFD on bad bytes
```

## Crates

| Need | Crate |
| --- | --- |
| Graphemes, word boundaries | `unicode-segmentation` |
| NFC/NFD | `unicode-normalization` |
| Terminal column width | `unicode-width` |
| SIMD search | `memchr` |
| Millions of short strings | `compact_str`, `smartstring` |
| Intern to `u32` | `string-interner`, `lasso` |
| Editable documents | `ropey` |
| Locale sort/case | `icu` |

## Rules of Thumb

- Store bytes, validate once at the boundary, never re-validate.
- Get slice offsets from `find` / `char_indices` / `split` — never by arithmetic.
- Graphemes only when a **human** perceives the boundary (truncation, cursors, columns).
- Normalize to NFC at the edge; then `==` is correct again.
- Never carry byte offsets across a case conversion.
- Borrow through the pipeline, own at the sink.
- Intern anything compared or hashed repeatedly.
- `eq_ignore_ascii_case` for protocol tokens; `icu` folding for human text.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| `&s[..100]` truncation | Panic: "byte index is not a char boundary" on first non-ASCII input |
| `s += &part` in a loop | Θ(n²); seconds at 100k parts |
| Un-normalized compare | Two visually identical strings unequal; dedup silently fails |
| Assumed case preserves length | Buffer overflow / stale offsets on `ß`, `ﬁ`, `İ` |
| `to_string()` per field | 10M allocations; profile dominated by `malloc` |
| `chars().count()` as "length" | Emoji counted as 5; UI columns wrong |
| `String` for a file path | Fails on non-UTF-8 paths |

## Key References

- [Rust Book ch. 8.2](https://doc.rust-lang.org/book/ch08-02-strings.html)
- [UAX #29 (segmentation)](https://unicode.org/reports/tr29/) · [UAX #15 (normalization)](https://unicode.org/reports/tr15/)
- Spolsky, ["The Absolute Minimum … Unicode"](https://www.joelonsoftware.com/2003/10/08/the-absolute-minimum-every-software-developer-absolutely-positively-must-know-about-unicode-and-character-sets-no-excuses/)
