# Memory Layout — Learning Notes

## The Hardware Mechanism

Two hardware rules generate everything in this topic:

**Rule 1 — Alignment: the machine loads naturally-aligned words.** A load of an N-byte primitive is cheapest when its address is a multiple of N ("natural alignment"). The compiler therefore guarantees it: every type has an **alignment** (`u8`: 1, `u32`: 4, `u64`/pointers: 8, SIMD vectors: 16/32/64), and a struct's fields must each sit at an offset divisible by their alignment. Where the field order makes that impossible, the compiler inserts invisible **padding bytes**. (Modern x86 and Apple Silicon handle *unaligned* loads within a cache line at ~no penalty — the real costs appear when a value straddles two lines or two pages, and when atomics or SIMD demand alignment outright. But you don't get to choose "unaligned and fast": safe Rust references require alignment by language contract — more under `packed` in When It Hurts.)

**Rule 2 — The line tax, inherited from [cache locality](../cache-locality/learning.md).** Memory arrives in 64-byte lines (128 on Apple M-series); a struct's *size* determines how many entities fit per line, and its padding is bandwidth you fetch but can never use. A 40-byte struct with 15 bytes of padding wastes 37% of every line it occupies — before any algorithm runs. Layout is where the "shrink the working set" and "raise the useful fraction of every line" levers from the cache doc actually get pulled.

The consequence of the two rules together: **struct size is not the sum of field sizes** — it's the sum *plus padding*, rounded up to the struct's own alignment (so arrays tile correctly). Field order decides the padding:

```
(repr(C) semantics — fields laid out in declaration order)
struct Bad  { a: u8, b: u64, c: u16 }   // 1 +7pad + 8 + 2 +6pad = 24 bytes
struct Good { b: u64, c: u16, a: u8 }   // 8 + 2 + 1 +5pad      = 16 bytes
```

Same fields, 33% smaller — purely from ordering largest-first. That 8-byte difference, multiplied by a million-element `Vec`, is 8 MB of working set and a third of the line traffic.

## Mental Model

**A type is a shape stamped into cache lines. You are designing the stamp.** The quantities that matter: *size* (entities per line), *padding* (guaranteed-wasted bandwidth), and *which bytes are hot* (a big struct where one field is scanned pays for all the cold ones). The Rust-specific model on top:

