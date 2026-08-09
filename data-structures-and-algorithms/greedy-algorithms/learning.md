# Greedy Algorithms — Learning Notes

## Mental Model

**A greedy algorithm makes the locally-best choice at each step and never reconsiders.** That's it — which makes greedy the simplest paradigm to *write* and the hardest to *trust*.

The asymmetry is the whole topic. A greedy algorithm that works is usually a few lines and Θ(n log n); a greedy algorithm that's wrong looks identical and produces plausible answers. **[Dynamic programming](../dynamic-programming/learning.md) explores all choices and is safe by construction; greedy commits and needs a proof.** So the discipline is:

> Never ship a greedy algorithm you haven't proved correct or verified exhaustively against brute force on small inputs.

The canonical illustration: making change for 30 with coins {25, 10, 1}. Greedy takes 25, then five 1s — six coins. The optimum is three 10s. The *same* greedy is optimal for {25, 10, 5, 1} (US coins), which is why the bug hides: it works on the denominations you tested.

Two proof techniques cover almost everything:

- **Exchange argument.** Take any optimal solution; show you can transform it, step by step, into the greedy one without making it worse. Therefore greedy is optimal. This is the workhorse — it's how [MST](../minimum-spanning-trees/learning.md)'s cut property is proved, and Stage 5 already used it.
- **Greedy stays ahead.** Show that after each step, greedy's partial solution is at least as good as any other algorithm's partial solution by some measure. Used for interval scheduling.

And one structural test worth knowing: if the problem's feasible sets form a **matroid**, greedy is *guaranteed* optimal. That's a strong theorem — Kruskal's correctness is the graphic matroid instance — and while you rarely verify the matroid axioms directly, knowing the concept tells you the guarantee has a shape rather than being luck.

## The Invariant

> After every step, the partial solution is **extendable to an optimal solution**. (Equivalently: some optimal solution contains everything greedy has chosen so far.)

This is exactly the [MST](../minimum-spanning-trees/learning.md) invariant generalized, and it's what an exchange argument establishes: each greedy choice preserves the property that *some* optimum agrees with all choices made so far. When the last step completes the solution, the optimum that agrees with everything *is* the greedy solution.

The two structural conditions people memorize are the preconditions for that invariant:

- **Greedy-choice property** — a globally optimal solution can be reached by making locally optimal choices. *This is the one that fails*, and it fails silently.
- **Optimal substructure** — after committing to a choice, the remaining problem is the same problem on a smaller instance. Shared with DP.

Note DP requires only the second. **Greedy requires both**, and the extra requirement is precisely what you must prove.

## Mechanics

### The classic problems, and what makes each work

| Problem | Greedy rule | Why it works |
| --- | --- | --- |
| **Interval scheduling** (max non-overlapping) | Earliest **finish** time | Greedy stays ahead — leaves the most room |
| Interval partitioning (min rooms) | Sort by start; reuse any free room | Lower bound = max overlap, and greedy achieves it |
| **Fractional knapsack** | Highest value/weight ratio | Exchange: swap any lower-ratio unit for a higher one |
| **0-1 knapsack** | — | **Greedy fails.** Use DP |
| **Huffman coding** | Merge the two lowest frequencies | Exchange: the two rarest symbols are siblings at max depth |
| **[Kruskal / Prim](../minimum-spanning-trees/learning.md)** | Cheapest safe edge | Cut property (a matroid) |
| **[Dijkstra](../shortest-paths/learning.md)** | Closest unfinished vertex | Needs non-negative weights |
| Job sequencing with deadlines | Highest profit, latest free slot | Matroid |
| Coin change, arbitrary denominations | — | **Greedy fails.** Use DP |
| Activity selection with weights | — | **Greedy fails.** Use DP |

**Interval scheduling is the one to internalize**, because the *wrong* greedy rules are so tempting: earliest start (a long early interval blocks everything), shortest duration (a short interval can straddle and block two), fewest conflicts (constructible counterexample). Only **earliest finish time** is correct, and the proof is one line — finishing earliest leaves the maximum remaining time, so greedy's k-th chosen interval finishes no later than any other algorithm's k-th.

### The paired near-misses

The most useful way to hold this topic is as pairs where a one-word change flips the answer:

| Greedy works | Greedy fails |
| --- | --- |
| **Fractional** knapsack | **0-1** knapsack |
| Coin change with {1,5,10,25} | Coin change with {1,10,25} |
| Interval scheduling (unweighted) | **Weighted** interval scheduling |
| MST (minimize total) | Shortest path tree (different objective) |
| Dijkstra with non-negative weights | Dijkstra with **negative** weights |
| Huffman (prefix-free codes) | Optimal BST (needs interval DP) |

