# Disjoint Set Union — Learning Notes

## Mental Model

**DSU answers exactly one question — "are these two things in the same group?" — and supports exactly one modification: "merge these two groups."** That's the entire interface. It cannot enumerate a group, split a group, or delete an element. In exchange for giving all of that up, it becomes essentially free: **effectively Θ(1) per operation**, on an array of integers with no pointers and no allocation.

The representation is the idea: each element points to a *parent*, each group is a tree, and a group's identity is its root. `find(x)` walks to the root; `union(a, b)` points one root at the other. Nothing about the tree's *shape* matters, because the tree is never traversed downward and the structure isn't a search structure at all — it's a "who is my representative?" structure.

That freedom is what makes the two optimizations possible, and they are unusually dramatic. Measured on this machine (n elements, 2n random unions, then n finds):

| n | No optimizations | Union by rank | Path compression | **Both** |
| --- | --- | --- | --- | --- |
| 10,000 | 73.6 ms | 0.5 ms | 0.5 ms | **0.2 ms** |
| 50,000 | 2,286.1 ms | 2.0 ms | 2.2 ms | **1.1 ms** |
| 200,000 | **66,767.0 ms** | 9.1 ms | 10.6 ms | **4.6 ms** |
| 1,000,000 | >60,000 ms | 50.5 ms | 67.2 ms | **25.7 ms** |

**At n = 200,000: 66.8 seconds versus 4.6 milliseconds — 14,500×.** The unoptimized version scales quadratically (5× the elements → 31× the time, then 4× → 29×), and each optimization is roughly *one line of code*.

That ratio is the best argument in this entire category for why the details of a data structure matter. Two one-line changes convert an unusable structure into one of the fastest known.

## The Invariant

> Every element has a `parent`; following parents from any element terminates at a **root** (an element that is its own parent). Two elements are in the same set **iff** they reach the same root. Every element belongs to exactly one set.

Three things follow:

- **`find` is the entire semantics.** Same-set is `find(a) == find(b)`; that's the only query.
- **The tree shape is unconstrained** — nothing requires balance, ordering, or depth bounds. Which is exactly why the naive version can degenerate into a 200,000-long chain, and why we're free to *rewrite the shape arbitrarily* during a `find` (path compression) without breaking anything. A BST could never do that.
- **The operation is one-way.** Merging is irreversible: there is no `split`. If you need to remove elements or undo merges, DSU is the wrong structure (or needs the rollback variant below).

With **union by rank/size** the invariant strengthens to bound the height: a tree of rank *r* contains at least 2^r elements, so height ≤ log₂ n even before path compression.

## Mechanics

### The two optimizations

**Union by rank (or size)** — always attach the shorter tree under the taller one. Without it, `union(a, b)` might attach a deep tree under a shallow one and grow the depth by one every time; with it, depth only increases when merging two trees of equal rank, which requires doubling the size. Height becomes Θ(log n).

**Path compression** — during `find`, after locating the root, point *every node on the path* directly at the root. The next `find` on any of them is Θ(1). This is the move that a search tree can't make, because DSU doesn't care about shape.

Together they give **Θ(α(n)) amortized**, where α is the inverse Ackermann function. α(n) ≤ 4 for any n that can be written down — 2^65536 and beyond. It is not literally Θ(1), but the distinction has never mattered in practice, and Tarjan proved the bound is tight for this class of algorithm.

The measured table shows something the theory doesn't emphasize: **either optimization alone captures most of the win** (9.1 and 10.6 ms versus 4.6 ms for both at n = 200k, against 66,767 ms for neither). Both together are roughly 2× better than either alone. So if you only remember one, remember either — but they're both one line.

### The implementation, complete

```rust
pub struct Dsu { parent: Vec<u32>, rank: Vec<u8>, count: usize }

impl Dsu {
    pub fn new(n: usize) -> Self {
        Dsu { parent: (0..n as u32).collect(), rank: vec![0; n], count: n }
    }

    /// Iterative two-pass: find the root, then compress the path.
    pub fn find(&mut self, x: u32) -> u32 {
        let mut root = x;
        while self.parent[root as usize] != root { root = self.parent[root as usize]; }
        let mut cur = x;                                  // path compression
        while self.parent[cur as usize] != root {
            let next = self.parent[cur as usize];
            self.parent[cur as usize] = root;
            cur = next;
        }
        root
    }

    /// Returns false if they were already in the same set.
    pub fn union(&mut self, a: u32, b: u32) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb { return false; }
        let (hi, lo) = if self.rank[ra as usize] >= self.rank[rb as usize] { (ra, rb) } else { (rb, ra) };
        self.parent[lo as usize] = hi;                    // union by rank
        if self.rank[hi as usize] == self.rank[lo as usize] { self.rank[hi as usize] += 1; }
        self.count -= 1;                                  // number of disjoint sets
        true
    }

    pub fn same(&mut self, a: u32, b: u32) -> bool { self.find(a) == self.find(b) }
    pub fn sets(&self) -> usize { self.count }
}
```

