# Spatial Data Structures — Learning Notes

## Mental Model

**Spatial structures make "near" cheap by making "far" skippable.** A nearest-neighbour query over n points is Θ(n) by brute force; a spatial index prunes whole regions whose *closest possible* point is already worse than the best found, so most of the data is never examined.

Measured on this machine — nearest neighbour in 2-D, 10,000 queries:

| n | Build | Linear scan | **kd-tree** | Speedup |
| --- | --- | --- | --- | --- |
| 10,000 | 301.83 µs | 125.32 ms | **1.87 ms** | **67×** |
| 1,000,000 | 36.30 ms | 12.65 s | **5.24 ms** | **2,414×** |

Note the shape: the linear scan's time grew 100× with n (as Θ(n) demands) while the kd-tree's grew only 2.8× — that's Θ(log n) behaviour, and it's why the gap widens so dramatically. The 36 ms build amortizes after **three queries**.

The unifying idea across every structure here is a **bounding-region hierarchy**: each node covers a region, and a query prunes a node when the *best possible* result inside that region can't beat what you already have. The structures differ only in how they partition:

- **kd-tree** — split by one coordinate at a time, cycling through dimensions. Partitions *points*.
- **Quadtree / octree** — split space into 4 (or 8) equal children. Partitions *space*.
- **R-tree** — group nearby objects into (overlapping) bounding boxes. Partitions *objects*, and handles extended shapes rather than points.
- **Grid / spatial hash** — fixed-size cells; Θ(1) lookup when density is uniform.

And the fact that reframes the whole topic: **all of them degrade to a linear scan in high dimensions.** Above roughly 10–20 dimensions, the pruning stops working — the "curse of dimensionality" — and the entire field switches to *approximate* nearest neighbour (HNSW, IVF, LSH). Knowing where that cliff is prevents the most expensive mistake here.

## The Invariant

**kd-tree:**

> At depth `d`, the node splits on dimension `d mod k`. Every point in the left subtree has a coordinate ≤ the node's along that dimension; every point in the right subtree has ≥.

**The pruning rule** is where the performance lives, and it's the same for every structure in the family:

> When searching for the nearest neighbour, after recursing into the near side, **only search the far side if `distance_to_splitting_plane² < best_distance_so_far²`.**

That single test is what turns Θ(n) into Θ(log n) on average. It's a *lower bound* on anything in the far region — if even the closest possible point over there is worse than what you have, skip it entirely. Get the comparison direction wrong and you either return incorrect results (pruning too aggressively) or degrade to a full scan (never pruning).

Note the squared distances: **comparing squared distances avoids a square root per comparison** and is exact for ordering purposes, since `sqrt` is monotonic. That's free correctness-preserving speed.

**R-tree:**

> Every node stores the **minimum bounding rectangle** of its children, and a child's MBR is contained in its parent's. Unlike a kd-tree, sibling MBRs **may overlap** — which is what allows R-trees to store extended objects, and also what means a query may need to descend several branches.

## Mechanics

### kd-tree construction and search

```rust
// Build: recursively split at the median along the cycling dimension.
// select_nth_unstable gives Θ(n) partitioning per level → Θ(n log n) total.
fn build(pts: &mut [P], depth: usize) {
    if pts.len() <= 8 { return; }                     // leaf bucket — see the cutover note
    let mid = pts.len() / 2;
    if depth % 2 == 0 { pts.select_nth_unstable_by(mid, |a, b| a.x.total_cmp(&b.x)); }
    else               { pts.select_nth_unstable_by(mid, |a, b| a.y.total_cmp(&b.y)); }
    let (l, r) = pts.split_at_mut(mid);
    build(l, depth + 1);
    build(&mut r[1..], depth + 1);
}

// Search: descend the near side first, then prune the far side.
fn nn(pts: &[P], depth: usize, q: P, best: &mut f32) {
    // ... check this node ...
    let diff = if depth % 2 == 0 { q.x - p.x } else { q.y - p.y };
    let (near, far) = if diff < 0.0 { (left, right) } else { (right, left) };
    nn(near, depth + 1, q, best);
    if diff * diff < *best { nn(far, depth + 1, q, best); }    // ← THE pruning test
}
```

