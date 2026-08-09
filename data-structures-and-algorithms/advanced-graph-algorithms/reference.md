# Advanced Graph Algorithms — Quick Reference

## At a Glance

Almost everything here is **three ideas**: (1) DFS timestamps + **low-link** → all structural results; (2) **augmenting paths** → matching and flow; (3) **preprocessing** → cheap tree-path queries. Most "hard" problems are one **reduction** away from something you know.

**Low-link:** `low[u]` = earliest discovery time reachable from u's subtree using **at most one** back edge.

| Condition | Means |
| --- | --- |
| `low[v] > disc[u]` (tree edge) | **(u,v) is a bridge** |
| `low[v] ≥ disc[u]`, u not root | **u is an articulation point** |
| root has ≥2 DFS children | root is an articulation point |
| `low[u] == disc[u]` (Tarjan) | **u is an SCC root** |

**Update rules (the classic bug):** tree edge → `low[u] = min(low[u], low[v])`; back edge → `low[u] = min(low[u], disc[w])` — **`disc`, not `low`**.

## Complexity

| Problem | Cost |
| --- | --- |
| SCC (Tarjan / Kosaraju) | Θ(V+E) |
| Bridges / articulation points | Θ(V+E) |
| **2-SAT** | Θ(V+E) via SCC — **3-SAT is NP-complete** |
| LCA binary lifting | Θ(n log n) prep, Θ(log n) query |
| LCA Euler + sparse table | Θ(n log n) prep, **Θ(1)** query |
| Heavy-light decomposition | Θ(log² n) per path op |
| Bipartite matching (Hopcroft-Karp) | Θ(E√V) |
| Max-flow (Dinic) | Θ(V²E); **Θ(E√V)** unit capacity |
| Min-cost max-flow | Θ(V·E·**F**) — **pseudo-polynomial** |
| **Max independent set: bipartite** | **polynomial** (König) |
| **Max independent set: general** | **NP-hard** |
| **Longest path** | **NP-hard** (shortest is easy!) |

## The Reduction List

| Problem | Reduces to |
| --- | --- |
| 2-SAT | SCC on the implication graph |
| Bipartite matching | Max-flow, unit capacities |
| Min vertex cover (bipartite) | Max matching (**König**) |
| Max independent set (bipartite) | V − max matching |
| Min path cover of a DAG | n − max matching (**Dilworth**) |
| Project selection / max closure | **Min-cut** |
| Image segmentation | Min-cut |
| Feasibility with lower bounds | Flow with a super-source |

**Max-flow min-cut:** max flow = min cut. The cut = vertices reachable from `s` in the final residual graph.

## Choose This When

| Use | For |
| --- | --- |
| **Tarjan SCC** | Cycles, condensation, 2-SAT — one pass, no reverse graph |
| Kosaraju | You already have the reverse graph |
| **Low-link DFS** | Bridges, articulation points, biconnectivity |
| Binary lifting | LCA + k-th ancestor + path aggregates |
| Euler + sparse table | LCA, queries ≫ updates (Θ(1)) |
| Heavy-light | Path *updates* on trees |
| **Dinic's** | Max-flow/min-cut — and matching as a special case |
| Hungarian | Pure assignment — Θ(V³), no weight dependence |
| **Stop** | Longest path / colouring / clique / independent set on a general graph |

## Rules of Thumb

- **Write all of these iteratively** — recursive DFS aborts at ~200k depth (~50k on a worker thread).
- Condensation by SCC always yields a **DAG** — then toposort/DP become available.
- Check the NP-hard list *before* implementing.
- Structure restores tractability: bipartite, planar, bounded treewidth, DAG.
- Verify low-link code against a brute-force oracle (remove edge/vertex, count components).
- Min-cost flow's `F` is the *flow value*, not the graph size — use capacity scaling.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Recursive DFS | Uncatchable abort on a deep graph |
| `low[w]` instead of `disc[w]` on a back edge | Wrong bridges/SCCs; passes many tests |
| Tarjan stack-membership not tracked | SCCs merged incorrectly |
| Min-cost flow with large capacities | Hours on a few-hundred-node graph |
| Hand-rolled heuristic | Slower and approximate where an exact reduction existed |
| Attacking an NP-hard problem exactly | Works on tests, never finishes in prod |

## Key References

- Tarjan (1972) — SCCs, bridges, articulation points from one DFS
- Aspvall, Plass & Tarjan (1979) — 2-SAT via SCC, two pages
- Bender & Farach-Colton (2000) — LCA ↔ RMQ
- [CP-Algorithms: Graphs](https://cp-algorithms.com/#graphs) — working code for all of it
