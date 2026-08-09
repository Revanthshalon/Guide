# Data Structures & Algorithms — Learning Index

The full curriculum for this category: every topic worth learning to reach mastery, in the order to learn it. Prerequisites for each entry are all *above* it.

Topic folders are scaffolded as they're started (from [_template-learning.md](_template-learning.md) and [_template-reference.md](_template-reference.md)), not all at once — this index is the roadmap, and links go live as folders appear. **✅ marks a written topic.**

## What "mastery" means here

Not "can recite the algorithm." Three specific things:

1. **Derive, don't recall.** Given a problem shape, you can reconstruct the structure that fits it — because you know which invariant buys which operation, not because you memorized a name.
2. **Know the real cost.** You know where the asymptotic bound lies: which constant is huge, where cache behavior inverts the ranking, and the actual *n* at which the "worse" algorithm wins. This is where this category meets [performance-optimization](../performance-optimization/LEARNING-INDEX.md).
3. **Implement it in Rust without fighting the language.** Ownership makes some textbook structures (doubly-linked lists, parent pointers, graph nodes) hostile. Knowing the standard answers — arena + index handles, `Rc<RefCell>`, split borrows, when `unsafe` is genuinely warranted — is part of the skill, not a detour.

**The loop for each topic:** derive it on paper → implement in Rust → benchmark against the std equivalent → break it deliberately (adversarial input, degenerate case) → answer the doc's creative-thinking questions. A topic you've only read is not a topic you know.

---

## Stage 0 — Foundations

Everything else assumes these two. They're short but non-skippable.

| Topic | Folder | Covers |
| --- | --- | --- |
| Complexity Analysis ✅ | [learning](complexity-analysis/learning.md) · [reference](complexity-analysis/reference.md) | Big-O/Θ/Ω honestly; worst vs. average vs. expected vs. amortized (and the three amortization methods); recurrences and the Master Theorem; why O(n log n) with a bad constant loses to O(n²) at real n; why asymptotics ignore the memory hierarchy and what to do about it |
| Rust for Data Structures ✅ | [learning](rust-for-data-structures/learning.md) · [reference](rust-for-data-structures/reference.md) | The ownership patterns every later topic reuses: arena + index handles vs. `Box` vs. `Rc<RefCell>`; generational indices; `Ord`/`Hash`/`Borrow` contracts; iterator and slice-splitting idioms; where `unsafe` is warranted and how to contain it; benchmarking with criterion |

## Stage 1 — Linear structures

| Topic | Folder | Prereq | Covers |
| --- | --- | --- | --- |
| Arrays & Dynamic Arrays ✅ | [learning](arrays-and-dynamic-arrays/learning.md) · [reference](arrays-and-dynamic-arrays/reference.md) | 0 | `Vec` growth and amortized push; capacity vs. length; slices; why contiguity beats everything at small n; small-vector optimization |
| Linked Lists ✅ | [learning](linked-lists/learning.md) · [reference](linked-lists/reference.md) | 0, arrays | Singly/doubly/circular; why they're rare in practice and hostile in Rust; the arena-and-indices rewrite; intrusive lists; where they still win (O(1) splice, stable addresses, LRU) |
| Stacks & Queues ✅ | [learning](stacks-and-queues/learning.md) · [reference](stacks-and-queues/reference.md) | arrays | `Vec` as stack; `VecDeque` and the ring buffer; deques; bounded vs. unbounded, and the [backpressure](../architecture-patterns/backpressure-and-rate-limiting/learning.md) connection |
| Strings & Text ✅ | [learning](strings-and-text/learning.md) · [reference](strings-and-text/reference.md) | arrays | UTF-8 as a data structure; `String` vs. `&str` vs. `Cow`; byte vs. char vs. grapheme indexing; interning; ropes and gap buffers for editable text |

## Stage 2 — Searching & sorting

| Topic | Folder | Prereq | Covers |
| --- | --- | --- | --- |
| Binary Search ✅ | [learning](binary-search/learning.md) · [reference](binary-search/reference.md) | 1 | The predicate/monotone-boundary generalization (far more useful than "find in sorted array"); lower/upper bound; `partition_point`; binary search on the *answer*; the off-by-one taxonomy; branchless and interpolation variants |
| Sorting ✅ | [learning](sorting/learning.md) · [reference](sorting/reference.md) | 1, binary search | Merge/quick/heap; pdqsort (what `sort_unstable` actually runs); stability and when it matters; sort keys and decorate-sort-undecorate; non-comparison sorts (counting, radix, bucket) and their preconditions; external merge sort; the lower bound and how to escape it |
| Selection & Order Statistics ✅ | [learning](selection-and-order-statistics/learning.md) · [reference](selection-and-order-statistics/reference.md) | sorting | Quickselect, median-of-medians, `select_nth_unstable`; top-k via heap vs. partial sort vs. quickselect; streaming quantiles as the preview of Stage 8 |