Two details that matter in practice. **Descending the near side first** is what makes pruning effective — it finds a good `best` early, so the far-side test usually fails. And the **leaf bucket** (`len <= 8`) is the [divide & conquer](../divide-and-conquer/learning.md) base-case cutover: below it, a linear scan of 8 contiguous points beats further recursion.

`select_nth_unstable` is the right tool for median-finding here — Θ(n) expected rather than Θ(n log n) for a full sort ([selection & order statistics](../selection-and-order-statistics/learning.md), measured 10.7× faster than sorting).

### The structure family

| Structure | Partitions | Handles | Balanced | Best for |
| --- | --- | --- | --- | --- |
| **kd-tree** | points (median splits) | points | yes, if built from all data | Static point sets, exact k-NN, low d |
| **Quadtree/octree** | space (equal quarters) | points, regions | no — depends on distribution | Non-uniform density; easy insert/delete |
| **R-tree** | objects (bounding boxes) | **extended shapes** | yes (B-tree-like) | Rectangles, polygons; **disk-backed GIS** |
| **Grid / spatial hash** | space (uniform cells) | points | n/a | **Uniform density**, Θ(1), trivial to update |
| BVH | objects (bounding volumes) | triangles, meshes | built for query cost | Ray tracing |
| **HNSW / IVF** | approximate graph/clusters | high-dim vectors | n/a | **High dimensions** — the only thing that works |

**The grid deserves more credit than it gets.** For uniformly distributed points with a known query radius, a spatial hash is Θ(1) per query, trivially updatable, and often beats a tree — it's what particle simulations and collision broad-phases use. Trees earn their place when density is *non-uniform*.

**R-trees are the database answer**: PostGIS, SQLite's R\*Tree, and most GIS indexes are R-trees because they handle extended geometry and page well to disk (their node structure is B-tree-like — see [b-trees](../b-trees/learning.md)).

### The curse of dimensionality

As dimension `d` grows, the pruning test `distance_to_plane² < best²` almost always succeeds, so both branches get searched and the tree degenerates to a scan. Intuitively: in high dimensions almost all the volume of a hypersphere is near its surface, so all points end up roughly equidistant and "nearest" stops being meaningful.

Practical thresholds: kd-trees are excellent to ~10 dimensions, marginal to ~20, and useless beyond. Since embeddings are 128–1536 dimensions, **exact k-NN is off the table** and the answer is approximate:

| Method | Idea | Trade |
| --- | --- | --- |
| **HNSW** | Multi-layer navigable small-world graph | Best recall/speed; high memory; the current default |
| IVF (inverted file) | Cluster, search nearest clusters only | Lower memory, needs training |
| **LSH** | Hash so nearby points collide | Simple, tunable, weaker recall |
| Product quantization | Compress vectors, search compressed | Huge memory savings; used with IVF |

## Complexity

| Operation | kd-tree | Quadtree | R-tree | Grid | Linear |
| --- | --- | --- | --- | --- | --- |
| Build | Θ(n log n) | Θ(n log n) | Θ(n log n) | **Θ(n)** | — |
| **Nearest neighbour** | **Θ(log n) avg**, Θ(n) worst | Θ(log n) avg | Θ(log n) avg | **Θ(1) avg** | Θ(n) |
| Range query | Θ(√n + k) in 2-D | Θ(log n + k) | Θ(log n + k) | Θ(cells + k) | Θ(n) |
| Insert | Θ(log n), unbalances | Θ(log n) | Θ(log n) | **Θ(1)** | Θ(1) |
| Delete | hard — usually rebuild | Θ(log n) | Θ(log n) | **Θ(1)** | Θ(1) |
| Space | Θ(n) | Θ(n) can blow up | Θ(n) | Θ(cells + n) | Θ(n) |
| **High dimensions** | **degrades to Θ(n)** | worse | degrades | useless | Θ(n) |

**Where the table misleads.** "Θ(log n) average" for nearest neighbour is conditional on low dimension *and* reasonably uniform data — the worst case really is Θ(n), and it's reached routinely in high dimensions rather than being a theoretical corner. Measured, the 2-D case gave 2,414× at n = 1M; in 100 dimensions the same code would give roughly 1×.

