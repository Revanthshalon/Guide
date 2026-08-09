# Concurrent Data Structures — Learning Notes

## Mental Model

**The problem is not that threads interleave — it's that a shared cache line can only be owned by one core at a time.** Every concurrent structure is an answer to "how do we let threads make progress without all of them fighting over the same line?"

That reframing matters because the intuitive fix — replace the lock with a lock-free algorithm — often doesn't help. A lock-free counter and a mutex-protected counter both funnel every thread through the *same cache line*, so both serialize at the coherence protocol. The win comes from **partitioning**, not from removing the lock.

Measured on this machine — a `HashMap` under a 95%-read workload, 400,000 operations per thread over a 100,000-key space:

| Threads | `Mutex<HashMap>` | `RwLock<HashMap>` | **64-way sharded** |
| --- | --- | --- | --- |
| 1 | 14.9 ms | 8.9 ms | 9.2 ms |
| 2 | 29.6 ms | 28.0 ms | **11.0 ms** |
| 4 | 95.2 ms | 65.5 ms | **18.6 ms** |
| 8 | 267.7 ms | 232.2 ms | **69.2 ms** |

Two findings, and the second is the one people don't expect:

1. **Sharding scales; a single lock doesn't.** At 8 threads the total work is 8× the single-threaded case; sharded time grew 7.5× (roughly linear — it *scales*), while the mutex grew 18× (each operation got 2.2× *more expensive* as threads were added).
2. **`RwLock` barely helps even at 95% reads** — 232 ms against the mutex's 268 ms. A reader must still *write* to the shared reader count, so the lock's cache line ping-pongs between cores exactly as a mutex's does. Read-write locks solve a logical-exclusion problem, not a coherence problem.

At one thread, sharding is slightly *worse* (9.2 ms vs 8.3 ms for a mutex in the 50/50 mix) — the extra indirection costs when there's no contention to relieve. **Concurrency structures are a response to measured contention, not a default.**

## The Invariant

**Linearizability** is the correctness condition worth knowing:

> Every operation appears to take effect **instantaneously at some point between its invocation and its return**, and that order is consistent with real time.

It's what makes a concurrent structure usable without reasoning about interleavings: if a `push` returns before a `pop` is called, the `pop` must see it. Weaker conditions exist (sequential consistency drops the real-time requirement; eventual consistency drops much more), and they buy performance at the cost of surprising behaviour.

**Progress guarantees**, from strongest to weakest:

| Guarantee | Means |
| --- | --- |
| **Wait-free** | *Every* thread completes in a bounded number of steps |
| **Lock-free** | *Some* thread makes progress; individual threads can starve |
| **Obstruction-free** | A thread makes progress if it runs alone |
| Blocking (locks) | A stalled thread can block everyone |

"Lock-free" does **not** mean "fast" — it means no thread's suspension can block the others. That property matters enormously in a kernel or a real-time system and often matters very little in an application, where a mutex whose holder is descheduled is merely slow.

**The atomic primitive** everything is built on:

> **Compare-and-swap:** atomically, "if this location still holds `expected`, store `new` and report success; otherwise report the current value." CAS is what lets you publish a change only if nobody else changed the thing underneath you.

That's the same optimistic-concurrency shape as `expected_version` in [event sourcing](../../architecture-patterns/event-sourcing/learning.md) — one mechanism at two scales.

## Mechanics

### Sharding — the technique that actually scaled

```rust
struct Sharded { shards: Vec<Mutex<HashMap<u64, u64>>> }

impl Sharded {
    #[inline]
    fn idx(k: u64) -> usize {
        (k.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 58) as usize % SHARDS   // hash the key
    }
    fn get(&self, k: u64) -> Option<u64> {
        self.shards[Self::idx(k)].lock().unwrap().get(&k).copied()
    }
}
```

Each shard has its own lock and its own cache lines, so threads touching different keys never contend. That's the whole idea, and it's the same partitioning insight as [sharding](../../architecture-patterns/sharding/learning.md) one scale up — and as [false sharing](../../performance-optimization/false-sharing/learning.md) one scale down.

**Note the hash**: sharding by `k % SHARDS` would collide badly with structured keys (sequential IDs, aligned pointers), concentrating traffic on a few shards. Hash first, then take the shard.

`dashmap` is this, productionized. It is the default answer for a concurrent map in Rust.

