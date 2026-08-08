# Serialization & Encoding — Learning Notes

## The Hardware Mechanism

Serialization is the toll booth between two representations of the same data: the **layout-optimized** in-memory form ([structs shaped for cache lines](../memory-layout/learning.md), pointers, native endianness) and the **contract-optimized** wire form (self-contained bytes, portable, versionable). Every boundary crossing — network, disk, IPC, cache store — pays the toll, and the toll is built from machine costs already catalogued in this repo:

- **Parsing is branch work.** A text format like JSON is decoded byte by byte: every character is a decision (`{`? `"`? digit? escape?) driven by *data* — [data-random branching](../branch-prediction/learning.md) at its purest, which is why naive JSON parsing runs at ~100s of MB/s while the same machine memcpys at tens of GB/s, and why simdjson's [mask-based branchless design](../simd/learning.md) buys an order of magnitude. Number parsing (`"3.14159"` → f64) and UTF-8 validation are their own branch-and-multiply gauntlets.
- **Materializing is allocation work.** Decoding into owned structures (`String` fields, `Vec`s, nested maps) is an [allocation](../allocation-strategies/learning.md) per node — a deserialized tree is a walk through the allocator, and the resulting objects are heap-scattered ([the locality tax](../cache-locality/learning.md)) before your code touches them.
- **Encoding is copy-and-format work.** Writing bytes out is bounded by [copies and buffer discipline](../zero-copy/learning.md) (assemble-then-send vs. write-into-one-buffer) plus formatting cost (integer→decimal is surprisingly expensive; binary formats skip it entirely).
- **The escape exists:** if the wire format *is* a valid memory layout ([`repr(C)`-disciplined](../memory-layout/learning.md), offsets instead of pointers), decoding can be **zero work** — point at the bytes and read fields in place. That's the rkyv/FlatBuffers/Cap'n Proto family, and it converts deserialization cost into a *trust/validation* question, which is the most interesting trade in this topic.

## Mental Model

**A format is a position in a three-axis trade — bytes on the wire, CPU to encode/decode, and evolvability of the contract — and no format wins all three. Choose per boundary, not per project.**

