# Recursion & Backtracking — Learning Notes

## Mental Model

**Recursion is an inductive proof you can run.** You state what the function does for the smallest case, then assume it works for smaller inputs and use that to handle the current one. If both halves hold, the function is correct — and the "assume it works" step is not hand-waving, it's the induction hypothesis. That reframing is the practical skill: **stop tracing the call stack and start checking the two obligations** (base case correct, recursive step correct *given* the assumption).

**Backtracking is DFS over a decision tree that you never build.** At each node you choose a value for the next decision variable, recurse, then undo the choice. The tree is implicit — the state space of a puzzle, the permutations of a set, the assignments of a constraint problem — which is exactly the [implicit graph](../graph-representations/learning.md) idea from Stage 5 applied to search.

What makes backtracking work at all is **pruning**: abandoning a partial solution the moment it cannot be completed. This is not an optimization on top of the search; it is the entire algorithm. Measured on N-queens, comparing prune-as-you-go against generate-every-assignment-then-test:

| n | Solutions | Pruned | Generate-then-test | Ratio | Full space (nⁿ) |
| --- | --- | --- | --- | --- | --- |
| 6 | 4 | 4.08 µs | 1.24 ms | 305× | 46,656 |
| 7 | 40 | 10.00 µs | 13.74 ms | 1,374× | 823,543 |
| 8 | 92 | 23.92 µs | 119.92 ms | 5,014× | 16,777,216 |
| **9** | 352 | **50.38 µs** | **2.20 s** | **43,580×** | 387,420,489 |

The ratio grows with n because pruning cuts entire *subtrees* — rejecting a queen placement at row 2 eliminates n^(n−2) leaves at once. **Both versions are exponential; pruning changes the base of the exponent**, and that is the difference between usable and not.

The third thing to hold, and it's Rust-specific: the call stack is a **fixed-size resource**. Measured in [graph traversal](../graph-traversal/learning.md), plain recursion aborts around 100,000–200,000 frames on the main thread's 8 MB stack, and roughly four times sooner on a spawned thread's 2 MB. When depth comes from untrusted input, recursion is a denial-of-service vector with an uncatchable failure.

## The Invariant

For any recursive function, two obligations — and nothing else:

> **Base case:** the function is correct for inputs it does not recurse on.
> **Inductive step:** *assuming* the function is correct for every strictly smaller input, it is correct for this one.
> **Progress:** every recursive call is on a strictly smaller input under some well-founded measure, so the recursion terminates.

The "well-founded measure" is where infinite recursion comes from. `f(n-1)` on a `usize` looks decreasing and isn't when `n == 0`. A tree recursion that follows a cycle never shrinks. State the measure explicitly — "the length of the remaining slice", "the number of unassigned variables", "the depth remaining" — and check that every call decreases it.

For **backtracking**, add the state invariant:

> At every node of the search, the partial assignment is **consistent** (violates no constraint), and the mutable state describing it is **exactly restored** on the way back up.

That second clause is the source of most backtracking bugs. You mutate shared state on the way down (mark a column used, push onto a path) and must undo precisely that on the way up. Any asymmetry corrupts sibling branches, and the corruption appears far from its cause.

## Mechanics

### The backtracking skeleton

```rust
fn solve(state: &mut State, depth: usize, out: &mut Vec<Solution>) {
    if depth == state.n {                       // 1. complete?
        out.push(state.snapshot());
        return;
    }
    for choice in state.candidates(depth) {     // 2. iterate choices
        if !state.is_feasible(depth, choice) { continue; }   // 3. PRUNE — the whole algorithm
        state.apply(depth, choice);             // 4. make
        solve(state, depth + 1, out);           // 5. recurse
        state.undo(depth, choice);              // 6. UNMAKE — must exactly mirror `apply`
    }
}
```

Steps 3 and 6 are the ones that matter. Step 3 is what turns an intractable enumeration into a feasible search; step 6 is what keeps siblings independent.

### Pruning techniques, in increasing power

