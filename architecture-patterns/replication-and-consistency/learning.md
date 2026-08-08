# Replication & Consistency Models — Learning Notes

## Mental Model

**The moment data exists in two places, "what is the current value?" stops having one answer.** Replication and consistency models are the vocabulary for what answer the system promises.

You replicate for three reasons: survive machine loss (durability/availability), serve reads closer to users (latency), and spread read load (throughput). The cost is fundamental, not incidental: replicas are updated over a network that can delay, drop, and reorder messages, so at any instant replicas can disagree. A **consistency model** is the contract that says which disagreements a client is allowed to observe.

The key insight: consistency models form a **spectrum of promises, each with a price**. At one end, linearizability — the system behaves as if there were one copy, and you pay in latency and availability (coordination on every operation). At the other, eventual consistency — replicas converge if writes stop, and you pay in anomalies the application must tolerate. Between them sit the session guarantees (read-your-writes, monotonic reads, causal consistency) that fix specific user-visible anomalies without full coordination.

Two framing results to hold on to:

- **CAP** (as a slogan, not a theorem to over-apply): during a network partition, a replicated system must choose between serving consistently (refuse some requests) and serving available (return possibly-stale data). No partition, no forced choice — but partitions are a *when*, not an *if*.
- **PACELC** — the more useful refinement: if Partition, trade Availability vs Consistency; Else, trade Latency vs Consistency. Even on a healthy network, stronger consistency costs round trips. This is why the trade-off is permanent, not just a disaster-mode concern.

Everything downstream in this repo — [event-sourcing projections](../event-sourcing/learning.md) lagging the write side, [CDC](../change-data-capture/learning.md) pipelines, cache staleness in [caching strategies](../caching-strategies/learning.md) — is this same phenomenon wearing different clothes: **derived copies lag the source, and you must choose what to promise readers.**

## Core Concepts

### Leader-based (single-leader) replication

- **What it is:** One replica (leader) accepts all writes and streams a change log to followers; reads can go to leader or followers. The default design of Postgres/MySQL replication and of most consensus-backed systems.
- **Why it exists:** A single write point gives all writes a total order for free — no write conflicts, ever. This buys the simplest correctness story available in a replicated system.
- **Example:** Postgres primary + two streaming replicas. All `INSERT`s hit the primary; replicas replay the WAL. A read from a replica may be milliseconds-to-minutes behind — every anomaly in this document follows from that lag.

### Synchronous vs. asynchronous replication

- **What it is:** Sync: the leader waits for follower acknowledgment before confirming the write. Async: the leader confirms immediately and ships changes in the background. Semi-sync: wait for *one* (or a quorum) of N.
- **Why it exists:** This knob is the durability/latency trade in its rawest form. Async: fast writes, but leader failure loses acknowledged-but-unshipped writes. Fully sync: no loss, but one slow follower stalls all writes.
- **Example:** Postgres `synchronous_commit = on` with one synchronous standby: commit latency includes one network round trip; failover to that standby loses nothing. With async replication, failover typically loses the last few hundred milliseconds of "confirmed" writes — and someone must decide whether that's acceptable *before* it happens.

### Multi-leader and leaderless replication

- **What it is:** Multi-leader: several nodes accept writes (multi-datacenter active-active, offline-capable clients). Leaderless (Dynamo-style: Cassandra, Riak): any replica accepts writes; reads and writes go to **quorums** (W + R > N gives overlap).
- **Why it exists:** Writes survive partitions and continue near every user — availability and write latency that a single leader can't offer across regions.
- **Example:** Two datacenters both accept `UPDATE profile SET name = ...`. The same row is updated differently in both during a link failure. Now there are **conflicts** — concurrent versions that something must merge: last-writer-wins (silently drops one write), application merge logic, or CRDTs (data types whose merge is automatic and principled). Conflict handling is not an edge case of multi-leader; it *is* the design.

### Replication lag and its anomalies

- **What it is:** The follower's position behind the leader, and the three classic user-visible bugs it causes: (1) **read-your-writes violation** — user saves, refreshes, edit is gone (read hit a stale replica); (2) **non-monotonic reads** — a value appears, then disappears (second read hit a more-stale replica); (3) **causal violations** — an answer visible before its question (the two writes took different replication paths).
- **Why it exists:** Lag is physics plus load — it can't be eliminated, only bounded and routed around. The anomalies matter because each has a targeted, cheap fix (session guarantee), whereas "just make everything strongly consistent" pays coordination cost on every read in the system.
- **Example:** Fixes in practice: read-your-writes — route the user's reads to the leader for N seconds after their write, or remember the write's log position and only read from replicas at-or-past it. Monotonic reads — pin each session to one replica (consistent hashing on user id). Causal — read-at-or-after the causally-preceding write's position, or accept it as rare.