Two implementation notes worth having:

- **`find` takes `&mut self`** because compression mutates. That's occasionally awkward (you can't call it from a `&self` method), and the workarounds — a non-compressing `find`, or interior mutability — both cost you either the bound or the borrow checker's help. Accept the `&mut`.
- **Write `find` iteratively.** The recursive version is prettier but recurses to the tree depth, and a *deliberately* adversarial input can reach a deep chain before compression kicks in — the [Rust for data structures](../rust-for-data-structures/learning.md) stack-overflow risk again.
- **`rank` fits in a `u8`.** Rank ≤ log₂ n, so 8 bits covers n up to 2^255. Using `u8` instead of `u32` shrinks the struct and helps locality.

### The variants worth knowing

| Variant | Adds | Cost |
| --- | --- | --- |
| **Union by size** | Track set sizes instead of rank | Same bound, and you get set sizes for free — usually preferable |
| **DSU with rollback** | Undo the last k unions | Must drop path compression (it destroys history); Θ(log n) per op via union by rank only |
| **Weighted / potential DSU** | Store an offset to the parent | "Is a exactly d units from b?" — used for equation systems and parity constraints |
| **Persistent DSU** | Query any past version | Persistent arrays; much slower constants |
| **Small-to-large merging** | Merge actual collections, not just labels | Θ(n log n) total when you need per-set data |

DSU-with-rollback is the one people miss: because path compression rewrites history, it must be dropped to support undo — which is exactly the trade needed for offline dynamic connectivity (below).

## Complexity

| Operation | Naive | Union by rank only | Path compression only | **Both** |
| --- | --- | --- | --- | --- |
| `find` | Θ(n) | Θ(log n) | Θ(log n) amortized | **Θ(α(n))** |
| `union` | Θ(n) | Θ(log n) | Θ(log n) amortized | **Θ(α(n))** |
| `same` | Θ(n) | Θ(log n) | Θ(log n) amortized | **Θ(α(n))** |
| Space | Θ(n) | Θ(n) | Θ(n) | Θ(n) |

α(n) ≤ 4 for all practical n. The bound is *amortized* — a single `find` can still be Θ(log n); it's the sequence that's near-linear.

**Where the table understates things.** The Θ notation hides that DSU's constant is about as small as a data structure gets: one `Vec<u32>` indexed by element ID, no allocation after construction, no pointers, and after compression most `find` calls are one or two array reads. That's why the measured "both" column is 25.7 ms for 3 million operations at n = 10⁶ — roughly **8 ns per operation** including cache misses.

**What it can't do**, and this is the real limitation: no `split`, no enumerate-a-set, no delete. If you need any of those, you need a different structure — or the offline trick below.

## Rust Implementation

```rust
// Kruskal's MST — the canonical DSU application.
edges.sort_unstable_by_key(|e| e.weight);
let mut dsu = Dsu::new(n);
let mut mst = Vec::new();
for e in edges {
    if dsu.union(e.u, e.v) {          // returns false if it would form a cycle
        mst.push(e);
        if mst.len() == n - 1 { break; }
    }
}

// Connected components in one pass.
for (u, v) in edge_list { dsu.union(u, v); }
let component_count = dsu.sets();

// Grouping by equivalence: collect members per root.
let mut groups: HashMap<u32, Vec<u32>> = HashMap::new();
for i in 0..n as u32 { groups.entry(dsu.find(i)).or_default().push(i); }
```

**Coordinate mapping is the usual glue.** DSU works on `0..n` integer IDs, so real keys (strings, coordinates, node handles) get interned first — a `HashMap<K, u32>` at the boundary, exactly the interning pattern from [hash tables](../hash-tables/learning.md). For grid problems, `id = row * width + col`.

**Crates:** `union-find`, `petgraph::unionfind`. Honestly, the 30 lines above are usually better than a dependency — you'll want to specialize it (sizes, rollback, weights) and it's short enough to own.

## Use Cases

