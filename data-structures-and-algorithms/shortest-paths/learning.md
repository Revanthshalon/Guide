# Shortest Paths — Learning Notes

## Mental Model

**Every shortest-path algorithm is the same operation applied in a different order.** That operation is **relaxation**:

```
if dist[u] + w(u,v) < dist[v] { dist[v] = dist[u] + w(u,v); prev[v] = u; }
```

Nothing else happens. The algorithms differ only in *which edge to relax next*, and that single choice determines what they cost and what they can handle:

| Algorithm | Relaxation order | Cost | Handles |
| --- | --- | --- | --- |
| BFS | Frontier order (queue) | Θ(V + E) | Unweighted only |
| **Dijkstra** | **Always the closest unfinished vertex** | Θ(E log V) | Non-negative weights |
| Bellman-Ford | **Every edge, V−1 times** | Θ(V·E) | **Negative weights**; detects negative cycles |
| DAG relaxation | Topological order | **Θ(V + E)** | Any weights, but acyclic only |
| Floyd-Warshall | All pairs, via each intermediate | Θ(V³) | All pairs, negative weights |
| A\* | Closest by `dist + heuristic` | Θ(E log V), far less in practice | Non-negative + an admissible heuristic |

Dijkstra's ordering rule is the interesting one, and it rests on an assumption worth stating plainly: **if you always finalize the closest unfinished vertex, its distance can never improve later — because reaching it via any other vertex would mean going *further* first, and distances only grow.** That argument requires edges to be non-negative. A single negative edge breaks it, and Dijkstra doesn't detect the violation — it just returns wrong answers.

The practical payoff of using the right ordering is enormous. Measured on this machine, a connected graph with 200,000 vertices and 1,000,000 edges, both implementations producing identical results:

| Approach | Time |
| --- | --- |
| Dijkstra with a binary heap | **56.34 ms** |
| Dijkstra with an O(V²) linear scan | **78.92 s** |

**1,401×** — and at n = 20,000 it was 66×, so the gap grows with n exactly as Θ(E log V) versus Θ(V²) predicts.

## The Invariant

Dijkstra's correctness rests on one claim:

> When a vertex is **finalized** (popped with `d == dist[u]`), `dist[u]` is its true shortest-path distance and can never improve.

The proof is short and worth holding: suppose some shorter path to `u` exists. It must leave the finalized set at some vertex `x` not yet finalized. Then `dist[x] ≤` that path's length up to `x` `≤` the whole path's length `< dist[u]`. But we popped `u`, meaning `dist[u] ≤ dist[x]` — contradiction. **The step that fails with negative edges is "≤ the whole path's length"**: with a negative edge later on, a longer prefix can lead to a shorter total.

More generally, all of these algorithms maintain:

> `dist[v]` is the length of *some* path from the source to `v` (an upper bound on the true distance), and relaxation only ever decreases it. The algorithm terminates when no edge can be relaxed — which is exactly the condition `dist[u] + w(u,v) ≥ dist[v]` for every edge.

Bellman-Ford makes this literal: after `k` rounds of relaxing all edges, `dist[v]` is correct for every vertex whose shortest path uses at most `k` edges. Since a simple path has at most `V−1` edges, `V−1` rounds suffice — **and if a `V`-th round still improves something, a negative cycle is reachable.** That's not a bolt-on check; it falls out of the invariant.

## Mechanics

### Dijkstra, with lazy deletion

```rust
let mut dist = vec![u64::MAX; n];
let mut pq = BinaryHeap::new();
dist[src] = 0;
pq.push(Reverse((0u64, src as u32)));
while let Some(Reverse((d, u))) = pq.pop() {
    if d > dist[u as usize] { continue; }              // ← stale entry, discard
    for (v, w) in g.neighbours(u) {
        let nd = d + w as u64;
        if nd < dist[v as usize] {
            dist[v as usize] = nd;
            prev[v as usize] = u;
            pq.push(Reverse((nd, v)));                 // ← push a duplicate instead of decrease-key
        }
    }
}
```

The `if d > dist[u]` line is **lazy deletion**, from [heaps](../heaps-and-priority-queues/learning.md): rather than updating a vertex's priority in place (which a binary heap can't do without a side index), push a new entry and discard stale ones on pop.