The quadtree space row is worth noting: because it splits *space* rather than points, a cluster of nearly-coincident points forces deep subdivision, and the tree can be arbitrarily larger than n. kd-trees split at the median so they're balanced by construction — which is the main reason to prefer them for static point sets.

## Use Cases

- **Games and simulation** — collision broad-phase (grid or BVH), visibility, AI perception queries. Uniform grids dominate here because objects move every frame and rebuilding a tree is too expensive.
- **Geospatial** — "restaurants within 2 km", map tile lookup, routing preprocessing. R-trees in PostGIS; geohash/S2/H3 as grid-like alternatives that turn 2-D into sortable 1-D keys.
- **Ray tracing** — BVHs, built to minimize expected traversal cost rather than to balance.
- **Vector search / RAG** — embedding similarity at 128–1536 dimensions. **HNSW**, because exact k-NN is impossible at that dimensionality — `hnsw_rs`, `usearch`, or a vector database.
- **Clustering** — k-means acceleration, DBSCAN's neighbourhood queries.
- **Robotics and path planning** — RRT builds a kd-tree of sampled configurations and queries nearest repeatedly.
- **Nearest-neighbour classification** — k-NN in low-dimensional feature spaces.
- **Deduplication** — near-duplicate detection via LSH or MinHash ([probabilistic data structures](../probabilistic-data-structures/learning.md)).

## When to Use Which

| Reach for | When |
| --- | --- |
| **Linear scan** | n small, or fewer than ~3 queries (build amortized in 3 here) |
| **Grid / spatial hash** | Uniform density, known query radius, frequent updates |
| **kd-tree** | Static point set, exact k-NN, **d ≤ ~10** |
| Quadtree/octree | Non-uniform density; frequent insert/delete |
| **R-tree** | Extended shapes (rectangles, polygons), or disk-backed |
| BVH | Ray/triangle intersection |
| Geohash / S2 / H3 | You want 1-D sortable keys — composes with `BTreeMap` |
| **HNSW / IVF** | **d > ~20** — approximate is the only option |

## Pitfalls in Depth

### Pitfall: Using a spatial tree in high dimensions

