# Intractability & Approximation — Learning Notes

## Mental Model

**The most valuable thing this topic teaches is when to stop.** Recognizing that a problem is NP-hard *before* spending a week on an exact algorithm is worth more than any algorithm in this repo — because the week is unrecoverable and the algorithm doesn't exist.

The classes, stated usefully rather than formally:

- **P** — solvable in polynomial time. Sorting, shortest paths, matching, max-flow, MST.
- **NP** — a proposed solution can be *verified* in polynomial time. Note this says nothing about finding one.
- **NP-complete** — in NP, and every NP problem reduces to it. SAT, 3-SAT, vertex cover, Hamiltonian path, subset sum, graph colouring.
- **NP-hard** — at least as hard as NP-complete, possibly not in NP itself. TSP (the optimization version), halting problem.

The practical consequence: **if a problem is NP-hard, no polynomial algorithm is known and finding one would resolve P vs NP.** You will not find one by trying harder.

What makes this genuinely difficult in practice is that the tractable and intractable versions look identical:

| Polynomial | NP-hard | Difference |
| --- | --- | --- |
| **Shortest** path | **Longest** path | one word |
| Euler circuit (every *edge* once) | Hamiltonian circuit (every *vertex* once) | edge vs vertex |
| 2-SAT | 3-SAT | clause size |
| Bipartite matching | General matching with weights on hypergraphs | structure |
| Max independent set on **bipartite** | Max independent set on **general** graphs | graph class |
| Minimum spanning **tree** | Minimum **Steiner** tree | optional intermediate nodes |
| Linear programming | **Integer** linear programming | continuous vs integral |

Every left-hand entry appears earlier in this repo. Every right-hand entry is a cliff, and nothing in the problem statement warns you.

Now the measured reality of the exponential wall — exact TSP by bitmask [dynamic programming](../dynamic-programming/learning.md), Θ(2ⁿ·n²):

| n | Exact DP time |
| --- | --- |
| 8 | 23.58 µs |
| 12 | 686.83 µs |
| 16 | 35.55 ms |
| 18 | 101.39 ms |
| **20** | **401.81 ms** |

Roughly **4× per two additional cities**. Extrapolating: n = 30 is ~7 minutes, n = 40 is ~5 days, n = 50 is ~14 years. The wall is not gradual.

And the finding that makes this topic honest — comparing the exact optimum against two heuristics on the same instances:

| n | Exact | Nearest-neighbour heuristic | **NN ratio** | 2-approximation (2×MST) |
| --- | --- | --- | --- | --- |
| 8 | 2664.0 | 2673.7 | **1.00×** | 3199.0 |
| 16 | 4050.7 | 4304.3 | **1.06×** | 6122.7 |
| 18 | 3536.6 | 4216.2 | 1.19× | 6116.6 |
| 20 | 3637.9 | 3745.6 | **1.03×** | 5912.6 |

**The heuristic with no guarantee (nearest neighbour, 1.00–1.19× of optimal) consistently beat the algorithm with a proven 2× bound (1.20–1.63×).** That's the practical lesson: a proven approximation ratio is a *worst-case* promise, and a good heuristic often does far better in the average case while promising nothing.

## The Invariant

**What a reduction proves:**

> If problem A reduces to problem B in polynomial time (`A ≤ₚ B`), then **B is at least as hard as A**. To prove B is NP-hard, reduce a *known* NP-hard problem **to** B — not the other way round.

The direction is the thing people invert. Reducing B to SAT proves nothing about B's hardness (it proves B is *no harder* than SAT); reducing SAT to B proves B is at least as hard as SAT.

**What an approximation ratio means:**

> An algorithm is a **ρ-approximation** if for every input, `cost(returned) ≤ ρ · cost(optimal)` (minimization). The guarantee is **worst-case over all inputs**, not typical.

That's exactly why the measured nearest-neighbour heuristic beat the 2-approximation: 2×MST is *guaranteed* never to exceed twice optimal, and nearest-neighbour is guaranteed nothing — but on random Euclidean instances, "guaranteed nothing" was 1.03× while "guaranteed 2×" was 1.63×.

**The hierarchy of what's achievable:**

