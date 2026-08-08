# Consensus & Leader Election — Learning Notes

## Mental Model

**Consensus is how a group of machines agrees on one answer — and keeps agreeing even while some of them crash, restart, or get cut off.** Everything it's used for reduces to that: electing exactly one leader, maintaining one replicated log everyone applies in the same order, holding one lock, storing one linearizable value. The [replication notes](../replication-and-consistency/learning.md) kept saying "this needs consensus" — failover without split brain, linearizable stores, safe locks. This document is what that black box contains.

Why is agreement even hard? Because of one brutal fact from the [delivery-semantics notes](../idempotency-and-delivery-semantics/learning.md): **over a network, "crashed" and "slow" are indistinguishable.** If node A can't reach node B, A cannot know whether B is dead (safe to take over) or alive-but-partitioned (taking over creates two leaders — split brain). No amount of waiting resolves it; any timeout you pick can be wrong. A naive "if I can't see the leader, I'll become leader" rule *manufactures* split brain under exactly the network conditions where you most need correct behavior.

The escape is the one idea underneath every consensus protocol: **stop asking "is the leader dead?" (unanswerable) and ask "can I get a majority to agree I'm the new leader?" (answerable).** Majorities — quorums — have the property that carries the whole field: **any two majorities of the same group overlap in at least one member.** Two candidates can't both win an election in the same round, because each needs a majority and majorities intersect. A new leader's majority necessarily includes someone who knows about every previously committed decision, so nothing agreed is ever lost. Partition the cluster and at most one side has a majority — the minority side *stops serving* rather than diverging. That's the deliberate trade, and it's [CAP](../replication-and-consistency/learning.md) made concrete: consensus chooses consistency, paying with availability on the minority side and with latency everywhere (every decision costs a round trip to a quorum).

Two more load-bearing ideas complete the model:

- **Terms (epochs) turn time into a counter.** Since clocks can't be trusted, protocols number leadership eras: term 1, term 2, … Each term has at most one leader; a higher term always wins; stale leaders discover their obsolescence by meeting a higher term number and step down. Every message carries the term, so "who is current" is a comparison, not a clock read.
- **Real systems agree on a *log*, not a value.** Agreeing once is a primitive; what systems need is a **replicated state machine**: agree on entry 1, entry 2, … so every node applies the same operations in the same order and computes identical state. This is the same "one ordered log, deterministic appliers" architecture as [event sourcing](../event-sourcing/learning.md) — with the log itself made fault-tolerant by quorum agreement on every entry.

What this buys, concretely: linearizable stores (etcd, ZooKeeper), safe automated failover (Patroni), OpenBao/Vault's integrated Raft storage, Kafka's controller quorum, and every managed database's control plane. What it costs: quorum round-trips per write, a minimum of 3 (usually 5) nodes, and hard unavailability when quorum is lost. The craft is using it *only where its guarantee is irreplaceable* — coordination, membership, locks, metadata — and keeping it out of the data path.

## Core Concepts

### Quorum intersection (why majorities, specifically)

- **What it is:** Decisions require acknowledgment from a majority (⌈(N+1)/2⌉ of N nodes: 2 of 3, 3 of 5). Not "most nodes I can currently see" — a majority of the *configured membership*.
- **Why it exists:** Overlap is the entire trick: any two majorities share a member, so two same-round elections can't both succeed, and a new leader's quorum always contains a witness to every committed entry. This converts "unknowable global truth" into "locally checkable arithmetic."
- **Example:** 5-node cluster, partition splits it 3/2. The 3-side can elect and commit; the 2-side can do neither — it refuses writes rather than fork history. This is also why **even node counts add nothing**: 4 nodes need 3 for quorum, tolerating 1 failure — same as 3 nodes, with more hardware and more ways to fail. Clusters are 3 or 5, occasionally 7.

### Raft in one page