### Linearizability (and what it actually promises)

- **What it is:** The strongest single-object guarantee: every operation appears to take effect atomically at some instant between its start and end, consistent with real time. Once any client reads the new value, no client may read the old one.
- **Why it exists:** Some coordination problems are simply wrong without it: leader election, uniqueness enforcement, locks, "exactly one of us does this." These need an answer to "what is the value *now*" that all parties agree on — which is what [consensus](../consensus-and-leader-election/learning.md) provides and why linearizable stores are built on consensus protocols.
- **Example:** Two app instances race to claim job 17 with `SET IF NOT EXISTS`. Against a linearizable store (etcd), exactly one wins, guaranteed. Against an eventually-consistent store, both can "win" during a partition — a lock service on such a store is a bug generator. Note the scope: linearizability is per-object; it says nothing about transactions across objects (that's serializability — a different axis).

### Eventual and causal consistency

- **What it is:** Eventual: replicas converge to the same value if writes stop — no promise about *when* or *what you read meanwhile*. Causal: the one meaningful strengthening short of linearizability — writes that causally depend on each other are seen in order everywhere; concurrent writes may be seen in different orders.
- **Why it exists:** Causal consistency is the sweet spot worth knowing exists: it eliminates the anomalies humans actually notice (effect-before-cause), yet remains available under partition — coordination is only needed for true concurrency decisions, which causality doesn't make.
- **Example:** Comment thread: Q ("does this ship to EU?") then A ("yes"). Under eventual consistency a replica may show A without Q. Under causal consistency, never — A carries a dependency on Q. Most real systems approximate causal consistency with session guarantees rather than implementing full dependency tracking.

## Worked Example

A product with a Postgres primary, two async read replicas, and a profile-edit screen.

**1. The bug report.** User edits their display name, hits save (write → primary), page reloads (read → replica B, 800 ms behind). Old name shows. User saves again, confused; support ticket says "edits randomly don't stick."

```
t=0     UPDATE name='Ana'  → primary   (log position 1042)
t=10ms  page reload, read  → replica B (at position 1038)  → returns old name
t=800ms replica B reaches 1042 — too late, user already saw stale data
```

**2. Diagnose with the model.** This is a read-your-writes violation — not a bug in the code, but an unpromised guarantee being assumed. The fix menu is known:

- (a) Read from primary always — correct, and gives up the reason replicas exist.
- (b) Sticky: user's reads → primary for 10 s after their write — cheap, handles the common case, cross-device edits still break.
- (c) Track positions: write returns log position 1042; session stores it; replica reads add `WAIT FOR position ≥ 1042` (or the router picks a sufficiently-caught-up replica). Precise, works cross-request, costs plumbing.

**3. Pick by blast radius.** Only the user's *own* profile screen needs the guarantee — everyone else can see the old name for a second, harmlessly. So: (b) or (c) scoped to own-profile reads; everything else stays on cheap replica reads. This scoping move — *strong guarantees only where the anomaly is user-visible* — is the entire craft of applying consistency models, and the same reasoning used for event-sourcing's projection lag.

**4. Now the failover question.** Primary dies. Async replicas are at position 1040 and 1038; the last 2 confirmed transactions exist nowhere but the dead disk. Promote the replica at 1040: those transactions are gone — was that acceptable? If not, the answer was semi-sync replication (confirm after 1-of-2 replicas ack), decided *before* the failure. Durability policy is a business decision wearing an infrastructure costume.

## Pitfalls in Depth

### Pitfall: Assuming replica reads are current

- **What goes wrong:** Code reads from a replica and treats the result as the present: checks a balance before payout, verifies "email not taken," renders a just-saved form. Under lag, payouts double-spend, duplicates slip in, edits vanish — intermittently, worst under load, invisible in dev where lag ≈ 0.
- **Why it happens (the mechanism):** ORMs and connection poolers make replica routing invisible; dev/staging have no lag; the code *looks* synchronous. The failure needs production load plus unlucky timing, so it ships.
- **How to handle it in production, and why that works:** Classify every read: **decision reads** (something is validated or enforced) go to the leader — full stop; a stale decision read is a correctness bug. **Display reads** tolerate lag, with session guarantees where the user would notice. Make routing *explicit* in code (a `read_for_decision()` vs `read_stale_ok()` API) so review can catch misclassification. Test with injected lag (e.g. `recovery_min_apply_delay` on a test replica) to surface assumptions.
- **Trade-offs of the fix:** Decision reads concentrate load back on the leader — if that's too much, the fix is redesign (claims/reservations through one writer, as in event-sourcing's uniqueness handling), not sneaking decisions back to replicas.

