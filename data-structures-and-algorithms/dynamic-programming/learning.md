# Dynamic Programming — Learning Notes

## Mental Model

**Dynamic programming is recursion plus a cache — and that's not a simplification, it's the whole thing.**

The mystique around DP comes from teaching it as tabulation ("fill this 2-D table left to right") without saying where the table came from. Start instead from the recursion, which is always the honest expression of the problem, and add a memo. Measured on the canonical case:

| Fibonacci, n = 40 | Time |
| --- | --- |
| Naive recursion | **312.84 ms** |
| Memo (`HashMap`) | 4.46 µs |
| Memo (array) | **333 ns** |

**~939,000×** from a two-line change — and the gap doubles with every increment of n, because the recursion tree has Θ(2ⁿ) nodes but only Θ(n) *distinct* states.

That ratio is the entire subject. **DP applies exactly when a recursion revisits states**, and the two conditions people memorize are just names for that:

- **Optimal substructure** — the optimal solution is built from optimal solutions to subproblems. This is what makes the recursion *correct*.
- **Overlapping subproblems** — the same subproblem recurs. This is what makes the cache *pay*.

Miss the first and DP gives wrong answers (greedy problems have optimal substructure but so do many things that aren't decomposable the way you assumed). Miss the second and the memo is pure overhead.

The skill that actually transfers is **state design**: deciding what tuple of values uniquely identifies a subproblem. Everything else — memo vs table, dimension order, space optimization — is mechanical once the state is right. If you're stuck on a DP problem, you are almost always stuck on the state, not on the code.

## The Invariant

> **`dp[s]` is the correct answer for subproblem `s`, computed from the correct answers of strictly smaller subproblems.** The "smaller" relation must be a partial order with no cycles, and every subproblem must be resolved before anything that depends on it.

Three obligations, and violating any one is a distinct bug class:

- **The state must capture everything the future depends on.** If two different histories reach the "same" state but lead to different optimal futures, the state is under-specified and the memo returns wrong answers. This is the most common DP bug and the hardest to see, because the code looks right.
- **The dependency order must be acyclic.** Bottom-up tabulation requires you to iterate in an order where dependencies are already computed; top-down memoization requires the recursion to terminate. A cyclic dependency means it isn't a DP — it's a shortest-path problem on a graph with cycles, which needs [Dijkstra or Bellman-Ford](../shortest-paths/learning.md).
- **Transitions must be exhaustive.** Every way of reaching state `s` must be considered, or you compute the optimum over a subset of the real choice space.

There's a clean way to see all three at once: **a DP is a shortest/longest path on a DAG whose vertices are states and whose edges are transitions.** Tabulation is relaxing edges in topological order — which is literally the [DAG shortest-path algorithm](../shortest-paths/learning.md) from Stage 5. If your state graph has cycles, you need a different tool.

## Mechanics

### Top-down vs bottom-up

| | Memoization (top-down) | Tabulation (bottom-up) |
| --- | --- | --- |
| Shape | Recursion + cache | Loops filling an array |
| Computes | **Only reachable states** | All states in range |
| Order | Implicit (call order) | **You must get it right** |
| Overhead | Call + cache lookup per state | Array write per state |
| Space optimization | Hard | **Easy** (rolling arrays) |
| Stack risk | **Yes** — depth = recursion depth | No |
| Best for | Sparse state spaces, deriving the recurrence | Dense state spaces, production |

The practical workflow: **write the recursion first, add a memo, then convert to tabulation only if you need the space optimization or hit the stack limit.** The recursion is where the thinking is; the conversion is mechanical.

Memoization's "only reachable states" advantage is real and under-appreciated: if the state space is 10⁹ but only 10⁵ states are reachable from your input, tabulation does 10,000× the work.

### Memo representation matters

Measured: **an array memo beat a `HashMap` memo by 13.4×** (333 ns vs 4.46 µs). If state components are small non-negative integers, index directly — a `Vec<Option<T>>` or a flat `Vec<T>` with a sentinel. Reserve `HashMap` for sparse or non-integer states (strings, sets, tuples with huge ranges).

### Space optimization — rolling arrays

Most 2-D DPs only look back one row, so you only need two rows. Measured on LCS of two 3,000-character strings:

| | Time | Memory |
| --- | --- | --- |
| Full 2-D table | 17.43 ms | **17 MB** |
| Two rolling rows | **13.34 ms** | **11 KB** |

**~1,500× less memory, and slightly faster** — because two rows fit in cache while a 17 MB table doesn't. This is the most reliable optimization in the whole topic, and it's nearly free.

The catch: **you lose the ability to reconstruct the solution**, since the path through the table is gone. If you need the actual LCS (not just its length), either keep the full table or use Hirschberg's divide-and-conquer trick to reconstruct in Θ(n) space at 2× the time.

### The state-design catalogue

Most DP problems are one of these shapes. Recognizing the shape gives you the state:

| Shape | State | Transition | Examples |
| --- | --- | --- | --- |
| **Linear / 1-D** | `dp[i]` = answer for prefix ending at i | from `dp[i-1]`, `dp[i-2]`, … | Fibonacci, house robber, climbing stairs |
| **Two sequences** | `dp[i][j]` = answer for prefixes i, j | match/skip either | LCS, edit distance, sequence alignment |
| **Knapsack** | `dp[i][w]` = best using first i items, capacity w | take / don't take | 0-1 knapsack, subset sum, partition |
| **Interval** | `dp[i][j]` = answer for subarray i..j | split at every k in between | matrix chain, burst balloons, optimal BST |
| **Bitmask** | `dp[mask]` or `dp[mask][i]` = over a subset | add one element to the set | TSP, assignment, set cover — n ≤ ~20 |
| **Tree** | `dp[node][state]` | combine children | tree independent set, tree diameter |
| **Digit** | `dp[pos][tight][state]` | choose the next digit | "count numbers < N with property P" |
| **DP on a DAG** | `dp[v]` = answer at vertex v | over incoming edges | longest path, critical path |

**Knapsack is the one to internalize** because the 0-1 vs unbounded distinction is a pure loop-direction question:

```rust
// 0-1 knapsack (each item once): iterate capacity DOWNWARD.
for item in items {
    for w in (item.weight..=capacity).rev() {
        dp[w] = dp[w].max(dp[w - item.weight] + item.value);
    }
}
// Unbounded (each item any number of times): iterate capacity UPWARD.
for item in items {
    for w in item.weight..=capacity {
        dp[w] = dp[w].max(dp[w - item.weight] + item.value);
    }
}
```

Downward means `dp[w - weight]` still refers to the *previous* item round (item used at most once). Upward means it may already include this item (unlimited copies). **One loop direction is the entire difference**, which is worth knowing because it's easy to write the wrong one and get a plausible-looking answer.

### Recognizing DP

The trigger phrases, and what they usually mean:

- "**minimum/maximum** number of ways to…" → optimization DP
- "**count the number of ways**" → counting DP (same recurrence, `+` instead of `min`/`max`)
- "**is it possible to**…" → boolean DP (same recurrence, `||`)
- "**longest/shortest** subsequence/substring/path" → sequence DP
- Choices at each step with **future consequences** → DP
- Choices where the **locally best is provably globally best** → [greedy](../greedy-algorithms/learning.md), not DP

That last line is the important distinction: greedy is faster when it applies, and applying it when it doesn't is a silent wrong answer. If you can't prove the exchange argument, use DP.

### Optimizations beyond the basic recurrence

| Technique | Turns | Into | When |
| --- | --- | --- | --- |
| Rolling array | Θ(n·m) space | Θ(m) space | Only the last row is needed |
| **Monotonic deque** | Θ(n·k) | **Θ(n)** | Transition is a min/max over a sliding window |
| Prefix sums | Θ(n·k) | Θ(n) | Transition is a sum over a range |
| Convex hull trick | Θ(n²) | Θ(n log n) | Transition is `min over j of (a[j]·x + b[j])` |
| Divide & conquer opt | Θ(n²·k) | Θ(n·k log n) | The optimal split point is monotone |
| Knuth optimization | Θ(n³) | Θ(n²) | Quadrangle inequality holds |
| Matrix exponentiation | Θ(n) | Θ(log n) | Linear recurrence, huge n |

The [monotonic deque](../monotonic-stack-and-queue/learning.md) one is the most broadly useful — "the transition takes a max over the last k states" is an extremely common shape, and it drops a factor of k.

## Complexity

**DP cost = (number of states) × (cost per transition).** That's the whole formula, and it's how you decide whether a formulation is feasible before writing it.

| Problem | States | Per transition | Total |
| --- | --- | --- | --- |
| Fibonacci | Θ(n) | Θ(1) | **Θ(n)** |
| LCS / edit distance | Θ(n·m) | Θ(1) | **Θ(n·m)** |
| 0-1 knapsack | Θ(n·W) | Θ(1) | **Θ(n·W)** — pseudo-polynomial |
| Interval DP | Θ(n²) | Θ(n) split | **Θ(n³)** |
| Bitmask (TSP) | Θ(2ⁿ·n) | Θ(n) | **Θ(2ⁿ·n²)** — n ≤ ~20 |
| Tree DP | Θ(n·k) | Θ(k) | Θ(n·k²) or Θ(n·k) |
| Digit DP | Θ(digits · states) | Θ(10) | Θ(d · s) |

**Where the table misleads.** Knapsack's Θ(n·W) is **pseudo-polynomial** — `W` is the capacity *value*, not the input size, so doubling the capacity number doubles the runtime while the input grows by one bit. A knapsack with capacity 10⁹ is intractable despite the polynomial-looking bound. This is the same trap as min-cost flow's `F` in [advanced graph algorithms](../advanced-graph-algorithms/learning.md), and it's why subset-sum is NP-hard despite having a "polynomial" DP.

The bitmask row is the other one to watch: 2ⁿ states means n ≤ 20 or so, and that ceiling arrives fast.

## Rust Implementation

```rust
// Top-down: write this first — the recursion IS the thinking.
fn lcs(a: &[u8], b: &[u8], i: usize, j: usize, memo: &mut Vec<Vec<i32>>) -> i32 {
    if i == 0 || j == 0 { return 0; }
    if memo[i][j] >= 0 { return memo[i][j]; }
    let r = if a[i-1] == b[j-1] { lcs(a, b, i-1, j-1, memo) + 1 }
            else { lcs(a, b, i-1, j, memo).max(lcs(a, b, i, j-1, memo)) };
    memo[i][j] = r;
    r
}

// Bottom-up with a rolling array: 1,500× less memory, measured.
let (mut prev, mut cur) = (vec![0u16; m + 1], vec![0u16; m + 1]);
for i in 1..=n {
    for j in 1..=m {
        cur[j] = if a[i-1] == b[j-1] { prev[j-1] + 1 } else { prev[j].max(cur[j-1]) };
    }
    std::mem::swap(&mut prev, &mut cur);
}
let answer = prev[m];

// Flat array beats Vec<Vec> — one allocation, contiguous, no pointer chase.
let mut dp = vec![0u32; (n + 1) * (m + 1)];
let at = |i: usize, j: usize| i * (m + 1) + j;
```

Three Rust-specific notes:

**A flat `Vec` with manual indexing beats `Vec<Vec<_>>`** for the same reason CSR beat nested vectors in [graph representations](../graph-representations/learning.md): one allocation, contiguous rows, no per-row pointer chase.

**Top-down DP inherits recursion's stack limit** — measured at ~100k–200k frames in [recursion & backtracking](../recursion-and-backtracking/learning.md). A memoized recursion over a 10⁶-element sequence will abort. Convert to tabulation, or add a depth guard.

**The borrow checker fights `&mut memo` in recursive calls.** Passing `&mut Vec<Vec<i32>>` down works; trying to hold a reference *into* the memo across a recursive call does not. Index, don't borrow — the same discipline as arena-based structures.

**Crates:** none needed — DP is loops and arrays. `ndarray` if you want multi-dimensional indexing sugar; `memoize` for a proc-macro memo on pure functions.

## Use Cases

- **Sequence comparison** — `diff`, `git`, spell-checkers, DNA alignment. All edit distance or LCS variants.
- **Resource allocation** — knapsack shapes: budget allocation, ad selection, cargo loading.
- **Text processing** — word wrap (minimize raggedness), word break, regex matching with `.` and `*`.
- **Finance** — optimal trading with constraints, option pricing lattices.
- **Bioinformatics** — Smith-Waterman and Needleman-Wunsch are edit distance with domain-specific scoring.
- **Compilers** — optimal instruction selection, register allocation on trees, Knuth-Plass line breaking (used by TeX).
- **Probability** — HMM forward/backward, Viterbi decoding. Viterbi is DP on a DAG of states over time.
- **Counting** — combinatorial counting where inclusion-exclusion is unwieldy; digit DP for "how many numbers below N have property P".

## When to Use Which

| Reach for | When |
| --- | --- |
| **Memoization** | Deriving the recurrence; sparse/reachable-only state space |
| **Tabulation** | Dense state space; you need space optimization; deep recursion |
| **Rolling array** | The recurrence only looks back a fixed number of rows |
| Full table | You must **reconstruct** the solution, not just its value |
| Hirschberg's | Need reconstruction *and* Θ(n) space — 2× time |
| Bitmask DP | Subsets matter and n ≤ ~20 |
| [Greedy](../greedy-algorithms/learning.md) | You can prove locally-best is globally-best — much faster |
| [DAG relaxation](../shortest-paths/learning.md) | The state graph is explicit and acyclic |
| Bellman-Ford / Dijkstra | The state graph has **cycles** — DP doesn't apply |
| Branch and bound / solver | State space too large even for DP (Stage 10) |

## Pitfalls in Depth

### Pitfall: An under-specified state

- **What goes wrong:** The memo key omits something the future depends on. Two different histories map to the same key, the first result is cached, and the second gets a wrong answer. The classic instances: a knapsack keyed on item index but not remaining capacity; a path DP keyed on position but not on "have I already used the one allowed skip"; a game DP keyed on the board but not on whose turn it is. The code looks correct and the answers are subtly wrong — often right on small inputs where the collision never occurs.
- **Why it happens (the mechanism):** The recursion's *parameters* and the subproblem's *identity* are not automatically the same thing. Any value that (a) varies across calls and (b) affects the result must be in the key. Mutable state captured by reference is invisible in the key by construction, which is why memoizing a method that reads `self` is so dangerous.
- **How to handle it in production, and why that works:** State the subproblem in one English sentence — "the best value using items i.. with capacity w remaining" — and make the key exactly the nouns in that sentence. Then verify against brute force on small inputs: an exhaustive search over n ≤ 12 compared to the DP catches an under-specified state immediately, because collisions become likely at small sizes.
- **Trade-offs of the fix:** A more complete state means more states, and the memo grows multiplicatively with each added dimension — adding a boolean doubles it, adding a position multiplies by n. So there's real pressure to find the *minimal* sufficient state, and that tension is where DP problems get hard. Sometimes a seemingly-needed dimension can be eliminated by reformulating (e.g. processing items in sorted order removes the need to track which were used).

### Pitfall: Missing the overlapping-subproblems check

- **What goes wrong:** A memo is added to a recursion whose subproblems are all distinct. The cache never hits, and you've paid allocation, hashing, and lookup for nothing — often a 2–3× slowdown. Conversely and worse: a recursion with heavy overlap is left un-memoized. Measured on Fibonacci at n = 40: **312.84 ms without, 333 ns with — ~939,000×**.
- **Why it happens (the mechanism):** Overlap isn't visible in the code. The recursion correctly expresses "this problem in terms of smaller ones" whether or not those smaller ones repeat. Fibonacci's tree has 2ⁿ nodes over n distinct states; a permutation-generating recursion has n! nodes over n! distinct states. Same shape, opposite conclusions.
- **How to handle it in production, and why that works:** Count distinct states against total recursive calls — instrument the recursion with a counter and a `HashSet` of keys on a small input. If calls ≫ distinct states, memoize; if they're comparable, don't. This takes two minutes and replaces guessing.
- **Trade-offs of the fix:** The instrumentation is throwaway code. The deeper cost is that adding memoization constrains the recursion to be *pure* — no reliance on mutable outer state — which sometimes forces a refactor that's worth doing anyway.

### Pitfall: The wrong loop direction in knapsack

- **What goes wrong:** 0-1 knapsack is written with the capacity loop ascending, so each item can be used unlimited times — the answer is too high, and it's a *valid* answer to a different problem, so it looks plausible. Or unbounded knapsack is written descending and each item is used at most once, giving an answer that's too low.
- **Why it happens (the mechanism):** In the 1-D space-optimized form, `dp[w - weight]` refers either to the previous item's row (if you haven't overwritten it yet — descending) or to the current item's row (if you have — ascending). The direction *is* the semantics, and nothing in the code says so. In the 2-D form the distinction is explicit (`dp[i-1][...]` vs `dp[i][...]`) and the bug can't happen — it's introduced by the space optimization.
- **How to handle it in production, and why that works:** Write the 2-D version first, where the row index makes the dependency explicit and correct by construction. Then collapse to 1-D and choose the direction that preserves which row you're reading. Leave a comment stating which it is. Test with an instance where the answer differs — a single item with weight 1 and a capacity of 10 gives 1 (0-1) or 10 (unbounded), which distinguishes them instantly.
- **Trade-offs of the fix:** Writing the 2-D version first is an extra step you'll usually discard. It costs a few minutes and prevents a bug that produces confident wrong answers.