- **What it is:** The consensus protocol designed to be understandable (vs. Paxos's reputation), and the one you'll actually meet — it's inside etcd, OpenBao, Consul, CockroachDB, TiKV, Kafka's KRaft. Three pieces:
  1. **Election:** followers expect heartbeats from the leader; a follower that times out increments the term, becomes a candidate, and requests votes. One vote per node per term (persisted to disk); majority of votes → leader. Randomized election timeouts make split votes rare and self-resolving.
  2. **Log replication:** clients send operations to the leader; the leader appends to its log and replicates to followers; when a majority has *persisted* an entry, it is **committed** — applied to the state machine and acknowledged to the client. Followers' logs are forced to match the leader's (conflicting uncommitted entries are overwritten).
  3. **The safety rule that makes restarts harmless:** a node votes only for candidates whose log is at least as up-to-date as its own. Combined with quorum overlap, this guarantees every elected leader already *has* every committed entry — committed data survives any minority of failures, mechanically.
- **Why it exists:** It's the standard answer to "replicate a log so it survives node loss, with automatic failover." Understanding roughly how it works tells you how it *fails* (see pitfalls: fsync sensitivity, quorum loss, election storms) — which is what you need operationally.
- **Example:** 3 nodes, leader L in term 4 crashes after committing entry 17. Follower F1 (log through 17) times out, becomes candidate in term 5, requests votes. F2 (log through 16 — it missed 17) votes yes; F1 wins. Had F2 been the candidate, F1 would have refused it (F2's log is behind), F2's election would fail, and F1 would win a later timeout: the up-to-date-log rule steering committed data to safety. Entry 17 survives; the client that wrote it is never betrayed.

### Leases (how leadership becomes fast)

- **What it is:** A quorum-granted right to act alone for a bounded time: "you are leader until T; renew before then." Between renewals the leader serves without per-operation quorum round-trips. The same primitive rented out to applications is a **distributed lock** with a TTL.
- **Why it exists:** Pure consensus on every read would be unusably slow; leases amortize agreement over a time window. The catch that must be tattooed somewhere visible: **a lease is a bet on clocks and schedulers.** The holder believes it has time remaining; a GC pause, VM freeze, or clock drift can make that belief false *while it's acting* — the holder wakes up mid-operation, lease expired, successor already elected, and its in-flight writes land anyway. A lease alone bounds *when* a zombie can act; it cannot stop a zombie already in flight.
- **Example:** The classic disaster (Kleppmann's redlock argument): client A holds a lock lease, pauses 12 s in GC; the lease expires at 10 s; client B acquires the lock and writes; A resumes and writes — two writers, lock "held" the whole time. The lock did exactly what it promised; the promise was weaker than everyone assumed. The fix is the next concept.

### Fencing tokens (making stale actors harmless)

- **What it is:** Every lease/lock grant carries a monotonically increasing number (the term, the lock generation). Every *downstream write* carries the token, and the resources being written **reject tokens older than the newest seen**: `UPDATE ... WHERE fencing_token <= $mine` fails for the zombie.
- **Why it exists:** It closes the gap leases leave: instead of trying to stop the stale actor from acting (impossible — you can't reach into a paused process), make its actions *detectably stale at the destination*. Correctness moves from "the zombie never acts" (unenforceable) to "the zombie's actions bounce" (a comparison at the resource). This is the general answer to every split-brain variant — the [zombie projector's](../event-sourcing/learning.md) CAS checkpoint and the [replication failover fencing](../replication-and-consistency/learning.md) are both instances.
- **Example:** Job scheduler with etcd lock. Worker A gets the lock at generation 33, stalls; worker B gets generation 34 and starts writing output rows stamped `gen=34`. A resumes, writes `gen=33` — the storage layer's `WHERE 33 >= current_gen` check fails, A's writes bounce, A notices and kills itself. Note the requirement it smuggles in: *the protected resource must check tokens* — fencing needs cooperation from the thing being fenced, which is the design work.

### Linearizability from consensus (and the read subtlety)

- **What it is:** A consensus-replicated state machine gives [linearizable](../replication-and-consistency/learning.md) operations: everything serializes through the leader's committed log. This is why etcd/ZooKeeper are where locks, uniqueness, and membership live. The subtlety: **reads are only linearizable if they also go through the protocol.** A leader answering reads from local state alone might be a deposed leader serving stale data (it wouldn't know yet); real systems make reads safe via read-index (leader confirms leadership with a quorum heartbeat before answering) or lease-based reads (clock-bounded), and some expose the choice per request.
- **Why it exists:** The read subtlety matters practically: "serializable" or "stale-ok" read modes are dramatically cheaper, and clients accidentally choosing them under a linearizable mental model recreate the very anomalies they adopted consensus to kill.
- **Example:** etcd: default (linearizable) read costs a quorum round-trip; `serializable=true` reads any member's local state — fast, possibly stale. A lock implementation that checks "am I still holder?" with a serializable read has quietly reintroduced the zombie problem.

### Membership, reconfiguration, and what "down" costs

- **What it is:** The cluster's member list is itself state the cluster must agree on; adding/removing nodes goes *through the log* (Raft: one change at a time, or joint consensus), never by editing configs on each box. Quorum math follows the *configured* membership: a 5-node cluster with 2 dead nodes still needs 3 acks — the dead still count in the denominator until formally removed.
- **Why it exists:** Ad-hoc membership edits create disjoint quorum views — the split-brain generator again, one level up. And the denominator rule is the operational gotcha: losing quorum doesn't lose data, but it does mean *no writes and no elections* until quorum is restored — a stopped cluster, by design, rather than a forked one.
- **Example:** 3-node OpenBao Raft cluster; two nodes' disks die. The survivor has all committed data but no quorum: it serves nothing. Recovery is a documented, deliberate manual override (`peers.json`-style forced reconfiguration) — dangerous precisely because it lets a human assert "the others are really dead," the thing the protocol correctly refuses to assume.

## Worked Example

The scenario every ad-hoc failover script gets wrong, run twice — once badly, once with consensus.

**Setup:** Postgres primary P, replica R, and an app that must never have two primaries (split brain = divergent WALs = data loss on merge).

**1. The naive script.** A watchdog pings P every 5 s; three failures → promote R.

```
t=0    network hiccup isolates the watchdog from P   (P is healthy, serving app traffic)
t=15   watchdog: "P is dead" → promotes R
t=15+  P: still primary, still serving writes ────────┐
       R: now also primary, serving writes ───────────┴── SPLIT BRAIN
t=?    hiccup heals; two divergent histories; someone picks which writes to lose
```

The unfixable flaw isn't the timeout's length — it's that *the watchdog answered an unanswerable question* ("is P dead?") from one vantage point.

**2. Same failure, consensus-based (Patroni-style: leadership = a lease in etcd).**

```
P holds key /leader with lease, term 7, renewing every 5 s
t=0    P partitioned from the etcd quorum (app might still reach P!)
t=10   P's lease expires — P cannot renew without quorum
       P's own rule: no lease ⇒ demote self to read-only        ← P fences itself
t=11   R watches /leader vanish, requests it: etcd's quorum grants term 8
       R promotes; proxy/DNS repoints via the same etcd state
t=?    partition heals; P sees term 8 > 7 → follows R. One history. No merge.
```

Three things did the work, and all three were concepts above: the **lease** made P's leadership self-expiring (P demotes because it *can't prove* it's still leader — the burden of proof is inverted); the **quorum** made R's promotion exclusive (only one side of any partition can win term 8); the **term** made the healed P's staleness self-evident. 

**3. The residual hole, closed by fencing.** Between t=10 and P's demotion there's a sliver where P has in-flight writes. Belt-and-suspenders: replication slots/proxy routing carry the term, so a term-7 write arriving after term 8 exists gets rejected at the proxy — the fencing token pattern, applied to database writes. Production systems (Patroni + HAProxy/pgbouncer with health checks against etcd) compose exactly these pieces; nothing here is exotic.

**4. What you paid.** An etcd cluster (3 nodes, its own care and feeding), lease-renewal sensitivity to etcd latency, and ~10 s of write unavailability during the failover window. What you bought: the impossibility of the t=15+ line in version 1. That trade — bounded unavailability for the elimination of divergence — is consensus in one sentence.

## Pitfalls in Depth

### Pitfall: Hand-rolling leader election (the unanswerable question)

- **What goes wrong:** A team writes "simple" failover: health checks + promote script, or a "leader" row in a shared database, or `SETNX` in a single Redis. It works in every test and every drill. Months later a *partial* network failure — the case where "dead" and "unreachable-from-here" diverge — produces two leaders, and the cost isn't the outage but the *merge*: two divergent histories and a human deciding which customers' writes to discard.
- **Why it happens (the mechanism):** The failure detector problem is invisible until a real partition, because in dev and in drills, "can't reach it" and "dead" always coincide. Every DIY design ultimately asks one observer to distinguish them; no observer can. (A single Redis/DB as arbiter just relocates the single point of judgment — and if that arbiter itself fails over asynchronously, its answer can fork too.)
- **How to handle it in production, and why that works:** Never answer "is it dead?"; answer "do I hold the majority-granted lease?" — use an existing consensus store (etcd, ZooKeeper, Consul) or a tool that embeds one (Patroni for Postgres; Kubernetes leases for app-level singletons; OpenBao's Raft does this internally). The quorum answers exclusively, terms make stale leaders self-detecting, and *self-demotion on lease loss* puts the burden of proof on the leader — the inversion that kills split brain. Rehearse the partial-partition case specifically (iptables drops between subsets), not just kill -9.
- **Trade-offs of the fix:** A consensus dependency (3+ nodes) for what felt like a small feature, and failover latency now tied to lease TTLs (seconds, not instant). Cheap against the merge. If the singleton truly doesn't matter (a metrics summarizer), skip election entirely and make the work [idempotent](../idempotency-and-delivery-semantics/learning.md) — two runners is then a cost, not a catastrophe; that's the honest alternative to doing consensus right.

### Pitfall: A lock without a fencing token

- **What goes wrong:** Distributed lock acquired, critical section entered, and the protected invariant is violated *anyway*: two workers processed the same job, two writers appended to the same file. The lock service's logs show a clean sequence — A held it, then B — because the lock behaved perfectly. What lied was the assumption that *holding* the lock and *acting* on it are simultaneous.
- **Why it happens (the mechanism):** The lease-holder's actions can outlive its lease: GC pause, CPU starvation, swapped-out VM, a slow syscall — anything that delays the process between "check lock" and "write" (or mid-write). Locks bound *acquisition*; they cannot reach into a stalled process and cancel its in-flight effects. Every TTL-based lock — etcd, ZooKeeper, Redis, database advisory locks with timeouts — has this gap; the vendor's guarantees are about the lock, not about your downstream writes.
- **How to handle it in production, and why that works:** Fence at the resource: lock grants carry a generation number (etcd revision, ZooKeeper zxid, your own counter); every protected write carries it; the resource rejects stale generations (CAS/conditional write). Correctness now rests on a comparison at the destination, which no pause can subvert. Where the resource can't check tokens (an external API, a file store without conditionals), be honest that the lock is *advisory* — an optimization to reduce duplicate work — and make the operation idempotent so duplicates are harmless: fencing and idempotency are the two exits, and "neither" is not a safe answer.
- **Trade-offs of the fix:** Token-checking must be built into every protected resource (real design work — it's the same cooperation the zombie-projector CAS demands). Idempotency-instead-of-fencing gives up mutual exclusion and keeps only correctness — often the better deal, since it also covers retries you were getting anyway.

### Pitfall: Quorum arithmetic surprises (sizing, spreading, and the even-node trap)

- **What goes wrong:** A 2-node "HA" cluster that goes read-only the moment either node restarts (quorum of 2 = both). A 4-node cluster bought for "extra safety" that tolerates exactly as many failures as 3. A 6-node cluster split 3/3 by a network partition: *neither* side has a majority; total write outage from a partition a 5-node cluster would have shrugged off. A 3-node cluster spread across 2 datacenters, where the DC holding two nodes going dark takes quorum with it.
- **Why it happens (the mechanism):** Intuition says "more nodes = more resilient" and "spread = safe"; the majority function says failure tolerance = ⌈N/2⌉−1, so only odd increments buy anything, even counts add partition-deadlock modes, and *placement* decides which real-world events translate to quorum loss. The arithmetic is trivial; connecting it to failure domains is the step that gets skipped.
- **How to handle it in production, and why that works:** 3 nodes (tolerates 1) or 5 (tolerates 2 — also: survives one down *during* one's maintenance), never even, 7+ only for read-scale reasons with the write-latency cost understood. Place across failure domains such that **no single domain holds a majority**: 5 across three zones as 2/2/1, never 3 anywhere; if you only have 2 sites, understand that consensus cannot save you from the majority site's loss without a tiebreaker in a third location (a tiny witness node is the standard fix). Then rehearse quorum *loss* anyway: it's a stop-the-world state with a manual, dangerous recovery procedure (forced reconfiguration) that should be a runbook, not an improvisation.
- **Trade-offs of the fix:** Three failure domains is a real infrastructure requirement (the witness node mitigates it cheaply). 5 nodes double the quorum write fan-out vs 3. The runbook for forced recovery is itself a loaded gun — gate it behind the same "two humans agree the others are truly dead" bar the protocol was protecting you from skipping.

### Pitfall: Consensus in the data path (the throughput ceiling nobody priced)

- **What goes wrong:** Success story: the team likes etcd, so now every request touches it — per-request locks, work-queue items, rate-limit counters, session state. At modest load the consensus cluster saturates: every write is a quorum round-trip *and an fsync on a majority of nodes* (Raft persists before acking — correctness requires it). Latency climbs, election timeouts start firing under load (a leader too busy to heartbeat looks dead), and the resulting elections drop throughput further: a metastable failure with consensus at the center.
- **Why it happens (the mechanism):** Consensus write cost is structural — network RTT to quorum + majority durable writes, per decision — and it doesn't shard (one group = one serialized log). What was correctly priced for *coordination* rates (elections, membership, config: Hz) gets used at *data* rates (kHz+). The fsync dependency also makes consensus brutally sensitive to disk latency: put etcd/OpenBao-Raft on shared/burstable storage and p99 spikes translate directly into missed heartbeats and spurious elections.
- **How to handle it in production, and why that works:** Consensus is a **control-plane tool**: leadership, membership, locks, small config/metadata — things that change at human/failure timescales. Data-plane traffic goes to data systems, which *use* consensus once per failover rather than once per write (the [replication](../replication-and-consistency/learning.md) architecture: consensus elects the leader; the leader streams). Fast local disks (NVMe, no shared burst credits) for anything running Raft; monitor fsync p99 and leader-election frequency as first-class signals; partition truly heavy coordination by running *multiple* consensus groups (per-shard Raft, as CockroachDB/TiKV do) — sharding the *groups*, since you can't shard a log.
- **Trade-offs of the fix:** Keeping consensus out of the data path means the data path has weaker guarantees (leases + fencing instead of per-op linearizability) — that's the performance/consistency dial, chosen consciously. Multi-group designs reintroduce cross-group consistency questions (now you need [sagas](../saga-pattern/learning.md) or 2PC *above* the groups — no free lunch).

### Pitfall: Trusting the consensus store's client more than the protocol

- **What goes wrong:** The cluster is perfect; the *usage* is broken. A service watches etcd for the leader key, caches "I am leader," and keeps acting on the cache after its session expired (the watch reconnected silently). Or it checks leadership with a stale (serializable) read. Or its lease-renewal loop shares a thread pool with request handling — load starves renewals, leadership flaps, and each flap drops work. The postmortem blames etcd; the protocol never made a false promise.
- **Why it happens (the mechanism):** The guarantee lives at the protocol boundary; every layer between it and your business logic — client library reconnects, local caches, callback queues, thread pools — can stretch "I hold the lease" across time until it's false. The same clock/pause physics as the lock pitfall, recreated in the application's own plumbing.
- **How to handle it in production, and why that works:** Treat leadership as *always-currently-verified, never cached*: gate actions on the session/lease object's live state (client libraries expose this — e.g. a lease keep-alive channel whose closure must halt the actor), isolate the renewal loop from application load (dedicated thread/task, monitored), use linearizable reads for any leadership check that guards a write, and — the durable answer — carry the fencing token through to the effects anyway, so even a confused client's writes bounce. Chaos-test the *client*: pause the process (SIGSTOP) mid-leadership and verify nothing lands.
- **Trade-offs of the fix:** The always-verify discipline threads leadership state through code that would rather not know (an ambient "am I leader?" flag is exactly the cache you must not build). Fencing-everywhere remains the backstop that forgives client bugs — which is the strongest argument for designing resources token-aware from day one.

## Design Decisions & Trade-offs

**Buy, embed, or build — in that order.** Use the consensus already in your platform (Kubernetes leases for singleton controllers, your database's built-in Raft, OpenBao's integrated storage) → run a dedicated store where needed (etcd, ZooKeeper, Consul) → embed a library (raft-rs, openraft) only when building a *data system* whose product is the replicated log itself → write a protocol never (the Jepsen archives are a museum of teams who did).

**One cluster's blast radius.** A shared etcd serving locks for forty teams is a correlated-failure machine (and its own noisy-neighbor problem: one team's lock storm is everyone's election storm). Prefer per-system consensus (each tool embedding its own Raft) or per-domain clusters; share only with quotas and monitoring.

**Lease TTL = failover time vs. flap risk.** Short TTLs (seconds) give fast failover and punish every GC pause and network blip with an election; long TTLs (tens of seconds) ride out blips and stretch outages. Tune with the renewal period at TTL/3, watch election frequency, and remember the *floor*: TTL can't be shorter than your worst honest pause (JVM full GC, VM live-migration) or healthy leaders flap.

**Where this repo's threads converge.** The fencing token is the universal split-brain answer — the same move as the zombie projector's CAS checkpoint, replication's fenced failover, and the saga orchestrator's epoch. Consensus is how the token's *monotonicity* is guaranteed. Once you see "quorum grants an era; effects carry the era; resources reject old eras," every safe-singleton design in this repo is one pattern with different costumes.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Why must any two quorums intersect, and what *specifically* breaks if a "quorum" is defined as "most of the nodes I can currently reach"?
2. A 5-node Raft cluster commits entry 42, then two nodes die. Can a newly elected leader lack entry 42? Which two rules together make the answer "no"?
3. Your lock service is perfect; your critical section still ran twice. Reconstruct the timeline that did it, and name the two distinct fixes (one at the resource, one in the operation's design).
4. Why does a 4-node cluster tolerate no more failures than a 3-node one? What *new* failure mode does the 4th node add?
5. A leader answers a read from local state without contacting anyone. Under what circumstance is the answer stale, and what do etcd-style stores do about it?
6. OpenBao runs 3-node Raft across 2 datacenters (2+1). List every event that costs quorum, and fix the topology with minimal new hardware.

Build exercises:

- Reproduce split brain: two processes contend for a Postgres advisory lock with a TTL you enforce in code; SIGSTOP the holder past its TTL, let the second proceed, resume the first — watch both write. Then add a fencing column (`WHERE generation <= $n` rejected) and watch the zombie bounce. (This is the whole topic in 100 lines.)
- Run the [Raft visualization](https://raft.github.io/) and force: a split vote, a leader partition mid-replication, and a rejoin with conflicting uncommitted entries. Narrate each recovery step in terms of terms + quorum + up-to-date-vote rule.
- On a 3-node etcd (three containers): write under a partition of the leader, watch the term bump and the minority stall; then pause one node's disk I/O and watch election behavior — connect what you see to the fsync sensitivity pitfall.

## References

- Diego Ongaro & John Ousterhout, ["In Search of an Understandable Consensus Algorithm"](https://raft.github.io/raft.pdf) (the Raft paper) — genuinely readable as promised; read §5 (the protocol) and §5.4.1 (the election-safety argument) at minimum.
- [raft.github.io](https://raft.github.io/) — the interactive visualization; the fastest route to intuition about elections and log repair.
- Martin Kleppmann, *Designing Data-Intensive Applications*, ch. 8–9 — the failure-detector impossibility, linearizability, and the lease/pause argument this doc's lock pitfall compresses.
- Martin Kleppmann, ["How to do distributed locking"](https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html) — the fencing-token argument in full, against Redlock; the single most practically useful essay in this topic.
- [etcd documentation](https://etcd.io/docs/) — especially the API guarantees page (linearizable vs serializable reads) and the tuning page (the fsync/heartbeat sensitivity story, firsthand).
- [Jepsen analyses](https://jepsen.io/analyses) — etcd, Consul, and ZooKeeper analyses show where even consensus-backed systems' *client-visible* guarantees have holes; calibration for the "trusting the client" pitfall.
- Related topics in this repo: [Replication & Consistency](../replication-and-consistency/learning.md) (what consensus is *for*; failover and linearizability), [Event Sourcing & CQRS](../event-sourcing/learning.md) (zombie projector = fencing; replicated state machine = the log architecture), [Idempotency & Delivery Semantics](../idempotency-and-delivery-semantics/learning.md) (the crashed-vs-slow ambiguity that starts everything), [Saga Pattern](../saga-pattern/learning.md) (coordination *without* consensus, via compensation), [OpenBao](../../oss-tools/openbao/learning.md) (a Raft system you'll actually operate).
