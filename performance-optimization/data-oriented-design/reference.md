# Data-Oriented Design — Quick Reference

Core model: design data for the transforms, not types for the taxonomy. Ask "what does this batch do to which bytes, at what N?" Escalation: enum-in-Vec → per-kind/per-state vectors (existence-based: branches become membership) → SoA + hot/cold. Row/AoS stays correct for entity-at-a-time all-fields access (OLTP); DoD is for the sweeps. Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| Hot sweeps over large collections (frames, batches, scans) | Entity-at-a-time, all-fields CRUD — AoS/row is *correct* there |
| `Vec<Box<dyn T>>` in the profile: misses + dispatch storm | N below ~thousands — obvious code wins |
| Per-element flag/kind branches on shuffled data | Cross-collection invariants dominate the logic |
| Heading for SIMD or rayon | ECS ceremony for one system of two Vecs |

## The Escalation (each stage deletes one tax)

| Stage | Move | Tax deleted | Counter that moves |
| --- | --- | --- | --- |
| 1 | `Vec<Box<dyn T>>` → enum-in-`Vec` | Pointer chase + vtable | LLC-misses ↓ (~3×, free in code terms) |
| 2 | Per-kind/per-state vectors; no flags (existence-based) | The branch itself + dead-item lines | branch-misses ↓ (~2×) |
| 3 | SoA + hot/cold split on the sweep | Line waste; enables SIMD | IPC ↑, vectorizes (~2–3×) |

## Rules of Thumb

- Where there's one, there are many — design for the `Vec`, N in the signature.
- Information in *where data lives* beats information in *what data says* (membership > flags).
- Identity = generational handle (`slotmap`), never bare index across mutations; newtype indices per collection.
- `swap_remove` for O(1) deletes; move between vectors on state change (at ingest, not per tick).
- SoA is *more* borrow-checker-friendly, not less — per-field Vecs borrow independently.
- APIs take slices of what they touch (`fn f(pos: &mut [Vec3], vel: &[Vec3])`) — batchable, rayon-ready.
- Tabular domain? That's `Arrow`/`polars` — SoA with an ecosystem.
- OOP stays at the edges (config, I/O, cold paths); DoD is a hot-path discipline.
- ECS (`bevy_ecs`/`hecs`) when component-combination bookkeeping dominates; two Vecs + zip until then.
- Verify stage 3 vectorized (`cargo asm` / IPC jump) — unvectorized SoA leaves the biggest multiplier unclaimed.

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Worked-example total (shape only, logic unchanged) | ~17× |
| Stage 1 alone (enum-in-Vec) | ~3×, zero design cost |
| Line efficiency: dyn-OOP sweep vs SoA sweep | <20% vs ~100% |
| Generational handle | 8 B (idx + gen) vs 8 B pointer — plus dangling detection |

## Benchmark Checklist

- [ ] Whole-transform items/sec at production N (not toy-N ns/item)
- [ ] Each stage moves *its* counter (misses → branches → IPC), else revert it
- [ ] N swept across cache levels; crossover N recorded
- [ ] Realistic churn included (spawns/deletes/moves — the overhead side)
- [ ] Vectorization verified in assembly

## Key References

- Acton, "Data-Oriented Design and C++" (CppCon 2014).
- Fabian, *Data-Oriented Design* (free: dataorienteddesign.com) — ch. 4, existence-based processing.
- Kelley, "Practical Data-Oriented Design" — DoD in a compiler, measured.
