# Minimum Spanning Trees — Learning Notes

## Mental Model

**An MST is the cheapest way to keep a graph connected.** Given a connected weighted graph, find the subset of edges that touches every vertex, has no cycles, and minimizes total weight. It always has exactly V−1 edges — that's forced, since fewer disconnects and more creates a cycle.

What makes this topic worth studying isn't the algorithms — there are two, both short — it's that **MST is the cleanest case where greedy is provably optimal.** Most greedy algorithms are wrong and the burden is on you to prove otherwise; here the proof is a single lemma that both algorithms are instances of:

> **The cut property.** For any partition of the vertices into two non-empty sets, the **minimum-weight edge crossing that cut is in some MST.**

The proof is an exchange argument in two sentences: take an MST not containing that minimum crossing edge `e`; adding `e` creates a cycle; that cycle must cross the cut a second time at some edge `f` with `w(f) ≥ w(e)`; swap `f` for `e` and you have a spanning tree that is no heavier. Therefore a minimum crossing edge is always safe to take.

Both algorithms are just different ways of choosing cuts:

- **Prim** grows one tree, repeatedly taking the cheapest edge crossing the cut between "in the tree" and "not in the tree."
- **Kruskal** sorts all edges and takes each one that joins two different components — which is the cheapest edge across the cut separating those components.

The dual is worth knowing too: the **cycle property** — the *heaviest* edge on any cycle is in no MST — which is the same lemma read backwards and is what justifies "reverse-delete" style reasoning.

Measured on this machine, a connected graph with 200,000 vertices and 1,000,000 edges, both producing identical total weight (2,460,957):

| Algorithm | n = 20,000 / 100k edges | n = 200,000 / 1M edges |
| --- | --- | --- |
| **Kruskal (sort + DSU)** | **1.31 ms** | **14.63 ms** |
| Prim (binary heap, lazy) | 7.19 ms | 108.02 ms |

**Kruskal was 7.4× faster** — which is the opposite of the usual textbook framing that Prim wins on dense graphs. The reason is that Kruskal's two components are both extremely fast in practice: `sort_unstable` is a highly tuned pdqsort, and [DSU](../disjoint-set-union/learning.md) with both optimizations is ~8 ns per operation on a flat array. Prim's lazy heap, by contrast, pushes an entry per edge and discards most of them.

## The Invariant

**Kruskal:** the accepted edge set is always a **forest** (an acyclic subgraph), and every accepted edge is the minimum-weight edge joining its two components at the moment it's accepted.

**Prim:** the accepted set is always a **single tree** containing a subset S of the vertices, and it is a minimum spanning tree *of the subgraph induced by S*.

Both maintain the same global guarantee, which is the real invariant:

> The current edge set is a subset of some minimum spanning tree.

The cut property is what preserves it: each algorithm only ever adds a minimum edge across some cut, and that edge is by the lemma in *some* MST extending the current set.

Two consequences people trip on:

- **If edge weights are all distinct, the MST is unique.** With ties, multiple MSTs can exist — all with the same total weight. So "the MST" is only well-defined up to ties, and two correct implementations can output different edge sets. Comparing implementations must compare *weights*, not edge lists.
- **On a disconnected graph there is no spanning tree** — you get a minimum spanning **forest**, one tree per component. Both algorithms produce it naturally: Kruskal simply runs out of joining edges, and Prim must be restarted from each unvisited vertex.

## Mechanics

### Kruskal — sort, then union

```rust
edges.sort_unstable_by_key(|e| e.weight);
let mut dsu = Dsu::new(n);
let mut total = 0u64;
let mut taken = 0;
for &(u, v, w) in &edges {
    if dsu.union(u, v) {              // false ⇒ same component ⇒ would form a cycle
        total += w as u64;
        taken += 1;
        if taken == n - 1 { break; }  // early exit: the tree is complete
    }
}
```

The whole algorithm is "sort, then ask DSU whether this edge joins two components." The `union` return value *is* the cycle check — which is why ignoring it is the classic bug ([DSU](../disjoint-set-union/learning.md)).

