# Graph Traversal — Learning Notes

## Mental Model

**BFS and DFS are the same algorithm.** There is one traversal:

```
put the start in the frontier
while the frontier is not empty:
    take a vertex out
    if already visited, skip; mark visited
    put its unvisited neighbours in
```

Take from the **front** and it's BFS. Take from the **back** and it's DFS. Take the *smallest* and it's [Dijkstra](../shortest-paths/learning.md). The container *is* the algorithm — everything else is identical, which is why [stacks & queues](../stacks-and-queues/learning.md) was a prerequisite and why "which frontier?" is the first question to ask about any search.

What each ordering buys is different in kind, not degree:

- **BFS explores in order of distance**, so the first time it reaches a vertex it has done so along a shortest path *in edges*. That gives shortest paths on unweighted graphs for free — and it is the only reason to prefer BFS for that job.
- **DFS explores one path to exhaustion before backtracking**, so it discovers *structure*: the recursion's entry and exit times reveal cycles, articulation points, topological order, and strongly connected components. None of these fall out of BFS.

The second thing to internalize is a Rust-specific hazard with a measured threshold. Recursive DFS uses the call stack as its frontier, and that stack is fixed-size. Measured on this machine with a path graph (depth = n):

| n | Recursive DFS | Explicit stack |
| --- | --- | --- |
| 100,000 | ✅ | ✅ |
| **200,000** | **aborts: `fatal runtime error: stack overflow`** | ✅ |
| 5,000,000 | — | ✅ |

The abort is not a panic — it can't be caught, and on a spawned thread (2 MB rather than 8 MB) the threshold is roughly a quarter of that. **If the depth is derived from input you don't control, recursive DFS is a denial-of-service vector**, exactly as with the recursive parsers in [stacks & queues](../stacks-and-queues/learning.md).

## The Invariant

> Every vertex is in exactly one of three states — **undiscovered**, **discovered (in the frontier)**, **finished** — and the state only ever moves forward. A vertex is pushed into the frontier at most once per discovery, and the traversal terminates because each vertex can be finished only once.

For **BFS** the invariant strengthens into the property that makes it useful:

> When BFS dequeues a vertex at distance *d*, every vertex at distance < *d* has already been dequeued, and every vertex currently in the queue is at distance *d* or *d+1*.

That "the queue holds at most two distinct distances" property is what proves BFS finds shortest paths in edges — and it is exactly what breaks when edges have weights, which is why Dijkstra needs a priority queue rather than a plain one.

For **DFS**, the useful invariant is about time:

> Each vertex has a discovery time `d[u]` and a finish time `f[u]`, and the intervals `[d[u], f[u]]` are properly **nested** — for any two vertices they're either disjoint or one contains the other.

This *parenthesis structure* is the foundation of every advanced DFS result. Edge `(u,v)` classifies by comparing the intervals: a **back edge** (v is an ancestor, still open) means a cycle; the reverse order of finish times is a topological order; and the low-link computations behind SCCs and bridges are all statements about these intervals.

## Mechanics

### The unified traversal

```rust
// Change VecDeque→Vec and pop_front→pop and this becomes DFS. Nothing else changes.
fn bfs(g: &Csr, start: u32) -> Vec<u32> {
    let mut dist = vec![u32::MAX; g.n];
    let mut q = VecDeque::new();
    dist[start as usize] = 0;
    q.push_back(start);
    while let Some(u) = q.pop_front() {
        for v in g.neighbours(u) {
            if dist[v as usize] == u32::MAX {          // mark on PUSH for BFS
                dist[v as usize] = dist[u as usize] + 1;
                q.push_back(v);
            }
        }
    }
    dist
}
```

### Mark on push, or mark on pop — a real decision

- **Mark on push** (as above): a vertex enters the frontier at most once, so the frontier is bounded by V. **Required for BFS**, because the first discovery is the shortest one and a later, longer discovery must not overwrite it.
- **Mark on pop**: a vertex may be pushed several times; you skip it if already visited when popped. The frontier can grow to Θ(E). This is what an explicit-stack DFS usually does — it's what makes the iterative version's visit order match the recursive one, and it's the same pattern as Dijkstra's lazy deletion.

Getting this wrong on BFS silently produces wrong distances; getting it wrong on DFS produces a correct traversal in an unexpected order.

### Iterative DFS, and the ordering subtlety

