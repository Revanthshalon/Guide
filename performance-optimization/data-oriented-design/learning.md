# Data-Oriented Design — Learning Notes

## The Hardware Mechanism

Data-oriented design introduces no new hardware — it is the *architectural response* to the three mechanisms already established, so this section is an indictment rather than an introduction. The defendant is the default object-oriented program shape: entities as heap-allocated objects, behavior as per-object (often virtual) methods, relationships as pointers. Judged by the machine, that shape pays all three taxes at once, on every hot loop:

- **The line tax** ([cache locality](../cache-locality/learning.md)): objects scattered by the allocator mean every entity is a cache-line fetch (or several); fat objects with cold fields mean the fetched lines are mostly waste. Line efficiency of "call `update()` on each object in a `Vec<Box<dyn Entity>>`" is routinely under 20%.
- **The latency chain** ([cache locality](../cache-locality/learning.md) again): pointer-linked objects defeat the prefetcher — each `Box` deref is a serialized full-latency miss. The allocator's placement, not your access pattern, decides adjacency.
- **The dispatch and branch tax** ([branch prediction](../branch-prediction/learning.md)): a virtual call per object over heterogeneous types thrashes the indirect predictor; per-object flag checks (`if entity.alive`, `if e.kind == …`) over shuffled data are the 50/50-branch disaster.
- Plus the **footprint tax** ([memory layout](../memory-layout/learning.md)): vtable pointers, `Box` indirections, per-object allocator overhead, and padding — bytes that displace real data from every cache level.

None of these costs appears in the code's *logic*; all of them appear in its *shape*. DoD's premise: reshape the data, keep the logic.

## Mental Model

**Design the data for the transforms that run over it — not the types for the taxonomy the domain suggests.** OOP asks "what *is* an entity, and what can it do?" DoD asks "**what does this frame/request/batch actually do to which bytes, for how many items?**" — and lets the answers dictate layout. The tenets:

1. **The purpose of a program is to transform data.** A frame update, a request handler, a query — each is a pipeline of transforms over collections. Performance design = matching each collection's layout to its dominant transform's access pattern. (This is why the model transfers straight to databases: a row store *is* AoS, a column store *is* SoA, and OLTP-vs-OLAP is the same decision this doc teaches, made by an industry forty years ago.)
2. **Where there's one, there are many.** Never design for `Entity` — design for `Vec<Entity>` (there is always a plural; if N were 1 you wouldn't be optimizing). The interesting number in every design conversation is N and the access pattern at N, not the elegance of the singular type.
3. **Separate by access pattern, not by concept.** Three escalating moves, each deleting one tax:
   - **AoS → SoA**: fields the sweep reads become their own arrays (`positions: Vec<Vec3>`, `healths: Vec<f32>`) — line efficiency for a per-field sweep goes to ~100%, and [SIMD](../simd/learning.md) becomes possible ("SoA is how data asks to be vectorized").
   - **Hot/cold split**: fields the sweep *doesn't* read leave the hot arrays entirely ([memory layout](../memory-layout/learning.md)'s move, at collection scale).
   - **Existence-based processing**: state flags become *membership*. Instead of `alive: bool` checked per iteration, dead entities aren't *in* the alive list; instead of `match kind` per element, each kind has its own vector processed by its own loop. The branch doesn't get predicted better — **it ceases to exist**, and with it the cold data of the excluded cases. This is the deepest DoD move and the least obvious: *put information in* where *data lives* *rather than in* what data says.
