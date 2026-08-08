# NUMA Awareness — Quick Reference

Core model: a NUMA machine is a small distributed system sharing an address space — each node owns a memory controller; local ~80–100 ns, remote ~1.5–2× plus congested interconnect, and single-node *saturation* (not latency) is the usual killer. Pages live where first *written* (first-touch), threads migrate unless pinned. Server-class concern: check `numactl --hardware` before believing any of it applies. Details in [learning.md](learning.md).

## When to Reach for It

| Helps when | Hurts when |
| --- | --- |
| Multi-socket / chiplet-NUMA Linux target, memory-bound workload | Single-node box (most consumer/small-cloud) — pure ritual |
| Sequential init before parallel compute (the classic bug) | Compute-bound, cache-resident work — NUMA hides in the noise |
| One node's controller saturated while others idle | Fine-grained pinning would fight legitimate rebalancing |
| Big shared irregular structures hammering node 0 | vNUMA topology is fake/unstable — verify first |

## The Policy Escalation

| Step | Move | For |
| --- | --- | --- |
| 1 | Parallel first-touch init, partitioned like the compute (rayon `par_chunks_mut` init) | Partitionable data — often free; ~2× in the worked example |
| 2 | Pin pools to nodes (`start_handler` + `core_affinity`, or `numactl --cpunodebind`) | Making placement durable against migration |
| 3 | `numactl --interleave=all` | Unpartitionable shared structures — evens congestion, forfeits local best-case |
| 4 | Replicate read-mostly per node | Lookup tables/config — RAM spent, update story required |
| 5 | Shard-per-node processes | The shared-nothing endgame; deletes remote access by construction |

## Rules of Thumb

- Placement follows the *first write*, not malloc — initialization is a design act; audit any sequential load phase before parallel compute.
- Aggregate bandwidth can look "fine" while node 0 saturates — read per-node counters (`numastat`, pcm-memory).
- `numastat -p <pid>`: numa_foreign/other_node climbing = the smoking gun.
- Pin coarse (pools→nodes), not fine (threads→cores), until measurement demands finer.
- Interleave raises the floor, never the ceiling.
- Cross-node line ping-pong costs multiples of same-node — contended atomics/locks worsen across the fabric.
- Huge pages: first-touch places 2 MB at a time — keep init chunks ≥ huge-page size.
- Cloud: log `numactl --hardware` at boot; re-verify after resizes/migrations.
- Rust: no std API — `numactl` wrapper first (zero code), then `core_affinity`/`hwloc`, `libnuma` bindings last.

## Numbers to Remember

| Thing | Number |
| --- | --- |
| Local DRAM | ~80–100 ns |
| Remote DRAM | ~130–200 ns (1.5–2×), lower bandwidth, contended |
| Naive init on 2 sockets | ~1.08× of one node (v0) |
| Parallel first-touch + pinned | ~1.85× (v1) — zero bytes moved at compute time |
| Interleave on shared table | ~1.7× vs 0.9× default (congestion evened) |

## Benchmark Checklist

- [ ] Machine matrix measured once: MLC/STREAM per (cpu-node, mem-node) pair — the calibration card
- [ ] v0/v1 init A/B with `numastat` deltas — isolates placement, sizes your exposure
- [ ] Per-node bandwidth watched, not just aggregate
- [ ] Scaling curves per policy (default / first-touch+pin / interleave)
- [ ] Working set sized past LLC; runs long enough for migration to show (or pinned and stated)

## Key References

- Drepper §5 — first-touch and policies, from the source.
- Lameter, "NUMA: An Overview" (ACM Queue) — the kernel-side survey.
- `numactl`/`numastat` man pages + Intel MLC — the toolkit.
