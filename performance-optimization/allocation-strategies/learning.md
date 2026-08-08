# Allocation Strategies — Learning Notes

## The Hardware Mechanism

`Box::new` is not a hardware operation — it's a call into a small database. A modern allocator (macOS libmalloc, jemalloc, mimalloc) maintains **size classes** (separate free lists for 16 B, 32 B, 48 B… blocks), **per-thread caches** (so the fast path needs no lock), and **spans/slabs** obtained from the OS via `mmap` in bulk. The fast path — pop a block off the thread-local free list for your size class — costs **~10–30 ns**. That sounds cheap. The damage is everywhere else:

- **The slow paths are cliffs.** Thread cache empty → refill from a shared structure (synchronization); size class exhausted → carve a new span; large allocation (≳ tens of KB) → straight to `mmap` and back to `munmap` on free (syscalls, ~µs); freshly mapped pages → **page faults on first touch** (the kernel zeroes and wires each 4 KB page as you write it — often the dominant cost of "allocating" a big buffer).
- **Placement is the allocator's choice, not yours.** Consecutive allocations land wherever free blocks happen to be — the heap-scatter that defeats prefetching in the [cache-locality doc](../cache-locality/learning.md) *is* allocator placement. Every `Box` you create is a future cache miss you scheduled.
- **Allocator metadata competes for your cache.** Free-list nodes, size-class headers, thread-cache structures — the allocator's own working set evicts yours. Alloc-heavy loops show a signature smear of allocator frames and elevated misses.
- **Frees aren't free.** `Drop` walks structures (a `Vec<String>` frees N+1 blocks), returns blocks to lists, occasionally coalesces or returns spans. Deep-tree teardown at scope end is real time — visible as a mysterious wide `drop_in_place` in flamegraphs.
- **Under threads, the allocator is a shared service.** Cross-thread frees (allocate on A, drop on B — the standard message-passing pattern) hit remote-free paths; a hot shared allocator becomes a [contention point](../false-sharing/learning.md) that per-thread caches only partly hide. Allocator choice matters most here.

The Rust-specific frame: no GC means allocation cost is *explicit and local* — it happens exactly where the code says (`String::from`, `collect()`, `clone()`, `format!`, `Vec` growth) — which is why it's both the most common performance finding in idiomatic Rust ([the profiling doc](../profiling-and-measurement/learning.md)'s dhat note) and the most mechanically fixable: every allocation has an address in your source.

## Mental Model

**Total cost = allocation frequency × (allocator work + placement damage). You have four levers, in order of preference: allocate less, allocate once, allocate together, allocate elsewhere.**

