# Regular Expressions — Learning Notes

## Mental Model

**A regex pattern is not one thing — it's a specification that gets executed by one of two fundamentally different engines, and which one decides both what the pattern can express and how dangerous it is.**

1. **Backtracking engines** (PCRE, and what's behind Perl, Python's `re`, Java, JavaScript, Ruby, `grep -P`, `rg -P`). These try a match, and on failure, rewind and try a different way of splitting the input — recursively. This gives you real expressive power (backreferences, lookaround) at the cost of **worst-case exponential time** on certain patterns.
2. **Finite-automaton engines** (RE2, Go's `regexp`, Rust's `regex` crate — which is what `rg` uses by default). These compile the pattern into a state machine and run it in a single linear pass over the input, with **no backtracking possible**. This guarantees linear time, but the trade-off is structural: no backreferences, no lookaround, because those features require remembering match history that a pure automaton can't express.

This isn't an implementation detail you can ignore — it's the single fact that predicts whether a pattern from an untrusted source can hang your process. See [ripgrep & grep](../ripgrep-and-grep/learning.md#regex-engine-choice) for the concrete case (`rg` vs `rg -P`).

The second idea worth holding permanently: **a regex flavor is a dialect, not a standard.** "Regex" the word covers at least four incompatible syntaxes in daily use — POSIX BRE, POSIX ERE, PCRE, and each language's own variant (Rust's `regex` crate, JavaScript's `RegExp`) — and the same three characters (`(`, `+`, `\d`) mean different things, or nothing, depending on which one you're in. Assuming "regex is regex" is the single most common source of "it silently matched nothing" bugs.

## The Model in Detail

### Backtracking vs. automaton is a feature/safety trade-off, not a quality difference

- **What it is:** Backtracking engines support backreferences (`(\w+)\s+\1` — match a repeated word) and lookaround (`(?<=foo)bar`, `foo(?!bar)`) because they can explore multiple parse paths and remember what matched where. Automaton engines (RE2, Rust `regex`) cannot support these *by construction* — a finite automaton has no memory of "how did I get here," only "what state am I in."
- **Why it matters in practice:** If you need a backreference or lookaround, you need a backtracking engine, full stop — there is no automaton-based workaround for genuine backreferences. But if you don't need them (and most patterns don't), preferring the automaton engine converts an entire bug class (ReDoS) into a structural non-issue rather than something you have to review for.

### Greedy, lazy, and possessive quantifiers change what "backtrack" costs

- **What it is:** `a*`, `a+`, `a{2,5}` are **greedy** — they match as much as possible, then give characters back if the rest of the pattern fails. `a*?`, `a+?` are **lazy** — match as little as possible, then take more if needed. `a*+` (PCRE/Java only) is **possessive** — match as much as possible and *never* give characters back, even if the rest of the pattern then fails to match at all.
- **Why it matters in practice:** Greedy vs. lazy is a correctness question (`<.*>` on `<a><b>` matches the whole string; `<.*?>` matches just `<a>`). Possessive (and its cousin, atomic groups `(?>...)`) is a **performance** answer specific to backtracking engines: it forbids the engine from re-exploring a subexpression, which is exactly what defuses catastrophic backtracking in nested-quantifier patterns — at the cost of occasionally rejecting a match a fully-backtracking version would have found.

### Nested quantifiers are where backtracking becomes exponential

- **What it is:** A pattern like `(a+)+b` against a string of 30 `a`s with no trailing `b` has exponentially many ways to partition those 30 `a`s across the inner and outer `+`. The engine tries all of them before concluding failure.
- **Why it matters in practice:** This is **ReDoS** (Regular Expression Denial of Service), and it's not a rare pathological case — `(\w+\s?)+$`, `(.*)*`, and `([a-zA-Z]+)*` are all real-world variants of the same shape, and they show up in URL validators and email regexes copy-pasted from Stack Overflow. Any pattern with a quantified group that can itself match an empty-or-overlapping-with-itself substring, nested inside another quantifier, is a candidate. The fix is structural (rewrite to remove the ambiguity, use possessive quantifiers/atomic groups, or switch to a linear-time engine) — there is no flag that makes a backtracking engine safe against an adversarial pattern *and* adversarial input simultaneously.

### `^`/`$` and `.` mean different things depending on mode flags

- **What it is:** By default in most engines, `^` and `$` match start/end of the *whole string*, and `.` does not match `\n`. **Multiline mode** (`(?m)`, or a tool-level flag) makes `^`/`$` match start/end of *each line*. **Dotall/single-line mode** (`(?s)`) makes `.` match `\n` too. These are independent flags that are easy to conflate because the word "multiline" is used inconsistently across tools for one or the other or both.
- **Why it matters in practice:** A pattern built and tested against a single line silently stops matching (or over-matches) the moment the input spans multiple lines, and which flag fixes it depends on which failure mode you have. This is the same underlying issue as `rg`/`sed`/`awk` being line-oriented by default (see [sed & Text Processing](../sed-and-text-processing/learning.md)) — regex mode flags are the fine-grained control over that assumption.