## Stage 3 — Hashing

| Topic | Folder | Prereq | Covers |
| --- | --- | --- | --- |
| Hash Tables ✅ | [learning](hash-tables/learning.md) · [reference](hash-tables/reference.md) | 1 | Chaining vs. open addressing; linear/quadratic probing and Robin Hood; SwissTable (hashbrown, and therefore `HashMap`) with its SIMD probe; load factor, resizing, tombstones; iteration-order non-determinism; why a hash map's constant is not the asymptotic story |
| Hashing Techniques ✅ | [learning](hashing-techniques/learning.md) · [reference](hashing-techniques/reference.md) | hash tables | What a good hash function is; SipHash vs. aHash/FxHash and the HashDoS trade-off; the `Hash`/`Eq` contract and how to break it; rolling hashes (Rabin-Karp); fingerprinting; consistent hashing (see [sharding](../architecture-patterns/sharding/learning.md)); perfect hashing |

## Stage 4 — Trees & priority structures

| Topic | Folder | Prereq | Covers |
| --- | --- | --- | --- |
| Binary Search Trees & Balancing ✅ | [learning](binary-search-trees/learning.md) · [reference](binary-search-trees/reference.md) | 2 | The BST invariant; degeneration; rotations; AVL vs. red-black vs. treap vs. splay — what each optimizes; augmented BSTs (order statistics, interval trees); why Rust pushes you to arenas here |
| B-Trees ✅ | [learning](b-trees/learning.md) · [reference](b-trees/reference.md) | BSTs | Why `BTreeMap` beats a red-black tree on real hardware; node fanout as a cache/page decision; B+ trees and range scans; the bridge to on-disk indexes and Stage 9 |
| Heaps & Priority Queues ✅ | [learning](heaps-and-priority-queues/learning.md) · [reference](heaps-and-priority-queues/reference.md) | 2 | Binary heap as implicit tree; sift up/down; heapify in O(n); `BinaryHeap` and the `Reverse` trick; d-ary, pairing, Fibonacci heaps and why the last one loses in practice; decrease-key and its Dijkstra consequence; indexed heaps |
| Tries & Radix Trees ✅ | [learning](tries-and-radix-trees/learning.md) · [reference](tries-and-radix-trees/reference.md) | 1, strings | Prefix trees; compressed/PATRICIA radix trees; ART; memory blowup and the map-per-node question; autocomplete, routing tables, IP lookup |
| Range Query Structures ✅ | [learning](range-query-structures/learning.md) · [reference](range-query-structures/reference.md) | heaps, arrays | Prefix sums → Fenwick (BIT) → segment tree; lazy propagation; sparse tables for idempotent queries; sqrt decomposition; choosing among them by update/query mix |
| Disjoint Set Union ✅ | [learning](disjoint-set-union/learning.md) · [reference](disjoint-set-union/reference.md) | 1 | Union by rank/size + path compression; the inverse-Ackermann bound; the "merge equivalence classes" problem shape; Kruskal, connectivity, offline dynamic connectivity |

## Stage 5 — Graphs

| Topic | Folder | Prereq | Covers |
| --- | --- | --- | --- |
| Graph Representations ✅ | [learning](graph-representations/learning.md) · [reference](graph-representations/reference.md) | 1, 3 | Adjacency list vs. matrix vs. CSR; when the representation *is* the optimization; index-based nodes in Rust; `petgraph`; implicit graphs (the grid/state-space you never materialize) |
| Graph Traversal ✅ | [learning](graph-traversal/learning.md) · [reference](graph-traversal/reference.md) | representations, stacks/queues | BFS, DFS, iterative vs. recursive (and stack overflow); the DFS edge classification; topological sort (Kahn and DFS); cycle detection; connected components; bipartite check; multi-source and 0-1 BFS |
| Shortest Paths ✅ | [learning](shortest-paths/learning.md) · [reference](shortest-paths/reference.md) | traversal, heaps | Dijkstra and why it needs non-negative weights; the lazy-deletion heap trick; Bellman-Ford and negative cycles; A* and admissible heuristics; Floyd-Warshall; DAG shortest path via topo order; bidirectional search |
| Minimum Spanning Trees ✅ | [learning](minimum-spanning-trees/learning.md) · [reference](minimum-spanning-trees/reference.md) | DSU, heaps | Kruskal vs. Prim; the cut and cycle properties; Borůvka; MSTs as the exchange-argument teaching case for greedy |
| Advanced Graph Algorithms ✅ | [learning](advanced-graph-algorithms/learning.md) · [reference](advanced-graph-algorithms/reference.md) | all of Stage 5 | SCCs (Tarjan, Kosaraju) and condensation; bridges and articulation points; 2-SAT; LCA (binary lifting, Euler tour + RMQ); heavy-light decomposition; bipartite matching (Hopcroft-Karp); max-flow/min-cut (Dinic) and modeling problems as flow |