Cost: Θ(E log E) for the sort, plus Θ(E · α(V)) for the DSU operations. **The sort dominates**, so if the edges arrive already sorted (or the weights are small integers permitting a radix sort) Kruskal becomes effectively Θ(E · α(V)).

### Prim — grow one tree with a priority queue

```rust
let mut in_mst = vec![false; n];
let mut pq = BinaryHeap::new();
in_mst[0] = true;
for (v, w) in g.neighbours(0) { pq.push(Reverse((w, v))); }
while let Some(Reverse((w, v))) = pq.pop() {
    if in_mst[v as usize] { continue; }        // ← stale entry (lazy deletion)
    in_mst[v as usize] = true;
    total += w as u64;
    for (u, w2) in g.neighbours(v) {
        if !in_mst[u as usize] { pq.push(Reverse((w2, u))); }
    }
}
```

This is **lazy Prim** — it pushes an entry per edge and skips vertices already in the tree, exactly the lazy-deletion pattern from [Dijkstra](../shortest-paths/learning.md) and [heaps](../heaps-and-priority-queues/learning.md). The heap therefore holds Θ(E) entries, which is the practical difference from Dijkstra's measured 2.0×V and a large part of why Prim lost the benchmark here.

The **eager** variant keeps one entry per vertex (the cheapest known edge into the tree) and needs `decrease_key`, giving Θ(E log V) with a Θ(V) heap. It's more code and needs an indexed heap.

Notice the structural similarity to Dijkstra: same loop, same lazy deletion, and the **only difference is the key** — Prim uses the edge weight `w`, Dijkstra uses the accumulated distance `d + w`. That one-token difference is the whole distinction between "cheapest edge to attach" and "cheapest path from the source", and it's worth internalizing because it makes both algorithms one thing.

### Why Kruskal won, against the textbook

The standard advice is "Prim for dense, Kruskal for sparse", justified by Θ(E log V) versus Θ(E log E). Since `log E ≤ 2 log V`, those are the same to within a constant — the advice is asymptotically vacuous, and the measurement shows what actually decides it:

- **Kruskal's constants are exceptional.** `sort_unstable` on 1M `(u32,u32,u32)` tuples is a few milliseconds of highly-tuned, cache-friendly, branch-predictable work. DSU is a flat `Vec<u32>` at ~8 ns/op with essentially no memory overhead.
- **Lazy Prim pushes Θ(E) heap entries** and discards most — each push and pop is a sift through a heap far larger than V, with poor locality.
- **Kruskal exits early** once V−1 edges are taken, often long before consuming all edges.

Prim's genuine advantages are elsewhere: it needs only the graph, not a materialized sorted edge list (better when E is huge or edges stream), it works naturally on **implicit** graphs where enumerating all edges is impossible, and the eager variant with an adjacency matrix is Θ(V²) — which beats sorting when the graph is genuinely dense.

### Borůvka — the one that parallelizes

Each round, every component simultaneously picks its own cheapest outgoing edge; add them all and contract. The component count at least halves each round, so it finishes in Θ(log V) rounds of Θ(E) work — Θ(E log V) total.

It's the oldest MST algorithm (1926) and the most relevant one today, because every round's choices are **independent**: it's the basis of parallel and distributed MST, and of the Θ(E α(V)) randomized algorithms. Its one correctness subtlety is that ties must be broken **consistently** (e.g. by edge index), or two components can each pick "the other direction" of the same tie and form a cycle.

## Complexity

| Algorithm | Time | Space | Notes |
| --- | --- | --- | --- |
| **Kruskal** | Θ(E log E) sort + Θ(E α(V)) | Θ(V + E) | Sort dominates; Θ(E α(V)) if pre-sorted |
| Kruskal + radix sort | **Θ(E · d + E α(V))** | Θ(V + E) | Small integer weights |
| **Prim (lazy, binary heap)** | Θ(E log E) | **Θ(E) heap** | What most people write |
| Prim (eager, indexed heap) | Θ(E log V) | Θ(V) heap | Needs `decrease_key` |
| Prim (array, dense) | **Θ(V²)** | Θ(V) | Beats the heap when E ≈ V² |
| Prim (Fibonacci heap) | Θ(E + V log V) | Θ(V) | Loses in practice |
| **Borůvka** | Θ(E log V) | Θ(V + E) | **Parallelizable** |
| Karger-Klein-Tarjan | **Θ(E) expected** | Θ(E) | Randomized; theoretical |