Every right-hand entry is a problem where a locally-best choice can be globally wrong, and every one of them has a DP solution. **When in doubt, the safe move is DP** — it costs more time but can't be silently wrong.

### Proving it — the exchange argument, concretely

For interval scheduling: let `G = g₁, g₂, …` be greedy's choices sorted by finish time and `O = o₁, o₂, …` any optimal solution. Claim: `finish(gᵢ) ≤ finish(oᵢ)` for all i. Induction — greedy picks the earliest-finishing compatible interval, and by the hypothesis `gᵢ₋₁` finishes no later than `oᵢ₋₁`, so every interval available to `O` at step i is available to `G`. If `O` had more intervals than `G`, greedy would have had a compatible interval left to pick and wouldn't have stopped. Therefore `|G| = |O|`.

That's the shape: **greedy's i-th choice is never worse than the optimum's i-th choice**, so greedy can't run out first.

### Verifying it — brute force on small inputs

The proof is the ideal; the practical safety net is exhaustive verification:

```rust
#[test]
fn greedy_matches_brute_force() {
    for _ in 0..10_000 {
        let instance = random_instance(/* n ≤ 10 */);
        assert_eq!(greedy(&instance), brute_force_optimal(&instance),
                   "counterexample: {instance:?}");
    }
}
```

At n ≤ 10 a brute force over all 2ⁿ subsets is instant, and a wrong greedy rule almost always fails within a few hundred random instances. **This finds the coin-change bug in seconds** and it's far cheaper than constructing a proof. Run it before trusting any greedy you didn't prove.

## Complexity

| Problem | Greedy | DP alternative |
| --- | --- | --- |
| Interval scheduling | **Θ(n log n)** (the sort) | Θ(n log n) weighted |
| Fractional knapsack | **Θ(n log n)** | — |
| 0-1 knapsack | ✗ wrong | Θ(n·W) |
| Huffman | **Θ(n log n)** | — |
| MST (Kruskal) | **Θ(E log E)** — measured 14.63 ms at E=1M | — |
| Coin change, arbitrary | ✗ wrong | Θ(n·amount) |
| Weighted interval scheduling | ✗ wrong | Θ(n log n) |

**Where the table misleads.** Greedy's cost is almost always dominated by a **sort** — the greedy loop itself is Θ(n). So "greedy is fast" really means "sorting is fast", and if the input arrives sorted (or the keys permit a radix sort) greedy becomes Θ(n). This is exactly what made Kruskal beat Prim by 7.4× in Stage 5: `sort_unstable` plus a linear pass over an almost-free DSU.

The ✗ rows matter more than the timings. A wrong greedy is Θ(n log n) and useless; the Θ(n·W) DP is slower and correct.

## Use Cases

- **Scheduling** — meeting rooms, CPU tasks, job shops. Interval scheduling and partitioning are directly applicable.
- **Compression** — Huffman coding in DEFLATE, JPEG, MP3.
- **Network design** — [MST](../minimum-spanning-trees/learning.md) for cabling; Dijkstra for routing.
- **Caching** — eviction policies are greedy heuristics (LRU is greedy on recency); Bélády's optimal is greedy on future use, which is why it's unimplementable but useful as a bound.
- **Resource allocation** — bandwidth, ad slots, VM packing. Note bin packing's greedy (first-fit-decreasing) is an *approximation* at 11/9·OPT + 6/9, not exact.
- **Approximation algorithms** — greedy set cover gives an H(n) ≈ ln n approximation and that's provably the best possible unless P = NP. When exact is impossible, greedy is often the best available.
- **Streaming** — greedy is naturally online: it commits per element without lookahead, which is why it's the paradigm for problems where you can't see the whole input.

## When to Use Which

| Reach for | When |
| --- | --- |
| **Greedy** | You have a proof (exchange argument / stays-ahead / matroid) |
| Greedy + brute-force test | You *believe* it's right — verify before shipping |
| **[DP](../dynamic-programming/learning.md)** | Choices have future consequences you can't rule out |
| Greedy as an **approximation** | The exact problem is NP-hard; a proven ratio is acceptable |
| Greedy as a **heuristic** | No guarantee needed; speed matters; results are checked downstream |
| Exact solver | NP-hard, instances small enough, optimality required |

## Pitfalls in Depth

### Pitfall: An unproved greedy that works on your test data

