# LSM Trees & Write-Optimized Structures — Learning Notes

## Mental Model

**An LSM tree makes writes fast by refusing to write in place.** Every update is appended to an in-memory buffer and later flushed as a new immutable sorted file; nothing is ever modified where it sits. Reads then have to consult several files, and a background **compaction** process merges them to keep that number bounded.

Compare with a [B-tree](../b-trees/learning.md), which updates in place: one random write per update, plus the page read to modify it. On spinning disks a random write was ~100× a sequential one; on SSDs the gap is smaller but writing a 4 KB page to change 20 bytes still costs a full erase-block cycle eventually. **The LSM converts random writes into sequential ones**, which is the entire performance argument.

The framing that makes the whole design space legible is the **RUM conjecture**: you can optimize at most two of

- **R**ead amplification — how many blocks you read to answer one lookup
- **U**pdate amplification — how many bytes you write per byte of user data
- **M**emory (space) amplification — how much storage you use per byte of live data

A B-tree picks low read and low space amplification, paying in write amplification. An LSM picks low write amplification, paying in read and (depending on compaction strategy) space. **There is no configuration that wins all three**, so "which two?" is the design question, and it's answered by the workload rather than by the structure.

The read cost is where [probabilistic data structures](../probabilistic-data-structures/learning.md) earn their place: each SSTable carries a Bloom filter, so a lookup skips files that definitely don't contain the key. Measured there, ~10 bits per key gives a **0.824% false-positive rate**, meaning 99% of unnecessary file reads are avoided for 1.2 MB per million keys. That single structure is what makes LSM reads viable.

## The Invariant

> **Every level is a set of sorted, immutable runs. Newer data shadows older data.** A key's current value is the one found in the newest run containing it; deletions are recorded as **tombstones**, not removals.

Three consequences that define the whole engineering:

- **Reads must check newest-to-oldest and stop at the first hit** — including a tombstone, which means "deleted" rather than "keep looking". Getting that order wrong resurrects deleted data.
- **Nothing is ever updated in place**, so writes are sequential appends and the structure is naturally crash-friendly: a partially written file is discarded, and the write-ahead log replays the memtable.
- **Garbage accumulates.** Old versions and tombstones remain until compaction removes them, so space amplification is a function of how aggressively you compact — which trades directly against write amplification.

The **memtable** invariant: it's a sorted in-memory structure (a skip list in RocksDB and LevelDB, precisely because [skip lists](../binary-search-trees/learning.md) are easy to make concurrent) plus a write-ahead log for durability. On flush it becomes an immutable SSTable.

## Mechanics

### The write path

```
write → append to WAL (durability) → insert into memtable (sorted, in memory)
      → memtable full → freeze it, flush as an immutable SSTable at level 0
      → background compaction merges runs, drops shadowed values and tombstones
```

Each stage is sequential I/O. The WAL append is the only synchronous disk operation on the critical path, and it's an append.

### The read path

```
memtable → immutable memtables → L0 files (may overlap, check all) → L1..Ln (one file per level)
   at each candidate file: consult its Bloom filter first; skip on a negative
   stop at the first match, including a tombstone
```

L0 is special: files there come straight from memtable flushes and can have **overlapping key ranges**, so all of them must be checked. Deeper levels are non-overlapping, so a binary search picks exactly one file.

### Compaction strategies — the RUM trade made concrete

| Strategy | Write amp | Read amp | Space amp | Use |
| --- | --- | --- | --- | --- |
| **Leveled** | **high** (10–30×) | **low** (~1 file/level) | **low** (~1.1×) | Read-heavy; the LevelDB/RocksDB default |
| **Tiered (size-tiered)** | **low** (~4×) | high (many runs/level) | high (~2×+) | Write-heavy; Cassandra's default |
| Hybrid / leveled-N | tunable | tunable | tunable | RocksDB's universal compaction |
| **FIFO** | ~1× | high | low | Time-series with TTL — just drop old files |

Leveled compaction keeps each level 10× the previous and non-overlapping, so a lookup checks ~one file per level — but merging a file into the next level rewrites ~10× its size, which is where the write amplification comes from. Tiered compaction waits until several similar-sized runs accumulate and merges them together, writing far less but leaving more runs to search.

**This is the RUM conjecture as a configuration knob**, and it's why the same engine ships both.

### Why writes are cheap and reads are not

| | B-tree | LSM (leveled) |
| --- | --- | --- |
| Write path | read page, modify, write page — **random** | append to WAL + memtable — **sequential** |
| Write amplification | ~1× (plus page granularity) | 10–30× (compaction rewrites) |
| Read path | Θ(log_B n) transfers, one file | check L0 + one file per level, Bloom-filtered |
| Space amplification | ~1.5× (50% node occupancy) | 1.1× leveled, 2×+ tiered |
| Range scans | **excellent** (linked leaves) | merge across runs |

Note the surprise in the write-amplification row: LSMs are *write-optimized* yet write **more total bytes** than a B-tree. The win is that those bytes are written **sequentially and in the background**, off the critical path, where a B-tree's are random and synchronous. Amplification and latency are different axes.