```rust
let mut stack = vec![start];
while let Some(u) = stack.pop() {
    if seen[u as usize] { continue; }                  // mark on POP
    seen[u as usize] = true;
    for v in g.neighbours(u) {
        if !seen[v as usize] { stack.push(v); }
    }
}
```

This visits the **last** neighbour first, mirroring the recursive version's order. If order matters (deterministic output, matching a reference implementation), push neighbours in reverse. If you need *finish* times — and every advanced DFS application does — a simple `pop` loop isn't enough: you need an explicit state machine that pushes a vertex twice (enter, then exit) or tracks a per-vertex child cursor.

### What DFS gives you that BFS cannot

| Result | Mechanism |
| --- | --- |
| **Cycle detection** (directed) | A back edge — an edge to a vertex still on the recursion stack |
| **Topological sort** | Reverse order of finish times |
| **Strongly connected components** | Tarjan's low-link, or Kosaraju's two passes with the reverse graph |
| **Bridges and articulation points** | Low-link versus discovery time |
| Path existence with small memory | The stack holds one path, not a whole frontier level |

The recurring tool is the **low-link value**: the earliest discovery time reachable from a vertex's subtree via at most one back edge. Comparing it against the discovery time detects cycles, bridges, articulation points, and SCC roots. Learning it once pays for all four (Stage 5's advanced topic).

### Topological sort — two ways

**Kahn's (BFS-flavoured):** repeatedly emit a vertex with in-degree 0 and decrement its neighbours' in-degrees. Detects cycles naturally — if fewer than V vertices are emitted, the remainder is a cycle. Iterative, so no stack risk, and it extends to lexicographically-smallest order (use a heap) and to parallel scheduling (all in-degree-0 vertices are independent).

**DFS-flavoured:** reverse order of finish times. Fewer moving parts, but needs cycle detection bolted on and carries the recursion-depth hazard.

Prefer Kahn's unless you already need finish times for something else.

### The traversal variants worth knowing

| Variant | Frontier | Use |
| --- | --- | --- |
| BFS | queue | Shortest path in **edges**; level order |
| DFS | stack / recursion | Structure: cycles, topo order, SCCs, bridges |
| **Multi-source BFS** | queue seeded with *all* sources at distance 0 | "Nearest of any source" — fire spread, distance-to-any-exit. Θ(V+E), not k separate BFS runs |
| **0-1 BFS** | `VecDeque`: push_front for weight 0, push_back for weight 1 | Shortest paths with weights in {0,1} in Θ(V+E) — no heap needed |
| **Bidirectional BFS** | two frontiers meeting in the middle | Roughly `b^(d/2)` instead of `b^d` — a large win on big state spaces |
| Iterative deepening | repeated depth-limited DFS | DFS memory with BFS optimality; standard in game search |
| **Direction-optimizing** | switch push↔pull when the frontier is large | Big constant-factor win on high-degree graphs (Ligra/GAP) |

Multi-source BFS and 0-1 BFS are the two most under-used: both solve problems people reach for Dijkstra on, in Θ(V+E) with no priority queue.

## Complexity

| Algorithm | Time | Space | Notes |
| --- | --- | --- | --- |
| BFS | Θ(V + E) | Θ(V) frontier + Θ(V) visited | Frontier ≤ V with mark-on-push |
| DFS (iterative) | Θ(V + E) | **Θ(E)** stack with mark-on-pop | Θ(V) if you mark on push |
| DFS (recursive) | Θ(V + E) | **Θ(depth) call stack** | **Aborts past ~100k depth** |
| Topological sort (Kahn) | Θ(V + E) | Θ(V) | Detects cycles for free |
| Connected components | Θ(V + E) | Θ(V) | Or [DSU](../disjoint-set-union/learning.md) over an edge stream |
| Multi-source BFS | Θ(V + E) | Θ(V) | Independent of source count |
| 0-1 BFS | Θ(V + E) | Θ(V) | Beats Dijkstra's Θ(E log V) |
| Bidirectional BFS | ~Θ(b^(d/2)) | ~Θ(b^(d/2)) | Only for point-to-point |

**Where the table misleads.** Θ(V + E) suggests traversal is cheap and uniform; in practice it is **memory-bound and irregular** — every neighbour visit is a random access into the visited array, and the access pattern depends entirely on the graph's structure rather than on your code. Measured in [graph representations](../graph-representations/learning.md), the same Θ(V + E) BFS ran 1.76× faster purely by switching from `Vec<Vec<_>>` to CSR. Two implementations with identical bounds routinely differ by 2× or more, and the difference is layout, not algorithm.