1. **`repr(Rust)` (the default) is field-order-free — and the compiler already optimizes it.** Unlike C, Rust may reorder fields, so the `Bad` struct above is automatically laid out like `Good`. Consequences: you get size-optimal ordering for free in pure-Rust types (declare fields in logical order; stop hand-sorting); but the layout is **unstable and unspecified** — never memcpy/transmute a `repr(Rust)` struct across a boundary, and any FFI, on-disk, or wire format needs `#[repr(C)]`, where *you* re-inherit the C rules and the manual largest-first discipline.
2. **Niches make Rust's favorite idioms free.** The compiler exploits invalid bit-patterns ("niches") to store enum discriminants without extra bytes: `Option<&T>`, `Option<Box<T>>`, `Option<NonZeroU32>` are *the same size as the inner type* (null/zero is the `None`). This is why `Option<Box<Node>>` linked structures cost nothing over raw pointers, and why choosing `NonZeroU32` over `u32` for ids is a layout act: you're donating a niche to every enum that wraps it.
3. **Enums are as big as their largest variant** (plus discriminant, minus niche luck). One 200-byte variant makes every instance 200+ bytes — including the 95% that are the 8-byte variant. The idiom: box the fat variant (`Rare(Box<BigPayload>)`), collapsing the enum to pointer-size + discriminant. `Result<T, Box<Error>>` over `Result<T, BigError>` is this rule at the API surface (clippy's `result_large_err` lint).
4. **Know your word costs:** references/`Box` = 8 B; `&[T]`, `&str`, `&dyn Trait` = 16 B (fat pointers: ptr + len/vtable); `Vec`/`String` = 24 B *on the stack* plus the heap block; `Arc<T>` = 8 B handle + 16 B of refcounts co-located with the data. A struct of three `String`s is 72 B of handles pointing at three scattered heap blocks — versus 48 B and *zero* extra hops for three `Box<str>` (16 B each) or one arena with ranges. Indirection costs both bytes *and* a line fetch; layout thinking counts both.
5. **AoS vs. SoA is layout at collection scale.** Array-of-structs (a `Vec<Particle>`) puts each entity's fields adjacent — perfect for "all fields of one entity." Struct-of-arrays (`positions: Vec<Vec3>, velocities: Vec<Vec3>, …`) puts each *field* adjacent across entities — perfect for "one field of all entities," which is what hot loops usually do, and what [SIMD](../simd/learning.md) requires. The full treatment is [data-oriented design](../data-oriented-design/learning.md); the layout-level fact is that SoA is how you get 100% useful-line fraction for a field sweep.

Where the model stops: if the profile says compute-bound, or the working set fits in L1 regardless, layout wins are unmeasurable — this doc is subordinate to [the funnel](../profiling-and-measurement/learning.md) like everything else.

## Worked Example

A game-ish entity, shrunk in four moves. Sizes verified with `std::mem::size_of` (do the same — the numbers below are exact, not illustrative, for `repr(Rust)` on a 64-bit target).

**v0 — the natural first draft: 96 bytes.**

```rust
struct Entity {
    name: String,            // 24
    id: u64,                 // 8
    kind: Kind,              // 16  ← enum with one fat variant (see below)
    active: bool,            // 1
    health: f64,             // 8
    pos: [f32; 3],           // 12
    dirty: bool,             // 1
    generation: u32,         // 4
}                            // size_of = 96 (repr(Rust) packs; padding to align 8)

enum Kind { Player, Npc, Boss { loot_table: [u64; 1] } } // illustrative fat variant
```

**Move 1 — box the fat variant** (`Boss(Box<BossData>)`): `Kind` drops to 16 → 8+niche → often 8 bytes; every entity pays pointer-size for the rare case instead of array-size. **Move 2 — shrink the integers**: `id: u64` → `NonZeroU32` (4 bytes, donates a niche — `Option<EntityId>` stays 4), `health: f64` → `f32` (game precision is fine — *a domain judgment, made consciously*). **Move 3 — intern the name**: `String` (24 B + heap block + hop) → `NameId(u32)` into a string table; the hot loops never read names anyway. **Move 4 — pack the flags**: two `bool`s → a `bitflags` `u8` (also: a `bool` is 1 byte, but two of them force padding twice).

**v1 — 40 bytes:**

```rust
struct Entity {
    pos: [f32; 3],           // 12
    health: f32,             // 4
    id: NonZeroU32,          // 4
    name: NameId,            // 4
    generation: u32,         // 4
    kind: Kind,              // 8 (boxed variant)
    flags: EntityFlags,      // 1 (+3 pad)
}                            // size_of = 40
```

2.4× smaller → 2.4× more entities per line, per L1, per GB/s of bandwidth. On a 1M-entity sweep, the cache-locality doc's arithmetic predicts roughly that factor in memory-bound throughput — verify with the size-sweep methodology (exercise).

**Move 5 — hot/cold split, when the profile demands it.** The per-frame loop touches only `pos`/`health`/`flags` (17 of the 40 bytes — 42% line efficiency). Split:

```rust
struct EntityHot  { pos: [f32; 3], health: f32, flags: EntityFlags }  // 17 → 20 B
struct EntityCold { id: NonZeroU32, name: NameId, generation: u32, kind: Kind }
// entities: Vec<EntityHot> + Vec<EntityCold>, same index — 3 hot entities per line, cold never fetched
```

That's SoA's first step (two arrays); taking it per-field is the full [DoD](../data-oriented-design/learning.md) move. Note what each move cost: an indirection (names), a precision decision (f32), API churn (flags) — layout wins are bought, not found.

## Applying It

- **See the layout before believing it:** `std::mem::size_of::<T>()` / `align_of` in a unit test (`assert_eq!(size_of::<Entity>(), 40)`) pins the size and *fails the build when someone fattens the struct* — the cheapest regression gate in this repo. `cargo +nightly rustc -- -Zprint-type-sizes` dumps every type's layout including padding holes and niche use; the `top-type-sizes` crate ranks them.
- **`repr` selection:** default `repr(Rust)` unless a contract demands otherwise; `#[repr(C)]` for FFI/wire/disk (then order fields largest-first yourself); `#[repr(transparent)]` for newtypes that must be ABI-identical to their inner type; `#[repr(align(64))]` to give a type its own cache line ([false sharing](../false-sharing/learning.md)'s tool); `#[repr(u8)]` on fieldless enums for minimal discriminants. `packed` is a trap, not a tool — see below.
- **Niche craft:** `NonZero*` for ids and counts that can't be zero; enum nesting where one niche serves several wrappers. Check what you got: `size_of::<Option<YourType>>() == size_of::<YourType>()` is the test.
- **Fat-variant hygiene:** clippy's `large_enum_variant`/`result_large_err` on; box what they flag. Same logic for rarely-used big fields (`Option<Box<Extras>>` not `Extras`).
- **Handle-size menu, for reference-heavy structs:** `&T`/`Box<T>` 8 B, `Option<Box<T>>` 8 B, `Box<str>`/`Box<[T]>` 16 B (vs `String`/`Vec` 24 B — right when immutable-after-build), `Rc`/`Arc` 8 B + refcount block, arena index `u32` 4 B. Choosing from this menu *is* layout design; the arena row is why the [cache doc](../cache-locality/learning.md)'s index-arena refactor also shrinks structs.
- **Collection-scale tools:** `soa_derive`/`soa-rs` generate SoA from an AoS definition; `bytemuck` (`Pod`/`Zeroable`) for safe reinterpret-casting of `repr(C)` data (the zero-copy load path); `smallvec`/`arrayvec`/`compact_str` trade heap hops for inline bytes when lengths are usually small — measure, they're not free (branch + bigger stack type).

## When It Hurts

- **`#[repr(packed)]` is almost always the wrong answer.** It removes padding — and makes field references UB-prone (a `&` to a misaligned field is instant undefined behavior; the compiler forces `addr_of!`/copies), breaks atomics, and can *slow* access (line-straddling loads). Reorder and shrink instead; reach for `packed` only at genuine wire-format boundaries, with copies at the edge.
- **Over-shrinking creates conversion churn.** `u16` indices everywhere save bytes and cost `as`/`try_into` noise, overflow risk at 65 536, and widening at every arithmetic site. Shrink where the *count of instances* is large (the million-element `Vec`), not on principle in one-off structs.
- **`repr(C)` forfeits Rust's help.** Fixed order disables field reordering and most niche tricks — pay it at real ABI boundaries only, not "just in case." (Serialization via serde doesn't need `repr(C)`; only raw-bytes formats do.)
- **Hot/cold splits and interning add indirection and lifetime plumbing.** Two arrays that must stay index-synchronized, a string table with its own lifetime story — real complexity; the profile, not aesthetics, should order it.
- **Micro-layout below the noise floor.** Reordering a struct that lives once on one stack frame changes nothing measurable. Layout leverage scales with instance count × access frequency; spend it on the types that dominate the heap profile ([dhat](../profiling-and-measurement/learning.md) tells you which).

## Benchmarking Methodology

- **Assert sizes in tests; diff layouts in review.** `-Zprint-type-sizes` before/after is the layout equivalent of a flamegraph diff — it shows exactly where padding went and which niches fired.
- **Measure at collection scale with the size-sweep** from the [cache doc](../cache-locality/learning.md): the win from a 96→40 B struct appears as the staircase cliffs moving right (more entities fit per level). Sweep the *entity count*, plot ns/entity, run v0 vs v1 on the same axes.
- **Isolate line-efficiency effects with counters:** same sweep under `perf stat`/cachegrind — the layout win shows as fewer `LLC-load-misses` *per entity* at identical instruction counts (or slightly higher instructions and much lower misses, for interning/flag-packing — the trade made visible).
- **Watch for allocator size-class effects:** heap blocks round up to allocator classes; a 40-byte allocation may occupy a 48-byte class. `dhat` reports both requested and actual — footprint wins can be quantized away or amplified by class boundaries.
- **A/B honestly:** same data, same traversal, only the type changed; `black_box` the field reads so dead fields aren't optimized into a fake win.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Compute (then verify) `size_of` for `repr(C)` structs `{u8, u64, u16}` vs `{u64, u16, u8}` — and explain why `repr(Rust)` makes them equal.
2. Why is `Option<Box<T>>` 8 bytes but `Option<u32>` 8 bytes too? What type makes an optional 4-byte id cost 4 bytes, and what's the general mechanism?
3. An enum has variants of 8, 12, and 480 bytes; 99% of instances are the 8-byte one. Total cost per million instances before and after the standard fix?
4. Your struct is 64 bytes; the per-frame loop reads 12 of them. What's the line efficiency, what are the two escalating fixes, and what does each cost in code terms?
5. Why does `#[repr(packed)]` interact catastrophically with references, and what should you do instead in the two common cases (pure-Rust type; wire format)?
6. A `Vec<String>` of 1M short names vs a `String` arena + `(u32, u32)` ranges: count the bytes and the pointer hops per name access in each.

Measurement exercises:

- Take the worked example through all five moves in real code: assert each `size_of` milestone, then run the 1M-entity sweep (cache-doc methodology) at v0, v1, and hot/cold — three points on one plot. Compare the measured factor to the size ratio's prediction and explain the gap.
- Run `-Zprint-type-sizes` on a real project of yours; find the three largest frequently-instantiated types, and for each: padding bytes, niche opportunities missed, fat variants. Fix one; measure or justify not measuring.
- Verify the niche table yourself: `size_of` for `Option<T>` where T ∈ {`&u8`, `Box<u8>`, `NonZeroU32`, `u32`, `bool`, `char`, `(bool, bool)`} — two of these will surprise you; explain both.

## Open Questions

- Allocator size classes on macOS (libmalloc) vs jemalloc/mimalloc: where are the class boundaries actually, and how much footprint do they quantize away for 33–48 B structs — measure with dhat against each.
- `soa-rs` vs `soa_derive` vs hand-written SoA in 2026: ergonomics, iterator quality, and whether the generated code optimizes as well as manual — trial on the entity example.
- When does `compact_str`/`smallvec`'s inline-vs-heap branch cost more than the saved hop — find the length distribution where it flips on the benchmark.
- Cross-language check: how do these rules map onto what `#[repr(C)]` guarantees for interop with a C library you actually use — write the static assertions both sides.
- Bit-packing beyond `bitflags` (4-bit fields, packed arrays via `bitvec`): at what field width does extraction cost exceed the line-efficiency win in a scan?

## References

- [The Rustonomicon, "Data Layout"](https://doc.rust-lang.org/nomicon/data.html) — the authoritative chapter on repr(Rust)/repr(C)/packed/align semantics; short and load-bearing.
- Nicholas Nethercote, [The Rust Performance Book — "Type Sizes"](https://nnethercote.github.io/perf-book/type-sizes.html) — the practical catalog this doc's Applying-It section extends, including `-Zprint-type-sizes` workflow.
- [`std::mem` docs](https://doc.rust-lang.org/std/mem/) (`size_of`, `align_of`) + [`bytemuck`](https://docs.rs/bytemuck) — the verification and reinterpretation toolset.
- Ulrich Drepper, *What Every Programmer Should Know About Memory*, §6 — the C-era struct-packing craft; mechanisms unchanged, Rust automates half of it.
- Related topics in this repo: [Cache Locality](../cache-locality/learning.md) (why size and padding are bandwidth), [Data-Oriented Design](../data-oriented-design/learning.md) (AoS/SoA taken to program scale), [False Sharing](../false-sharing/learning.md) (`repr(align(64))`'s reason to exist), [SIMD](../simd/learning.md) (alignment and SoA as prerequisites), [Allocation Strategies](../allocation-strategies/learning.md) (the heap-side half of footprint).
