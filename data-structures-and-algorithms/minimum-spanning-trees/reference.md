# Minimum Spanning Trees — Quick Reference

## At a Glance

The cheapest edge set keeping a graph connected — always exactly **V−1 edges**. The cleanest case where greedy is *provably* optimal.

**Cut property:** for any partition of the vertices, the minimum-weight edge crossing that cut is in some MST. (Exchange argument: adding it creates a cycle that crosses the cut again at some `f` with `w(f) ≥ w(e)`; swap.)
**Cycle property (dual):** the heaviest edge on any cycle is in no MST.
**Invariant both maintain:** the current edge set is a subset of some MST.

## The Numbers (measured, identical weights)

| Algorithm | V=20k, E=100k | V=200k, E=1M |
| --- | --- | --- |
| **Kruskal (sort + DSU)** | **1.31 ms** | **14.63 ms** |
| Prim (lazy binary heap) | 7.19 ms | 108.02 ms |

**Kruskal 7.4× faster** — opposite of the usual "Prim for dense" advice. Same Θ(E log E); the constants decide.

## Complexity

| Algorithm | Time | Space |
| --- | --- | --- |
| **Kruskal** | Θ(E log E) sort + Θ(E α(V)) | Θ(V+E) |
| Kruskal, pre-sorted / radix | **Θ(E α(V))** | Θ(V+E) |
| **Prim (lazy heap)** | Θ(E log E) | **Θ(E) heap** |
| Prim (eager, indexed) | Θ(E log V) | Θ(V) heap |
| Prim (array, dense) | **Θ(V²)** | Θ(V) |
| **Borůvka** | Θ(E log V) | Θ(V+E) — **parallelizable** |
| Karger-Klein-Tarjan | Θ(E) expected | Θ(E) |

## Choose This When

| Use | For |
| --- | --- |
| **Kruskal** | **The default** — 7.4× measured, simplest, forest for free |
| Kruskal + radix sort | Small integer weights, large E |
| **Prim (lazy)** | Edges not materialized; implicit graph |
| Prim (array Θ(V²)) | Dense + matrix + small V |
| **Borůvka** | Parallel / distributed |
| DSU alone | Connectivity only, not the tree |
| **Dijkstra** | You actually wanted a *shortest-path* tree |

## Snippets

```rust
// Kruskal — the whole algorithm
edges.sort_unstable_by_key(|e| e.2);
let mut dsu = Dsu::new(n);
for e in edges {
    if dsu.union(e.0, e.1) {                 // ← the return value IS the cycle check
        total += e.2 as u64; tree.push(e);
        if tree.len() == n - 1 { break; }    // early exit
    }
}
// tree.len() < n-1 ⇒ the graph was disconnected (a FOREST, not a tree)

// Deterministic output despite ties:
edges.sort_unstable_by_key(|e| (e.2, e.0, e.1));

// Maximum spanning tree: sort descending — the cut property dualizes
edges.sort_unstable_by_key(|e| Reverse(e.2));
```

## Prim vs Dijkstra — one token

| | Heap key | Optimizes |
| --- | --- | --- |
| **Prim** | `w` | total tree weight |
| **Dijkstra** | `d + w` | distance from the source |

## Rules of Thumb

- Kruskal by default; reach for Prim only when edges aren't materializable.
- `union`'s bool is the cycle check — `#[must_use]` it.
- Break at V−1 accepted edges.
- The MST is unique **only if all weights are distinct** — tests compare *weight*, not edge sets.
- `tree.len() != n-1` means the graph is disconnected. Surface it.
- MST path minimizes the **maximum** edge (bottleneck), not the total — that's not a shortest path.
- Floats → `total_cmp` / `NotNan` / integer-scale.
- Borůvka needs **consistent** tie-breaking or components form cycles.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Ignored `union` return | Cycles in the output; weight above minimum |
| MST used for routing | Paths arbitrarily longer than shortest |
| Test compares edge lists | Intermittent failures once weights tie |
| Lazy Prim on a dense graph | Θ(E) heap; 7.4× slower than Kruskal |
| Assumed connectivity | Silent forest reported as a spanning tree |
| Inconsistent Borůvka ties | Cycle in the "tree" |

## Key References

- Borůvka (1926) — first, and the one that parallelizes
- Kruskal (1956) · Prim (1957) — both short
- CLRS ch. 23 — the cut property with a careful proof
