# Batching & Amortization — Quick Reference

Core model: every operation = fixed F + marginal m. Singly: N×(F+m); batched: F+N×m. Knee at **N ≈ F/m** — past it, bigger batches buy nothing and cost latency/memory. The fixed-cost ladder spans 7 orders of magnitude (function call → syscall ~1 µs → fsync → network RTT ~1 ms+): hunt at the top rung first (N+1 queries beat buffer tuning). Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| Crossing count ≈ item count (write-per-record, query-per-row, msg-per-publish) | F already ≪ m — machinery for nothing |
| High-F boundary in the loop (syscall, fsync, RTT) | Latency budget can't afford the wait (T floor) |
| Tiny per-item work in parallel/channel consumers | Items need independent failure fates |
| Counters/metrics hammering shared atomics | Batch working set would exceed cache / lock hold time |

## The Fixed-Cost Ladder (memorize the rungs)

| Crossing | F | Knee example (m) |
| --- | --- | --- |
| Function call | ~1–5 ns | Compiler inlines (not your job) |
| Atomic/lock | ~10–20 ns | Per-thread accumulate, merge later |
| Syscall | ~100–500 ns+ | BufWriter: ~100–200 items |
| Context switch | ~1–10 µs | Drain channels per wakeup |
| fsync | ~50 µs–ms | Group commit |
| Network RTT | ~1–100 ms | N+1 → `ANY($ids)`/JOIN: 100–1000× |

## Rules of Thumb

- Size-or-time always: flush at N items **or** T ms — N rules under load (throughput), T rules at trickle (latency floor).
- Hoist before batching: identical fixed work (parse, handshake, regex compile) is paid *once*, not amortized.
- Verify crossings, not just time (`strace -c`/`dtruss`, query logs) — counts are deterministic CI gates.
- Any query/RPC inside a loop is a finding until proven cold.
- APIs take slices — item-at-a-time signatures forbid callers from amortizing.
- Buffered ≠ durable: name the crash story per batcher (acceptable loss / outbox / flush = txn boundary). Audit `flush()`.
- Bound the queue: unbounded batching = buffer bloat (Little's law: in-flight = rate × wait).
- Wildly varying item cost → batch by bytes/work, not count (head-of-line).
- Consumers drain per wakeup; producers coalesce into one `Vec` per send.
- Failure semantics decided at API design: split-and-retry / per-item results / all-or-nothing + idempotent retry.

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Syscall overhead | ~100–500 ns+ before any work |
| 10M unbuffered writes → BufWriter | ~25×; syscalls 10M → ~50 K |
| 8 KB → 1 MB buffer | ~1.2× — the knee, observed |
| N+1 (1000 items, 1 ms RTT) → batch | ~350× |
| Knee formula | N ≈ F/m |

## Benchmark Checklist

- [ ] Batch-size sweep with **two curves**: throughput and p99 item latency
- [ ] Crossing count verified alongside time
- [ ] Both regimes measured: flood (N-bound) and trickle (T-bound — the embarrassing one)
- [ ] Failure path exercised (injected bad item; split-and-retry timed)
- [ ] Item-age-at-flush metric built in for production

## Key References

- Dean & Barroso, "The Tail at Scale" — the latency invoice.
- Gregg, *Systems Performance* ch. 3–5 — crossing-cost anatomy.
- io_uring design doc — syscall amortization industrialized.