### Pitfall: Pseudo-polynomial blowup

- **What goes wrong:** A knapsack or subset-sum DP is Θ(n·W) and looks polynomial, so it's used with capacities in the millions or with floating-point weights scaled to integers. With n = 100 items and W = 10⁹ that's 10¹¹ table entries — not merely slow, unallocatable.
- **Why it happens (the mechanism):** `W` is the *numeric value* of the capacity, not the size of the input. Encoding W takes log W bits, so runtime is exponential in the input length — the definition of pseudo-polynomial. This is exactly why subset-sum is NP-complete despite having a "polynomial" DP, and it catches people because Θ(n·W) reads like Θ(n·m).
- **How to handle it in production, and why that works:** Check whether the numeric range is bounded by something small before choosing the formulation. If capacities are large but *values* are small, flip the DP to be indexed by value (`dp[v]` = minimum weight achieving value v), which is Θ(n·V) — often the tractable direction. If both are large, DP is the wrong tool: use branch and bound, an FPTAS (round values to get a (1−ε) approximation in polynomial time), or an ILP solver.
- **Trade-offs of the fix:** Flipping the DP dimension only helps when one range is small. An FPTAS gives up exactness for a tunable ε. A solver adds a dependency and unpredictable runtime. All beat a table you can't allocate.