**Where the table misleads.** Kruskal and lazy Prim have the *same* Θ(E log E), and measured they differ by **7.4×**. The bound is identical; the constants are not. Kruskal's log factor lives inside `sort_unstable` (sequential, vectorized, cache-friendly); Prim's lives inside a heap holding Θ(E) entries with scattered access. When a complexity table shows two rows as equal, that's the signal to measure rather than to pick by reputation.

Also note the Fibonacci-heap row is the same trap as in [shortest paths](../shortest-paths/learning.md): asymptotically superior, practically slower.

## Rust Implementation

```rust
// Kruskal: the default. Returns total weight and the chosen edges.
pub fn kruskal(n: usize, mut edges: Vec<(u32, u32, u32)>) -> (u64, Vec<(u32, u32, u32)>) {
    edges.sort_unstable_by_key(|e| e.2);
    let mut dsu = Dsu::new(n);
    let (mut total, mut tree) = (0u64, Vec::with_capacity(n.saturating_sub(1)));
    for e in edges {
        if dsu.union(e.0, e.1) {                 // the return value IS the cycle check
            total += e.2 as u64;
            tree.push(e);
            if tree.len() == n - 1 { break; }
        }
    }
    (total, tree)                                 // tree.len() < n-1 ⇒ graph was disconnected
}

// Minimum spanning FOREST on a possibly-disconnected graph: identical code,
// just don't assume tree.len() == n - 1. dsu.sets() gives the component count.

// Maximum spanning tree: sort descending. The cut property dualizes cleanly.
edges.sort_unstable_by_key(|e| Reverse(e.2));
```

**Float weights** are the usual hazard: `sort_unstable_by(|a, b| a.2.partial_cmp(&b.2).unwrap())` panics on `NaN`. Use `total_cmp`, `ordered_float::NotNan`, or integer-scaled weights.

**Ties make the MST non-unique**, so tests must compare total weight, not the edge set — or break ties deterministically (by `(weight, u, v)`) so the output is reproducible.

**Crates:** `petgraph::algo::min_spanning_tree` (Kruskal-based). For anything performance-sensitive the 15 lines above plus your own DSU will beat it, and you'll want control over the sort anyway.

## Use Cases

- **Network design** — laying cable, pipelines, or circuit traces to connect all sites at minimum cost. This is the problem MST was invented for (Borůvka, 1926, for electrifying Moravia).
- **Clustering** — build the MST, then delete the k−1 heaviest edges to get k clusters. This is **single-linkage hierarchical clustering**, and the MST computes the whole dendrogram in one pass.
- **Image segmentation** — Felzenszwalb-Huttenlocher merges pixel regions in MST order with an adaptive threshold; the DSU-driven implementation is essentially Kruskal.
- **Approximation algorithms** — the MST is a 2-approximation for metric TSP (walk the tree twice), and Christofides improves it to 1.5 using an MST plus a matching.
- **Maze generation** — a random-weight MST of a grid graph is a uniform-ish spanning tree, i.e. a perfect maze.
- **Cycle detection while building** — Kruskal's `union` return value is exactly "would this edge close a cycle?", useful independently of MSTs.
- **Bottleneck paths** — the MST path between two vertices minimizes the *maximum* edge weight on the path (the minimax path), which is a different and useful guarantee.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Kruskal (sort + DSU)** | **The default.** Measured 7.4× faster; simplest code; gives the forest for free |
| Kruskal + radix sort | Small integer weights and E is large — removes the log factor |
| Kruskal, pre-sorted | Edges already sorted — Θ(E α(V)) |
| **Prim (lazy heap)** | Edges aren't materialized; graph is implicit; E ≫ what you can sort |
| Prim (array, Θ(V²)) | Dense graph with an adjacency matrix, V small |
| **Borůvka** | Parallel or distributed MST |
| DSU alone | You only need connectivity, not the tree |
| Nothing | The graph is a tree already, or you need a *shortest-path* tree — that's [Dijkstra](../shortest-paths/learning.md), not an MST |