- **What goes wrong:** A kd-tree is built over 768-dimensional embeddings and is *slower* than a linear scan — it does the same number of distance computations plus tree traversal overhead. The 2,414× measured in 2-D becomes a small loss.
- **Why it happens (the mechanism):** The pruning test `distance_to_plane² < best²` is what skips work. In high dimensions, the distance to any single splitting plane is small relative to the distance between points (most of a hypersphere's volume is near its surface), so the test almost always succeeds and both branches are searched. The tree still recurses but prunes nothing.
- **How to handle it in production, and why that works:** Above ~20 dimensions use **approximate** nearest neighbour — HNSW is the current default and gives high recall at a fraction of the work by navigating a small-world graph rather than partitioning space. Alternatively reduce dimensionality first (PCA, random projection) so an exact structure becomes viable, or use product quantization to make the linear scan itself much cheaper.
- **Trade-offs of the fix:** Approximate means you can miss the true nearest neighbour — recall is a tunable (typically 95–99%), and whether that's acceptable is a product decision. HNSW also uses substantially more memory than the vectors themselves and is expensive to build, and deletion is awkward.

### Pitfall: Rebuilding, or degrading, on updates

- **What goes wrong:** Points are inserted into a kd-tree built from a median split. The tree progressively unbalances (inserts go to leaves without rebalancing), queries degrade toward Θ(n), and deletion is worse still — removing an internal node requires finding a replacement along the splitting dimension, which is genuinely awkward. Teams end up rebuilding the whole tree per frame or per batch.
- **Why it happens (the mechanism):** A kd-tree's balance comes from choosing the *median* at build time over all the data. Incremental inserts have no median to choose, so the structure has no mechanism to stay balanced — unlike a [B-tree or balanced BST](../binary-search-trees/learning.md), which rebalance on the path. Measured, the build cost 36.30 ms at n = 1M, so per-frame rebuilding is not viable for interactive workloads.
- **How to handle it in production, and why that works:** Match the structure to the update rate. Frequent movement (games, simulation) → a **uniform grid or spatial hash**, where an update is removing from one cell and adding to another in Θ(1). Moderate updates → a **quadtree or R-tree**, both of which support insert and delete natively. Static or batch-rebuilt data → kd-tree, and amortize the build (measured: it pays for itself in three queries).
- **Trade-offs of the fix:** A grid is only good with roughly uniform density — clustered data puts everything in one cell and degrades to a scan. R-trees allow overlapping MBRs, so query performance depends on insertion order and split heuristics (R\*-tree exists precisely to improve those).

### Pitfall: Getting the pruning test wrong

- **What goes wrong:** The far-side test compares against the wrong quantity — unsquared distance against squared, or the distance to the *node* rather than to the splitting *plane*. Pruning too aggressively returns **wrong answers** (a true nearest neighbour in the skipped branch); pruning too little silently degrades to a full scan with tree overhead on top.
- **Why it happens (the mechanism):** The test is a lower bound on anything in the far region, and the correct bound is the perpendicular distance to the splitting hyperplane — not the distance to the node's point, which is larger and would prune valid candidates. Mixing squared and unsquared distances is easy because the rest of the code uses squares for speed. Both errors produce *plausible* results: the wrong answer is usually a near-neighbour, and the slow version is merely slow.
- **How to handle it in production, and why that works:** Always compare squared quantities consistently (`diff * diff < best_squared`), and use the distance to the *plane*. Then verify against a brute-force scan on random inputs — the measurement in this doc asserted the kd-tree's results matched the linear scan's to within 1e-4, which is exactly the check that catches an over-aggressive prune.
- **Trade-offs of the fix:** The brute-force oracle is Θ(n) per query so it stays in tests. Keeping it is worth it — it validates every variant you write later, and it's five lines.

### Pitfall: Ignoring the build/query break-even

- **What goes wrong:** A spatial index is built for a handful of queries. Measured, the build at n = 1,000,000 cost 36.30 ms while a linear-scan query cost ~1.27 ms — so for one or two queries, scanning wins.
- **Why it happens (the mechanism):** The per-query speedup is enormous (2,414×), which makes indexing feel unconditional. But the comparison is `build + q × fast` versus `q × slow`, and the break-even is `build / (slow − fast)` — here about **three queries**, which is low, but not zero, and it rises sharply if the data changes between query batches.
- **How to handle it in production, and why that works:** Compute the break-even from your own numbers. It's usually tiny for spatial structures (three queries here), which means indexing is almost always right for a static point set — but for a set that changes between every small batch of queries, the rebuild cost dominates and a scan or a grid is better.
- **Trade-offs of the fix:** Deferring index construction until a query threshold adds a mode switch and a latency spike on the crossing query. Given a break-even of three, the simpler choice is usually to just build it.

### Pitfall: A uniform grid on clustered data

- **What goes wrong:** A spatial hash is chosen for its Θ(1) simplicity, and the data is clustered — city centres, particle aggregates, hotspots. Most cells are empty while a few hold thousands of points, so a query in a dense region scans thousands of candidates and the Θ(1) claim evaporates.
- **Why it happens (the mechanism):** A grid's Θ(1) depends on **bounded occupancy per cell**, which holds only under roughly uniform density. Cell size is a single global parameter that can't adapt: make cells small enough for the dense regions and the sparse regions require scanning many empty cells; make them large enough for the sparse regions and dense cells overflow.
- **How to handle it in production, and why that works:** Use a structure that adapts to density — a **quadtree** subdivides only where points are dense, and a **kd-tree**'s median splits are balanced by construction regardless of distribution. Alternatively use a hierarchical grid (several resolutions) or size cells from the *observed* density rather than the extent.
- **Trade-offs of the fix:** Quadtrees can subdivide deeply around near-coincident points, so they need a depth cap and a leaf bucket. kd-trees give up the grid's Θ(1) update. Hierarchical grids multiply memory and complicate the query. The grid remains the right answer when density genuinely is uniform — which is common in particle simulation and rare in geospatial data.
