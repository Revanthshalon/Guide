# Strings & Text — Learning Notes

## Mental Model

**A Rust string is a `Vec<u8>` with one extra invariant — valid UTF-8 — and every design consequence follows from that single addition.**

The consequence people trip on first: **there is no O(1) "character" indexing, by design.** In UTF-8 a character occupies 1–4 bytes, so the byte offset of character *i* cannot be computed; it must be walked. Rust could have hidden this behind an O(n) `s[i]` (as Python does) or stored fixed-width 32-bit characters (as some languages do, at 4× the memory). It does neither: `s[i]` for an integer `i` doesn't compile, `s[a..b]` takes **byte** offsets and *panics* if they aren't character boundaries. That's the language forcing you to confront a question every text bug comes from — **which unit of "character" do you actually mean?**

There are four, and they are all different:

| Level | Unit | `"café"` | `"👨‍👩‍👧"` | Rust type |
| --- | --- | --- | --- | --- |
| Bytes | UTF-8 code units | **5** | **18** | `u8`, `s.len()` |
| Scalar values | Unicode code points | **4** | **5** | `char`, `s.chars()` |
| Grapheme clusters | What a user calls a character | 4 | **1** | `unicode-segmentation` crate |
| Words | Locale-dependent | 1 | 1 | `unicode-segmentation` |

All four numbers above are measured. The family emoji is 18 bytes, 5 `char`s (three people plus two zero-width joiners), and **one** thing a user would call a character. `"🇮🇳"` is 8 bytes and 2 `char`s (two regional indicators) and one flag. A `char` is *not* a character — it's a Unicode scalar value, always exactly 4 bytes in memory, and `chars().count()` is not "the length the user sees."

So the working model:

> Store bytes. Validate once at the boundary. Slice by byte offsets you obtained from Rust itself (`find`, `char_indices`, `split`), never by arithmetic. Only reach for graphemes when a *human* will perceive the result — truncation, cursor movement, column alignment.

## The Invariant

> A `String` / `&str` is **always valid UTF-8**. Every index into it must fall on a character boundary. Both are enforced: construction validates (`String::from_utf8` returns `Result`), and slicing panics on a non-boundary.

Measured on `"café"` (5 bytes, `é` occupying bytes 3–4):

```
s.get(0..3) == Some("caf")     is_char_boundary(3) == true
s.get(0..4) == None            is_char_boundary(4) == false   ← mid-character
s.get(0..5) == Some("café")    is_char_boundary(5) == true
```

`&s[0..4]` on that same string **panics**. This is the invariant defending itself: a byte slice that split a character would no longer be UTF-8, so Rust refuses rather than producing a `str` that violates its own type contract. `get()` is the non-panicking form and is what you want anywhere the offset came from outside your control.

The invariant is also a *performance* asset, not just a safety one: because validity is guaranteed by the type, every operation downstream — iteration, comparison, searching — can skip validation. Validate once at the edge; never again.

## Mechanics

### The type zoo, and when each applies

| Type | Size | Owns? | Use |
| --- | --- | --- | --- |
| `&str` | **16 B** (ptr + len) | no | **Function parameters.** The universal borrowed string |
| `String` | **24 B** (ptr + len + cap) | yes | Growable owned text |
| `Box<str>` | **16 B** | yes | Owned, never resized — saves 8 B, e.g. in a large `Vec` |
| `Cow<'a, str>` | 24 B | maybe | Usually borrowed, occasionally modified — the parser's friend |
| `char` | **4 B** | — | One Unicode scalar value, not one "character" |
| `OsStr`/`OsString` | — | — | OS-native encoding; **not** guaranteed UTF-8 |
| `Path`/`PathBuf` | — | — | Paths; also not guaranteed UTF-8 |
| `[u8]`/`Vec<u8>` | — | — | Bytes that *might* not be text — network, files |

Measured: `Option<String>` is also 24 bytes and `Option<&str>` 16 — the null-pointer niche means optional strings are free.

**Note what's absent: `String` has no small-string optimization.** Unlike C++'s `std::string`, every non-empty Rust `String` is a heap allocation, even a 3-byte one. For workloads with millions of short strings that matters, and the answer is a crate: `compact_str` or `smartstring` store up to ~22–24 bytes inline.

