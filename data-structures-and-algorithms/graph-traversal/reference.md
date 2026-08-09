# Graph Traversal — Quick Reference

## At a Glance

**BFS and DFS are one algorithm.** The frontier container decides which: front → BFS, back → DFS, smallest → Dijkstra.

**Invariant:** every vertex is undiscovered → discovered → finished, one way only.
**BFS:** when a vertex at distance *d* is dequeued, the queue holds only *d* and *d+1* — that's the shortest-path proof, and exactly what weights break.
**DFS:** discovery/finish intervals are properly **nested** — the parenthesis structure behind cycles, topo order, SCCs, bridges.

## The Number

Path graph, depth = n (measured):

| n | Recursive DFS | Explicit stack |
| --- | --- | --- |
| 100,000 | ✅ | ✅ |
| **200,000** | **abort: stack overflow** | ✅ |
| 5,000,000 | — | ✅ |

Not a panic — uncatchable. Spawned threads (2 MB vs 8 MB) fail ~4× sooner.

## Complexity

| Algorithm | Time | Space |
| --- | --- | --- |
| BFS | Θ(V+E) | Θ(V) |
| DFS iterative | Θ(V+E) | Θ(E) mark-on-pop, Θ(V) mark-on-push |
| **DFS recursive** | Θ(V+E) | **Θ(depth) call stack — aborts ~100k** |
| Kahn toposort | Θ(V+E) | Θ(V) — detects cycles free |
| Multi-source BFS | Θ(V+E) | Θ(V) — independent of source count |
| 0-1 BFS | Θ(V+E) | Θ(V) — beats Dijkstra's Θ(E log V) |
| Bidirectional BFS | ~Θ(b^(d/2)) | ~Θ(b^(d/2)) |

## Mark on Push vs Pop

| | When | Frontier bound |
| --- | --- | --- |
| **Push** | **BFS — required** (first discovery is shortest) | ≤ V |
| **Pop** | DFS (matches recursive order); Dijkstra lazy deletion | Θ(E) |

## Snippets

```rust
// Unified: VecDeque+pop_front = BFS; Vec+pop = DFS; BinaryHeap = Dijkstra
while let Some(u) = frontier.pop_front() {
    for v in g.neighbours(u) {
        if dist[v] == MAX { dist[v] = dist[u] + 1; frontier.push_back(v); }  // mark on PUSH
    }
}

// Multi-source: seed ALL sources at 0. Still Θ(V+E).
for &s in sources { dist[s] = 0; q.push_back(s); }

// 0-1 BFS: deque replaces the heap entirely
if w == 0 { dq.push_front(v) } else { dq.push_back(v) }

// Kahn: iterative, cycle-detecting, no stack risk
while let Some(u) = q.pop_front() {
    order.push(u);
    for v in g.neighbours(u) { indeg[v] -= 1; if indeg[v] == 0 { q.push_back(v); } }
}
if order.len() != n { /* cycle */ }
```

## Choose This When

| Use | For |
| --- | --- |
| **BFS** | Shortest path in **edges**; level order |
| **DFS (iterative)** | Structure: cycles, topo order, SCCs, bridges |
| DFS (recursive) | Depth provably small |
| **Kahn's** | DAG ordering — iterative + free cycle detection |
| **Multi-source BFS** | "Nearest of k sources" — one pass, not k |
| **0-1 BFS** | Weights ∈ {0,1} — no heap needed |
| Dijkstra | Arbitrary non-negative weights |
| Bidirectional BFS | Point-to-point in a big space |
| DSU | Connectivity only, edges streaming |

## What Only DFS Gives You

| Result | Mechanism |
| --- | --- |
| Directed cycle detection | Back edge — target still on the stack |
| Topological sort | Reverse finish order |
| SCCs | Tarjan low-link / Kosaraju two-pass |
| Bridges, articulation points | Low-link vs discovery time |

## Rules of Thumb

- Iterative DFS whenever depth isn't provably bounded.
- BFS marks on **push**; DFS marks on **pop**. Don't copy one skeleton to the other.
- BFS on a weighted graph answers a *different question* — silently.
- Visited set is touched per **edge** — use `Vec<bool>`/bitset, not `HashSet`, for bounded spaces.
- Write traversal against a neighbour *function* so implicit graphs work unchanged.
- k sources → seed them all, don't run k searches.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Recursive DFS on deep input | Uncatchable abort at ~100k (~25k on a worker thread) |
| BFS marks on pop | Queue grows to Θ(E); distances can be wrong |
| BFS on weighted edges | Fewest-edges path returned as "shortest" |
| `HashSet` visited on a dense graph | Hash per edge dominates the traversal |
| Non-injective state encoding | Valid states silently pruned |
| k separate Dijkstras for k sources | Θ(k·E log V) instead of Θ(V+E) |

## Key References

- CLRS ch. 22 — BFS, DFS, parenthesis theorem, edge classification, toposort
- Beamer et al., "Direction-Optimizing BFS" (2012)
- Kahn (1962) — the in-degree toposort