| Class | Means | Example |
| --- | --- | --- |
| **FPTAS** | (1+ε) for any ε, polynomial in n *and* 1/ε | Knapsack |
| **PTAS** | (1+ε) for any ε, polynomial in n (maybe exponential in 1/ε) | Euclidean TSP |
| **Constant-factor** | Fixed ρ | Metric TSP (1.5, Christofides), vertex cover (2) |
| **Logarithmic** | ρ = O(log n) | Set cover (ln n — **provably optimal unless P=NP**) |
| **Inapproximable** | No constant ρ unless P=NP | General TSP, max clique |

The set-cover entry is worth internalizing: greedy achieves ln n, and **that is the best possible** — no cleverness will improve it, which converts "can we do better?" from an open question into a settled one.

## Mechanics

### Recognizing NP-hardness

The practical test is pattern-matching against the canonical list. If your problem is one of these in disguise, stop:

| Canonical problem | Disguises |
| --- | --- |
| **SAT / 3-SAT** | Any "assign values satisfying constraints" with ≥3-way clauses |
| **Vertex cover** | "Smallest set of X covering all Y" |
| **Set cover** | Same shape; also feature selection, test-suite minimization |
| **TSP** | Routing, drilling order, sequencing with transition costs |
| **Bin packing** | VM placement, cutting stock, memory allocation |
| **Graph colouring** | Register allocation, scheduling with conflicts, frequency assignment |
| **Subset sum / knapsack** | Budget allocation, partitioning by value |
| **Clique / independent set** | "Largest mutually compatible set" |
| **Hamiltonian path** | "Visit every X exactly once" |

**Then check for restoring structure.** Many NP-hard problems become polynomial on restricted inputs, and real instances are often restricted:

| Restriction | Makes tractable |
| --- | --- |
| **Bipartite** graph | Independent set, vertex cover, colouring (2 colours) |
| **DAG** | Longest path — Θ(V+E) by [DAG relaxation](../shortest-paths/learning.md) |
| **Planar** graph | 4-colouring; PTAS for many problems |
| **Bounded treewidth** | Almost everything, by DP over the tree decomposition |
| **Interval** graph | Colouring, independent set — greedy works |
| **2 clauses** instead of 3 | 2-SAT via [SCC](../advanced-graph-algorithms/learning.md) |
| Small numeric values | Knapsack via pseudo-polynomial DP |

That table is the single highest-value part of this topic: **check whether your instance has structure before concluding it's hopeless.**

### The four responses to intractability

1. **Exact on small instances** — bitmask DP (measured: n = 20 in 400 ms, n = 25 impractical), branch and bound, or an ILP/SAT solver.
2. **Approximation with a proven ratio** — when you need a bound you can state.
3. **Heuristic with no guarantee** — often better in practice, as measured. Local search, simulated annealing, genetic algorithms, beam search.
4. **Restrict the problem** — exploit structure, or accept a slightly different problem that is tractable.

**Modern SAT and ILP solvers deserve more respect than they get.** They routinely handle instances with millions of variables that the theory says are intractable, because real instances have structure that worst-case analysis ignores. Encoding your problem for a solver is frequently faster to write *and* faster to run than a hand-rolled search — `good_lp`, `russcip`, or `varisat` in Rust.

### Branch and bound

The exact-but-pruned approach, and the reason [backtracking](../recursion-and-backtracking/learning.md)'s pruning discussion matters here:

```
maintain the best complete solution found so far
at each node, compute a BOUND on the best achievable completion
if bound is worse than the incumbent, prune the entire subtree
```

The bound's quality is everything — a tight bound prunes most of the tree, a loose one degenerates to exhaustive search. For TSP, the MST of the unvisited set is a standard lower bound. Measured in [backtracking](../recursion-and-backtracking/learning.md), pruning gave **43,580×** on N-queens at n=9, and the same mechanism applies here.

## Complexity

| Problem | Best known exact | Best approximation |
| --- | --- | --- |
| **TSP (general)** | Θ(2ⁿ·n²) DP | **inapproximable** within any constant |
| TSP (metric) | Θ(2ⁿ·n²) | **1.5** (Christofides) |
| TSP (Euclidean) | Θ(2ⁿ·n²) | **PTAS** (1+ε) |
| **Vertex cover** | Θ(1.28ⁿ) | **2** (greedy matching) |
| **Set cover** | Θ(2ⁿ) | **ln n — provably optimal** |
| **Knapsack** | Θ(n·W) pseudo-poly | **FPTAS** (1+ε) |
| Bin packing | Θ(2ⁿ) | 11/9·OPT + 6/9 (FFD) |
| **Graph colouring** | Θ(2ⁿ) | n^(1−ε) — essentially hopeless |
| **Max clique** | Θ(1.19ⁿ) | n^(1−ε) — hopeless |
| **3-SAT** | Θ(1.31ⁿ) | — (decision problem) |
| **Longest path** | Θ(2ⁿ·n²) | no constant factor |