The recursive-DFS space row is the one that bites: Θ(depth) sounds benign until depth is adversary-controlled and the failure mode is an uncatchable abort.

## Rust Implementation

```rust
// Multi-source BFS: seed the queue with every source. Still Θ(V+E).
let mut dist = vec![u32::MAX; n];
let mut q = VecDeque::new();
for &s in sources { dist[s as usize] = 0; q.push_back(s); }
while let Some(u) = q.pop_front() { /* identical body */ }

// 0-1 BFS: a deque replaces the heap entirely.
let mut dist = vec![u32::MAX; n];
let mut dq = VecDeque::from([start]);
dist[start as usize] = 0;
while let Some(u) = dq.pop_front() {
    for (v, w) in g.neighbours(u) {                       // w ∈ {0, 1}
        let nd = dist[u as usize] + w;
        if nd < dist[v as usize] {
            dist[v as usize] = nd;
            if w == 0 { dq.push_front(v) } else { dq.push_back(v) }
        }
    }
}

// Kahn's topological sort — iterative, cycle-detecting, no stack risk.
let mut indeg = vec![0u32; n];
for u in 0..n as u32 { for v in g.neighbours(u) { indeg[v as usize] += 1; } }
let mut q: VecDeque<u32> = (0..n as u32).filter(|&u| indeg[u as usize] == 0).collect();
let mut order = Vec::with_capacity(n);
while let Some(u) = q.pop_front() {
    order.push(u);
    for v in g.neighbours(u) {
        indeg[v as usize] -= 1;
        if indeg[v as usize] == 0 { q.push_back(v); }
    }
}
if order.len() != n { /* the remaining vertices form a cycle */ }
```

**Write traversal against a neighbour function**, not a concrete structure, so the same code runs over CSR, a grid, or a generated state space:

```rust
fn bfs<I: Iterator<Item = u32>>(n: usize, start: u32, nbrs: impl Fn(u32) -> I) -> Vec<u32> { … }
```