### The lock-free classics

| Structure | Mechanism | Note |
| --- | --- | --- |
| **Treiber stack** | CAS the head pointer | The simplest lock-free structure; ABA-prone |
| **Michael-Scott queue** | CAS head and tail separately | The canonical lock-free FIFO |
| **Work-stealing deque** | Owner uses one end, thieves the other | Contention only on steals — see [work stealing](../../performance-optimization/parallelism-and-work-stealing/learning.md) |
| Lock-free hash map | CAS per bucket / split-ordered lists | Complex; `dashmap`'s sharding is usually enough |
| **Skip list** | CAS per level | Easier to make concurrent than a tree — no rotations |

The skip-list entry explains a design choice that recurs: **concurrent ordered maps are skip lists, not balanced trees**, because a tree's rebalancing rotates nodes far from the insertion point, which is very hard to do lock-free. A skip list's structure is local, so [randomness replacing balance logic](../randomized-algorithms/learning.md) pays a second dividend here.

### The two hard problems

**The ABA problem.** A thread reads pointer `A`, is descheduled; other threads pop `A`, push `B`, and push `A` again. The original thread's CAS on `A` **succeeds** — the value matches — but the structure has changed underneath it, and the CAS corrupts it. Solutions: tagged pointers (pack a counter into unused address bits), double-width CAS, or hazard pointers/epochs.

**Memory reclamation.** In a lock-free structure, when is it safe to free a node another thread might still be reading? There's no lock to tell you nobody holds it. This is *the* reason lock-free code is hard — the algorithm is often 30 lines and the reclamation scheme is 300.

| Scheme | How | Cost |
| --- | --- | --- |
| **Epoch-based** (`crossbeam-epoch`) | Threads announce an epoch; free when all have advanced | Low overhead; deferred frees can accumulate |
| Hazard pointers | Threads publish what they're reading | Per-read store; bounded memory |
| RCU | Writers copy, readers never block, free after a grace period | Read-side nearly free; kernel favourite |
| **`Arc`** | Refcount | Simple, but every clone is a contended atomic RMW |

### Memory ordering

Rust exposes C++11 orderings, and getting them wrong produces bugs that appear only on weakly-ordered hardware (ARM, including Apple Silicon) and never on x86:

| Ordering | Use |
| --- | --- |
| `Relaxed` | Counters where only the final value matters |
| `Acquire` / `Release` | **The pair for publishing data**: release the store, acquire the load |
| `AcqRel` | Read-modify-write that both publishes and observes |
| `SeqCst` | Total order across all threads — the safe default, and the slowest |

The rule that avoids most trouble: **write `SeqCst` first, and weaken only with a measured reason and an argument for why it's still correct.** x86 gives acquire/release almost for free, so a bug in the ordering is invisible there and appears on ARM — which is precisely the machine this repo's measurements run on.

### Choosing

| Situation | Use |
| --- | --- |
| Low contention, simple | **`Mutex<T>`** — measured *fastest* single-threaded |
| Read-heavy, moderate contention | `RwLock<T>` — but measured only ~15% better than a mutex at 8 threads |
| **Concurrent map** | **`dashmap`** (sharded) — measured 3.9× a mutex at 8 threads |
| Producer/consumer | `crossbeam-channel`, `tokio::sync::mpsc` |
| Bounded MPMC queue | `crossbeam::queue::ArrayQueue` |
| Counters | `AtomicU64` with `Relaxed`, or per-thread counters summed |
| Read-mostly config | **`arc-swap`** — readers never block |
| Ordered concurrent map | Skip list (`crossbeam-skiplist`) |

## Complexity

| Structure | Operation | Contention behaviour |
| --- | --- | --- |
| `Mutex<HashMap>` | Θ(1) | **Serializes; degrades superlinearly** — 18× time for 8× work |
| `RwLock<HashMap>` | Θ(1) | Readers still write the reader count — line ping-pongs |
| **Sharded (64-way)** | Θ(1) + hash | **Scales ~linearly** — 7.5× time for 8× work |
| Treiber stack | Θ(1) amortized | All threads CAS one pointer — a hot spot |
| Michael-Scott queue | Θ(1) amortized | Head and tail are separate lines — better |
| Work-stealing deque | Θ(1) owner-side | Contention only on steals |
| `Arc::clone` | Θ(1) | **Contended atomic RMW** — a scalability trap |