### Pitfall: Space-optimizing away the reconstruction

- **What goes wrong:** The rolling-array optimization is applied — measured, 17 MB down to 11 KB — and then someone needs the actual alignment, the chosen items, or the path, not just the optimal value. The information required to reconstruct it was in the discarded rows.
- **Why it happens (the mechanism):** The rolling array keeps only enough to compute the *next* row's values. Reconstruction requires walking backwards through the decisions, which needs the full decision history. The optimization and the requirement are in tension by construction, and the requirement usually arrives later.
- **How to handle it in production, and why that works:** Decide up front whether you need the value or the witness. Value only → roll the array. Witness needed → keep the full table (Θ(n·m) memory, usually acceptable), or store a compact **decision** table (one or two bits per cell recording which transition won) rather than the full values, which is often 8–16× smaller than the value table. For genuinely large inputs, **Hirschberg's algorithm** reconstructs in Θ(min(n,m)) space at 2× the time by divide-and-conquer on the midpoint.
- **Trade-offs of the fix:** A decision table still costs Θ(n·m) memory, just less of it. Hirschberg's is genuinely more complex to implement and doubles the runtime. The full table is simplest and is fine until n·m gets large — measured, a 3,000×3,000 `u16` table was 17 MB, which is nothing; at 30,000×30,000 it would be 1.7 GB, which isn't.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| **Persist it** | What if you kept every intermediate table? | Solution reconstruction; the decision table as a separate artifact |
| **Batch it** | What if you computed all states at once? | Tabulation itself; and SIMD/vectorized DP over a row |
| **Approximate it** | What if you rounded the values? | **FPTAS for knapsack** — (1−ε)-optimal in polynomial time, escaping pseudo-polynomial |
| Randomize it | What if you sampled the state space? | Monte Carlo tree search — DP's answer when the space is too big |
| Externalize it | What if the table exceeded RAM? | Block-wise tabulation with I/O-aware ordering; Hirschberg's for linear space |
| **Parallelize it** | Where's the independence? | Anti-diagonals of a 2-D DP are independent; wavefront parallelism |
| **Invert it** | What if you indexed by *value* instead of *weight*? | Flips Θ(n·W) to Θ(n·V) — the fix when capacity is huge but values are small |
| **Augment it** | What does one more state dimension buy? | Constraints: "at most k skips", "must end in state s", "cooldown of 1 day" |
| **Specialize it** | What if the transition were a sliding-window min? | **Monotonic deque** drops a factor of k — [monotonic stack & queue](../monotonic-stack-and-queue/learning.md) |
| Amortize it | What if the recurrence were linear? | **Matrix exponentiation** — Θ(log n) instead of Θ(n) for huge n |