## Pitfalls in Depth

### Pitfall: Confusing an MST with a shortest-path tree

- **What goes wrong:** An MST is computed and used for routing — "the cheapest network to connect everything" gets mistaken for "the cheapest route between any two points." The MST path between two vertices can be arbitrarily longer than their shortest path. A network built on this reasoning has minimum total cable and terrible latency between specific pairs.
- **Why it happens (the mechanism):** Both are greedy tree-building algorithms over weighted graphs with nearly identical code — Prim and Dijkstra differ only in the heap key (`w` versus `d + w`). That one token is the difference between minimizing *total tree weight* and minimizing *distance from a source*, and the algorithms otherwise look the same, so the outputs get conflated.
- **How to handle it in production, and why that works:** Name the objective before choosing. "Connect everything as cheaply as possible" → MST. "Get from A to everywhere as cheaply as possible" → Dijkstra's shortest-path tree. If you need *both* properties you can't have them from one tree — that's a spanner or a Steiner-tree problem. Note what the MST *does* guarantee for paths: its path between two vertices minimizes the **maximum** edge weight (the bottleneck), which is genuinely useful and often what's actually wanted for capacity-limited networks.
- **Trade-offs of the fix:** Shortest-path trees don't minimize total weight, so a routing-optimal network costs more to build. The Steiner-tree formulation (allowing extra intermediate nodes) is NP-hard, so real network design uses approximations — the MST being a common starting point.

### Pitfall: Ignoring `union`'s return value

- **What goes wrong:** Kruskal adds every edge it examines to the result instead of only those that actually merged two components. The output has cycles, more than V−1 edges, and a weight far above the true minimum. Alternatively the loop runs to completion over all E edges instead of exiting at V−1, wasting time on a graph where the tree completes early.
- **Why it happens (the mechanism):** `union(a, b)` on two vertices already in the same component is a legitimate silent no-op — that *is* the cycle test. If the caller ignores the boolean, there's no other signal distinguishing "merged" from "already connected", and the resulting edge list looks superficially plausible.
- **How to handle it in production, and why that works:** Have `union` return `bool` and mark it `#[must_use]`, then gate acceptance on it: `if dsu.union(u, v) { take(e); }`. Track the accepted count and break at V−1. Assert at the end that `tree.len() == n - 1` for a graph you believe is connected — if it isn't, you've learned something real about your input.
- **Trade-offs of the fix:** None meaningful. `#[must_use]` occasionally forces `let _ =` where you genuinely only want the side effect, which is a small price for making the cycle check impossible to skip.

### Pitfall: Assuming the MST is unique

- **What goes wrong:** A test compares the returned edge list against a fixed expected list and fails intermittently — after a compiler upgrade, an input reordering, or a switch from `sort` to `sort_unstable`. Two runs produce different edge sets with the same total weight, and it reads like a nondeterminism bug in the algorithm.
- **Why it happens (the mechanism):** The MST is unique **only when all edge weights are distinct**. With ties, several minimum spanning trees exist, all optimal. `sort_unstable` doesn't preserve the input order of equal elements ([sorting](../sorting/learning.md)), so which tied edge is examined first — and therefore which lands in the tree — varies.
- **How to handle it in production, and why that works:** Compare **total weight**, not edge sets, in tests. If you need a reproducible edge set, make the sort key total by adding tiebreakers: `sort_unstable_by_key(|e| (e.weight, e.u, e.v))`. That's more robust than switching to a stable sort, because it survives changes in input order too.
- **Trade-offs of the fix:** A composite key makes comparisons slightly more expensive. And there are legitimate reasons to want a *specific* MST among the ties (e.g. minimizing the number of edges of a certain type), which needs a deliberately designed tiebreaker rather than an arbitrary one.

### Pitfall: Lazy Prim's Θ(E) heap on a dense graph