**Where the table misleads.** These are *worst-case* statements, and modern solvers routinely solve instances the table calls intractable — a 10,000-city TSP has been solved to proven optimality, and SAT solvers handle industrial instances with millions of clauses. The bounds describe adversarial inputs; real inputs have structure.

Conversely, the approximation column understates practical quality in the other direction: measured, a 2-approximation delivered 1.20–1.63× and an *unguaranteed* heuristic delivered 1.00–1.19×. **Neither the hardness bound nor the approximation bound predicts what you'll actually observe.**

## Use Cases

- **Scheduling and rostering** — almost always NP-hard; solved with ILP solvers or metaheuristics in practice.
- **Vehicle routing** — TSP with capacity and time windows; the industry runs on heuristics (LKH, OR-Tools) plus local search.
- **Register allocation** — graph colouring; compilers use a linear-scan heuristic or Chaitin-style colouring with spilling.
- **VM / container placement** — bin packing; Kubernetes' scheduler is a scoring heuristic, not an optimizer.
- **Feature selection and test-suite minimization** — set cover; greedy's ln n is used because it's provably the best available.
- **Circuit design and verification** — SAT solvers are the workhorse; the field's practical success is why SAT solving improved so dramatically.
- **Protein folding, drug design** — inapproximable in general; addressed with physics-informed heuristics and now ML.
- **Compiler optimization** — instruction scheduling and many other passes are NP-hard, which is why compilers use heuristics with tunable effort levels.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Check for structure first** | Bipartite, DAG, planar, bounded treewidth, small values |
| Exact DP / branch and bound | n small — measured, bitmask TSP is viable to n ≈ 20 |
| **ILP / SAT solver** | Instances are large but structured — **usually the right answer** |
| **Heuristic** (greedy, local search) | You need an answer now and no bound is required |
| Approximation with a ratio | You must *state* a guarantee (SLA, contract, paper) |
| FPTAS | Knapsack-shaped, and you can name an ε |
| Metaheuristic (annealing, GA) | Large, messy, no structure — and you can afford tuning |
| **Change the problem** | The tractable variant is good enough for the business need |

## Pitfalls in Depth

### Pitfall: Not recognizing NP-hardness

- **What goes wrong:** A week is spent building an exact algorithm for what turns out to be graph colouring, bin packing, or longest path. It works on test instances and never finishes on real ones. The exponential wall is abrupt — measured, exact TSP goes from 35.55 ms at n = 16 to 401.81 ms at n = 20, and extrapolates to ~5 days at n = 40.
- **Why it happens (the mechanism):** Nothing in a problem statement signals hardness, and the tractable neighbours are seductive: shortest path is Θ(E log V) and longest path is NP-hard on the same graph; 2-SAT is linear and 3-SAT is NP-complete. The difference is one word or one parameter, and both versions are equally easy to *state*.
- **How to handle it in production, and why that works:** Before implementing, pattern-match against the canonical NP-hard list — SAT, vertex cover, set cover, TSP, bin packing, colouring, subset sum, clique, Hamiltonian path. If it matches, **then** check whether your instances have restoring structure (bipartite, DAG, planar, bounded treewidth, small numeric values), because real instances often do and the restricted version is frequently polynomial.
- **Trade-offs of the fix:** Recognizing hardness requires knowing the list, which is a real prerequisite. And the classification is worst-case — declaring a problem hopeless when a solver would handle your specific instances in seconds is the mirror mistake, which is why "try a solver" belongs in the response set.

### Pitfall: Treating an approximation guarantee as a performance prediction

- **What goes wrong:** A 2-approximation is chosen over an unguaranteed heuristic on the reasoning that the bound makes it better. Measured on the same TSP instances, the 2-approximation (2×MST) produced tours **1.20–1.63×** optimal while nearest-neighbour — which guarantees nothing — produced **1.00–1.19×**. The guaranteed algorithm was consistently worse.
- **Why it happens (the mechanism):** An approximation ratio is a **worst-case** bound over all possible inputs, including adversarial ones constructed to defeat the algorithm. It says nothing about typical inputs. A heuristic optimized for typical structure can beat it routinely while retaining a terrible worst case that never occurs in your data.
- **How to handle it in production, and why that works:** Use the guarantee when you must *state* one — an SLA, a contract, a correctness argument — and otherwise **benchmark both on your actual data**. Better: run the heuristic and compute the guaranteed algorithm's bound (or an LP relaxation) as a *certificate* of how close you are, which gives you the practical quality of the heuristic plus a provable bound on the gap.
- **Trade-offs of the fix:** Benchmarking requires representative instances, which you may not have early. And a heuristic with no bound can fail badly on an input shift you didn't anticipate — the guarantee's value is precisely that it survives distribution changes.

