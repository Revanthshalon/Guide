# Async & I/O — Learning Notes

## The Hardware Mechanism

I/O is the regime where the CPU is *not* the actor. The device (NIC, NVMe) does the work via DMA and raises an interrupt when done; the CPU's involvement is asking and being told. The costs that shape everything:

- **The waits are enormous on a CPU's clock:** NVMe read ~20–100 µs, same-DC network RTT ~50–500 µs, WAN ~10–100 ms — against ~0.3 ns cycles, a single network wait is *hundreds of thousands to millions* of instruction-opportunities. The entire topic is about not wasting them.
- **A blocking syscall parks the thread:** the kernel puts it to sleep until the I/O completes — burning a context switch out and back (~1–10 µs × 2) plus [cache/TLB repopulation](../batching-and-amortization/learning.md), and, more importantly, *renting an entire OS thread per in-flight wait*. Thread-per-connection at 10 K connections = 10 K stacks (≈ 8 MB virtual / ~tens of KB touched each), 10 K schedulable entities churning the scheduler — the C10K problem that created this field.
- **Readiness models — epoll (Linux) / kqueue (macOS):** one syscall ("which of these 10 K fds are ready?") returns many answers — *the [batching](../batching-and-amortization/learning.md) move applied to waiting*. The kernel tells you a socket is readable; you still make the `read` syscall yourself. One thread can now supervise tens of thousands of connections; this is the engine under tokio's reactor.
- **Completion models — io_uring (Linux):** two shared-memory rings (submission, completion); you post *operations* ("read this fd into this buffer"), the kernel executes and posts results — many ops per syscall, optionally *zero* syscalls (kernel-side polling), and it works for **files** (which epoll never handled — regular-file "readiness" is meaningless, the historical reason async file I/O was a lie on Linux) plus registered buffers ([zero-copy's](../zero-copy/learning.md) kernel-boundary lever). macOS has no equivalent — kqueue readiness is the ceiling here, which matters for what tokio can do per-platform (below).

## Mental Model

**Async is concurrency for *waiting* — the [parallelism doc's](../parallelism-and-work-stealing/learning.md) missing half. Parallelism computes more at once; async *waits* more at once, on fewer threads. It does not make any individual I/O faster; it makes the waits overlap.**

1. **Rust's async is a compiler transformation, not a runtime service.** An `async fn` compiles to a **state machine enum**: one variant per await point, holding exactly the locals that live across that await ([memory layout](../memory-layout/learning.md) applies — the future's size is the *largest* variant, and a 16 KB buffer held across an await is a 16 KB future; `Box::pin` the outliers). The future is **inert** — it runs only when *polled*. No callbacks, no green-thread stacks, no per-await allocation: a spawned task is one heap allocation totaling ~hundreds of bytes, versus a thread's stack — the ~1000× density that makes 100 K concurrent tasks unremarkable.
2. **The runtime is the missing piece:** an executor polls tasks; a **reactor** sits on epoll/kqueue; the **Waker** ties them together — a poll that would block registers the waker and returns `Pending` (the thread moves on to other tasks); the reactor's readiness event calls `wake()`, re-queueing the task. Tokio's multi-thread runtime schedules tasks across workers with **work-stealing deques** — [the same Chase-Lev machinery](../parallelism-and-work-stealing/learning.md), scheduling waits instead of compute. The elegance to internalize: *a task switch is a function return* (~ns, user-space) versus a thread switch (~µs, kernel) — that three-orders gap is async's entire performance content.
3. **Cooperative scheduling is the contract, and its violation is the cardinal sin.** Tasks yield *only* at `.await`. Anything that occupies the thread between awaits — a sync DB call, zlib compression, a 50 ms parse, even a surprisingly-long loop — freezes *every other task* on that worker. The runtime cannot preempt; it can only wait. **Blocking the runtime is the production incident this doc exists to prevent**: the escape hatches are `spawn_blocking` (ships the call to an unbounded-ish blocking pool — for *waiting*-shaped sync work: sync clients, file APIs pre-uring) and a [rayon bridge](../parallelism-and-work-stealing/learning.md) via oneshot channel (for *compute*-shaped work). File I/O deserves special honesty: on tokio, `tokio::fs` *is* `spawn_blocking` underneath (epoll can't do files) — async files are a convenience API over a thread pool, not an event-driven path, until io_uring runtimes change the story on Linux.
4. **Cancellation is a feature with teeth.** Any future can be *dropped* at any `.await` — that's how `select!`, `timeout`, and task aborts work. Dropped mid-operation, a future simply never resumes: a half-completed multi-read loses the bytes read so far unless the operation was designed **cancel-safe** (state lives in the buffer/connection, not in the future's locals). This is Rust-async's sharpest correctness edge — [the idempotency doc's](../../architecture-patterns/idempotency-and-delivery-semantics/learning.md) "what happens on retry?" question, reborn as "what happens on drop?" — and it must be asked of every arm of every `select!`.
5. **When threads win anyway:** at ~hundreds of connections or fewer, thread-per-connection is *simpler, debuggable with ordinary tools, immune to the blocking sin, and fast enough* — async's density dividend pays at thousands of concurrent waits, fan-out patterns (100 backend calls per request), or protocol servers. Choosing async for 12 connections buys the complexity and none of the rent. (And inside the handler, [the sync mental models survive](../batching-and-amortization/learning.md): buffered I/O, batching, backpressure — async changes who waits, not what's worth doing.)