### Related write-optimized structures

- **Fractal tree / B^ε tree** — buffers pending updates inside B-tree nodes and pushes them down lazily. Gets much of the LSM's write advantage while keeping the B-tree's range-scan quality (TokuDB, and the basis of several newer engines).
- **Bitcask / log-structured hash** — append-only log plus an in-memory hash index. Θ(1) reads and writes, but the index must fit in RAM and there are no range scans.
- **Copy-on-write B-tree** — never overwrites a page; the root swap is an atomic commit (LMDB, Btrfs). Excellent read and crash behaviour, high write amplification.

## Complexity

| Operation | B-tree | LSM (leveled) | LSM (tiered) |
| --- | --- | --- | --- |
| Point write | Θ(log_B n) random I/O | **Θ(1) amortized sequential** | **Θ(1) amortized sequential** |
| Point read | **Θ(log_B n)** | Θ(levels) with Bloom ≈ **Θ(1) effective** | Θ(runs), worse |
| Range scan | **Θ(log_B n + k/B)** | Θ(levels · log + k/B) — merge | worse |
| Write amplification | ~1× | **10–30×** | ~4× |
| Space amplification | ~1.5× | ~1.1× | ~2×+ |
| Delete | in place | **tombstone** — space reclaimed only on compaction | same |

**Where the table misleads.** "Θ(1) amortized sequential" for writes hides that compaction is doing Θ(log n) work per key in the background — the amortization is real but the work is only *deferred*, and it competes for the same disk and CPU. Under sustained write pressure compaction can fall behind, at which point read amplification climbs (more un-merged runs) and the system can hit a **write stall** — a cliff, not a gradual degradation, and the most common LSM operational failure.

Also, the "Θ(1) effective" read depends entirely on the Bloom filters being sized correctly. A saturated or undersized filter ([probabilistic data structures](../probabilistic-data-structures/learning.md)) silently stops filtering, and reads degrade to checking every level.

## Use Cases

- **Write-heavy key-value stores** — RocksDB, LevelDB, Cassandra, ScyllaDB, HBase. Anything ingesting events faster than it queries them.
- **Time-series and metrics** — append-dominated with TTL; FIFO compaction is nearly free because old files are dropped rather than merged.
- **Embedded storage in Rust** — `sled` (LSM-ish, lock-free), `rocksdb` bindings, `fjall`. `redb` is the copy-on-write B-tree alternative.
- **Stream processing state stores** — Kafka Streams and Flink keep local state in RocksDB precisely because state updates are write-heavy.
- **Blockchain / ledger storage** — append-only by nature, and the immutability matches.
- **Search index building** — Lucene segments are the same idea: immutable sorted runs merged in the background, with per-segment structures for skipping.

**Where a B-tree is the better answer:** read-dominated workloads, workloads dominated by range scans, and anything where predictable latency matters more than throughput (an LSM's compaction causes latency variance a B-tree doesn't have).

## When to Use Which

| Reach for | When |
| --- | --- |
| **LSM, leveled** | Write-heavy but reads matter; want low space amplification |
| **LSM, tiered** | Write-dominated; can tolerate read and space amplification |
| LSM, FIFO | Time-series with TTL — old data is dropped, not merged |
| **B+ tree** | Read-heavy, range-scan-heavy, or latency must be predictable |
| Copy-on-write B-tree | Want crash safety from the root swap; read-heavy (LMDB) |
| Fractal / B^ε tree | Want LSM-ish writes with B-tree range scans |
| Log + in-memory hash | Everything fits in RAM's index; no range scans needed |
| **In-memory `BTreeMap`** | It fits in memory — none of this applies |

## Pitfalls in Depth

### Pitfall: Compaction falling behind, and the write stall

- **What goes wrong:** Sustained write throughput exceeds what compaction can merge. L0 accumulates files, read amplification climbs (every read checks more overlapping runs), and eventually the engine **stalls writes** to let compaction catch up. Throughput doesn't degrade smoothly — it goes from fine to near-zero at a threshold, usually under exactly the load spike you provisioned for.
- **Why it happens (the mechanism):** Compaction is background work competing for the same disk bandwidth and CPU as the foreground writes that create it, and leveled compaction's 10–30× write amplification means each user byte generates that many bytes of background I/O. The system is stable only while `write_rate × amplification < available_bandwidth`, and that inequality is invisible until it's violated.
- **How to handle it in production, and why that works:** Monitor the two signals that lead the stall — **L0 file count** and **pending compaction bytes** — rather than only watching write latency, which is fine right up until it isn't. Provision disk bandwidth for `write_rate × write_amplification`, not for the user write rate. If the workload is genuinely write-dominated, switch to tiered compaction (~4× amplification instead of 10–30×) and accept the read/space cost. This is the same [backpressure](../../architecture-patterns/backpressure-and-rate-limiting/learning.md) shape: an unbounded internal queue with a cliff at the end.
- **Trade-offs of the fix:** Tiered compaction roughly doubles space amplification and meaningfully worsens read amplification. Over-provisioning bandwidth costs money. Rate-limiting compaction smooths its impact but makes falling behind *more* likely — the knob genuinely trades foreground latency against stall risk.