- **What goes wrong:** Lazy Prim pushes an entry for every edge examined, so on a graph with 1,000,000 edges the heap can hold on the order of E entries rather than V. Memory balloons and every sift walks a much deeper heap. Measured, lazy Prim took **108.02 ms against Kruskal's 14.63 ms** on the same 200,000-vertex, 1,000,000-edge graph.
- **Why it happens (the mechanism):** A binary heap has no `decrease_key`, so the lazy pattern pushes a duplicate whenever a cheaper edge to a vertex is found and filters stale entries at pop time. Unlike [Dijkstra](../shortest-paths/learning.md) — where re-pushes only happen on distance *improvements*, measured at just 2.0×V — Prim pushes on every examined edge out of every newly-added vertex, which is genuinely Θ(E).
- **How to handle it in production, and why that works:** Use Kruskal, which was measured 7.4× faster and is less code. If Prim is required (implicit graph, unmaterialized edges), use the **eager** variant with an indexed priority queue: one entry per vertex holding its cheapest known connecting edge, giving a Θ(V) heap and Θ(E log V) total. For a dense graph with a matrix, the Θ(V²) array version has no heap at all and excellent locality.
- **Trade-offs of the fix:** The eager variant needs an indexed heap (`priority-queue` crate or a hand-rolled position map), which adds a second invariant that must be maintained through every swap — the same complexity trade discussed in [heaps](../heaps-and-priority-queues/learning.md). Kruskal requires materializing and sorting all edges, which lazy Prim doesn't.

### Pitfall: Silently producing a forest and calling it a tree

- **What goes wrong:** The graph is disconnected — a component nobody knew about, a filtered edge set, a parsing bug — and the algorithm returns fewer than V−1 edges. Downstream code assumes a spanning tree, so a traversal reaches only part of the graph, or a "total cost" is reported for connecting a network that isn't actually connected.
- **Why it happens (the mechanism):** Both algorithms degrade gracefully into computing a minimum spanning *forest*: Kruskal simply runs out of joining edges, and Prim exhausts its heap after covering one component. Neither raises an error, because a forest is the mathematically correct answer for a disconnected input — the failure is that the caller assumed connectivity.
- **How to handle it in production, and why that works:** Check `tree.len() == n - 1` (or `dsu.sets() == 1`) and treat a shortfall as a domain-level signal rather than an error to swallow — it tells you the input graph has multiple components, which is usually worth surfacing. For Prim, loop over all vertices and restart from each unvisited one to get the full forest, since a single run only covers the start vertex's component.
- **Trade-offs of the fix:** Sometimes a forest is genuinely what you want (clustering deliberately produces one by cutting edges), so the check should be an assertion about *your* expectations rather than a hard error inside the algorithm.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if edges were added over time? | **Dynamic MST** — maintain under insertions; link-cut trees give Θ(log n) updates |
| Batch it | What if you processed all components at once? | **Borůvka** — every component picks simultaneously; Θ(log V) rounds |
| Approximate it | What if near-minimum sufficed? | Spanners: (1+ε) stretch with far fewer edges than an all-pairs guarantee needs |
| Randomize it | What if you sampled edges first? | **Karger-Klein-Tarjan** — Θ(E) *expected* MST via random sampling and verification |
| Externalize it | What if edges exceeded RAM? | External Kruskal — the bottleneck is the sort, so external merge sort applies directly |
| **Parallelize it** | Where's the independence? | **Borůvka** again — each component's choice is independent, so it's the parallel/distributed MST algorithm |
| **Invert it** | What if you wanted the *maximum* spanning tree? | Sort descending — the cut property dualizes; used in maximum-likelihood tree inference |
| Augment it | What does the MST tell you about *paths*? | **Bottleneck/minimax paths** — the MST path minimizes the maximum edge weight |
| **Specialize it** | What if weights were small integers? | Radix-sort the edges — removes the log factor, leaving Θ(E α(V)) |
| Amortize it | What if you deleted the heaviest edges? | **Single-linkage clustering** — one MST gives the whole hierarchy of k-clusterings |

**Questions:**

