# NUMA Awareness — Learning Notes

> Scope note: NUMA is a **server-class concern**. Your M-series Mac presents effectively uniform memory (unified memory; even Ultra's fused dies are engineered to near-uniformity) — this doc is for the Linux multi-socket/chiplet targets you deploy to, and its experiments need such a machine (or a cloud instance that honestly exposes topology).

## The Hardware Mechanism

On a multi-socket server (and increasingly *within* one package), "RAM" is not one place. Each socket — or each chiplet cluster (AMD's CCDs, Intel's sub-NUMA clusters) — has its **own memory controller and its own DRAM**; the sockets are joined by an interconnect (UPI, Infinity Fabric). Every core can reach all memory, but not at one price — **Non-Uniform Memory Access**:

- **Local access** (your node's DRAM): ~80–100 ns — the number the [cache-locality doc](../cache-locality/learning.md) called "DRAM."
- **Remote access** (across the interconnect): ~130–200 ns — **1.5–2× the latency**, and *bandwidth* is worse than latency: the interconnect carries a fraction of a node's local bandwidth, and remote reads also *contend* with the remote node's own traffic.
- **The contention multiplier:** when every core in the machine hammers *one* node's memory, that node's controller saturates while the others idle — the machine behaves like a fraction of itself. This, not the 1.5× latency, is what usually shows up in production.
- **Coherence gets slower too:** [false sharing's](../false-sharing/learning.md) line ping-pong is priced per topology — cross-*node* transfers cost multiples of cross-core-same-node; every contended atomic and lock is worse across the fabric.

The mechanism that decides where your data lives — and the source of the classic bug — is the OS's **first-touch policy**: a page is physically allocated on the node of the thread that first *writes* it. Not where you `malloc`ed (allocation reserves virtual pages; the physical placement waits for the first store), not where you'll use it — **where you initialized it**. A main thread that zeroes a 100 GB buffer has just placed all 100 GB on its own node, whatever happens next. Threads, meanwhile, are free to *migrate* between nodes unless pinned — so even correct placement decays if the scheduler wanders.

## Mental Model

**A NUMA machine is a small distributed system that happens to share an address space.** Every design rule in this doc is one you already know from the architecture side, shrunk to nanoseconds: place compute next to its data, partition by owner, replicate what's read-mostly, and don't let one node become the hot shard. [Sharding](../../architecture-patterns/sharding/learning.md) and [replication](../../architecture-patterns/replication-and-consistency/learning.md) logic, inside one chassis.

1. **Memory has a home; cost = distance + the home's congestion.** The two failure modes are *remote-heavy access* (paying interconnect latency per touch) and *single-node saturation* (everyone's data homed on node 0 — the default outcome of naive init). The second is worse and more common.
2. **Placement follows initialization (first-touch), so initialization is a design act.** The idiom: **initialize in parallel, with the same partitioning as the compute** — if worker i will process chunk i, worker i must also be the one that first writes chunk i. In rayon terms: an `par_chunks_mut`-shaped init pass places pages correctly *for free*, because the same threads that will sweep the data fault its pages in. Sequential init followed by parallel compute is the canonical self-sabotage.
3. **Affinity makes placement durable.** First-touch placement is only as good as thread residency: pin workers to nodes (`core_affinity`, rayon's `start_handler`, or `numactl --cpunodebind`) so the thread that owns chunk i stays where chunk i lives. Unpinned + migrated = silently remote.
4. **The policy menu, for when partitioning isn't possible:** *bind* (`numactl --membind`) forces memory to named nodes; *interleave* (`--interleave=all`) round-robins pages across nodes — sacrificing the local-access best case to guarantee no node saturates (the right call for large, irregularly-accessed shared structures: big hash tables, graph adjacency — where per-thread partitioning has no meaning); *replicate* read-mostly data per node (one copy each — memory spent to make every read local; the per-node config/lookup-table pattern).
5. **The shared-nothing endgame:** run **one shard per node** — a process or pool per NUMA node, each owning its slice of data and traffic, communicating by [message passing](../parallelism-and-work-stealing/learning.md)/network like the distributed system it secretly is (thread-per-core runtimes take this to per-core granularity — [async doc's](../async-and-io/learning.md) glommio note). This deletes remote access by construction and is how NUMA-serious databases and proxies are actually built.

Where the model stops: on single-node machines (most consumer hardware, small instances) *none of this applies* — `numactl` cargo-culted onto a one-node box is pure ritual. And below memory-bound intensity, NUMA effects hide: a compute-bound workload ([high IPC, low miss rate](../profiling-and-measurement/learning.md)) barely notices remote memory. Topology check first, profile second, policy third.

## Worked Example

A 2-socket Linux box (2 × 24 cores, ~50 GB/s local bandwidth per node), summing a 64 GB array with 48 threads. Illustrative numbers in realistic proportions.

**v0 — the naive version.**

```rust
let mut data = vec![0f64; N];              // virtual reservation only
init_sequential(&mut data);                 // main thread writes every page → ALL pages on node 0
let sum: f64 = data.par_iter().sum();       // 48 threads, half on node 1
```

```
observed: ~52 GB/s aggregate — barely above one node's ceiling
  node 0 controller: saturated (serving 48 threads)
  node 1 controller: idle
  node 1's 24 threads: 100% remote access, through a congested interconnect
numastat -p <pid>:  numa_foreign / other_node counts climbing — the smoking gun
```

Adding a second socket bought ~8%. This is the shape teams call "NUMA doesn't matter, we measured" — the measurement was of their init pattern, not their machine.

**v1 — first-touch done right.** One change: initialize with the same partitioning as the sweep:

```rust
data.par_chunks_mut(CHUNK).for_each(|c| c.fill(0.0));   // each worker faults its own pages
// + rayon pool pinned: workers 0-23 → node 0 cores, 24-47 → node 1 (start_handler + core_affinity)
let sum: f64 = data.par_iter().sum();                    // same chunking → same threads → local reads
```

```
observed: ~96 GB/s aggregate — both controllers busy, interconnect quiet
numastat: local_node ≈ everything
```

~1.85× from *moving zero bytes at compute time* — the pages were simply born in the right place. The delta is the whole topic.

**v2 — when partitioning has no meaning.** Same box, but the workload is random probes into one shared 40 GB hash table (every thread touches everything):

```
default (table faulted-in by loader thread): node 0 saturates       → ~0.9× of one node
numactl --interleave=all ./server:          pages round-robin       → ~1.7× (both controllers share load)
```

Interleave didn't make access *local* — it made congestion *even*. That's the honest ceiling for unpartitionable data, and the reason the next escalation is architectural (shard the table per node and route requests — v1's logic applied at the service layer).

## Applying It

- **See the topology first:** `lscpu` (NUMA node lines), `numactl --hardware` (node sizes + distance matrix), `hwloc`/`lstopo` (the full picture, including chiplet sub-nodes). On cloud VMs *verify* — vNUMA may present one fake node on a two-socket host, or honest topology; placement work on a lying topology is wasted.
- **Measure before engineering:** `numastat -p <pid>` (local vs. foreign access counts — the v0-vs-v1 diagnostic), `perf c2c` for cross-node line ping-pong, Intel MLC / STREAM run per-node (`numactl --membind=X --cpunodebind=Y`) to get *your* local/remote latency and bandwidth matrix — the numbers this doc quotes generically.
- **The cheap wins, in order:** (1) parallel first-touch init with compute-matched partitioning (often free with rayon — audit any sequential init/load phase before a parallel compute phase); (2) pin the pool (`core_affinity` in `ThreadPoolBuilder::start_handler`, or wrap the process in `numactl --cpunodebind --membind`); (3) `--interleave=all` for big shared irregular structures; (4) per-node replication of read-mostly tables; (5) shard-per-node architecture.
- **Allocator interaction:** per-thread caches (jemalloc/mimalloc) keep *allocator* metadata and recycled blocks node-local as a side effect of thread affinity — one more reason [the allocation doc's](../allocation-strategies/learning.md) arena-per-worker pattern ages well on NUMA; a shared global pool of buffers recycled across nodes quietly shuffles pages' *users* away from their homes.
- **Rust reality:** no std NUMA API. The toolkit is `numactl` at process level (start there — it's zero code), `core_affinity`/`hwloc` crates for explicit pinning, `libnuma` bindings (`numa`-family crates) for `mbind`/`numa_alloc_onnode` when page-level control is truly needed. Most services never need more than `numactl` + correct init order.
- **Huge pages compose:** 2 MB pages cut [TLB pressure](../cache-locality/learning.md) *and* make first-touch coarser (one touch places 2 MB) — mostly aligned with chunked init; just ensure chunk size ≥ huge-page size so partitioned init doesn't interleave accidentally.

## When It Hurts

- **On machines without NUMA, it's ritual.** One-node laptops, most small cloud instances, your Mac: `numactl` flags and pinning ceremonies change nothing but complexity. Check `numactl --hardware` before believing any NUMA story — including a vendor's.
- **Pinning fights the scheduler, and sometimes the scheduler is right.** Hard-pinned threads can't be rebalanced around noisy neighbors, interrupts, or uneven work ([the imbalance problem](../parallelism-and-work-stealing/learning.md) reintroduced by hand); a pinned-but-idle node is capacity you promised away. Pin *pools to nodes* (coarse), not threads to cores (fine), unless measurement demands finer.
- **Interleave is a floor-raiser, not a fix:** it converts best-case-local into guaranteed-average — right for unpartitionable data, wrong as a default (it *forfeits* the v1 win where partitioning was possible).
- **Replication spends RAM and buys incoherence risk:** per-node copies of "read-mostly" data need an update story ([the replication doc's](../../architecture-patterns/replication-and-consistency/learning.md) whole problem, at process scale) — stale per-node caches inside one box are a genuinely confusing bug class.
- **Compute-bound workloads don't care:** high-IPC code touching cache-resident data sees NUMA only in the noise. The [funnel](../profiling-and-measurement/learning.md) ordering holds — memory-bound diagnosis first ([roofline](../cache-locality/learning.md)), NUMA placement second.
- **Cloud abstractions decay silently:** an instance resize, a migration, or a hypervisor update can change vNUMA presentation; hard-tuned placement should be re-verified by a boot-time topology check (log `numactl --hardware` at startup) rather than trusted forever.

## Benchmarking Methodology

- **Establish the machine's matrix first:** per-node STREAM/MLC under `numactl --membind=X --cpunodebind=Y` for all (X, Y) — local and remote bandwidth/latency, measured once, kept with the [staircase plot](../cache-locality/learning.md) as the machine's calibration card.
- **The v0/v1 A/B is the canonical experiment:** same compute, init pattern flipped, `numastat` deltas + aggregate GB/s. It isolates placement from all other effects and its delta *is* your workload's NUMA exposure.
- **Watch per-node counters, not just totals:** aggregate bandwidth can look fine while node 0 saturates and node 1 idles — `numastat`, `pcm-numa`/`pcm-memory` (per-controller bandwidth) tell the distribution story.
- **Scaling curves per policy:** threads × {default, first-touch+pinned, interleave} — three curves on one plot; where they diverge is where topology starts taxing ([the scaling-curve instrument's](../false-sharing/learning.md) fourth appearance).
- **Beware placement luck in short benchmarks:** a benchmark whose working set fits in cache never exercises DRAM homing; size past LLC ([as ever](../cache-locality/learning.md)) and run long enough for migration effects to appear (or pin explicitly and say so).

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. First-touch: what exactly places a page, why does `vec![0u8; N]` on the main thread place everything on one node, and what's the idiomatic rayon fix?
2. Reconstruct v0's numbers: why did the second socket buy only ~8%, and which *two* mechanisms (not just latency) were at work?
3. Interleave vs. first-touch partitioning: what does each optimize, which workload shape demands each, and why is interleave a "floor-raiser"?
4. Map four architecture-doc concepts onto their NUMA-scale twins (sharding, replication, hot shard, place-compute-near-data).
5. Why does cross-node false sharing hurt more than same-node, and what does that imply for the padded-counter design on a 2-socket box?
6. Your service runs on cloud VMs. List the three topology-related verifications before and during deployment.
7. When is NUMA work provably irrelevant? Give the two disqualifiers and the command/counter that checks each.

Measurement exercises (need a multi-node Linux box or honest cloud instance):

- Run the machine matrix: MLC or STREAM per (cpu-node, mem-node) pair; tabulate local/remote latency and bandwidth. This is the calibration card for everything else.
- Reproduce v0/v1: 2×-LLC-sized array, sequential-init vs. parallel-init (+ pinned pool), `numastat -p` before/after, aggregate GB/s. Target: reproduce the ~2× shape and the numa_foreign collapse.
- Take the false-sharing doc's padded-counter benchmark and run it pinned same-node vs. cross-node — measure the topology multiplier on the ping-pong cost (open question from that doc, answered here).

## Open Questions

- Apple Silicon Ultra internals: measurable memory-latency variance across the fused dies under contention? (Engineered near-uniform — verify with a latency sweep pinned per cluster; likely a non-issue, worth one measurement.)
- AMD chiplet reality on the deploy target: does the CCD/CCX sub-topology surface as NUMA nodes (`NPS` BIOS settings), and does per-CCX pinning measurably beat per-socket on our workloads?
- `mbind`/`move_pages` from Rust: which of the `numa` crates is currently maintained and sound; what does page migration cost when correcting placement live?
- vNUMA on the actual cloud provider in use: what does `numactl --hardware` report per instance family, and is it stable across host migrations?
- Thread-per-core frameworks (glommio/monoio) on NUMA boxes: does per-core sharding automatically deliver the shard-per-node win, and what breaks when a shard's data outgrows its node?

## References

- Ulrich Drepper, *What Every Programmer Should Know About Memory*, §5 (NUMA) — first-touch, policies, and the programming implications; the mechanism from the source, still current.
- Christoph Lameter, ["NUMA (Non-Uniform Memory Access): An Overview"](https://queue.acm.org/detail.cfm?id=2513149) (ACM Queue) — the kernel-side view: policies, reclaim, migration; the best single survey.
- `numactl(8)` and `numastat(8)` man pages + [Intel Memory Latency Checker (MLC)](https://www.intel.com/content/www/us/en/developer/articles/tool/intelr-memory-latency-checker.html) — the operational toolkit.
- Frank Denneman's NUMA deep-dive series (frankdenneman.nl) — topology, vNUMA, and scheduling, written for virtualized environments; the cloud-reality supplement.
- Related topics in this repo: [Cache Locality](../cache-locality/learning.md) (DRAM was never one place; the calibration-card habit), [False Sharing](../false-sharing/learning.md) (coherence priced per topology), [Parallelism](../parallelism-and-work-stealing/learning.md) (pinning vs. stealing; the imbalance trade), [Allocation Strategies](../allocation-strategies/learning.md) (arena-per-worker ages well here), [Sharding](../../architecture-patterns/sharding/learning.md) + [Replication & Consistency](../../architecture-patterns/replication-and-consistency/learning.md) (the same design space, one box down).