### Pitfall: Tombstones that never get collected

- **What goes wrong:** A workload deletes heavily — a queue-like table, an expiring cache — and reads get slower over time even though the live data set is small. Range scans in particular crawl, because they must read and discard thousands of tombstones. In Cassandra this is a well-known way to make a cluster unusable.
- **Why it happens (the mechanism):** A delete is a *write* of a tombstone marker, not a removal. The tombstone must be retained until compaction can prove no older value for that key exists in any lower level — which for leveled compaction may require the tombstone to migrate all the way down. Until then, every read over that key range pays for it. Deleting data therefore *increases* both space and read cost, temporarily.
- **How to handle it in production, and why that works:** Prefer designs that drop whole files over designs that delete keys: time-partitioned tables with TTL, so expiry becomes "drop the SSTable" (FIFO compaction) rather than "write a tombstone per key". Where per-key deletes are unavoidable, monitor tombstone ratios and tune compaction to prioritize files with high tombstone density (RocksDB's compaction priority does this).
- **Trade-offs of the fix:** Time-partitioning constrains the schema and complicates queries that span partitions. Aggressive compaction to clear tombstones is itself write amplification, so you're paying in the axis you were trying to protect.

### Pitfall: Range scans on an LSM

- **What goes wrong:** An analytics query or a paginated listing does a large range scan and is far slower than the equivalent on a B-tree. The scan must open an iterator on every level, merge their outputs, and skip shadowed values and tombstones — where a B+ tree walks a linked list of leaves sequentially.
- **Why it happens (the mechanism):** LSM data for a key range is *scattered across runs by age*, so a scan is a k-way merge ([heaps](../heaps-and-priority-queues/learning.md)) rather than a sequential read. The Bloom filters that rescue point lookups **do not help range scans at all** — a filter answers "is this exact key present", not "does this file overlap this range".
- **How to handle it in production, and why that works:** If the workload is range-scan-dominated, use a B+ tree engine (LMDB, `redb`) rather than an LSM. Within an LSM, leveled compaction is much better than tiered for scans (fewer runs to merge), and prefix Bloom filters help when scans share a key prefix. Reducing the number of levels via a larger memtable and larger L1 also directly reduces merge width.
- **Trade-offs of the fix:** Leveled compaction is precisely the higher-write-amplification choice, so improving scans worsens the write path. Switching engines is a large migration. Prefix filters only help specific query shapes.

### Pitfall: Undersized or missing Bloom filters

- **What goes wrong:** Point lookups for **absent** keys — the common case in a cache-miss path or a uniqueness check — read one file per level from disk. With 6 levels that's 6 I/Os to conclude "not found", where a correctly-sized Bloom filter would have answered in memory ~99% of the time.
- **Why it happens (the mechanism):** Bloom filters are the mechanism that makes LSM reads competitive, and their effectiveness depends on bits-per-key, which is a tunable that defaults conservatively. Measured in [probabilistic data structures](../probabilistic-data-structures/learning.md), 10 bits/key gives 0.824% false positives while 8 bits gives 2.15% — and a filter sized for fewer keys than it holds degrades silently toward 100% with no signal.
- **How to handle it in production, and why that works:** Set bits-per-key from the measured rule that **every ~4.8 bits divides the false-positive rate by 10**, and verify against your actual read pattern — negative lookups are the case that matters. Monitor the filter's useful-negative rate if the engine exposes it. Note that filters cost memory proportional to key count, so the tuning is a memory-versus-I/O trade with a clean formula.
- **Trade-offs of the fix:** More bits per key means more memory held per open SSTable, which competes with the block cache — and block cache hits are also a way to avoid I/O. At some point spending the memory on cache beats spending it on filters, and only measurement distinguishes them.

### Pitfall: Using an LSM when the data fits in memory

- **What goes wrong:** An embedded LSM is adopted for a dataset of a few hundred megabytes. It brings compaction threads, write amplification, latency variance from background merges, and tuning knobs — to solve a disk-I/O problem that doesn't exist, since the whole dataset would sit in a `BTreeMap` or a `HashMap`.
- **Why it happens (the mechanism):** LSMs are the default answer for "embedded key-value store", and the phrase describes the *interface* rather than the requirement. The entire structure exists to make disk writes sequential; with no disk in the critical path, every one of its costs remains and its benefit is zero.
- **How to handle it in production, and why that works:** Size the data first. Fits in memory and durability is optional → an in-memory map. Fits in memory but needs durability → an in-memory map plus a write-ahead log or periodic snapshot, which is far simpler than an LSM and has no compaction. Genuinely exceeds memory → then choose LSM versus B-tree on the read/write mix.
- **Trade-offs of the fix:** In-memory plus WAL has a recovery time proportional to log length, needing periodic snapshots. And a dataset that fits today may not next year — but adopting an LSM prematurely means paying its operational cost for years before the benefit arrives.