**Questions:**

1. DP is a shortest path on a DAG of states. Given that, explain precisely why a cyclic state graph means DP doesn't apply, and name the algorithm that does.
2. Under "invert it", knapsack can be indexed by value instead of weight. Write both recurrences and state the condition that decides which is tractable.
3. The 0-1 vs unbounded knapsack difference is one loop direction. Explain what `dp[w - weight]` refers to in each case, and why the 2-D form can't have this bug.
4. Under "approximate it", the knapsack FPTAS rounds values. Sketch why rounding to `⌊v · k/v_max⌋` gives a (1−ε) guarantee, and what it costs.
5. Measured, an array memo beat a `HashMap` memo by 13.4×. Under what circumstances would the `HashMap` be the *right* choice despite that?
6. Under "parallelize it", anti-diagonals of a 2-D DP are independent. Prove it from the dependency pattern of LCS, and say what that costs in cache behaviour versus row-major order.
7. Rolling arrays gave 1,500× less memory *and* were slightly faster. Explain the second part — why would using less memory make it faster?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the two conditions for DP and say which bug each one prevents.
2. Give the measured Fibonacci numbers for naive, `HashMap` memo, and array memo, and explain both gaps.
3. State the DP invariant and its three obligations.
4. Give the LCS rolling-array numbers and say what you give up.
5. Write both knapsack loops and state which is which, and why.
6. Why is Θ(n·W) knapsack not actually polynomial? Name another algorithm with the same trap.

