# Intervals & Sweep Line — Learning Notes

## Mental Model

**Sweep line converts a 2-D problem into a sequence of 1-D problems.** Imagine a vertical line moving left to right across the plane. It only stops at *interesting* x-coordinates — the endpoints of intervals, the corners of rectangles, the vertices of segments — and at each stop you update a small structure describing the current cross-section. Between stops, nothing changes, so you never examine the continuum.

The pattern is always the same three parts:

1. **Events** — sort the interesting x-coordinates (Θ(n log n), and this dominates).
2. **A status structure** — what's currently "active" as the line passes.
3. **A processing rule** — what to do at each event.

Once you see it, a large family of problems collapses into one template: merging intervals, counting overlaps, finding segment intersections, computing the union area of rectangles, scheduling meeting rooms, and the closest-pair strip check from [divide & conquer](../divide-and-conquer/learning.md).

For the simplest and most common case — "how many intervals overlap at the busiest point?" — the status structure is just a counter, and the entire algorithm is a [difference array](../prefix-sums-and-difference-arrays/learning.md) over events:

```
+1 at each start, −1 at each end, sort, running maximum
```

That's Θ(n log n) for what looks like a Θ(n²) pairwise-overlap problem, and it's the version worth having in muscle memory.

**The first move for any interval problem is: sort by start (or by endpoint), then make one pass.** The naive alternative — compare every interval against every other — is Θ(n²), which is the same shape as the Θ(n²)→Θ(n) wins measured in [two pointers](../two-pointers-and-sliding-window/learning.md) (5,135× at n = 100,000) and [monotonic deque](../monotonic-stack-and-queue/learning.md) (591×).

## The Invariant

**Merging intervals (sorted by start):**

> After processing the first k intervals, `output` contains disjoint, sorted intervals whose union equals the union of those k. The last element of `output` is the only one that can still be extended — because everything is sorted by start, no later interval can reach back past it.

That "only the last can extend" property is what makes the merge a single pass with no lookback.

**Sweep line generally:**

> At sweep position x, the status structure exactly describes the set of objects intersecting the line at x, and all events with coordinate < x have been processed.

Two obligations:

- **Event ordering must be total and deterministic**, including ties. When a start and an end share a coordinate, which comes first *is* the semantics: process ends first and touching intervals `[1,2]` and `[2,3]` count as non-overlapping; process starts first and they overlap. Neither is universally right — but leaving it to an unstable sort makes it non-deterministic, which is always wrong.
- **The status structure must support the queries you need at each event.** A counter answers "how many active"; a `BTreeSet` ordered by y answers "who is my neighbour" (needed for segment intersection); a [segment tree](../range-query-structures/learning.md) over coordinates answers "total covered length" (needed for rectangle-union area).

## Mechanics

### Merging overlapping intervals

```rust
intervals.sort_unstable_by_key(|iv| iv.start);
let mut out: Vec<Interval> = Vec::new();
for iv in intervals {
    match out.last_mut() {
        Some(last) if iv.start <= last.end => last.end = last.end.max(iv.end),  // extend
        _ => out.push(iv),                                                       // disjoint
    }
}
```

`<=` versus `<` in the overlap test is the touching-intervals decision above. `last.end.max(iv.end)` — not `iv.end` — matters because the current interval may be entirely contained in the previous one.

### The event sweep

```rust
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Kind { End, Start }                       // End < Start ⇒ ends processed first on ties

let mut events: Vec<(i64, Kind)> = Vec::with_capacity(2 * n);
for iv in &intervals {
    events.push((iv.start, Kind::Start));
    events.push((iv.end,   Kind::End));
}
events.sort_unstable();                        // total order including the tie rule

let (mut active, mut max_active) = (0i32, 0i32);
for (_x, kind) in events {
    match kind { Kind::Start => active += 1, Kind::End => active -= 1 }
    max_active = max_active.max(active);
}
```

Deriving the tie rule from the enum's `Ord` (rather than a comparator) makes it explicit and impossible to get inconsistently. `max_active` is the minimum number of meeting rooms, the peak concurrency, the maximum overlap — all the same quantity.

### The status-structure ladder

The problem determines what the sweep must maintain:

| Problem | Status structure | Cost |
| --- | --- | --- |
| Max overlap / min rooms | a **counter** | Θ(n log n) (the sort) |
| Merge intervals | the **last output interval** | Θ(n log n) |
| Interval scheduling (max non-overlapping) | last chosen finish time — [greedy](../greedy-algorithms/learning.md), earliest finish | Θ(n log n) |
| Segment intersection | **`BTreeSet` ordered by y** (neighbours only) | Θ((n+k) log n) — Bentley-Ottmann |
| Rectangle union area | **segment tree over y**, covered length | Θ(n log n) |
| Skyline | max-heap of active heights | Θ(n log n) |
| Closest pair | y-sorted strip within δ | Θ(n log n) |
| Stabbing queries ("which intervals contain x?") | **interval tree** ([BST augmentation](../binary-search-trees/learning.md)) | Θ(log n + k) per query |

