# Persistent & Immutable Structures — Quick Reference

## At a Glance

New version on every update, all old versions stay valid — via **structural sharing**: copy only the root-to-change path, share every untouched subtree by reference. Θ(log n) new nodes per update instead of Θ(n).

**Invariant:** nodes are never mutated once reachable. Every previously-returned root is still a complete structure.

**In Rust the killer property:** an immutable structure is **`Sync` for free** — readers hold an `Arc` and it can never change under them. No locks, no aliasing question.

## Levels

| Level | Read old | **Update** old | Example |
| --- | --- | --- | --- |
| Partial | yes | no | versioned arrays |
| **Full** | yes | **yes** | `im::Vector`, git |
| Confluent | yes | yes + **merge** | git merges, ropes |

## Complexity

| Structure | Lookup | Update | **Clone** | Space/version |
| --- | --- | --- | --- | --- |
| `Vec` / `HashMap` | **Θ(1)** | Θ(1) | **Θ(n)** | Θ(n) |
| `BTreeMap` | Θ(log n) | Θ(log n) | Θ(n) | Θ(n) |
| Persistent BST | Θ(log n) | Θ(log n) | **Θ(1)** | **Θ(log n)** |
| **HAMT** (`im::HashMap`) | Θ(log₃₂ n) ≈ **4 hops** | Θ(log₃₂ n) | **Θ(1)** | Θ(log₃₂ n) |
| **RRB** (`im::Vector`) | Θ(log₃₂ n) | Θ(log₃₂ n) | **Θ(1)** | Θ(log₃₂ n) |
| Cons list | Θ(n) | **Θ(1) prepend** | **Θ(1)** | **Θ(1)** |

Θ(1) clone is real. Θ(log₃₂ n) lookup hides ~4 dependent pointer loads — typically **2–10× slower** than the mutable equivalent on read-heavy work.

## Path Copying

```rust
Some(n) if key < n.key => Arc::new(Node {
    key: n.key,
    left:  Some(insert(&n.left, key)),   // copy this path
    right: n.right.clone(),              // ← Arc clone: whole subtree shared in Θ(1)
}),
```

**Balance matters more than for a mutable tree** — path copying costs Θ(depth) in *time and allocations*. Depth 99,999 ⇒ 99,999 allocations per insert.

## HAMT

Trie over **hash bits**, 32-way branching: 32-bit bitmap + packed child array, index via `popcount(bitmap & ((1<<i)-1))`. **~4 levels for 1M entries**, uniform depth regardless of key distribution.

## Choose This When

| Use | For |
| --- | --- |
| **`Vec` / `HashMap`** | Single owner, no versioning — the default |
| **`Arc<T>` + `Arc::make_mut`** | Usually one owner; copies only when actually shared |
| `Cow<'a, T>` | Usually borrowed, occasionally modified |
| **`im` / `rpds`** | Genuine versioning, or many concurrent readers |
| `rpds::List` | Prepend-heavy — persistence is **free** |
| **`arc-swap`** | Read-mostly shared state, atomically republished |
| CoW B-tree (LMDB, `redb`) | Persistent **and** on disk |

## Rules of Thumb

- Ask: is more than one version ever alive at once? No ⇒ don't pay for persistence.
- `Arc::make_mut` = persistent semantics at mutable speed when refcount is 1.
- Wide fanout (32-way) keeps the copied path to ~4 nodes.
- **Bound the version history** — retained roots pin all structure reachable from them.
- Immutable makes *reading* lock-free; **publishing** still needs CAS or a writer lock.
- A cons list's tail sharing is the one genuinely free persistence.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Persistence with no versioning | 2–10× slower for a clone nobody performs |
| Unbalanced persistent tree | Θ(n²) time **and allocations** |
| Retained old roots | Unbounded growth; every allocation genuinely reachable |
| Two threads deriving from one root | **Lost update**, silently — needs CAS on the root |
| Frequent snapshots + `make_mut` | Θ(n) cliff on every first write after a snapshot |

## Key References

- Okasaki, *Purely Functional Data Structures* — the canonical text
- Bagwell (2001) — "Ideal Hash Trees" (HAMT)
- Bagwell & Rompf (2011) — RRB-trees
- Driscoll et al. (1989) — making structures persistent