### The operations that are cheap, and the ones that aren't

| Operation | Cost | Note |
| --- | --- | --- |
| `s.len()` | Θ(1) | **Bytes**, not characters |
| `s.as_bytes()` | Θ(1) | Free — the representation *is* bytes |
| `&s[a..b]` | Θ(1) | Panics off-boundary; `get()` returns `Option` |
| `s.chars()` | Θ(1) per step, Θ(n) to count | `chars().count()` is a full scan |
| `s.chars().nth(i)` | **Θ(i)** | The "index a character" trap |
| `s.push_str` | Θ(k) amortized | Same doubling as `Vec` |
| `s + &t` in a loop | **Θ(n²)** | The classic accidental quadratic |
| `s.find(pat)` | Θ(n·m) worst, near-Θ(n) typical | Uses a two-way/`memmem`-style search, not naive |
| `s.to_uppercase()` | Θ(n), **length may change** | See pitfalls |
| `s == t` | Θ(n) | Byte comparison — **not** Unicode equivalence |
| `s.split(pat)` | Θ(n) lazily | Returns borrowed `&str`s — no allocation |

**`char_indices()` is the one to internalize**: it yields `(byte_offset, char)` pairs, which is how you get a byte offset that is *guaranteed* to be a valid boundary. Almost every "slice a string safely" problem is solved by getting offsets from `char_indices`, `find`, or `split` rather than computing them.

### Encoding vs. normalization vs. collation — three separate problems

- **Encoding** — how code points become bytes. UTF-8 is settled; Rust makes it non-negotiable.
- **Normalization** — the *same* text can have multiple valid code-point sequences. Measured: `"é"` precomposed (NFC) is **2 bytes, 1 char**; the same glyph as `e` + combining acute (NFD) is **3 bytes, 2 chars**. They render identically and compare **unequal** under `==`. macOS filesystems historically hand you NFD; most web input is NFC. Fix with the `unicode-normalization` crate, normalizing to NFC at the boundary.
- **Collation** — sort order, which is locale-dependent. `sort()` on `&str` gives *byte* order: all uppercase before all lowercase, and "Ä" after "Z". For user-visible sorting you need ICU-style collation (`icu` crate), not `sort()`.

These are independent: normalizing doesn't fix collation, and neither is about encoding.

### Case conversion is not a per-character map

Measured facts that break the "just map each char" assumption:

- `"ß".to_uppercase()` → `"SS"` — **one char becomes two.**
- `"ﬁ".to_uppercase()` → `"FI"` — a ligature expands.
- `"İ".to_lowercase()` → `"i\u{307}"` — **two chars**, and Turkish locale rules differ again.

So `to_uppercase()` returns a `String`, not an in-place transform, and any code assuming length or char-count is preserved is wrong. For case-insensitive *comparison*, `eq_ignore_ascii_case` is correct and fast for ASCII; for Unicode, case-fold (`str::to_lowercase` is an approximation; proper case folding needs `caseless`/`icu`).

## Complexity

| Operation | Average | Worst | Space |
| --- | --- | --- | --- |
| Byte index / slice | Θ(1) | Θ(1) | — |
| `chars().nth(i)` | Θ(i) | Θ(n) | — |
| `chars().count()` | Θ(n) | Θ(n) | — |
| Append (`push_str`) | Θ(k) amortized | Θ(n+k) realloc | — |
| Concatenation in a loop | — | **Θ(n²)** | — |
| Substring search | ~Θ(n) | Θ(n·m) | Θ(1) |
| Comparison | Θ(min(n,m)) | Θ(n) | — |
| Split (lazy) | Θ(n) total | Θ(n) | Θ(1) — borrows |
| `to_uppercase` | Θ(n) | Θ(n) | Θ(n) — new allocation |
| Grapheme iteration | Θ(n) | Θ(n) | Θ(1) |

**Where the table misleads:** substring search worst case is Θ(n·m), but std's `find` uses a two-way algorithm with good practical behaviour, and `memchr`/`memmem` are SIMD-accelerated well beyond it — a byte search runs at GB/s. Don't hand-roll a matcher before measuring the library one (the algorithms behind it are Stage 7).

## Rust Implementation

