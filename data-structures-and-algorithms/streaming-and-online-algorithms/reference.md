# Streaming & Online Algorithms — Quick Reference

## At a Glance

**Two different constraints, often confused:**

| | Constraint | Question |
| --- | --- | --- |
| **Streaming** | data too large to store; one pass, sublinear space | *What can I compute in o(n) space?* |
| **Online** | must **commit before seeing the rest** | *How much worse than a clairvoyant?* |

**Competitive ratio:** c-competitive if `cost(online) ≤ c · cost(OPT-offline) + const`, where OPT knows the future.

**Ask two questions:** Can I store everything? (No ⇒ streaming.) Must I decide now? (Yes ⇒ online.)

## The Number

LRU vs Bélády's optimal, Zipf-like trace, 200k references (measured):

| k | LRU misses | OPT misses | **LRU/OPT** | Theory |
| --- | --- | --- | --- | --- |
| 16 | 83.5% | 53.9% | **1.55×** | 16× |
| 64 | 43.5% | 22.4% | **1.94×** | 64× |
| 256 | 18.3% | 9.0% | **2.03×** | **256×** |

> **The competitive ratio overstates the real gap by ~126×.** The bound is tight — against an adversary who requests exactly what you evicted. It describes an adversary, not a workload.

## Streaming Toolkit

| Problem | Structure | Space |
| --- | --- | --- |
| **Distinct count** | HyperLogLog | **Θ(1)** — 16 KB, any n |
| Membership | Bloom / cuckoo | Θ(n·bits) |
| **Heavy hitters** | Count-Min, Misra-Gries | Θ(1/ε) |
| **Quantiles** | t-digest, DDSketch, GK | Θ(1/ε) |
| **Uniform sample** | Reservoir | **Θ(k)** |
| Sliding-window aggregate | Exponential histogram | Θ((1/ε) log n) |

Streaming is **inherently approximate** — exact distinct-count provably needs Θ(n) space.

## Online Classics

| Problem | Algorithm | Ratio |
| --- | --- | --- |
| **Paging** | LRU, FIFO | **k** (tight) |
| Paging, randomized | Marking | **2·H_k ≈ 2 ln k** |
| **Ski rental** | Buy when rent = purchase | **2** (1.58 randomized) |
| Load balancing | Greedy least-loaded | 2 − 1/m |
| Secretary | Observe n/e, take next better | 1/e success |
| List update | Move-to-front | 2 |

**Ski rental generalizes:** disk spin-down, VM migration, reserved vs on-demand instances, spin-then-sleep locks.

## Cache Policies

| Policy | Weakness |
| --- | --- |
| **LRU** | **Not scan-resistant** — one scan evicts everything |
| LFU | Slow to adapt; stale hot items |
| ARC / 2Q | More state, tuning |
| **W-TinyLFU** (`moka`) | The modern default — Count-Min **admission** filter |
| **Bélády (OPT)** | Unimplementable — use as a **benchmark** |

W-TinyLFU is a **streaming sketch inside an online algorithm** — the two halves of this topic meeting.

## Choose This When

| Use | For |
| --- | --- |
| **Reservoir sampling** | Uniform sample, unbounded stream, Θ(k) |
| **HyperLogLog** | Distinct count |
| **Count-Min / Misra-Gries** | Heavy hitters |
| **t-digest / DDSketch** | Quantiles, **mergeable** |
| **LRU** | Default cache — ~2× OPT and simple |
| **W-TinyLFU** | Hit rate matters; scan-resistant |
| **Bélády** | Benchmark on replayed traces only |
| Ski-rental reasoning | Any "keep paying or commit?" decision |
| Exact offline | It fits and you can make two passes |

## Rules of Thumb

- **Benchmark against Bélády on your own replayed trace** — that's the real headroom.
- Competitive ratios describe adversaries; measure typical behaviour.
- **Ship the sketch, not the estimate** — merge, then query.
- Streaming ≠ online. Space vs future-knowledge.
- If the data is stored and re-readable, you're **not** in the streaming model.
- Resource augmentation (2k cache vs OPT's k) gives far more realistic bounds.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Ratio taken as a prediction | Complex policy built for a 2× ceiling |
| Streaming/online confused | Wrong toolbox entirely |
| Averaging per-host estimates | Inflated distinct counts; meaningless "p99" |
| LRU + a big scan | Hit rate collapses; working set evicted |
| Sketch where two passes were available | Approximate number on an invoice |
| Mismatched sketch parameters | Merge silently corrupts |

## Key References

- Bélády (1966) — the optimal (unimplementable) policy
- Sleator & Tarjan (1985) — competitive analysis, amortized efficiency
- Muthukrishnan, *Data Streams: Algorithms and Applications* (free)
- Einziger et al. (2017) — TinyLFU