## Stage 6 — Algorithm design paradigms

This is the stage that converts "knows structures" into "solves new problems." It's the heart of the category.

| Topic | Folder | Prereq | Covers |
| --- | --- | --- | --- |
| Recursion & Backtracking ✅ | [learning](recursion-and-backtracking/learning.md) · [reference](recursion-and-backtracking/reference.md) | 1 | Recursion as invariant plus base case; the call stack as a data structure; converting to iteration; backtracking as DFS over a decision tree; pruning and constraint propagation; permutations/subsets/N-queens/sudoku as the canonical shapes |
| Divide & Conquer ✅ | [learning](divide-and-conquer/learning.md) · [reference](divide-and-conquer/reference.md) | 2, recursion | The split-solve-combine skeleton; recurrence analysis; merge sort, closest pair, Karatsuba, matrix exponentiation; the parallelism connection ([work stealing](../performance-optimization/parallelism-and-work-stealing/learning.md) is D&C with a scheduler) |
| Greedy Algorithms ✅ | [learning](greedy-algorithms/learning.md) · [reference](greedy-algorithms/reference.md) | sorting, heaps, MSTs | Why greedy is the hardest paradigm to trust; proving it with exchange arguments and the greedy-stays-ahead pattern; matroid intuition for when greedy is *guaranteed*; interval scheduling, Huffman, fractional knapsack; the near-miss cases that make it fail |
| Dynamic Programming ✅ | [learning](dynamic-programming/learning.md) · [reference](dynamic-programming/reference.md) | recursion, D&C | State design as the whole skill; memoization vs. tabulation; 1-D/2-D/knapsack/LIS/edit distance/interval DP; DP on trees; bitmask DP; digit DP; space-reduction by rolling arrays; optimizations (monotonic deque, divide-and-conquer opt, convex hull trick); recognizing "this is DP" from problem shape |
| Two Pointers & Sliding Window ✅ | [learning](two-pointers-and-sliding-window/learning.md) · [reference](two-pointers-and-sliding-window/reference.md) | 1, 2 | The monotone-window invariant; fixed vs. variable windows; when a window can shrink; the "why is this not O(n²)" amortization argument |
| Prefix Sums & Difference Arrays ✅ | [learning](prefix-sums-and-difference-arrays/learning.md) · [reference](prefix-sums-and-difference-arrays/reference.md) | 1 | 1-D and 2-D prefix sums; difference arrays for range updates; XOR/product prefixes; the preprocess-once-answer-many trade; the bridge to Fenwick trees |
| Monotonic Stack & Queue ✅ | [learning](monotonic-stack-and-queue/learning.md) · [reference](monotonic-stack-and-queue/reference.md) | stacks/queues | The next-greater-element family; sliding-window min/max in O(n); largest rectangle in histogram; recognizing the "each element pushed and popped once" amortization |
| Bit Manipulation ✅ | [learning](bit-manipulation/learning.md) · [reference](bit-manipulation/reference.md) | 0 | Masks, shifts, and the standard idioms; `count_ones`/`leading_zeros`/`trailing_zeros`; subset enumeration; bitsets as sets; XOR tricks and their invariants; bit tricks as the interface to [SIMD](../performance-optimization/simd/learning.md) |
| Intervals & Sweep Line ✅ | [learning](intervals-and-sweep-line/learning.md) · [reference](intervals-and-sweep-line/reference.md) | sorting, heaps | Merging/inserting intervals; the sweep-line event model; meeting-rooms via heap; coordinate compression; the geometry preview |