```rust
// Take &str, not &String — accepts literals, slices, and Strings alike.
fn parse(input: &str) -> Result<Ast, Error> { /* ... */ }

// Safe slicing: get offsets FROM the string, never by arithmetic.
if let Some(i) = s.find(':') {
    let (key, rest) = s.split_at(i);       // i came from find → guaranteed boundary
    let value = &rest[1..];                // ':' is 1 byte, so this one is safe
}

// Truncate safely — the "cut at N bytes" bug, fixed.
fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((byte_idx, _)) => &s[..byte_idx],
        None => s,
    }
}

// Build efficiently: one allocation, no quadratic concat.
let mut out = String::with_capacity(estimated);
for part in parts { out.push_str(part); }
// or: let out: String = parts.concat();  /  parts.join(", ");

// Cow: borrow unless you actually had to change something.
fn normalize_ws(s: &str) -> Cow<'_, str> {
    if s.contains("  ") { Cow::Owned(s.split_whitespace().collect::<Vec<_>>().join(" ")) }
    else { Cow::Borrowed(s) }
}

// Bytes that might not be text — don't force them through String.
let text = std::str::from_utf8(&buf)?;              // borrow, validate once
let lossy = String::from_utf8_lossy(&buf);          // Cow: replaces bad bytes with U+FFFD
```

**Crates that matter:**

| Need | Crate |
| --- | --- |
| Grapheme clusters, word boundaries | `unicode-segmentation` |
| NFC/NFD normalization | `unicode-normalization` |
| Display width in terminal columns | `unicode-width` |
| Fast substring/byte search | `memchr` (SIMD) |
| Millions of short strings | `compact_str`, `smartstring` (inline ≤ ~24 B) |
| Interning to `u32` IDs | `string-interner`, `lasso` |
| Editable large text (editors) | `ropey` — a rope: Θ(log n) insert/delete anywhere |
| Locale-correct sorting/case | `icu` |

**Interning is the highest-leverage trick** for anything comparing or hashing the same strings repeatedly: convert each distinct string to a `u32` once, then compare and hash integers. This is what `rustc`'s `Symbol` does, and it converts the O(k)-in-key-length problem from [complexity analysis](../complexity-analysis/learning.md) into genuine O(1).

## Use Cases

- **Parsers and tokenizers.** Borrow everything: tokens are `&str` slices into one input buffer, zero allocation. `Cow` for the rare token needing an escape decoded. This is the single biggest performance decision in a parser.
- **Log and protocol processing.** `memchr` to find delimiters, `split` to yield borrowed fields, parse numbers straight from `&str`. Allocating a `String` per field is the most common reason a log processor is slow.
- **Identifiers and keys.** Intern to `u32` at the boundary; store and compare integers internally.
- **User-facing truncation.** "Show the first 20 characters" — graphemes, not bytes and not `char`s, or you'll cut a family emoji into pieces or a combining accent off its base letter.
- **Text editors.** `ropey` or a piece table; a flat `String` makes an insert at the front Θ(n) and unusable at document scale.
- **Filesystem paths.** `Path`/`OsStr`, not `String` — paths on Linux are arbitrary bytes and on Windows are UTF-16-ish, and neither is guaranteed valid UTF-8.

## When to Use Which

| Reach for | When |
| --- | --- |
| **`&str`** | Every function parameter that reads text |
| `String` | Owned, growable, built at runtime |
| `Box<str>` | Owned, final, stored in bulk — 8 B less per instance |
| `Cow<'a, str>` | Usually unchanged, occasionally modified |
| `&[u8]` / `Vec<u8>` | Not guaranteed to be text; or you only need bytes |
| `OsStr` / `Path` | Anything from the OS: paths, args, env vars |
| `compact_str` | Millions of short strings |
| Interned `u32` | The same strings compared/hashed repeatedly |
| `ropey` | Editable documents, insert/delete in the middle |
| Grapheme iteration | A human will perceive the boundary |

## Pitfalls in Depth

### Pitfall: Slicing at a byte offset you computed yourself