| Technique | What it does | Example |
| --- | --- | --- |
| **Feasibility check** | Reject a choice violating a constraint now | N-queens: column/diagonal already attacked |
| **Constraint propagation** | Deduce forced values after each choice | Sudoku: a cell with one candidate left |
| **Bound pruning** | Reject if the best possible completion is worse than the best found | Branch and bound for TSP, knapsack |
| **Symmetry breaking** | Explore one representative per equivalence class | N-queens: only half the first row |
| **Ordering heuristics** | Choose the most-constrained variable first | Sudoku: fill the cell with fewest candidates (MRV) |
| **Memoization** | Cache results of identical subproblems | Turns backtracking into [DP](../dynamic-programming/learning.md) |

The last row is the bridge to the next topic: **when a backtracking search revisits identical subproblems, adding a cache converts it into dynamic programming** — the same code, an exponential-to-polynomial change.

### Bitmask state — why N-queens is so fast

The measured pruned version tracks attacked columns and diagonals as three `u32` bitmasks:

```rust
fn queens(n: usize, row: usize, cols: u32, d1: u32, d2: u32) -> u64 {
    if row == n { return 1; }
    let full = (1u32 << n) - 1;
    let mut avail = full & !(cols | d1 | d2);   // all safe columns, at once
    let mut count = 0;
    while avail != 0 {
        let bit = avail & avail.wrapping_neg(); // lowest set bit — isolate one choice
        avail -= bit;
        count += queens(n, row + 1, cols | bit, (d1 | bit) << 1 & full, (d2 | bit) >> 1);
    }
    count
}
```

The diagonals shift by one per row, so `d1 << 1` and `d2 >> 1` propagate the attack pattern automatically — no per-cell checks. Feasibility for the entire row is one AND, and iterating candidates is the `x & -x` lowest-set-bit trick from [bit manipulation](../bit-manipulation/learning.md). This is why the pruned version ran 9-queens in 50 µs.

### Recursion → iteration

Any recursion can be made iterative, and how hard it is depends on its shape:

- **Tail recursion** (the recursive call is the last thing): becomes a `while` loop directly, no stack needed. Rust does **not** guarantee tail-call optimization, so write the loop yourself when depth could be large.
- **Single non-tail recursion** (e.g. tree traversal): an explicit `Vec` as the stack. Straightforward if you only need pre-order; needs an enter/exit state machine if you need post-order (finish times).
- **Multiple recursion / backtracking**: an explicit stack of `(node, choice_iterator)` frames. Genuinely more code, and where the readability loss is real.

The decision rule: **recurse when depth is provably bounded** (a balanced tree is Θ(log n); a DAG of known depth; a fixed-size board). **Iterate when depth scales with input size** — lists, paths, chains, or anything an adversary controls.

## Complexity

| Pattern | Time | Space | Note |
| --- | --- | --- | --- |
| Linear recursion | Θ(n) | **Θ(n) stack** | Convert to a loop |
| Binary recursion, no memo | Θ(2ⁿ) | Θ(n) | Fibonacci — see [DP](../dynamic-programming/learning.md) |
| Divide & conquer | Θ(n log n) typical | Θ(log n) | Balanced split — [D&C](../divide-and-conquer/learning.md) |
| Permutations | Θ(n!) | Θ(n) | n ≤ ~11 |
| Subsets | Θ(2ⁿ) | Θ(n) | n ≤ ~25 |
| Backtracking, pruned | Θ(b^d) with a **smaller b** | Θ(d) | Pruning changes the base |
| Branch and bound | Θ(b^d) worst, far less typical | Θ(d) | Bound quality decides everything |

**Where the table misleads.** Backtracking's bound is honest but useless for prediction: the measured N-queens ratio grew from 305× to 43,580× as n went 6 → 9, entirely because pruning removed subtrees. Two algorithms both "Θ(exponential)" can differ by orders of magnitude, and the difference is the *pruning*, which no complexity notation captures. **For backtracking, measure; don't estimate from the bound.**

The space column is the one that bites in Rust: Θ(n) *stack* space is not the same as Θ(n) heap space — it's capped at 8 MB (or 2 MB on a spawned thread) and overflowing it aborts the process.

## Rust Implementation