- **Kruskal's minimum spanning tree** — the textbook use; DSU is what makes the cycle check Θ(α) instead of Θ(n).
- **Connected components** — in a graph given as an edge stream, DSU computes components in one pass without building an adjacency structure or doing a traversal.
- **Percolation and flood fill on grids** — "is the top connected to the bottom?" Each cell is an element; opening a cell unions it with open neighbours.
- **Image segmentation** — merge adjacent pixels with similar values; the classic Felzenszwalb-Huttenlocher algorithm is DSU-driven.
- **Type inference / unification** — unifying type variables is literally union-find, and it's how Hindley-Milner implementations track equivalence classes.
- **Cycle detection while building** — "would adding this edge create a cycle?" is `find(u) == find(v)`.
- **Offline dynamic connectivity** — with rollback plus divide-and-conquer over the timeline, you can answer connectivity queries under both edge insertions *and* deletions, even though DSU itself can't split. The trick is processing queries offline in a segment tree over time.
- **Equivalence classes generally** — merging duplicate records, clustering by transitive similarity, grouping equal registers in a compiler.

## When to Use Which

| Reach for | When |
| --- | --- |
| **DSU** | Merge-only grouping; "same set?" queries; you never need to split or enumerate |
| DSU **by size** | Same, and you want set sizes — strictly more useful than rank |
| DSU with **rollback** | You need to undo unions (offline dynamic connectivity) — costs path compression |
| **Weighted DSU** | Relations with offsets: "a is d more than b", parity constraints |
| BFS/DFS traversal | You need the actual components' contents, paths, or structure |
| `HashMap<K, Vec<V>>` | You need to enumerate or modify group *members*, not just identity |
| Link-cut trees / ETT | You need genuinely dynamic connectivity **online**, with deletions |

## Pitfalls in Depth

### Pitfall: Skipping the optimizations

- **What goes wrong:** A DSU is written with just `parent` and a naive `find`/`union` — it's obviously correct, passes tests on small inputs, and is catastrophically slow at scale. Measured: **66,767 ms versus 4.6 ms at n = 200,000 — 14,500×** — and the naive version scales quadratically, so the gap widens with every increment in n. At n = 10⁶ it didn't finish in a minute.
- **Why it happens (the mechanism):** Without union by rank, `union` may attach a tall tree under a short one, growing the chain by one each time; random union sequences reliably produce chains thousands of nodes long. Every subsequent `find` walks the whole chain, so both `find` and `union` become Θ(n) and the total is Θ(n²). Nothing signals this — the structure is *correct*, just slow, and small test fixtures never build deep chains.
- **How to handle it in production, and why that works:** Add both. Union by rank bounds the height at Θ(log n) structurally; path compression flattens the path on every traversal so repeated queries are Θ(1). Together they give Θ(α(n)) — and each is about one line, which is the point.
- **Trade-offs of the fix:** Path compression forces `find(&mut self)`, which propagates through your API and prevents calling it from `&self` contexts. Union by rank costs one extra array (`u8` per element). Both are negligible, and the measured table shows either alone already captures ~99.99% of the win — so there is no defensible reason to ship neither.

### Pitfall: Expecting operations DSU doesn't have

- **What goes wrong:** Code needs to *split* a set (undo a merge), *delete* an element, or *enumerate* a set's members, and DSU offers none of these. People bolt on a `HashMap<root, Vec<member>>` alongside — which then goes stale on every union, because the root changes and nothing updates the map.
- **Why it happens (the mechanism):** DSU's speed comes precisely from storing *only* the parent pointer. There is no downward link, so a set's members are not reachable from its root; you'd have to scan all n elements. And merging is destructive — path compression actively rewrites parent pointers, so the history needed to split is gone.
- **How to handle it in production, and why that works:** Decide up front which operations you need. Merge-only with identity queries → DSU. Need members → keep `HashMap<u32, Vec<u32>>` keyed by root and merge the *smaller list into the larger* on every union (small-to-large), which is Θ(n log n) total because each element moves at most log n times. Need real deletion or online splits → link-cut trees or Euler-tour trees, which are far more complex. Need deletions but can batch → offline dynamic connectivity with rollback DSU over a segment tree on time.
- **Trade-offs of the fix:** Small-to-large adds Θ(n log n) and real memory for the member lists. Link-cut trees are a large implementation. The offline approach requires knowing all queries in advance, which rules out interactive systems — but when it applies, it's dramatically simpler than the online structures.

### Pitfall: Recursive `find` overflowing the stack

