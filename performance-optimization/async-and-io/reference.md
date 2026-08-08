# Async & I/O — Quick Reference

Core model: async is concurrency for *waiting* (parallelism computes more at once; async waits more at once, on fewer threads). Rust futures = compiler-generated inert state machines; task switch = function return (~ns) vs thread switch (~µs) — that gap is the entire performance content. Cooperative: tasks yield only at `.await` — blocking a worker freezes all its tasks. Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| Thousands of concurrent waits (connections, fan-out RPC) | ≲ hundreds of connections — threads are simpler, debuggable, blocking-immune |
| Idle-heavy connection populations (tasks cost only memory) | CPU-bound work (bridge to rayon; never inline) |
| Protocol servers, proxies, high fan-out | Low-concurrency CLIs/tools — complexity rent, no dividend |

## The Cardinal Hazards

| Hazard | Signature | Fix |
| --- | --- | --- |
| Blocking the runtime (sync call/CPU loop in handler) | p50 fine, **p99 explodes**, CPU not saturated | `spawn_blocking` (wait-shaped) / rayon bridge (compute-shaped); tokio-console poll times find it |
| Cancellation data loss (`select!`/`timeout` drops mid-op) | Rare protocol desync "under load" | State outside the future (buffer in the connection, `Framed`-style); audit every select arm: "dropped here — what's lost?" |
| Mutex held across `.await` | Deadlock / won't compile (Send) | Drop guard before await; tokio::Mutex if must; actor pattern usually best |
| Unbounded channels | Overload → RAM + latency (buffer bloat) | Bounded channels = backpressure; `Semaphore` for in-flight caps |
| Task leaks (dropped JoinHandle) | Orphans holding sockets | `JoinSet`/`TaskTracker` + `CancellationToken` |
| Future bloat (big locals across awaits) | Multi-KB state machines | `Box::pin` at spawn; buffers external |

## Rules of Thumb

- Every external await gets a `timeout` — no deadline = unbounded liability.
- `tokio::fs` is `spawn_blocking` underneath (epoll can't do files); io_uring changes this on Linux only — macOS ceiling is kqueue readiness.
- `spawn_blocking`'s pool is ~unbounded — semaphore it against thundering herds.
- Buffered I/O, batching, `Bytes` pipelines all still apply — async changes who waits, not what's worth doing.
- Manual ready-loops (`try_read`) need `yield_now()` — don't starve the worker.
- Cancel-safe set: tokio channel `recv` yes; `read_buf` into external buffer yes; hand-rolled multi-step loops almost certainly not.
- One process: tokio workers + rayon + your threads ≠ 3 × num_cpus.
- Threads win at low concurrency — async is a scaling instrument, not a style.

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Task switch vs thread switch | ~ns (function return) vs ~µs (+ cache repopulation) |
| Task vs thread footprint | ~hundreds of B vs MB-scale stack (~1000× density) |
| NVMe / same-DC RTT / WAN | ~20–100 µs / ~50–500 µs / ~10–100 ms |
| Blocking incident (8 workers, 1% × 50 ms @ 800 rps) | p99 2 ms → ~260 ms; p50 barely moves |
| Worker count | ≈ cores — a handful of blockers freezes the fleet |

## Benchmark Checklist

- [ ] Open-loop (constant-rate) load generator — coordinated omission hides exactly the stalls you hunt
- [ ] p50/p99/p999 at fixed rates; concurrency axis swept (100 → 50 K conns) vs thread baseline
- [ ] tokio-console poll-time histogram = the async flamegraph; blocking-pool depth watched
- [ ] Pathologies injected in staging: 1% sync sleep, timeout storm (cancel-safety), slow consumer
- [ ] Graceful shutdown under load measured (drain, zero loss)

## Key References

- Tokio tutorial; Alice Ryhl's "What is blocking?" + "Actors with Tokio."
- tokio-console — learn before the incident.
- Axboe, io_uring design doc.
