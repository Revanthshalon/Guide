# Complexity Analysis — Quick Reference

## At a Glance

How cost *scales* with input size — deliberately blind to constants and to small inputs. Use it to eliminate disasters; measure to choose between survivors.

**Invariant:** `f = O(g)` iff ∃c>0, ∃n₀, ∀n≥n₀: `f(n) ≤ c·g(n)`. Every "Big-O lied" story is the constant `c` or the threshold `n₀`.

## Growth Rates

| Class | n=1,000 | n=10⁶ | Feasible n in 1s |
| --- | --- | --- | --- |
| O(1) | 1 | 1 | ∞ |
| O(log n) | 10 | 20 | ∞ |
| O(n) | 10³ | 10⁶ | 10⁹ |
| O(n log n) | 10⁴ | 2×10⁷ | 5×10⁷ |
| O(n²) | 10⁶ | 10¹² (~17 min) | 3×10⁴ |
| O(n³) | 10⁹ | — | 10³ |
| O(2ⁿ) | — | — | ~30 |
| O(n!) | — | — | ~11 |

**n² is the cliff that ships** — invisible in tests, fatal in prod.

## The Four Cost Measures

| Measure | Quantified over | Holds when |
| --- | --- | --- |
| Worst case | all inputs | always |
| Average case | a distribution of inputs | your input matches that distribution |
| Expected | the algorithm's coin flips | always, even adversarial input |
| Amortized | a sequence of operations | always (but not per-operation) |

- **Average ≠ expected** — randomize the *algorithm* to survive adversarial input.
- **Amortized ≠ average** — it's worst-case over a sequence; individual ops can still spike.

## Master Theorem

`T(n) = a·T(n/b) + f(n)`, compare f(n) to `n^(log_b a)`:

| f smaller | f equal | f larger |
| --- | --- | --- |
| Θ(n^(log_b a)) | Θ(n^(log_b a)·log n) | Θ(f(n)) |

| Recurrence | Result |
| --- | --- |
| T(n) = 2T(n/2) + n | Θ(n log n) — merge sort |
| T(n) = T(n/2) + 1 | Θ(log n) — binary search |
| T(n) = 3T(n/2) + n | Θ(n^1.585) — Karatsuba |
| T(n) = 7T(n/2) + n² | Θ(n^2.807) — Strassen |
| T(n) = T(n−1) + n | Θ(n²) — "removes one element" |

## std Complexity

| Type | Get | Insert | Remove |
| --- | --- | --- | --- |
| `Vec` | O(1) | O(1)* push / O(n) `insert` | O(1) `swap_remove` / O(n) `remove` |
| `VecDeque` | O(1) | O(1)* both ends | O(1) both ends |
| `HashMap` | O(1) exp. | O(1)* exp. | O(1) exp. |
| `BTreeMap` | O(log n) | O(log n) | O(log n) |
| `BinaryHeap` | O(1) peek | O(log n) | O(log n) pop |

`*` amortized · `BinaryHeap::from(vec)` heapifies in **O(n)**

## Linear Ops That Look Constant

Treat any of these inside a loop as a defect:

| Trap | Fix |
| --- | --- |
| `vec.contains(x)` | `HashSet` / `BTreeSet` (above n ≈ 12, measured `u32`) |
| `vec.remove(0)` | `VecDeque::pop_front` |
| `vec.remove(i)` | `swap_remove` if order is free |
| `vec.insert(0, x)` | `VecDeque::push_front` |
| `s = s + &part` | `String::with_capacity` + `push_str` |
| repeated `remove` in loop | `retain` |
| `.iter().position()/.find()` | index map built once |

## Doubling Experiment

Measure at n, 2n, 4n; `ratio = T(2n)/T(n)`:

| Ratio | Complexity |
| --- | --- |
| ~1.0 | O(1) |
| ~1.1 | O(log n) |
| ~2.0 | O(n) |
| ~2.1 | O(n log n) |
| ~4.0 | O(n²) |
| ~8.0 | O(n³) |

Start above cache-resident sizes; `black_box` the result.

## Rules of Thumb

- Say **Θ** when you mean tight; O is only an upper bound.
- Name **every** variable: `HashMap<String,V>` lookup is O(1) in entries, O(k) in key length.
- Amortized O(1) ≠ good p99 — `reserve()` when size is boundable.
- Recursion stack counts as space: Rust main thread 8 MB ≈ 100–150k frames.
- Default `HashMap` (SipHash + random seed) for attacker-reachable keys; `FxHashMap`/`aHash` only for self-generated keys.
- Demand a **crossover point**, not an exponent, before accepting an "asymptotically better" rewrite.
- Same class or within a log factor → stop analyzing, go measure.

## Numbers to Remember

| Thing | Number |
| --- | --- |
| L1 access | ~1 ns |
| Random DRAM access | ~100 ns |
| Sequential scan | ~10 GB/s |
| Simple ops/sec | ~10⁹ |
| SipHash cost | ~1 ns/byte |
| `sort_unstable` insertion-sort cutoff | ~20 elements |
| B-tree fanout 100, n=10⁹ | ~4.5 block reads vs ~30 for a BST |

## Key References

- CLRS ch. 3, 4, 17 (asymptotics, recurrences, amortization)
- [Rust std collections docs](https://doc.rust-lang.org/std/collections/) — official complexity table
- Aggarwal & Vitter (1988) — the I/O model