4. **Identity becomes an index.** When layout owns placement, entities need stable names that survive reshuffling: generational indices (`slotmap`-style `(index, generation)` handles) replace pointers — 4–8 bytes, no lifetime fights, dangling detected by generation mismatch. The [arena move](../cache-locality/learning.md), industrialized.
5. **ECS is this model productized.** Entity-component-systems (bevy_ecs, hecs, flecs) are existence-based SoA with automated bookkeeping: components in dense per-archetype columns, queries as typed sweeps over exactly the touched columns, membership changes handled by the framework. Useful as infrastructure *and* as proof the model scales to real engines — but ECS is the *industrial form*, not the entry fee: two parallel `Vec`s and a zip is already DoD.
6. **Abstraction survives; its boundary moves.** DoD is routinely caricatured as "no abstraction." The honest version: abstraction boundaries go *around transforms over collections* (`fn integrate(pos: &mut [Vec3], vel: &[Vec3], dt: f32)`) instead of *around single entities* (`entity.update(dt)`). The former is testable, parallelizable, and layout-honest; the latter hides N and the access pattern — precisely the two things that matter.

Where the model stops: when access is genuinely *entity-at-a-time and all-fields* (a CRUD handler loading one order), AoS/row layout is *correct* — that's not a failure of nerve, it's the OLTP half of the database wisdom. DoD is for the sweeps; most systems contain both shapes, and the design act is knowing which collection is which.

## Worked Example

A simulation tick over 1M entities: move the living, damage the burning, expire the dead. Four stages, each deleting one tax; illustrative numbers (typical x86 desktop — reproducing the staircase is exercise one) and the counter that moves at each stage.

**Stage 0 — idiomatic OOP: 1.0× baseline.**

```rust
entities: Vec<Box<dyn Entity>>          // heap-scattered, vtable each
for e in &mut entities { e.update(dt); }
```

`perf stat`: IPC ~0.4, LLC-misses ≈ one per entity, indirect-branch misses high. All three taxes at once: pointer chase + line waste + dispatch storm. **~14 ms/tick.**

**Stage 1 — one enum, one Vec (AoS, no dispatch): ~3× faster.**

```rust
enum Kind { Walker(WalkerData), Burner(BurnerData) }
struct Entity { kind: Kind, pos: Vec3, vel: Vec3, health: f32, alive: bool }
entities: Vec<Entity>                   // contiguous; match instead of vtable
```

Contiguity restores prefetch; the `match` is one predictable-ish jump instead of a BTB-thrashing call. LLC-misses collapse; branch misses remain (shuffled kinds, `alive` checks). **~4.5 ms.** *This stage is free in code terms — it's just choosing enum-in-Vec over trait objects — which is why it's the default Rust idiom.*

**Stage 2 — existence-based: per-kind, per-state vectors: ~2× again.**

```rust
walkers: Vec<Walker>,  burners: Vec<Burner>      // membership = kind
// death: swap_remove into a freelist; no `alive` flag anywhere
for w in &mut walkers { integrate(w); }           // zero branches in the body
for b in &mut burners { integrate(b); burn(b); }
```

The `match` and the `alive` check *cease to exist*; dead entities occupy no line. Branch-misses drop to ~noise. **~2.2 ms.** Cost paid: identity is now a generational handle (`slotmap`), and cross-references go through it.

**Stage 3 — SoA on the hot loop: ~2–3× again, and SIMD-ready.**

```rust
struct Walkers { pos: Vec<Vec3>, vel: Vec<Vec3>, /* cold: */ meta: Vec<Meta> }
for (p, v) in walkers.pos.iter_mut().zip(&walkers.vel) { *p += *v * dt; }
```

The integrate sweep now reads *only* position/velocity bytes — 100% line efficiency — and autovectorizes (check with `cargo asm`; the [SIMD doc](../simd/learning.md) takes it further). IPC climbs past 3. **~0.8 ms. Total: ~17× from shape alone — the logic never changed.**

The stage-by-stage counter story is the pedagogy: stage 1 fixes *misses*, stage 2 fixes *branches*, stage 3 fixes *line efficiency and width*. Three docs, one refactor each.

## Applying It

