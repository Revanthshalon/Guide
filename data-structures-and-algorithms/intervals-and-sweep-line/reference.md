# Intervals & Sweep Line — Quick Reference

## At a Glance

**Converts a 2-D problem into a sequence of 1-D problems.** A line moves left to right, stopping only at interesting coordinates.

**Three parts:** sorted **events** · a **status structure** (what's active) · a **processing rule**.

**Invariant:** at sweep position x, the status structure exactly describes objects intersecting the line, and all events < x are processed.
**Obligations:** the event order must be **total including ties** · the status structure must answer the queries you need.

**First move for any interval problem: sort by start, one pass.** Naive is Θ(n²).

## The Simplest Case

Max overlap / min meeting rooms = a **difference array over events**:
```
+1 at each start, −1 at each end, sort, running maximum
```

## Complexity

| Problem | Naive | Sweep |
| --- | --- | --- |
| Merge intervals | Θ(n²) | **Θ(n log n)** |
| Max overlap | Θ(n²) | **Θ(n log n)** |
| Interval scheduling | Θ(2ⁿ) | **Θ(n log n)** greedy (earliest finish) |
| Segment intersections | Θ(n²) | Θ((n+k) log n) — **output-sensitive** |
| Rectangle union area | Θ(n²) | Θ(n log n) |
| Stabbing query, dynamic | Θ(n) | **Θ(log n + k)** interval tree |

Every Θ(n log n) here is **the sort**. Pre-sorted events ⇒ Θ(n).

## Status-Structure Ladder

| Problem | Structure |
| --- | --- |
| Max overlap | a **counter** |
| Merge | the **last output interval** |
| Segment intersection | **`BTreeSet` by y** (neighbours only) |
| Rectangle union area | **segment tree over y** |
| Skyline | max-heap of active heights |
| Stabbing, dynamic | **interval tree** (BST + subtree max) |

## Snippets

```rust
// Merge — note .max(), and <= vs < is the touching convention
intervals.sort_unstable_by_key(|iv| iv.start);
for iv in intervals {
    match out.last_mut() {
        Some(last) if iv.start <= last.end => last.end = last.end.max(iv.end),
        _ => out.push(iv),
    }
}

// Event sweep — derive the tie rule from Ord so it's explicit
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Kind { End, Start }              // End < Start ⇒ ends first ⇒ touching = non-overlapping
events.sort_unstable();
for (_x, kind) in events {
    match kind { Kind::Start => active += 1, Kind::End => active -= 1 }
    max_active = max_active.max(active);
}

// Coordinate compression
xs.sort_unstable(); xs.dedup();
let idx = |v| xs.partition_point(|&x| x < v);
```

## Choose This When

| Use | For |
| --- | --- |
| **Sort + one pass** | Merge, max overlap, scheduling — try first |
| Event sweep + counter | Max concurrency, min resources |
| Sweep + `BTreeSet` | Segment intersection |
| Sweep + segment tree | Rectangle union area |
| **Interval tree** | **Dynamic** set + stabbing queries |
| **Difference array** | Small integer coordinate range |
| Coordinate compression | Huge coordinates, few distinct |
| Naive Θ(n²) | Small n, or dense output |

## Rules of Thumb

- Make the tie rule part of the sort key; test with two touching intervals.
- Merging always uses `last.end.max(iv.end)` — nested intervals are the counterexample.
- Small integer coordinate range → difference array, not a sweep.
- Sweep line is **offline**. Interleaved updates+queries ⇒ interval tree.
- Bentley-Ottmann is **output-sensitive** — dense intersections make it worse than naive.
- Prefer half-open `[start, end)` to match Rust ranges and settle the tie question.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Undefined start/end tie order | Results vary by input order and between runs |
| `last.end = iv.end` (no `max`) | Nested intervals silently truncate coverage |
| Sweep on a 168-slot range | More code and slower than a difference array |
| Bentley-Ottmann on dense data | Slower than the 5-line all-pairs test |
| Re-running the sweep per query | Θ(n log n) per query instead of Θ(log n + k) |

## Key References

- Bentley & Ottmann (1979) — segment intersection sweep
- de Berg et al., *Computational Geometry* ch. 2 — the definitive sweep-line treatment
- Fortune (1987) — Voronoi diagrams as a sweep