- **What goes wrong:** A greedy rule is chosen because it seems natural, tested against realistic inputs, and shipped. It's optimal on those inputs and suboptimal on others. The classic: coin change with {25, 10, 1} for 30 — greedy gives six coins (25+1×5), optimal is three (10×3). The same code is optimal for US denominations, so the test suite passes.
- **Why it happens (the mechanism):** Greedy produces a *valid* solution always — just not always an *optimal* one. There's no crash, no assertion, no signal. And suboptimality is often small and data-dependent, so it reads as noise rather than as a bug. The greedy-choice property is a mathematical fact about the problem structure, and nothing in the code asserts it.
- **How to handle it in production, and why that works:** Either prove it (exchange argument or greedy-stays-ahead — both are short when they work) or verify exhaustively against brute force on small random instances, which finds counterexamples in seconds because collisions are dense at small n. If neither succeeds, use DP: it's slower and cannot be silently wrong.
- **Trade-offs of the fix:** The DP is typically Θ(n·W) or Θ(n²) against greedy's Θ(n log n), which can be the difference between feasible and not. And for genuinely NP-hard problems greedy-with-a-proven-ratio is the *right* answer — the fix isn't "always use DP", it's "know which guarantee you have."

### Pitfall: The right idea with the wrong greedy rule

- **What goes wrong:** Interval scheduling is implemented sorting by **start time** (a single long interval starting first blocks everything after it), or by **shortest duration** (a short interval straddling two others blocks both), or by **fewest conflicts**. All are intuitive; all are wrong; only earliest-**finish** is optimal.
- **Why it happens (the mechanism):** Several rules are locally plausible and differ only on specific configurations. Random test data rarely contains the adversarial arrangement — you need an interval that straddles a boundary, or one long interval among many short ones. So the wrong rule passes.
- **How to handle it in production, and why that works:** Once you've decided greedy applies, the remaining question is *which* rule, and that requires the same proof. For interval scheduling, "earliest finish leaves the most remaining time" is the one-line justification, and it's checkable: after each pick, is the remaining feasible region maximal? Brute-force verification distinguishes the rules immediately.
- **Trade-offs of the fix:** None — it's the same amount of code with a different sort key. The cost is entirely in doing the thinking rather than picking the first plausible rule.

### Pitfall: Confusing "greedy is an approximation" with "greedy is optimal"

- **What goes wrong:** Greedy set cover, first-fit-decreasing bin packing, or greedy vertex cover is used and its output treated as optimal. Downstream code assumes optimality — reporting "the minimum number of bins" — when the guarantee is 11/9·OPT + 6/9.
- **Why it happens (the mechanism):** For NP-hard problems greedy is often the *best practical* algorithm, so it gets used routinely, and the approximation ratio is easy to forget once the code works. The output is a valid solution and there's no marker distinguishing "optimal" from "within a factor".
- **How to handle it in production, and why that works:** Record the guarantee where the algorithm is used — greedy set cover is H(n) ≈ ln n, FFD bin packing is 11/9·OPT + 6/9, greedy vertex cover (via matching) is 2·OPT. Then report results as "at most X bins" rather than "the minimum". For small instances, an exact solver can validate how close you actually are.
- **Trade-offs of the fix:** Exact solvers don't scale, so you're often stuck with the approximation regardless — the fix is honesty about what's guaranteed, not a better algorithm. Also, greedy set cover's ln n ratio is provably optimal unless P = NP, so "improve the algorithm" isn't available.

### Pitfall: Greedy on a problem with negative or non-monotone structure

- **What goes wrong:** [Dijkstra](../shortest-paths/learning.md) is run with a negative edge weight and returns wrong distances with no error. Or a greedy "always take the profitable step" is applied where a temporary loss enables a larger gain.
- **Why it happens (the mechanism):** Greedy's commitment is justified by an argument that going further can't help — Dijkstra's finalization proof requires that reaching a vertex via any other route means travelling *at least as far* first, which needs non-negativity. Any mechanism that makes a longer prefix lead to a better total breaks the argument, and the algorithm has already moved on.
- **How to handle it in production, and why that works:** Identify the monotonicity your greedy depends on and assert it — `debug_assert!(weights.iter().all(|&w| w >= 0))` at the entry to Dijkstra costs nothing in release and documents the precondition. Where it doesn't hold, use the algorithm that doesn't assume finality: Bellman-Ford for negative weights, DP for non-monotone gains.
- **Trade-offs of the fix:** Bellman-Ford is Θ(V·E) against Dijkstra's Θ(E log V) — measured in Stage 5, Dijkstra did 200k vertices in 56 ms, and the general algorithm is thousands of times slower. So establishing the precondition is much better than defensively using the general algorithm.