- **The two-Vec starter kit.** DoD in Rust begins with parallel `Vec`s + `zip` — no framework: `positions.iter_mut().zip(&velocities)` fuses into one clean loop, and the borrow checker *rewards* the split (per-field `Vec`s borrow independently; the classic "can't borrow two fields of the same struct through a method" fight dissolves — SoA is *more* idiomatic Rust than object graphs, not less).
- **Stable identity: `slotmap`/`generational-arena`** for handles (`(u32 idx, u32 gen)`); `swap_remove` for O(1) deletion from dense vectors (order isn't sacred in a set); freelists for recycling. Never bare indices across mutations — that's the use-after-free of DoD, and generations are the cheap fence.
- **Group at ingest, not per frame.** The per-kind/per-state split is cheapest maintained *incrementally* (move an entity between vectors on state change — O(1) with swap_remove + handle fixup) rather than re-partitioned per tick. State transitions become explicit membership moves: the design makes state machines *visible* in the data topology.
- **Reach for ECS when bookkeeping dominates:** many component types × many access patterns × frequent composition changes = `bevy_ecs`/`hecs` territory (archetype storage automates the per-combination vectors). One system's worth of parallel Vecs? Skip the framework.
- **Transform-shaped APIs.** Public functions take slices/iterators of the data they touch (`fn settle(invoices: &mut [Invoice], ledger: &Ledger)`), not single items in a loop at the call site — the N and the access pattern become part of the signature, batchable and rayon-parallelizable (`par_iter` over dense vectors is embarrassingly clean — the [parallelism doc](../parallelism-and-work-stealing/learning.md) inherits this shape).
- **Columnar kinship:** for data-analysis workloads, this whole doc is spelled `Arrow`/`polars` — columnar formats are SoA with an ecosystem; use them rather than hand-rolling when the domain is tabular.
- **Keep OOP at the edges.** Config, plugins, I/O drivers, cold orchestration — trait objects are fine where N is small and calls are rare. DoD is a *hot-path* discipline, not a religion; the skill is drawing the line where the profile says, not where the paradigm preference does.

## When It Hurts

- **Entity-at-a-time, all-fields access.** Load one order, touch every field, write it back: row/AoS layout is optimal and SoA pays a scattered fetch per field (the OLTP case). Most services are *mostly* this shape — apply DoD to their few genuine sweeps, not their CRUD.
- **Small N.** Below thousands of items, the obvious `Vec<Struct>` with a plain loop is within noise of anything clever. The two-Vec kit costs nothing, but ECS adoption, handle indirection, and membership choreography for N=200 is architecture cosplay.
- **Index-synchronization bugs replace lifetime bugs.** Parallel arrays must stay aligned; membership moves must fix up every handle. The type system that guarded pointers doesn't guard indices — newtype the index per collection (`WalkerIdx(u32)`), and let `slotmap` own the invariants rather than hand-rolling.
- **Cross-collection invariants get harder.** "Damage the walker its burner targets" now spans two vectors via handles — logic that was one method call is a two-lookup dance, and batch-ordering questions appear (all burns then all deaths, or interleaved?). Phase-structured ticks (read phase → write phase) restore order; they're real design work.
- **Domain readability can suffer** — `entity.take_damage(x)` reads like the domain; `healths[h.idx] -= x` reads like the machine. Mitigate with transform-level naming (`apply_damage(healths, hits)`) and accept that hot-path code serves the machine first; that's the contract DoD signs.

## Benchmarking Methodology

- **Measure whole transforms, in items/sec** (entities/tick, rows/query) at *production N* — per-item nanoseconds at toy N sits on the wrong staircase step ([cache doc](../cache-locality/learning.md)) and hides everything this discipline fixes.
- **Attribute each stage with its counter:** stage transitions should move the counter they claim (misses, then branch-misses, then IPC/line-efficiency). A stage that doesn't move its counter at your N is a stage you revert — the staged refactor is also a staged *measurement* protocol.
- **Sweep N across the cache levels**: DoD's advantage *grows* with N (OOP's scatter degrades faster than SoA's streams); the crossover N where reshaping starts paying is a number to measure once per workload class, not to assume.
- **Include churn in the benchmark:** membership moves, spawns, and deletes are DoD's overhead side — a benchmark of pure sweeps flatters it. Simulate realistic mutation rates; measure the swap_remove/fixup cost against the sweep savings.
- **Verify vectorization actually happened** at stage 3 (`cargo asm`, or the IPC jump): SoA that doesn't vectorize left the biggest multiplier on the table, usually over an alignment or iterator-shape detail.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Name the three taxes `Vec<Box<dyn Entity>>` pays and the specific stage of the worked example that deletes each.
2. Why is existence-based processing strictly stronger than making the `alive` branch predictable? Two distinct resources it reclaims.
3. Map OLTP/row-store and OLAP/column-store onto this doc's vocabulary. Why is a CRUD handler *correctly* AoS?
4. Why does the borrow checker *prefer* SoA — what fight disappears when fields become separate `Vec`s?
5. What failure mode replaces use-after-free in index-based designs, and what two mechanisms fence it?
6. When is ECS the wrong tool despite the workload being sweep-heavy? Give the N-shaped and the composition-shaped answer.
7. Your stage-2 refactor didn't move branch-misses. List three hypotheses in the order you'd test them.

Measurement exercises:

- Build the four-stage worked example (1M entities, two kinds, 10% churn/tick) and produce the stage table on your machine: ms/tick + the three counters per stage (Instruments/cachegrind on macOS). Compare your ratios to the doc's; explain the biggest deviation.
- Take the stage-3 hot loop and break its vectorization deliberately (iterate via indices with a bounds-checked gather, or make Vec3 `repr(C)` with a padding field) — watch IPC drop with counters otherwise flat. Reversing it teaches the verify-vectorization habit.
- Find a real sweep in code you own (serialization pass, validation loop, report aggregation); apply *only* stage 1 (enum-in-Vec / flatten the indirection) and measure. Stage 1's cost-free 3× is the most transferable result in this doc.

## Open Questions

- bevy_ecs vs hecs vs hand-rolled at the 1M-entity benchmark: framework overhead per entity, and where archetype moves (composition changes) start dominating — measure.
- Rayon over SoA vs over AoS chunks: how does layout interact with work-stealing granularity ([parallelism doc](../parallelism-and-work-stealing/learning.md) crossover)?
- `soa-rs`/`soa_derive` ergonomics at stage 3 vs hand-written struct-of-Vecs on the worked example — does the generated iteration codegen identically?
- Phase-structured ticks (read/write separation) vs interleaved with fixups: measurable cost of each discipline at high churn?
- Where does the enum-in-Vec (stage 1) vs per-kind-Vecs (stage 2) crossover sit when kinds are *many* (20+) and unbalanced — does the match's predictability at sorted-by-kind order close the gap?

## References

- Mike Acton, "Data-Oriented Design and C++" (CppCon 2014) — the field's founding polemic; the "typical C++ bullshit" slides are this doc's Hardware Mechanism section delivered with more anger.
- Richard Fabian, *Data-Oriented Design* ([dataorienteddesign.com/dodmain](https://www.dataorienteddesign.com/dodmain/) — free online) — the book-length treatment; existence-based processing (ch. 4) is the load-bearing chapter.
- Andrew Kelley, "Practical Data-Oriented Design" (Handmade Seattle 2021) — the same ideas applied to a compiler (Zig's), with measured wins; the best evidence DoD isn't games-only.
- [bevy_ecs](https://docs.rs/bevy_ecs) / [hecs](https://docs.rs/hecs) docs — archetype storage as running production code; read bevy's storage internals once.
- [slotmap](https://docs.rs/slotmap) docs — generational indices done right; the identity half of the discipline.
- Related topics in this repo: this doc *is* [cache locality](../cache-locality/learning.md) + [memory layout](../memory-layout/learning.md) + [branch prediction](../branch-prediction/learning.md) composed at architecture scale; [SIMD](../simd/learning.md) is stage 4; [parallelism](../parallelism-and-work-stealing/learning.md) inherits the transform-shaped APIs; [profiling](../profiling-and-measurement/learning.md) arbitrates every stage.