The Bentley-Ottmann insight is worth naming: **two segments can only intersect if they are ever adjacent in the y-order**, so you only test neighbours as they swap, not all pairs.

### Coordinate compression

When coordinates are huge but few, map them to `0..m` first:

```rust
let mut xs: Vec<i64> = intervals.iter().flat_map(|iv| [iv.start, iv.end]).collect();
xs.sort_unstable(); xs.dedup();
let idx = |v: i64| xs.partition_point(|&x| x < v);      // Θ(log m)
```

This is the enabler for array- or segment-tree-based sweeps over coordinates in the billions — you only ever need the 2n distinct endpoints. It composes with [range query structures](../range-query-structures/learning.md) constantly.

### Offline vs online

Sweep line is **offline**: it needs all events up front to sort them. If intervals arrive dynamically and queries interleave, you need an **interval tree** (a BST augmented with subtree-max endpoint, Θ(log n + k) stabbing queries) or a segment tree over compressed coordinates. Recognizing "can I batch this?" is the deciding question — offline is almost always simpler and faster when it applies.

## Complexity

| Problem | Naive | Sweep / sort |
| --- | --- | --- |
| Merge intervals | Θ(n²) | **Θ(n log n)** |
| Max overlap | Θ(n²) | **Θ(n log n)** |
| Interval scheduling (max count) | Θ(2ⁿ) | **Θ(n log n)** greedy |
| Segment intersections (k of them) | Θ(n²) | **Θ((n+k) log n)** |
| Rectangle union area | Θ(n²) | **Θ(n log n)** |
| Skyline | Θ(n²) | Θ(n log n) |
| Stabbing query, dynamic set | Θ(n) per query | **Θ(log n + k)** interval tree |

**Where the table misleads.** Every Θ(n log n) here is **the sort** — the sweep itself is Θ(n) or Θ(n log n) depending on the status structure. So if events arrive pre-sorted (timestamps from a log, a time-ordered stream), these become Θ(n). That's the same observation as [greedy](../greedy-algorithms/learning.md): the paradigm's cost is usually the sort, not the algorithm.

Bentley-Ottmann's Θ((n+k) log n) is **output-sensitive** — it depends on the number of intersections found, which can be Θ(n²) in the worst case. For a dense arrangement, the naive Θ(n²) all-pairs test is simpler and no worse.

## Use Cases