## Stage 7 — String algorithms

| Topic | Folder | Prereq | Covers |
| --- | --- | --- | --- |
| String Matching ✅ | [learning](string-matching/learning.md) · [reference](string-matching/reference.md) | strings, hashing | Naive → KMP (the failure function as the real lesson) → Z-algorithm → Rabin-Karp → Boyer-Moore; what `memmem`/`str::find` actually does; Aho-Corasick for multi-pattern |
| Suffix Structures ✅ | [learning](suffix-structures/learning.md) · [reference](suffix-structures/reference.md) | string matching, sorting, tries | Suffix arrays + LCP (Kasai); suffix automaton; suffix trees and why you rarely build one; the problems these make trivial (longest repeated/common substring, distinct substrings) |
| Edit Distance & Alignment ✅ | [learning](edit-distance-and-alignment/learning.md) · [reference](edit-distance-and-alignment/reference.md) | DP, strings | Levenshtein and its variants; the DP table and its space reduction; Myers' bit-parallel algorithm; diff as an LCS problem; fuzzy matching in practice |

## Stage 8 — Randomized & probabilistic

| Topic | Folder | Prereq | Covers |
| --- | --- | --- | --- |
| Randomized Algorithms ✅ | [learning](randomized-algorithms/learning.md) · [reference](randomized-algorithms/reference.md) | 2, 3 | Las Vegas vs. Monte Carlo; randomized quicksort/quickselect; reservoir sampling; shuffling correctly (Fisher-Yates, and the biased version everyone writes); skip lists and treaps as "balancing by coin flip"; randomness as the defense against adversarial input |
| Probabilistic Data Structures ✅ | [learning](probabilistic-data-structures/learning.md) · [reference](probabilistic-data-structures/reference.md) | hashing, randomized | Bloom and counting/cuckoo filters; Count-Min sketch; HyperLogLog; t-digest/quantile sketches; the accuracy-vs-space dial; false-positive math and how to size them; where these show up in [caching](../architecture-patterns/caching-strategies/learning.md) and LSM reads |

## Stage 9 — Systems-scale structures

Where this category and [performance-optimization](../performance-optimization/LEARNING-INDEX.md) merge. Read after that category's Stage 1–2 (cache locality, memory layout) if you can.

| Topic | Folder | Prereq | Covers |
| --- | --- | --- | --- |
| Cache-Aware & Cache-Oblivious Structures ✅ | [learning](cache-aware-structures/learning.md) · [reference](cache-aware-structures/reference.md) | 4, cache locality | Why B-trees beat BSTs and flat arrays beat linked structures; van Emde Boas layout; Eytzinger binary search; the external-memory (I/O) model; measuring instead of assuming |
| LSM Trees & Write-Optimized Structures ✅ | [learning](lsm-trees/learning.md) · [reference](lsm-trees/reference.md) | B-trees, sorting, probabilistic | Memtable/SSTable/compaction; read vs. write vs. space amplification as the real trilemma; bloom filters on the read path; how this differs from a B-tree index and why databases pick one |
| Persistent & Immutable Structures ✅ | [learning](persistent-immutable-structures/learning.md) · [reference](persistent-immutable-structures/reference.md) | 4 | Path copying; persistent lists/trees; HAMTs and RRB-trees (`im`, `rpds`); structural sharing as the cost model; partial vs. full vs. confluent persistence; the connection to [event sourcing](../architecture-patterns/event-sourcing/learning.md) |
| Concurrent Data Structures ✅ | [learning](concurrent-data-structures/learning.md) · [reference](concurrent-data-structures/reference.md) | 1, 3, lock-free | Lock-based vs. lock-free; Treiber stack, Michael-Scott queue; concurrent maps (`DashMap`, sharded locks); epoch-based reclamation (`crossbeam-epoch`) and the ABA problem; RCU; when a `Mutex<HashMap>` is simply correct — see [lock-free concurrency](../performance-optimization/lock-free-concurrency/learning.md) |
| Spatial Data Structures ✅ | [learning](spatial-data-structures/learning.md) · [reference](spatial-data-structures/reference.md) | 4 | k-d trees, quadtrees/octrees, R-trees, BVHs; grid hashing; nearest-neighbour and range queries; the curse of dimensionality; ANN indexes (HNSW, IVF) as the modern application |

## Stage 10 — Limits and the mathematical toolkit