```rust
// Depth guard: turns an uncatchable abort into a clean error.
const MAX_DEPTH: usize = 10_000;
fn parse(&mut self, depth: usize) -> Result<Ast, Error> {
    if depth > MAX_DEPTH { return Err(Error::TooDeep); }
    // ...
}

// Backtracking with exact undo — the `&mut` + restore pattern.
fn permutations(v: &mut Vec<u32>, k: usize, out: &mut Vec<Vec<u32>>) {
    if k == v.len() { out.push(v.clone()); return; }
    for i in k..v.len() {
        v.swap(k, i);
        permutations(v, k + 1, out);
        v.swap(k, i);                 // undo — exactly mirrors the make
    }
}

// Tail-recursive shape → write the loop; Rust does not guarantee TCO.
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 { let t = b; b = a % b; a = t; }
    a
}
```

**Ownership makes backtracking pleasant in one specific way:** `&mut State` plus explicit undo is the idiomatic form, and the borrow checker guarantees no sibling branch holds a stale reference into the state you're mutating. The cost is that you cannot hold references into the state across a recursive call — which is a feature, since that's exactly the aliasing that makes backtracking bugs so hard to find in other languages.

**For deep recursion you can't avoid**, the `stacker` crate grows the stack dynamically (`stacker::maybe_grow`), which is what `rustc` itself uses for deeply nested types. It's a real tool, not a hack — but an explicit stack is preferable when practical.

**Crates:** `stacker` (dynamic stack growth), `pathfinding` (IDA\*, DFS over implicit graphs), `itertools` (`permutations`, `combinations`, `powerset` — use these before hand-rolling).

## Use Cases

- **Constraint satisfaction** — Sudoku, N-queens, graph colouring, scheduling with either/or constraints. The whole family is backtracking plus propagation.
- **Combinatorial generation** — permutations, combinations, subsets, partitions. Reach for `itertools` first.
- **Parsing** — recursive descent mirrors the grammar exactly, which is why it's the default for hand-written parsers. And it's the standard place the depth guard is needed, since nesting comes from input.
- **Tree and expression evaluation** — recursion matches the structure; depth is Θ(tree height), so it's safe for balanced trees and unsafe for degenerate ones.
- **Game search** — minimax with alpha-beta pruning is backtracking with bound pruning; iterative deepening bounds the depth.
- **Path enumeration** — all paths, all Hamiltonian cycles, all spanning trees. Note these are exponential *by nature*, not by implementation.
- **Divide and conquer** — merge sort, quicksort, closest pair. Covered separately in [divide & conquer](../divide-and-conquer/learning.md).

## When to Use Which

| Reach for | When |
| --- | --- |
| **Recursion** | Depth provably bounded — balanced trees, fixed-size boards, bounded grammars |
| **Explicit stack** | Depth scales with input, or input is untrusted |
| Depth guard + recursion | You want recursion's clarity but need a clean failure |
| **Backtracking + pruning** | Search a combinatorial space for *some*/*all*/*best* solution |
| Backtracking + memoization | Subproblems repeat → it's [DP](../dynamic-programming/learning.md), and exponential becomes polynomial |
| Branch and bound | Optimization over a combinatorial space with a computable bound |
| `itertools` | Standard combinatorial generation — don't hand-roll |
| Iterative deepening | Depth unknown or huge, memory constrained, need optimality |
| **A solver (SAT/ILP)** | The problem is NP-hard and instances are large — see Stage 10 |

## Pitfalls in Depth

### Pitfall: Recursion depth from untrusted input