Build exercises:

- Take naive Fibonacci → `HashMap` memo → array memo → tabulation → matrix exponentiation, measuring each at n = 40 (and n = 10⁹ for the last two, mod something). Reproducing the ~939,000× and the 13.4× makes the whole topic concrete in ten minutes.
- Implement LCS with a full table and with rolling rows; reproduce the 17 MB → 11 KB and confirm the rolling version is not slower. Then implement reconstruction, discover you can't do it with rolling rows, and implement Hirschberg's.
- Implement 0-1 and unbounded knapsack from the same 2-D recurrence, collapse both to 1-D, and write the test that distinguishes them (one item, weight 1, capacity 10). Then find the largest W your machine can allocate and confirm the pseudo-polynomial wall.
- Implement TSP with bitmask DP and find where it stops being feasible on this machine — the 2ⁿ ceiling arrives fast, and knowing your own n is worth more than knowing it's "about 20".

## Open Questions

- Where exactly does bitmask TSP become infeasible here — n = 20, 22, 24? Memory or time first?
- Does a flat `Vec` with manual indexing measurably beat `Vec<Vec<_>>` for a 3,000×3,000 LCS, and by how much?
- Anti-diagonal (wavefront) parallel LCS with rayon: what speedup is achievable given the worse cache behaviour?
- Does the monotonic-deque optimization of a sliding-window-max DP transition reproduce the 591× measured for the standalone problem, or does DP overhead dilute it?
- For a memoized recursion over 10⁶ states, at what depth does it actually abort, and does `stacker` make top-down viable at that scale?

## References

- CLRS ch. 15 — rod cutting, matrix chain, LCS, optimal BST, with the optimal-substructure discussion done carefully.
- Bellman, *Dynamic Programming* (1957) — the origin, and the explanation of the deliberately vague name.
- Hirschberg, "A linear space algorithm for computing maximal common subsequences" (1975) — reconstruction in Θ(n) space.
- Knuth & Plass, "Breaking Paragraphs into Lines" (1981) — DP in a real system (TeX), and a good example of a non-obvious state.
- [CP-Algorithms: DP optimizations](https://cp-algorithms.com/) — convex hull trick, divide-and-conquer optimization, Knuth optimization.
- Related in this repo: [Recursion & Backtracking](../recursion-and-backtracking/learning.md) (DP is backtracking plus a memo), [Greedy Algorithms](../greedy-algorithms/learning.md) (when locally-best suffices — much faster), [Shortest Paths](../shortest-paths/learning.md) (DP *is* DAG relaxation; cyclic state graphs need Bellman-Ford), [Monotonic Stack & Queue](../monotonic-stack-and-queue/learning.md) (the sliding-window transition optimization), [Prefix Sums](../prefix-sums-and-difference-arrays/learning.md) (range-sum transitions).
