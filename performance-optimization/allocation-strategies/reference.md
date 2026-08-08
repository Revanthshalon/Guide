# Allocation Strategies — Quick Reference

Core model: cost = frequency × (allocator work + placement damage). Four levers in order: allocate **less** (reuse/borrow), **once** (capacity), **together** (arenas, by lifetime), **elsewhere** (inline/stack) — then the meta-lever (swap the allocator). Cheap levers first: reuse/borrow typically removes 99% of allocations before any machinery. Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| dhat/flamegraph shows malloc/free/`drop_in_place` width | Allocation isn't in the profile — skip the plumbing |
| Per-iteration temporaries (String/Vec per line/request) | Long-lived, individually-dropped, size-varied objects — that's what malloc is *for* |
| Phase-shaped lifetimes (request/frame/batch die together) | Arena data carries RAII resources (Drop trap) |
| Threaded service with cross-thread frees | A mutexed pool would out-contend the allocator's thread cache |

## The Levers

| Lever | Idiom | Wins when |
| --- | --- | --- |
| Allocate less | Hoist + `clear()` (keeps capacity); borrow `&str` slices; miss-path-only insert | Temporaries die per iteration |
| Allocate once | `with_capacity`; exact-size `collect` | Final size knowable |
| Allocate together | `bumpalo` bump arena: alloc = ptr bump, free = reset; `typed-arena` if Drop needed; index-arena `Vec<T>` for API-crossing data | One death-time for many objects |
| Allocate elsewhere | `SmallVec`/`ArrayVec`/`CompactStr`/`Cow` | Distribution measured mostly-small |
| Meta | `#[global_allocator]` mimalloc/jemalloc | Alloc-heavy + threaded; one line, try early, measure |

## Accidental-Allocation Lint List (hot loops)

`format!`/`to_string` (→ `write!` into reused buf) · `collect()` where iteration works · `clone()` for the borrow checker (restructure) · `Vec` growth sans capacity · `String` `+` concat · return `Vec` where `impl Iterator` serves · boxed errors on hot paths

## Rules of Thumb

- `clear()` keeps capacity — that's the whole reuse trick; add `shrink_to` policy for heavy-tailed sizes.
- Allocate only on the miss path; hot (repeat) path allocates zero.
- **`bumpalo` never runs `Drop`** — plain data only; resources stay outside.
- Bump `&'bump` refs for tight scopes; `u32` index-arenas for data that crosses APIs.
- 1M-element growth without capacity ≈ 20 reallocs + full copies each — "amortized O(1)" hides the traffic.
- Cross-thread free (alloc on A, drop on B) is the allocator-stress pattern — recycle buffers via return channel, or send handles.
- Counting-allocator asserts in tests = the size_of-assert of this doc; counts are noise-free CI gates.
- First write to fresh pages pays kernel zeroing — warm up or measure it deliberately.

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Fast-path malloc | ~10–30 ns (thread-cache pop) |
| Bump-arena alloc | ~2 ns (pointer increment), contiguous placement free |
| Large alloc | mmap + page faults on first touch (~µs scale) |
| Allocator swap on alloc-heavy threaded service | ~5–20%; ~0 after levers 1–4 |
| Worked example, lever 1 alone | 114M → 200 K allocs, 2.4× time |

## Benchmark Checklist

- [ ] dhat counts + bytes per stage; time as confirmation, counts as the CI gate
- [ ] RSS watched (`/usr/bin/time -l`) — reuse/pools trade speed for held memory
- [ ] Teardown measured (arena wins concentrate in `drop_in_place`)
- [ ] Allocator A/B at production thread counts with real free patterns
- [ ] Warmup past first-touch faults unless startup is the question

## Key References

- Nethercote, [perf-book "Heap Allocations"](https://nnethercote.github.io/perf-book/heap-allocations.html).
- [dhat](https://docs.rs/dhat) + [bumpalo](https://docs.rs/bumpalo) docs (the no-Drop section twice).
- mimalloc/jemalloc design docs — replace allocator folklore once.
