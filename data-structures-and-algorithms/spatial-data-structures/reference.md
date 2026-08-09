# Spatial Data Structures — Quick Reference

## At a Glance

**Make "near" cheap by making "far" skippable.** Every structure is a bounding-region hierarchy; a query prunes a node when the *best possible* result inside it can't beat the current best.

**The pruning rule (the whole performance story):** after searching the near side, search the far side **only if `distance_to_splitting_plane² < best_so_far²`.**

**kd-tree invariant:** at depth `d`, split on dimension `d mod k`; left subtree ≤ node ≤ right subtree along that axis.
**R-tree invariant:** each node stores its children's minimum bounding rectangle; sibling MBRs **may overlap**.

## The Number

2-D nearest neighbour, 10,000 queries (measured):

| n | Build | Linear | **kd-tree** | Speedup |
| --- | --- | --- | --- | --- |
| 10,000 | 301.83 µs | 125.32 ms | **1.87 ms** | **67×** |
| 1,000,000 | 36.30 ms | 12.65 s | **5.24 ms** | **2,414×** |

Linear grew 100× with n; kd-tree grew 2.8×. **Build amortizes after ~3 queries.**

## The Family

| Structure | Partitions | Handles | Best for |
| --- | --- | --- | --- |
| **kd-tree** | points (median) | points | Static, exact k-NN, **d ≤ ~10** |
| Quadtree/octree | space (quarters) | points, regions | Non-uniform density; easy updates |
| **R-tree** | objects (MBRs) | **extended shapes** | GIS, disk-backed (PostGIS) |
| **Grid / spatial hash** | space (uniform) | points | **Uniform density**, Θ(1) updates |
| BVH | objects | triangles | Ray tracing |
| **HNSW / IVF** | approximate | high-dim vectors | **d > ~20** |

## Complexity

| Operation | kd-tree | Quadtree | R-tree | Grid | Linear |
| --- | --- | --- | --- | --- | --- |
| Build | Θ(n log n) | Θ(n log n) | Θ(n log n) | **Θ(n)** | — |
| **NN** | **Θ(log n) avg**, Θ(n) worst | Θ(log n) avg | Θ(log n) avg | **Θ(1) avg** | Θ(n) |
| Insert | Θ(log n), **unbalances** | Θ(log n) | Θ(log n) | **Θ(1)** | Θ(1) |
| Delete | **hard — rebuild** | Θ(log n) | Θ(log n) | **Θ(1)** | Θ(1) |
| **High d** | **→ Θ(n)** | worse | degrades | useless | Θ(n) |

## Snippets

```rust
// Build: median split via select_nth_unstable — Θ(n) per level
if pts.len() <= 8 { return; }                       // leaf bucket cutover
pts.select_nth_unstable_by(mid, |a, b| a.x.total_cmp(&b.x));

// Search: near side FIRST (finds a good `best` early), then prune
let (near, far) = if diff < 0.0 { (left, right) } else { (right, left) };
nn(near, depth + 1, q, best);
if diff * diff < *best { nn(far, depth + 1, q, best); }   // ← THE test
```

Compare **squared** distances — exact for ordering, no `sqrt`.

## The Dimensionality Cliff

kd-trees: excellent to ~10 d, marginal to ~20, **useless beyond**. Embeddings are 128–1536 d ⇒ **exact k-NN is off the table**.

| Method | Trade |
| --- | --- |
| **HNSW** | Best recall/speed; high memory; current default |
| IVF | Lower memory; needs training |
| LSH | Simple, tunable, weaker recall |
| Product quantization | Big memory savings; pairs with IVF |

## Choose This When

| Use | For |
| --- | --- |
| **Linear scan** | n small, or < ~3 queries |
| **Grid / spatial hash** | Uniform density, frequent updates |
| **kd-tree** | Static points, exact k-NN, d ≤ ~10 |
| Quadtree | Non-uniform density + updates |
| **R-tree** | Extended shapes, disk-backed |
| Geohash / S2 / H3 | 1-D sortable keys — composes with `BTreeMap` |
| **HNSW** | d > ~20 |

## Rules of Thumb

- Descend the **near side first** — it makes pruning effective.
- Squared distances throughout; never mix squared and unsquared.
- Leaf bucket (~8 points) — the D&C base-case cutover.
- `select_nth_unstable` for medians, not a full sort.
- Grid Θ(1) assumes **bounded occupancy per cell** — clustered data breaks it.
- kd-trees don't rebalance on insert; frequent updates ⇒ grid or quadtree.
- Keep a brute-force oracle in tests.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| kd-tree at 768 dimensions | Slower than a linear scan |
| Wrong pruning comparison | Wrong answers (over-prune) or full scan (under-prune) |
| Distance to *node* not *plane* | Prunes valid candidates |
| Mixed squared/unsquared | Plausible near-misses |
| Incremental inserts into a kd-tree | Degrades toward Θ(n) |
| Uniform grid on clustered data | Θ(1) evaporates; one cell holds thousands |

## Key References

- Bentley (1975) — kd-trees
- Guttman (1984) — R-trees · Beckmann et al. (1990) — R\*-trees
- Malkov & Yashunin (2016) — HNSW
