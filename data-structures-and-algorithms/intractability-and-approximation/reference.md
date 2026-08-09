# Intractability & Approximation — Quick Reference

## At a Glance

**The most valuable thing here is knowing when to stop.** Recognizing NP-hardness *before* implementing is worth more than any algorithm.

| Class | Means |
| --- | --- |
| **P** | solvable in polynomial time |
| **NP** | a solution can be **verified** in polynomial time |
| **NP-complete** | in NP, and everything in NP reduces to it |
| **NP-hard** | at least as hard as NP-complete |

**Reduction direction:** `A ≤ₚ B` means **B is at least as hard as A**. To prove B hard, reduce a *known-hard* A **to** B. Encoding your problem *to* SAT is a **solution strategy**, not a hardness proof.

## The Cliff — one word apart

| Polynomial | NP-hard |
| --- | --- |
| **Shortest** path | **Longest** path |
| Euler circuit (edges) | Hamiltonian circuit (vertices) |
| **2**-SAT | **3**-SAT |
| Independent set on **bipartite** | on **general** graphs |
| Minimum spanning **tree** | Minimum **Steiner** tree |
| Linear programming | **Integer** LP |

## The Numbers (measured)

Exact TSP, bitmask DP Θ(2ⁿ·n²):

| n | 8 | 12 | 16 | 18 | **20** |
| --- | --- | --- | --- | --- | --- |
| Time | 23.6 µs | 687 µs | 35.6 ms | 101 ms | **402 ms** |

~**4× per two cities**. n=30 ≈ 7 min · n=40 ≈ 5 days · n=50 ≈ 14 years.

**Solution quality on the same instances:**

| n | Exact | NN heuristic (**no guarantee**) | 2-approx (**proven 2×**) |
| --- | --- | --- | --- |
| 8 | 2664.0 | **1.00×** | 1.20× |
| 18 | 3536.6 | 1.19× | 1.73× |
| 20 | 3637.9 | **1.03×** | 1.63× |

> **The unguaranteed heuristic beat the guaranteed one, consistently.** A ratio is a *worst-case* promise, not a prediction.

## Restoring Structure (check this first!)

| Restriction | Makes tractable |
| --- | --- |
| **Bipartite** | Independent set, vertex cover, 2-colouring |
| **DAG** | **Longest path** — Θ(V+E) |
| Planar | 4-colouring; PTAS for many |
| Bounded treewidth | Almost everything (DP over decomposition) |
| Interval graph | Colouring, independent set — greedy |
| **2** clauses not 3 | 2-SAT via SCC |
| Small numeric values | Knapsack pseudo-poly DP |

## Approximability

| Class | Example |
| --- | --- |
| **FPTAS** (1+ε, poly in n and 1/ε) | Knapsack |
| **PTAS** | Euclidean TSP |
| **Constant** | Metric TSP 1.5 (Christofides), vertex cover 2 |
| **Logarithmic** | Set cover ln n — **provably optimal unless P=NP** |
| **Inapproximable** | General TSP, max clique |

## The Four Responses

1. **Exact** on small n (bitmask DP viable to n≈20 measured; branch and bound)
2. **Approximation** with a stated ratio
3. **Heuristic** with none — often better in practice
4. **Restrict the problem** — exploit structure

## Choose This When

| Use | For |
| --- | --- |
| **Check structure first** | Bipartite/DAG/planar/small values |
| **ILP / SAT solver** | Large but structured — **usually the right answer** |
| Exact DP / branch and bound | n small |
| Heuristic + local search | Need an answer now, no bound needed |
| Approximation | You must **state** a guarantee |
| FPTAS | Knapsack-shaped, can name an ε |
| **Change the problem** | The tractable variant is good enough |

Rust solvers: `good_lp` (MILP), `russcip` (SCIP), `varisat`/`splr` (SAT).

## Rules of Thumb

- Pattern-match against SAT / vertex cover / set cover / TSP / bin packing / colouring / subset sum / clique / Hamiltonian **before** implementing.
- Real instances have structure that worst-case analysis ignores — try a solver.
- A ratio is worst-case; **benchmark on your data**.
- Run the heuristic, compute a bound as a **certificate** of the gap.
- **Pseudo-polynomial** (Θ(n·W)) is exponential in input *size*.
- Branch and bound lives or dies on bound quality.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Didn't recognize NP-hardness | Works on tests; never finishes in prod |
| Guarantee treated as prediction | Picked the worse algorithm |
| Hand-rolled search vs a solver | Weeks of work, beaten by a day's encoding |
| Θ(n·W) with W = 10⁹ | Unallocatable table |
| Reduced in the wrong direction | "Proved" hardness by showing it's easy |

## Key References

- Garey & Johnson, *Computers and Intractability* — the canonical NP-hard catalogue
- Vazirani, *Approximation Algorithms* · Williamson & Shmoys (free online)
- CLRS ch. 34–35
