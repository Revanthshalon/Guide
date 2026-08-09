# <Structure or Algorithm Name> — Learning Notes

<!-- Study material: read top-to-bottom and understood, not scanned. Ground everything in the invariant and the machine, never in the textbook recitation. -->

## Mental Model

<!-- The one-sentence idea, then the picture to reason with. What problem shape does this exist to answer? What does it buy, and what does it charge for it? -->

## The Invariant

<!-- The property maintained at all times — the thing every operation must preserve. State it precisely; most bugs in this topic are invariant violations, and most variants are this invariant relaxed or strengthened. -->

## Mechanics

<!-- How each operation actually works, step by step, with a small trace. Where the pointers/indices move, what gets rebalanced, what gets amortized away. -->

## Complexity

| Operation | Average | Worst | Amortized | Space | Notes |
| --- | --- | --- | --- | --- | --- |
| | | | | | |

<!-- Then, in prose: where the bound is misleading. Which constant is large, what the cache behavior does to the asymptotics at realistic n, and the n at which the "worse" alternative actually wins. -->

## Rust Implementation

<!-- Idiomatic implementation, with the ownership story made explicit: Vec-of-indices vs Box vs Rc<RefCell>, why the borrow checker pushes toward arenas here, where `unsafe` would be needed and whether it's worth it. -->

```rust
```

**In std / the ecosystem:** <!-- What already exists (`Vec`, `BTreeMap`, `BinaryHeap`, `hashbrown`, `petgraph`...), how it differs from the textbook version, and when to write your own instead. -->

## Use Cases

<!-- Real problems this is the right answer to — concrete systems, not "when you need fast lookup". Include at least one from this repo's other categories where the structure shows up in production. -->

## When to Use Which

| Reach for this when | Reach for <alternative> instead when |
| --- | --- |
| | |

<!-- The decision, framed as trade-offs against the 2-3 structures actually competing for the same job. -->

## Pitfalls in Depth

### Pitfall: <name>

- **What goes wrong:**
- **Why it happens (the mechanism):**
- **How to handle it in production, and why that works:**
- **Trade-offs of the fix:**

## Creative & Lateral Thinking

<!-- The point of this section: stop memorizing structures, start deriving them. Apply the transformation lenses, then answer the topic-specific questions. -->

**Transformation lenses** — run this structure through each and note what falls out (some produce a real, named structure; some produce nothing, and knowing why is the lesson):

| Lens | Question to ask |
| --- | --- |
| Persist it | What if every update returned a new version and the old one stayed valid? |
| Batch it | What if updates arrived 10,000 at a time instead of one at a time? |
| Approximate it | What if a 1% error rate bought an order of magnitude in space or time? |
| Randomize it | What if a coin flip replaced the balancing logic? |
| Externalize it | What if it didn't fit in RAM and the unit of transfer were a 4 KB page? |
| Parallelize it | What if 16 threads touched it at once — what's the contention point? |
| Invert it | What if you swapped which operation is fast and which is slow? |
| Augment it | What extra field per node buys a whole new query for free? |
| Specialize it | What if keys were known to be small integers / sorted / unique? |
| Amortize it | What if you allowed one operation to be terrible so the rest could be great? |

**Questions:**

<!-- 4-6 topic-specific questions with no lookup-able answer: derive-it-from-scratch, why-not-the-obvious-alternative, what-breaks-if-the-invariant-relaxes, what-does-this-become-under-lens-X. -->

## Exercises & Self-Test

<!-- Retrieval practice: 4-6 questions answerable from the mental model without rereading. Then build exercises in Rust — implement it, break it deliberately, and benchmark against the std equivalent. -->

## Open Questions

<!-- Things not yet understood or verified — revisit on the next study session. -->

## References

<!-- Papers, chapters, talks, source code worth reading — annotate each with what it's good for. -->
