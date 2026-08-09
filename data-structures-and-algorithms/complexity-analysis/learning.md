# Complexity Analysis — Learning Notes

## Mental Model

**Complexity analysis predicts how cost *scales*, and deliberately refuses to predict how long anything takes.** That refusal is the source of all its power and every one of its lies.

The model underneath it is the **RAM machine**: unit-cost arithmetic, unit-cost random access to any memory address, no cache, no branch predictor, no allocator. On that machine, counting operations is counting time. On real hardware, a random DRAM access costs ~100 ns and a sequential L1 access ~1 ns — a **100× spread that Big-O is defined to ignore**. So the right way to hold it:

> Asymptotics tell you which algorithm wins **eventually**. Measurement tells you whether "eventually" is before or after the largest input you'll ever see.

Both halves are load-bearing. Skip the asymptotics and you ship an O(n²) loop that works fine on 500 rows and melts on 50,000. Skip the measurement and you ship a "galactic" algorithm with a constant factor of 3,000 that loses to the naive one at every input in the universe.

The useful working stance: **use asymptotics to eliminate, use measurement to choose.** Asymptotics reliably rules out the disasters — an O(n²) scan inside a request handler, an O(n) `remove(0)` in a loop. Once two candidates are in the same class (or within one log factor), asymptotics has nothing more to say and you go measure, because the answer is now decided entirely by cache behavior, branch predictability, and allocation.

## The Invariant

An asymptotic claim is a statement with quantifiers, and every misuse comes from dropping them:

> `f(n) = O(g(n))` iff **∃** c > 0, **∃** n₀, **∀** n ≥ n₀: `f(n) ≤ c · g(n)`

Two escape hatches are baked into the definition, and every "Big-O lied to me" story is one of them:

- **The constant `c`** — unbounded. `n` and `1000n` are both O(n). Big-O cannot tell you that a linked-list traversal and an array scan differ by 100×, because it is *defined* not to.
- **The threshold `n₀`** — unbounded. The claim only applies for large enough n. If n₀ is 4 billion, the claim is true and useless.

The related notations, kept honest:

| Notation | Means | The usual sin |
| --- | --- | --- |
| `O(g)` | upper bound — "no worse than" | Saying O(n²) when you mean Θ(n²); technically every algorithm is O(n!) |
| `Θ(g)` | tight bound — "exactly this rate" | This is what people *mean* 90% of the time they say Big-O |
| `Ω(g)` | lower bound — "no better than" | Used for proving problem hardness (comparison sort is Ω(n log n)) |
| `o(g)` | strictly smaller | Rare outside proofs |

## Mechanics

### The four cost measures — these are not interchangeable

| Measure | Quantified over | Guarantee strength | Example |
| --- | --- | --- | --- |
| **Worst case** | all inputs | absolute | Quicksort: Θ(n²) |
| **Average case** | a distribution over inputs | only if your input matches that distribution | Quicksort on uniformly random input: Θ(n log n) |
| **Expected** | the algorithm's own coin flips | holds for *every* input | Randomized quicksort: Θ(n log n) expected, regardless of input |
| **Amortized** | a *sequence* of operations | absolute, worst-case over the sequence | `Vec::push`: Θ(1) amortized |

Two distinctions worth burning in:

**Average ≠ expected.** Average case assumes your inputs are random. Expected means *the algorithm* is random, and holds even for adversarial input. This is exactly why Rust's `HashMap` seeds its hasher randomly and why `sort_unstable` randomizes its pivot choice: it converts an average-case hope into an expected-case guarantee an attacker cannot defeat.

**Amortized ≠ average.** Amortized is not probabilistic at all — it's a worst-case bound on a sequence, spread across the operations in it. "O(1) amortized" means *n* pushes cost O(n) total, guaranteed. It does **not** mean any individual push is fast, which is the pitfall that bites latency-sensitive code.

### The three amortization arguments

Take `Vec::push` with capacity doubling:

