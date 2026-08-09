# Computational Geometry — Quick Reference

## At a Glance

Almost everything in 2-D is built from **one primitive**:

```
orient(a,b,c) = (b.x−a.x)(c.y−a.y) − (b.y−a.y)(c.x−a.x)
                > 0 CCW · < 0 CW · = 0 collinear
```

**Invariant:** antisymmetric (swap two points ⇒ flip sign) and **invariant under cyclic permutation**. Algorithms' proofs depend on both.

## The Number

ULP-spaced lattice near 0.5, `f64` vs exact integer arithmetic (measured):

| Test | Result |
| --- | --- |
| `f64` orientation sign vs exact | **39.9% disagree** |
| Three cyclic permutations of the **same** orientation agreeing | **40.2% disagree with each other** |

**The predicate isn't self-consistent** ⇒ hull loops forever, panics, or emits a non-convex polygon.

## Exactness Is Often Free

> The determinant needs the **sign**, not the value. With b-bit integer coordinates it needs **2b+2 bits**.

`i64` coordinates ⇒ compute in `i128`. One wider multiply, exact.

```rust
fn orient(a:(i64,i64), b:(i64,i64), c:(i64,i64)) -> i32 {
    let v = (b.0-a.0) as i128 * (c.1-a.1) as i128
          - (b.1-a.1) as i128 * (c.0-a.0) as i128;
    v.signum() as i32
}
```

Real-valued input ⇒ **adaptive predicates** (`robust` crate): float fast path, exact fallback only near zero.

## Complexity

| Problem | Time | Notes |
| --- | --- | --- |
| Orientation | Θ(1) | exact needs 2b+2 bits |
| **Convex hull** (monotone chain) | **Θ(n log n)** | the sort dominates; it's a **monotonic stack** |
| Closest pair | Θ(n log n) | D&C + Θ(n) strip |
| Segment intersection (naive) | Θ(n²) | **often the right answer** |
| Segment intersection (sweep) | Θ((n+k) log n) | **output-sensitive** — worse than naive when dense |
| Delaunay / Voronoi | Θ(n log n) | duals of each other |
| Point in polygon | Θ(n) | ray casting; half-open rule for ties |
| **Hull in d dimensions** | **Θ(n^⌊d/2⌋)** | exponential in dimension |

## Convex Hull

```rust
pts.sort_unstable_by(|p,q| p.0.cmp(&q.0).then(p.1.cmp(&q.1)));
pts.dedup();
// lower hull then upper; pop while the turn is wrong (monotonic stack)
while hull.len() >= 2 + lower_len
   && orient(hull[hull.len()-2], hull[hull.len()-1], p) <= 0 { hull.pop(); }
hull.push(p);
```

`<= 0` vs `< 0` decides whether **collinear points stay on the hull**. Pick deliberately; test with three collinear points.

## Degeneracies to Test

| Case | Breaks |
| --- | --- |
| Three collinear points | Hull membership, triangulation |
| Duplicate points | Nearly everything — **dedup first** |
| Vertical segments | Sweep comparators (infinite slope) |
| Shared endpoints | Intersection counting |
| Overlapping collinear segments | Bentley-Ottmann's point-intersection assumption |
| Point exactly on an edge | In/out tests |

## Choose This When

| Use | For |
| --- | --- |
| **Integer coords + `i128`** | You control input — exactness is free |
| `robust` crate | Real-valued coords, correctness matters |
| **Monotone chain** | Convex hull |
| **Naive Θ(n²)** | n small, or intersections dense |
| Bentley-Ottmann | Sparse intersections, large n |
| `geo` / `spade` / CGAL | Anything beyond hulls and intersections |
| kd-tree / R-tree | Proximity queries, not construction |

## Rules of Thumb

- Never trust a bare `f64` orientation test near degeneracy.
- Integer coordinates make exactness a type choice, not an algorithm choice.
- Decide `<=` vs `<` for collinearity **explicitly**, and test it.
- Half-open rules settle ray-crossing and sweep ties.
- Sweep vs naive depends on **k**, a property of the data.
- Geometry is a low-dimensional tool — the hull bound is exponential in d.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| `f64` predicates | Non-convex "hull", infinite loop, index panic |
| Untested collinear points | Hull includes/excludes them inconsistently |
| Duplicate points | Comparator contradictions; degenerate turns |
| Ray through a vertex | Interior point reported outside |
| Bentley-Ottmann on dense data | Slower than the 5-line naive loop |
| Hull in 10 dimensions | Θ(n⁵) output — never finishes |

## Key References

- Shewchuk (1997) — adaptive exact predicates; the definitive treatment
- Kettner et al., "Classroom Examples of Robustness Problems in Geometric Computations"
- de Berg et al., *Computational Geometry* — the standard text