### Pitfall: Hand-rolling a search instead of using a solver

- **What goes wrong:** A custom branch-and-bound or genetic algorithm is written for a scheduling or assignment problem. It takes weeks, is hard to modify when constraints change, and is beaten by an off-the-shelf ILP solver given a straightforward encoding.
- **Why it happens (the mechanism):** NP-hard reads as "no good general tool exists", so people build a bespoke one. But SAT and ILP solvers embody decades of engineering — clause learning, restarts, cutting planes, presolve — that no hand-rolled search reproduces, and they exploit structure in real instances that worst-case analysis says shouldn't help.
- **How to handle it in production, and why that works:** Encode the problem for a solver first, as a baseline. It's usually a day's work, it gives an exact answer or a proven bound, and it's trivially adaptable when a constraint changes (add a row, don't rewrite a search). In Rust: `good_lp` (LP/MILP front end), `russcip` (SCIP bindings), `varisat`/`splr` (SAT). Reach for a custom algorithm only when the solver demonstrably can't handle your instances.
- **Trade-offs of the fix:** Solvers add a dependency, sometimes a licence concern for the commercial ones, and have unpredictable runtime — a solver that usually finishes in seconds may occasionally take hours on a similar instance. For hard real-time requirements a bounded-effort heuristic is more appropriate.

### Pitfall: Pseudo-polynomial mistaken for polynomial

- **What goes wrong:** Knapsack's Θ(n·W) DP is used with capacities in the billions, or subset-sum with large values. The bound looks polynomial and the table is unallocatable — at n = 100 and W = 10⁹ that's 10¹¹ cells.
- **Why it happens (the mechanism):** `W` is the *numeric value* of the capacity, not the size of the input. Encoding W takes log W bits, so runtime is exponential in the input *length* — the definition of pseudo-polynomial. This is exactly why subset sum is NP-complete despite having a "polynomial" DP, and it's the same trap as min-cost flow's `F` in [advanced graph algorithms](../advanced-graph-algorithms/learning.md) and factorization's Θ(√n) in [number theory](../number-theory-and-combinatorics/learning.md).
- **How to handle it in production, and why that works:** Check whether the *numeric range* is small before choosing the DP. If capacities are huge but values are small, flip the DP to index by value — Θ(n·V) — which is often the tractable direction. If both are large, use the **FPTAS**: round values to `⌊v·n/(ε·v_max)⌋`, run the DP on the scaled values, and get a (1−ε)-optimal answer in polynomial time.
- **Trade-offs of the fix:** Flipping the dimension only helps when one range is small. The FPTAS gives up exactness for a tunable ε and its runtime grows as 1/ε, so a very tight ε is expensive. Both beat a table you can't allocate.

### Pitfall: Reducing in the wrong direction

- **What goes wrong:** Someone "proves" their problem is hard by reducing it *to* SAT — showing they can encode it as SAT. That demonstrates the problem is **no harder** than SAT, which is the opposite of a hardness proof. Effort is then abandoned on a problem that may well be in P.
- **Why it happens (the mechanism):** Both directions are called "a reduction" and both involve translating one problem into another, so the asymmetry is easy to lose. `A ≤ₚ B` means "A is no harder than B" — so to show B is hard you need a *known-hard* A reducing **to** B, using B as a subroutine to solve A.
- **How to handle it in production, and why that works:** State the direction explicitly: "if I could solve B efficiently, I could solve 3-SAT efficiently — therefore B is NP-hard." That sentence form makes the direction unambiguous and is the standard proof shape. And practically: encoding your problem *to* SAT isn't a hardness proof, it's a **solution strategy** — which is the useful thing to do with it.
- **Trade-offs of the fix:** Constructing a genuine hardness proof is real work and usually unnecessary in engineering — knowing the problem *contains* a known NP-hard problem as a special case is enough to stop looking for a polynomial algorithm, and that's a much lighter argument.