- **Aggregate.** *n* pushes trigger reallocations at capacities 1, 2, 4, …, n. Total elements copied = 1 + 2 + 4 + … + n < 2n. So n pushes cost < 3n operations → **O(1) amortized**. The geometric series is the whole trick; a growth factor of 1.5 or 2 works, but growing by a *constant* (+16 each time) gives 1+2+…+n/16 = Θ(n²) total — Θ(n) amortized per push, and this is a real bug people write.
- **Accounting (banker's).** Charge each push 3 credits: 1 for the write, 2 saved on the element. When the vector doubles from n to 2n, the n elements copied are each paid for by their own saved credits. Credits never go negative → the charge is an upper bound.
- **Potential.** Define Φ = 2·len − cap (≥ 0 when at least half full). Amortized cost = actual + ΔΦ. A cheap push: actual 1, ΔΦ = 2 → 3. A doubling push: actual n+1 copies, but cap doubles so ΔΦ = 2 − n → still ~3. Constant everywhere.

The potential method is the one that generalizes — it's how splay trees, Fibonacci heaps, and union-find are analyzed.

### Recurrences

Divide-and-conquer costs come out of `T(n) = a·T(n/b) + f(n)` — *a* subproblems of size *n/b*, plus f(n) to split and combine. The **Master Theorem** compares f(n) against `n^(log_b a)`:

| Case | Condition | Result | Example |
| --- | --- | --- | --- |
| 1 — leaves dominate | f(n) = O(n^(log_b a − ε)) | Θ(n^(log_b a)) | Karatsuba: a=3, b=2, f=n → Θ(n^1.585) |
| 2 — balanced | f(n) = Θ(n^(log_b a)) | Θ(n^(log_b a) · log n) | Merge sort: a=2, b=2, f=n → Θ(n log n) |
| 3 — root dominates | f(n) = Ω(n^(log_b a + ε)), regular | Θ(f(n)) | T(n) = 2T(n/2) + n² → Θ(n²) |

Worth memorizing: binary search (a=1, b=2, f=1) → Θ(log n). Strassen (a=7, b=2) → Θ(n^2.807). And the shape that isn't covered by the theorem but shows up constantly: `T(n) = T(n−1) + n` → Θ(n²), the signature of "recursive call that only removes one element."

### Where the RAM model breaks: the I/O model

When data doesn't fit in a fast level, the honest model counts **block transfers**, not operations: memory moves in units of B (a 64-byte cache line, or a 4 KB page). Under this model a B-tree with fanout B costs `O(log_B n)` transfers versus a binary tree's `O(log₂ n)`:

| Structure | n = 10⁹, unit | Transfers |
| --- | --- | --- |
| Binary search tree | log₂(10⁹) | ~30 random accesses ≈ 3 µs |
| B-tree, fanout 100 | log₁₀₀(10⁹) | ~4.5 block reads ≈ 0.5 µs |

Same asymptotic class (both logarithmic), 6× real difference. This is why `BTreeMap` exists in std and a red-black tree does not, and it's the seam where this category meets [cache locality](../../performance-optimization/cache-locality/learning.md).

## Complexity

Growth rates, with the number that actually matters — operations at realistic n:

| Class | n = 10 | n = 1,000 | n = 1,000,000 | Verdict at scale |
| --- | --- | --- | --- | --- |
| O(1) | 1 | 1 | 1 | — |
| O(log n) | 3 | 10 | 20 | free |
| O(n) | 10 | 10³ | 10⁶ | ~1 ms at 1 ns/op |
| O(n log n) | 33 | 10⁴ | 2×10⁷ | ~20 ms |
| O(n²) | 100 | 10⁶ | 10¹² | **~17 minutes** |
| O(n³) | 10³ | 10⁹ | 10¹⁸ | ~30 years |
| O(2ⁿ) | 10³ | — | — | dead past n ≈ 25 |
| O(n!) | 3.6×10⁶ | — | — | dead past n ≈ 11 |

The practical reading: **the n² cliff is the one that ships to production**, because n² is invisible in tests and catastrophic at scale. n³ and 2ⁿ are caught in code review; a nested loop over the same collection is not.

A companion table — how much n you can afford in one second at ~10⁹ simple ops/sec:

| Complexity | Feasible n |
| --- | --- |
| O(log n) | anything |
| O(n) | ~10⁹ |
| O(n log n) | ~5×10⁷ |
| O(n²) | ~3×10⁴ |
| O(n³) | ~1,000 |
| O(2ⁿ) | ~30 |
| O(n!) | ~11 |

**Space complexity** follows the same rules with two traps: *auxiliary* space (extra, beyond input) is what people usually mean by "in-place", and the **recursion stack counts**. A recursive DFS on a 1M-node path graph is O(n) stack — and Rust's main thread gets 8 MB (spawned threads 2 MB), which at ~48–64 bytes/frame overflows somewhere around 100k–150k frames. That's a crash, not a slowdown.

## Rust Implementation

**The complexity contract of std**, which is documented and stable — worth knowing cold:

| Type | Get | Insert | Remove | Notes |
| --- | --- | --- | --- | --- |
| `Vec<T>` | O(1) | O(1)* amortized push; O(n) `insert` | O(1) `swap_remove`, O(n) `remove` | `remove(0)` in a loop is the classic accidental O(n²) |
| `VecDeque<T>` | O(1) | O(1) amortized both ends | O(1) both ends | ring buffer; not contiguous — `make_contiguous()` if you need a slice |
| `HashMap<K,V>` | O(1) expected | O(1) amortized expected | O(1) expected | SwissTable (hashbrown); worst case O(n) |
| `BTreeMap<K,V>` | O(log n) | O(log n) | O(log n) | fanout ~11; wins on range queries and small n |
| `BinaryHeap<T>` | O(1) peek | O(log n) push | O(log n) pop | `from(vec)` heapifies in O(n), not O(n log n) |
| `HashSet`/`BTreeSet` | as their map | | | |

**Measuring the exponent empirically — the doubling experiment.** This is the single most useful measurement technique in this doc, because it recovers the true complexity *including* constants and cache effects:

```rust
// Run the operation at n, 2n, 4n, 8n and take ratios of the times.
// ratio = T(2n)/T(n)  →  exponent ≈ log2(ratio)
//
//   ratio ≈ 1.0  → O(1)
//   ratio ≈ 1.1  → O(log n)
//   ratio ≈ 2.0  → O(n)
//   ratio ≈ 2.1  → O(n log n)      (the log shows up as the 0.1)
//   ratio ≈ 4.0  → O(n²)
//   ratio ≈ 8.0  → O(n³)
fn doubling_experiment<F: Fn(usize)>(f: F, start: usize, steps: u32) {
    let mut prev = None;
    for i in 0..steps {
        let n = start << i;
        let t0 = std::time::Instant::now();
        std::hint::black_box(f(n));
        let dt = t0.elapsed().as_secs_f64();
        match prev {
            Some(p) => println!("n={n:>9}  {dt:>9.4}s  ratio={:.2}", dt / p),
            None => println!("n={n:>9}  {dt:>9.4}s"),
        }
        prev = Some(dt);
    }
}
```

Two cautions: start above the cache-resident sizes or the first few ratios measure L2, not your algorithm; and `black_box` the result or the optimizer deletes the work (see [profiling & measurement](../../performance-optimization/profiling-and-measurement/learning.md)).

**The `n₀` in Rust's own source.** `sort_unstable` (pdqsort) drops to insertion sort below ~20 elements. That constant *is* n₀ made concrete: insertion sort is Θ(n²) and still faster there, because 20 elements are one or two cache lines and the branch predictor learns them. Every well-tuned library has these thresholds; finding yours is a measurement, never a derivation.

## Use Cases

- **Reviewing a PR.** The highest-value application is the n² sniff test: a loop containing `.contains()`, `.position()`, `.iter().find()`, `remove(0)`, or string `+=` on a growing buffer. Each is a linear operation nested inside a linear loop.
- **Capacity planning.** "This is O(n log n) at 50k rows today; at 5M rows it's 100× the work, not 100× the rows" — the arithmetic that decides whether a design survives the next order of magnitude.
- **Choosing between two library types.** `HashMap` vs `BTreeMap` is not decided by O(1) vs O(log n); it's decided by whether you need ordered iteration or range queries, and (at small n or expensive hashes) by measurement.
- **Setting SLOs.** Amortized bounds are the tell: if your p99 matters, an O(1)-amortized structure with O(n) worst-case spikes needs `reserve()` up front or a different structure entirely.
- **Knowing when to stop.** Recognizing a problem as NP-hard *before* spending a week on an exact algorithm — the Stage 10 topic *Intractability & Approximation*, not yet written.

## When to Use Which

| Reason from asymptotics when | Reason from measurement when |
| --- | --- |
| Candidates differ by a whole class (n vs n²) | Candidates are in the same class, or within a log factor |
| n varies by orders of magnitude or is attacker-influenced | n is bounded and known (e.g. always < 1000) |
| You're choosing a data structure before writing code | You're choosing between two working implementations |
| Reviewing for scalability cliffs | Chasing a constant factor: layout, allocation, branches |
| The cost is dominated by operation count | The cost is dominated by memory traffic (usually) |

## Pitfalls in Depth

### Pitfall: Quoting the average case as if it were a guarantee

- **What goes wrong:** "Hash map lookup is O(1)" gets written into a design doc. In production, a user-controlled key space collides — or an attacker constructs colliding keys — and every lookup degrades to a linear scan of one bucket. p99 goes from 200 µs to 40 ms and the service falls over under load it handled yesterday.
- **Why it happens (the mechanism):** O(1) for hashing is an *average over a random hash function*, not a worst case. Adversarial keys make the bucket distribution arbitrarily bad. The 2011 hash-collision DoS wave broke PHP, Java, Python, Ruby and Node simultaneously for exactly this reason.
- **How to handle it in production, and why that works:** Rust's default is already the fix — `HashMap` uses SipHash-1-3 with a per-process random seed, so an attacker cannot precompute collisions. Keep it for anything touching user input. Swap to `FxHashMap`/`aHash` (often 2–3× faster) **only** for keys you generate yourself: node indices, interned IDs, enum discriminants.
- **Trade-offs of the fix:** SipHash costs ~1 ns/byte, which genuinely dominates lookup time for small keys — this is a real 2–3× on hot internal maps. The rule is per-map, not global: classify each map by whether its keys are attacker-reachable.

### Pitfall: Treating "amortized O(1)" as a latency guarantee

- **What goes wrong:** A request handler appends to a `Vec` that has grown to 4M elements. One unlucky push reallocates: 16 MB memcpy plus an allocator round trip, ~2 ms, on a p99 budget of 5 ms. The pathology is invisible in averages and shows up as an unexplained periodic spike.
- **Why it happens (the mechanism):** Amortization spreads cost *analytically*, not *temporally*. The doubling still happens all at once, and it gets more expensive exactly as the structure gets larger — so the spikes grow while getting rarer, which is the worst possible shape for tail latency.
- **How to handle it in production, and why that works:** `Vec::with_capacity(n)` / `reserve()` when the size is known or boundable — this moves the entire cost to a point you chose. When it isn't boundable, use a chunked structure that never copies (`VecDeque` for queues, or a chunked/rope-like vector), so growth allocates a new block instead of relocating everything.
- **Trade-offs of the fix:** Preallocating over-commits memory when the estimate is high; chunked structures give up contiguity, which costs you `&[T]` slices, SIMD-friendly scans, and some cache locality. It's a genuine latency-vs-throughput trade, so make it deliberately.

### Pitfall: Complexity stated in the wrong variable

- **What goes wrong:** "Lookup is O(1)" for a `HashMap<String, V>` where keys are 200-byte paths. Every lookup hashes 200 bytes and, on a hit, memcmp's 200 bytes. The map is O(1) in the *number of entries* and O(k) in the *key length* — and k is what dominates. Same class of error: "sorting is O(n log n)" when comparisons are string comparisons of length k, making it O(k · n log n).
- **Why it happens (the mechanism):** The RAM model assumes unit-cost comparison and hashing. That's fine for `u64`, false for anything variable-length. The n in your head is the count; the cost lives in a variable you didn't name.
- **How to handle it in production, and why that works:** Name every variable in the bound — n *and* k. Then shrink k: **intern** strings to `u32` IDs at the boundary and use `HashMap<u32, V>` internally (this is what compilers do, and why `rustc` has a `Symbol` type); or precompute and cache hashes. For sorting, decorate-sort-undecorate with a cheap comparable key.
- **Trade-offs of the fix:** Interning adds a bidirectional table and a translation step at the edges, and IDs are meaningless in logs without a lookup. Worth it exactly when the same keys are compared or hashed many times.

### Pitfall: The accidental quadratic

- **What goes wrong:** Code that reads as one linear pass but nests a linear operation inside it. `for x in &items { if seen.contains(&x) {...} }` where `seen` is a `Vec`. `while !v.is_empty() { v.remove(0) }`. Building a string with `s = s + &part` in a loop. Each is fine on the 200-element test fixture and quadratic on the 200,000-element production input.
- **Why it happens (the mechanism):** The inner cost is hidden behind a method call that *looks* O(1). `Vec::contains` is a scan; `Vec::remove(0)` shifts every element; `String + &str` in a loop reallocates and copies the accumulated prefix each time. Nothing in the syntax signals the nesting.
- **How to handle it in production, and why that works:** Learn the small set of linear-operations-that-look-constant (`contains`, `position`, `find`, `remove(i)`, `insert(i)`, `dedup` on unsorted data) and treat any of them inside a loop as a defect. The fixes are mechanical: `HashSet`/`BTreeSet` for membership, `VecDeque::pop_front` for queues, `swap_remove` when order doesn't matter, `String::push_str` into a `with_capacity` buffer, `retain` instead of repeated `remove`.
- **Trade-offs of the fix:** A `HashSet` costs a hash per probe and loses cache locality; below roughly 30–100 elements a linear scan of a `Vec` genuinely beats it. The point isn't "always use a set" — it's that the choice must be *made* rather than inherited from whichever type was already there.

### Pitfall: Optimizing an asymptotic win that never arrives

- **What goes wrong:** Replacing a straightforward O(n log n) with an O(n) algorithm that has an enormous constant, or hand-rolling a Fibonacci heap for its O(1) decrease-key. The new code is slower at every n the system will ever see, and it's harder to read and to debug.
- **Why it happens (the mechanism):** The n₀ escape hatch. Fibonacci heaps have famously bad constants and poor locality — a binary heap beats them on real graphs despite the worse bound. Galactic algorithms (matrix multiplication at O(n^2.37)) are the extreme case: correct, provable, never used.
- **How to handle it in production, and why that works:** Require the crossover point as evidence, not the exponent: "this wins above n = 40,000; our p99 input is 900" ends the discussion with data. Run the doubling experiment on both implementations over the actual input range.
- **Trade-offs of the fix:** Sometimes the asymptotically better algorithm really is needed and its crossover is genuinely below your n — the demand is for measurement, not for conservatism. The failure mode in the other direction (dismissing a real n² because "our data is small") is equally expensive when the data grows.

## Creative & Lateral Thinking

**Transformation lenses** applied to *analysis itself* — each lens is really a different cost model:

| Lens | What it changes about the analysis | What it yields |
| --- | --- | --- |
| Persist it | Count versions, not just operations | Persistence overhead; structural-sharing bounds |
| Batch it | Analyze a sequence, not an operation | Amortized analysis; the potential method |
| Approximate it | Trade exactness for a bound | (1+ε)-approximation; sketch space bounds |
| Randomize it | Quantify over coin flips, not inputs | Expected-case bounds that hold on adversarial input |
| Externalize it | Count block transfers, not operations | The I/O model; O(log_B n) and why B-trees exist |
| Parallelize it | Two measures: work T₁ and span T∞ | Brent's bound T_p ≤ T₁/p + T∞; parallelism = T₁/T∞ |
| Invert it | Bound the *problem*, not the algorithm | Lower bounds: Ω(n log n) for comparison sorting |
| Augment it | Charge for maintaining extra invariants | Why augmented trees stay O(log n) |
| Specialize it | Add a precondition to the input | Radix sort's O(nk) escaping the comparison bound |
| Amortize it | Let one operation be terrible | Dynamic arrays; union-find's inverse Ackermann |

**Questions:**

1. Every algorithm is O(n!). Why is that statement both true and worthless — and what does it tell you about why practitioners should almost always say Θ?
2. Randomized quicksort is Θ(n log n) *expected* on every input; deterministic quicksort is Θ(n log n) on *average* over random inputs. An attacker controls your input. Explain precisely why only one of these survives, without using the word "random" loosely.
3. You have an O(n) algorithm and an O(n log n) one, and the O(n) one is slower for every n up to 10⁹. Nothing has been violated — where exactly is the O(n) claim still true, and what would you have to change about the hardware to make it win?
4. Under the parallelize lens, an algorithm with T₁ = n and T∞ = n has parallelism 1. Give an example, and explain why its perfect linear work bound is irrelevant on 64 cores.
5. `BinaryHeap::from(vec)` heapifies in O(n) while n pushes cost O(n log n) — both build the same heap. Where does the log go? (Hint: count nodes by height, not by index.)
6. Amortized analysis gives worst-case guarantees over a sequence; a real-time system rejects it anyway. What does "deamortizing" a structure mean, what does it cost, and which of the amortized structures you know would be hardest to deamortize?
7. Under the externalize lens, B-tree fanout is chosen to fill a page. If a "page" were 64 bytes (a cache line) instead of 4 KB, what fanout falls out for `u64` keys — and is that the number `BTreeMap` actually uses?

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Write the full quantified definition of `f = O(g)`, then name which quantifier is responsible for each of: "Big-O ignores constants", "Big-O only applies to large inputs".
2. A `Vec` grows by adding 16 slots instead of doubling. Give the total cost of n pushes and the amortized per-push cost, and identify which series changed.
3. Give the Master Theorem case and result for: T(n) = 2T(n/2) + n; T(n) = 4T(n/2) + n; T(n) = T(n/2) + 1; T(n) = T(n−1) + n.
4. A doubling experiment reports ratios 2.05, 2.08, 2.11, 2.09. What is the complexity, and how do you distinguish this from O(n) with measurement noise?
5. You see `HashMap<String, V>` with 200-byte keys and 50 entries, in a hot loop. Give two independent reasons the O(1) claim is misleading here, and the fix for each.
6. Which of these is a worst-case guarantee, and which two are conditional on something: amortized O(1), expected O(1), average O(1)? Name the condition for each conditional one.

Build exercises:

- Implement the doubling experiment above and run it against `Vec::contains` (linear), `HashSet::contains` (constant), and `BTreeSet::contains` (logarithmic) at n from 2¹⁰ to 2²². Confirm the ratios come out at 2.0, 1.0, and ~1.05 — and find the n where `Vec::contains` first loses to `HashSet::contains` on your machine. That crossover is n₀ made real; note it down, it's a number you'll reuse.
- Reproduce the amortization spike: push 10M `u64` into a `Vec` and record the per-push time; plot it. Find the reallocation spikes, verify they double in cost and halve in frequency, then re-run with `with_capacity` and watch them vanish. Report the p99 push time both ways.
- Write the accidental quadratic on purpose: deduplicate a 100k-element `Vec<String>` using `Vec::contains`, time it, then with a `HashSet`. Predict the ratio from the asymptotics *before* running, then explain the gap between your prediction and the measurement (it will be off — constants).

## Open Questions

- What is the actual crossover n between `Vec` linear scan and `HashSet` on this machine, for `u32` keys vs `String` keys? Both numbers, measured, written down.
- `BTreeMap`'s node fanout in std is tuned to ~11 (B=6) rather than to a full cache line. Read the source and find out why that beat larger fanouts in their benchmarks.
- Is there a practical Rust harness that *fits* a complexity curve automatically from a doubling run (log-log regression), rather than eyeballing ratios?
- The potential-method analysis of splay trees — work through it once properly rather than accepting the result.
- How much does the SipHash → FxHash swap actually buy on a realistic internal map (u32 keys, 100k entries) on Apple Silicon? Measure rather than repeat the folklore 2–3×.

## References

- Cormen, Leiserson, Rivest, Stein, *Introduction to Algorithms* — ch. 3 (asymptotics), ch. 4 (recurrences and the Master Theorem), ch. 17 (amortized analysis: aggregate, accounting, potential, worked on dynamic tables). The amortization chapter is the one worth reading twice.
- Sedgewick & Wayne, *Algorithms* — the doubling-experiment methodology and the empirical-vs-theoretical framing this doc's measurement half is built on.
- Jon Bentley, *Programming Pearls*, ch. 8 — the classic worked example of algorithmic improvement dwarfing micro-optimization, and the counterpoint that constants still decide close races.
- Aggarwal & Vitter, "The Input/Output Complexity of Sorting and Related Problems" (1988) — the origin of the I/O model; why the block transfer, not the operation, is the unit that matters off-chip.
- [Rust std collections documentation](https://doc.rust-lang.org/std/collections/) — the module docs carry the official complexity table and the guidance on which container to pick.
- Rust `sort_unstable` / pdqsort source — read the insertion-sort threshold and pivot selection as a case study in where theory hands off to tuning.
- Related topics in this repo: [Rust for Data Structures](../rust-for-data-structures/learning.md) (the other half of Stage 0), [Profiling & Measurement](../../performance-optimization/profiling-and-measurement/learning.md) (how to measure honestly — this doc's measurement half assumes it), [Cache Locality](../../performance-optimization/cache-locality/learning.md) (why the RAM model lies). The I/O model gets taken seriously in the Stage 9 topic *Cache-Aware & Cache-Oblivious Structures*, not yet written.