- **What goes wrong:** `&s[..100]` to truncate a field for display or storage. It works for every ASCII input and panics — `byte index 100 is not a char boundary` — the first time a user's name contains an accent, an emoji, or any non-Latin script. A panic in a request handler, triggered by data, that no test covered.
- **Why it happens (the mechanism):** Byte offsets and character positions coincide exactly for ASCII, which is what all the test fixtures are. UTF-8 makes them diverge for everything else, and the invariant defends itself by panicking rather than producing an invalid `str`.
- **How to handle it in production, and why that works:** Get offsets *from* the string: `char_indices().nth(n)` for a character-count truncation, `find`/`split` for delimiters, or `s.get(..n)` for a non-panicking attempt. For anything user-visible, use `unicode-segmentation`'s grapheme iterator — that's the only unit that matches what a person sees. `floor_char_boundary` (nightly at time of writing) does the "back off to the nearest valid boundary" operation directly.
- **Trade-offs of the fix:** `char_indices().nth(n)` is Θ(n), so truncating a huge string to a small prefix is now a scan rather than a constant-time slice — irrelevant for display strings, potentially relevant in a hot loop. Grapheme segmentation is slower still and pulls in a Unicode table. Match the level to the consumer: bytes for storage limits, graphemes for humans.

### Pitfall: Building strings by repeated concatenation

- **What goes wrong:** `for part in parts { result = result + part; }` or `result += &part` inside a loop over thousands of items. Each step may reallocate and copy the entire accumulated prefix, making the build Θ(n²) in total output size. Fine for 100 parts; several seconds for 100,000.
- **Why it happens (the mechanism):** The same accidental quadratic as `Vec`, wearing different syntax. `String + &str` consumes and returns the `String`, so it looks like a single cheap step; the copying is the reallocation, and it recurs as the buffer grows.
- **How to handle it in production, and why that works:** `String::with_capacity(estimate)` then `push_str` — one allocation, Θ(n) total. Or `parts.concat()` / `parts.join(sep)`, which compute the exact final length up front and allocate once. `write!` into a `String` via `fmt::Write` is the same win for formatted output.
- **Trade-offs of the fix:** `with_capacity` needs an estimate; a bad one either wastes memory or reintroduces growth. `concat`/`join` need the parts materialized in a collection first, which costs memory you might not have for a streaming workload — there, write into a `String` or straight into a `BufWriter`.

### Pitfall: Comparing un-normalized Unicode

