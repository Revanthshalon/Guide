# Zero-Copy — Quick Reference

Core model: trace source→sink, count byte duplications, delete by referencing in place. A copy costs bandwidth ×2 + cache pollution ×2 + usually an allocation. Two disciplines: don't duplicate in-process (borrow/refcount views), don't ferry across the kernel (sendfile/mmap/io_uring). Views are for transit, copies are for retention. Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| memcpy/clone/malloc wide in the profile; GB-scale pipelines | Items ≲ few hundred B — copy is ~2 ns, bookkeeping costs more |
| Parse-and-route shapes (views can flow) | Data retained long-term (amplification trap — copy deliberately) |
| CPU ferrying kernel↔user for data it never reads | Borrow lifetimes would infect every API signature |
| Large read-mostly files (mmap territory) | Mutation needed — read-sharing only |

## The Lifetime Regimes

| Regime | Tool | Scope | Liability |
| --- | --- | --- | --- |
| Borrowed view | `&str`/`&[u8]`, `#[serde(borrow)]`, `Cow` | Dies with the buffer's scope | Lifetime infection; pins buffer against reuse |
| Refcounted view | `bytes::Bytes` slices, `Arc<[u8]>` | Crosses tasks/threads/queues | Amplification: 40 B slice pins 4 MB chunk |
| Kernel-side | `io::copy` specializations, sendfile, `memmap2`, io_uring | Process never touches bytes | Silently reversible (wrappers defeat it); mmap SIGBUS on truncation |
| Deliberate copy | `to_owned`, `Bytes::copy_from_slice` | Retention, decoupling | The budget spent — fine when chosen |

## Rules of Thumb

- Parse to views by default; make `to_owned` visible and deliberate.
- Retention = copy: caches/session/dedup stores never hold `Bytes` slices of big chunks.
- Encoders write into caller's `BytesMut`; assemble with `write_vectored`/`IoSlice`, not concat-Vec.
- `Cow` when most items borrow, some transform.
- mmap: large + read-mostly + long-lived + immutable-by-convention; first touch = latency spike; truncation = SIGBUS.
- Verify kernel specialization with `dtruss`/`strace` — a `BufReader` wrapper silently defeats `sendfile`.
- Contended `Bytes` clones = refcount line ping-pong (false-sharing territory).
- RSS vs live-bytes divergence is the amplification signature — monitor it.

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Copy = bandwidth | 2× data size in traffic, both ways through cache |
| 64 B copy | ~2 ns — cheaper than one refcount atomic (~10–20 ns) |
| Copy-vs-view crossover | ~low hundreds of bytes (measure yours) |
| read+write serving loop | 2 CPU copies, both deletable (sendfile: 0) |
| Worked example v0 → v1 | ~0.9 → ~4.8 GB/s; allocs/record 2 → 0 |

## Benchmark Checklist

- [ ] Allocs/item (dhat) and reasoned copies/byte before timing
- [ ] RSS beside throughput for any Bytes/mmap work
- [ ] Item-size sweep; crossover marked
- [ ] Syscall counts verify kernel paths engaged
- [ ] mmap measured cold *and* warm, sequential *and* random

## Key References

- [`bytes` docs](https://docs.rs/bytes) — the pipeline currency.
- [serde lifetimes](https://serde.rs/lifetimes.html) — `#[serde(borrow)]` semantics.
- `sendfile(2)`/`splice(2)`/`copy_file_range(2)` — the kernel menu.