The usual claim is that this makes the heap hold Θ(E) entries. **Measured, it doesn't.** On the 200,000-vertex, 1,000,000-edge graph:

- **391,050 pushes** — that's **2.0 × V**, not E (which is 2,000,000 arcs).
- **191,051 stale pops — 49% of all pops discarded.**

So half the pops are wasted work, but the heap stays proportional to V rather than E, because a vertex is only re-pushed when its distance actually *improves*, and on a random graph that happens about twice per vertex. The "heap grows to E" worry is a worst case, not the common case — worth knowing before adding an indexed heap to avoid it.

### The others, briefly

**Bellman-Ford** — relax every edge, V−1 times; a V-th improving round means a negative cycle:

```rust
for _ in 0..n - 1 {
    let mut changed = false;
    for &(u, v, w) in edges {
        if dist[u] != INF && dist[u] + w < dist[v] { dist[v] = dist[u] + w; changed = true; }
    }
    if !changed { break; }                              // early exit — usually far before V−1
}
// One more pass: anything that still improves is on/after a negative cycle.
```

The early exit matters — on typical graphs it converges in far fewer than V−1 rounds. **SPFA** (queue-based Bellman-Ford) only relaxes edges out of vertices whose distance changed; it's much faster in practice but still Θ(V·E) worst case and is defeatable by crafted input.

**DAG shortest paths** — topologically sort, then relax edges in that order. **Θ(V + E)**, handles negative weights, and beats Dijkstra. Whenever the graph is acyclic, this is the answer — scheduling, build critical paths, and most dynamic-programming-on-a-DAG problems are this in disguise.

**Floyd-Warshall** — all pairs, `Θ(V³)` on an adjacency matrix:

```
for k in 0..n { for i in 0..n { for j in 0..n {
    d[i][j] = d[i][j].min(d[i][k] + d[k][j]); } } }
```

The `k` loop must be **outermost** — it's the DP over "paths using only intermediates from `{0..k}`". Simple, cache-friendly, and genuinely the right tool for V ≤ ~500. For sparse graphs, running Dijkstra from every vertex (Θ(V·E log V)) beats it; Johnson's algorithm handles negative edges by reweighting with Bellman-Ford first.

**A\*** — Dijkstra with the priority `dist[v] + h(v)` where `h` estimates the remaining distance. If `h` is **admissible** (never overestimates) the result is optimal; if additionally **consistent** (`h(u) ≤ w(u,v) + h(v)`) then each vertex is finalized once, exactly as in Dijkstra. With `h ≡ 0` it *is* Dijkstra. A good heuristic (straight-line distance for maps) cuts the explored set dramatically without giving up optimality.

### Choosing

| Situation | Use |
| --- | --- |
| Unweighted | **BFS** — Θ(V+E) |
| Weights ∈ {0,1} | **0-1 BFS** — Θ(V+E), no heap |
| Small integer weights ≤ C | Dial's / bucket queue — Θ(E + VC) |
| Non-negative weights | **Dijkstra** — Θ(E log V) |
| **Acyclic** | **DAG relaxation** — Θ(V+E), any weights |
| Negative weights | Bellman-Ford / SPFA — Θ(V·E) |
| Need negative-cycle detection | Bellman-Ford |
| All pairs, dense or V ≤ ~500 | Floyd-Warshall — Θ(V³) |
| All pairs, sparse | Dijkstra from each vertex; Johnson's if negative |
| Point-to-point with geometry | **A\*** |
| Repeated queries, static graph | Precompute: contraction hierarchies, landmarks |

## Complexity

| Algorithm | Time | Space | Negative weights |
| --- | --- | --- | --- |
| BFS | Θ(V + E) | Θ(V) | n/a (unweighted) |
| 0-1 BFS | Θ(V + E) | Θ(V) | no |
| **Dijkstra (binary heap)** | **Θ((V + E) log V)** | Θ(V + heap) | **no** |
| Dijkstra (Fibonacci heap) | Θ(E + V log V) | Θ(V) | no — and slower in practice |
| DAG relaxation | **Θ(V + E)** | Θ(V) | **yes** |
| Bellman-Ford | Θ(V · E) | Θ(V) | **yes**, detects negative cycles |
| SPFA | Θ(V · E) worst, fast typical | Θ(V) | yes |
| Floyd-Warshall | Θ(V³) | Θ(V²) | yes (no negative cycles) |
| Johnson's | Θ(V·E log V) | Θ(V²) | yes |
| A\* | Θ(E log V) worst | Θ(V) | no |