### Pitfall: Failover that loses or forks history

- **What goes wrong:** Primary fails; automation promotes the most-caught-up async replica. Acknowledged writes that never reached it are gone. Worse: the old primary comes back, still thinks it's leader, and accepts writes — **split brain**; two histories diverge and someone must merge or discard one by hand.
- **Why it happens (the mechanism):** Async replication means "confirmed" ≠ "replicated." And a partitioned old leader can't know it was deposed — without a mechanism that *forces* it to know (fencing), it keeps serving. This is exactly the problem [consensus](../consensus-and-leader-election/learning.md) exists to solve; ad-hoc failover scripts re-derive it badly.
- **How to handle it in production, and why that works:** Decide the RPO explicitly: if losing acknowledged writes is unacceptable, run semi-sync (quorum-ack) replication. Use consensus-based failover (Patroni/etcd for Postgres) so leadership is a linearizable fact, and **fence** the old leader (kill its ability to serve — STONITH, revoked lease tokens checked on every write) before promoting. Rehearse failover regularly; an untested failover path is a rumor.
- **Trade-offs of the fix:** Semi-sync adds a round trip to every commit and couples write availability to standby health. Fencing infrastructure is more moving parts. The alternative — discovering the trade-offs during an outage — is strictly worse.

### Pitfall: Last-writer-wins silently eating writes

- **What goes wrong:** A multi-leader or Dynamo-style store resolves concurrent updates by timestamp (LWW). Two updates to the same key from two sides of a partition: the one with the smaller timestamp vanishes — no error, no log, no conflict surfaced. With clock skew, the "later" write can even be the one that *lost*.
- **Why it happens (the mechanism):** LWW is the default in several stores (Cassandra) precisely because it's the only resolution that needs no application involvement — convergence is bought by discarding data. Physical timestamps compound it: clocks skew, so "last" is not even well-defined.
- **How to handle it in production, and why that works:** First, prefer single-leader per key (partition the keyspace so each key has one writer) — conflicts then can't occur. Where multi-writer is genuinely needed: model data so conflicts merge — CRDTs (counters, sets, maps with principled merges) or domain merges (append both, dedupe later); or detect concurrency honestly with version vectors and surface siblings to the application. Reserve LWW for data that is genuinely last-write-meaningful (a sensor's latest reading).
- **Trade-offs of the fix:** CRDTs constrain your data model and carry metadata overhead. Version-vector siblings push merge complexity into application code. Both are the honest price of multi-writer; LWW just hides the same price as silent data loss.

### Pitfall: Quorums treated as strong consistency

- **What goes wrong:** A team sets W=2, R=2, N=3 ("quorum reads and writes — so it's consistent, right?") and then hits stale reads anyway: a read overlapping an in-flight write returns the old value from one replica and the new from another; a partially-failed write (reached 1 of 3, reported failure, never rolled back) resurfaces later; read-repair races produce non-monotonic reads.
- **Why it happens (the mechanism):** W + R > N guarantees *overlap*, not *atomicity*: there is no instant at which the write happens everywhere, no transaction isolation between the write path and the read path, and failed writes are not undone. Overlap without ordering ≠ linearizability — Dynamo-style quorums were designed for availability, and their edge cases are inherent, not bugs.
- **How to handle it in production, and why that works:** Treat leaderless quorums as *tunable eventual consistency* and use them where that's fine (high-write telemetry, carts with CRDT merge). Anything needing real "one current value" semantics — locks, uniqueness, leader election, conditional writes — goes to a consensus-backed store (etcd, ZooKeeper, or your SQL primary), where linearizability is actually implemented (single log, not mere overlap).
- **Trade-offs of the fix:** Splitting data across stores by consistency need adds operational surface. The alternative — pretending one store's guarantee is stronger than it is — fails at the worst possible time (under partition, at load).

### Pitfall: One consistency level for everything

