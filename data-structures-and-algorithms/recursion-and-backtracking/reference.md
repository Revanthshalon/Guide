# Recursion & Backtracking — Quick Reference

## At a Glance

Recursion is an **inductive proof you can run**. Backtracking is **DFS over a decision tree you never build** — and **pruning is the entire algorithm**, not an optimization.

**Obligations:** base case correct · inductive step correct *assuming* smaller inputs work · every call strictly decreases a well-founded measure.
**Backtracking invariant:** the partial assignment is always consistent, and state is **exactly restored** on the way back up.

## The Number

N-queens, prune-as-you-go vs generate-then-test (measured):

| n | Solutions | Pruned | Generate-then-test | Ratio |
| --- | --- | --- | --- | --- |
| 6 | 4 | 4.08 µs | 1.24 ms | 305× |
| 8 | 92 | 23.92 µs | 119.92 ms | 5,014× |
| **9** | 352 | **50.38 µs** | **2.20 s** | **43,580×** |

The ratio **grows with n** — a rejection at depth d removes n^(n−d) leaves. Both are exponential; pruning changes the **base**.

**Recursion depth (measured):** ~100k frames OK, aborts by 200k (8 MB main thread); ~4× sooner on a 2 MB spawned thread. Explicit stack handled **5,000,000**.

## Complexity

| Pattern | Time | Space |
| --- | --- | --- |
| Linear recursion | Θ(n) | **Θ(n) stack** |
| Binary recursion, no memo | Θ(2ⁿ) | Θ(n) |
| Permutations | Θ(n!) | Θ(n) — n ≤ ~11 |
| Subsets | Θ(2ⁿ) | Θ(n) — n ≤ ~25 |
| Backtracking, pruned | Θ(b^d), **smaller b** | Θ(d) |

For backtracking, **measure — don't estimate from the bound.**

## The Skeleton

```rust
fn solve(state: &mut State, depth: usize, out: &mut Vec<Solution>) {
    if depth == state.n { out.push(state.snapshot()); return; }
    for choice in state.candidates(depth) {
        if !state.is_feasible(depth, choice) { continue; }  // ← PRUNE: the algorithm
        state.apply(depth, choice);
        solve(state, depth + 1, out);
        state.undo(depth, choice);                          // ← must exactly mirror apply
    }
}
```

## Pruning, in Increasing Power

| Technique | Example |
| --- | --- |
| Feasibility check | N-queens: column/diagonal attacked |
| Constraint propagation | Sudoku: cell with one candidate |
| Bound pruning | Branch and bound (TSP, knapsack) |
| Symmetry breaking | Half the first row |
| Ordering (MRV) | Fill most-constrained cell first |
| **Memoization** | → becomes **dynamic programming** |

## Bitmask State (why 9-queens took 50 µs)

```rust
let mut avail = full & !(cols | d1 | d2);       // whole row's feasibility: one AND
while avail != 0 {
    let bit = avail & avail.wrapping_neg();     // lowest set bit = one choice
    avail -= bit;
    queens(n, row+1, cols|bit, (d1|bit)<<1 & full, (d2|bit)>>1);
}
```

## Rules of Thumb

- Recurse when depth is **provably bounded**; iterate when it scales with input.
- Rust does **not** guarantee tail-call optimization — write the loop.
- `apply` and `undo` adjacent, no early exits between them (or use a `Drop` guard).
- Subproblems repeat → add a memo → it's DP.
- Array memo beat `HashMap` memo by **13.4×** for integer keys.
- Use `itertools` for permutations/combinations/powerset.
- Depth guard (`serde_json`-style) converts an abort into an `Err`.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Untrusted input depth | Uncatchable process abort |
| Missing/partial undo | Siblings corrupted; symptom far from cause |
| Early `return`/`?` between make and unmake | Same, intermittently |
| Generate-then-test | 43,580× slower at n=9; worse as n grows |
| Ignored repeated subproblems | 939,000× (naive Fibonacci at n=40) |
| Expected TCO | Overflow in debug; works in release |
| Memo on mutable state, wrong key | Silently wrong answers |

## Key References

- Knuth, TAOCP 4A §7.2.2 — backtracking and pruning
- Knuth, "Dancing Links" (2000) — exact cover
- Russell & Norvig, *AIMA* ch. 6 — MRV, forward checking, arc consistency
