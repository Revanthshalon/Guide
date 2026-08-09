# Linked Lists — Quick Reference

## At a Glance

Trades locality for splice: Θ(1) insert/remove **at a cursor you already hold**, paid for with a dependent-load chain per element. Right only when splices outnumber traversals *and* something else supplies the cursor.

**Invariant (singly):** one owner per node; `next == None` exactly at the tail; chain from head reaches every node once.
**Invariant (doubly):** `node.next.prev == node` **and** `node.prev.next == node` — mutual, and the source of every doubly-linked bug.

## The Number That Decides It

Summing 5M `u64`, all Θ(n), measured:

| Layout | ns/element | Relative |
| --- | --- | --- |
| `Vec<u64>` | 0.13 | 1× |
| `Box` list, allocation order | 1.30 | 10× |
| `Box` list, scattered (realistic) | **84.78** | **641×** |

## Complexity

| Operation | Singly | Doubly |
| --- | --- | --- |
| Push/pop front | Θ(1) | Θ(1) |
| Push/pop back | Θ(n), Θ(1) with tail ptr | Θ(1) |
| Insert/remove **at cursor** | Θ(1)* | Θ(1) |
| Insert/remove at index i | Θ(i) | Θ(i) |
| Search | Θ(n) | Θ(n) |
| **Splice / concat** | **Θ(1)** | **Θ(1)** |
| Space | 3–4× array | 4× array |

`*` needs the *predecessor's* link, not the node.

## Choose This When

| Use it when | Use `Vec`/`VecDeque` when |
| --- | --- |
| A hash map supplies the cursor (LRU) | You have to search for the position |
| Intrusive: object on several lists, zero alloc | Any traversal-heavy workload |
| Θ(1) splice of large ranges | Random access or scanning |
| Lock-free queue (CAS swings one pointer) | Basically every other case |
| Elements need stable addresses | — |

## Rust

| Need | Use |
| --- | --- |
| Learning / immutable stack | `Option<Box<Node>>` |
| **Doubly-linked, practical** | **Arena + `u32` prev/next** |
| LRU | `lru` crate (don't rewrite) |
| Multi-list, zero alloc | `intrusive-collections` |
| Concurrent queue | `crossbeam` |
| `std::collections::LinkedList` | Essentially never — no stable mutable cursor, so no splice |

## Snippets

```rust
// Mandatory: without this, drop aborts at ~300k nodes
impl<T> Drop for List<T> {
    fn drop(&mut self) {
        let mut cur = self.head.take();
        while let Some(mut node) = cur { cur = node.next.take(); }
    }
}

// Cursor into the LINK, not the node
let mut cur = &mut self.head;
while let Some(node) = cur { cur = &mut cur.as_mut().unwrap().next; }

// Arena unlink: read neighbours first, then write
let (prev, next) = (nodes[i].prev, nodes[i].next);
match prev { Some(p) => nodes[p].next = next, None => head = next }
match next { Some(n) => nodes[n].prev = prev, None => tail = prev }
```

## Implementation Checklist

- [ ] Asked "where does the cursor come from?" before choosing this
- [ ] Iterative `Drop` written (`Box` variants)
- [ ] Doubly-linked: neighbours read into locals *before* any write
- [ ] All 8 boundary paths tested (head/tail/empty/single × insert/remove)
- [ ] `#[cfg(test)] check_invariants()` walks both directions, driven by `proptest`
- [ ] `Rc` back-edges are `Weak` (or, better, use the arena)
- [ ] Benchmarked the **aged** list, not the freshly-built one

## Common Bugs

| Bug | Symptom |
| --- | --- |
| No iterative `Drop` | `fatal runtime error: stack overflow` at ~300k (2 MB thread: ~75k) |
| Pointer writes ordered wrong | Traversable forward, corrupt backward; found much later |
| `Rc<RefCell>` both directions | Silent leak — refcounts never reach zero |
| Overlapping `borrow_mut()` | `BorrowMutError` panic under specific interleaving |
| Stale arena handle after reuse | Reads the wrong node, no crash |
| Benchmarked sequentially-built list | 65× optimistic vs production |

## Rules of Thumb

- A linked list you have to **search** is strictly worse than an array.
- "O(1) insert" is conditional — state where the cursor comes from or don't claim it.
- Doubly-linked in Rust → arena with `u32` indices, not `Rc<RefCell>`.
- Splice/concat is the one operation no array can match: Θ(1) at any size.
- Lists are the natural lock-free shape because CAS swings one pointer.
- Benchmark aged, not fresh.

## Key References

- [Too Many Linked Lists](https://rust-unofficial.github.io/too-many-lists/) — seven representations, all their failure modes
- Michael & Scott (1996) — the lock-free linked queue
- Linux `include/linux/list.h` — intrusive lists done right
