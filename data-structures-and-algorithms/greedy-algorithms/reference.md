# Greedy Algorithms — Quick Reference

## At a Glance

Make the locally-best choice, never reconsider. **Simplest to write, hardest to trust** — a wrong greedy looks identical to a right one and produces plausible answers.

**Invariant:** after every step, the partial solution is **extendable to an optimal solution**.
**Needs both:** greedy-choice property (*this is the one that fails*) **and** optimal substructure. DP needs only the second.

> Never ship a greedy you haven't **proved** or **verified against brute force**.

## Two Proofs

| Technique | Shape |
| --- | --- |
| **Exchange argument** | Transform any optimum into greedy's answer without worsening it |
| **Greedy stays ahead** | Greedy's i-th choice is never worse than the optimum's i-th |
| **Matroid** | If feasible sets form a matroid, greedy is *guaranteed* optimal (Kruskal) |

## Works vs Fails — the paired near-misses

| Greedy works | Greedy fails |
| --- | --- |
| **Fractional** knapsack | **0-1** knapsack |
| Coins {1,5,10,25} | Coins {1,10,25} (30 → 6 coins vs 3) |
| Interval scheduling | **Weighted** interval scheduling |
| MST (total weight) | Shortest-path tree |
| Dijkstra, non-negative | Dijkstra with **negative** weights |
| Huffman | Optimal BST |

Every right-hand entry has a DP solution. **When in doubt, DP** — slower, never silently wrong.

## The Classics

| Problem | Rule | Why |
| --- | --- | --- |
| **Interval scheduling** | **Earliest finish** | Stays ahead — leaves the most room |
| Interval partitioning | Sort by start, reuse free room | Achieves the max-overlap lower bound |
| Fractional knapsack | Best value/weight | Exchange |
| Huffman | Merge two lowest frequencies | Exchange |
| Kruskal / Prim | Cheapest safe edge | Cut property (matroid) |
| Dijkstra | Closest unfinished | Non-negative weights |

**Interval scheduling's wrong rules** (all tempting): earliest start · shortest duration · fewest conflicts.

## The Safety Net

```rust
#[test]
fn greedy_matches_brute_force() {
    for _ in 0..10_000 {
        let inst = random_instance(/* n ≤ 10 */);
        assert_eq!(greedy(&inst), brute_force_optimal(&inst), "counterexample: {inst:?}");
    }
}
```

Finds the coin-change bug in seconds. Cheaper than a proof.

## Complexity

Greedy's cost is almost always **the sort** — the loop itself is Θ(n).

| Problem | Greedy | DP |
| --- | --- | --- |
| Interval scheduling | Θ(n log n) | Θ(n log n) weighted |
| Huffman | Θ(n log n) | — |
| Kruskal MST | Θ(E log E) | — |
| 0-1 knapsack | ✗ | Θ(n·W) |
| Coin change (arbitrary) | ✗ | Θ(n·amount) |

Input pre-sorted ⇒ greedy becomes Θ(n).

## Approximation Guarantees (NP-hard problems)

| Problem | Greedy gives |
| --- | --- |
| Set cover | H(n) ≈ ln n — **provably optimal unless P=NP** |
| Bin packing (FFD) | 11/9·OPT + 6/9 |
| Vertex cover (via matching) | 2·OPT |
| Metric TSP (via MST) | 2·OPT |

Report "at most X", never "the minimum".

## Rules of Thumb

- Proof or brute-force test before shipping. No exceptions.
- Greedy produces a *valid* answer always — suboptimality is silent.
- Interval scheduling: **earliest finish**, and know why the other three rules fail.
- Assert the precondition your proof needs (`debug_assert!` non-negative weights).
- Greedy is naturally **online** — it's the paradigm when you can't see the whole input.
- NP-hard + proven ratio is a legitimate answer; record the ratio at the call site.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Unproved greedy | Optimal on test data, suboptimal in prod, no error |
| Wrong greedy rule | Passes random tests; fails on straddling intervals |
| Approximation treated as optimal | "The minimum" reported when it's 11/9·OPT |
| Negative weights + Dijkstra | Confidently wrong distances |

## Key References

- CLRS ch. 16 — greedy, matroids, Huffman, with the exchange arguments
- Kleinberg & Tardos ch. 4 — the best treatment of greedy proof techniques
- Vazirani, *Approximation Algorithms* — where greedy is the right answer