**Where the table misleads.** Every row is Θ(1), and they differ by 3.9× at 8 threads and diverge further with more. Asymptotic notation has no term for coherence traffic, which is the only thing that matters here. The right mental model is: **an uncontended atomic is ~20 cycles; a contended one is a cache-line transfer between cores, ~100+ cycles, and it serializes.**

Also note the single-thread row: the mutex was *fastest* uncontended at 95% reads (14.9 ms — though `RwLock` at 8.9 ms was better), and sharding cost slightly more. Concurrency machinery has a floor price.

## Use Cases

- **Caches and memoization tables** shared across request handlers — the canonical `dashmap` case.
- **Work queues in thread pools** — work-stealing deques; `rayon`'s scheduler is built on one.
- **Metrics and counters** — per-thread counters summed on read, avoiding a shared line entirely ([false sharing](../../performance-optimization/false-sharing/learning.md)).
- **Configuration hot-reload** — `arc-swap` publishing a new `Arc<Config>`; readers never block, which composes with [persistent structures](../persistent-immutable-structures/learning.md).
- **Connection and object pools** — bounded queues with blocking semantics.
- **Lock-free logging and tracing** — MPSC queues to a consumer thread, keeping the hot path off any lock.
- **Database buffer pools and page tables** — sharded latches; this is where the technique originated.
- **In-memory indexes** — concurrent skip lists (the memtable in an [LSM tree](../lsm-trees/learning.md) is one, for exactly this reason).

## When to Use Which

| Reach for | When |
| --- | --- |
| **`Mutex<T>`** | Default. Low contention, or the critical section is short |
| `RwLock<T>` | Reads vastly dominate **and** critical sections are long enough to matter |
| **`dashmap`** | Concurrent map with real contention — **3.9× measured** |
| Manual sharding | You need control over the shard function or per-shard structure |
| **Per-thread state + merge** | Counters, accumulators — removes sharing entirely |
| `arc-swap` | Read-mostly data replaced wholesale |
| `crossbeam-channel` | Message passing instead of shared state |
| `crossbeam-epoch` + hand-rolled | You have measured that nothing else suffices |
| **A single-threaded design** | Contention is the problem — consider removing the sharing |

## Pitfalls in Depth

### Pitfall: Expecting `RwLock` to fix read contention

- **What goes wrong:** A `Mutex<HashMap>` is contended under a read-heavy workload, so it's swapped for `RwLock`, and throughput barely moves. Measured at 95% reads and 8 threads: **232.2 ms against the mutex's 267.7 ms** — a 13% improvement where a 10× one was expected.
- **Why it happens (the mechanism):** Acquiring a read lock is not a read — it **increments a shared reader counter**, which is a write to a shared cache line. Every reader on every core therefore takes exclusive ownership of that line, and it ping-pongs between cores exactly as a mutex's lock word does. The logical exclusion is relaxed; the physical contention is unchanged. `RwLock` also typically has a *higher* uncontended cost than a mutex because it tracks more state.
- **How to handle it in production, and why that works:** **Partition instead of relaxing.** Sharding gives each thread a different cache line to contend on, which is why it measured 69.2 ms — 3.4× better than `RwLock` at the same thread count. Where the data is read-mostly and replaced wholesale, `arc-swap` is better still: readers take an `Arc` clone with no shared mutable state on the read path at all.
- **Trade-offs of the fix:** Sharding costs an extra indirection and a hash, measurably worse when uncontended (9.2 ms vs 8.9 ms single-threaded). It also makes cross-shard operations — iteration, size, atomic multi-key updates — either expensive or impossible, which is a real API limitation.

### Pitfall: Assuming lock-free means faster

- **What goes wrong:** A hand-rolled lock-free stack or counter replaces a mutex-protected one and performance is the same or worse — while the code is far harder to review and now needs a reclamation scheme.
- **Why it happens (the mechanism):** Lock-free is a **progress** guarantee, not a performance one. If every thread CASes the same word, they all serialize at the cache-coherence protocol just as they would on a lock — a contended CAS *is* a cache-line transfer. Worse, failed CAS attempts are retried, so under heavy contention lock-free structures can do more total work than a lock, which at least parks the loser.
- **How to handle it in production, and why that works:** Reach for lock-free when you need the progress guarantee (a thread may be descheduled or killed at an arbitrary point — kernels, real-time, signal handlers), not when you want throughput. For throughput, **reduce sharing**: shard, use per-thread state and merge, or pass messages. The measured 3.9× came from partitioning, not from removing locks.
- **Trade-offs of the fix:** Sharding and per-thread state change the API (no global iteration, approximate totals). Message passing adds latency and a queue to size. Both are usually still simpler than correct lock-free code with reclamation.

