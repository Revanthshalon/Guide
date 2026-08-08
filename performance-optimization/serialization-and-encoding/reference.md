# Serialization & Encoding — Quick Reference

Core model: a format is a position on three axes — bytes-on-wire, encode/decode CPU, evolvability — plus an allocation profile; nothing wins all of them. Choose per *boundary*, not per project. Decode cost = branches (text) + materialization (allocs) + copies; access-in-place converts decode into a trust/validation question. Details in [learning.md](learning.md).

## Format Map by Boundary

| Boundary | Format | Why |
| --- | --- | --- |
| Public/debuggable edge | JSON (`serde_json`; `simd-json` at bulk) | curl-ability is a real feature; profile before replacing |
| Cross-service internal | protobuf (`prost`/`tonic`) | Field numbers = industrialized additive evolution |
| Ephemeral internal hop (both ends co-deploy) | `postcard` | Near-memcpy; **no evolution story — ephemeral only** |
| Durable (events, archives) | protobuf, or versioned envelope + upcasters | Never bare bincode/postcard — the immutability trap |
| Own mmap'd caches/indexes | rkyv | Access-in-place; trusted or bytecheck'd |

## The Worked-Example Shape (illustrative; run yours)

| Format | Size | Decode | Allocs |
| --- | --- | --- | --- |
| serde_json | 412 B | 3.4 µs | 7 |
| MessagePack | 198 B | 1.1 µs | 7 |
| protobuf | 151 B | 0.55 µs | 4 |
| postcard | 118 B | 0.16 µs | 4 |
| rkyv (trusted) | 176 B | **4 ns** (cast) | 0 |
| rkyv + bytecheck | 176 B | 0.19 µs | 0 |

## Rules of Thumb

- rkyv on untrusted bytes without bytecheck = UB-as-a-service; with it, the comparison often shrinks to protobuf-with-extra-steps. Know the trust boundary per consumer.
- Typed structs, never `serde_json::Value`, unless the shape is genuinely unknown.
- `#[serde(borrow)]` on string/bytes fields = allocs → 0, lifetime bill attached.
- Encode into reused buffers (`to_writer`, `encode` into `BytesMut`) — never fresh `Vec<u8>` per message.
- Protobuf: field numbers are forever; reserve deletions; pass unknown fields through.
- Compression is a fourth axis: measure size post-zstd (compressed JSON can win the size axis); trained dictionaries pay on small event payloads.
- Money/reconciled data: integers (cents, nanos) and canonical binary — JSON float round-trips and reordered sums are semantics decisions.
- Below a dominant RTT, format tuning is deck-chairs — batch first (N+1 lesson).
- String-heavy vs numeric payloads reorder every ranking — only your message shape counts.

## Benchmark Checklist

- [ ] All four columns: size, encode, decode, allocs (dhat) — on real instances, not `Default::default()`
- [ ] Honest variants: rkyv *with* validation for untrusted; sizes post-compression; decode to the form the code actually uses
- [ ] Round-trip + cross-version property tests beside the perf suite
- [ ] Macro re-check: does the decode win survive RTT?

## Key References

- Koloski, [rust_serialization_benchmark](https://github.com/djkoloski/rust_serialization_benchmark) — check current numbers before believing anyone.
- [rkyv book](https://rkyv.org/) + [protobuf encoding guide](https://protobuf.dev/programming-guides/encoding/).
- [serde lifetimes](https://serde.rs/lifetimes.html) — the borrow semantics.
