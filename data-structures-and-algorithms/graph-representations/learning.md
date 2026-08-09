# Graph Representations — Learning Notes

## Mental Model

**A graph is a relation, and the representation you choose decides which questions are cheap.** The abstract object — vertices and edges — is the same in every case; what changes is whether "who are u's neighbours?" is a contiguous scan, a pointer chase, or a scan of the entire vertex set.

The choice is dominated by one number: **density**, `d = E / V²`. Real graphs are almost always *sparse* — a road network has ~2.5 neighbours per node, a social graph a few hundred out of billions, a web graph a few dozen. For sparse graphs the adjacency matrix is not merely wasteful, it's disqualifying: measured at V = 500,000, a bit-packed adjacency matrix would need **29 GB**, while the same graph with 4 million directed arcs fits in **32 MB** as CSR — a factor of ~900.

The second idea, and the one that carries into every later graph topic: **the fastest representation is the one with no pointers at all.** Compressed Sparse Row (CSR) stores a graph as two flat arrays — an offset per vertex and a concatenated neighbour list — so a neighbour scan is a contiguous walk. Measured against the idiomatic `Vec<Vec<(u32,u32)>>` on 500k vertices and 4M arcs:

| | `Vec<Vec<…>>` | CSR | Ratio |
| --- | --- | --- | --- |
| Build | 132.3 ms | **40.6 ms** | 3.26× |
| Memory | ~41 MB | **~32 MB** | 1.29× |
| BFS | 38.2 ms | **21.7 ms** | 1.76× |

Same asymptotics throughout. The `Vec<Vec>` version pays 500,000 separate allocations, 24 bytes of `Vec` header per vertex, and a pointer dereference per vertex visited; CSR pays none of it. This is the [arrays](../arrays-and-dynamic-arrays/learning.md) contiguity argument applied to graphs, and it's why every serious graph library and every graph-processing framework uses CSR internally.

Third, and easy to miss: **many graphs should never be materialized at all.** A grid, a state space, a game tree, a set of strings one edit apart — these are *implicit* graphs where neighbours are computed on demand from the vertex itself. Building an adjacency structure for a 10⁹-state puzzle is impossible; generating the ≤4 neighbours of the current state is trivial.

## The Invariant

There isn't one invariant — there are three, one per representation, and knowing which you're holding is the whole topic:

> **Adjacency list:** for each vertex `u`, `adj[u]` contains exactly the vertices `v` such that `(u,v) ∈ E`. For an undirected graph, every edge appears **twice** — in `adj[u]` and `adj[v]`.
>
> **Adjacency matrix:** `M[u][v]` is true iff `(u,v) ∈ E`. For an undirected graph the matrix is **symmetric**, so half of it is redundant.
>
> **CSR:** `offs[u]..offs[u+1]` is the half-open range in `tgt` holding u's neighbours, and `offs` is non-decreasing with `offs[0] = 0` and `offs[V] = 2E` (undirected) or `E` (directed).

The undirected double-entry rule is the source of a whole family of bugs: an edge count that's twice what you expect, a self-loop that gets stored once or twice depending on your code path, and a "remove edge" that removes only one direction. State explicitly whether your structure is directed, and if undirected, whether the two entries are kept in sync by construction.

## Mechanics

### The three representations, priced

| | Space | Neighbours of u | Edge exists? (u,v) | Add edge | Locality |
| --- | --- | --- | --- | --- | --- |
| **Adjacency list** (`Vec<Vec<_>>`) | Θ(V + E) | **Θ(deg u)** | Θ(deg u) | Θ(1) amortized | poor — a chase per vertex |
| **Adjacency matrix** | **Θ(V²)** | Θ(V) | **Θ(1)** | Θ(1) | excellent, but V² of it |
| **CSR** | Θ(V + E) | **Θ(deg u)**, contiguous | Θ(log deg) if sorted | **immutable** | **best** |
| Edge list | Θ(E) | Θ(E) | Θ(E) | Θ(1) | n/a — for Kruskal, streaming |

The decision rule follows directly:

- **Sparse (E ≪ V²) and static** → CSR. Nearly always the right answer for algorithms.
- **Sparse and mutating** → adjacency list, or an arena of edge nodes.
- **Dense (E ≈ V²) or V small (≤ a few thousand)** → matrix. Floyd-Warshall, transitive closure, and bitset-based algorithms genuinely want it.
- **Edges are the unit of work** → edge list. Kruskal sorts edges; Bellman-Ford relaxes all edges each round.

The matrix's Θ(1) edge-existence test is its only real advantage, and it costs Θ(V²) space to get. At V = 500,000 that's the 29 GB above; at V = 2,000 it's 500 KB and completely reasonable. **The matrix isn't wrong — it's wrong for large V**, and "large" starts around a few thousand.

### CSR — the whole construction

Two passes: count degrees, prefix-sum them into offsets, then place each arc.

```rust
pub struct Csr { offs: Vec<u32>, tgt: Vec<u32>, wt: Vec<u32> }

pub fn build(n: usize, edges: &[(u32, u32, u32)]) -> Csr {
    // Pass 1: degree histogram, shifted by one so it becomes offsets after the prefix sum.
    let mut offs = vec![0u32; n + 1];
    for &(a, b, _) in edges { offs[a as usize + 1] += 1; offs[b as usize + 1] += 1; }
    for i in 0..n { offs[i + 1] += offs[i]; }        // prefix sum → offs[u]..offs[u+1]

    // Pass 2: place each arc using a moving cursor per vertex.
    let mut cur = offs.clone();
    let (mut tgt, mut wt) = (vec![0u32; 2 * edges.len()], vec![0u32; 2 * edges.len()]);
    for &(a, b, w) in edges {
        let i = cur[a as usize] as usize; tgt[i] = b; wt[i] = w; cur[a as usize] += 1;
        let j = cur[b as usize] as usize; tgt[j] = a; wt[j] = w; cur[b as usize] += 1;
    }
    Csr { offs, tgt, wt }
}

impl Csr {
    #[inline]
    pub fn neighbours(&self, u: u32) -> impl Iterator<Item = (u32, u32)> + '_ {
        let (s, e) = (self.offs[u as usize] as usize, self.offs[u as usize + 1] as usize);
        (s..e).map(move |i| (self.tgt[i], self.wt[i]))
    }
}
```

That prefix-sum-then-place pattern is worth recognizing: it's a **counting sort** by source vertex, which is why the build is Θ(V + E) with no comparisons and no allocation per vertex — and why it measured 3.26× faster than pushing into 500,000 separate `Vec`s.

The cost is immutability: adding an edge means rebuilding. That's the right trade when you build once and run many algorithms, which is the common case.

### Structure-of-arrays, and why `tgt`/`wt` are separate

Notice `tgt` and `wt` are *parallel arrays*, not a `Vec<(u32,u32)>`. An unweighted traversal (BFS, connectivity, topological sort) then touches only `tgt` and never pulls weights into cache — halving the bytes moved. That's the [data-oriented design](../../performance-optimization/data-oriented-design/learning.md) split applied to the hottest inner loop in the whole category.

### Implicit graphs

```rust
// A grid: never materialize the adjacency structure.
fn neighbours(r: usize, c: usize, h: usize, w: usize) -> impl Iterator<Item = (usize, usize)> {
    const D: [(isize, isize); 4] = [(0,1),(1,0),(0,-1),(-1,0)];
    D.iter().filter_map(move |&(dr, dc)| {
        let (nr, nc) = (r as isize + dr, c as isize + dc);
        (nr >= 0 && nc >= 0 && (nr as usize) < h && (nc as usize) < w).then_some((nr as usize, nc as usize))
    })
}
// Vertex ID for arrays: id = r * w + c
```

Every algorithm in Stage 5 works unchanged on an implicit graph — you only replace "iterate `adj[u]`" with "compute u's neighbours." Puzzle solvers, pathfinding on maps, and word-ladder problems all live here, and the state space is usually far too large to store.

### Rust representations

The [Rust for data structures](../rust-for-data-structures/learning.md) lesson applies with full force: **a graph of `Rc<RefCell<Node>>` is the wrong answer.** Graphs are cyclic by nature, so reference counting leaks without `Weak`, and shared mutation panics at runtime. Vertices are `u32` indices into flat arrays — which is exactly what CSR and `petgraph`'s `NodeIndex` are.

