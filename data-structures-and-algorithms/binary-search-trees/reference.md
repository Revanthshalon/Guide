# Binary Search Trees & Balancing — Quick Reference

## At a Glance

Binary search made incremental: ordered *and* mutable, both Θ(log n). But the bound is a property of the **shape**, not the structure — and shape depends on insertion order.

**Invariant (search):** left subtree < node < right subtree. Says **nothing about height**.
**Invariant (balance):** whatever bounds the height — this is what makes a BST usable.

## The Number

100,000 keys, insert + lookup, unbalanced `Box` BST:

| Input | Time | Max depth |
| --- | --- | --- |
| **Sorted** | **29,689 ms** | **99,999** |
| Shuffled | 19.2 ms | 38 |
| `BTreeMap` (sorted input) | 9.2 ms | — |

**1,546×** from input order alone. Sorted input is the *common* case, not an edge case.

## Complexity

| Operation | Balanced | Unbalanced |
| --- | --- | --- |
| Search / insert / delete | Θ(log n) | **Θ(n)** |
| Min / max / pred / succ | Θ(log n) | Θ(n) |
| Range [a,b] | Θ(log n + k) | Θ(n) |
| In-order traversal | Θ(n) | Θ(n) |
| `select(k)` / `rank(x)` | Θ(log n) *if augmented* | — |
| Space | Θ(n), 2–3 ptr/node | same |

Θ(log n) ≈ 20 **dependent cache misses** at n = 10⁶ — why `BTreeMap` wins.

## Balancing Schemes

| Structure | Invariant | Height | Optimizes |
| --- | --- | --- | --- |
| AVL | subtree heights differ ≤ 1 | ≤ 1.44 log₂ n | reads |
| Red-black | equal black-height, no double red | ≤ 2 log₂ n | writes |
| **Treap** | heap order on **random** priority | ~3 log₂ n expected | **simplicity** |
| Splay | none — move accessed to root | Θ(log n) amortized | skewed access |
| Scapegoat | weight-balanced, rebuild subtree | amortized | no per-node data |

## Choose This When

| Use | For |
| --- | --- |
| **`BTreeMap`/`BTreeSet`** | Any ordered map/set — the default |
| `HashMap` | No ordering or ranges needed |
| Sorted `Vec` | Static/rarely mutated — faster ranges, smaller |
| **Augmented BST** | `select`/`rank` or interval overlap on a **changing** set |
| Treap / skip list | Want balance with far simpler code |
| Plain BST | **Never in production** |

## Augmentations

| Store per node | Buys |
| --- | --- |
| Subtree size | `select(k)`, `rank(x)` — order-statistic tree |
| Max endpoint | Interval overlap queries — interval tree |
| Subtree sum/min/max | Range aggregates over a changing set |

Maintainable iff computable from children in Θ(1). **Must be fixed inside rotations**, children before parent.

## Snippets

```rust
use std::collections::BTreeMap;
m.range(lo..hi);                                   // Θ(log n + k) — HashMap can't
m.first_key_value();                               // ordered min
m.range(..=target).next_back();                    // predecessor

// Rotation: fix augmented values bottom-up
self.fix_size(y);   // demoted node first
self.fix_size(x);   // then the promoted one
```

## Rules of Thumb

- Never ship a plain BST; balancing is what makes it work.
- Treap = ~20 lines for expected Θ(log n) on *any* insertion order.
- Rotations preserve the in-order sequence — that's why balancing is safe.
- Deletion's two-children case moves a **key**, not a node.
- Any augmentation must be recomputed inside every rotation.
- `Box` tree of depth 100k = a 100k-deep recursive drop → abort. Use an arena.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| No balancing + sorted input | 1,546× slowdown; depth = n |
| Augmentation not fixed in rotation | `select`/`rank` off by a data-dependent amount |
| Two-children delete mishandled | Silently lost nodes; traversal still looks sorted |
| Skipped rebalance on delete path | Gradual, unexplained decay |
| Recursive drop on a degenerate tree | `fatal runtime error: stack overflow` |

## Key References

- CLRS ch. 12–14 — BSTs, red-black, and the augmentation methodology
- Seidel & Aragon (1996) — treaps
- Pugh (1990) — skip lists