### Pitfall: Memory reclamation in hand-rolled lock-free code

- **What goes wrong:** A lock-free structure frees a node that another thread is still dereferencing — a use-after-free that manifests as sporadic corruption or a crash under load, and never in testing. Or the reclamation scheme is conservative and never frees, so memory grows without bound.
- **Why it happens (the mechanism):** With a lock, "nobody else is here" is implied by holding it. Lock-free removes that guarantee, so after unlinking a node you cannot know whether a reader still holds a pointer to it. This is the genuine difficulty of lock-free programming: the algorithm is short and the reclamation is not.
- **How to handle it in production, and why that works:** Use `crossbeam-epoch`, which defers frees until every thread has passed through a quiescent point — you get `Guard`-scoped access and `defer_destroy`, and the crate has been reviewed far more than your version will be. Run everything under **Miri** and, for interleaving coverage, **`loom`**, which exhaustively explores thread schedules for small tests and finds the ordering bugs that stress testing misses.
- **Trade-offs of the fix:** Epoch reclamation defers frees, so memory can lag behind — a thread that stalls in a pinned epoch delays reclamation globally. `loom` requires restructuring code to use its mocked atomics and is exponential in interleavings, so it only works on small units.

### Pitfall: `Arc::clone` in a hot path

- **What goes wrong:** A shared structure is passed around as `Arc<T>` and cloned per operation. Each clone is an atomic increment on the *same* refcount word, so every thread contends on one cache line — the exact hot spot the design was trying to avoid. Throughput plateaus and adding cores makes it worse.
- **Why it happens (the mechanism):** `Arc::clone` looks like a cheap pointer copy and is an atomic read-modify-write on shared state. Uncontended that's ~20 cycles; contended it's a cache-line transfer plus serialization, and it scales the way the mutex row in the measured table does. The refcount is a shared mutable counter, which is precisely the thing that doesn't scale.
- **How to handle it in production, and why that works:** Pass `&T` where lifetimes allow, so no refcount is touched. Clone the `Arc` **once per thread** at setup and reuse it, rather than per operation. Where a snapshot is needed, `arc-swap`'s `load` is optimized to avoid the refcount on the common path. This is [false sharing](../../performance-optimization/false-sharing/learning.md) with the counter as the shared line.
- **Trade-offs of the fix:** Passing `&T` requires a lifetime that outlives the usage, which is exactly what `Arc` was avoiding — it can force restructuring or scoped threads. Hoisting the clone out of the loop is usually free and should be the first move.

### Pitfall: Weakening memory ordering without an argument

- **What goes wrong:** `SeqCst` is replaced with `Relaxed` "because it's faster". The code works on x86 — which provides acquire/release semantics in hardware for ordinary loads and stores — and produces reordering bugs on ARM, including Apple Silicon and most cloud ARM instances. The failure is rare, non-deterministic, and usually manifests far from the atomic.
- **Why it happens (the mechanism):** x86's memory model is strong enough that most incorrect orderings are indistinguishable from correct ones there. ARM and other weakly-ordered architectures genuinely reorder loads and stores, so a missing `Release` on the store that publishes data (or a missing `Acquire` on the load that reads it) lets a reader observe a pointer before the data it points to.
- **How to handle it in production, and why that works:** Start with `SeqCst`, which is always correct, and weaken only where profiling shows the ordering itself is the cost — which is rarer than expected, since the cache-line transfer usually dominates the fence. When you do weaken, use the **Acquire/Release pair** for publication and write down the happens-before argument in a comment. Test on ARM, and use `loom`, which models the weak memory ordering explicitly and will find these.
- **Trade-offs of the fix:** `SeqCst` genuinely costs more on ARM (a full barrier versus a one-way fence), so on a hot path the difference is measurable. The discipline is to make weakening a deliberate, justified, tested change rather than a default.
