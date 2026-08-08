# Cache Locality — Quick Reference

Core model: memory moves in 64 B lines (128 B on Apple M-series); price of a byte = distance to its line (L1 ≈ 1 ns … DRAM ≈ 100 ns, a 1:50–100 ratio). Use whole lines (spatial), reuse while resident (temporal), keep the address stream predictable (prefetcher). Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| IPC ≤ 1 with high LLC misses (latency-bound) | Working set already fits L1/L2 — flat part of the staircase |
| Big sweeps over collections; wasted-line fraction high | IPC ~3–4, low misses: compute-bound → SIMD/algorithm instead |
| Pointer-graph traversals dominate the profile | Access is necessarily random and small — compact the target instead |
| Data grew past a cache level ("suddenly 10× slower") | Tiling/flattening costs clarity the profile hasn't justified |

## Rules of Thumb

- `Vec`/slice is the default; every pointer structure must justify itself. `LinkedList`: no.
- Traverse the contiguous axis innermost; flatten `Vec<Vec<T>>` to one `Vec<T>`.
- Pointer graphs → index arenas (`Vec<Node>` + `u32` ids): denser, halves reference size, pleases the borrow checker.
- Fuse passes (iterator chains are one pass by construction); tile what can't fuse to L2-sized chunks.
- Random keys? Sort, then sweep — convert random into sequential.
- Shrink the working set: smaller ints, hot/cold split — every byte shaved is line capacity back.
- Sequential ≈ prefetch-hidden; strided = waste × line-fraction; random = full latency, serialized (~10× worse again).
- HashMap iteration is randomized scatter — iterate a `Vec` beside it.
- Linear scan beats HashMap up to ~tens of elements; measure your crossover once.

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Cache line | 64 B x86 / **128 B Apple M-series** |
| L1 / L2 / L3 / DRAM latency | ~1 ns / ~4 ns / ~15 ns / ~60–110 ns |
| L1d size | 32–48 KB (x86) / 128 KB (M P-core) |
| TLB reach (4 KB pages) | few MB — then dTLB misses stack on cache misses |
| f64s per x86 line | 8 — stride-8 wastes 7/8 of every fetch |
| Sequential vs random full-array sweep | ~50–100× (measure yours) |

## Diagnostic Signatures (`perf stat` / cachegrind)

| Signature | Meaning | Go to |
| --- | --- | --- |
| IPC ≤ 0.5, LLC-miss ≈ 1/access | Latency-bound pointer chasing | Arenas, batch+sort, layout |
| High BW use, moderate IPC | Bandwidth-bound, wasting lines | Shrink types, SoA, fuse passes |
| dTLB-load-misses high | Page-walk tax on scatter | Huge pages + compaction |
| Slow at power-of-two size/stride | Set-conflict artifact | Pad the leading dimension |

## Benchmark Checklist

- [ ] Size sweep 16 KB → 1 GB done once; staircase plot kept (know your L1/L2/L3 steps)
- [ ] Benchmark size states which cache level it measures; cold paths sized past L3 or buffers rotated
- [ ] Latency measured with shuffled order; throughput with the real pattern — deliberately chosen
- [ ] Counters recorded, not just time (IPC + misses name the mechanism)

## Key References

- Drepper, *What Every Programmer Should Know About Memory* (parts 2–4, 6).
- [Interactive latency numbers](https://colin-scott.github.io/personal_website/research/interactive_latency.html).
- Nethercote, [Rust Performance Book](https://nnethercote.github.io/perf-book/).