- **What goes wrong:** The elegant recursive `find` — `if p[x] != x { p[x] = find(p[x]); } p[x]` — recurses once per level. On a chain built before compression has flattened it (a naive union sequence, or an adversarial input), depth can reach hundreds of thousands, and the process **aborts** with a stack overflow rather than panicking.
- **Why it happens (the mechanism):** Recursion depth equals tree depth, and tree depth is only bounded if union by rank is present *from the start*. A DSU built with union-by-size but processing a pathological insertion order, or one where compression hasn't run yet on a fresh deep chain, can exceed the ~250k-frame ceiling measured in [Rust for data structures](../rust-for-data-structures/learning.md) — and a worker thread's 2 MB stack fails at roughly a quarter of that.
- **How to handle it in production, and why that works:** Write `find` iteratively in two passes: walk to the root, then walk again setting parents to the root. Same Θ(α) bound, constant stack, and it's barely longer than the recursive version. Alternatively use path *halving* (`p[x] = p[p[x]]` while walking), a single-pass variant that achieves the same amortized bound with even less code.
- **Trade-offs of the fix:** Two passes over the path instead of one — irrelevant, since the path is nearly always length 1–2 after compression. Path halving is a single pass but compresses slightly less aggressively; the asymptotic bound is unchanged and it's a common production choice.

### Pitfall: Forgetting that `union` may be a no-op

- **What goes wrong:** Kruskal's algorithm adds an edge to the MST every time it calls `union`, without checking whether the union actually happened. The result has cycles and more than n−1 edges. Or a component counter is decremented on every `union` call, so the count drifts below the true number of sets.
- **Why it happens (the mechanism):** `union(a, b)` on two elements already in the same set is a legitimate, silent no-op — it's how cycle detection works. If the return value is ignored, the caller can't distinguish "merged two sets" from "these were already connected", and both the MST construction and the set count depend on that distinction.
- **How to handle it in production, and why that works:** Return `bool` from `union` (as in the implementation above) and mark it `#[must_use]` so ignoring it is a warning. Maintain the set count inside `union`, decrementing only on an actual merge, so callers can't get it wrong. Kruskal's then reads naturally: `if dsu.union(u, v) { mst.push(e); }`.
- **Trade-offs of the fix:** None meaningful — the boolean is free, and `#[must_use]` occasionally forces a `let _ =` at call sites that genuinely don't care.

### Pitfall: Assuming `find` results are stable

- **What goes wrong:** A root ID is cached — used as a map key, stored in a struct, written to output — and later unions change which element is the root. The cached value now identifies a set that has been absorbed into a larger one, so lookups miss and grouping is wrong. The bug is timing-dependent: it only manifests when a union happens between caching and use.
- **Why it happens (the mechanism):** The root is an *arbitrary representative*, not a stable identity. `union` deliberately repoints one root at the other (chosen by rank, not by any property you control), so any element's root can change at any merge. Path compression additionally rewrites intermediate parents, though that doesn't change the root itself.
- **How to handle it in production, and why that works:** Treat `find(x)` as valid only until the next `union`. Never store roots across mutations — re-`find` at the point of use, which is Θ(α) and effectively free. If you need a stable per-set identity that survives merges, maintain it separately (e.g. keep the minimum element ID per set, updated in `union`), because that's a property you define rather than one DSU provides.
- **Trade-offs of the fix:** Re-finding at every use is cheap but means you can't hold a `&` into set metadata across unions — the borrow checker will actually help here, since `find` needs `&mut self`. Maintaining a stable ID costs an extra array and a line in `union`.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if you could query any past state? | Persistent DSU via persistent arrays; or **rollback DSU** (drop compression, undo unions) |
| Batch it | What if all queries were known in advance? | **Offline dynamic connectivity** — segment tree over time + rollback DSU handles deletions DSU can't |
| Approximate it | What if "same set" could be wrong rarely? | Not useful here — but hashing set identity gives Θ(1) equality checks on *frozen* partitions |
| Randomize it | What if you attached roots randomly instead of by rank? | Expected Θ(log n) height with no rank array — the same average-vs-expected move as treaps |
| Externalize it | What if n didn't fit in memory? | External DSU via sorting-based connected components (used in large-graph pipelines) |
| Parallelize it | Where's the contention? | Concurrent DSU with CAS on parent pointers; wait-free variants exist and are genuinely used in parallel MST |
| **Invert it** | What if you stored children instead of parents? | You could enumerate sets — but merging becomes Θ(size). That trade *is* small-to-large merging |
| Augment it | What does one more array per element buy? | Sizes (free stats); weights/offsets (**weighted DSU** for `a − b = d` constraints); min/max element per set |
| Specialize it | What if elements were grid cells? | `id = row*w + col`; percolation, flood fill, connected-component labelling |
| Amortize it | What if one operation could be terrible? | Path compression itself — one `find` pays Θ(log n) so all later ones are Θ(1) |