## Worked Example

An echo-ish TCP service, three chapters.

**1. Density: threads vs tasks at 10 K connections.** Thread-per-connection: 10 K threads ≈ 10 K stacks (tens-of-KB touched each → GBs virtual, ~1 GB resident is typical), and the scheduler visibly degrades — p99 climbs from scheduler queueing alone. Tokio: 10 K tasks on 8 workers ≈ *tens of MB* total task state; connection count stops being a resource question until file-descriptor limits. (Idle connections are where async embarrasses threads most — 10 K parked tasks cost memory only; 10 K parked threads still cost the scheduler.)

**2. The blocking incident, reproduced deliberately.** Add one innocent line to the handler — a sync `zlib` compress of ~50 ms on 1% of requests:

```
8 workers; 1% of requests block a worker for 50 ms
→ at 800 req/s: ~8 blocked-worker-events/s — statistically, some worker is
  frozen much of the time; every task queued on it stalls
p50: 0.4 ms → 0.6 ms        (barely visible — the median lies)
p99: 2 ms   → 260 ms        (tasks trapped behind frozen workers)
```

The signature: **healthy p50, catastrophic p99, CPU far from saturated** — the runtime is starved, not the machine. Diagnosis: `tokio-console` shows tasks with multi-ms *poll times* (a poll should be µs); the busy worker is visible directly. Fix: `spawn_blocking` for the compress (or rayon if it's truly compute at volume) — p99 returns to ~2 ms. The lesson generalizes: **in async, latency bugs are usually *scheduling* bugs**, and the poll-time histogram is the flamegraph of this world.

**3. The cancellation bug, then the fix.** A read-with-timeout, written the obvious way:

```rust
// BUG: if the timeout fires mid-message, the partial read is DROPPED —
// bytes already consumed from the socket are lost; next read starts mid-frame
select! {
    msg = read_frame(&mut conn) => handle(msg?),
    _ = sleep(Duration::from_secs(5)) => return Err(Timeout),
}
```

`read_frame`'s future held the partial buffer in *its own state*; drop = data loss, and the connection is now desynchronized — a corruption bug that manifests as "rare protocol errors under load." Cancel-safe fix: the accumulation state moves *out* of the future into the connection (`conn.read_buf` persists; `read_frame` resumes from it on the next call — this is precisely how `tokio_util::codec::Framed` is built). Rule: **state that must survive cancellation lives outside the future.** Every `select!` arm gets audited with one question: "this future is dropped here — what's lost?"

## Applying It

- **Runtime hygiene:** multi-thread runtime by default; `#[tokio::main]`; never size your *own* thread pools plus tokio's plus rayon's all at `num_cpus` ([the oversubscription audit](../parallelism-and-work-stealing/learning.md)). CPU work → rayon bridge; sync-waiting work → `spawn_blocking` (and know its pool is effectively unbounded — a thundering herd of blocking calls becomes a thread explosion; bound it with a `Semaphore`).
- **Every external await gets a timeout** (`tokio::time::timeout`) — an awaited future with no deadline is an unbounded liability wired to someone else's reliability; this is [the stuck-saga lesson](../../architecture-patterns/saga-pattern/learning.md) at microscale. Layer [circuit breakers](../../architecture-patterns/circuit-breaker/learning.md) above for repeated failure.
- **Bounded channels only** (`mpsc::channel(n)`, never `unbounded_channel` on data paths): the bound *is* the [backpressure](../../architecture-patterns/backpressure-and-rate-limiting/learning.md) — unbounded queues convert overload into memory exhaustion plus latency ([buffer bloat](../batching-and-amortization/learning.md), again). `Semaphore` for concurrency ceilings (in-flight request caps, connection limits); `JoinSet` for supervised fan-out.
- **Locks across awaits:** holding a `std::sync::Mutex` guard across an `.await` is a deadlock generator (the task migrates/parks holding a thread-blocking lock — and it doesn't compile under `Send` bounds for good reason). Short critical sections around awaits: std/parking_lot mutex, drop before awaiting. Must hold across: `tokio::sync::Mutex` (async-aware, more expensive) — or restructure to message-passing/actor shape (a task *owning* the state, channels in — [Alice Ryhl's actor pattern](https://ryhl.io/blog/actors-with-tokio/); usually the best answer).
- **Cancel-safety discipline:** know the cancel-safe set (`recv` on tokio channels: yes; `read_buf` into an external buffer: yes; your hand-rolled multi-step loop: almost certainly not); state-outside-the-future as the design rule; `CancellationToken` for orderly shutdown rather than abort-and-pray.
- **I/O craft still applies:** `BufReader`/`BufWriter` around streams ([syscall batching](../batching-and-amortization/learning.md) — async didn't repeal it), `write_vectored`, [`Bytes` as pipeline currency](../zero-copy/learning.md), read timeouts distinct from connect timeouts.
- **Observability:** `tracing` spans per request (the async-aware replacement for thread-local context), `tokio-console` in staging (poll times, task counts, waker churn), and runtime metrics (worker busy ratio, blocking-pool depth) exported — the [funnel](../profiling-and-measurement/learning.md) needs async-shaped instruments, since thread-based profilers show every worker as "busy polling."
- **io_uring, honestly:** on Linux, `tokio-uring`/`monoio`/`glommio` (thread-per-core designs) deliver real wins for storage-heavy and ultra-high-connection services; the ecosystem is younger than tokio's and the APIs infect ownership models (buffers lent to the kernel). On macOS it's academic — develop against tokio, benchmark uring claims on the Linux target ([the batching doc's io_uring question](../batching-and-amortization/learning.md), one layer up).

## When It Hurts

- **The blocking sin, in all its disguises:** sync database clients (`diesel` without the async adapter, `reqwest::blocking`), DNS via `getaddrinfo` (sneaks into sync resolvers), `std::fs` in handlers, long CPU loops (serde on a 100 MB payload!), `println!` to a slow terminal. Worker count is small (≈ cores); a handful of concurrent blockers freezes the fleet. Defense in depth: clippy lints, `tokio-console` poll-time alerts, and the *cultural* rule that any dependency's sync-ness is a review question.
- **Async infection and complexity rent:** `async` propagates up the call graph, trait ergonomics are worse (though native async-in-traits landed), stack traces are worse, and every function's signature carries the runtime's shadow. For low-concurrency tools, threads + blocking I/O is the *better* engineering. Async is a scaling instrument, not a style.
- **Cancellation as data-loss generator:** every `select!`/`timeout` is a potential mid-operation drop (worked example 3). Symptoms are protocol desync and rare lost writes "under load" — i.e., when timeouts actually fire. The audit question is mechanical; asking it is the discipline.
- **Task leaks and orphans:** `spawn` returns a handle that's easy to drop — the task runs on, unsupervised, holding sockets and buffers ([the zombie-actor cousin](../../architecture-patterns/event-sourcing/learning.md)). `JoinSet`/`TaskTracker` + `CancellationToken` make ownership of tasks as explicit as ownership of memory.
- **Future bloat:** deep `async fn` chains with big locals build multi-KB state machines, copied on every move; `Box::pin` at the spawn boundary, keep buffers external ([layout](../memory-layout/learning.md) discipline in a new costume).
- **Fairness cliffs:** a task that loops over ready I/O without yielding (`while let Ok(n) = stream.try_read(...)`) starves its worker's queue — tokio's budget system mitigates but doesn't absolve; `yield_now().await` in manual ready-loops.

## Benchmarking Methodology

- **Latency distributions under *sustained* load, never averages:** p50/p99/p999 at fixed arrival rates, and beware **coordinated omission** — a load generator that waits for responses before sending the next request silently pauses *with* your stalls and hides exactly the tail you're hunting (use open-loop/constant-rate generators: `wrk2`-style, `goose`/`oha` with rate targets).
- **Scale the concurrency axis, not just the rate axis:** 100 / 1 K / 10 K / 50 K open connections at fixed per-connection rates — the density claim is *the* async claim; verify it against a thread-per-connection baseline at the low end (where threads may win) and watch memory + p99 diverge at the high end.
- **Poll-time histograms are the async flamegraph:** `tokio-console` (or runtime metrics) for max/mean poll durations, blocking-pool queue depth, and per-worker busy ratios. A CPU profile of an async runtime without task attribution is mush; `tracing` spans give the per-request story.
- **Inject the pathologies deliberately in staging:** a 50 ms sync sleep at 1% (worked example 2's numbers should reproduce), a timeout storm (verify cancel-safety under mass cancellation), slow-consumer backpressure (bounded channels filling — does the system degrade or explode?).
- **Benchmark shutdown:** graceful drain under load (CancellationToken → tasks finish, connections close, nothing lost) is a *measurable* property and the first thing incident recovery relies on.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Readiness vs completion models: what does each syscall pattern look like for 10 K sockets, why can't epoll handle regular files, and what does io_uring change structurally?
2. Walk the Waker cycle: poll → Pending → readiness → wake → re-poll. Where does the ~ns-vs-µs task/thread switch gap come from?
3. Do the blocking math: 8 workers, 500 req/s, 2% of requests make a 100 ms sync call. What fraction of workers is frozen on average, and what happens to p99? Name both escape hatches and when each applies.
4. Construct the cancellation data-loss bug from a `select!` with a hand-rolled length-prefixed read; state the rule that fixes it and identify where `Framed` keeps its state.
5. Why is a future's size the largest live-across-await set? Predict which of two given `async fn`s compiles to the bigger state machine, and name the two mitigations.
6. When is thread-per-connection the *better* design? Give the three concrete advantages threads keep.
7. What is coordinated omission, and why does it specifically flatter async services with blocking bugs?

Measurement exercises:

- Reproduce the blocking incident: echo server + 1%-of-requests 50 ms sync sleep; measure p50/p99 at fixed rate (open-loop generator), watch tokio-console's poll times find the culprit, fix with `spawn_blocking`, re-measure. The before/after percentile table is the artifact.
- Build the cancel-safety bug (select! + timeout around a hand-rolled framed read over TCP with injected latency); demonstrate frame desync under timeout storms; fix by moving the buffer into the connection; re-run the storm and verify zero loss.
- Density curve: connections ∈ {100, 1 K, 10 K} × {thread-per-conn, tokio} — RSS and p99 per cell; find where the curves cross on your machine (kqueue + macOS thread limits will teach their own lesson).

## Open Questions

- macOS specifics: kqueue behavior under tokio at high connection counts, thread limits (`kern.num_taskthreads`), and how the density experiment differs from Linux — document local numbers.
- io_uring runtimes on the Linux deploy target: tokio-uring vs monoio vs glommio maturity check; what does the buffer-ownership API (`read` *consuming* the buffer, returning it) do to code built on `Bytes` pipelines?
- Tokio's task budget mechanism: how many polls/ops before forced yield, and can a pathological task still starve peers within budget?
- `async` in traits post-stabilization: current dyn-compatibility story and its cost vs `Box<dyn Future>` — re-check ecosystem idioms.
- Thread-per-core (glommio-style, no work stealing, no `Send` bounds) vs tokio's stealing: for which workload shapes does pinned-shared-nothing win, and how does it interact with [NUMA](../numa-awareness/learning.md)?

## References

- [Tokio tutorial](https://tokio.rs/tokio/tutorial) — the practical foundation; the "Shared state" and "Select" chapters map to this doc's hazards.
- Alice Ryhl, ["Async: What is blocking?"](https://ryhl.io/blog/async-what-is-blocking/) and ["Actors with Tokio"](https://ryhl.io/blog/actors-with-tokio/) — the two most operationally valuable async essays in the ecosystem; the blocking essay is this doc's chapter 2 from the source.
- *Async Rust* (without.boats' blog archives) — the deepest public reasoning on why Rust's model is poll-based state machines; read when you want the *why* behind the design.
- [tokio-console](https://github.com/tokio-rs/console) docs — the poll-time instrument; learn it before the incident.
- Jens Axboe, [io_uring design document](https://kernel.dk/io_uring.pdf) — the completion model from its author.
- Related topics in this repo: [Parallelism & Work Stealing](../parallelism-and-work-stealing/learning.md) (the other half; shared deque machinery; the runtime boundary), [Batching & Amortization](../batching-and-amortization/learning.md) (epoll/io_uring as batched waiting; buffered I/O), [Zero-Copy](../zero-copy/learning.md) (`Bytes` pipelines; registered buffers), [Memory Layout](../memory-layout/learning.md) (futures as state-machine enums), [Backpressure & Rate Limiting](../../architecture-patterns/backpressure-and-rate-limiting/learning.md) + [Idempotency](../../architecture-patterns/idempotency-and-delivery-semantics/learning.md) (bounded channels; cancellation as the drop-shaped retry question).
