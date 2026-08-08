# Batching & Amortization — Learning Notes

## The Hardware Mechanism

Unlike the previous topics, the mechanism here isn't one hardware structure — it's a *cost shape* that recurs at every layer of the stack: **almost every operation is a fixed cost plus a marginal cost**, and the fixed cost is usually the bigger number. A ladder of fixed costs, small to large:

| Boundary | Fixed cost per crossing | What it buys |
| --- | --- | --- |
| Function call | ~1–5 ns | Frame setup, register spills (inlining amortizes it — [compiler's job](../compiler-optimizations/learning.md)) |
| Atomic RMW / uncontended lock | ~10–20 ns | Coherence traffic ([false sharing](../false-sharing/learning.md)'s currency) |
| Allocator call | ~10–30 ns fast path | [The allocation doc](../allocation-strategies/learning.md)'s whole subject |
| **Syscall** | **~100–500 ns+** (post-Spectre mitigations) | Mode switch, register save, entry/exit — *before any work* |
| Context switch | ~1–10 µs | Plus the invisible tax: caches and TLB repopulate afterward |
| Disk fsync | ~50 µs (NVMe) – ms (HDD) | Durability per call, however few bytes |
| **Network round trip** | **~50 µs (same-DC) – 1–100 ms (WAN)** | Per *request*, however small |

Two readings of the table. First, **the crossings dominate the work**: a `write()` of 8 bytes and of 8 KB cost nearly the same syscall overhead; a DB query returning one row and forty rows cost the same RTT. Second, **the ladder spans seven orders of magnitude**, so the same technique — carry N items per crossing instead of one — pays wildly different amounts: batching function calls saves nanoseconds (the compiler already does it); batching network round trips saves milliseconds (nothing else you do today will save more). Amortization is the umbrella: spread a fixed cost F over N items so each item pays F/N. It is the *only* technique in this repo that appears at every single layer — SIMD is batched instructions, arenas are batched frees, the [outbox relay's](../../architecture-patterns/outbox-pattern/learning.md) batched publishes and the database's group commit are batched fsyncs. One idea, seven altitudes.

## Mental Model

**Singly: N × (F + m). Batched: F + N × m. The win is the ratio, and it saturates.** With per-item marginal cost m and fixed cost F, batching N items cuts the fixed contribution from F to F/N per item. The curve this generates is the topic's central object:

- Throughput vs. batch size rises steeply while F/N dominates, then **flattens at the knee** — roughly where F/N drops below m (N ≈ F/m). Past the knee, bigger batches buy ~nothing and start costing (memory held, latency added, cache overflowed). **N ≈ F/m is the sizing formula**: syscall F ≈ 1 µs over m ≈ 10 ns of per-byte work → knee at ~100 items; network F ≈ 1 ms over m ≈ 1 µs → knee at ~1000. Estimate it, then find it empirically.
- **The price is latency.** A batch waits to fill: the first item in a batch of N ages while N−1 arrive. Batching converts a *latency* resource into a *throughput* resource — which is why every production batcher has the **size-or-time shape**: flush at N items *or* T milliseconds, whichever first. N caps memory and sets the amortization; T bounds the worst-case wait (and does all the work under low load, where batches never fill). Choosing (N, T) *is* choosing your position on the throughput/latency frontier — [the tail-at-scale trade](../profiling-and-measurement/learning.md), made explicit.
- **Amortization also lives inside data structures.** `Vec`'s doubling growth is amortized O(1) *because* it batches: one reallocation serves the next N/2 pushes. The [allocation doc](../allocation-strategies/learning.md) showed the constants hiding in "amortized"; this doc names the design move — pay a big F rarely instead of a small F always — so you recognize it in `HashMap` rehashing, log-structured storage compaction, and every "generation" or "epoch" scheme you'll ever meet.
- **The dual move: hoist, don't just batch.** Batching amortizes a fixed cost you keep paying; *hoisting* pays it once and never again — the prepared statement (parse once, execute N times), the compiled regex outside the loop (the [profiling doc's](../profiling-and-measurement/learning.md) worked example), the connection pool (handshake once). Ask "can this F be paid once?" before "how do I spread this F?" — hoisting beats amortizing when the fixed work is *identical* across items rather than merely *similar*.

Where the model stops: when F is already negligible relative to m (batching adds machinery for nothing), and when items are *independent decisions* whose coupling creates new failure semantics — a batch is a shared fate, and the When-It-Hurts section is mostly about that.

## Worked Example

The same arithmetic at two rungs of the ladder — ~1 µs and ~1 ms — to make the altitude point concrete.

**Rung 1: syscalls.** Write 10M small records (~40 bytes) to a file.

```rust
// A. One write() per record: 10M syscalls
for r in &records { file.write_all(&encode(r))?; }               // ~11.2 s

// B. BufWriter (8 KB buffer): ~200 records per syscall, ~49 K syscalls
let mut w = BufWriter::new(file);
for r in &records { w.write_all(&encode(r))?; }
w.flush()?;                                                       // ~0.44 s   ~25×

// C. Bigger buffer (1 MB): ~26K records per syscall, ~380 syscalls
let mut w = BufWriter::with_capacity(1 << 20, file);              // ~0.36 s   ← the knee: past it, nothing
```

Verify the mechanism, not just the time: `strace -c` (Linux) / `dtruss` (macOS) shows the syscall count collapsing 10M → 49 K → 380. B-vs-C is the knee made visible: a 128× bigger buffer bought 1.2×, because past N ≈ F/m the marginal work (encoding, memcpy) dominates. And the classic bug rides along: forget `flush()` and the tail of the data silently never lands — batching means *data in flight at crash time*, the durability question every batcher must answer (sound familiar? It's the [outbox](../../architecture-patterns/outbox-pattern/learning.md) crash-window, at file scale).

**Rung 2: network round trips — the N+1 query.** Render 1 000 users with their team names:

```
A. 1 query for users + 1 query per user for team:  1001 × ~1 ms RTT  ≈ 1.05 s
B. 1 query + one `WHERE id = ANY($ids)` batch:     2 × ~1 ms         ≈ 3 ms    ~350×
C. One JOIN:                                        1 × ~1 ms         — hoisting the second crossing entirely
```

Same formula, F a thousand times larger, so the win is a thousand times larger — **the highest-leverage batching in ordinary backend work is at the network rung**: N+1 queries, chatty per-item HTTP calls, one-message-per-publish brokers. The ladder tells you where to hunt first.

## Applying It

- **I/O:** `BufReader`/`BufWriter` on every unbuffered stream (and audit the `flush` story); lock stdout once (`let out = stdout().lock()`) instead of per-`println!`; `write_vectored` for scatter/gather; [io_uring](../async-and-io/learning.md) as syscall amortization's industrial form (submit/complete many per crossing).
- **Database:** multi-row `INSERT`/`COPY`, `= ANY($1)` batch lookups, prepared statements (hoisting), pipeline mode (`tokio-postgres` pipelining — multiple queries per RTT); the N+1 detector habit: any query inside a loop is a finding until proven cold.
- **Channels and queues:** consumers drain — `while let Ok(x) = rx.try_recv()` after the blocking recv (or `recv_many` where the runtime offers it) — processing the backlog per wakeup instead of one message per context switch. Producers coalesce: accumulate locally, send a `Vec` (one channel crossing, one allocation, better [locality](../cache-locality/learning.md) for the consumer sweep).
- **Locks and atomics:** batch under one acquisition (drain the queue inside one `lock()`, not lock-per-item — but watch hold time, below); per-thread accumulate + periodic merge for counters/metrics (the [false-sharing](../false-sharing/learning.md) fix and a batching pattern, same move).
- **Parallelism:** `par_chunks(k)` over `par_iter` when per-item work is tiny — amortize the scheduler's F over k items ([work-stealing doc's](../parallelism-and-work-stealing/learning.md) granularity knob, same formula).
- **Durability:** group commit — accumulate transactions, one fsync for many (every serious database does this; your WAL-shaped code should too); the [outbox relay's](../../architecture-patterns/outbox-pattern/learning.md) batched publish is the same pattern one layer up.
- **The production batcher shape:** size-or-time (N, T) with both knobs explicit and monitored; a metrics pair (batch-size distribution, item age at flush) tells you which knob is binding. Under low load T rules (latency floor); under high load N rules (throughput ceiling); alert when the distribution shifts regime.
- **API design:** accept slices (`fn send(&mut self, msgs: &[Msg])`), not items — the [DoD transform-API rule](../data-oriented-design/learning.md) again: an interface that only takes one item at a time *forbids* callers from amortizing your fixed costs. Batch-capable APIs are a design decision made at the signature.

## When It Hurts

- **Latency is the invoice.** Every batch adds up-to-T wait; under low load, time-based flushes set your p50. A batcher tuned for peak throughput and never re-examined is the classic source of "why is the idle system slow?" Size T from the latency budget, not from throughput dreams.
- **A batch is a shared fate.** One malformed item fails the batch: now you need partial-failure semantics — split-and-retry (bisect to isolate the bad item), per-item results, or all-or-nothing with [idempotent](../../architecture-patterns/idempotency-and-delivery-semantics/learning.md) retry. Deciding this *after* the incident is the standard failure; the batch API must define it on day one. (Batch writes to a DB inherit transaction semantics — often a feature; batch HTTP calls inherit nothing — always a design task.)
- **In-flight data at crash time.** Buffered ≠ durable: the BufWriter tail, the un-flushed group commit, the coalesced messages in the producer's `Vec` — all vanish on crash. Either the loss is acceptable (metrics), or the batcher needs the [outbox discipline](../../architecture-patterns/outbox-pattern/learning.md) (persist intent first), or flush points are transaction boundaries. Name which, per batcher.
- **Too-big batches backfire mechanically:** batch working set falls out of cache (the [staircase](../cache-locality/learning.md) — process-then-flush in cache-sized chunks instead), lock hold time grows until batching *creates* the contention it saved ([lock-free doc's](../lock-free-concurrency/learning.md) concern), allocation spikes for batch buffers, and timeout amplification (a 30 s batch inside a 10 s caller timeout fails always — and retries as a whole).
- **Unbounded batching is buffer bloat.** A queue that grows while the flusher falls behind converts overload into memory exhaustion plus seconds of added latency — bound the queue and push back ([backpressure](../../architecture-patterns/backpressure-and-rate-limiting/learning.md) is the architecture-level face of this rule). Little's law is the audit: items-in-flight = arrival rate × time-in-batcher; if that number can grow without bound, so can your latency.
- **Head-of-line blocking:** one slow item delays its batch-mates (a giant row in the multi-insert, one slow key in the batched fetch). Where item cost varies wildly, batch by *cost* (bytes, estimated work) rather than count, or segregate classes into separate batchers.

## Benchmarking Methodology

- **Sweep batch size, plot two curves:** throughput *and* p99 item latency vs. N (log-x). The throughput curve finds the knee; the latency curve shows what the knee costs; the decision is where *your* SLO sits between them. One curve alone is half an answer.
- **Verify the crossing count**, not just the time: `strace -c`/`dtruss` for syscalls, DB query logs for round trips, channel-wakeup counters — the mechanism is "fewer crossings," so count crossings (they're deterministic, like the [allocation doc's](../allocation-strategies/learning.md) counts — good CI gates).
- **Measure both load regimes:** high load (N-bound: batches full, throughput story) and trickle load (T-bound: every item waits the timer — the regime where naive batchers embarrass themselves). The idle-system p50 is a first-class result.
- **Include the failure path:** benchmark split-and-retry under an injected bad item; a batcher whose failure mode has never been exercised has an unexercised outage in it.
- **Item age at flush** is the observability metric to build in from day one: its distribution tells you which knob (N or T) is binding, live, in production.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Derive the knee formula N ≈ F/m, and compute it for: syscall (F = 1 µs, m = 10 ns), fsync group commit (F = 100 µs, m = 5 µs), WAN API call (F = 50 ms, m = 100 µs).
2. Why does every production batcher have both an N and a T? Which regime does each rule, and what breaks if T is missing?
3. The BufWriter example: why did 8 KB → 1 MB buy only 1.2×? What single number predicts this?
4. Name the amortization inside `Vec::push`, and the design move it exemplifies. Where else in this repo does that move appear?
5. Batching vs. hoisting: define both, give the DB-flavored example of each, and state the rule for which to try first.
6. A batch insert of 500 rows fails on row 217. Enumerate the three failure-semantics designs and what each demands of the caller.
7. Connect unbounded batching to Little's law: what quantity explodes, and which architecture doc owns the fix?

Measurement exercises:

- Reproduce rung 1: the 10M-record write at buffer sizes 0/4 KB/64 KB/1 MB/16 MB, with syscall counts (`dtruss`/`strace -c`) beside wall time. Plot both; mark the knee; confirm crossing-count collapse explains the time.
- Build a size-or-time channel batcher (tokio: `recv` + `timeout`, flush at N=100 or T=5 ms) and measure item age at p50/p99 under trickle (10/s) and flood (100 K/s). The two regimes' signatures are the lesson.
- Find an N+1 in real code you own (ORM lazy-loading is the usual suspect); fix with a batch fetch; measure before/after and count the round trips.

## Open Questions

- macOS `dtruss`/Instruments syscall-counting workflow vs Linux `strace -c` — establish the local equivalent once.
- io_uring's actual amortization curve: syscalls per completed operation vs. queue depth on a real NVMe — where's *its* knee ([async & I/O](../async-and-io/learning.md) preview)?
- tokio `recv_many` vs. drain-loop: allocation and wakeup behavior compared, at what arrival rates does it matter?
- Adaptive batching (Nagle-style dynamic T, TCP_NODELAY debates): when does auto-tuning beat explicit (N, T), and what pathologies does it import (the Nagle+delayed-ACK interaction as cautionary tale)?
- Group commit in embedded engines (SQLite WAL, sled): what (N, T) equivalents do they expose, and what do their defaults imply for your write-heavy paths?

## References

- Dean & Barroso, ["The Tail at Scale"](https://research.google/pubs/pub40801/) (CACM 2013) — the latency-vs-throughput frontier and why tails dominate at fan-out; the paper behind this doc's latency warnings.
- Brendan Gregg, *Systems Performance* — syscall/context-switch cost anatomy and the measurement tooling (ch. 3–5).
- [io_uring design docs](https://kernel.dk/io_uring.pdf) (Jens Axboe) — syscall amortization as a kernel interface; read once even before the async doc.
- Nagle's algorithm + delayed-ACK history (RFC 896 and the folklore) — the canonical adaptive-batching cautionary tale.
- Related topics in this repo: [Allocation Strategies](../allocation-strategies/learning.md) (arenas = batched frees; `Vec` growth), [SIMD](../simd/learning.md) (batched instructions), [Parallelism](../parallelism-and-work-stealing/learning.md) (chunking = batched scheduling), [Async & I/O](../async-and-io/learning.md) (io_uring = batched syscalls), [Outbox](../../architecture-patterns/outbox-pattern/learning.md) / [Backpressure](../../architecture-patterns/backpressure-and-rate-limiting/learning.md) / [Idempotency](../../architecture-patterns/idempotency-and-delivery-semantics/learning.md) (batching's architecture-level face: relay batches, bounded queues, batch-failure semantics).
