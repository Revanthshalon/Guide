# Computational Geometry — Learning Notes

## Mental Model

**Almost every 2-D geometry algorithm is built from one primitive: the orientation test.** Given three points, is `c` to the left of, to the right of, or on the directed line `a → b`?

```
orient(a, b, c) = (b.x − a.x)(c.y − a.y) − (b.y − a.y)(c.x − a.x)
                  > 0 counter-clockwise · < 0 clockwise · = 0 collinear
```

That's a 2×2 determinant — the signed area of the triangle, doubled. From it you get: convex hull (keep turning the same way), segment intersection (endpoints on opposite sides, both ways), point-in-polygon, and the comparator that sorts points by angle. **Learn this one predicate and most of the field follows.**

And then the thing that makes computational geometry genuinely different from the rest of this category: **the hard part is not the algorithm, it's that floating point makes the predicate unreliable.** Measured on this machine, over a 256×256 lattice of points spaced one ULP apart near 0.5:

| Test | Result |
| --- | --- |
| `f64` orientation sign vs exact integer arithmetic | **39.9% disagree** (26,148 / 65,536) |
| Three cyclic permutations of the *same* orientation agreeing | **40.2% disagree with each other** |

The second row is the one that matters. `orient(p,q,r)`, `orient(q,r,p)` and `orient(r,p,q)` are the same determinant and must be identical — yet in `f64` they disagree 40% of the time at this scale. **The predicate isn't self-consistent**, so an algorithm that reasons from it can reach contradictory conclusions: a convex-hull routine can loop forever, produce a non-convex "hull", or crash on an index that "can't" happen.

That's why every serious geometry library uses **exact predicates**. It is not fastidiousness — it's that the algorithms' correctness proofs assume a consistent predicate, and floating point doesn't provide one.

## The Invariant

> `orient(a,b,c)` is the sign of the determinant `|b−a, c−a|`. It is **antisymmetric** (swapping two points flips the sign) and **invariant under cyclic permutation** (`orient(a,b,c) = orient(b,c,a) = orient(c,a,b)`).

Both properties are what algorithms rely on, and both are what floating point breaks. Convex hull's correctness argument is "every consecutive triple turns the same way"; if the predicate can report different answers for the same three points depending on argument order, the argument collapses.

The **exactness requirement** is more precise than "use more bits":

> The determinant needs the *sign*, not the value. With coordinates as b-bit integers, the determinant needs about **2b + 2 bits** to be computed exactly.

So 32-bit integer coordinates need 64-bit (or 128-bit) intermediate arithmetic — which Rust gives you directly with `i64`/`i128`. **If your coordinates are integers, exactness is free**: just widen the intermediate type. The problem only becomes hard when coordinates are genuinely real-valued.

## Mechanics

### The primitive, done right

```rust
// Integer coordinates: exact, and the only cost is a wider intermediate.
fn orient(a: (i64, i64), b: (i64, i64), c: (i64, i64)) -> i32 {
    let v = (b.0 - a.0) as i128 * (c.1 - a.1) as i128
          - (b.1 - a.1) as i128 * (c.0 - a.0) as i128;
    v.signum() as i32
}
```

