# LSM Trees & Write-Optimized Structures — Quick Reference

## At a Glance

**Makes writes fast by refusing to write in place.** Appends to an in-memory buffer, flushes immutable sorted files, merges them in the background. Converts **random writes into sequential** ones.

**Invariant:** every level is a set of sorted, immutable runs; **newer data shadows older**; deletes are **tombstones**, not removals.

**RUM conjecture — optimize at most two of three:**
**R**ead amplification · **U**pdate (write) amplification · **M**emory (space) amplification.
B-tree picks R and M. LSM picks U. No configuration wins all three.

## LSM vs B-tree

| | B-tree | LSM (leveled) |
| --- | --- | --- |
| Write path | read-modify-write, **random** | append WAL + memtable, **sequential** |
| Write amplification | ~1× | **10–30×** |
| Read | Θ(log_B n), one file | levels, Bloom-filtered |
| Space amplification | ~1.5× | 1.1× leveled / 2×+ tiered |
| **Range scans** | **excellent** (linked leaves) | merge across runs; **Blooms don't help** |

LSMs write **more total bytes** than a B-tree — the win is that they're sequential and off the critical path. *Amplification and latency are different axes.*

## Compaction Strategies

| Strategy | Write amp | Read amp | Space amp | Use |
| --- | --- | --- | --- | --- |
| **Leveled** | high (10–30×) | **low** | **low (1.1×)** | read-heavy (RocksDB default) |
| **Tiered** | **low (~4×)** | high | high (2×+) | write-dominated (Cassandra) |
| FIFO | ~1× | high | low | time-series + TTL |

## Read Path

```
memtable → immutable memtables → L0 (OVERLAPPING — check all) → L1..Ln (one file each)
  consult each file's Bloom filter first; skip on negative
  stop at first match — INCLUDING a tombstone
```

Bloom filters at 10 bits/key give **0.824% FP** (measured) — 99% of unnecessary file reads avoided. They are what make LSM reads viable.

## Complexity

| Operation | B-tree | LSM leveled | LSM tiered |
| --- | --- | --- | --- |
| Write | Θ(log_B n) random | **Θ(1) amortized sequential** | Θ(1) |
| Read | **Θ(log_B n)** | Θ(levels), ≈Θ(1) with Blooms | Θ(runs) |
| Range scan | **Θ(log_B n + k/B)** | merge across levels | worse |
| Delete | in place | tombstone until compaction | same |

"Θ(1) amortized" defers Θ(log n) work to compaction — deferred, not eliminated, and it competes for the same disk.

## Choose This When

| Use | For |
| --- | --- |
| **LSM leveled** | Write-heavy, reads still matter, low space amp |
| **LSM tiered** | Write-dominated; tolerate read/space amp |
| FIFO | Time-series with TTL |
| **B+ tree** | Read-heavy, range-scan-heavy, **predictable latency** |
| CoW B-tree (LMDB, `redb`) | Crash safety via root swap; read-heavy |
| Fractal / B^ε tree | LSM-ish writes **with** B-tree scans |
| **In-memory `BTreeMap`** | It fits in memory — none of this applies |

## Rules of Thumb

- Provision disk bandwidth for `write_rate × write_amplification`, not the user rate.
- Monitor **L0 file count** and **pending compaction bytes** — write latency is fine until the cliff.
- Deleting *increases* space and read cost until compaction — prefer dropping whole files (TTL partitions).
- Bloom filters help point lookups, **never range scans**.
- ~4.8 bits/key divides the Bloom FP rate by 10.
- Memtable is a skip list because skip lists are easy to make concurrent.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| Compaction falls behind | **Write stall** — a cliff, not a slope |
| Heavy per-key deletes | Reads slow over time; range scans crawl |
| Range-scan workload on an LSM | Far slower than a B-tree; Blooms don't help |
| Undersized Bloom filters | Negative lookups cost one I/O per level |
| Wrong newest-to-oldest order | Deleted data resurrects |
| LSM for an in-memory dataset | All the costs, none of the benefit |

## Key References

- O'Neil et al. (1996) — the original LSM tree
- Athanassoulis et al., "Designing Access Methods: The RUM Conjecture" (2016)
- RocksDB tuning guide — leveled vs universal compaction in practice
