# Advanced Graph Algorithms — Learning Notes

## Mental Model

**Almost everything in this topic is one of three ideas applied repeatedly.** The algorithms look intimidating individually and collapse into a small set once you see the shared machinery:

1. **DFS timestamps and low-link.** Every result about *structure* — strongly connected components, bridges, articulation points, biconnectivity, 2-SAT — comes from the nested discovery/finish intervals of a DFS plus one derived number: **low-link**, the earliest discovery time reachable from a vertex's subtree using at most one back edge. Learn low-link once and four algorithms follow.
2. **Augmenting paths.** Bipartite matching and max-flow are the same algorithm: repeatedly find a path from source to sink along which you can push more, and push. The correctness argument in both cases is that you stop exactly when no such path exists, and that condition coincides with optimality (König's theorem, max-flow min-cut).
3. **Preprocessing to turn queries into lookups.** LCA, and by extension many tree-path queries, are answered by spending Θ(n log n) once so each query becomes Θ(log n) or Θ(1). Binary lifting and Euler-tour-plus-RMQ are two spellings of the same trade.

The second framing worth carrying: **most "hard" graph problems are a reduction away from something you already know.** 2-SAT is SCCs on an implication graph. Bipartite matching is max-flow with unit capacities. Minimum vertex cover on a bipartite graph is max-flow via König. Project selection, image segmentation, and baseball elimination are all min-cut. **Recognizing the reduction is the skill**; the algorithms are library code.

And the counterweight, which matters more than any algorithm here: **the same problems on general (non-bipartite) graphs are usually NP-hard.** Maximum independent set is polynomial on bipartite graphs and NP-hard in general. Longest path is NP-hard while shortest path is easy. Knowing where that cliff is prevents the most expensive mistake in this topic — spending a week on an exact algorithm that cannot exist.

## The Invariant

**Low-link, the shared foundation:**

> `low[u]` = the minimum discovery time reachable from `u`'s DFS subtree using tree edges downward plus **at most one** back edge.

Everything follows from comparing `low` against `disc`:

| Condition | Meaning |
| --- | --- |
| `low[v] > disc[u]` for tree edge (u,v) | **(u,v) is a bridge** — no way back around it |
| `low[v] ≥ disc[u]` for tree edge (u,v), u not root | **u is an articulation point** |
| root has ≥ 2 DFS children | root is an articulation point |
| `low[u] == disc[u]` (Tarjan, directed) | **u is the root of an SCC** |

The "at most one back edge" clause is the part people get wrong. Using a back edge means leaving the subtree via a non-tree edge, and allowing two would let you escape the subtree entirely and return, which breaks every one of the above conditions.

**Max-flow:**

> A flow is **feasible** if it respects capacities (`0 ≤ f(e) ≤ c(e)`) and conserves flow at every vertex except source and sink. It is **maximum** iff the residual graph contains **no augmenting path** from s to t.

That last equivalence is the max-flow min-cut theorem, and it's why the algorithms are simply "find augmenting paths until none exist" — termination *is* optimality. The set of vertices reachable from s in the final residual graph is the minimum cut, which is how you *extract* the cut rather than just its value.

**LCA by binary lifting:**

> `up[k][v]` = the 2^k-th ancestor of `v`. Since any integer is a sum of distinct powers of two, any ancestor jump decomposes into ≤ log n table lookups.

## Mechanics

### Strongly connected components

An SCC is a maximal set of vertices that are all mutually reachable. Two algorithms, both Θ(V + E):

**Kosaraju** — DFS the graph recording finish times, then DFS the **reverse** graph in decreasing finish order; each tree in the second pass is an SCC. Two passes, conceptually simple, and it needs the reverse graph (cheap to build from CSR — see [graph representations](../graph-representations/learning.md)).

**Tarjan** — one pass. Maintain a stack of "vertices whose SCC isn't yet closed"; when `low[u] == disc[u]`, pop the stack down to `u` and that's an SCC. Faster in practice (one pass, no reverse graph) but the stack-membership bookkeeping is easy to get wrong.

The payoff is the **condensation**: contract each SCC to a single vertex and you get a **DAG**, always. That's what makes SCCs so useful — an arbitrary directed graph becomes acyclic, and then topological order, DAG shortest paths, and DP all become available.

### 2-SAT — the reduction worth knowing

For a formula in 2-CNF, build an implication graph: each clause `(a ∨ b)` becomes two edges `¬a → b` and `¬b → a`. Then:

> The formula is **satisfiable iff no variable and its negation are in the same SCC.**

If satisfiable, assign each variable by comparing SCC order in the condensation (a variable is true if its SCC comes *later* in reverse topological order than its negation's). Θ(V + E), for a problem that looks like SAT.

This is the cleanest example of the "reduction is the skill" point: 2-SAT is trivially easy and **3-SAT is NP-complete**. The line between them is exactly the cliff worth knowing about.

### LCA — two standard approaches

**Binary lifting:** precompute `up[k][v]` for `k` up to `log n`. To find LCA(u,v): lift the deeper one to the other's depth, then lift both together by decreasing powers of two while their ancestors differ. Θ(n log n) preprocessing, Θ(log n) per query, and the table also answers "the k-th ancestor" and "the maximum edge on the path" if you store aggregates alongside.

**Euler tour + RMQ:** flatten the tree by an Euler tour, and LCA becomes a range-minimum query over depths in that array. With a sparse table ([range query structures](../range-query-structures/learning.md)) that's Θ(n log n) preprocessing and **Θ(1) per query**. This is the reduction Bender & Farach-Colton made famous, and it's why RMQ and LCA are usually taught together.

**Heavy-light decomposition** generalizes further: decompose the tree into chains so any root-to-node path crosses Θ(log n) chains, then put a [segment tree](../range-query-structures/learning.md) on each chain. That gives path *updates and aggregates* in Θ(log² n), not just LCA.

### Bipartite matching and max-flow

**Hopcroft-Karp** finds a maximum bipartite matching in Θ(E√V) by augmenting along *many* shortest augmenting paths per phase (BFS to find the layer structure, DFS to augment). The simpler Hungarian/Kuhn's algorithm is Θ(V·E) and usually fine.

**Dinic's** generalizes it to max-flow: BFS builds a level graph, DFS finds blocking flows within it. Θ(V²E) in general, **Θ(E√V)** on unit-capacity networks (so bipartite matching is a special case), and fast in practice well beyond its bound.

The theorems that make these useful are the reductions:

| Theorem | Statement | Use |
| --- | --- | --- |
| **Max-flow min-cut** | max flow = min cut capacity | Extract the cut from the final residual graph |
| **König's** | In bipartite graphs, max matching = min vertex cover | Vertex cover in polynomial time |
| **Dilworth's** | Min path cover of a DAG = n − max matching | Scheduling, chain decomposition |
| **Hall's** | A perfect matching exists iff every subset S has \|N(S)\| ≥ \|S\| | Feasibility without computing the matching |

### The topics, and what each is really for

| Algorithm | Time | Built on | Real use |
| --- | --- | --- | --- |
| **Tarjan / Kosaraju SCC** | Θ(V+E) | low-link / reverse DFS | Condensation → DAG; deadlock cycles; module clustering |
| **Bridges, articulation points** | Θ(V+E) | low-link | Network single points of failure |
| **2-SAT** | Θ(V+E) | SCC | Constraint solving, scheduling with either/or |
| **LCA (binary lifting)** | Θ(n log n) / Θ(log n) | doubling | Tree path queries, phylogenetics, VCS merge-base |
| **LCA (Euler + RMQ)** | Θ(n log n) / **Θ(1)** | sparse table | Same, when queries dominate |
| **Heavy-light decomposition** | Θ(log² n) per op | segment trees | Path updates on trees |
| **Hopcroft-Karp** | Θ(E√V) | augmenting paths | Assignment, scheduling |
| **Dinic's max-flow** | Θ(V²E), Θ(E√V) unit | augmenting paths | Cuts, project selection, segmentation |
| **Min-cost max-flow** | Θ(V·E·flow) | Bellman-Ford + flow | Assignment with costs, transport |

## Complexity

| Problem | Best known | Notes |
| --- | --- | --- |
| SCC | Θ(V + E) | Tarjan (1 pass) or Kosaraju (2 passes + reverse graph) |
| Bridges / articulation points | Θ(V + E) | One DFS with low-link |
| 2-SAT | Θ(V + E) | Via SCC. **3-SAT is NP-complete** |
| Topological sort | Θ(V + E) | Kahn's or DFS finish order |
| LCA | Θ(n log n) prep, Θ(log n) or Θ(1) query | Lifting vs Euler+RMQ |
| Bipartite matching | Θ(E√V) | Hopcroft-Karp |
| Max-flow (general) | Θ(V²E) Dinic; Θ(VE) newer | Practice far beats the bound |
| Max-flow (unit capacity) | Θ(E√V) | Dinic |
| Min-cost max-flow | Θ(V·E·F) | SPFA-based, or Johnson potentials |
| **Max independent set (bipartite)** | **polynomial** | = V − max matching (König) |
| **Max independent set (general)** | **NP-hard** | The cliff |
| **Longest path** | **NP-hard** | Even though shortest path is easy |
| **TSP, graph colouring, max clique** | **NP-hard** | See the Stage 10 topic |

**Where the table misleads.** Max-flow's Θ(V²E) bound is almost never observed — Dinic's on realistic networks behaves far better, and the theoretical bounds have improved repeatedly without changing what people run. Conversely, min-cost max-flow's `F` factor (the flow value) is a genuine trap: it is **pseudo-polynomial**, so a network with capacities in the millions can be catastrophically slow even though the graph is small.

The NP-hard rows are the most important in the table. **Shortest path is easy and longest path is NP-hard** on the same graph, and nothing in the problem statement warns you. This is the checkpoint before implementing anything in this topic: *is the problem I actually have on the polynomial side of the line?*

## Rust Implementation

```rust
// Tarjan SCC — iterative, because recursion aborts past ~100k depth
// (measured in graph-traversal). The state machine replaces the call stack.
struct Tarjan { disc: Vec<u32>, low: Vec<u32>, on_stack: Vec<bool>, stack: Vec<u32>, time: u32 }

// The comparison that does all the work:
//   after visiting child v:  low[u] = min(low[u], low[v])
//   for a back edge to w on the stack: low[u] = min(low[u], disc[w])
//   if low[u] == disc[u]: pop the stack down to u — that's one SCC

// Bridges: identical DFS, one different comparison.
//   if low[v] > disc[u] { bridges.push((u, v)); }

// Binary lifting for LCA
let logn = (n as f64).log2().ceil() as usize + 1;
let mut up = vec![vec![u32::MAX; n]; logn];
up[0] = parent.clone();
for k in 1..logn {
    for v in 0..n {
        let mid = up[k - 1][v];
        up[k][v] = if mid == u32::MAX { u32::MAX } else { up[k - 1][mid as usize] };
    }
}
```

**Always write these iteratively.** Every algorithm here is DFS-based, and [graph traversal](../graph-traversal/learning.md) measured recursive DFS aborting at 200,000 depth (≈4× lower on a spawned thread). An SCC algorithm on a user-supplied graph with recursion is a denial-of-service vector, and the failure is an uncatchable abort rather than a panic.

**Crates:** `petgraph` (`tarjan_scc`, `kosaraju_scc`, `condensation`, `is_bipartite_undirected`, `ford_fulkerson`), `pathfinding` (`bipartite matching`, flows). For max-flow specifically, `petgraph`'s implementation is basic — a hand-rolled Dinic's is ~80 lines and much faster.

## Use Cases

- **Deadlock and cycle detection** — SCCs in a wait-for graph; a cycle *is* a deadlock.
- **Dependency analysis** — SCCs find circular imports; the condensation gives the buildable order. `cargo` and every module system needs this.
- **Network reliability** — bridges and articulation points are the single points of failure whose loss disconnects the network.
- **2-SAT constraint solving** — scheduling with either/or constraints, sequence assignment, some layout problems.
- **Assignment problems** — workers to tasks, students to schools, ads to slots. Bipartite matching, or min-cost flow when preferences are weighted.
- **Image segmentation** — min-cut on a pixel graph with terminal edges to foreground/background (GrabCut and relatives).
- **Project selection / maximum closure** — pick a profitable subset with prerequisite constraints; this is a min-cut.
- **Version control merge base** — LCA on the commit DAG (generalized, since it's a DAG rather than a tree).
- **Baseball elimination, scheduling feasibility** — max-flow feasibility, a classic and genuinely non-obvious reduction.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Tarjan SCC** | Cycles, condensation, 2-SAT — one pass, no reverse graph |
| Kosaraju SCC | You already have the reverse graph, or want the simpler code |
| **Low-link DFS** | Bridges, articulation points, biconnected components |
| **2-SAT via SCC** | Constraints of the form "a or b" |
| Binary lifting | LCA plus k-th ancestor plus path aggregates |
| Euler tour + sparse table | LCA when queries vastly outnumber updates (Θ(1)) |
| Heavy-light decomposition | Path *updates* on a tree, not just queries |
| **Hopcroft-Karp** | Bipartite matching |
| **Dinic's** | Max-flow / min-cut, including bipartite matching as a special case |
| Min-cost max-flow | Assignment with weights — watch the pseudo-polynomial `F` |
| **Stop and reconsider** | The problem is longest path, colouring, clique, or independent set on a general graph — it's NP-hard |

## Pitfalls in Depth

### Pitfall: Recursive DFS in an SCC or bridge algorithm

- **What goes wrong:** Tarjan's SCC, articulation points, or bridge-finding is written recursively — as every textbook presents it — and the process **aborts** with `fatal runtime error: stack overflow` on a deep graph. Measured in [graph traversal](../graph-traversal/learning.md), plain recursive DFS survives 100,000 depth and aborts at 200,000; on a spawned thread (2 MB rather than 8 MB) it fails around a quarter of that. It's not catchable, so one malformed input kills the whole process.
- **Why it happens (the mechanism):** These algorithms need discovery and finish times, which the call stack provides for free — so the recursive form is genuinely the natural expression, and every reference implementation uses it. Random test graphs have Θ(log n) diameter, so the deep case never appears in testing; a path-shaped graph, a long dependency chain, or an adversarially-constructed input produces it immediately.
- **How to handle it in production, and why that works:** Write the iterative version with an explicit state machine — a stack of `(vertex, child_iterator_position)` frames so you can distinguish "entering u" from "returning to u after child v". Depth is then bounded by heap memory rather than by a thread's stack size; the same measurement showed an explicit stack handling 5,000,000 vertices.
- **Trade-offs of the fix:** The iterative version is substantially less readable — you're hand-rolling what the compiler does for you, and the enter/exit bookkeeping is where bugs hide. For a graph you generate yourself with provable depth bounds, recursion is fine and the rewrite is wasted effort. The trigger is *untrusted or unbounded* input depth.

### Pitfall: Attacking an NP-hard problem as though it were polynomial

- **What goes wrong:** A week is spent building an exact algorithm for longest path, graph colouring, maximum clique, or independent set on a general graph. It works on the test instances and is unusably slow in production. Or it never terminates and the failure is misread as a bug.
- **Why it happens (the mechanism):** Nothing in the problem statement distinguishes tractable from intractable, and the tractable neighbours are seductive: **shortest path is Θ(E log V) while longest path is NP-hard** on the very same graph. Similarly, maximum independent set is polynomial on bipartite graphs (via König) and NP-hard one edge away from bipartite. The cliff is invisible without knowing where it is.
- **How to handle it in production, and why that works:** Check the problem against the known-hard list *before* implementing. If it's NP-hard, decide deliberately among: exact on small instances (DP over subsets, branch and bound), an approximation with a proven ratio (MST gives 2-approx metric TSP), a heuristic (local search, simulated annealing), or a solver (ILP/SAT — modern solvers handle surprisingly large instances and are far better than a hand-rolled search). Also check whether your graph has structure that restores tractability: bipartite, planar, bounded treewidth, or a DAG all make several NP-hard problems easy.
- **Trade-offs of the fix:** Approximations give up optimality, and the ratio may be unacceptable for the domain. Solvers add a dependency and can be unpredictable in runtime. But all of these beat an exact algorithm that doesn't finish — and recognizing the situation early is worth more than any of them.

### Pitfall: Getting low-link's "at most one back edge" wrong

- **What goes wrong:** In an articulation-point or bridge implementation, `low[u]` is updated from a child's `disc` instead of its `low`, or from `low` of a vertex reached by a back edge instead of `disc`. The algorithm reports bridges that aren't, misses real ones, or finds SCCs that are merged incorrectly. It's subtly wrong on some graphs and right on many, so tests pass.
- **Why it happens (the mechanism):** The two updates look symmetric but are not: for a **tree edge** to child `v` you take `low[v]` (the child's subtree can reach further back); for a **back edge** to an already-discovered `w` you take `disc[w]`, *not* `low[w]`. Taking `low[w]` would chain through a second back edge, violating the "at most one" clause and letting the value escape the subtree entirely.
- **How to handle it in production, and why that works:** Write the two update rules explicitly and comment which is which. Then test against a brute-force oracle: a bridge is an edge whose removal increases the component count, and an articulation point is a vertex whose removal does — both computable in Θ(V·E) by brute force, which is fast enough for a property test on small random graphs. That oracle catches every variant of this bug.
- **Trade-offs of the fix:** The brute-force checker is Θ(V·E) per assertion so it stays in tests only. Writing it is real work, but it's the same "verify against an obvious slow implementation" discipline that the augmented-tree and lazy-segment-tree topics needed, and it's the only reliable way to validate low-link code.

### Pitfall: Min-cost max-flow's pseudo-polynomial blowup

- **What goes wrong:** A min-cost max-flow is used for an assignment problem where capacities represent quantities — units of inventory, minutes of time, currency. The graph has only a few hundred nodes, so it looks small, and the algorithm takes hours. The complexity Θ(V·E·F) has `F` = the *flow value*, not the graph size.
- **Why it happens (the mechanism):** Successive-shortest-path algorithms augment one unit (or one bottleneck) at a time, so the number of iterations scales with the total flow, which scales with the *magnitude of the numbers in the input* rather than with their count. That's the definition of pseudo-polynomial: doubling the capacity values doubles the runtime while the input size grows by one bit.
- **How to handle it in production, and why that works:** Use capacity **scaling** (process high-order capacity bits first, roughly Θ(E² log C)), so the iteration count depends on log C rather than C. Or scale the units down if the precision isn't meaningful — minutes instead of seconds. For assignment specifically, the Hungarian algorithm is Θ(V³) with no dependence on weights at all, which is strictly better when the problem is a pure assignment.
- **Trade-offs of the fix:** Capacity scaling is meaningfully more complex to implement correctly. Reducing units loses precision, which may or may not matter. The Hungarian algorithm only applies to the assignment problem, not to general min-cost flow.

### Pitfall: Not recognizing the reduction

- **What goes wrong:** A custom heuristic is built for a problem that has an exact polynomial algorithm via a standard reduction. "Select a profitable subset of projects respecting prerequisites" gets a greedy approximation when it is exactly **maximum closure**, solvable optimally by min-cut. "Assign workers to shifts with either/or constraints" gets a backtracking search when it is **2-SAT** or **bipartite matching**. The result is a slower, approximate answer to a problem with a fast exact one.
- **Why it happens (the mechanism):** The reductions are not obvious — nothing about "profitable project selection" says min-cut, and the mapping (source→profitable projects with capacity = profit, prerequisites as infinite-capacity edges) has to be known rather than derived. Meanwhile a heuristic is always available and always *works*, just not optimally.
- **How to handle it in production, and why that works:** Learn the canonical reduction list — 2-SAT ← SCC; bipartite matching ← max-flow; min vertex cover ← max matching (König); min path cover ← matching (Dilworth); project selection / image segmentation ← min-cut; feasibility with lower bounds ← flow with a super-source. Then, when facing a combinatorial selection problem, check the list before designing anything. The reduction is the hard part; the algorithm is library code.
- **Trade-offs of the fix:** Reductions can blow up the instance (a flow network with a node per pixel is large), and the mapping is a place to introduce bugs — an exact algorithm on a wrongly-constructed network is worse than an honest heuristic. Verify the reduction on small instances against brute force before trusting it.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if the graph changed between queries? | Dynamic connectivity; link-cut trees; incremental SCC maintenance |
| Batch it | What if all queries were known up front? | Offline LCA (Tarjan's, with DSU); offline dynamic connectivity over a segment tree on time |
| Approximate it | What if near-optimal sufficed? | Approximate max-flow; local-search matching; the MST 2-approximation for metric TSP |
| Randomize it | What if you contracted random edges? | **Karger's min-cut** — repeatedly contract a random edge; embarrassingly simple, and it works |
| Externalize it | What if the graph exceeded RAM? | Semi-external SCC; Pregel/GraphX-style vertex-centric computation |
| **Parallelize it** | Where's the independence? | Borůvka-style component contraction; forward-backward parallel SCC; push-relabel max-flow (naturally parallel, unlike augmenting paths) |
| **Invert it** | What if you solved the dual? | **Max-flow ↔ min-cut**; matching ↔ vertex cover (König); the dual is often the easier thing to extract |
| **Augment it** | What does one extra number per vertex buy? | **Low-link** — one integer turns DFS into SCCs, bridges, articulation points, and 2-SAT |
| Specialize it | What if the graph were bipartite / planar / a DAG? | NP-hard problems become polynomial: independent set, colouring (4-colour), longest path on a DAG |
| Amortize it | What if you preprocessed the tree? | Binary lifting, Euler+RMQ, heavy-light — Θ(n log n) once, then cheap queries forever |

**Questions:**

1. Low-link uses "at most one back edge." Construct a graph where allowing two back edges makes the bridge test report a false negative, and explain the mechanism.
2. Under "invert it", max-flow and min-cut are duals. Given a final residual graph, describe exactly how to extract the minimum cut, and why the reachable set from `s` is the right one.
3. 2-SAT is Θ(V+E) and 3-SAT is NP-complete. What structural property of 2-clauses makes the implication graph work, and precisely where does it fail for 3-clauses?
4. Under "randomize it", Karger's algorithm contracts a random edge repeatedly. Derive why a *specific* min cut survives with probability ≥ 2/(n(n−1)), and what that implies about the repetition count.
5. Under "augment it", one integer per vertex (low-link) yields four algorithms. Name them, and say what each comparison against `disc` detects.
6. Shortest path is polynomial and longest path is NP-hard. What exactly breaks in the DP when you flip the objective — and why does restricting to a DAG restore tractability?
7. Under "specialize it", maximum independent set is polynomial on bipartite graphs via König. State the chain of equalities (independent set → vertex cover → matching) and say why it doesn't generalize.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Define low-link precisely, including the "at most one back edge" clause, and give the four conditions derived from it.
2. What is the condensation of a graph by its SCCs, and why is that useful?
3. Give the 2-SAT reduction and the satisfiability condition. Why doesn't it extend to 3-SAT?
4. State max-flow min-cut and describe how to extract the cut from the residual graph.
5. Name four problems that are polynomial on bipartite graphs or DAGs and NP-hard in general.
6. Why must these algorithms be written iteratively in Rust, and at what depth does the recursive version fail?

Build exercises:

- Implement iterative Tarjan SCC with an explicit `(vertex, child_index)` frame stack, and verify against a brute-force oracle (u and v are in the same SCC iff each reaches the other) on small random graphs. Then run it on a 200,000-vertex path graph — the recursive version aborts there, and yours shouldn't.
- Implement bridges and articulation points from the same DFS, property-tested against the brute-force definitions (remove the edge/vertex and count components). Then deliberately change one `low` update from `disc[w]` to `low[w]` and watch the oracle catch it.
- Implement 2-SAT via your SCC code and use it to solve a real scheduling instance with either/or constraints. Verifying satisfiability *and* extracting an assignment is the full exercise.
- Implement Dinic's max-flow (~80 lines), then use it for bipartite matching and confirm it matches Hopcroft-Karp. Then use the same code for project selection via maximum closure — one algorithm, three problems, which is the point of the topic.

## Open Questions

- How much faster is iterative Tarjan than `petgraph::tarjan_scc` on a large CSR graph, and does `petgraph` handle the deep-graph case?
- Dinic's versus push-relabel in Rust on realistic flow networks — the theory favours push-relabel asymptotically; does it hold?
- Binary lifting versus Euler-tour+sparse-table LCA: where's the crossover in query count, given the sparse table's Θ(n log n) memory?
- For min-cost max-flow, how much does capacity scaling actually buy on a realistic assignment instance versus plain successive-shortest-paths?
- Is there a good Rust ILP/SAT solver binding worth reaching for when a problem turns out NP-hard, and how large an instance is practical?

## References

- Tarjan, "Depth-First Search and Linear Graph Algorithms" (1972) — SCCs, bridges, articulation points, all from one DFS. The paper that established low-link as the central idea.
- Aspvall, Plass & Tarjan (1979) — the 2-SAT-via-SCC reduction, in two pages.
- Bender & Farach-Colton, "The LCA Problem Revisited" (2000) — the Euler-tour reduction from LCA to RMQ and back.
- Hopcroft & Karp (1973) — Θ(E√V) bipartite matching; Dinic (1970) for the general flow version.
- CLRS ch. 22.5 (SCC), 26 (max-flow, matching) — with the max-flow min-cut proof.
- [CP-Algorithms: Graph](https://cp-algorithms.com/#graphs) — the most complete practical catalogue of these algorithms with working code.
- Related in this repo: [Graph Traversal](../graph-traversal/learning.md) (DFS timestamps — the foundation, and the measured recursion limit), [Graph Representations](../graph-representations/learning.md) (reverse CSR for Kosaraju), [Shortest Paths](../shortest-paths/learning.md) (Bellman-Ford inside min-cost flow), [Range Query Structures](../range-query-structures/learning.md) (sparse tables for LCA; segment trees for heavy-light), [Disjoint Set Union](../disjoint-set-union/learning.md) (offline LCA, and the offline-connectivity trick).