- **What goes wrong:** Either everything reads the leader "to be safe" — the leader saturates, replicas idle, latency climbs, and the replication investment is wasted — or everything reads replicas "for scale" and the correctness bugs from pitfall #1 trickle in for years.
- **Why it happens (the mechanism):** Choosing per-read requires classifying reads, which requires domain thought; a blanket policy requires none. Both blankets fail: consistency needs are a property of *each read's purpose*, not of the system.
- **How to handle it in production, and why that works:** Do the classification pass once: for each query — is a stale answer *wrong* (decision) or merely *dated* (display)? Would the *writing user* notice staleness (session guarantee) or only others (don't care)? Route accordingly, encode it in the data-access layer's API so every new query must declare its class, and monitor lag against the assumptions (alert when replica lag exceeds what display reads were promised).
- **Trade-offs of the fix:** An upfront audit and a slightly wider API. It also produces the most valuable artifact of the exercise: an explicit map of which parts of the domain actually need which promise.

## Design Decisions & Trade-offs

**Topology: single-leader unless proven otherwise.** Single-leader gives total write order and no conflicts; escalate to multi-leader only for multi-region active-active write latency or offline clients — and budget for conflict resolution as a first-class design area, not an afterthought. Leaderless buys write availability and pays in the quorum edge cases above.

**Durability (sync policy) is a business decision.** Ask "how many seconds of acknowledged writes may a failover lose?" If zero: quorum-ack synchronous replication and the latency that comes with it. If a few: async with monitored lag. Write the answer down; it's the RPO.

**Read routing is per-query, not per-system.** Decision reads → leader. Own-data display reads → session guarantees (sticky or position-tracking). Everything else → replicas. This is the highest-leverage consistency decision an application makes, and it's cheap.

**Failover: buy, don't build.** Leader election done correctly is consensus; use the battle-tested implementations (Patroni, managed database failover) with fencing, and rehearse. Hand-rolled failover scripts are where split brain comes from.

**Cross-object consistency is a different axis.** Everything here is per-object/per-stream ordering. Atomicity across objects is transactions (serializability), and across services it doesn't exist — you get [sagas](../saga-pattern/learning.md) and compensation instead. Keep the axes separate in your head; conflating "linearizable" and "serializable" is a classic interview-and-production error.

**The unifying frame for this repo:** event-sourcing projections, CDC pipelines, cache invalidation, and search-index feeds are all *asynchronous single-leader replication* — the event log or WAL is the leader, derived stores are followers, and every question ("can I read my write?", "is this current?") is answered with the vocabulary in this document.

## Open Questions

- Postgres specifics: how exactly does `synchronous_standby_names` quorum syntax behave when a standby stalls — does write availability drop immediately?
- What does position-tracking read-your-writes look like concretely with pgbouncer/pgcat in the path — who tracks the LSN?
- CRDTs in Rust: survey the `crdts` crate — which types are production-ready, and what's the metadata overhead per operation?
- Where is the actual lag distribution (p50/p99) on a loaded Postgres replica, and how does it correlate with vacuum and batch jobs? Measure before trusting any "usually milliseconds" claim.
- Jepsen reports: read the Cassandra and etcd analyses — which of the quorum edge cases above show up as real test failures?

## References

- Martin Kleppmann, *Designing Data-Intensive Applications*, ch. 5 (Replication) and ch. 9 (Consistency & Consensus) — the backbone of this entire topic; ch. 5's lag-anomaly taxonomy and ch. 9's linearizability treatment are definitive.
- [Jepsen analyses](https://jepsen.io/analyses) — real distributed stores failing their claimed guarantees under partition; the best calibration for healthy skepticism.
- Kyle Kingsbury, [Consistency models map](https://jepsen.io/consistency) — the lattice of models (linearizable → sequential → causal → eventual) in one diagram; useful once the prose here is internalized.
- Daniel Abadi, "PACELC" (paper: *Consistency Tradeoffs in Modern Distributed Database System Design*) — the else-latency refinement of CAP that matches how systems actually behave.
- Werner Vogels, "Eventually Consistent" (CACM 2009) — short, canonical statement of the session guarantees (read-your-writes, monotonic reads) from the Dynamo lineage.
- Related topics in this repo: [Consensus & Leader Election](../consensus-and-leader-election/learning.md) (how linearizability and safe failover are actually implemented), [Event Sourcing & CQRS](../event-sourcing/learning.md) (projection lag = replication lag), [Change Data Capture](../change-data-capture/learning.md) (log-tailing as replication), [Caching Strategies](../caching-strategies/learning.md) (caches as the loosest replicas), [Sharding](../sharding/learning.md) (the orthogonal axis: splitting data instead of copying it).
