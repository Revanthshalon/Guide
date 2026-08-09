# Graph Representations — Quick Reference

## At a Glance

The representation decides which questions are cheap. Dominated by **density** `d = E/V²` — real graphs are almost always sparse, which disqualifies the matrix.

**Invariants:** *adjacency list* — `adj[u]` = exactly u's neighbours; undirected edges appear **twice**. *Matrix* — `M[u][v]` iff edge; symmetric if undirected. *CSR* — `offs[u]..offs[u+1]` indexes `tgt`; `offs` non-decreasing, `offs[V] = 2E`.

## The Numbers (V=500k, 4M arcs, measured)

| | `Vec<Vec<…>>` | **CSR** | Ratio |
| --- | --- | --- | --- |
| Build | 132.3 ms | **40.6 ms** | 3.26× |
| Memory | ~41 MB | **~32 MB** | 1.29× |
| BFS | 38.2 ms | **21.7 ms** | 1.76× |

Bit-packed **adjacency matrix would be 29 GB** — ~900× CSR.

## Complexity

| Operation | Adj list | Matrix | **CSR** | Edge list |
| --- | --- | --- | --- | --- |
| Space | Θ(V+E) | **Θ(V²)** | Θ(V+E) | Θ(E) |
| Neighbours of u | Θ(deg u) | **Θ(V)** | Θ(deg u) contiguous | Θ(E) |
| Edge exists? | Θ(deg u) | **Θ(1)** | Θ(log deg) | Θ(E) |
| Add edge | Θ(1) | Θ(1) | **rebuild** | Θ(1) |
| All edges | Θ(V+E) | Θ(V²) | Θ(V+E) | **Θ(E)** |
| Degree of u | Θ(1) | Θ(V) | **Θ(1)** | Θ(E) |

## Choose This When

| Use | For |
| --- | --- |
| **CSR** | Sparse + static + algorithms run repeatedly — **the default** |
| Adjacency list | Sparse + mutating; prototyping |
| `petgraph` | Mutating, or you want the algorithm library |
| **Matrix / bitset rows** | Dense, or V ≤ ~2,000, or need Θ(1) edge tests |
| **Edge list** | Kruskal, Bellman-Ford, streaming |
| **Implicit (nothing stored)** | Grids, state spaces, puzzles |
| DSU alone | Connectivity over an edge stream |

## CSR Construction — a counting sort by source

```rust
let mut offs = vec![0u32; n + 1];
for &(a, b, _) in edges { offs[a as usize + 1] += 1; offs[b as usize + 1] += 1; }
for i in 0..n { offs[i + 1] += offs[i]; }              // prefix sum
let mut cur = offs.clone();
for &(a, b, w) in edges {
    let i = cur[a as usize] as usize; tgt[i] = b; wt[i] = w; cur[a as usize] += 1;
    let j = cur[b as usize] as usize; tgt[j] = a; wt[j] = w; cur[b as usize] += 1;
}
```

Keep `tgt` and `wt` as **parallel arrays** — unweighted traversals then never touch weights.

## Rules of Thumb

- Compute `E/V²` before choosing. Sparse → CSR; V ≤ ~2,000 → matrix is fine.
- Vertices are `u32` indices, never `Rc<RefCell<Node>>` — graphs are cyclic by nature.
- Funnel edge insertion through one `add_edge`; assert `sum(deg) == 2E`.
- Write algorithms against a neighbour *function* so implicit graphs work unchanged.
- `u32` targets + `u64` offsets for large graphs.
- Prototype with `Vec<Vec<_>>`, convert to CSR once it works.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Matrix on a sparse graph | 29 GB at V=500k; BFS becomes Θ(V²) |
| Only `adj[a].push(b)` for undirected | Half the graph missing; wrong components |
| Counted `sum(adj.len())` as E | Every derived statistic doubled |
| Materialized an implicit graph | OOM on a state space that search would barely touch |
| `usize` vertex IDs | 2× memory on the dominant allocation |
| `u32` CSR offsets on a huge graph | Silent wrap above 4.29B arcs |

## Key References

- CLRS ch. 22.1 — representations and trade-offs
- [`petgraph`](https://docs.rs/petgraph/) — `Graph` vs `StableGraph` vs `Csr`
- Shun & Blelloch, "Ligra" (2013) — representation drives everything at scale