- **What goes wrong:** A recursive-descent parser, a tree walker, or a graph DFS meets deeply nested input — `[[[[[…]]]]]`, a long dependency chain, a path-shaped graph — and the process **aborts** with `fatal runtime error: stack overflow`. It's not a panic, so it can't be caught; the whole process dies, taking unrelated in-flight work with it. Measured: plain recursion survives ~100,000 frames on the main thread and aborts by 200,000, with a spawned thread failing around a quarter of that.
- **Why it happens (the mechanism):** The call stack is a fixed-size region set at thread creation and guarded by an unmapped page; overflowing it traps, and unwinding from a stack overflow isn't safe, so the runtime aborts. Nothing in a function's type marks it as depth-unbounded, so the risk is invisible in review, and test inputs are almost never deep.
- **How to handle it in production, and why that works:** Either convert to an explicit stack (depth then bounded by heap memory — the same measurement showed 5,000,000 handled fine), or keep recursion and add an explicit depth counter checked on entry, returning `Err` past a limit. `serde_json` does exactly the latter, which is why it has a recursion limit. The guard converts an uncatchable process abort into an ordinary rejected request.
- **Trade-offs of the fix:** The explicit-stack rewrite is significantly less readable for anything needing post-order (finish times), because you hand-roll the enter/exit machine the compiler gave you for free. A depth limit is a magic number that will eventually reject legitimate input. For provably shallow recursion — balanced trees, bounded grammars — both are wasted complexity; the trigger is *input-controlled depth*, not recursion.

### Pitfall: Forgetting to undo state

- **What goes wrong:** A backtracking search mutates shared state on the way down (mark a cell, push to a path, add to a set) and the undo is missing, partial, or in the wrong order. Sibling branches then start from a corrupted state: solutions are missed, duplicates appear, or a "used" marker never clears and the search terminates early with an empty result. The symptom appears in branches far from the missing undo.
- **Why it happens (the mechanism):** The make/unmake pair is separated by the recursive call, often by many lines, and any `continue`, early `return`, or `?` between them skips the undo. It's the same shape as a missing `free` — and the compiler can't help, because the mutation is perfectly legal.
- **How to handle it in production, and why that works:** Make undo structurally impossible to skip: put `apply` and `undo` adjacent around a single recursive call with no early exits between them, or use a guard type whose `Drop` performs the undo so any exit path restores state. Alternatively pass state **by value** for small states (clone the partial solution), which trades allocation for eliminating the entire bug class. Then add a `#[cfg(test)]` assertion that the state equals its entry value after each loop iteration.
- **Trade-offs of the fix:** Cloning state per node is Θ(state size) per node and can dominate the search — fine for a 9-element permutation, ruinous for a Sudoku grid explored millions of times. A `Drop` guard adds a type and some ceremony. The assertion is Θ(state size) so it stays behind `cfg(test)`.

### Pitfall: Backtracking without pruning

- **What goes wrong:** The search generates complete assignments and tests them at the leaves, rather than rejecting partial assignments as soon as they become infeasible. Measured on 9-queens: **2.20 s versus 50.38 µs — 43,580×**, for identical output. And the ratio *grows* with n (305× at n=6), so a version that seems merely slow on small instances is unusable on real ones.
- **Why it happens (the mechanism):** Generate-then-test is the straightforward translation of the problem statement ("try all assignments, keep the valid ones") and it's obviously correct, which makes it the natural first draft. But testing at the leaf explores nⁿ leaves; testing at each node prunes an entire subtree of size n^(n−depth) on every rejection. Pruning doesn't shave a constant — it removes whole branches, changing the effective branching factor.
- **How to handle it in production, and why that works:** Check feasibility **incrementally at every node**, using state that makes the check Θ(1) — bitmasks of attacked columns and diagonals for N-queens, candidate sets per cell for Sudoku. Then add ordering heuristics (most-constrained-variable first) so failures happen as high in the tree as possible, and symmetry breaking to avoid exploring equivalent branches.
- **Trade-offs of the fix:** Incremental feasibility state must be maintained through make and unmake, which is more code and another place for the undo bug above. Strong pruning (full constraint propagation) costs real time per node and can be net-negative on loosely-constrained problems where few branches would fail anyway — measure the node count *and* the wall clock, since fewer nodes at higher per-node cost isn't automatically a win.

### Pitfall: Ignoring repeated subproblems

