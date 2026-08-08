# Consensus & Leader Election — Quick Reference

Core model: "is it dead?" is unanswerable (crashed ≡ slow); consensus replaces it with "do I hold a majority-granted lease for the current term?" Quorum overlap makes elections exclusive; terms make stale leaders self-detecting; fencing tokens make their leftover writes bounce. Details in [learning.md](learning.md).

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| Exactly-one semantics: leader, lock, unique claim, membership, config | Data-path traffic (kHz writes) — consensus is a control-plane tool |
| Automated failover that must never split-brain | The singleton is harmless duplicated → make work idempotent instead |
| Linearizable metadata (service discovery, feature flags with teeth) | You're tempted to write the protocol yourself — buy or embed |

## Rules That Do the Work

| Rule | Consequence |
| --- | --- |
| Quorum = majority of *configured* members | Two quorums always overlap → one leader per term; dead nodes still count in the denominator |
| One vote per node per term, persisted | Restart-safe elections |
| Vote only for logs ≥ yours | Every leader already holds all committed entries |
| Commit = majority has fsynced | Disk p99 → heartbeat misses → spurious elections |
| Lease expired ⇒ demote *yourself* | Burden of proof inverted; minority side stops, never forks |
| Effects carry the term/generation; resources reject old ones | Zombies bounce (fencing) — the universal split-brain answer |

## Sizing & Placement

| Cluster | Tolerates | Note |
| --- | --- | --- |
| 3 | 1 | Minimum real HA |
| 4 | 1 | Never — adds 2/2 deadlock, no tolerance gained |
| 5 | 2 | Survives failure during maintenance |
| Across 2 DCs | — | Majority DC loss = outage; add a 3rd-site witness |
| 5 across 3 zones | 2 | Place 2/2/1 — no zone holds a majority |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Hand-rolled failover / leader row / single-Redis lock | Existing consensus (etcd, ZK, Patroni, k8s leases); rehearse *partial* partitions | Works in every drill; forks in the first asymmetric partition |
| Lock held but critical section ran twice | Fencing tokens checked at the resource; or make the operation idempotent | GC pause outlives lease; lock logs look clean |
| Quorum loss = total write outage | Runbook for forced reconfiguration, gated by human quorum | Data survives; availability doesn't — by design |
| Consensus saturated by data-path use | Control plane only; leases+fencing for the fast path; NVMe under Raft; monitor fsync p99 + election rate | Busy leader looks dead → election storm → metastable collapse |
| Client-side leadership caching / stale reads | Gate on live lease channel; linearizable reads for guards; SIGSTOP chaos tests; fence anyway | The protocol was fine; your thread pool wasn't |

## Key References

- Raft paper + [raft.github.io](https://raft.github.io/) visualization.
- Kleppmann, ["How to do distributed locking"](https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html) — fencing tokens.
- Kleppmann, *DDIA* ch. 8–9.