1. **Allocate less — the same buffer, again.** Most hot-loop allocations produce a value that dies before the next iteration: the `String` per line, the `Vec` per record. The fix is reuse: hoist the buffer, `clear()` per iteration (`clear` keeps capacity — that's the whole trick), or don't own at all — borrow slices of the input (`&str` views, zero-copy parsing). Frequency drops from N to 1; the allocator leaves your profile.
2. **Allocate once — capacity up front.** A `Vec` growing to 1M elements through doubling reallocates ~20 times, *copying everything each time* — amortized-O(1) hides real constants and peak-2× memory traffic. `with_capacity(n)` when n is knowable (often it is: `collect` with exact-size iterators does this automatically; `collect` from a filter can't) turns 20 allocations + 20 copies into 1 + 0.
3. **Allocate together — arenas group by lifetime.** When many objects share one death-time (everything for this request/frame/compile-unit), a **bump arena** (`bumpalo`) replaces the allocator wholesale: allocation = pointer increment (~2 ns, contiguous placement — the locality is a bonus), deallocation = reset the pointer, freeing *everything at once*. Lifetime-grouped allocation is the deepest fix because it's *shape-matched*: the arena encodes "these die together" that the general allocator can't know. The [event-sourcing per-request scope, game frames, parser ASTs] — phase-shaped work is everywhere.
4. **Allocate elsewhere — stack and inline.** `SmallVec<[T; N]>`/`ArrayVec`/`CompactStr` keep small payloads inline (no heap at all, until N is exceeded); plain arrays and generics keep sizes static. Wins when the distribution really is mostly-small — the inline branch and fatter type cost something, so this is a *measured* move, not a default.
5. **Meta-lever: choose the allocator.** One line (`#[global_allocator]`) swaps in jemalloc/mimalloc — typically 5–20% whole-program for alloc-heavy multithreaded services (better thread caching, cheaper remote frees), ~nothing for programs that took levers 1–4 seriously. It's the cheapest experiment in this doc and the least fundamental: a faster allocator is still scattered placement and per-object bookkeeping.

Where the model stops: allocation that isn't in the profile isn't a problem (dhat first — this doc is subordinate to [the funnel](../profiling-and-measurement/learning.md) like everything else); and long-lived, individually-dropped, size-varied objects are what the general allocator is *for* — arenas and pools mis-fit that shape (see When It Hurts).

## Worked Example

The log-analysis CLI from [the profiling doc](../profiling-and-measurement/learning.md), allocation story told fully this time. Baseline: 2 GB file, 38M lines.

**Stage 0 — idiomatic first draft.**

```rust
for line in reader.lines() {                  // String per line          38M allocs
    let fields: Vec<&str> = line?.split(',').collect();   // Vec per line  38M allocs
    let key = format!("{}-{}", fields[0], fields[2]);     // String per line 38M allocs
    *counts.entry(key).or_insert(0) += 1;
}
```

`dhat`: **114M allocations, 6.1 GB cumulative**, allocator frames ≈ 30% of the flamegraph; wide `drop_in_place` at loop bottom. Time: **9.8 s**.

**Stage 1 — reuse and borrow (lever 1).** Read into a reused `String` buffer (`read_line(&mut buf)` after `buf.clear()`); split lazily without collecting (`split(',').nth()` or iterate); build the key into a reused `String` with `write!`:

```rust
let mut buf = String::new(); let mut key = String::new();
while reader.read_line({ buf.clear(); &mut buf })? > 0 {
    let mut f = buf.trim_end().split(',');
    let (a, c) = (f.next().unwrap(), f.nth(1).unwrap());
    key.clear(); write!(key, "{a}-{c}")?;
    // entry API still needs an owned key on miss only:
    if let Some(v) = counts.get_mut(key.as_str()) { *v += 1 }
    else { counts.insert(key.clone(), 1); }
}
```

`dhat`: 114M → **~200 K allocations** (one per *unique* key, not per line). Time: **4.1 s**. Note the pattern in the map update: *allocate only on the miss path* — the hot path (repeat keys) allocates zero.

**Stage 2 — capacity and interning (levers 2 + 3).** `HashMap::with_capacity(expected_keys)` kills rehash-and-rehash-again; keys into a `lasso`-style interner backed by one growing arena instead of 200 K individual `String`s — unique-key cost drops to a `u32` + arena bytes. `dhat`: **~40 allocations total**. Time: **3.6 s** — diminishing returns on schedule (the allocator has left the profile; remaining time is parsing and hashing — a different doc's problem).

**Stage 3 — the arena variant, for shape.** If the loop instead built a *per-batch structure* (parse 10 K records → process → discard), the idiomatic form is `bumpalo`: allocate every record/`&str` into the bump, process the batch, `arena.reset()` — thousands of frees become one pointer move, and the batch is contiguous for the processing sweep. Same levers, phase-shaped.

The general lesson in numbers: **stage 1 (reuse/borrow) bought 2.4× and removed 99.8% of allocations — before any exotic machinery.** Arena/interner polish bought the last 12%. The order matters: cheap levers first.

## Applying It

- **Diagnose with `dhat`** (counts, bytes, call sites — the ranked "who allocates" list) or heaptrack on Linux; in flamegraphs look for `malloc`/`free`/`drop_in_place` width. **Gate with a counting allocator in tests**: a 20-line `GlobalAlloc` wrapper (or the `allocation-counter` crate) makes "this function performs ≤ K allocations" a unit test — the layout-assert of this doc, and the only way reuse discipline survives refactors.
- **The accidental-allocation lint list** — each of these in a hot loop is a finding: `format!`/`to_string()` (use `write!` into a reused buffer), `collect()` where iteration would do, `clone()` to appease the borrow checker (restructure instead), `Vec` growth without `with_capacity`, `String` concatenation with `+`, returning `Vec<T>` where `impl Iterator` serves, `Box<dyn Error>` construction on hot error paths.
- **Reuse idioms:** `clear()` keeps capacity (`Vec`, `String`, `HashMap` all do); thread-local scratch buffers (`thread_local!` + `RefCell`) for reuse across call boundaries; the miss-path-only allocation pattern from the worked example; `mem::take`/`replace` for buffer swaps in pipelines.
- **Arenas:** `bumpalo` (fast, allows `&'bump` references, **does not run `Drop`** — see When It Hurts), `typed-arena` (single-type, runs Drop), or an index-arena `Vec<T>` (the [cache doc](../cache-locality/learning.md)'s move — ids instead of references, no lifetime plumbing). Scope one arena per phase (request/frame/batch); reset, don't rebuild.
- **Pools** for expensive-to-*construct* objects (large buffers, connections): grab/return with capacity intact (`object-pool`-style crates, or a `Vec<Buffer>` behind a mutex — measure the lock vs. the alloc). Pools shine where arenas can't: objects that outlive phases or carry non-memory resources.
- **Inline/small types:** `SmallVec`, `ArrayVec`, `CompactStr`/`SmartString`, `Cow<'_, str>` for maybe-owned — each trades a branch and a fatter type for heap avoidance; adopt on measurement at *your* size distribution, and remember [memory-layout](../memory-layout/learning.md): the inline capacity rides in every instance, hot loop or not.
- **Allocator swap:** `tikv-jemallocator` or `mimalloc` via `#[global_allocator]` — try it early (one line, reversible); keep it if the service is alloc-heavy and threaded; don't let it substitute for levers 1–4. (macOS note: system malloc is decent; the swap moves less here than on Linux server workloads — measure, per the funnel.)
- **Message-passing hygiene:** allocate-on-A-free-on-B is the pattern that stresses allocators most; where it's hot, prefer sending indices/handles into shared arenas, or recycle buffers back to the producer (a return channel of empty `Vec`s is crude and extremely effective).

## When It Hurts

- **`bumpalo` doesn't run `Drop` — by design.** Memory is reclaimed on reset; *destructors never execute*. Bump-allocate a `File`, `MutexGuard`, or anything RAII and the resource leaks silently. Rule: arenas hold **plain data** (`Copy` types, slices, arena-internal references); resources stay outside. `typed-arena` runs Drop at arena death if you need both.
- **Arena lifetimes infect signatures.** `&'bump` threads through every function touching arena data — refactoring cost that index-arenas avoid (indices are `'static`). Choose bump-references for tight scopes, indices for structures that flow through APIs.
- **Reuse hides growth.** A reused buffer high-watermarks at the largest item ever seen and holds it forever; a pool of 4 MB buffers × 200 idle connections is 800 MB of RSS doing nothing. Add shrink policies (`shrink_to` on outliers, pool caps) where item sizes are heavy-tailed.
- **Pools + threads = the contention you were avoiding.** A mutexed pool hotter than the allocator's thread-cache path is a net loss — the allocator people spent decades on exactly this. Benchmark pool-vs-allocator under real concurrency before shipping the pool.
- **`SmallVec` mis-sized is pure tax:** inline capacity chosen too big fattens every instance ([layout](../memory-layout/learning.md) tax) and too small pays branch + heap anyway. It wants a measured size distribution, not a guess.
- **Premature buffer plumbing.** Threading `&mut` scratch buffers through ten call layers to avoid an allocation that dhat ranks 40th is complexity spent where the profile isn't. The lint list is for *hot* loops; elsewhere, `format!` is fine and the readable code wins.

## Benchmarking Methodology

- **Allocation counts are the primary metric, time the confirmation:** dhat before/after per stage (counts, bytes, sites), then criterion/hyperfine for the time delta. Counts are deterministic — immune to the noise that plagues time — which makes them ideal CI gates (counting-allocator asserts).
- **Watch RSS alongside speed** (`/usr/bin/time -l` on macOS, `-v` on Linux): reuse and pools trade allocator time for held memory; the high-watermark effect only shows in RSS, and "faster but 3× the memory" is a trade to make consciously.
- **Benchmark teardown too:** arena-vs-individual-drop differences concentrate at scope end (`drop_in_place` width); a benchmark that measures the loop but not the teardown misses half the arena's win.
- **Concurrency changes everything:** allocator A/B (system vs jemalloc vs mimalloc) *must* run at production thread counts with the real cross-thread-free pattern — single-threaded results are near-meaningless for this comparison.
- **First-touch page faults distort first iterations:** big fresh buffers pay the kernel-zeroing cost on first write — warm up past it, or measure it deliberately if startup is the concern.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Name the five ways allocation costs beyond the ~20 ns fast path, and rank which two dominate in (a) a single-threaded parser, (b) a threaded message-passing service.
2. Why does `clear()` beat re-creating the `Vec`, mechanically? What does it preserve and what invisible cost does it introduce over time?
3. The four levers in order — and for each, the workload shape that makes it the *right first* move.
4. Why is a bump arena's free "impossible to beat," and what language feature does `bumpalo` sacrifice to get it? Construct the bug that sacrifice invites.
5. A `Vec` grows from empty to 1M elements: how many reallocations, and what total copy traffic vs `with_capacity`? Why does "amortized O(1)" mislead here?
6. Your miss-path-only map insert still shows N allocations in dhat. Three hypotheses, in testing order.
7. When does an object pool lose to the general allocator, and why is that outcome common?

Measurement exercises:

- Reproduce the worked example's staging on any parser you own: dhat baseline → lever 1 (reuse/borrow) → lever 2 (capacity) → measure counts and time at each stage. Verify the doc's claim that lever 1 dominates.
- Write the counting-allocator test harness (wrap `System`, `AtomicUsize` counters) and pin your hot function's allocation count in a unit test. Break it with a stray `to_string()` and watch CI catch what review wouldn't.
- Allocator A/B at thread counts 1/4/16 on an alloc-heavy benchmark (or the stage-0 parser): system vs mimalloc. Plot the scaling curves — the divergence *is* the shared-service story of the mechanism section.

## Open Questions

- macOS libmalloc vs mimalloc/jemalloc on M-series for a threaded Rust service: real numbers at production-like cross-thread-free rates (the Linux folklore may not transfer).
- `bumpalo::collections` (`Vec<'bump>`, `String<'bump>`) ergonomics at scale — where does the lifetime plumbing actually bite in a mid-sized parser, and does an index-arena rewrite read better?
- Size-class internals of the current macOS allocator: where are the boundaries, and how much does the [layout doc](../memory-layout/learning.md)'s 40-byte struct actually occupy — measure via allocation counters + RSS deltas.
- The `allocation-counter` crate vs hand-rolled counting allocator: overhead, thread-attribution, CI ergonomics.
- io_uring-style buffer rings and registered buffers ([async & I/O](../async-and-io/learning.md)) as the kernel-boundary version of pooling — how does the discipline compose?

## References

- Nicholas Nethercote, [The Rust Performance Book — "Heap Allocations"](https://nnethercote.github.io/perf-book/heap-allocations.html) — the canonical lint list and crate tour; this doc's Applying-It section is its mechanism-first expansion.
- [dhat crate docs](https://docs.rs/dhat) — the measurement half; the profiling doc's dhat note operationalized.
- [bumpalo docs](https://docs.rs/bumpalo) — read the "This crate does not run Drop" section twice; it's the sharpest edge in this topic.
- mimalloc (Microsoft) and jemalloc papers/READMEs — how modern allocators actually structure size classes, thread caches, and remote frees; skim once to replace folklore.
- Related topics in this repo: [Profiling & Measurement](../profiling-and-measurement/learning.md) (dhat as the routing signal), [Cache Locality](../cache-locality/learning.md) (placement damage; index arenas), [Memory Layout](../memory-layout/learning.md) (allocator size classes quantize your struct sizes), [Data-Oriented Design](../data-oriented-design/learning.md) (arenas and handles as architecture), [Batching & Amortization](../batching-and-amortization/learning.md) (the phase-shaped work arenas want).