- **What goes wrong:** A recursive solution explores the same subproblem many times. The canonical case is naive Fibonacci: measured at n = 40, **312.84 ms recursively against 333 ns with an array memo — roughly 939,000×**, and the gap doubles with each increment of n. Real instances of this are subtler: "minimum coins for amount X", "ways to reach cell (i,j)", "can this string be segmented" — all look like fresh recursions and all revisit states.
- **Why it happens (the mechanism):** The recursion tree has Θ(2ⁿ) nodes but only Θ(n) *distinct* states, so almost every node recomputes something already computed. The redundancy is invisible from the code, which correctly describes the relationship between a problem and its subproblems — it just doesn't notice that subproblems overlap.
- **How to handle it in production, and why that works:** Add a memo keyed by the subproblem's parameters, which is a two-line change converting exponential to polynomial. Then check whether the memo key space is dense: measured, an **array memo beat a `HashMap` memo by 13.4×** (333 ns vs 4.46 µs at n = 40) because integer-indexed states don't need hashing. If the recursion has no cycles, converting to bottom-up tabulation removes the call overhead entirely — that's [dynamic programming](../dynamic-programming/learning.md).
- **Trade-offs of the fix:** Memoization costs Θ(number of distinct states) memory, which can itself be prohibitive (a memo over subsets is Θ(2ⁿ) entries). It also only helps when subproblems actually repeat — adding a memo to a search whose states are all distinct is pure overhead. And a memo on a *mutable* state must key on the full state, which is often the reason a naive memo returns wrong answers.

### Pitfall: Assuming Rust optimizes tail calls

- **What goes wrong:** A tail-recursive function — accumulator-passing style, a loop written recursively — is expected to compile to a jump and instead consumes a stack frame per call. It overflows on large inputs, and the developer, coming from a language with guaranteed TCO, is surprised.
- **Why it happens (the mechanism):** Rust makes **no guarantee** of tail-call optimization. LLVM often performs it in release builds when it can prove no destructors need to run at the call site, but Rust's `Drop` semantics frequently prevent exactly that — a local with a destructor must be dropped *after* the call returns, which makes the call not-a-tail-call. So the optimization is unreliable, and it's absent in debug builds where you'd first hit the overflow.
- **How to handle it in production, and why that works:** Write the loop. Tail recursion is mechanically convertible — the accumulator becomes a mutable local and the call becomes a `continue`. The result is guaranteed constant stack, works identically in debug and release, and is usually clearer in Rust's idiom anyway.
- **Trade-offs of the fix:** Some algorithms genuinely read better recursively (mutual recursion between parser rules), and mechanical loop conversion can obscure them. For those, the depth guard is the better answer than a contorted loop.

## Creative & Lateral Thinking

| Lens | Question | What it produces |
| --- | --- | --- |
| Persist it | What if you kept every partial solution? | The full search tree — useful for explanation, replay, and debugging a search |
| **Batch it** | What if you cached repeated subproblems? | **Memoization → [dynamic programming](../dynamic-programming/learning.md)**; exponential becomes polynomial |
| Approximate it | What if you accepted a good-enough answer? | Local search, simulated annealing, beam search — bounded work, no optimality proof |
| **Randomize it** | What if you shuffled the choice order? | Randomized restarts — escapes the pathological orderings that make backtracking thrash |
| Externalize it | What if the search tree exceeded RAM? | Iterative deepening — DFS memory with BFS's completeness |
| **Parallelize it** | Where's the independence? | Subtrees are independent — fork at shallow depth; the challenge is load balance, since subtree sizes vary wildly |
| **Invert it** | What if you searched from the goal backwards? | Bidirectional search; regression planning; meet-in-the-middle splits 2ⁿ into 2·2^(n/2) |
| **Augment it** | What does a bound per node buy? | **Branch and bound** — prune by "the best possible completion is still worse than what I have" |
| **Specialize it** | What if the state fitted in a machine word? | **Bitmask state** — the measured 9-queens in 50 µs; feasibility as one AND |
| Amortize it | What if one branch could be expensive? | Constraint propagation: pay more per node to prune far more subtree |

**Questions:**