| Topic | Folder | Prereq | Covers |
| --- | --- | --- | --- |
| Computational Geometry | `computational-geometry/` | sorting, sweep line | Orientation predicates and robustness (floating point *is* the hard part); convex hull (Andrew's monotone chain); segment intersection; polygon area and point-in-polygon; closest pair |
| Number Theory & Combinatorics | `number-theory-and-combinatorics/` | 0 | GCD/extended Euclid; modular arithmetic and inverses; fast exponentiation; sieves; factorization; combinatorics and Pascal/Lucas; matrix exponentiation for linear recurrences; FFT/NTT for convolution |
| Intractability & Approximation | `intractability-and-approximation/` | 5, 6 | P/NP/NP-complete without hand-waving; recognizing an NP-hard problem *before* burning a week on it; reductions; approximation ratios; heuristics that work (local search, simulated annealing, beam search); parameterized tractability; when to call an ILP/SAT solver instead |
| Streaming & Online Algorithms | `streaming-and-online-algorithms/` | 8 | The streaming model (one pass, sublinear space); online algorithms and competitive ratio; the ski-rental and paging classics — LRU vs. LFU vs. ARC connect straight back to [caching strategies](../architecture-patterns/caching-strategies/learning.md) |

---

## Shorter paths

- **Interview / competitive-programming core:** 0 → 1 → 2 → 3 → 4 (BSTs, heaps, DSU) → 5 (through shortest paths) → 6 (all of it). Stage 6 is where the marks actually are.
- **Backend engineering, not puzzles:** 0 → 1 → 2 → 3 → 4 (B-trees, heaps) → 5 (traversal) → 9 (cache-aware, LSM, concurrent). Skim Stage 6 for DP and greedy recognition.
- **Systems / database work:** 0 → 1 → 2 (incl. external merge sort) → 3 → 4 (B-trees) → 8 → 9 entirely.
- **Building something with graphs in it:** 0 → 1 → 3 → 5 in full, then 6's DP and greedy.

## Cross-category interlocks

| This category | Elsewhere in the repo | The link |
| --- | --- | --- |
| Hash tables, B-trees, cache-aware structures | [Cache Locality](../performance-optimization/cache-locality/learning.md), [Memory Layout](../performance-optimization/memory-layout/learning.md) | The constant factor that decides the real winner |
| Concurrent data structures | [Lock-Free Concurrency](../performance-optimization/lock-free-concurrency/learning.md), [False Sharing](../performance-optimization/false-sharing/learning.md) | Same material from the hardware side |
| Consistent hashing | [Sharding](../architecture-patterns/sharding/learning.md) | The same partitioning problem, one scale up |
| Persistent structures | [Event Sourcing & CQRS](../architecture-patterns/event-sourcing/learning.md) | Immutable history with structural sharing |
| Probabilistic structures, online algorithms | [Caching Strategies](../architecture-patterns/caching-strategies/learning.md) | Bloom filters and eviction policies |
| Rust ownership patterns | [Rust best practices](../language-best-practices/rust/learning.md) | Arenas, handles, and interior mutability |
| Divide & conquer | [Parallelism & Work Stealing](../performance-optimization/parallelism-and-work-stealing/learning.md) | Work stealing is D&C with a scheduler |
| Complexity analysis | [Profiling & Measurement](../performance-optimization/profiling-and-measurement/learning.md) | The asymptotic claim vs. what the profile says |

## The transformation lenses

Every `learning.md` in this category ends with these. They're the creative engine of the whole subject: most named structures are a simpler one run through one lens. Learn to apply them and you stop needing to memorize the catalogue.

| Lens | Ask | Example of what it produces |
| --- | --- | --- |
| Persist it | What if updates returned a new version? | BST → persistent BST → HAMT |
| Batch it | What if 10,000 updates arrived at once? | B-tree → LSM tree |
| Approximate it | What does 1% error buy? | Set → Bloom filter; counter → HyperLogLog |
| Randomize it | What if a coin flip replaced the balancing? | BST → treap / skip list |
| Externalize it | What if the unit of transfer were a page? | BST → B-tree |
| Parallelize it | Where's the contention point? | Queue → Michael-Scott queue |
| Invert it | Swap which operation is fast | Array vs. linked list; write- vs. read-optimized |
| Augment it | What does one extra field per node buy? | BST → order-statistic tree / interval tree |
| Specialize it | What if keys were small integers? | Comparison sort → radix sort; map → bitset |
| Amortize it | What if one operation could be terrible? | Array → dynamic array; heap → Fibonacci heap |
