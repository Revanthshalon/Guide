# Shortest Paths — Quick Reference

## At a Glance

Every algorithm is the **same relaxation** — `if dist[u]+w < dist[v] { dist[v] = dist[u]+w }` — applied in a different order. The order is the algorithm.

**Dijkstra's invariant:** when a vertex is popped with `d == dist[u]`, `dist[u]` is final. The proof needs **non-negative** edges; one negative edge breaks it *silently*.
**Bellman-Ford's invariant:** after k rounds, `dist[v]` is correct for paths of ≤ k edges. Hence V−1 rounds; a V-th improving round ⇒ **negative cycle**.

## The Numbers (V=200k, E=1M, measured, identical results)

| Approach | Time |
| --- | --- |
| **Dijkstra + binary heap** | **56.34 ms** |
| Dijkstra + O(V²) scan | **78.92 s** (**1,401×**) |

At V=20k it was only 66× — this scales *into* the problem.

**Lazy deletion, measured:** 391,050 pushes = **2.0×V** (not E), **49% of pops stale**. The "heap grows to Θ(E)" worry is worst case, not typical.

## Complexity

| Algorithm | Time | Negative weights |
| --- | --- | --- |
| BFS | Θ(V+E) | n/a |
| 0-1 BFS | Θ(V+E) | no |
| **Dijkstra (binary heap)** | **Θ((V+E) log V)** | **no** |
| Dijkstra (Fibonacci) | Θ(E + V log V) | no — **slower in practice** |
| **DAG relaxation** | **Θ(V+E)** | **yes** |
| Bellman-Ford | Θ(V·E) | **yes**, detects neg. cycles |
| SPFA | Θ(V·E) worst, fast typical | yes |
| Floyd-Warshall | Θ(V³) | yes |
| A\* | Θ(E log V) worst, far less typical | no |

## Choose This When

| Weights | Use |
| --- | --- |
| Unweighted | **BFS** |
| {0,1} | **0-1 BFS** |
| Small ints ≤ C | Dial's / bucket queue — Θ(E+VC) |
| Non-negative | **Dijkstra + heap + lazy deletion** |
| **Graph is acyclic** | **DAG relaxation** — Θ(V+E), allows negatives |
| Negative | Bellman-Ford / SPFA |
| Need neg-cycle detection | Bellman-Ford |
| All pairs, V ≤ ~500 | Floyd-Warshall |
| Point-to-point + heuristic | **A\*** |
| Millions of queries, static | Contraction hierarchies |

## Snippets

```rust
// Dijkstra with lazy deletion — the default
while let Some(Reverse((d, u))) = pq.pop() {
    if d > dist[u] { continue; }                 // ← REQUIRED: 49% of pops are stale
    for (v, w) in g.neighbours(u) {
        if d + w < dist[v] { dist[v] = d + w; prev[v] = u; pq.push(Reverse((d + w, v))); }
    }
}

// DAG: Θ(V+E), negatives fine, no heap
for u in topo_order {
    if dist[u] == INF { continue; }
    for (v, w) in g.neighbours(u) { if dist[u]+w < dist[v] { dist[v] = dist[u]+w; } }
}

// Bellman-Ford + negative cycle detection
for _ in 0..n-1 { /* relax all edges; break if unchanged */ }
// one more pass: anything still improving is on/after a negative cycle

// Floyd-Warshall — k MUST be outermost
for k in 0..n { for i in 0..n { for j in 0..n {
    d[i][j] = d[i][j].min(d[i][k] + d[k][j]); }}}
```

## Rules of Thumb

- Check the weights before choosing. Unweighted → BFS, not Dijkstra.
- **Acyclic → DAG relaxation.** Θ(V+E), and it's the only way to get *longest* path.
- Binary heap + lazy deletion, never a Fibonacci heap.
- The staleness check is load-bearing, not an optimization.
- Integer-scale weights (mm, ms, cents) — floats aren't `Ord` and aren't associative.
- `u64::MAX` as infinity **overflows** on the first addition — guard or use `MAX/2`.
- Multi-source: seed all sources at 0, one run.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Negative edge + Dijkstra | Confidently wrong distances, no error |
| O(V²) scan instead of a heap | 1,401× at V=200k; invisible at V=20k |
| Missing staleness check | ≥2× the work; still correct, looks like a mystery |
| `dist[u] + w` with `u` unreached | Overflow wraps to ~0 (release) or panics (debug) |
| Dijkstra on a DAG | Θ(E log V) instead of Θ(V+E); rejects negatives |
| DAG relaxation on a cyclic graph | Silently wrong — use Kahn's to detect |
| `partial_cmp().unwrap()` float weights | Panic on `NaN`; order-dependent ties |

## Key References

- Dijkstra (1959) — two and a half pages
- CLRS ch. 24–25 — all of them, with proofs
- Hart, Nilsson & Raphael (1968) — A\*, admissibility, consistency
- Geisberger et al., "Contraction Hierarchies" (2008)
