# Rust for Data Structures — Quick Reference

## At a Glance

Textbook structures assume many mutable pointers into one node graph — the one thing Rust forbids. Pick one of five representation strategies up front; it determines API, performance, and whether you need `unsafe`.

**Invariant:** aliasing XOR mutability — any number of `&T`, *or* exactly one `&mut T`, never both.

## The Five Strategies

| Strategy | Use when | Cost |
| --- | --- | --- |
| `Box` tree | Downward links only, one owner (BST, trie, AST) | No back-links; recursive `Drop` |
| **Arena + `u32` handles** | **Any graph, back-pointers, cycles — the default** | Manual node lifetime; stale handles |
| Generational arena | Arena + handles outlive removals | +4 B/handle, +4 B/slot |
| `Rc<RefCell<T>>` | Nodes genuinely shared with external code | Runtime panics, refcounts, cycle leaks |
| Slices / split borrows | It's really an array (heap, union-find, Fenwick) | Contiguous data only |
| Raw pointers + Miri | Intrusive/self-referential, or measured hot path | Full `unsafe` obligation |

**An index is not a borrow** — that's the whole arena trick.

## Complexity of the Choice

| Strategy | Per link | Traversal | Alloc/node | Safe at compile time |
| --- | --- | --- | --- | --- |
| `Box` | 8 B | scattered chase | yes | yes |
| Arena `u32` | 4 B | sequential-friendly | no | handles unchecked |
| Gen. arena | 4 B | sequential-friendly | no | stale → `None` |
| `Rc<RefCell>` | 8 B + 16 B counts | chase + refcount | yes | runtime panic |

## Snippets

```rust
// Descend a Box tree with &mut — cursor into the hole
let mut cur = &mut self.root;
while let Some(node) = cur {
    cur = if key < node.key { &mut node.left } else { &mut node.right };
}
*cur = Some(Box::new(Node { key, left: None, right: None }));

// Iterative Drop — required for any deep Box chain
impl Drop for List {
    fn drop(&mut self) {
        let mut cur = self.head.take();
        while let Some(mut node) = cur { cur = node.next.take(); }
    }
}

// Generational handle
struct Handle { idx: u32, gen: u32 }
fn get(&self, h: Handle) -> Option<&T> {
    let slot = self.slots.get(h.idx as usize)?;
    if slot.0 == h.gen { slot.1.as_ref() } else { None }
}

// Disjoint &mut into one buffer
let (l, r) = slice.split_at_mut(mid);
let [a, b] = arr.get_disjoint_mut([i, j]).unwrap();   // 1.86+
```

## std Facts That Change Constants

| Fact | Consequence |
| --- | --- |
| `Option<Box<T>>` = 8 B (niche) | Nullable links are free |
| `Vec<T>` = 24 B, `Box<[T]>` = 16 B | Freeze built arrays |
| `HashMap` = hashbrown SwissTable | SIMD probe; good constant |
| Default hasher = SipHash-1-3 + random seed | ~1 ns/byte; DoS-resistant |
| `FxHashMap`/`aHash` | **4.6–6.0×** faster for `u32` keys, only ~1.2× for 16-char `String` — **self-generated keys only** |
| `BinaryHeap` is a max-heap | `BinaryHeap<Reverse<T>>` for min |
| `Entry` API | One hash instead of two |
| `String: Borrow<str>` | `map.get("x")` on `HashMap<String,V>` |

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Main-thread stack | 8 MB |
| Spawned-thread stack | 2 MB |
| Recursive `Box` drop survives | ~250k nodes (measured, release) |
| Recursive `Box` drop aborts | ~300k nodes |

## Implementation Checklist

- [ ] Chose a strategy deliberately (default: arena)
- [ ] Deep `Box` chain? iterative `Drop` written
- [ ] Handles escape or outlive removals? generational
- [ ] `Rc` back-edges are `Weak`
- [ ] `Ord` total (floats: `total_cmp` / `ordered-float`)
- [ ] `Hash` and `Eq` derived together
- [ ] Hasher chosen per map by key provenance
- [ ] `unsafe`? contained in module + `cargo +nightly miri test`
- [ ] Invariant checker behind `#[cfg(test)]`, driven by `proptest`

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Recursive `Drop` | `fatal runtime error: stack overflow` at drop, far from build site |
| Stale arena handle | Silent read/write of the wrong node |
| `Rc` cycle without `Weak` | Steady memory growth, no leak in tests |
| Overlapping `borrow_mut()` | `BorrowMutError` panic under specific interleaving |
| Non-total `Ord` | `BTreeMap` insert-then-get returns `None` |
| `partial_cmp().unwrap()` on floats | Weekly panic when a `NaN` appears |
| `Hash`/`Eq` disagree | Duplicate entries in a `HashSet` |

## Key References

- [Learn Rust With Entirely Too Many Linked Lists](https://rust-unofficial.github.io/too-many-lists/) — read before writing any linked structure
- Catherine West, RustConf 2018 keynote — why generational arenas won
- [`slotmap`](https://docs.rs/slotmap/) · [Miri](https://github.com/rust-lang/miri)