1. Pruning gave 43,580× at n = 9 and 305× at n = 6. Explain why the ratio *grows* with n, in terms of what a rejection at depth d removes.
2. Under "invert it", meet-in-the-middle turns 2ⁿ into 2·2^(n/2). Describe the technique for subset-sum, state the memory cost, and say why it doesn't apply to N-queens.
3. Under "batch it", adding a memo to a backtracking search makes it DP. State the precise condition on the search that makes memoization *valid*, and give a search where it silently gives wrong answers.
4. The N-queens bitmask propagates diagonals with `<<1` and `>>1`. Derive why those shifts are correct, and what `& full` is preventing.
5. Under "parallelize it", subtrees are independent but wildly unequal in size. Why does that make static partitioning bad, and which scheduling strategy fixes it?
6. Branch and bound prunes using an upper bound on the best completion. What happens to correctness if the bound is *too tight* (optimistic), and what happens to performance if it's too loose?
7. Under "randomize it", randomized restarts help backtracking. What distribution of runtimes makes restarts a win, and what does that say about the search's failure mode?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the two obligations of a correct recursion plus the termination requirement, and name what "well-founded measure" means concretely.
2. Give the backtracking skeleton's six steps and say which two are load-bearing.
3. Give the measured N-queens numbers at n = 9 and explain the mechanism behind the ratio.
4. State the state-restoration invariant and name three ways to make undo impossible to skip.
5. When is recursion safe in Rust, and what are the two fixes when it isn't?
6. Naive Fibonacci at n = 40 took 312.84 ms; an array memo took 333 ns. What's the structural property that makes memoization apply?

Build exercises:

- Implement N-queens three ways — generate-then-test, per-node feasibility with arrays, and bitmask state — and reproduce the ratios at n = 6…10. Count *nodes visited* as well as wall time; the node counts show pruning working, the wall time shows the bitmask constant factor.
- Write a recursive-descent parser for nested brackets, feed it 1,000,000 nested `[`, and watch it abort. Then fix it twice — explicit stack, and depth guard — and compare the failure modes (process death vs a clean `Err`).
- Implement Sudoku solving with plain backtracking, then add most-constrained-variable ordering, then add constraint propagation. Measure nodes and time at each step on a hard puzzle; MRV alone typically buys orders of magnitude.
- Take naive Fibonacci, add a `HashMap` memo, then an array memo, then convert to tabulation. Measure all four at n = 40 and reproduce the ~939,000× and the 13.4× memo-representation gap. This is the smoothest on-ramp to [dynamic programming](../dynamic-programming/learning.md).

## Open Questions

- How much does most-constrained-variable ordering buy over plain feasibility pruning on hard Sudoku, in nodes and in wall time?
- Does `stacker` have measurable overhead on a recursion that never needs to grow, and is it a reasonable default for parsers?
- For parallel backtracking with rayon, what depth should you fork at to balance subtree sizes without excessive task overhead?
- Meet-in-the-middle for subset-sum: at what n does the 2^(n/2) memory become the binding constraint on this machine?
- Is there a Rust SAT-solver binding worth reaching for once a backtracking search stops scaling, and what instance size does it handle?

## References

- CLRS ch. 4 (recursion and recurrences), ch. 34–35 (NP-completeness, approximation — where backtracking's limits live).
- Knuth, *The Art of Computer Programming*, Vol. 4A, §7.2.2 — backtracking, dancing links, and the definitive treatment of pruning.
- Knuth, "Dancing Links" (2000) — exact cover via DLX; the fastest known approach to Sudoku and N-queens-style exact-cover problems.
- Russell & Norvig, *AIMA* ch. 6 — constraint satisfaction: MRV, degree heuristic, forward checking, arc consistency, with the empirical comparisons.
- [`itertools`](https://docs.rs/itertools/) — `permutations`, `combinations`, `powerset`; use before hand-rolling.
- Related in this repo: [Dynamic Programming](../dynamic-programming/learning.md) (backtracking plus a memo), [Divide & Conquer](../divide-and-conquer/learning.md) (the other recursion shape), [Graph Traversal](../graph-traversal/learning.md) (backtracking is DFS on an implicit graph; the measured recursion limit), [Bit Manipulation](../bit-manipulation/learning.md) (bitmask state and `x & -x`), [Stacks & Queues](../stacks-and-queues/learning.md) (the explicit-stack rewrite).
