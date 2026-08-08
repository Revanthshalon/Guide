# Lock-Free Concurrency — Learning Notes

## The Hardware Mechanism

Two hardware facts underlie everything here — one about *atomicity*, one about *ordering*.

**Fact 1: the hardware's unit of agreement is the atomic read-modify-write.** Cores provide a handful of primitives that read, modify, and write a memory location as one indivisible act: `fetch_add`, `swap`, and the queen of them all, **compare-and-swap** (CAS: "if the value is still A, replace it with B; else tell me what it was"). On x86 these are `lock`-prefixed instructions; on ARM (Apple Silicon), load-linked/store-conditional pairs (LL/SC) or the newer LSE atomics. Mechanically, an atomic RMW acquires the cache line in Modified state and holds it through the operation — which yields the cost model that governs this whole topic: **uncontended, an atomic RMW is ~10–20 ns** (line already in your cache: cheap); **contended, it's a line ping-pong** (~40–100+ cycles per steal — [the false-sharing doc's](../false-sharing/learning.md) economics exactly, now with every core aiming at the *same* line on purpose). Note what this implies: an uncontended mutex lock is *itself* one CAS — a locked and a lock-free counter increment cost roughly the same instructions when nobody's fighting. **Contention, not the lock, is the enemy.**

**Fact 2: neither the compiler nor the CPU executes your memory operations in program order.** Stores sit in per-core **store buffers** before reaching cache (so a core sees its own writes early and others see them late); out-of-order execution and compiler reordering shuffle independent accesses freely. Single-threaded code can't tell — the illusion of order is preserved for you alone. The moment two threads communicate through memory, the illusion breaks: thread A writes `data` then sets `ready`; thread B sees `ready == true` and reads *stale* `data`, because A's stores drained out of order or B's loads were hoisted. **Memory orderings** are the leash: each atomic op carries an ordering (`Relaxed`, `Acquire`, `Release`, `AcqRel`, `SeqCst`) that constrains which reorderings the compiler and CPU may perform around it. Platform reality: x86 is strongly ordered (TSO — Acquire/Release compile to plain loads/stores; only SeqCst pays an `mfence`), ARM is weakly ordered (Acquire/Release are real barrier-carrying instructions). Consequence worth framing: **ordering bugs hide on x86 and reproduce on your M-series Mac** — for once, the laptop is the honest machine.

## Mental Model

**"Lock-free" is not a speed claim — it's a progress guarantee. The performance story is the same one as always: contention and coherence traffic. Solve sharing first; choose primitives second.**

1. **What locks actually cost — and when.** Uncontended: one CAS in, one store out (parking_lot / std futex-based mutexes are this thin). Contended: the line ping-pong *plus* the OS (waiting threads park: syscall, context switch, [cache repopulation](../batching-and-amortization/learning.md)). Pathologies beyond throughput: a lock-holder *preempted mid-critical-section* stalls every waiter for a scheduling quantum; priority inversion; deadlock. Lock-free algorithms exist to delete those pathologies — an interrupted lock-free thread stalls nobody; some *other* thread's CAS just succeeds. The price: retry loops (whose cost under contention is the [hot-aggregate retry storm](../../architecture-patterns/event-sourcing/learning.md) at nanosecond scale — the same optimistic-concurrency shape as `expected_version`, all the way down) and the hardest memory-management problem in systems programming (reclamation, below).
2. **The progress-guarantee ladder** (definitions you should own): **lock-free** = *some* thread makes progress in bounded steps (individual threads may starve); **wait-free** = *every* thread progresses in bounded steps (rare, expensive); below both, a mutex guarantees only that progress resumes when the holder is scheduled. These matter for *tail latency and fault isolation*, not throughput — which is why the honest use cases are signal handlers, real-time-ish paths, and "a stalled thread must not stall the system," not "make my counter faster."
3. **The ordering model in one pattern.** Nearly every correct use of non-`Relaxed` atomics is the **publish/subscribe edge**: writer prepares data (plain writes), then `store(&flag_or_ptr, Release)` — "everything I wrote before this is visible to whoever Acquires this." Reader: `load(Acquire)`, then reads the data safely. Release publishes; Acquire subscribes; together they create the happens-before edge. `Relaxed` is for values that *are* the entire message (counters, IDs — no other memory depends on them). `SeqCst` adds a single global order across *unrelated* atomics — needed astonishingly rarely (flag-pairs like Dekker's pattern), and reaching for it "to be safe" is a smell: it papers over an edge you haven't identified, at `mfence` prices, and the bug usually survives.
4. **The hard problem is reclamation, not the algorithm.** Pop a node from a lock-free stack — when may you `free` it? Another thread may *still hold a pointer* it read before your CAS. GC languages dodge this entirely (the collector waits); Rust makes you choose: **epoch-based reclamation** (`crossbeam-epoch`: free deferred until no thread is in an earlier epoch), **hazard pointers**, **`Arc`** (refcount = per-object reclamation, at [contended-refcount prices](../false-sharing/learning.md)), or — the underrated champion — **indices instead of pointers** (a `slotmap`-style [generational arena](../data-oriented-design/learning.md): slots are never freed, generations detect staleness, ABA and reclamation both dissolve). The **ABA problem** is reclamation's evil twin: CAS checks *value equality*, not *history* — the value went A→B→A (node freed and reallocated at the same address) and your CAS succeeds on a corpse. Tagged/generation counters or epoch reclamation are the standard antidotes.
5. **The pragmatism ladder — where you should actually live.** (0) *Don't share*: per-thread state + merge ([false-sharing's](../false-sharing/learning.md) strongest fix). (1) *Share immutably*: `Arc<T>`, and `arc-swap` for read-mostly-reload data. (2) *Locks, properly*: `parking_lot`/std `Mutex` with short critical sections — the correct default, faster than folklore says. (3) *Proven structures*: crossbeam queues/deques, `flume`/tokio channels, `dashmap` — someone already fought ABA for you. (4) *Own atomics for simple state*: counters, flags, seqnums, the publish pattern. (5) *Novel lock-free data structures*: research-grade — enter with `loom`, a reviewer who's done it before, and a written justification for why rungs 0–4 failed. Most "we need lock-free" conversations end correctly at rung 0, 2, or 3.

## Worked Example

Two vignettes: the cost story, then the correctness story.

**A. The counter, four ways** — 8 threads, 10M increments each ([false-sharing's](../false-sharing/learning.md) benchmark, extended up the ladder; illustrative numbers):

```
1. Mutex<u64>, contended            ~9.5 s     lock ping-pong + parking
2. AtomicU64, fetch_add(Relaxed)    ~2.8 s     no parking — but the line still ping-pongs
3. Sharded: 8 × CachePadded<Atomic> ~0.11 s    contention deleted, merge on read
4. Per-thread u64 + final merge     ~0.03 s    sharing deleted entirely
```

Readings: 1→2 is the *lock-free win* — real (no syscalls, no parking) but modest, because the coherence traffic remains: **lock-free didn't fix contention, it only removed the OS from it**. 2→3→4 is the *sharing fix* — 25× then 4× more, dwarfing the primitive choice. The ladder's rung 0 beat rung 4's cleverness by two orders of magnitude. This table is the topic's thesis in four rows.

**B. Config hot-reload — the publish pattern, and how it breaks.** A server reloads config; request threads read it constantly. Rung-1 answer: `arc-swap`:

```rust
static CONFIG: ArcSwap<Config> = /* … */;
// reloader:  CONFIG.store(Arc::new(new_config));     // Release under the hood
// requests:  let c = CONFIG.load();                  // Acquire; lock-free, ~ns, no contention
```

Under the hood this is `AtomicPtr` + Release/Acquire. Hand-rolled, to see the ordering do real work:

```rust
// writer                                    // reader
let p = Box::into_raw(Box::new(cfg));        let p = PTR.load(Acquire);
PTR.store(p, Release);                       let cfg = unsafe { &*p };   // sound: Acquire saw Release
```

Why not `Relaxed`? The writer's *plain stores into the `Config` itself* may still be in its store buffer or reordered after the pointer store — the reader dereferences a published pointer to **unpublished contents**: torn config, on ARM, sometimes, under load. This is the canonical ordering bug: invisible on x86 (TSO drains stores in order), real on your M-series. And who frees the *old* config? `arc-swap` answers with refcounts — readers holding the old `Arc` keep it alive; the last one frees it. You've just watched reclamation get solved by rung 1 instead of by epochs — the ladder saving you from rung 5, in production shape.

**Verification, not vibes:** `loom` (exhaustively explores interleavings of a bounded test — it *finds* the Relaxed bug above deterministically) and `miri` (catches data races and reclamation UB in tests). Concurrent code without loom/miri coverage is untested code that happens to pass.

## Applying It

- **Default stack:** `parking_lot::Mutex`/`RwLock` for shared mutable state (short critical sections — compute outside, mutate inside); `Arc` for shared ownership; `arc-swap` for read-mostly; channels (`flume`, `crossbeam-channel`, tokio's) to replace shared state with message passing; `dashmap` for concurrent maps (sharded locks — rung 2.5, honest and fast); crossbeam `ArrayQueue`/`SegQueue`/deque for lock-free queues someone else verified.
- **Own-atomics craft (rung 4):** `compare_exchange_weak` in retry loops (maps to LL/SC on ARM — spurious failure is fine in a loop, and cheaper); `crossbeam::Backoff` in CAS loops (spin → yield escalation — unbounded hot spinning in user space is how you fight the scheduler and lose); `fetch_update` for read-modify-write closures; orderings chosen per the publish pattern, with a one-line comment naming the edge (`// Release: publishes the entries written above`) — the comment is the review artifact.
- **Ordering defaults:** counters/metrics → `Relaxed`; flag/pointer publication → `Release` store + `Acquire` load; both-directions handshake → `AcqRel` on the RMW; `SeqCst` only with a written argument naming the multi-atomic invariant. When unsure, you're missing a happens-before edge — find it, don't `SeqCst` it.
- **Reclamation menu, in preference order:** redesign to indices/generations (`slotmap` — dissolves the problem); `Arc`/`arc-swap` (per-object refcount); `crossbeam-epoch` (for real node-based structures); hazard pointers (niche). Never raw `free` after CAS-removal.
- **Test like it's hostile:** `loom` for every hand-rolled atomic protocol (bounded but exhaustive); `miri` in CI for the `unsafe` blocks; contention benchmarks on *ARM* (the weak-ordering machine you own is the honest one); [thread-scaling sweeps](../false-sharing/learning.md) as the perf harness.
- **Spinlocks: almost never in user space.** A preempted spinlock holder leaves everyone burning CPU into the scheduler's blind spot; futex-based mutexes (std, parking_lot) spin briefly *then park* — that adaptive shape is strictly better outside kernels and interrupt contexts.

## When It Hurts

- **Contended CAS loops are retry storms.** N threads CAS one hot word: one wins per round, N−1 refetch and retry — the [quadratic collapse](../../architecture-patterns/event-sourcing/learning.md) at cache-line scale, *plus* livelock risk that mutexes' queuing avoids. Under real contention a well-built lock (which parks waiters, forming an orderly queue) can *beat* lock-free retry chaos. Backoff mitigates; sharding (rung 0/3) cures.
- **SeqCst-everywhere is a correctness placebo at barrier prices.** It doesn't fix missing edges — it reorders *atomics* globally, not your plain stores' visibility per se — and it teaches reviewers to stop thinking. Every `SeqCst` without a named invariant is tech debt with a fence attached.
- **Ordering bugs are the worst bug class you can ship:** no crash, no race detector hit in normal runs, reproduction measured in weeks, and heisenbugs that vanish under instrumentation. This is *why* the ladder exists — rungs 0–3 make entire bug classes unconstructable — and why rung 4+ without loom is negligence dressed as performance work.
- **Hand-rolled structures rot.** The Treiber stack from a blog post works until the ABA scenario nobody tested (pop A, thread stalls; A freed, reallocated, pushed; stalled CAS succeeds on the corpse). If you can't *narrate* your structure's ABA-and-reclamation story, you're not done; if you can, you've reimplemented crossbeam, slower.
- **Lock-free can be slower *and* more complex** — the double loss: uncontended it matches a mutex (~same CAS), contended it retries where the mutex queues; its wins are the *pathology deletions* (no priority inversion, no holder-preemption stalls, signal-safety) and specific proven structures (work-stealing deques, MPSC queues). If you can't name which pathology you're deleting, you wanted rung 2.
- **`Relaxed` counters lie in reads:** N threads `fetch_add(Relaxed)` then one thread reads "the total" mid-flight — fine for metrics, wrong for decisions (the read races ongoing adds; no ordering makes it a consistent snapshot). Decisions want the merge protocol (stop-and-sum) or a lock — [replication's](../../architecture-patterns/replication-and-consistency/learning.md) read-classification, at word scale.

## Benchmarking Methodology

- **Contention is the independent variable:** sweep threads × shared-words (1 hot word → sharded → thread-local), report per-thread and total throughput — the [false-sharing harness](../false-sharing/learning.md) extended one axis. Uncontended fast paths measured separately (they're a different regime and usually the common one).
- **Always benchmark against the boring baseline:** `parking_lot::Mutex` at the same contention levels. Lock-free that doesn't beat it *at your contention profile* is complexity without a payer.
- **Correctness tooling is part of the benchmark suite:** a `loom` test per protocol, `miri` in CI, and — for perf runs — verify on ARM *and* x86 if you ship both (the ordering-visibility asymmetry means green-on-x86 proves little).
- **Watch for coordinated-omission-style traps in latency claims:** lock-free's tail-latency advantage (no holder-preemption stalls) only shows under *oversubscription* (more threads than cores) — benchmark that regime deliberately; it's where the progress guarantees earn rent.
- **Measure the reclamation tax:** epoch pins, `Arc` refcount traffic, generation checks — the memory-management scheme is part of the structure's cost, and benchmarks that pre-allocate everything hide it.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Why does an uncontended mutex cost roughly the same as an uncontended atomic increment? Where does the cost actually diverge, and into what two components (hardware + OS)?
2. State the publish pattern's two halves and the guarantee their pairing creates. Then construct the torn-read that `Relaxed` permits, and explain why x86 hides it while ARM shows it.
3. Define lock-free vs wait-free vs mutex-based progress. Which pathologies does lock-freedom delete, and why are those about tail latency rather than throughput?
4. Narrate ABA on a Treiber stack, step by step, and give the two antidote families. Then explain why generational indices dissolve *both* ABA and reclamation.
5. The counter table's 1→2 gap is 3.4×; 2→4 is ~90×. What does each gap measure, and what's the thesis this ratio proves?
6. When does a contended parking_lot mutex *beat* a contended CAS loop, mechanically?
7. Your colleague's code is all `SeqCst` "for safety." Give the two-part critique and the review question that replaces each `SeqCst`.

Measurement exercises:

- Reproduce the four-row counter table at 1/2/4/8 threads (pin threads; verify sharding layout by address). Add row 2.5: `fetch_add(SeqCst)` — measure what the fence costs on your M-series vs the Relaxed row.
- Build the hand-rolled publish (AtomicPtr + Box) with `Relaxed` everywhere; write the loom test that catches it; fix with Release/Acquire and watch loom pass. Then run the buggy version natively on your M-series under load — document whether/when the tear manifests (patience required; that's the lesson).
- Race `arc-swap` reads against `RwLock<Arc<Config>>` reads at 8 threads with a 1/s writer — the read-mostly regime where rung 1 shines; then invert (write-heavy) and watch the picture change.

## Open Questions

- LSE atomics vs LL/SC on Apple Silicon: which does rustc emit for `compare_exchange_weak` today, and does `compare_exchange` vs `_weak` measurably differ on M-series?
- `arc-swap` internals: the thread-local "debt" mechanism that makes uncontended loads refcount-free — read the source; what's the worst-case path?
- Seqlocks in Rust (`seqlock` crate soundness debates): when is a seqlock the right reader-tax escape, and what does it require of the data (`Copy`, no pointers)?
- crossbeam-epoch pin cost on the hot path: measure epoch enter/exit vs an uncontended Arc clone — where's the crossover collection size?
- Futex details on macOS (`os_unfair_lock`, `ulock`): what does parking_lot actually use here, and does the adaptive spin count differ from Linux?

## References

- Mara Bos, *Rust Atomics and Locks* ([marabos.nl/atomics](https://marabos.nl/atomics/) — free online) — **the** reference for this topic in Rust: orderings, futexes, building locks; read cover to cover, it's short.
- Jeff Preshing's blog ([preshing.com](https://preshing.com/)) — the classic acquire/release explainers ("Memory Barriers Are Like Source Control", "The Synchronizes-With Relation"); the intuitions this doc compresses.
- [crossbeam docs](https://docs.rs/crossbeam) (+ `crossbeam-epoch`'s design notes) — the proven-structures shelf and the reclamation machinery, documented by its authors.
- [loom docs](https://docs.rs/loom) — permutation testing for atomic protocols; the worked example's verification tool.
- Herlihy & Shavit, *The Art of Multiprocessor Programming* — the progress-guarantee theory and canonical structures, when you want the research floor under the practice.
- Related topics in this repo: [False Sharing](../false-sharing/learning.md) (the coherence economics all of this rides on), [Batching & Amortization](../batching-and-amortization/learning.md) (per-thread + merge; atomic F costs), [Data-Oriented Design](../data-oriented-design/learning.md) (generational indices dissolving reclamation), [Parallelism & Work Stealing](../parallelism-and-work-stealing/learning.md) (work-stealing deques as lock-free's flagship win), [Event Sourcing](../../architecture-patterns/event-sourcing/learning.md) (CAS = `expected_version` at nanosecond scale — optimistic concurrency all the way down).
