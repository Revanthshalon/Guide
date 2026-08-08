# Branch Prediction — Quick Reference

Core model: branches aren't slow — *surprising* branches are. Cost ≈ miss_rate × ~17 cycles: 99%-predicted ≈ free; 50/50 data-random ≈ 8+ cycles/iteration. Two escapes from random: make the data predictable (sort/group) or delete the branch (branchless select) — entropy decides which; predictable data makes branchless a tax. Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| Hot-loop branch with double-digit miss rate on the counter | Miss rate already < a few % — predictor has it; leave it |
| Branch condition is high-entropy data (unsorted values, shuffled tags) | Data is/can-be sorted — the branch becomes free by grouping |
| `dyn` dispatch per element over mixed types in a hot loop | Arms have side effects/panics — both-sides evaluation illegal |
| Filter/threshold loops headed for SIMD masks anyway | Latency-critical chain where cmov's data dependency serializes |

## Rules of Thumb

- Counters before surgery: `branch-misses/branches` in the hot loop, nothing else justifies this work.
- Sort/partition first — the data-side fix compounds with cache locality and keeps code readable.
- Bounds checks ≈ free (never-taken predicts perfectly); their real cost is blocked vectorization.
- Branchless spelling: `(cond) as i64 * val`, `max`/`min`/`clamp`, sign masks, lookup tables. `filter()` still branches; `map(mask)` doesn't.
- The source `if` is a request — LLVM auto-cmovs and re-branchifies; only the assembly is truth (`cargo asm`, godbolt).
- Branchless = constant cost, entropy-immune; branchy = free when predictable. Crossover: miss_rate × 17 vs. overhead.
- Group-by-type → `enum_dispatch` → generics: the escalation for hot `dyn` calls (BTB thrash).
- `#[cold]` on error paths; PGO (`cargo-pgo`) is the production-grade layout hint — often free 5–15%.
- Hot recursion > ~16–32 deep mispredicts every return (RSB overflow) — go iterative.
- Sorting for predictability costs O(n log n) — pays only on repeated sweeps.

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Mispredict penalty | ~15–20 cycles ≈ 60–150 instruction slots |
| Well-predicted branch | ~1 cycle, often fused ≈ free |
| Typical real-code prediction rate | 95–99%+ |
| 50/50 branch amortized | ~8–9 cycles/iteration |
| Return stack buffer | ~16–32 entries |
| Classic sorted-vs-shuffled ratio | ~6× on identical code |

## Diagnostic Signatures

| Signature | Meaning | Fix direction |
| --- | --- | --- |
| branch-misses ≥ 10%, IPC low | Data-random branch tax | Sort/group, else branchless |
| Misses down, instructions up, time down | Branchless trade succeeded | Ship it |
| Misses down, time *up* | cmov serialized the chain | Revert; keep the branch |
| Indirect-branch misses high | `dyn`/jump-table target thrash | Group by type / enum_dispatch |
| Near-zero misses in bench, high in prod | Predictor-training mirage | Rotate/realistic inputs |

## Benchmark Checklist

- [ ] Four cells: {branchy, branchless} × {sorted, shuffled}
- [ ] branch-misses + IPC + instructions per cell, not time alone
- [ ] Inputs rotated / sized past history tables; production entropy matched
- [ ] Assembly verified: branchless is branch-free, baseline didn't auto-cmov
- [ ] Sort cost amortization honestly counted; macro baseline re-run

## Key References

- SO: ["Why is processing a sorted array faster?"](https://stackoverflow.com/q/11227809) — the canonical demo.
- Agner Fog, microarchitecture manuals — predictor internals per core.
- Lemire's blog — measured branchless experiments, the working style.