**Questions:**

1. Path compression rewrites the tree arbitrarily during a read. Name the property of DSU that makes this legal, and explain why a BST could never do the equivalent.
2. Measured, either optimization alone gets within ~2× of both together, while neither is 14,500× worse. Explain why the *first* optimization captures almost all the win, whichever one it is.
3. Under "invert it": storing children lets you enumerate a set but makes merging Θ(size). Derive why merging the *smaller* into the larger makes the total Θ(n log n), and state the bound on how often one element moves.
4. Rollback DSU must abandon path compression. Explain precisely what compression destroys, and what the resulting per-operation bound becomes.
5. Under "batch it", offline dynamic connectivity handles edge *deletions* despite DSU having no split. Sketch how a segment tree over the timeline achieves that.
6. Under "augment it", weighted DSU stores an offset to the parent. What must happen to those offsets during path compression, and why is that the tricky part?
7. α(n) ≤ 4 for every writable n, so the bound is "effectively constant." What would have to be true for the difference between Θ(α(n)) and Θ(1) to ever matter in practice?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the DSU invariant and the three operations it does *not* support.
2. Give the measured numbers at n = 200,000 for all four optimization combinations, and identify the naive version's complexity class from the scaling.
3. Explain union by rank and path compression in one sentence each, and what each bounds.
4. Why does `find` need `&mut self`, and what do you lose by making it non-compressing?
5. Why must `union` return a bool, and give two bugs that follow from ignoring it.
6. You need to enumerate the members of each set. Give two approaches and their costs.

Build exercises:

- Implement DSU four ways (neither optimization, rank only, compression only, both) behind one interface and reproduce the measured table at n = 10⁴, 5×10⁴, 2×10⁵. Confirm the naive version is quadratic by checking that 4× the elements gives ~16× the time. Watching 66 seconds become 4.6 milliseconds from two one-line changes is the single most memorable measurement in this stage.
- Implement Kruskal's MST on a large random graph using your DSU, and verify the result against a Prim's implementation. Then deliberately ignore `union`'s return value and observe the cycles appear.
- Implement rollback DSU (union by rank only, with an undo stack) and use it for offline dynamic connectivity: a segment tree over the query timeline where each edge is active for an interval. This is the most sophisticated exercise in Stage 4 and it makes the "batch it" lens concrete.
- Implement weighted DSU to solve a system of `a − b = d` constraints, detecting contradictions. Getting the offset arithmetic right through path compression is the hard part — property-test it against a brute-force graph traversal.

## Open Questions

- Why is path-compression-only *slower* than rank-only at n = 10⁶ (67.2 ms vs 50.5 ms) but comparable at smaller n? Cache behaviour of the second pass is the suspect — verify with counters.
- Does path halving (single pass) actually beat two-pass compression on this machine, and by how much?
- Union by size vs union by rank measured head-to-head — is the extra usefulness of sizes free?
- Is a `u8` rank array measurably better than `u32` (locality) at n = 10⁷, or is it noise?
- Concurrent DSU with CAS: what's the realistic speedup on parallel connected-components for a large graph, given the contention on shared roots?

## References

- Tarjan, "Efficiency of a Good But Not Linear Set Union Algorithm" (1975) — the Θ(mα(n)) bound and the proof that it's tight for this class.
- Tarjan & van Leeuwen, "Worst-case Analysis of Set Union Algorithms" (1984) — compares every combination of compression and union rules, including path halving and splitting.
- CLRS ch. 21 — disjoint sets, the rank/compression analysis, and the connected-components application.
- [CP-Algorithms: Disjoint Set Union](https://cp-algorithms.com/data_structures/disjoint_set_union.html) — the practical variants: rollback, weighted DSU, offline dynamic connectivity.
- Felzenszwalb & Huttenlocher, "Efficient Graph-Based Image Segmentation" (2004) — DSU as the engine of a real algorithm outside textbooks.
- Related in this repo: [Complexity Analysis](../complexity-analysis/learning.md) (amortized analysis via the potential method — DSU is its hardest classic example), [Rust for Data Structures](../rust-for-data-structures/learning.md) (flat-array representation; the recursive-`find` stack risk), [Hash Tables](../hash-tables/learning.md) (interning arbitrary keys down to `0..n` IDs), [Binary Search Trees](../binary-search-trees/learning.md) (contrast: a structure whose shape *is* the semantics, so it can't be rewritten freely), [Sharding](../../architecture-patterns/sharding/learning.md) (equivalence classes and rebalancing, one scale up).
