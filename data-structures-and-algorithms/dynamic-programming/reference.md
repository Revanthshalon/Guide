# Dynamic Programming — Quick Reference

## At a Glance

**Recursion plus a cache.** Applies exactly when a recursion revisits states. The skill is **state design** — everything else is mechanical.

**Invariant:** `dp[s]` is correct for subproblem `s`, computed from strictly smaller ones. Three obligations: the state captures everything the future depends on · the dependency order is **acyclic** · transitions are exhaustive.

**A DP is a shortest path on a DAG of states.** Tabulation = DAG relaxation in topological order. Cyclic state graph ⇒ not a DP; use Bellman-Ford.

## The Numbers (measured)

| Fibonacci, n = 40 | Time |
| --- | --- |
| Naive recursion | **312.84 ms** |
| Memo (`HashMap`) | 4.46 µs |
| **Memo (array)** | **333 ns** (**~939,000×** vs naive; **13.4×** vs HashMap) |

| LCS, two 3,000-char strings | Time | Memory |
| --- | --- | --- |
| Full 2-D table | 17.43 ms | **17 MB** |
| **Two rolling rows** | **13.34 ms** | **11 KB** (~1,500× less, and *faster*) |

## Top-down vs Bottom-up

| | Memoization | Tabulation |
| --- | --- | --- |
| Computes | **only reachable states** | all states in range |
| Order | implicit | **you must get it right** |
| Space optimization | hard | **easy** (rolling arrays) |
| Stack risk | **yes** (~100k–200k frames) | no |
| Best for | deriving the recurrence; sparse states | production; dense states |

**Workflow:** write the recursion → add a memo → convert to tabulation only if you need space or hit the stack limit.

## State-Design Catalogue

| Shape | State | Examples |
| --- | --- | --- |
| Linear | `dp[i]` = prefix ending at i | Fibonacci, house robber |
| Two sequences | `dp[i][j]` | LCS, edit distance |
| Knapsack | `dp[i][w]` | subset sum, partition |
| Interval | `dp[i][j]`, split at k | matrix chain, optimal BST |
| Bitmask | `dp[mask][i]` | TSP, assignment — **n ≤ ~20** |
| Tree | `dp[node][state]` | tree independent set |
| Digit | `dp[pos][tight][state]` | "count numbers < N with P" |

## Knapsack — the loop direction IS the semantics

```rust
// 0-1 (each item once): capacity DOWNWARD
for item in items { for w in (item.w..=cap).rev() { dp[w] = dp[w].max(dp[w-item.w] + item.v); } }
// Unbounded (any number): capacity UPWARD
for item in items { for w in item.w..=cap { dp[w] = dp[w].max(dp[w-item.w] + item.v); } }
```

Test that distinguishes them: one item, weight 1, capacity 10 → 1 (0-1) vs 10 (unbounded).

## Complexity = states × transition cost

| Problem | Total |
| --- | --- |
| LCS / edit distance | Θ(n·m) |
| 0-1 knapsack | **Θ(n·W) — pseudo-polynomial** |
| Interval DP | Θ(n³) |
| Bitmask TSP | Θ(2ⁿ·n²) — n ≤ ~20 |

**Pseudo-polynomial:** `W` is the capacity *value*, not the input size. W = 10⁹ is intractable.

## Optimizations

| Technique | Turns | Into |
| --- | --- | --- |
| Rolling array | Θ(n·m) space | Θ(m) space |
| **Monotonic deque** | Θ(n·k) | **Θ(n)** |
| Prefix sums | Θ(n·k) | Θ(n) |
| Convex hull trick | Θ(n²) | Θ(n log n) |
| Matrix exponentiation | Θ(n) | Θ(log n) |

## Rules of Thumb

- Write the recursion first — that's where the thinking is.
- State = the nouns in your one-sentence description of the subproblem.
- Array memo for small integer states; `HashMap` only for sparse/non-integer.
- Flat `Vec` + manual indexing beats `Vec<Vec<_>>`.
- Rolling arrays lose **reconstruction** — keep the table or use Hirschberg's.
- Can you prove locally-best is globally-best? Use **greedy** instead.
- Verify against brute force on n ≤ 12 — that's what catches an under-specified state.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Under-specified state | Wrong answers; right on small inputs |
| Memo added with no overlap | 2–3× slower for nothing |
| No memo where overlap exists | 939,000× (Fibonacci n=40) |
| Wrong knapsack loop direction | Plausible answer to a *different* problem |
| Θ(n·W) with W = 10⁹ | Unallocatable table |
| Rolled the array, then needed the path | Reconstruction information gone |
| Top-down over 10⁶ states | Stack overflow abort |

## Key References

- CLRS ch. 15 — optimal substructure done carefully
- Hirschberg (1975) — reconstruction in Θ(n) space
- [CP-Algorithms](https://cp-algorithms.com/) — CHT, D&C opt, Knuth opt