**Visited-set choice matters.** For a bounded vertex space use `Vec<bool>` (one byte, indexed directly) or a bitset (one bit — 8× less memory, and at V = 10⁷ that's the difference between 10 MB and 1.25 MB, which can decide whether the working set fits in cache). For an unbounded implicit state space, a `HashSet` is unavoidable, and that hash cost usually dominates the traversal.

**Crates:** `petgraph` (`Bfs`, `Dfs`, `toposort`, `tarjan_scc`), `fixedbitset` (visited sets), `pathfinding` (BFS/DFS/IDA\*/bidirectional over implicit graphs — genuinely good for puzzle-style problems).

## Use Cases

- **Shortest path in unweighted graphs** — BFS, and nothing else is needed. Social-network degrees of separation, minimum moves in a puzzle, fewest hops.
- **Flood fill / connected regions** — grid BFS or DFS; image segmentation, "islands" problems, percolation.
- **Cycle detection** — DFS back edges for directed graphs; for undirected, DFS with a parent check or [DSU](../disjoint-set-union/learning.md).
- **Build and dependency ordering** — topological sort; `cargo`'s crate ordering, task schedulers, spreadsheet recalculation.
- **Deadlock detection** — a cycle in the wait-for graph.
- **Garbage collection** — mark-and-sweep is a traversal from the roots; the tricolour abstraction is exactly the three-state invariant above.
- **Web crawling** — BFS from seeds, with the frontier bounded and the visited set the interesting engineering problem at scale.
- **Puzzle and state-space search** — bidirectional BFS or IDA\* over an implicit graph.
- **Multi-source distance fields** — "distance to nearest hospital/exit/enemy" in one Θ(V+E) pass.

## When to Use Which

| Reach for | When |
| --- | --- |
| **BFS** | Shortest path in **edges**; level order; multi-source distance |
| **DFS (iterative)** | Structure — cycles, topo order, SCCs, bridges |
| DFS (recursive) | Depth is provably small (a balanced tree, a bounded DAG) |
| **Kahn's toposort** | Ordering a DAG — iterative and detects cycles for free |
| **Multi-source BFS** | "Nearest of any of these k sources" — one pass, not k |
| **0-1 BFS** | Weights are only 0 and 1 — beats Dijkstra, no heap |
| Dijkstra | Arbitrary non-negative weights |
| Bidirectional BFS | Point-to-point in a large space, both directions available |
| Iterative deepening | Huge/infinite depth, memory-constrained, need optimality |
| DSU | Connectivity only, edges arrive as a stream |

## Pitfalls in Depth

### Pitfall: Recursive DFS on input-controlled depth

- **What goes wrong:** A recursive DFS meets a long path — a linked-list-shaped graph, a deeply nested document, a chain of dependencies — and the process **aborts** with `fatal runtime error: stack overflow`. Measured on a path graph: fine at 100,000 vertices, **aborts at 200,000**. It is not a panic, so it cannot be caught; the whole process dies, taking every in-flight request with it. On a spawned thread (2 MB default versus the main thread's 8 MB) the threshold is roughly four times lower, so code that passes locally dies in a worker pool.
- **Why it happens (the mechanism):** Recursive DFS uses the call stack as its frontier, and that stack is a fixed-size resource allocated at thread creation. Nothing in the type system marks a function as depth-unbounded, and test graphs are almost never deep — random graphs have Θ(log n) diameter, so the pathological case is a *path*, which nobody generates by accident but an adversary generates trivially.
- **How to handle it in production, and why that works:** Use an explicit `Vec` as the stack — depth is then bounded by heap memory rather than by a linker setting, and it handled 5,000,000 vertices in the same measurement. Where recursion is genuinely clearer (you need finish times and the DFS state machine is ugly), enforce an explicit depth limit checked on entry so an over-deep input returns `Err` instead of killing the process.
- **Trade-offs of the fix:** The iterative version is meaningfully less readable for algorithms that need finish times, because you must encode enter/exit transitions yourself rather than letting the call stack do it. The visit order also differs unless you push neighbours in reverse. For a provably shallow graph — a balanced tree, a DAG of bounded depth — recursion is correct and the rewrite is wasted complexity; the trigger is *unbounded or adversary-controlled depth*, not recursion itself.

### Pitfall: Marking visited at the wrong time in BFS

- **What goes wrong:** A BFS marks vertices visited when they are *dequeued* rather than when they are *enqueued*. A vertex reachable from several vertices in the same level gets enqueued multiple times, so the queue grows toward Θ(E) instead of Θ(V) — and on a dense graph that is a memory blow-up. Worse, if distances are assigned at dequeue time, a vertex can be recorded with a longer distance than its first discovery, silently producing wrong shortest-path results.
- **Why it happens (the mechanism):** Iterative DFS legitimately marks on pop (it's what makes its order match the recursive version), so the pattern gets copied to BFS where it is wrong. BFS's correctness rests on "first discovery is the shortest," which requires claiming the vertex at *discovery* time — the moment it's pushed.
- **How to handle it in production, and why that works:** In BFS, set `dist[v]` and push in the same statement, and use `dist[v] != MAX` as the visited test. Then a vertex enters the queue exactly once, the frontier is bounded by V, and the distance recorded is by construction the first (shortest) one.
- **Trade-offs of the fix:** None for BFS — mark-on-push is strictly correct there. The nuance is only that you cannot blindly reuse the same skeleton for DFS and Dijkstra: DFS marks on pop, and Dijkstra's lazy deletion deliberately allows duplicates and filters them at pop time. Three algorithms, three different answers to the same question.

### Pitfall: Using BFS on a weighted graph

- **What goes wrong:** BFS is run on a graph with edge weights and returns the path with the fewest *edges*, not the lowest total weight. On a road network that means a route with three long motorway segments beats one with four short streets — the answer is confidently wrong, and it looks plausible enough to ship.
- **Why it happens (the mechanism):** BFS's guarantee comes from the queue holding at most two distinct distances at a time, which only holds when every edge adds exactly 1. Add weights and the frontier is no longer ordered by distance, so the first arrival at a vertex is no longer the cheapest. BFS doesn't fail loudly — it just answers a different question.
- **How to handle it in production, and why that works:** Non-negative arbitrary weights → [Dijkstra](../shortest-paths/learning.md). Weights only 0 and 1 → **0-1 BFS**, which restores the ordering by pushing zero-weight discoveries to the *front* of a deque, staying Θ(V + E) with no heap. Small integer weights bounded by C → a bucket queue (Dial's algorithm), Θ(E + VC).
- **Trade-offs of the fix:** Dijkstra costs a priority queue and Θ(E log V). 0-1 BFS and bucket queues are faster but only apply to their specific weight structure, so reaching for them requires actually checking the weights rather than assuming — and a later change that introduces a weight of 2 silently breaks 0-1 BFS the same way weights broke plain BFS.

### Pitfall: A visited set that doesn't fit, or is the wrong structure

- **What goes wrong:** Two failures at opposite scales. On a large explicit graph, `HashSet<u32>` is used for visited-tracking, costing a hash and a probe per neighbour visit, which dominates the Θ(V + E) work — a straight several-fold slowdown over an array. On an implicit state space, a `Vec<bool>` is used with vertex IDs computed from state, and either the ID space is enormous (allocating gigabytes for a sparsely-visited space) or IDs collide and the search silently prunes valid states.
- **Why it happens (the mechanism):** The visited set is touched once per *edge*, not per vertex, so it is the hottest structure in the traversal — but it looks like bookkeeping rather than the main event. And implicit graphs blur the distinction between "vertex ID" and "state encoding": a perfect hash of the state is an array index, an imperfect one is a bug.
- **How to handle it in production, and why that works:** Bounded, dense vertex space → `Vec<bool>`, or a bitset for 8× less memory (at V = 10⁷: 1.25 MB versus 10 MB, which can be the difference between fitting in L2 and not). Unbounded or sparse state space → `HashSet`, with a fast hasher since the keys are self-generated ([hash tables](../hash-tables/learning.md): 4.6–6.0× for small keys). If you encode states as integers, prove the encoding is injective, or use the state itself as the key.
- **Trade-offs of the fix:** A bitset costs a shift and mask per access versus a byte load, which is usually free but is measurable in the tightest loops. `Vec<bool>` allocates the whole vertex space up front, which is wasteful when a search touches a tiny fraction — the classic case for `HashSet` even on an explicit graph.

### Pitfall: Reaching for Dijkstra when a traversal would do

- **What goes wrong:** "Nearest hospital to each house" is implemented as k separate Dijkstra runs, one per hospital, then a min over the results — Θ(k · E log V). Or an unweighted shortest path uses Dijkstra out of habit, paying a priority queue for nothing.
- **Why it happens (the mechanism):** Dijkstra is the named "shortest path algorithm," so it's reached for by name. But multi-source distance is a single BFS with *all* sources seeded at distance 0 — the frontier is still ordered by distance, so the invariant holds unchanged, and it is Θ(V + E) regardless of how many sources there are.
- **How to handle it in production, and why that works:** Ask what the weights actually are. Unweighted → BFS. Weights in {0,1} → 0-1 BFS. Many sources, one distance field → multi-source BFS (or multi-source Dijkstra, same trick with a heap). Only arbitrary non-negative weights with a single source genuinely need plain Dijkstra.
- **Trade-offs of the fix:** Multi-source BFS gives distance to the *nearest* source, not per-source distances — if you need to know *which* source is nearest, carry the source ID alongside the distance; if you need all k distance fields, you really do need k runs.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if you kept the traversal tree? | Parent pointers → path reconstruction; the BFS/DFS tree as a structure in its own right |
| Batch it | What if you processed a whole level at once? | Level-synchronous BFS — the basis of parallel and GPU traversal |
| Approximate it | What if you didn't need exact distances? | Landmark-based estimates; sampled BFS for centrality |
| Randomize it | What if the frontier order were random? | Random-walk exploration; PageRank as an infinite random walk |
| Externalize it | What if the frontier exceeded RAM? | Semi-external BFS; sort-based traversal for out-of-core graphs |
| **Parallelize it** | Where's the independence? | A BFS *level* is independent — process it in parallel; **direction-optimizing** switches push↔pull as the frontier grows |
| **Invert it** | What if you searched from the goal too? | **Bidirectional BFS** — `b^(d/2)` instead of `b^d`; the single biggest win in state-space search |
| Augment it | What does a timestamp per vertex buy? | Discovery/finish times → the parenthesis structure → cycles, topo order, SCCs, bridges |
| **Specialize it** | What if weights were only 0 and 1? | **0-1 BFS** with a deque — Dijkstra's answer at BFS's cost |
| Amortize it | What if you re-ran the search repeatedly? | Incremental/dynamic BFS (D\* Lite) — repair the tree instead of rebuilding |

**Questions:**

1. BFS and DFS differ by one line. State precisely which property of the *container* gives BFS its shortest-path guarantee, and construct the weighted counterexample where it fails.
2. Under "specialize it", 0-1 BFS uses a deque. Prove the deque stays sorted by distance — i.e. that pushing zero-weight discoveries to the front preserves the two-distinct-values invariant.
3. Under "invert it", bidirectional search turns `b^d` into `2·b^(d/2)`. Compute the saving for b = 10, d = 12, then name two conditions that make bidirectional search inapplicable.
4. Under "augment it", DFS timestamps produce nested intervals. Show how the nesting alone detects a cycle, then how it produces a topological order.
5. Multi-source BFS handles k sources in one Θ(V+E) pass. Explain why the correctness argument is unchanged from single-source, and what you lose relative to running k separate searches.
6. Under "parallelize it", direction-optimizing BFS switches from push (scan the frontier's out-edges) to pull (scan unvisited vertices' in-edges). Derive when pull wins, and what it needs from the representation.
7. Recursive DFS aborts past ~100k depth but an explicit stack handled 5M. Both are Θ(depth) space — where does the difference come from, and why is one recoverable and the other not?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Write the unified traversal skeleton and say what container turns it into BFS, DFS, and Dijkstra.
2. State the BFS queue invariant and use it to prove the shortest-path property.
3. Give the measured recursive-DFS threshold and explain why the failure is an abort rather than a panic.
4. Mark-on-push vs mark-on-pop: which does BFS require, which does DFS use, and what breaks if you swap them?
5. Give three problems DFS solves that BFS structurally cannot, and name the shared mechanism.
6. You need "distance from each cell to the nearest of 500 exits." Give the algorithm and its complexity.

Build exercises:

- Write one traversal function parameterized over the frontier container, and derive BFS, DFS, and Dijkstra by swapping `VecDeque`, `Vec`, and `BinaryHeap`. Then run all three on the same graph and diff the visit orders — the equivalence stops being abstract once you've seen it.
- Reproduce the stack-overflow threshold: build a path graph and run recursive DFS at n = 50k, 100k, 200k, then the iterative version at 5M. Then run the recursive version on a spawned thread and find the lower threshold.
- Implement 0-1 BFS and verify it against Dijkstra on a graph with weights in {0,1}, then benchmark both. Getting the same answers with no heap is the point.
- Implement Kahn's topological sort with cycle detection and use it to order a real dependency graph (parse a `Cargo.lock`). Then implement the DFS finish-time version and confirm they agree up to ties.

## Open Questions

- How much does a bitset visited-set beat `Vec<bool>` on a large BFS here — is the memory saving worth the shift-and-mask?
- Where does bidirectional BFS start paying off on a real road network, given the overhead of two frontiers and the meeting test?
- Direction-optimizing BFS claims large wins on high-degree graphs; what's the crossover frontier size on this hardware?
- Does `petgraph`'s `Bfs` cost measurably more than a hand-rolled CSR traversal?
- For implicit state spaces, at what point does `HashSet` visited-tracking dominate the search, and does a Bloom filter pre-filter help or just add false prunes?

## References

- CLRS ch. 22 — BFS, DFS, the parenthesis theorem, edge classification, topological sort, and SCCs. The parenthesis theorem is the part worth reading twice.
- Beamer, Asanović & Patterson, "Direction-Optimizing Breadth-First Search" (2012) — the push/pull switch; a rare algorithmic idea that is purely about constant factors.
- Kahn, "Topological sorting of large networks" (1962) — the in-degree algorithm, still the better default.
- [`petgraph` traversal docs](https://docs.rs/petgraph/latest/petgraph/visit/index.html) — a well-designed set of visitor abstractions worth reading even if you hand-roll.
- Related in this repo: [Graph Representations](../graph-representations/learning.md) (CSR, and the 1.76× on the same BFS), [Stacks & Queues](../stacks-and-queues/learning.md) (the frontier *is* the algorithm; the recursion-depth hazard), [Shortest Paths](../shortest-paths/learning.md) (what happens when edges get weights), [Advanced Graph Algorithms](../advanced-graph-algorithms/learning.md) (everything built on DFS timestamps), [Disjoint Set Union](../disjoint-set-union/learning.md) (connectivity without traversal).
