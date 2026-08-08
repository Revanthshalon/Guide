# Parallelism & Work Stealing — Quick Reference

Core model: speedup fights three limits — serial fraction (Amdahl: 1/(s+(1−s)/N)), the shared DRAM pipe (memory-bound flattens at 2–3 cores regardless of count), and coordination cost (communication + imbalance). Work stealing solves the third: per-worker deques, owner LIFO-bottom (cache-warm), thieves FIFO-top (biggest subtree, rare steals) — coordination paid only when imbalance exists. Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| Compute-bound sweeps over dense (DoD) data | Memory-bound — GB/s already near ceiling; fix intensity/layout first |
| Irregular/heavy-tailed per-item costs (stealing balances) | Per-item work ≈ ns without `with_min_len` (scheduling drowns work) |
| Divide-and-conquer trees (join/scope) | Serial fraction dominates (lock in closure, single-threaded merge) |
| Serial version already optimized | It isn't — N cores × waste = N× waste, and complexity hides the fix |

## Rules of Thumb

- rayon by default: `par_iter`, `join`, `scope`; one pool per process; tokio ≠ rayon — CPU work bridges out, never inline on the async runtime.
- Accumulate via `fold`+`reduce` (thread-local partials) — a `Mutex`/`Atomic` in the closure is experiment C: self-inflicted Amdahl.
- Granularity = the batching knee (N ≈ F/m): trust adaptive splitting, `with_min_len`/`par_chunks` when items are tiny.
- Amdahl quick table @8 cores: s=5% → 5.9×; s=10% → 4.7×; s=25% → 2.9×. Measure s by curve-fit, then *name the lock it lives in*.
- Hidden serial fractions: locks, allocator (arena-per-worker), `println!`/tracing in closures (stdout lock!), per-item channel sends, final sort/collect.
- Speedup vs *best serial* is the honest metric — 8 threads of scalar pointer-chasing can lose to one SIMD core.
- Parallel float reduction reorders additions — determinism sign-off, run-to-run variance in tests.
- M-series: E-cores inflate `num_cpus`; measure with P-core-sized pool too; report migration spread.
- Total thread audit: tokio + rayon + ad-hoc all sized num_cpus = 3× oversubscription.

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Thread spawn / context switch | ~10s of µs / ~1–10 µs + cache repopulation |
| Memory-bound sum @8 cores | ~2.8× (the wall) vs compute-bound ~7.6× |
| Heavy-tail: static chunks vs stealing | ~3.9× vs ~7.4× |
| Accumulator: Mutex / Atomic / fold+reduce | ~1.9× / ~3.1× / ~7.7× |
| Rayon task overhead | ~ns-scale but real — measure yours via the knee |

## Diagnostic Signatures (scaling curve = the instrument)

| Signature | Meaning | Action |
| --- | --- | --- |
| Flattens early, GB/s ≈ ceiling | Bandwidth wall | Fuse passes, shrink data, accept |
| Flattens, GB/s low | Serial fraction / contention | Amdahl-fit; audit locks; padding experiment |
| Ragged per-worker busy histogram | Imbalance (static split) | Work stealing / finer tasks |
| Slower than sequential | Tiny tasks | `with_min_len` (knee formula) |
| Sawtooth variance on M-series | P/E migration | Pin or report spread |

## Benchmark Checklist

- [ ] Scaling curve 1..N with efficiency; pool size explicit
- [ ] GB/s reported against measured ceiling (separates the two flattenings)
- [ ] Amdahl fit → s extracted and attributed
- [ ] Per-worker busy/idle histogram for irregular loads
- [ ] Baseline = best *serial* version, not naive serial

## Key References

- Rayon docs + Matsakis's design post.
- Chase & Lev 2005 — the deque (crossbeam's blueprint).
- McKenney, *Is Parallel Programming Hard…* (free) — the deep end.