- **Calendar and scheduling** — meeting-room allocation (max overlap), free/busy computation (interval merge), conflict detection.
- **Resource capacity planning** — peak concurrent connections, licences in use, VMs required over a day; all are max-overlap.
- **Computational geometry** — segment intersection, rectangle union/intersection area, polygon clipping, Voronoi construction (Fortune's algorithm is a sweep).
- **Graphics** — the skyline problem, occlusion, scanline rasterization and polygon fill are literal sweep lines.
- **Genomics** — interval overlap over genomic ranges; BEDTools-style operations are interval merges and intersections at scale.
- **Networking** — IP range containment and overlap checking, firewall rule conflict detection.
- **Databases** — temporal joins, range-overlap predicates, and the interval-index structures that support them.
- **Time-series and monitoring** — computing periods where a condition held, merging alert windows, deduplicating overlapping incidents.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Sort + single pass** | Merge, max overlap, scheduling — **try this first** |
| **Event sweep with a counter** | Max concurrency, min resources |
| Sweep + `BTreeSet` | Segment intersection (neighbour queries) |
| Sweep + [segment tree](../range-query-structures/learning.md) | Rectangle union area, covered length |
| **Interval tree** | **Dynamic** set with stabbing queries |
| [Difference array](../prefix-sums-and-difference-arrays/learning.md) | Small integer coordinate range, bulk range updates |
| Coordinate compression + array | Huge coordinates, few distinct values |
| Naive Θ(n²) | n is small, or the output is dense anyway |

## Pitfalls in Depth

### Pitfall: Undefined tie-breaking between starts and ends

- **What goes wrong:** Two events share a coordinate — one interval ends exactly where another begins — and the sort doesn't specify which comes first. With `sort_unstable` the order depends on the input arrangement, so `[1,2]` and `[2,3]` sometimes count as overlapping and sometimes don't. Results change between runs, between input orderings, and after unrelated refactors.
- **Why it happens (the mechanism):** The tie *is* the semantics — whether touching intervals overlap is a domain decision (a meeting ending at 2pm doesn't conflict with one starting at 2pm; a closed genomic interval `[2,2]` does). Sorting only by coordinate leaves it to the sort's stability, and `sort_unstable` provides none ([sorting](../sorting/learning.md)).
- **How to handle it in production, and why that works:** Make the event type part of the sort key so the order is total. Deriving `Ord` on `enum Kind { End, Start }` puts ends first (the non-overlapping convention) and makes the choice visible in the type. Then write the test with two touching intervals — that single test pins the convention forever.
- **Trade-offs of the fix:** You must pick a convention, and the two are genuinely both used — half-open `[start, end)` intervals want ends-first, closed `[start, end]` intervals often want starts-first. Document which you're using; converting to half-open at the boundary is usually the cleanest resolution because it matches Rust's ranges.

### Pitfall: Not extending to the maximum end when merging

- **What goes wrong:** The merge writes `last.end = iv.end` instead of `last.end.max(iv.end)`. When one interval is entirely contained in the previous one — `[1, 10]` followed by `[2, 3]` — the merged interval is truncated to `[1, 3]`, silently dropping coverage. Sorted-by-start input makes containment common, and the bug only fires on nested intervals.
- **Why it happens (the mechanism):** After sorting by start, it's tempting to assume ends are also increasing. They aren't — sorting by start says nothing about ends, and a long interval followed by a short contained one is the counterexample.
- **How to handle it in production, and why that works:** Always `max` the ends. Then test with nested intervals explicitly (`[1,10]`, `[2,3]`) and with an interval that extends past two others. A property test against a brute-force "mark every unit and count" reference catches this immediately on random input.
- **Trade-offs of the fix:** None — it's the same line with a `max`. The value is in knowing the failure mode so the test gets written.

### Pitfall: Reaching for a sweep when the coordinate range is small

- **What goes wrong:** A sweep with sorting and an event structure is built for "count overlaps per hour over a week" — 168 possible coordinates. The Θ(n log n) sort and the event machinery are far more code and often slower than a 168-element [difference array](../prefix-sums-and-difference-arrays/learning.md): `diff[start] += 1; diff[end] -= 1`, then one prefix-sum pass.
- **Why it happens (the mechanism):** Sweep line is the general technique, so it's what you recall. But when coordinates are small integers, direct indexing removes the sort entirely — Θ(n + range) instead of Θ(n log n), with no comparator and no tie-breaking question.
- **How to handle it in production, and why that works:** Check the coordinate range first. Small and integral → difference array. Huge but few distinct values → coordinate-compress to `0..2n`, then difference array or segment tree. Genuinely continuous or needing neighbour queries → full sweep.
- **Trade-offs of the fix:** A difference array over a large range wastes memory proportional to the range, so the check must be on the *range*, not just on "the coordinates look like integers". Coordinate compression adds a sort back, though only once and with no tie subtleties.

### Pitfall: Assuming Bentley-Ottmann always beats the naive test

- **What goes wrong:** Θ((n+k) log n) is treated as strictly better than Θ(n²), so the full sweep — with a y-ordered `BTreeSet`, event insertion for discovered intersections, and careful handling of degeneracies — is implemented for a dense arrangement where k is Θ(n²). It ends up slower than the 5-line all-pairs test, and it's an order of magnitude more code, with degenerate cases (vertical segments, three-way intersections, overlapping collinear segments) that are genuinely hard to get right.
- **Why it happens (the mechanism):** The bound is **output-sensitive**: it's excellent when intersections are sparse and degrades to Θ(n² log n) when they're dense — *worse* than naive by a log factor. The complexity comparison depends on a property of the data (k), not just of n.
- **How to handle it in production, and why that works:** Estimate k. Sparse intersections (map overlays, circuit routing) → Bentley-Ottmann. Dense, or n small (a few thousand) → all-pairs, which is trivially correct and vectorizes well. Where robustness matters more than speed, a geometry library with exact predicates is a better answer than either.
- **Trade-offs of the fix:** The all-pairs test doesn't scale past a few thousand segments. And floating-point robustness bites both approaches — the sweep worse, because an inconsistent orientation predicate can corrupt the y-order and produce arbitrarily wrong output rather than one wrong pair.

### Pitfall: Using a sweep for a dynamic problem

- **What goes wrong:** Intervals are inserted and removed over time and queries interleave, so the sweep is re-run from scratch on every query — Θ(n log n) per query rather than Θ(log n + k).
- **Why it happens (the mechanism):** Sweep line is fundamentally **offline**: it sorts all events up front, which requires knowing them all. Nothing about the technique supports incremental updates, so the only way to accommodate a change is to redo the sweep.
- **How to handle it in production, and why that works:** Use an **interval tree** — a BST augmented with the maximum endpoint in each subtree ([binary search trees](../binary-search-trees/learning.md)) — giving Θ(log n) insert/delete and Θ(log n + k) stabbing queries. If updates are batched rather than truly interleaved, keep the sweep and re-run per batch; offline is simpler when it applies.
- **Trade-offs of the fix:** An interval tree is a real data structure to implement and maintain (the augmentation must be fixed through every rotation), against a sweep's sort-and-scan. It's also slower per *bulk* operation — processing all intervals through a tree is worse than one sweep. The deciding question is whether queries interleave with updates.