- **What goes wrong:** A username, filename, or search key doesn't match itself. `"café" == "café"` is `false` because one is NFC (`é` as one 2-byte code point) and the other NFD (`e` plus a combining accent, 3 bytes, 2 chars) — measured above. They render identically, so the bug is invisible in logs, in a debugger, and in the code review. Deduplication silently fails; a lookup misses; two "identical" accounts exist.
- **Why it happens (the mechanism):** Unicode allows multiple encodings of the same rendered text, and `==` on `str` is a byte comparison — fast, deterministic, and unaware of equivalence. Different sources normalize differently: macOS filesystems produce NFD, browsers usually submit NFC, so a round trip through a file path can change the bytes.
- **How to handle it in production, and why that works:** Normalize at the boundary, once, to NFC (the web's default), using `unicode-normalization`. Store normalized. Then byte comparison is correct again and stays fast, because the property is established at the edge rather than checked everywhere.
- **Trade-offs of the fix:** Normalization is Θ(n) with a Unicode table, so it's a real cost at the boundary and a dependency you now carry. It's also lossy in the sense that you can no longer round-trip the user's exact bytes — which matters if you must echo input back verbatim, or for cryptographic signing where the exact bytes are the signed object. Normalize the *key*, keep the *original* if you need both.

### Pitfall: Assuming case conversion preserves length or char count

- **What goes wrong:** Code allocates a same-size buffer for an uppercase result, or asserts `s.len() == s.to_uppercase().len()`, or maps case per-`char`. Measured counterexamples: `"ß".to_uppercase()` is `"SS"` (1 char → 2), `"ﬁ".to_uppercase()` is `"FI"`, `"İ".to_lowercase()` is 2 chars. Buffers overflow their estimate, offsets computed before conversion point at the wrong place afterward, and Turkish locales break `i`/`I` round-tripping outright.
- **Why it happens (the mechanism):** Case mapping is a *string-to-string* function in Unicode, not a character-to-character one — some mappings are one-to-many, some are context- or locale-dependent. `char::to_uppercase` returns an *iterator* precisely because one `char` can map to several, which is easy to overlook.
- **How to handle it in production, and why that works:** Treat `to_uppercase`/`to_lowercase` as allocating transforms whose output length is unknown until computed, and never carry offsets across them. For comparison rather than display, use `eq_ignore_ascii_case` when the domain is ASCII (identifiers, protocol tokens — fast and locale-free), and proper case folding via `icu`/`caseless` when it isn't.
- **Trade-offs of the fix:** Restricting to ASCII case rules is *wrong* for user text but *right* for protocol tokens, and picking the wrong one either mangles names or introduces locale-dependence into a wire format. Full Unicode case folding costs table lookups and a dependency. Decide per field whether it's human text or machine text.

### Pitfall: Allocating a `String` per field in a hot loop

- **What goes wrong:** A log or CSV processor does `line.split(',').map(|s| s.to_string()).collect::<Vec<String>>()` per line. At a million lines and ten fields that's ten million allocations. The profile shows most of the time in `malloc` and `memcpy`, and the parsing logic is a rounding error.
- **Why it happens (the mechanism):** `split` already yields borrowed `&str` slices into the input — zero allocation. Calling `to_string()` on each copies the bytes and allocates, converting a zero-cost view into a per-field heap object, usually just to satisfy a struct field declared as `String` out of habit.
- **How to handle it in production, and why that works:** Make the intermediate types borrow: `struct Record<'a> { name: &'a str, … }` tied to the input buffer's lifetime. Parse numbers directly from `&str` without materializing anything. Use `Cow<'a, str>` for fields that occasionally need transformation. Only allocate at the point where the value must outlive the buffer.
- **Trade-offs of the fix:** Lifetimes propagate — `Record<'a>` infects every struct that holds one, and you can't store it in a long-lived collection without either keeping the input buffer alive or converting to owned at that boundary. That's a real design constraint, and it's why the usual shape is "borrow through the pipeline, own at the sink."

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if every edit returned a new document? | Persistent ropes; the editor undo stack for free |
| Batch it | What if you appended once instead of n times? | `concat`/`join`/`with_capacity` — the Θ(n²)→Θ(n) fix |
| Approximate it | What if equality were "close enough"? | Edit distance, fuzzy matching, phonetic keys (Soundex), MinHash for near-duplicates |
| Randomize it | What if you hashed substrings? | Rolling hashes → Rabin-Karp; content-defined chunking for dedup |
| Externalize it | What if the text didn't fit in RAM? | Memory-mapped files + `&str` views; external suffix arrays |
| Parallelize it | Where's the boundary problem? | Chunked scanning must not split a code point — chunk on `is_char_boundary`, then merge |
| Invert it | What if you indexed positions by content instead of content by position? | The **inverted index** — the whole of search |
| Augment it | What does storing extra per chunk buy? | A rope's subtree byte/line counts → Θ(log n) "line 4,000" lookup |
| Specialize it | What if the text were known ASCII? | Byte ops, SIMD, `eq_ignore_ascii_case`, direct indexing — often 5–10× |
| Amortize it | What if one operation could be terrible? | Gap buffers: cheap edits at the cursor, one expensive gap move when it jumps |

**Questions:**

1. Rust makes `s[i]` for an integer a compile error rather than an O(n) operation. Argue the case *against* that decision, then say what it would have cost in bugs.
2. A `char` is 4 bytes and always exactly one scalar value. Given `"👨‍👩‍👧"` is 5 chars and 1 grapheme, at which of the four levels does "reverse this string" become correct, and what breaks at each of the other three?
3. Under "invert it", indexing positions by content gives an inverted index. What's the space cost relative to the text, and which single property of natural language makes it affordable?
4. A rope augments each subtree with its byte and line counts. Derive how that makes "jump to line 4,000" Θ(log n), and name what other augmentation would give you "jump to grapheme 4,000."
5. Under "parallelize it", chunked scanning must not split a code point. Give the algorithm for choosing chunk boundaries, and explain why UTF-8's design makes it Θ(1) per boundary rather than a rescan.
6. Interning turns string comparison into `u32` comparison. What did you give up, and what happens to the intern table in a long-lived process handling unbounded distinct strings?
7. Under "amortize it" you get the gap buffer. Compare it to a rope for a 100 MB file: which wins for typing, which for a scripted find-and-replace across the document, and why?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Give the four levels of "character" and the count of each for `"café"` and `"👨‍👩‍👧"`. Which does `.len()` return, and which does `.chars().count()`?
2. Why does `&s[0..4]` panic on `"café"` while `s.get(0..4)` doesn't? What does each return or do?
3. `"café" != "café"`. Explain the mechanism and the one-line fix at the boundary.
4. Name three case conversions that change the character count, and say why `char::to_uppercase` returns an iterator.
5. A log processor allocates 10M `String`s per run. What was the alternative, and what design constraint does adopting it impose?
6. Which type for: a function parameter, a struct field built once and stored a million times, a parsed token, an OS file path, and bytes off a socket that may not be text?

Build exercises:

- Write `truncate_display(s: &str, cols: usize) -> &str` three times — by bytes, by chars, and by graphemes with `unicode-width` — then run all three on `"café"`, `"👨‍👩‍👧"`, `"🇮🇳"`, and a string with combining accents. Tabulate what each produces. This one exercise makes the four-level model permanent.
- Build a zero-allocation CSV/log parser: `struct Record<'a>` with `&'a str` fields, `memchr` for delimiters. Benchmark against the `to_string()`-per-field version at 1M lines, and report the allocation count from `dhat` for both.
- Reproduce the normalization bug: build a `HashSet<String>` of usernames, insert an NFC `"café"` and an NFD `"café"`, and assert the set size. Then add normalization at the boundary and watch it become 1.
- Implement a rope with byte and line counts per node, supporting insert, delete, and "jump to line n". Compare insert-at-front against `String::insert` on a 100 MB document — this is the exercise that shows why editors don't use flat strings.

## Open Questions

- Does `compact_str` actually win for the "millions of short identifiers" case here, or does interning to `u32` dominate it? Measure both against `String` on a realistic symbol table.
- How much faster is `memchr` than `str::find` for single-byte delimiters on this machine, and at what haystack length does the SIMD version start winning?
- What's the real cost of NFC normalization per KB of typical user text — is boundary normalization ever too expensive to do unconditionally?
- `ropey` vs a gap buffer for a code editor: at what document size does the rope's Θ(log n) actually beat the gap buffer's locality?
- Is there a safe stable equivalent of `floor_char_boundary` yet, or is the `char_indices` scan still the idiom?

## References

- [The Rust Book, ch. 8.2 "Storing UTF-8 Encoded Text with Strings"](https://doc.rust-lang.org/book/ch08-02-strings.html) — the four-level framing and why indexing is disallowed.
- Joel Spolsky, ["The Absolute Minimum Every Software Developer Absolutely, Positively Must Know About Unicode and Character Sets"](https://www.joelonsoftware.com/2003/10/08/the-absolute-minimum-every-software-developer-absolutely-positively-must-know-about-unicode-and-character-sets-no-excuses/) — still the best 20-minute grounding in encoding.
- [UAX #29: Unicode Text Segmentation](https://unicode.org/reports/tr29/) — the actual grapheme-cluster rules behind `unicode-segmentation`; skim it once to see why "one character" is genuinely hard.
- [UAX #15: Unicode Normalization Forms](https://unicode.org/reports/tr15/) — NFC/NFD/NFKC/NFKD and when each applies.
- [`memchr` crate](https://docs.rs/memchr/) — SIMD substring/byte search; read the docs for why hand-rolling a matcher rarely wins.
- [`ropey`](https://docs.rs/ropey/) — a production rope; the source is a good worked example of the "augment it" lens.
- Related topics in this repo: [Arrays & Dynamic Arrays](../arrays-and-dynamic-arrays/learning.md) (`String` is a `Vec<u8>` with an invariant), [Complexity Analysis](../complexity-analysis/learning.md) (the O(k)-in-key-length trap that interning solves), [Serialization & Encoding](../../performance-optimization/serialization-and-encoding/learning.md) (why text parsing is slow and what zero-copy buys), [Zero-Copy](../../performance-optimization/zero-copy/learning.md) (borrow-don't-own applied to text).