For floating-point input, the production approach is **adaptive precision** (Shewchuk's predicates): compute with floats first and, if the result is within a provable error bound of zero, fall back to exact arithmetic. Since near-degenerate cases are rare in most data, you pay the fast path almost always and the exact path only when it matters. Rust: the `robust` crate.

### Convex hull — Andrew's monotone chain

```rust
pts.sort_unstable_by(|p, q| p.0.cmp(&q.0).then(p.1.cmp(&q.1)));
pts.dedup();
let mut hull: Vec<Point> = Vec::with_capacity(2 * pts.len());
for &p in pts.iter().chain(pts.iter().rev()) {          // lower hull, then upper
    while hull.len() >= 2 + lower_len
        && orient(hull[hull.len()-2], hull[hull.len()-1], p) <= 0 {
        hull.pop();                                     // ← monotonic stack!
    }
    hull.push(p);
}
```

Θ(n log n), dominated by the sort. Note the structure: **it's a [monotonic stack](../monotonic-stack-and-queue/learning.md)** — pop while the turn is wrong, and each point is pushed once and popped at most once, giving Θ(n) after sorting. The same amortization argument as everywhere else in Stage 6.

The `<= 0` versus `< 0` choice decides whether collinear points stay on the hull. Both are legitimate; picking one deliberately (and testing with three collinear points) is the difference between a correct implementation and a subtly wrong one.

### The core algorithms

| Problem | Approach | Cost |
| --- | --- | --- |
| **Convex hull** | Monotone chain (sort + monotonic stack) | Θ(n log n) |
| Convex hull, output-sensitive | Chan's algorithm | Θ(n log h), h = hull size |
| **Segment intersection** (all k) | [Sweep line](../intervals-and-sweep-line/learning.md), Bentley-Ottmann | Θ((n+k) log n) |
| Point in polygon | Ray casting (count crossings) or winding number | Θ(n) |
| **Closest pair** | [Divide & conquer](../divide-and-conquer/learning.md) + strip | Θ(n log n) |
| Delaunay triangulation | Incremental / divide & conquer | Θ(n log n) |
| Voronoi diagram | Fortune's sweep (dual of Delaunay) | Θ(n log n) |
| Polygon area | Shoelace formula (sum of orientations) | Θ(n) |
| Line-segment intersection point | Parametric solve | Θ(1) |
| **Nearest neighbour** | [kd-tree](../spatial-data-structures/learning.md) | Θ(log n) avg |

**Point-in-polygon** deserves a note: ray casting counts crossings of a ray from the point, and the entire difficulty is degenerate cases — the ray passing exactly through a vertex, or along an edge. The standard fix is a *half-open* rule (count an edge only if it strictly straddles the ray's y-coordinate), which is the same "make the tie rule part of the specification" discipline as [sweep line](../intervals-and-sweep-line/learning.md).

### Degeneracies — the second hard part

Beyond precision, geometry is full of special cases that the clean algorithm ignores:

| Degeneracy | Breaks |
| --- | --- |
| Three collinear points | Hull (are they on it?), triangulation |
| Duplicate points | Almost everything — dedup first |
| Vertical segments | Sweep-line comparators (infinite slope) |
| Segments sharing an endpoint | Intersection counting (is it an intersection?) |
| Overlapping collinear segments | Bentley-Ottmann's assumption of point intersections |
| Point exactly on an edge | In/out tests |

**Symbolic perturbation** ("simulation of simplicity") is the systematic answer: pretend every point is perturbed by an infinitesimal ε in a consistent way, so no degeneracy occurs. It removes all the special cases at the cost of making the output slightly arbitrary in degenerate configurations. In practice, most code handles the common degeneracies explicitly and documents which convention it chose.

## Complexity

| Problem | Time | Space | Notes |
| --- | --- | --- | --- |
| Orientation test | Θ(1) | Θ(1) | ~4 mults; exact needs 2b+2 bits |
| Convex hull (monotone chain) | **Θ(n log n)** | Θ(n) | The sort dominates |
| Convex hull (Chan's) | Θ(n log h) | Θ(n) | Output-sensitive |
| Closest pair | **Θ(n log n)** | Θ(n) | D&C with a Θ(n) strip check |
| Segment intersection (naive) | Θ(n²) | Θ(1) | **Often the right answer** |
| Segment intersection (sweep) | Θ((n+k) log n) | Θ(n) | Degrades to Θ(n² log n) when k is large |
| Delaunay / Voronoi | Θ(n log n) | Θ(n) | Θ(n) faces for n sites in 2-D |
| Point in polygon | Θ(n) | Θ(1) | Θ(log n) with preprocessing for convex |
| Convex hull in d dimensions | Θ(n^⌊d/2⌋) | — | **Exponential in dimension** |

**Where the table misleads.** The last row is the geometry version of the [dimensionality cliff](../spatial-data-structures/learning.md): a 2-D hull is Θ(n log n), a 3-D hull is Θ(n log n), and a 6-D hull is Θ(n³). Geometry algorithms are low-dimensional tools.

And the segment-intersection rows are the recurring lesson: Bentley-Ottmann's Θ((n+k) log n) is **output-sensitive**, so with dense intersections it is *worse* than the naive Θ(n²) — while being an order of magnitude more code and far more sensitive to degeneracies. The naive test is frequently the correct engineering choice.

## Use Cases

- **GIS and mapping** — point-in-polygon for "which region contains this coordinate", polygon clipping for map tiles, hull for bounding regions. Backed by [R-tree](../spatial-data-structures/learning.md) indexes in PostGIS.
- **Computer graphics** — collision detection (convex hulls and GJK), visibility, triangulation for rendering, BVH construction.
- **CAD and manufacturing** — boolean operations on solids, toolpath generation, offsetting. This is where exact predicates are non-negotiable, because a topologically inconsistent result is an unmanufacturable part.
- **Robotics** — configuration-space obstacles, path planning around polygonal obstacles, Voronoi diagrams for maximum-clearance paths.
- **Machine learning** — convex hull for outlier detection, Delaunay for interpolation and mesh generation, α-shapes for boundary reconstruction.
- **Games** — navmesh generation (triangulation), line-of-sight, physics broad-phase.
- **Data visualization** — Voronoi for nearest-label lookup and for treemap-like layouts, hull for cluster boundaries.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Integer coordinates + `i128` intermediates** | You control the input — **exactness is free** |
| `robust` crate (adaptive predicates) | Real-valued coordinates and correctness matters |
| Naive `f64` predicates | Coordinates are well-separated relative to their magnitude, and you've thought about it |
| **Monotone chain** | Convex hull — simplest correct algorithm |
| **Naive Θ(n²) intersection** | n is small (a few thousand) or intersections are dense |
| Bentley-Ottmann | Sparse intersections, large n, and you'll handle degeneracies |
| A geometry library (`geo`, `spade`, CGAL) | Anything beyond hulls and intersections |
| [kd-tree / R-tree](../spatial-data-structures/learning.md) | Proximity queries rather than construction |

## Pitfalls in Depth

### Pitfall: Floating-point predicates

- **What goes wrong:** The orientation test is computed in `f64` and returns inconsistent answers. Measured over a ULP-scale lattice, the `f64` sign **disagreed with exact arithmetic in 39.9% of cases**, and — worse — the three cyclic permutations of the *same* orientation disagreed with **each other 40.2% of the time**. Downstream, a convex-hull loop that pops "while the turn is wrong" can pop past the start of the array (index panic), never terminate, or emit a non-convex polygon.
- **Why it happens (the mechanism):** The determinant is a difference of two products of differences. When the points are nearly collinear, those products are nearly equal, and the subtraction is **catastrophic cancellation** — the result's leading bits cancel, leaving only rounding error, whose sign is essentially arbitrary. The algorithm's correctness proof assumes a *consistent, antisymmetric* predicate; it says nothing sensible when the predicate contradicts itself.
- **How to handle it in production, and why that works:** If coordinates are (or can be) integers, compute the determinant in a wider integer type — `i128` for `i64` coordinates — which is **exact and costs one wider multiply**. For real-valued input use adaptive predicates (`robust` crate, Shewchuk's method): evaluate in floating point with a computed error bound, and fall back to exact arithmetic only when the value is within that bound of zero. Degenerate cases are rare, so the fast path dominates.
- **Trade-offs of the fix:** Integer coordinates mean snapping input to a grid, which is a modelling decision with its own consequences (two distinct points can collapse). Adaptive predicates add a dependency and are slower in the rare exact-path case. Neither is as fast as a naive `f64` multiply — but the naive version is not an implementation of the algorithm you think you're running.

### Pitfall: Ignoring degeneracies

- **What goes wrong:** The algorithm is written for "general position" — no three points collinear, no duplicates, no vertical segments — and real data has all of them. A convex hull includes or excludes collinear points inconsistently; a sweep line's comparator can't order two segments that share an endpoint; a point-in-polygon test double-counts a ray passing exactly through a vertex, reporting "outside" for an interior point.
- **Why it happens (the mechanism):** Textbook presentations assume general position to keep the exposition clean, and random test data almost never produces degeneracies. Real data is full of them: architectural drawings have axis-aligned edges everywhere, GIS data has shared boundaries by construction, and any snapped or gridded input has collinear points in abundance.
- **How to handle it in production, and why that works:** Decide each convention explicitly and encode it in the comparison — `<= 0` versus `< 0` for collinear hull points, a **half-open** rule for ray-crossing (count an edge only if it strictly straddles the ray's y), a tie-break by index in the sweep-line order. Then write the degenerate tests: three collinear points, duplicate points, a vertical segment, two segments sharing an endpoint. Symbolic perturbation removes the cases systematically if you need that rigour.
- **Trade-offs of the fix:** Each convention makes the output "correct" in a specific sense that may not match a caller's expectation, so it must be documented. Symbolic perturbation makes the code uniform but produces arbitrary-looking results in degenerate configurations (a zero-area triangle gets an orientation), which can be worse than an explicit special case for a user-facing tool.

### Pitfall: Reaching for a sweep when Θ(n²) is better

- **What goes wrong:** Bentley-Ottmann is implemented for segment intersection because it's Θ((n+k) log n) against the naive Θ(n²). For a dense arrangement where k is Θ(n²), it's actually **slower** — Θ(n² log n) — and it's an order of magnitude more code, with a status structure whose ordering can be corrupted by exactly the precision problems above.
- **Why it happens (the mechanism):** The bound is **output-sensitive**: excellent when intersections are sparse, worse than naive when they're dense. The comparison depends on `k`, a property of the data, not on `n` alone. And the sweep's `BTreeSet` ordered by y is precisely where an inconsistent predicate does the most damage — a corrupted order silently produces wrong output rather than one wrong pair.
- **How to handle it in production, and why that works:** Estimate `k`. Sparse (map overlays, circuit routing, road networks) → sweep. Dense, or n in the low thousands → the naive double loop, which is five lines, trivially correct, and vectorizes well. Measure before assuming the asymptotically better algorithm is better.
- **Trade-offs of the fix:** The naive test doesn't scale past a few thousand segments. If you genuinely need both scale and robustness, use a library (CGAL, `spade`) rather than implementing Bentley-Ottmann with exact predicates yourself.

### Pitfall: Using geometry algorithms in high dimensions

- **What goes wrong:** A convex hull is computed over 50-dimensional feature vectors, or a Delaunay triangulation over 20-dimensional points. The algorithm doesn't finish: a d-dimensional hull has Θ(n^⌊d/2⌋) faces, so at d = 10 with n = 1,000 that's 10¹⁵ faces before any algorithm inefficiency.
- **Why it happens (the mechanism):** The complexity is exponential in the *dimension*, and the bound is on the **output size** — there genuinely are that many faces, so no algorithm can avoid it. This is the same cliff as [spatial structures](../spatial-data-structures/learning.md)' kd-trees degrading past ~20 dimensions, and it has the same root: volume concentrates in high dimensions and geometric intuition stops transferring.
- **How to handle it in production, and why that works:** Reduce dimension first (PCA, random projection) if the geometry is genuinely needed, or reformulate the problem — "outlier detection" rarely requires an actual hull, and "interpolation" rarely requires a true Delaunay triangulation. For proximity in high dimensions, use approximate nearest neighbour ([spatial structures](../spatial-data-structures/learning.md)).
- **Trade-offs of the fix:** Dimension reduction loses information and its validity is problem-dependent. Reformulating means giving up the exact geometric guarantee, which for some applications (collision, manufacturing) isn't acceptable — but those applications are 2-D and 3-D anyway.