**Where the table misleads.** The Fibonacci-heap row is the classic trap: Θ(E + V log V) is asymptotically better than the binary heap's Θ((V+E) log V), and it **loses in practice** — pointer-chasing, allocation, and cache behaviour swamp the improvement, as covered in [heaps](../heaps-and-priority-queues/learning.md). Use a binary heap with lazy deletion.

The other misleading entry is Dijkstra's log factor, which suggests the choice between Θ(E log V) and Θ(V²) is marginal. **Measured, it's 1,401×** at V = 200,000 — because Θ(V²) is 4×10¹⁰ operations against roughly 2×10⁷ for the heap version. Asymptotics understate this because the naive version is also *memory-bound*: each of its V scans touches the entire distance array.

## Rust Implementation

```rust
// Path reconstruction — record predecessors during relaxation, walk backwards after.
let mut path = vec![target];
let mut cur = target;
while let Some(p) = prev[cur as usize] { path.push(p); cur = p; }
path.reverse();

// Floats: never partial_cmp().unwrap(). Prefer integer-scaled weights.
// Distances in millimetres/milliseconds as u64 avoid the whole problem.

// A*: same loop, different key.
pq.push(Reverse((dist[start] + h(start), start)));
// ... pop by f = g + h, but compare against dist[] (g) for staleness.

// DAG shortest path: Θ(V+E), handles negative weights.
for u in topological_order {
    if dist[u] == INF { continue; }
    for (v, w) in g.neighbours(u) {
        if dist[u] + w < dist[v] { dist[v] = dist[u] + w; }
    }
}
```

**Use integer weights.** `f64` isn't `Ord`, so it can't go in a `BinaryHeap` without `ordered_float::NotNan`, and floating-point addition isn't associative — two equal-length paths can compare unequal, making results depend on traversal order. Scaling to integers (millimetres, milliseconds, cents) removes both problems.

**`u64::MAX` as infinity overflows** the moment you write `dist[u] + w` for an unreached `u`. Either guard with `if dist[u] == INF { continue; }` or use `u64::MAX / 2`.

**Crates:** `petgraph` (`dijkstra`, `bellman_ford`, `astar`, `floyd_warshall`), `pathfinding` (excellent for A\*, IDA\*, and implicit graphs).

## Use Cases

- **Routing and navigation** — Dijkstra or A\* on road networks; production systems precompute (contraction hierarchies) because query latency matters more than preprocessing.
- **Network routing protocols** — OSPF is Dijkstra; RIP is distance-vector, i.e. distributed Bellman-Ford (and its "count to infinity" problem is exactly a negative-cycle-free convergence issue).
- **Build systems and scheduling** — critical path through a DAG, which is DAG relaxation on negated weights (longest path).
- **Currency arbitrage** — take `−log(rate)` as the weight; a **negative cycle** is an arbitrage opportunity, and Bellman-Ford finds it. This is the canonical reason to care about negative-cycle detection.
- **Game pathfinding** — A\* with Manhattan or Euclidean heuristics on grids; jump-point search for uniform grids.
- **Dependency resolution with costs** — DAG relaxation.
- **Dynamic programming on DAGs** — most "minimum cost to reach state X" problems are shortest paths on an implicit DAG, and recognizing that gives you the algorithm for free.
- **Puzzle solving** — A\* over an implicit state space with an admissible heuristic (e.g. Manhattan distance for sliding puzzles).

## When to Use Which

| Reach for | When |
| --- | --- |
| **BFS** | Unweighted — don't reach for Dijkstra |
| **0-1 BFS** | Weights ∈ {0,1} |
| **DAG relaxation** | Graph is acyclic — Θ(V+E), beats Dijkstra, allows negatives |
| **Dijkstra + binary heap + lazy deletion** | Non-negative weights, single source — **the default** |
| **A\*** | Point-to-point *and* you have an admissible heuristic |
| **Bellman-Ford** | Negative weights, or you must *detect* negative cycles |
| SPFA | Negative weights, typical-case speed matters, input isn't adversarial |
| **Floyd-Warshall** | All pairs with V ≤ ~500, or dense |
| Johnson's | All pairs, sparse, negative weights |
| Contraction hierarchies / landmarks | Millions of queries on a static graph |

