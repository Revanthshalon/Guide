# Concurrent Data Structures — Quick Reference

## At a Glance

The problem isn't interleaving — it's that **a shared cache line can be owned by one core at a time**. Every concurrent structure answers "how do threads progress without fighting over one line?"

**The win comes from partitioning, not from removing the lock.**

**Linearizability:** each operation appears to take effect instantaneously between invocation and return, consistent with real time.

## The Numbers (95% read, 400k ops/thread, 100k keys, measured)

| Threads | `Mutex<HashMap>` | `RwLock<HashMap>` | **64-way sharded** |
| --- | --- | --- | --- |
| 1 | 14.9 ms | **8.9 ms** | 9.2 ms |
| 2 | 29.6 ms | 28.0 ms | **11.0 ms** |
| 4 | 95.2 ms | 65.5 ms | **18.6 ms** |
| 8 | 267.7 ms | 232.2 ms | **69.2 ms** |

- Sharded **scales** (7.5× time for 8× work); mutex **degrades** (18× for 8× work).
- **`RwLock` barely helps at 95% reads** — a reader still *writes* the reader count, so the line ping-pongs.
- Sharding is slightly *worse* single-threaded. It's a response to measured contention, not a default.

## Progress Guarantees

| Guarantee | Means |
| --- | --- |
| **Wait-free** | every thread finishes in bounded steps |
| **Lock-free** | *some* thread progresses; individuals can starve |
| Obstruction-free | progress if running alone |
| Blocking | a stalled holder blocks everyone |

**Lock-free ≠ fast.** It's a progress property. A contended CAS *is* a cache-line transfer.

## Sharding

```rust
#[inline] fn idx(k: u64) -> usize {
    (k.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 58) as usize % SHARDS   // HASH first
}
fn get(&self, k: u64) -> Option<u64> { self.shards[Self::idx(k)].lock().unwrap().get(&k).copied() }
```

`k % SHARDS` collides badly with sequential IDs — hash, then shard.

## Reclamation (the hard part)

| Scheme | Cost |
| --- | --- |
| **Epoch (`crossbeam-epoch`)** | low; deferred frees accumulate |
| Hazard pointers | per-read store; bounded memory |
| RCU | read side nearly free |
| `Arc` | simple; **every clone is a contended atomic RMW** |

**ABA:** value matches but the structure changed underneath. Fix with tagged pointers, DWCAS, or epochs.

## Memory Ordering

| Ordering | Use |
| --- | --- |
| `Relaxed` | counters where only the final value matters |
| **`Acquire`/`Release`** | **the publication pair** |
| `AcqRel` | RMW that publishes and observes |
| **`SeqCst`** | **start here**; weaken only with a measured reason |

x86 hides ordering bugs; **ARM (incl. Apple Silicon) exposes them.**

## Choose This When

| Use | For |
| --- | --- |
| **`Mutex<T>`** | Default — low contention, short critical section |
| `RwLock<T>` | Reads dominate **and** sections are long |
| **`dashmap`** | Concurrent map with real contention (**3.9×**) |
| **Per-thread state + merge** | Counters — removes sharing entirely |
| `arc-swap` | Read-mostly data replaced wholesale |
| `crossbeam-channel` | Message passing instead of shared state |
| `crossbeam-skiplist` | Ordered concurrent map (no rotations to coordinate) |
| **Single-threaded design** | Contention *is* the problem — remove the sharing |

## Rules of Thumb

- Partition; don't just relax exclusion.
- Concurrent ordered map ⇒ **skip list**, not a balanced tree (local structure, no rotations).
- Hoist `Arc::clone` out of hot loops — it's a contended atomic on one line.
- Uncontended atomic ≈ 20 cycles; contended ≈ 100+ **and serializes**.
- Run `unsafe` concurrent code under **Miri** and **`loom`**.
- Memtables are skip lists for exactly this reason.

## Common Bugs

| Bug | Symptom |
| --- | --- |
| `RwLock` to fix read contention | 13% better where 10× was expected |
| Hand-rolled lock-free for speed | Same or worse, far harder to review |
| Missing/incorrect reclamation | Sporadic use-after-free under load only |
| `Arc::clone` per operation | Throughput plateaus; more cores make it worse |
| Weakened ordering without argument | Works on x86, breaks on ARM |
| `k % SHARDS` on sequential keys | Traffic concentrates on a few shards |

## Key References

- Herlihy & Shavit, *The Art of Multiprocessor Programming*
- Michael & Scott (1996) — the lock-free queue
- [`crossbeam`](https://docs.rs/crossbeam/) · [`dashmap`](https://docs.rs/dashmap/) · [`loom`](https://docs.rs/loom/)
