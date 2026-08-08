# Lock-Free Concurrency — Quick Reference

Core model: "lock-free" is a progress guarantee, not a speed claim. Uncontended mutex ≈ uncontended CAS (~10–20 ns); contended *anything* = line ping-pong. Contention is the enemy — solve sharing first (per-thread + merge), choose primitives second. Ordering bugs hide on x86 (TSO) and reproduce on ARM — the M-series is the honest machine. Details in [learning.md](learning.md).

## The Pragmatism Ladder (live low)

| Rung | Tool | When |
| --- | --- | --- |
| 0 Don't share | Per-thread state + merge; rayon fold/reduce | Always first — beat rung 4 by 90× in the counter table |
| 1 Share immutable | `Arc`, `arc-swap` (read-mostly reload) | Config, routing tables, snapshots |
| 2 Locks, properly | `parking_lot` Mutex/RwLock, short critical sections | The correct default |
| 3 Proven structures | crossbeam queues/deque, channels, `dashmap` | Someone else fought ABA |
| 4 Own atomics | Counters, flags, publish pattern | With loom tests + ordering comments |
| 5 Novel structures | crossbeam-epoch, hazard pointers | Research-grade; written justification required |

## Ordering Defaults

| Use | Ordering | Note |
| --- | --- | --- |
| Counters/metrics (value is the whole message) | `Relaxed` | Reads race ongoing adds — metrics yes, decisions no |
| Publish data via flag/pointer | `Release` store / `Acquire` load | "Release publishes; Acquire subscribes" — the one pattern |
| Two-way RMW handshake | `AcqRel` | |
| Multi-atomic global order (Dekker-style) | `SeqCst` + written invariant | Rare; "SeqCst for safety" is a placebo at mfence prices |

## Rules of Thumb

- `compare_exchange_weak` in loops (LL/SC-friendly); `crossbeam::Backoff` — never unbounded hot spins in user space.
- Comment every ordering with the edge it creates (`// Release: publishes entries above`) — the review artifact.
- Reclamation preference: generational indices (`slotmap`, dissolves ABA too) → `Arc`/`arc-swap` → `crossbeam-epoch` → hazard pointers. Never free after CAS-removal.
- Contended CAS loop = retry storm; a parking mutex queues and can *win* under high contention.
- Lock-free's real wins: no holder-preemption stalls, no priority inversion, signal-safety — name the pathology you're deleting or use a mutex.
- Loom for every hand-rolled protocol; miri in CI; test on ARM.
- Spinlocks: kernels and interrupt context only; futex mutexes spin-then-park (strictly better in user space).

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Uncontended atomic RMW / mutex lock | ~10–20 ns (both ≈ one CAS) |
| Contended line steal | ~40–100+ cycles per op |
| Counter table: mutex → atomic | ~3.4× (OS removed, ping-pong remains) |
| Counter table: atomic → sharded → local | ~25× → ~4× more (sharing removed) |
| x86 Acquire/Release | Free (plain mov); SeqCst pays mfence |
| ARM Acquire/Release | Real barrier instructions — bugs reproduce here |

## Diagnostic Signatures

| Signature | Meaning | Action |
| --- | --- | --- |
| Throughput flat/falling with threads on one atomic | Deliberate single-line contention | Shard or thread-local + merge |
| Works on x86, corrupts on ARM | Missing Release/Acquire edge | Find the edge; loom-test it |
| CAS success rate collapsing under load | Retry storm / livelock | Backoff, then redesign sharing |
| Rare corpse-read in node structure | ABA | Generations/tags or epoch reclamation |

## Benchmark Checklist

- [ ] Contention swept (threads × sharing degree); uncontended path measured separately
- [ ] parking_lot baseline at same contention — beat it or delete the cleverness
- [ ] Oversubscribed regime tested (where progress guarantees pay)
- [ ] Reclamation cost included (epoch pins, refcounts — no pre-allocated flattery)
- [ ] loom + miri green; verified on ARM and x86 if shipping both

## Key References

- Mara Bos, *Rust Atomics and Locks* (free: marabos.nl/atomics) — read it whole.
- Preshing's acquire/release essays — the intuition source.
- crossbeam + loom docs — the shelf and the verifier.