| Need | Use |
| --- | --- |
| Static graph, run algorithms | **CSR** (hand-rolled — it's 20 lines) |
| Mutating graph, general purpose | `petgraph::Graph` / `StableGraph` |
| Dense, small V | `Vec<u64>` bitset rows, or `ndarray` |
| Edge-centric algorithms | `Vec<(u32, u32, u32)>` |
| Huge / streaming | CSR on memory-mapped files, or an out-of-core framework |

`petgraph` is the ecosystem default and worth using when the graph changes or when you want the algorithm library. For a build-once-then-analyze pipeline, hand-rolled CSR is faster and simpler than it sounds.

## Complexity

| Operation | Adjacency list | Matrix | CSR | Edge list |
| --- | --- | --- | --- | --- |
| Space | Θ(V + E) | **Θ(V²)** | Θ(V + E) | Θ(E) |
| Iterate neighbours of u | Θ(deg u) | **Θ(V)** | Θ(deg u) | Θ(E) |
| Edge exists (u,v)? | Θ(deg u) | **Θ(1)** | Θ(log deg u) | Θ(E) |
| Add edge | Θ(1) | Θ(1) | **rebuild** | Θ(1) |
| Remove edge | Θ(deg u) | Θ(1) | rebuild | Θ(E) |
| Iterate all edges | Θ(V + E) | **Θ(V²)** | Θ(V + E) | **Θ(E)** |
| Degree of u | Θ(1) | Θ(V) | **Θ(1)** | Θ(E) |

**Where the table misleads.** Adjacency list and CSR share every asymptotic entry and differ substantially in practice — measured 1.76× on BFS and 3.26× on build. The list's Θ(V + E) hides V separate heap allocations, a 24-byte `Vec` header per vertex, and a dependent load per vertex visited; CSR's hides none of those. When two rows of a complexity table are identical, the constant factor is the entire decision.

The matrix's Θ(V²) row is the one that actually disqualifies it: at V = 500,000, 29 GB even bit-packed. But note the flip side — for V ≤ ~2,000 a bitset matrix makes reachability and transitive closure run at 64 vertices per word, which no sparse structure can match.

## Use Cases

- **Road networks and routing** — sparse (~2.5 avg degree), static, huge. CSR, always.
- **Social and web graphs** — sparse but with extreme degree skew (a few hyper-connected vertices). CSR plus attention to load balancing, since one vertex may hold millions of arcs.
- **Dependency graphs** — build systems, package managers, task schedulers. Small enough that any representation works; usually an adjacency list because they mutate.
- **State-space search** — puzzles, planners, game trees. **Implicit**: never materialized.
- **Grids and maps** — implicit, with `id = r * w + c` for array indexing.
- **Dense small graphs** — all-pairs shortest paths via Floyd-Warshall, transitive closure, bipartite matching on small instances. Matrix, and often a bitset matrix.
- **Streaming edges** — connectivity over an edge stream needs no adjacency structure at all: feed edges to a [DSU](../disjoint-set-union/learning.md).

## When to Use Which

| Reach for | When |
| --- | --- |
| **CSR** | Sparse, static, algorithms run repeatedly — **the default** |
| Adjacency list (`Vec<Vec<_>>`) | Sparse and mutating; prototyping; graph is small |
| `petgraph` | Mutating graph, or you want the algorithm library |
| **Adjacency matrix** | Dense (E ≈ V²), or V ≤ ~2,000, or you need Θ(1) edge tests |
| Bitset matrix rows | Reachability/closure on small V — 64 vertices per word |
| **Edge list** | Kruskal, Bellman-Ford, streaming, or the graph is only ever edges |
| **Implicit (no structure)** | Grids, state spaces, anything generated on demand |
| DSU only | Connectivity over an edge stream — no graph needed at all |

## Pitfalls in Depth

### Pitfall: An adjacency matrix on a sparse graph

- **What goes wrong:** The matrix is chosen because it's the tidiest to write — `m[u][v] = true` — and the program either exhausts memory outright or spends all its time scanning empty rows. Measured at V = 500,000: a bit-packed matrix needs **29 GB** against CSR's **32 MB** for the same 4M arcs. And every neighbour iteration becomes Θ(V) instead of Θ(deg u), so a BFS goes from Θ(V + E) to Θ(V²) — at V = 500,000 that's 2.5×10¹¹ operations rather than 4.5×10⁶.
- **Why it happens (the mechanism):** The matrix is the representation textbooks introduce first, and it's the only one where the code reads like the mathematical definition. Its cost is invisible at small V — a 1,000-vertex matrix is 125 KB and fine — so the bad decision is made on a toy instance and only fails at scale.
- **How to handle it in production, and why that works:** Compute the density before choosing: `E / V²`. Below a few percent, use CSR or an adjacency list, whose cost tracks *edges* rather than *vertex pairs*. Above it — or when V is small enough that V² is a rounding error — the matrix's Θ(1) edge test and bitset parallelism genuinely win.
- **Trade-offs of the fix:** Sparse representations make "is (u,v) an edge?" Θ(deg u) instead of Θ(1), which hurts algorithms built around that query (some triangle-counting and clique algorithms). The mitigation is to keep neighbour lists sorted and binary-search them, or keep a `HashSet<(u32,u32)>` alongside for the specific query — paying memory only for the query you actually make.

### Pitfall: `Vec<Vec<T>>` as the default adjacency structure

- **What goes wrong:** The idiomatic-looking `Vec<Vec<(u32, u32)>>` costs V separate heap allocations, a 24-byte header per vertex regardless of degree, and a dependent pointer load before every neighbour scan. Measured on 500k vertices and 4M arcs: build **3.26× slower** (132.3 vs 40.6 ms), **1.29× more memory**, and BFS **1.76× slower** than CSR — with identical asymptotics throughout.
- **Why it happens (the mechanism):** The nested `Vec` maps directly onto the mental picture of "a list per vertex" and requires no thought. The cost is that each inner `Vec` is a separate allocation scattered across the heap, so traversing vertices in any order chases pointers into unrelated cache lines — and for a graph with average degree 8, the 24-byte header is larger than the 32 bytes of payload it points at.
- **How to handle it in production, and why that works:** Build CSR with the two-pass counting-sort construction above — degree histogram, prefix sum, place. It's about 20 lines, allocates exactly three vectors, and makes neighbour iteration a contiguous scan the prefetcher can follow. Splitting `tgt` and `wt` into parallel arrays additionally halves the bytes touched by unweighted traversals.
- **Trade-offs of the fix:** CSR is immutable — adding an edge means rebuilding in Θ(V + E). That's fine for build-once-analyze-many and wrong for a graph that mutates continuously, where `petgraph::StableGraph` or an adjacency list is correct. Prototyping with `Vec<Vec<_>>` and converting to CSR once the algorithm works is a reasonable path.

### Pitfall: Undirected edges stored inconsistently

- **What goes wrong:** An undirected graph is built by pushing only `adj[a].push(b)` — half the graph is missing, so BFS explores a directed subgraph and reports wrong components. Or the reverse: edges are added twice *and* the edge count is reported as `adj.iter().map(Vec::len).sum()`, which is 2E, so every derived statistic is doubled. Self-loops are stored once by one code path and twice by another.
- **Why it happens (the mechanism):** "Undirected" is a property of the *abstraction*, not of the storage — every representation stores directed arcs, and undirectedness is a convention you maintain. Nothing in the type system distinguishes `Graph` from `DiGraph`, so the invariant lives only in the code that builds it, and any second construction path can violate it.
- **How to handle it in production, and why that works:** Funnel all edge insertion through one `add_edge` method that adds both directions, so there is exactly one place the invariant can be wrong. Encode directedness in the type (`struct Undirected(Csr);`) so a directed algorithm can't silently consume an undirected structure. Assert `sum(degrees) == 2 * edge_count` in a `#[cfg(test)]` check, and decide explicitly what a self-loop contributes to degree (convention: 2 for undirected).
- **Trade-offs of the fix:** Doubling storage for undirected graphs is genuinely 2× the arcs — some algorithms (Kruskal, Bellman-Ford) only need the edge list and can skip it entirely. A newtype wrapper adds friction when you legitimately want to run a directed algorithm on the underlying arcs.

### Pitfall: Materializing an implicit graph

- **What goes wrong:** A grid pathfinding problem, a puzzle solver, or a word-ladder search begins by building an explicit adjacency list for every state. For a 15-puzzle that's 10¹³ states; for a 1000×1000 grid it's 4 million arcs built to answer a query that touches a few thousand of them. The program runs out of memory or spends all its time on construction.
- **Why it happens (the mechanism):** Every algorithm is taught against an explicit `adj[u]`, so "build the graph, then search it" reads as the required first step. But the adjacency relation is often a *function* — grid neighbours are ±1 in each axis, puzzle moves are the legal slides — and a function can be evaluated lazily on the states you actually reach, which in a targeted search is a vanishing fraction of the space.
- **How to handle it in production, and why that works:** Write algorithms against a neighbour *function* rather than a stored structure — in Rust, take `impl Fn(u32) -> impl Iterator<Item = u32>` or define a small `Graph` trait. Then the same BFS runs over CSR, over a grid, or over a generated state space unchanged. Track visited states in a `HashSet` (sparse exploration) or a flat `Vec<bool>` indexed by a computed ID (dense, bounded space).
- **Trade-offs of the fix:** Generating neighbours on demand recomputes them every visit, which is wasteful if the same vertex is expanded repeatedly — for a small graph traversed many times, materializing is faster. And a `HashSet` of visited states costs a hash per node against a `Vec<bool>`'s single byte, so implicit search's memory advantage shrinks when the reachable set is large.

### Pitfall: `u32` vertex IDs that silently overflow, or `usize` everywhere

- **What goes wrong:** Two opposite failures. Using `usize` (8 bytes) for vertex IDs doubles the size of every adjacency structure versus `u32` — measured, CSR's arrays are the dominant memory cost, so this is a straight 2× on the biggest allocation, and a corresponding halving of how much of the graph fits in cache. Conversely, `u32` silently wraps above 4.29 billion vertices, and — more commonly — an edge *count* stored as `u32` overflows at 4.29 billion arcs on a large graph.
- **Why it happens (the mechanism):** `usize` is the path of least resistance because it indexes slices without casting. But graph structures are dominated by ID storage, so the width of that type *is* the memory profile. Meanwhile `u32` is right for IDs on any realistic graph, but the CSR *offsets* array indexes into `2E` arcs, which can exceed `u32` long before vertex count does.
- **How to handle it in production, and why that works:** Use `u32` for vertex IDs (4 billion vertices is beyond any in-memory graph) and be deliberate about the offsets type — `u64` offsets with `u32` targets is the standard combination for large graphs, since it bounds arcs without doubling the target array. Wrap IDs in a newtype (`NodeId(u32)`) so they can't be confused with edge indices or array positions, which is the same generational-handle discipline from [Rust for data structures](../rust-for-data-structures/learning.md).
- **Trade-offs of the fix:** `u32` IDs need `as usize` casts at every index site — noise, and a place for a bug if a subtraction goes negative first. A newtype adds `.0` unwrapping throughout. Both are small costs against halving the memory of the structure your whole program is bound by.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if the graph had versions? | Copy-on-write CSR; versioned edge sets for time-travel queries over evolving graphs |
| Batch it | What if you built from a sorted edge stream? | CSR construction *is* a counting sort — Θ(V + E), no comparisons |
| Approximate it | What if you stored a sketch of the neighbourhood? | MinHash/SimHash per vertex for similarity; graph sparsification preserving cuts |
| Randomize it | What if you sampled edges? | Spectral sparsifiers; sampled BFS for approximate centrality |
| Externalize it | What if the graph exceeded RAM? | Memory-mapped CSR (offsets and targets are position-independent); out-of-core frameworks |
| Parallelize it | Where's the imbalance? | Degree skew — a few vertices hold most arcs, so partition by *edges*, not vertices |
| **Invert it** | What if you stored the **reverse** graph? | Reverse CSR — needed for SCCs (Kosaraju), backward BFS, bidirectional search, PageRank's pull form |
| Augment it | What does one array per vertex buy? | Vertex properties as parallel arrays (SoA) — colour, distance, component, all cache-friendly |
| Specialize it | What if the graph were a grid? | **Implicit** — neighbours computed, nothing stored |
| Amortize it | What if you rebuilt periodically? | Mutable overlay (a small delta list) over an immutable CSR base, compacted on a threshold |

**Questions:**

1. Measured, CSR beat `Vec<Vec>` by 1.76× on BFS with identical asymptotics. Name the three distinct costs the nested `Vec` pays, and say which one you'd expect to dominate for a graph of average degree 2 versus average degree 200.
2. CSR construction is a counting sort by source vertex. State the general condition under which counting sort applies, and say what it buys over sorting the edge list by source.
3. Under "invert it", several algorithms need the reverse graph. Give three, and say for each whether you'd store the reverse explicitly or derive it.
4. A bit-packed matrix at V = 500,000 is 29 GB; the same graph is 32 MB as CSR. Derive the density at which the two representations use equal memory, and check it against the real graph's density.
5. Under "parallelize it", degree skew makes vertex-based partitioning imbalanced. Design an edge-based partition and say what it costs when a single vertex's arcs span partitions.
6. Under "specialize it", implicit graphs store nothing. What does an algorithm need from a graph representation, minimally — write the trait — and which Stage 5 algorithms work unchanged against it?
7. `tgt` and `wt` are parallel arrays rather than `Vec<(u32,u32)>`. Quantify the bytes moved by an unweighted BFS under each, and say when the split would *hurt*.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Give the three representations' space and neighbour-iteration costs, and the density rule for choosing.
2. Describe CSR's two-pass construction, and name the classical algorithm it is.
3. Give the measured CSR-vs-`Vec<Vec>` numbers for build, memory, and BFS, and explain the gap despite identical asymptotics.
4. State the undirected double-entry invariant and two bugs that follow from violating it.
5. When is an adjacency matrix the *right* answer? Give two concrete situations.
6. What is an implicit graph, and what does an algorithm need in order to run on one?

Build exercises:

- Implement CSR construction and BFS, then benchmark against a `Vec<Vec<(u32,u32)>>` version on 500k vertices / 4M arcs. Reproduce the ~3× build and ~1.8× BFS gap, then split `tgt`/`wt` into parallel arrays and measure the unweighted traversal again.
- Write BFS once against a `Graph` trait with a single `neighbours(u)` method, then run the *same* function over CSR, over a grid computed on demand, and over a word-ladder state space. This is the exercise that makes implicit graphs stop feeling like a special case.
- Build the reverse CSR from a forward CSR in Θ(V + E) without sorting, and verify `reverse(reverse(g)) == g`. You'll need it for SCCs in the advanced topic.
- Measure the density crossover yourself: for V = 2,000 sweep E from V to V², timing BFS and edge-existence queries on a bitset matrix versus CSR. Find where each wins.

## Open Questions

- Where exactly is the memory/speed crossover between a bitset adjacency matrix and CSR on this machine, as a function of density?
- How much does the `tgt`/`wt` split actually buy on an unweighted BFS at 4M arcs? The theory says ~2× on bytes moved; measure the wall-clock.
- Does sorting each vertex's neighbour list improve BFS locality measurably, or does the prefetcher already handle it?
- `petgraph`'s `Graph` versus hand-rolled CSR on the same traversal — what is the abstraction costing?
- For degree-skewed graphs (power law), does a hybrid representation (CSR for low-degree, hash sets for hubs) pay off, or is the branch cost worse than the scan?

## References

- CLRS ch. 22.1 — the representations and their trade-offs, stated cleanly.
- Kepner & Gilbert, *Graph Algorithms in the Language of Linear Algebra* — the matrix view taken seriously; explains when the "wasteful" representation is the right one.
- [`petgraph`](https://docs.rs/petgraph/) — the Rust ecosystem default; read its `Graph` vs `StableGraph` vs `Csr` docs for a well-considered set of trade-offs.
- Shun & Blelloch, "Ligra: A Lightweight Graph Processing Framework" (2013) — direction-optimizing traversal and why representation choice drives everything at scale.
- Related in this repo: [Graph Traversal](../graph-traversal/learning.md) (the algorithms this feeds), [Arrays & Dynamic Arrays](../arrays-and-dynamic-arrays/learning.md) (contiguity — the whole CSR argument), [Rust for Data Structures](../rust-for-data-structures/learning.md) (indices, not `Rc<RefCell>`), [Data-Oriented Design](../../performance-optimization/data-oriented-design/learning.md) (parallel arrays for vertex properties), [Cache Locality](../../performance-optimization/cache-locality/learning.md) (why the pointer chase costs 1.76×).
