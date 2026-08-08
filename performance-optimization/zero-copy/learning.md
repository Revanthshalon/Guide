# Zero-Copy — Learning Notes

## The Hardware Mechanism

A copy looks free in source code (`buf.to_vec()`, one line) and costs three ways at the machine:

- **Bandwidth, twice.** `memcpy` reads every source byte and writes every destination byte — 2× the data's size in memory traffic, drawn from the same bounded budget as all your real work (the un-hideable wall from [cache locality](../cache-locality/learning.md)). Copying at 20 GB/s sounds fast until it's 40 GB/s of traffic on a 60 GB/s machine.
- **Cache pollution, twice.** Source and destination both stream through the cache hierarchy, evicting your working set. A 1 MB copy doesn't just cost its microseconds — it costs the misses your *next* million instructions pay to repopulate what it evicted. (Non-temporal/streaming stores exist to bypass the cache for exactly this reason; compilers use them for large `memcpy`.)
- **An allocation, usually.** The destination has to live somewhere: most copies in real Rust are `to_vec()`/`to_string()`/`clone()` — a copy *and* an [allocation](../allocation-strategies/learning.md) fused into one innocent-looking call.

The second half of the mechanism is *where copies come from* at the OS boundary. Devices already do **DMA**: the NIC and disk controller write directly into kernel memory (page cache, socket buffers) without the CPU touching a byte. So in a classic `read()`+`write()` file-serving loop, the hardware copies are free — what you pay for is the CPU ferrying bytes **kernel→user** (`read`) and **user→kernel** (`write`): two full CPU copies whose only purpose is to route data through your process, which never even looks at it. Kernel primitives exist to delete exactly those: `sendfile`/`splice`/`copy_file_range` (file→socket or file→file entirely inside the kernel), `mmap` (map the page cache into your address space — page tables instead of copies), io_uring registered buffers (DMA into buffers you own). "Zero-copy" names both disciplines: **don't duplicate in-process** (borrow, don't own) and **don't ferry across the kernel boundary** (let the kernel or the device move it).

Rust's relevance: in C, referencing instead of copying is how you get use-after-free; the borrow checker makes the reference the *safe default* and the copy the explicit act. Zero-copy is the performance dividend of the ownership system — lifetimes are the price tag, paid at API-design time.

## Mental Model

**Trace the data's path from source to sink and count the times any byte is duplicated. Each count is a candidate deletion; delete by referencing in place.** The model in four rules:

1. **Parse to views, not values.** A parser that returns `String`s owns a copy of everything it touched; one that returns `&str` slices into the input buffer returns *coordinates* — zero bytes moved ([the allocation doc's](../allocation-strategies/learning.md) lever 1, seen from the data's side). Same for binary: `&[u8]` spans, `serde` with `#[serde(borrow)]`, `nom`-style parsers. The general form: **a view = pointer + length into memory someone else keeps alive** — the fat pointer from [memory layout](../memory-layout/learning.md), doing economic work.
2. **Share by refcount when lifetimes can't be scoped.** Borrowed views die with their scope; data crossing task/thread/queue boundaries needs an owner. `bytes::Bytes` (and `Arc<[u8]>`/`Arc<str>`) is the idiom: a refcounted handle where **slicing is free** — `bytes.slice(8..24)` is a new handle into the same allocation, no copy, safe to send anywhere. This is the network-stack currency (tokio ecosystem): one receive buffer, many zero-copy views flowing through the pipeline.
3. **Assemble with gather, not concatenation.** Building a message by `extend_from_slice`-ing parts into one `Vec` copies every part once before the kernel copies it again. `write_vectored`/`IoSlice` (scatter-gather I/O) hands the kernel a *list* of slices — header here, body there — and it assembles on the way out. One copy deleted, and often the allocation with it.
4. **Cross the kernel boundary with intent.** File→socket: `sendfile`/`splice` (in Rust, `std::io::copy` already specializes to `copy_file_range`/`sendfile` on supported platforms for `File`→`File`/socket pairs — check your platform, or use `nix`/`libc` directly). Large read-mostly files: `mmap` (`memmap2`) — the page cache *is* your buffer, no user-space duplicate, shared across processes; with real caveats (below). High-rate I/O: [io_uring](../async-and-io/learning.md) registered buffers amortize both the syscall *and* the mapping cost — the [batching](../batching-and-amortization/learning.md) and zero-copy levers pulled together.

**When a copy is the *right* answer** (the model's boundary, worth stating early): copying ≲ a few hundred bytes costs nanoseconds — less than the refcount atomics or lifetime plumbing that avoiding it costs (the F-vs-m thinking from [batching](../batching-and-amortization/learning.md), applied to bookkeeping); copying *releases* a large buffer that a tiny view would otherwise pin (the amplification trap below); and copying decouples lifetimes at an API boundary where a borrow would infect every caller's signature. Zero-copy is a *budget* discipline, not a purity contest: spend copies where they buy simplicity, delete them where the profile says they're the traffic.

## Worked Example

An in-process pipeline — parse a 512 MB network capture of length-prefixed records, route each record's payload to a per-topic queue. Three versions; illustrative numbers, `dhat` counts exact in shape.

**v0 — own everything: the copy tax compounded.**

```rust
fn parse(buf: &[u8]) -> Vec<Record> {          // Record { topic: String, payload: Vec<u8> }
    /* per record: topic.to_string() + payload.to_vec() */
}
```

Every record copies its bytes out of the receive buffer (bandwidth ×2), allocates twice (topic + payload), and the queues then own scattered small heap blocks ([locality](../cache-locality/learning.md) loss for every downstream consumer). `dhat`: ~2 allocations/record; throughput: **~0.9 GB/s**, flat profile dominated by `memcpy` + `malloc`.

**v1 — borrow: views into the buffer.**

```rust
struct RecordView<'a> { topic: &'a str, payload: &'a [u8] }
fn parse(buf: &[u8]) -> impl Iterator<Item = RecordView<'_>>   // zero copies, zero allocs
```

`dhat`: ~0 allocations/record; throughput: **~4.8 GB/s** — the parse is now bounded by scanning, not duplicating. The price appears at the boundary: `RecordView<'a>` cannot outlive `buf` — fine inside one function's scope, but *it cannot be sent to the per-topic queues* (the borrow ends where the async boundary begins). v1 is the right shape for streaming aggregation; for routing, you need v2.

**v2 — `Bytes`: refcounted views that travel.**

```rust
let buf: Bytes = read_chunk();                          // one allocation per chunk
struct RecordMsg { topic: Bytes, payload: Bytes }        // slices of buf — no copy
queue[t].send(RecordMsg { topic: buf.slice(a..b), payload: buf.slice(c..d) });
```

Slicing bumps a refcount; payloads flow to queues with zero copies and zero per-record allocations; the chunk buffer is freed when the *last* view drops. Throughput: **~4.2 GB/s** end-to-end (refcount traffic costs a little over v1). The new liability is **amplification**: one 40-byte payload retained by a slow consumer pins its entire 4 MB chunk — RSS balloons while `dhat` sees nothing wrong (the memory is *reachable*). The standard fix: consumers that retain long-term do a *deliberate* copy (`Bytes::copy_from_slice`) — a copy *spent* to release megabytes, the budget discipline in action.

The three-row summary — copies per byte: v0 = 2 (+2 allocs/record), v1 = 0 (borrow-scoped), v2 = 0 (refcount-scoped, amplification-prone). Choosing among them is choosing a *lifetime regime*, which is the actual skill this topic teaches.

## Applying It

- **Parse borrowed by default:** `&str`/`&[u8]` outputs, `#[serde(borrow)]` for serde (works for `&str`/`&[u8]`/`Cow` fields from format buffers), `nom`/`winnow` parsers are borrow-native. Materialize (`to_owned`) only at the point where data must outlive the buffer — and make that point visible in the code, not incidental.
- **`Cow<'a, T>` for maybe-owned:** the escape valve when *most* items borrow but some need normalization (unescaping, case-folding) — `Cow::Borrowed` on the fast path, `Cow::Owned` only where transformation forced a copy anyway.
- **`bytes::Bytes`/`BytesMut` as pipeline currency:** receive into `BytesMut`, `split_to()`/`freeze()` into `Bytes`, route slices; the tokio/hyper/tonic ecosystem speaks it natively. `Arc<[u8]>` is the std-only equivalent (no cheap slicing — pair with `(Arc<[u8]>, Range<usize>)` if needed).
- **Gather-write:** `write_vectored(&[IoSlice::new(header), IoSlice::new(body)])` instead of assembling; check `is_write_vectored()` on the transport (not all impls really scatter). Encoders should *write into* a caller's buffer (`fn encode(&self, dst: &mut BytesMut)`) rather than return fresh `Vec`s — the API-signature rule from [batching](../batching-and-amortization/learning.md)/[DoD](../data-oriented-design/learning.md) again.
- **Kernel-side moves:** `std::io::copy` (specializes to `copy_file_range`/`sendfile`/`splice` where possible — verify with `strace`/`dtruss`); `memmap2` for large read-mostly files with the caveats below; [io_uring](../async-and-io/learning.md) with registered buffers where syscall rate is the profile.
- **Serialization formats that never materialize:** `rkyv`/FlatBuffers/Cap'n Proto access fields *in the wire bytes* — the zero-copy idea applied to the whole encode/decode layer; the trade-offs live in the [serialization doc](../serialization-and-encoding/learning.md).
- **Verify the deletion:** `dhat` allocation counts per item → 0; bandwidth math (GB processed vs GB of memory traffic — `perf stat` memory counters where available); `strace -c` for the kernel-boundary variants (a `sendfile` path shows *no* read/write pairs).

## When It Hurts

- **The amplification trap (the classic).** A tiny `Bytes` slice pins its whole backing chunk; ten thousand 40-byte retained slices of 4 MB chunks = 40 GB of unreclaimable RSS holding 400 KB of data. Rule: **views are for transit, copies are for retention** — anything stored long-term (caches, session state, dedup tables) gets a deliberate compact copy. Monitor RSS-vs-live-bytes divergence; it's this trap's signature.
- **Lifetime infection.** A borrowed parse output (`RecordView<'a>`) threads `'a` through every API it touches and pins the input buffer against reuse — the [arena-lifetime problem](../allocation-strategies/learning.md) again, and the reason self-referential "parse it and keep both" structs don't work in safe Rust. Escape hatches, in order: restructure scopes (process within the borrow), `Bytes` (refcount instead of borrow), or accept the copy at the boundary.
- **`mmap`'s sharp edges:** page faults are *latency spikes on first touch* (the "read" happens at access time, in the middle of your hot loop, invisible to strace); **truncation is UB-adjacent** — another process shrinking the file turns your mapped reads into `SIGBUS` (this is why `memmap2` is `unsafe` and why mapped files want advisory locks or immutable-by-convention storage); dirty-page writeback timing is the kernel's choice, not yours. mmap earns its keep for large, read-mostly, long-lived mappings — not as a general `read()` replacement.
- **Small data: the bookkeeping costs more than the copy.** A 64-byte copy is ~2 ns; a `Bytes` clone is an atomic RMW (~10–20 ns, and [contended refcounts ping-pong lines](../false-sharing/learning.md)); a lifetime parameter is minutes of engineering forever. Below ~a few hundred bytes, copy and move on — measured, per the funnel, but the prior is the copy.
- **Shared mutable temptation:** zero-copy sharing is *read* sharing; the moment two owners want to mutate, you're either copying anyway (`BytesMut::split` semantics, `Cow::to_mut`) or building the [false-sharing](../false-sharing/learning.md)/locking problem. Rust makes this explicit rather than impossible — listen when `make_mut` shows up in a profile.

## Benchmarking Methodology

- **Count, then time:** copies-per-byte and allocations-per-item are the mechanism metrics (`dhat` for allocs; reason the copy count from the code path, confirm with bandwidth math: bytes processed × copies ≈ memory traffic). Deterministic counts gate CI; time confirms.
- **Watch RSS beside throughput** in any `Bytes`/mmap benchmark — amplification and page-cache effects only show there (`/usr/bin/time -l` on macOS).
- **Sweep item size:** zero-copy's win grows with bytes moved; find the crossover under which the owned version is *faster* (it exists, usually in the low hundreds of bytes) and encode it as the API's guidance.
- **Kernel-path variants need syscall counts** (`strace -c`/`dtruss`): verify `sendfile`/specialized `io::copy` actually engaged — like autovectorization, kernel specialization is silently reversible (a `BufReader` wrapper, for instance, defeats it).
- **mmap benchmarks must include first-touch:** a warmed mapping benchmarks the page cache, not mmap; measure cold (after `purge`/drop-caches) and warm as separate results, and at random vs. sequential access — the answers differ by orders of magnitude.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Enumerate the three costs of an in-process copy, and explain why a 1 MB memcpy can cost more than its own duration.
2. In the classic `read()`+`write()` file-serving loop, which two copies does the CPU perform, why is neither necessary, and which primitive deletes them?
3. v1 borrows, v2 refcounts: state the lifetime regime of each, and the exact boundary (in the worked example) where v1 stops being usable.
4. Construct the amplification trap with numbers, name its observable signature, and state the transit-vs-retention rule that prevents it.
5. Why is `memmap2::Mmap` construction `unsafe`? What external event breaks it, and what deployment conventions make it sound?
6. Below what size is copying the right call, and which three costs of *not* copying dominate there?
7. Your `io::copy` file→socket path shows read/write pairs in `dtruss`. What happened, and what's the likely wrapper that caused it?

Measurement exercises:

- Build the three-version worked example on a synthetic capture (length-prefixed records, ~1 KB average): measure throughput + dhat counts for v0/v1/v2, then add a slow consumer retaining 1% of payloads and watch v2's RSS-vs-live-bytes diverge. Fix with retention-copies; re-measure. The full trap and cure, on your machine.
- Race `std::io::copy` (File→TcpStream) against a manual read/write loop; verify with `dtruss` whether specialization engaged on macOS, and measure both. Then wrap the file in `BufReader` and watch the specialization (and the win) vanish.
- Find the copy-vs-view crossover: parse-and-sum over records of 16/64/256/1 K/4 KB comparing owned vs borrowed output — plot and mark where they cross on your machine.

## Open Questions

- macOS specifics: which `io::copy` specializations exist on Darwin (no `splice`; `sendfile` yes) — verify current std behavior with `dtruss` and record it.
- io_uring registered buffers + `IORING_OP_SEND_ZC` (true zero-copy send): measured win over plain io_uring on a real NIC, and what does the completion-semantics change (buffer reuse only after ack) cost in code shape? ([async doc](../async-and-io/learning.md) follow-up.)
- `Bytes` internals: current representation (inline vs Arc variants), the cost of `slice()` vs `Arc<[u8]>`+range — microbenchmark both as pipeline currency.
- Practical guardrails for the amplification trap at scale: does a "max retained fraction per chunk before compacting" heuristic exist in production codebases (hyper/tonic internals worth reading)?
- `rkyv` access-in-place vs serde-borrow deserialization on this repo's future workloads — defer numbers to the [serialization doc](../serialization-and-encoding/learning.md), but sketch the benchmark now.

## References

- [`bytes` crate docs](https://docs.rs/bytes) — the `Bytes`/`BytesMut` model; read "Sharing" and the `split_to`/`freeze` lifecycle.
- [`memmap2` docs](https://docs.rs/memmap2) — including the safety discussion that doubles as the mmap-caveats list.
- Linux `sendfile(2)`, `splice(2)`, `copy_file_range(2)` man pages — the kernel-side menu; skim once so the primitives exist in your head.
- serde's [deserializer lifetimes documentation](https://serde.rs/lifetimes.html) — `#[serde(borrow)]` semantics and when borrowing from the input actually works.
- Related topics in this repo: [Allocation Strategies](../allocation-strategies/learning.md) (borrow-don't-own is lever 1; copies carry allocations), [Cache Locality](../cache-locality/learning.md) (bandwidth and pollution costs), [Batching & Amortization](../batching-and-amortization/learning.md) (F-vs-m applied to bookkeeping; io_uring), [Serialization & Encoding](../serialization-and-encoding/learning.md) (zero-copy formats), [Async & I/O](../async-and-io/learning.md) (registered buffers, zero-copy send), [False Sharing](../false-sharing/learning.md) (what contended refcounts do to lines).