### `\d`, `\w`, `\b` have a hidden dependency on character encoding and locale

- **What it is:** In ASCII/byte mode, `\d` is `[0-9]` and `\w` is `[A-Za-z0-9_]`. Under Unicode mode (default in Python 3, JavaScript with `/u`, Rust's `regex` crate), `\d` matches *any* Unicode decimal digit (including Arabic-Indic digits, full-width digits) and `\w` matches any Unicode "word" codepoint. `\b` (word boundary) is defined relative to whatever `\w` means in that mode.
- **Why it matters in practice:** A pattern validated against ASCII test data can behave differently against real-world Unicode input — sometimes correctly matching more than intended (silently accepting non-ASCII digits where only `0-9` was meant), sometimes failing to match what a human would call a "word boundary" in a non-Latin script. Byte-oriented tools (classic `grep`, `sed` in the C locale) and codepoint-oriented engines (most modern regex libraries) disagree here, and the failure is silent, not an error.

### Capturing groups have a cost and a naming answer

- **What it is:** `(...)` captures the matched text for later reference (`\1`, `$1`, named-group APIs); `(?:...)` groups without capturing. Named groups (`(?<name>...)` in PCRE/.NET, `(?P<name>...)` in Python) attach a label instead of a position.
- **Why it matters in practice:** Every capturing group the engine doesn't need to remember is wasted bookkeeping, and in a hot loop over many lines this is measurable. More importantly, positional captures (`\1`, `\2`) silently renumber when someone inserts a group earlier in the pattern during a later edit — a correct-looking diff that breaks the replacement. Named groups make that class of bug impossible to introduce silently.

## Portability & Variants

**Flavor is the actual portability boundary — not the tool.** The same three-character sequence means different things depending on which flavor you're in:

| Flavor | Where you'll meet it | `+` `?` `|` `(` | Lookaround | Backreferences | Time complexity |
| --- | --- | --- | --- | --- | --- |
| **POSIX BRE** | `grep`, `sed` (default) | Literal — must escape (`\+`) | No | `\1`–`\9` only | Backtracking (impl-defined) |
| **POSIX ERE** | `grep -E`, `sed -E`, `awk` | Metacharacters | No | No (in the standard) | Backtracking (impl-defined) |
| **PCRE / PCRE2** | `grep -P`, `rg -P`, most language stdlibs (historically) | Metacharacters | Yes | Yes | Backtracking — exponential worst case |
| **RE2 / Rust `regex`** | `rg` (default), Go `regexp` | Metacharacters | No | No | Automaton — linear guaranteed |
| **JavaScript `RegExp`** | browsers, Node | Metacharacters | Yes (lookbehind since ES2018) | Yes | Backtracking |

This machine's tools, concretely (see the tool-specific docs for the full picture): `grep` here is `ugrep`, `rg` is ripgrep — both default to a non-PCRE mode and require an explicit flag (`-P`) to opt into backtracking with lookaround/backreferences.

**The one habit that avoids most of this table:** write patterns assuming ERE-or-better syntax (metacharacters are metacharacters), and reach for `-E`/`-P` explicitly rather than relying on a tool's default. Test any pattern with lookaround or a backreference against the *specific* tool you'll run it with — "it works in my editor's regex tester" does not imply it works in `sed`.

## Pitfalls in Depth

### Pitfall: Catastrophic backtracking (ReDoS)

- **What goes wrong:** A pattern that has run fine in code review and testing suddenly pins a CPU core at 100% and never returns, taking down a request thread or a CI job. It reproduces only with certain (often attacker-chosen) input.
- **Why it happens (the mechanism):** A backtracking engine facing a quantified group nested inside another quantifier — `(a+)+`, `(a|a)*`, `(\w+\s?)+` — explores every way of partitioning matching input across both quantifiers before it can conclude "no match." The number of partitions grows exponentially with input length. This is purely a property of the pattern and the engine; it has nothing to do with how the input is generated.
- **How to handle it, and why that works:** Prefer a linear-time engine (RE2, Rust `regex`, `rg`'s default mode) for anything that touches untrusted input — the guarantee is structural, not a matter of writing a "safe" pattern. If you must use a backtracking engine, remove the ambiguity: `(a+)+` becomes `a+` if the intent was "one or more a's" (the outer quantifier was redundant), or use possessive quantifiers/atomic groups (`a++`, `(?>a+)+`) to forbid re-exploration. Tools like `safe-regex` (Node) or manual review against known-bad shapes catch this before it ships.
- **Trade-offs of the fix:** Rewriting to remove ambiguity sometimes changes what the pattern matches at the edges (empty-string cases especially) — test the rewrite against the same cases the original was written for. Switching engines loses backreferences/lookaround if the pattern genuinely needs them.

### Pitfall: Greedy `.*` swallowing more than intended

- **What goes wrong:** `<.*>` against `<b>bold</b> and <i>italic</i>` matches the entire string from the first `<` to the last `>`, not just `<b>`.
- **Why it happens (the mechanism):** `.*` is greedy: it first matches the entire rest of the line, then backtracks one character at a time until the rest of the pattern (`>`) can match — which happens at the *last* `>` in the string, not the first.
- **How to handle it, and why that works:** Use a lazy quantifier (`<.*?>`) to match the shortest possible span, or better, a negated character class (`<[^>]*>`) which cannot cross a `>` at all and doesn't need to backtrack to find one. The character-class form is also faster, since it can't over-match and retry.
- **Trade-offs of the fix:** A negated character class assumes the delimiter character (`>`) can't legitimately appear inside the content — true for simple tag matching, false for genuinely nested structures (which regex can't correctly parse anyway — see the next pitfall).

### Pitfall: Reaching for regex on data that has real structure

- **What goes wrong:** A regex is written to pull a value out of JSON, HTML, or a programming language's source. It works on the sample input and breaks the moment key order changes, whitespace changes, values contain the delimiter, or the structure nests one level deeper than the test case did.
- **Why it happens (the mechanism):** Regular expressions correspond to finite automata and cannot count or match arbitrarily deep, balanced nesting (this is a consequence of the pumping lemma, not a missing feature) — a regex can express a fixed textual shape, not a recursive grammar. JSON, HTML, and source code are all recursive grammars.
- **How to handle it, and why that works:** Use an actual parser for the format — `jq`/`yq` for JSON/YAML, an HTML/DOM parser, `ast-grep` or the language's own AST for source code. These operate on structure, so formatting differences are irrelevant by construction.
- **Trade-offs of the fix:** More tooling, and a real learning curve for query languages like `jq`. For genuinely flat, line-oriented data (logs, CSV without embedded delimiters), a regex remains the right and simpler tool — the trigger for switching is *nesting*, not *complexity*.

### Pitfall: Building a pattern from untrusted or unescaped input

- **What goes wrong:** `re.compile(user_input)` or `sed "s/$var/.../"` either throws on unexpected metacharacters in the input, matches far more than intended (a literal `.` in a filename matching *any* character), or — if the input is attacker-controlled — becomes a ReDoS or injection vector.
- **Why it happens (the mechanism):** Regex metacharacters (`.`, `*`, `+`, `(`, `[`, `\`) are common in real-world literal strings (filenames, IP addresses, version numbers) but are never escaped automatically when a string is substituted into a pattern.
- **How to handle it, and why that works:** Use the language's or tool's literal-escaping function before interpolating untrusted text into a pattern — `re.escape()` in Python, `RegExp.escape` (where available), or a tool flag that treats the input as a fixed string entirely (`rg -F`, `grep -F`) when no regex features are actually needed on that portion.
- **Trade-offs of the fix:** Escaping is one more step that's easy to forget on the "obviously safe" code path; treat it as mandatory whenever the string's origin isn't a compile-time literal.

## Open Questions

- Where's the practical crossover point where `ast-grep`'s structural matching is worth the setup cost over a well-anchored regex, for this repo's own doc/cross-link consistency checks?
- Does Rust's `regex` crate's lack of backreferences ever actually bite in this codebase's tooling, or is it consistently a non-issue given what these docs need to match?

## References

- [regular-expressions.info](https://www.regular-expressions.info/) — the most thorough cross-flavor reference available
- [Russ Cox, "Regular Expression Matching Can Be Simple And Fast"](https://swtch.com/~rsc/regexp/regexp1.html) — the automaton-vs-backtracking argument from the RE2 author, and why linear time is achievable
- [OWASP: ReDoS](https://owasp.org/www-community/attacks/Regular_expression_Denial_of_Service_-_ReDoS) — pattern shapes to avoid and how to test for them
- [`regex` crate syntax docs](https://docs.rs/regex/latest/regex/#syntax) — what `rg` accepts by default
- Related in this repo: [ripgrep & grep](../ripgrep-and-grep/learning.md) (engine choice as a security decision), [sed & Text Processing](../sed-and-text-processing/learning.md) (BRE vs ERE in practice), [String Matching](../../data-structures-and-algorithms/string-matching/learning.md) (the automaton/SIMD machinery underneath)
