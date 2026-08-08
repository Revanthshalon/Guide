# False Sharing — Quick Reference

Core model: the cache line is the unit of *coherence*, not just transfer — writing a line invalidates every other core's copy (~40–100+ cycles to steal back). False sharing = independent variables in one line ping-ponging between cores: 10–50× per-op tax, **negative** thread scaling, invisible in code. Density rules invert for written data: pack what's read, spread what's written — split by *writer*. Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| Per-thread throughput *falls* as threads rise | Data isn't write-shared at meaningful rates — padding wastes cache (16× footprint) |
| `counters[thread_id]`-shaped arrays; per-worker slots in shared arrays | The sharing is *true* (one variable, all threads) — padding can't fix algorithms |
| Hot-written field inside a read-mostly shared struct | Falling curve is actually locks / bandwidth roofline / NUMA / imbalance |
| `Arc::clone` per item in hot pipelines (refcount line) | |

## The Escalation

| Step | Move | Cost |
| --- | --- | --- |
| Isolate | `CachePadded<T>` per slot (128 B on the platforms that need it) | Wasted bytes; perfect scaling back |
| Localize | Thread-local accumulate + rare merge; rayon `fold`/`reduce` | Read-side lag; ~free per op |
| Restructure | Shard the counter; clone `Arc` once per thread; move hot field out of shared struct | Design work — the true-sharing fix |

## Rules of Thumb

- Writes are the poison; any-core reads of a written line pay the ping-pong too (the reader tax).
- Plain writes false-share exactly like atomics — no `Atomic` needed for the bug.
- Pad the *usage site* (array slot), not the payload type — `repr(align(128))` propagates into every container.
- Mutex + its data on one line is *good* sharing (one steal gets both).
- Ring-buffer head/tail on separate lines — read crossbeam's source once.
- Intel adjacent-line prefetcher + Apple Silicon: treat conflict granularity as 128 B.
- Try-the-padding is a legitimate 5-minute diagnostic; *keeping* speculative padding isn't.
- macOS has no `perf c2c` — the thread-scaling curve is your instrument; control benchmark layout explicitly (print slot addresses).

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Cross-core line transfer | ~40–100+ cycles (topology-dependent) |
| Falsely-shared vs padded increment | ~10–50× per op |
| Worked example: 8 threads adjacent | *Slower than 1 thread* (negative scaling) |
| `CachePadded<AtomicU64>` | 8 B payload, 128 B footprint |
| Thread-local vs padded-atomic | ~3.5× again (RMW cost isolated) |

## Diagnostic Signatures

| Signature | Meaning | Action |
| --- | --- | --- |
| Per-thread throughput falls with thread count | Coherence traffic (false or true) | Padding experiment; read who-writes-what |
| Padding fixes it | Was false sharing | Keep padding or localize |
| Padding changes nothing | True sharing / other cause | Shard, localize, or check locks/roofline/NUMA |
| `perf c2c` HITM flood on one line (Linux) | Attribution: the line and fields named | Fix that layout |

## Benchmark Checklist

- [ ] Thread sweep 1/2/4/…: per-thread *and* total throughput
- [ ] Slot adjacency verified by address math (allocator size classes fake padding)
- [ ] Three variants (adjacent / padded / thread-local) — two subtractions attribute coherence vs RMW
- [ ] Threads pinned or variance reported (P/E-core migration)
- [ ] Scaling benchmark kept as CI regression guard

## Key References

- Drepper, §3.3.4 / §6.4.2 — MESI and the fix catalog.
- `CachePadded` source — platform granularity table in `cfg` form.
- `perf c2c` docs — attribution when Linux is available.
