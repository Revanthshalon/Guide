# B-Trees — Quick Reference

## At a Glance

A search tree redesigned around **blocks, not bytes**: many keys per node, so one fetched block answers many comparisons. The log's **base becomes the fanout**, and fanout is chosen to fill a block.

**Invariant:** every node holds t−1 … 2t−1 keys (root exempt); k keys ⇒ k+1 children; keys sorted; **all leaves at equal depth**. Grows at the **root**, never sideways — which is why it needs no rotations.

## The Number

| Structure | n = 10⁹ | Transfers |
| --- | --- | --- |
| Binary search tree | log₂ | ~30 misses ≈ 3 µs |
| B-tree, fanout 100 | log₁₀₀ | **~4.5 ≈ 0.5 µs** |

Range query over 1M entries (measured):

| Width | `BTreeMap::range` | `HashMap` scan | Sorted `Vec` |
| --- | --- | --- | --- |
| 100 | **0.0002 ms** | 2.9145 ms (**~14,500×**) | 0.0001 ms |
| 10,000 | 0.0114 ms | 3.0791 ms | **0.0034 ms** |

100k sorted keys, insert+lookup: `BTreeMap` **9.2 ms** vs a *shuffled* binary BST's 19.2 ms — 2× faster on harder input.

## Complexity

| Operation | Comparisons | **Transfers** |
| --- | --- | --- |
| Search / insert / delete | Θ(log n) | **Θ(log_B n)** |
| Range [a,b] | Θ(log n + k) | Θ(log_B n + k/B) |
| Traversal | Θ(n) | **Θ(n/B)** — sequential |
| Space | Θ(n) | ≥ 50% occupancy guaranteed |

B-trees do **more comparisons, fewer transfers**. Counting comparisons makes them look pointless.

## B-tree vs B+ tree

| | B-tree | **B+ tree** (all real DB indexes) |
| --- | --- | --- |
| Values | in every node | **leaves only** → higher fanout |
| Leaves linked | no | **yes** → sequential range scans |

Rust's `BTreeMap` is a B-tree with **B = 6 (≤11 keys/node)** — far below "fill a cache line", chosen by measurement.

## Choose This When

| Use | For |
| --- | --- |
| **`BTreeMap`/`BTreeSet`** | Ordered map/set that **changes** |
| `HashMap` | No ordering or ranges |
| Sorted `Vec` | Static/batch-built — faster ranges, smaller |
| B+ tree on disk | Read-heavy persistent index |
| LSM tree | Write-heavy persistent store |
| Binary balanced BST | Essentially never |

## Snippets

```rust
m.range(lo..hi);                    // the operation HashMap cannot do
m.range(lo..).take(10);             // next 10 after lo
m.range(..=t).next_back();          // predecessor
m.range(t..).next();                // successor
m.first_key_value();                // ordered min
let upper = m.split_off(&pivot);
*m.entry(k).or_insert(0) += 1;
```

## Rules of Thumb

- Fanout follows the **transfer cost** of the level you're optimizing — then measure; don't derive a constant.
- `HashMap` + a manual scan is a `BTreeMap` you haven't recognized.
- Keys need a genuine **total** `Ord` — floats need `total_cmp`/`NotNan`.
- Bulk-load from sorted data: near-100% occupancy and a shallower tree.
- Sequential key inserts leave nodes ~50% full — that's the split trail.
- Iteration is sorted **and deterministic** (unlike `HashMap`).

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Non-total `Ord` / `partial_cmp().unwrap()` | Insert then `get` returns `None` — looks like data loss |
| Mutated key via interior mutability | Entry permanently unreachable |
| Fanout raised "because shallower is faster" | Slower — in-node search and shifting dominate |
| `HashMap` for range queries | ~14,500× tax that grows with the map, not the answer |
| Never rebuilt a delete-heavy index | ~50% occupancy; index 2× larger than needed |

## Key References

- Bayer & McCreight (1972) — the original, motivated by block-device costs
- Comer, "The Ubiquitous B-Tree" (1979) — the survey
- Graefe, "Modern B-Tree Techniques" (2011) — concurrency, prefix truncation, bulk loading