## Pitfalls in Depth

### Pitfall: Dijkstra with negative edge weights

- **What goes wrong:** Dijkstra is run on a graph containing a negative edge — a refund, a discount, a downhill segment, a `−log` rate. It returns confidently wrong distances with no error. The bug survives testing because negative edges are often rare in the data, and a wrong-but-plausible route is hard to spot.
- **Why it happens (the mechanism):** Dijkstra finalizes the closest unfinished vertex and never revisits it. That's justified only because reaching a vertex via any un-finalized vertex means travelling *at least* as far first — which requires every edge to be non-negative. One negative edge means a longer prefix can yield a shorter total, so a finalized vertex's distance can still improve, and Dijkstra has already moved on.
- **How to handle it in production, and why that works:** Check the weights. Negative weights on a DAG → **DAG relaxation** in Θ(V + E), which imposes no sign restriction because topological order already guarantees a vertex is finalized only after all its predecessors. Negative weights with cycles → Bellman-Ford, which relaxes every edge V−1 times and therefore never assumes finality. If negatives arise from a transformation you control (like `−log`), consider reweighting to non-negative instead (Johnson's algorithm does exactly this).
- **Trade-offs of the fix:** Bellman-Ford is Θ(V·E) — at V = 200,000 and E = 10⁶ that's 2×10¹¹ operations, thousands of times slower than Dijkstra's measured 56 ms. So paying for negative-weight support when you don't need it is very expensive; the right move is to *establish* non-negativity (assert it in debug builds) rather than defensively using the general algorithm.

### Pitfall: The O(V²) Dijkstra

- **What goes wrong:** Dijkstra is implemented with "scan all vertices to find the closest unfinished one" instead of a priority queue. It's correct and it's Θ(V²). Measured on 200,000 vertices with 1,000,000 edges: **78.92 s versus 56.34 ms — 1,401× slower**, producing byte-identical results. At 20,000 vertices it was only 66×, so this scales into the problem rather than announcing itself in tests.
- **Why it happens (the mechanism):** "Find the minimum" reads as a linear scan, and the textbook presentation often shows the array version first. The scan is Θ(V) per iteration and runs V times, so the total is Θ(V²) regardless of how sparse the graph is — the algorithm stops caring about E entirely, which is exactly backwards for a sparse graph where E ≪ V².
- **How to handle it in production, and why that works:** `BinaryHeap<Reverse<(dist, vertex)>>` with lazy deletion. Extraction becomes Θ(log V) instead of Θ(V), so the total is Θ((V + E) log V), and on a sparse graph that's a completely different growth curve.
- **Trade-offs of the fix:** For **dense** graphs (E ≈ V²) the array version is actually competitive — Θ(V²) versus Θ(V² log V) — and it has better locality and no heap. So the naive version isn't universally wrong, it's wrong for sparse graphs, which is nearly all real ones. Know which you have.

### Pitfall: Forgetting the staleness check

- **What goes wrong:** Lazy deletion is used (duplicates pushed on improvement) but the `if d > dist[u] { continue; }` guard is omitted. Vertices get processed multiple times, and each stale processing re-relaxes their neighbours with an outdated distance — pushing more entries and doing exponentially more work in bad cases. It still terminates and still gives the *right answer*, so it looks like a performance mystery rather than a bug.
- **Why it happens (the mechanism):** The guard looks like an optimization — "skip work we've already done" — rather than the load-bearing part of the design. Measured, **49% of all pops are stale**, so removing the check roughly doubles the vertices processed at minimum, and each of those re-pushes its improved neighbours, compounding.
- **How to handle it in production, and why that works:** Always pair lazy deletion with the staleness check, comparing the popped distance against the current `dist[]`. It's one line and it's what makes the "push duplicates" strategy sound: duplicates are permitted precisely because they're cheap to discard.
- **Trade-offs of the fix:** None — the check is a single comparison against an array you're about to read anyway. The alternative design (an indexed heap with real `decrease_key`) avoids duplicates entirely but adds a position map that must be updated on every heap swap, which is more code and a second invariant to break. Measured, the heap only reached 2.0×V pushes, so the indexed version solves a problem that mostly isn't there.

### Pitfall: Integer overflow on the infinity sentinel

- **What goes wrong:** `dist` is initialized to `u64::MAX` and the relaxation computes `dist[u] + w` for an unreached `u`. In release builds that wraps to a tiny number, so unreachable vertices appear to be at distance ~0 and the algorithm produces nonsense — or in debug builds it panics with "attempt to add with overflow", which at least fails loudly.
- **Why it happens (the mechanism):** `u64::MAX` is the natural spelling of "infinity", but it's also the value that overflows on any addition. The bug only fires when the graph is disconnected or when a vertex is popped before being reached — which random test graphs (usually connected) never exercise.
- **How to handle it in production, and why that works:** Guard the relaxation (`if dist[u] == INF { continue; }`), which is correct and explicit. Or use `u64::MAX / 2` as the sentinel so one addition can't wrap — a common competitive-programming idiom, though it leaves a value that isn't obviously "infinity" when debugging. Best of all, use `Option<u64>` or a dedicated enum where the type system prevents the arithmetic.
- **Trade-offs of the fix:** The guard is a branch in the hot loop (negligible, and well-predicted). `MAX / 2` is free but relies on a convention a reader must know. `Option<u64>` is the safest and costs a discriminant check plus, for a heap key, some ergonomics — usually worth it outside the hottest inner loops.

### Pitfall: Reaching for Dijkstra on an acyclic graph

- **What goes wrong:** A build-order cost, a project critical path, or a DP-on-a-DAG is solved with Dijkstra. It works, costs Θ(E log V) instead of Θ(V + E), and — worse — silently rejects the negative weights the problem may naturally have (for a *longest* path you negate weights, which Dijkstra can't handle at all).
- **Why it happens (the mechanism):** Dijkstra is the named shortest-path algorithm, so acyclicity goes unnoticed as an exploitable property. But a topological order already tells you the finalization sequence for free — once you process vertices in topological order, every predecessor of `v` has been finalized before `v` is reached, which is the exact guarantee Dijkstra's heap works to establish.
- **How to handle it in production, and why that works:** Check for acyclicity (you often know it from the domain: dependencies, schedules, layered state spaces). If acyclic, topologically sort ([graph traversal](../graph-traversal/learning.md)) and relax in that order: Θ(V + E), no priority queue, and negative weights are fine — which also gives you **longest path** by negating, something no general-graph algorithm can do in polynomial time.
- **Trade-offs of the fix:** You need the topological sort (Θ(V + E), so it's free asymptotically) and you must be *certain* the graph is acyclic — running DAG relaxation on a cyclic graph silently produces wrong answers. Kahn's algorithm detects the cycle as a by-product, so use it rather than assuming.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if you kept the whole shortest-path tree? | `prev[]` — path reconstruction; and the tree is what dynamic algorithms repair |
| Batch it | What if you had many sources? | Multi-source Dijkstra (seed all at 0) — one run, not k; Voronoi partition of the graph |
| Approximate it | What if 1.1× optimal were fine? | Weighted A\* (`f = g + ε·h`) — dramatically faster, bounded suboptimality |
| Randomize it | What if you sampled landmarks? | ALT heuristics — precomputed landmark distances give A\* a strong admissible `h` |
| Externalize it | What if the graph exceeded RAM? | Hierarchical routing; contraction hierarchies with a small overlay graph |
| **Parallelize it** | Where's the independence? | **Δ-stepping** — relax buckets of similar distance in parallel; the standard parallel Dijkstra |
| **Invert it** | What if you searched from the target? | Bidirectional Dijkstra — meet in the middle, ~half the explored set |
| Augment it | What does a heuristic per vertex buy? | **A\*** — Dijkstra plus `h`; with `h ≡ 0` it *is* Dijkstra |
| **Specialize it** | What if weights were tiny integers? | **Dial's algorithm** — bucket queue, Θ(E + VC), no comparisons |
| Amortize it | What if you preprocessed heavily? | **Contraction hierarchies** — hours of preprocessing, microsecond queries on continental road networks |

**Questions:**

1. Write Dijkstra's correctness proof, then identify the exact step that fails with a negative edge. Why does DAG relaxation not need that step?
2. Under "specialize it", Dial's algorithm uses `C+1` buckets instead of a heap. Derive the Θ(E + VC) bound and say when it beats Θ(E log V).
3. Measured, lazy deletion produced 2.0×V pushes and 49% stale pops — not the Θ(E) usually claimed. Explain why re-pushes are proportional to *improvements* rather than to edges, and construct a graph where it really would approach E.
4. A\* with `h ≡ 0` is Dijkstra; with a perfect `h` it walks straight to the target. What does *admissible* guarantee, what extra does *consistent* guarantee, and what breaks if `h` overestimates?
5. Under "invert it", bidirectional Dijkstra is subtler than bidirectional BFS — the stopping condition isn't "the frontiers touch". State the correct condition and explain why the naive one gives wrong answers.
6. Currency arbitrage becomes a negative cycle under `−log(rate)`. Show why the log transform turns a *product* condition into a *sum* condition, and which algorithm you must therefore use.
7. Under "parallelize it", Δ-stepping relaxes buckets of similar distance concurrently. What does Δ trade off, and what happens at Δ → ∞ and Δ → 0?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Write the relaxation operation, then state what distinguishes BFS, Dijkstra, Bellman-Ford, and DAG relaxation in one clause each.
2. Give the measured heap-vs-O(V²) numbers at V = 200,000 and explain why the ratio grew from 66× at V = 20,000.
3. State Dijkstra's finalization invariant and where the proof fails for negative edges.
4. Give the measured push count and stale-pop rate for lazy deletion, and say what each tells you.
5. Why is Bellman-Ford exactly V−1 rounds, and what does a V-th improving round mean?
6. Your graph is acyclic with some negative weights. Which algorithm, what complexity, and why can't you use Dijkstra?

Build exercises:

- Implement Dijkstra both ways — binary heap with lazy deletion, and the O(V²) scan — assert they produce identical distance arrays, then reproduce the 1,401× at V = 200,000. Instrument the heap version to count pushes and stale pops and confirm the 2.0×V / 49% figures.
- Implement Bellman-Ford with the early-exit and the V-th-round negative-cycle check, then use it to detect currency arbitrage on a small exchange-rate table under `−log(rate)`. Recovering the actual arbitrage cycle (not just detecting one) is the real exercise.
- Implement A\* on a grid with Manhattan distance and count expanded nodes against plain Dijkstra on the same instance. Then make the heuristic inadmissible (multiply by 1.5) and find an instance where it returns a suboptimal path.
- Implement DAG relaxation and use it for the critical path of a build graph, then negate the weights to get the longest path. Try the same on a graph with a cycle and observe the silent wrongness — that's why Kahn's cycle detection matters.

## Open Questions

- Where does an indexed heap (real `decrease_key`) beat lazy deletion on this machine, given the measured 2.0×V pushes? Possibly never for random graphs — but road networks have different structure.
- Δ-stepping in Rust with rayon: what speedup is achievable on 200k vertices, and what's the best Δ?
- Bidirectional Dijkstra on a road network — how much of the theoretical ~2× materializes once the stopping condition's overhead is included?
- How much do contraction hierarchies actually buy for repeated queries, and what's the preprocessing cost at continental scale?
- Does `petgraph::dijkstra` cost meaningfully more than a hand-rolled CSR version?

## References

- Dijkstra, "A note on two problems in connexion with graphs" (1959) — two and a half pages, and the algorithm is on the second.
- CLRS ch. 24–25 — Bellman-Ford, DAG relaxation, Dijkstra, Floyd-Warshall, Johnson's, with the proofs.
- Hart, Nilsson & Raphael, "A Formal Basis for the Heuristic Determination of Minimum Cost Paths" (1968) — A\*, admissibility, and consistency.
- Geisberger et al., "Contraction Hierarchies" (2008) — how production routing engines answer continental queries in microseconds.
- Meyer & Sanders, "Δ-stepping: A Parallel Single Source Shortest Path Algorithm" (1998).
- Related in this repo: [Graph Traversal](../graph-traversal/learning.md) (BFS and 0-1 BFS — often the right answer instead), [Heaps & Priority Queues](../heaps-and-priority-queues/learning.md) (lazy deletion, and why not Fibonacci heaps), [Graph Representations](../graph-representations/learning.md) (CSR — the inner loop's layout), [Complexity Analysis](../complexity-analysis/learning.md) (Θ(E log V) vs Θ(V²) measured at 1,401×), [Minimum Spanning Trees](../minimum-spanning-trees/learning.md) (the other greedy-on-graphs algorithm).