1. **The spectrum, by how much machine work decode does:**
   - **Human text** (JSON, YAML): schema-in-band, `curl`-able, universally supported — and maximally expensive: per-byte branches, number parsing, escape handling, then full materialization. Its *debuggability is a real feature* that pure benchmarks under-price.
   - **Compact self-describing binary** (MessagePack, CBOR): same data model as JSON with cheap length-prefixed tokens — ~2× smaller, several-× faster, still materializes. The mild upgrade.
   - **Schema'd binary** (protobuf, and Rust-native `bincode`/`postcard`): the schema lives *outside* the bytes. Protobuf: field tags + varints — compact, evolvable by design (field numbers), moderate decode cost (varint unpacking is branchy). Postcard/bincode: near-memcpy for numeric/POD data — the fast end of *parsing* formats, with almost no evolution story (below).
   - **Access-in-place** (rkyv, FlatBuffers, Cap'n Proto): encode once into a layout readable *as memory* (relative offsets, aligned fields); "decode" = cast + field access. Zero parse, zero materialization, [zero-copy's](../zero-copy/learning.md) whole doctrine applied to structured data — mmap a 10 GB archive and read one field in microseconds. The bill arrives as **trust**: on untrusted bytes you must validate the structure (rkyv's `bytecheck` — real cost, sometimes rivaling a parse) or you've built a UB machine; on trusted bytes (your own cache, your own files) it's the closest thing to free deserialization that exists.
2. **The three costs rank differently per workload:** bytes-on-wire matters when [bandwidth or RTT dominates](../batching-and-amortization/learning.md) (WAN, mobile, storage volume); decode CPU matters at ingest scale ([the funnel decides](../profiling-and-measurement/learning.md) — a service doing 500 K decode/s lives here); allocation profile matters for latency tails ([allocator churn](../allocation-strategies/learning.md)). String-heavy vs. numeric-heavy payloads reorder the rankings — *your* message shape is the only benchmark that counts.
3. **Evolvability is the architecture-side axis** — and this repo has met it before: the [event-sourcing schema-evolution pitfall](../../architecture-patterns/event-sourcing/learning.md) and the [outbox's payload-as-contract rule](../../architecture-patterns/outbox-pattern/learning.md) are serialization-format decisions wearing architecture clothes. Protobuf's field numbers *are* the additive-with-defaults discipline, industrialized: unknown fields skip, missing fields default, numbers are never reused. Formats without that story (`bincode`, raw `postcard`, memcpy'd `repr(C)`) are **fragile across versions by construction** — fine for ephemeral hops, disqualifying for anything durable.
4. **serde is Rust's meta-layer, and worth seeing clearly:** one `#[derive]` targets every format — the visitor architecture compiles to *static* per-format code (no runtime dispatch; genuinely fast), supports [`#[serde(borrow)]`](../zero-copy/learning.md) for zero-copy string/bytes fields, and costs you compile time (monomorphization per type × format) plus occasional impedance with formats that don't fit its data model (which is exactly why rkyv bypasses serde entirely — its data model *is* the archived layout).

## Worked Example

One message — a market-ish event (`u64` id, `u64` timestamp, `f64` price, `u32` qty, two short strings, a 6-element `Vec<f64>`) — through five formats. Illustrative numbers (typical desktop, ~150-byte logical payload; reproducing on *your* message shapes is exercise one):

```
format         size     encode    decode→owned   decode allocs   notes
serde_json     412 B    1.9 µs    3.4 µs         7               numbers+escapes dominate
MessagePack    198 B    0.6 µs    1.1 µs         7               same tree, cheaper tokens
protobuf       151 B    0.35 µs   0.55 µs        4               varints; evolvable
postcard       118 B    0.09 µs   0.16 µs        4               near-memcpy; no evolution
rkyv           176 B    0.14 µs   0.004 µs*      0               *access, not decode
rkyv+validate  176 B    0.14 µs   0.19 µs        0               bytecheck on untrusted bytes
```

Readings, each one a model point: **JSON→MessagePack** (~3×) is the price of *tokens*, same materialization. **→protobuf/postcard** is the price of *schema-out-of-band* — and postcard's edge over protobuf is varint/tag overhead vs. raw layout. **rkyv's 4 ns "decode"** is the category change: it's a pointer cast — but add `bytecheck` for untrusted input and it lands back near protobuf: **access-in-place's win is conditional on the trust boundary**, which is why it shines for *your own* mmap'd caches and archives, not for internet-facing inputs. **The alloc column** is the latency-tail story: 7 → 0 via [`#[serde(borrow)]`](../zero-copy/learning.md) on the string fields would cut JSON's and MessagePack's columns too — format choice and [lifetime-regime choice](../zero-copy/learning.md) compose.

Second lens — **the pipeline changes the ranking**: gzip/zstd the JSON and it lands near 160 B — if the boundary is WAN and bytes are the binding cost, *compressed JSON is suddenly competitive on the size axis* while charging even more CPU. Compression is a fourth format axis (cheap bytes, expensive CPU), and `zstd` with a trained dictionary on small messages is the under-used trick for event streams ([outbox](../../architecture-patterns/outbox-pattern/learning.md)/[event-store](../../architecture-patterns/event-sourcing/learning.md) payloads).

## Applying It

- **Default map, by boundary:** public/debuggable edges → JSON (serde_json; `simd-json` when bulk ingest shows up in the profile); cross-service internal → protobuf (`prost`/`tonic`) for the evolution story; ephemeral internal hops (IPC, task queues, sidecar links) → `postcard` (schemaless speed where both ends deploy together); durable + evolvable (events, archives) → protobuf or versioned-envelope + [upcaster discipline](../../architecture-patterns/event-sourcing/learning.md); mmap'd caches/indexes you produce and consume → rkyv (trusted, validated at build).
- **Encode into reused buffers:** every serializer worth using writes into a caller's buffer (`serde_json::to_writer`, `prost::Message::encode` into `BytesMut`) — the [allocation](../allocation-strategies/learning.md) and [zero-copy](../zero-copy/learning.md) disciplines applied at the boundary; returning fresh `Vec<u8>` per message is the accidental-allocation lint list's boundary edition.
- **Decode with intent:** typed structs, not `serde_json::Value` (the DOM is a materialization sinkhole — every object a map, every string owned); `#[serde(borrow)]` for string/bytes fields consumed within the buffer's scope; streaming (`Deserializer::from_reader`/iterative) for documents too big to tree.
- **Schema hygiene per format:** protobuf — field numbers are forever (never reuse; reserve deleted ones), unknown-field passthrough on middleboxes; postcard/bincode — version byte in front and *both ends deploy together*, or don't; rkyv — treat archived layouts as frozen contracts (evolution is manual and sharp).
- **Validation posture for access-in-place:** untrusted → `bytecheck`, always, and re-benchmark honestly with it; trusted → checksum/signature at the envelope ([encryption doc's](../../architecture-patterns/encryption-and-key-management/learning.md) AEAD covers integrity too — an authenticated envelope can *earn* the trust that skips validation).
- **Cross-cutting:** compression decided *with* the format (measure size post-zstd, not raw); floats round-tripping through JSON lose exactness bit-patterns sometimes (and [reordered sums differ anyway](../simd/learning.md)) — canonical binary for anything reconciled; timestamps/decimals as integers (cents, nanos), never floats, at money-touching boundaries.

## When It Hurts

- **Premature binary.** Replacing JSON at a low-volume edge trades away `curl`, browser DevTools, log greppability, and every teammate's fluency — for microseconds nobody was losing. The [funnel](../profiling-and-measurement/learning.md) rules here like everywhere: serialization must *appear in the profile* before it's worth a format migration.
- **`bincode`/`postcard` as a durable format — the trap with a name.** Non-self-describing, layout-coupled bytes in an event store or long-lived file: the first struct change strands every old record (the [event-sourcing immutability trap](../../architecture-patterns/event-sourcing/learning.md), self-inflicted at the encoding layer). Durable data gets an evolution story — schema'd format, version envelope + upcasters — *before* the first byte is written.
- **rkyv on untrusted input without validation is UB-as-a-service:** attacker-controlled offsets walked as references. `bytecheck` or an authenticated envelope is not optional there — and with validation priced in, the honest comparison often shrinks to "protobuf with extra steps." Know which side of the trust boundary each consumer sits on.
- **`Value`-shaped code:** building on dynamic JSON trees (`Value`, `HashMap<String, Value>`) where a typed struct would do — every access re-branches, every string owns, [locality](../cache-locality/learning.md) is destroyed. `Value` is for genuinely-unknown shapes; everything else derives.
- **Monomorphization bills:** serde across many types × many formats is a known compile-time and code-size line item; `erased-serde` or fewer formats when build times hurt. ([Compiler doc's](../compiler-optimizations/learning.md) territory.)
- **The boundary may not be the cost:** at 1 KB messages over WAN, [RTT and the N+1 shape](../batching-and-amortization/learning.md) dwarf any decode difference — batch the messages first, then discuss formats. Serialization tuning below a dominant network cost is deck-chair arrangement.

## Benchmarking Methodology

- **Three axes + allocs, always:** size, encode ns, decode ns, decode allocations ([dhat](../profiling-and-measurement/learning.md)) — a format table missing an axis is an argument, not a measurement. Run on **your** message shapes: string-heavy vs numeric payloads reorder every ranking.
- **Realistic instances:** `Default::default()` benchmarks flatter everyone (empty strings, zero Vecs); sample real traffic or generate to production distributions (sizes, string lengths, optional-field density).
- **Include the honest variants:** rkyv *with* bytecheck for untrusted paths; JSON *with* simd-json if that's the proposal; sizes *post-compression* if compression is in the pipeline; decode *to the owned/borrowed form the code actually uses*.
- **Round-trip property tests** beside the perf suite (encode→decode == identity, cross-version decode for evolvable formats) — a fast serializer that corrupts one f64 pattern is a very slow bug.
- **End-to-end check:** after choosing, verify at the [macro baseline](../profiling-and-measurement/learning.md) — decode wins that vanish behind RTT confirm the boundary wasn't the cost.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Why is JSON parsing branch-bound, mechanically? Which two other machine costs ride along, and which repo docs own each?
2. Place the five families on the three-axis trade. Which axis does each sacrifice?
3. Why is rkyv's decode 4 ns *and* why is that number conditional? State the trust-boundary rule and what bytecheck does to the comparison.
4. A colleague proposes bincode for the event store ("it's the fastest"). Give the two-part rejection, naming the architecture pitfall it recreates.
5. Protobuf field numbers: what do "never reuse" and unknown-field passthrough buy, and which event-sourcing mechanism are they the industrialized form of?
6. When is compressed JSON the right answer over a binary format? Which axis flipped?
7. Your decode profile shows 7 allocs/message, all strings. Two fixes, and what each demands of the calling code's lifetimes.

Measurement exercises:

- Run the five-format shootout on *your own* dominant message type (real instances, all four columns, rkyv both with and without validation). Compare your table's shape to the doc's; explain the biggest reordering.
- Take the JSON path and apply `#[serde(borrow)]` to every string field: measure decode time and allocs before/after, and document what broke at the call sites (the lifetime bill, itemized).
- Event-stream compression: zstd with and without a trained dictionary on 10 K small (~200 B) messages — size and CPU per message; find where dictionary training pays.

## Open Questions

- `simd-json` vs `serde_json` on this machine's real payloads — the claimed multiples, verified, and the API friction documented.
- rkyv's evolution story in practice: what does a versioned archived format actually look like (enum-wrapped roots? parallel archives?) — prototype before trusting it for anything long-lived.
- `prost` vs `protobuf` crate vs `quick-protobuf`: decode allocation profiles compared (prost's `String` fields vs borrowed alternatives).
- Postcard's `#[serde(borrow)]` interaction: how far can zero-alloc decode go for the ephemeral-hop use case?
- Columnar boundaries: when does the answer stop being "a message format" and become Arrow/Parquet ([DoD's](../data-oriented-design/learning.md) columnar kinship at the serialization layer)?

## References

- David Koloski, [rust_serialization_benchmark](https://github.com/djkoloski/rust_serialization_benchmark) — the living format-shootout for Rust; check your candidates' current numbers here before believing any blog post (including this doc).
- [rkyv book](https://rkyv.org/) — access-in-place from its author, including the validation and evolution honesty.
- [Protobuf encoding docs](https://protobuf.dev/programming-guides/encoding/) — varints, tags, and the field-number discipline; short and canonical.
- [serde book](https://serde.rs/) — the data model and `borrow` semantics ([lifetimes chapter](https://serde.rs/lifetimes.html) already cited by the zero-copy doc).
- Langdale & Lemire, [simdjson paper](https://arxiv.org/abs/1902.08318) — already in the [SIMD doc](../simd/learning.md); reread it now as *serialization* literature.
- Related topics in this repo: [Zero-Copy](../zero-copy/learning.md) (borrow/`Bytes` regimes at the boundary), [Allocation Strategies](../allocation-strategies/learning.md) (materialization = the allocator walk), [Branch Prediction](../branch-prediction/learning.md) + [SIMD](../simd/learning.md) (why text parsing is slow and how simdjson isn't), [Batching](../batching-and-amortization/learning.md) (RTT dominates small messages), [Event Sourcing](../../architecture-patterns/event-sourcing/learning.md) + [Outbox](../../architecture-patterns/outbox-pattern/learning.md) (evolvability as the architecture-side axis).