1. State the cut property and prove it with the exchange argument. Then show that both Prim and Kruskal are instances of it, naming the cut each uses.
2. Measured, Kruskal beat lazy Prim by 7.4× despite identical Θ(E log E) bounds. Decompose the gap into its three causes, and predict what happens to the ratio if the edges arrive pre-sorted.
3. Under "invert it", a maximum spanning tree comes from sorting descending. Prove the cut property's dual (the cycle property) and explain why the same code works.
4. Under "augment it", the MST path minimizes the maximum edge weight between two vertices. Prove it, then say why that's the right objective for a capacity-limited network and the wrong one for latency.
5. Borůvka requires consistent tie-breaking or components can form a cycle. Construct the two-component example where inconsistent tie-breaking fails.
6. Under "amortize it", deleting the k−1 heaviest MST edges gives k clusters. Explain why that's exactly single-linkage clustering, and what property of the MST makes one pass sufficient for all k.
7. Prim and Dijkstra differ only in the heap key (`w` vs `d + w`). Explain what each key is optimizing, and construct a graph where the two trees differ maximally.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the cut property and use it to justify Kruskal's acceptance rule in one sentence.
2. Give the measured Kruskal-vs-Prim numbers at V = 200,000 and the three reasons for the gap.
3. Why does an MST always have exactly V−1 edges, and what does it mean if your result has fewer?
4. When is the MST unique? What must tests compare instead of edge sets?
5. Prim vs Dijkstra: give the one-token difference and what each optimizes.
6. You need to connect 10,000 sites at minimum cable cost, and separately you need low latency between two head offices. Are those the same problem?

Build exercises:

- Implement both Kruskal (sort + your DSU from Stage 4) and lazy Prim, assert they produce the same total weight on random graphs, and reproduce the 7.4× gap at V = 200,000. Then instrument Prim to count heap pushes and confirm it's Θ(E) rather than Dijkstra's measured 2.0×V.
- Implement the eager Prim variant with an indexed heap and measure it against both. This is the cleanest way to see how much of Prim's loss was the lazy heap rather than the algorithm.
- Implement Borůvka with consistent tie-breaking, verify it against Kruskal, then deliberately break the tie-breaking and find the input that produces a cycle.
- Use an MST for single-linkage clustering: build it on a set of 2-D points, then sweep k from 1 to 20 by deleting the heaviest edges, and plot the clusters. One MST giving every k is the payoff.

## Open Questions

- Does the Kruskal advantage hold on a *dense* graph (E ≈ V²), where the sort has far more to do? Measure at V = 5,000 with E = 12M.
- How much does radix-sorting the edges (small integer weights) buy over `sort_unstable` for Kruskal at E = 10⁷?
- Eager Prim with an indexed heap versus Kruskal — does it close the 7.4× gap, and what's the crossover density?
- Borůvka in Rust with rayon: what parallel speedup is achievable on 200k vertices?
- Does `petgraph::algo::min_spanning_tree` cost meaningfully more than the 15-line hand-rolled Kruskal?

## References

- Borůvka (1926) — the first MST algorithm, motivated by electrifying Moravia, and the one that parallelizes.
- Kruskal, "On the shortest spanning subtree of a graph" (1956); Prim, "Shortest connection networks" (1957) — both short.
- CLRS ch. 23 — the cut property with a careful proof, plus both algorithms and their analyses.
- Karger, Klein & Tarjan, "A randomized linear-time algorithm to find minimum spanning trees" (1995) — Θ(E) expected, and a good demonstration of the "randomize it" lens.
- Felzenszwalb & Huttenlocher, "Efficient Graph-Based Image Segmentation" (2004) — MST-style merging in a real system.
- Related in this repo: [Disjoint Set Union](../disjoint-set-union/learning.md) (Kruskal's engine, and why the `union` return value matters), [Heaps & Priority Queues](../heaps-and-priority-queues/learning.md) (Prim's engine, and lazy deletion), [Shortest Paths](../shortest-paths/learning.md) (the one-token difference from Prim), [Sorting](../sorting/learning.md) (Kruskal's dominant cost, and why ties make the MST non-unique), the Stage 6 topic *Greedy Algorithms*, not yet written — MST is its teaching case for exchange arguments.
