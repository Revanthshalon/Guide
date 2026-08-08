# Replication & Consistency Models — Quick Reference

Core model: replicas lag; a consistency model is the promise about which disagreements readers may observe. Strong = coordination = latency (PACELC: even without partitions, consistency costs latency). Details in [learning.md](learning.md).

## Choosing a Guarantee per Read

| Read's purpose | Guarantee needed | Implementation |
| --- | --- | --- |
| Validates/enforces something (decision read) | Linearizable / leader | Read the primary, always |
| User views their own recent write | Read-your-writes | Sticky-to-leader window, or track log position and wait |
| User browsing same data repeatedly | Monotonic reads | Pin session to one replica |
| Anyone else's data, display only | Eventual | Any replica, monitor lag |
| Locks, uniqueness, leader election | Linearizable + consensus-backed | etcd/ZooKeeper/SQL primary — never a Dynamo-style quorum |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Replica read treated as current | Classify reads: decision → leader, display → replica; explicit API split | Invisible in dev (lag ≈ 0); test with injected lag |
| Failover loses acknowledged writes | Decide RPO first; semi-sync/quorum-ack if RPO = 0 | Sync standby stall couples write availability to standby health |
| Split brain after failover | Consensus-based promotion (Patroni etc.) + fence old leader | Untested failover = rumor; rehearse |
| LWW silently drops concurrent writes | Single-writer per key; else CRDTs / version vectors / domain merge | Clock skew makes "last" wrong; LWW only for genuinely latest-wins data |
| Quorum (W+R>N) assumed linearizable | It's overlap, not atomicity — tunable eventual only | Failed partial writes resurface; read-repair races |
| One consistency level for everything | Per-query classification, encoded in data-access API | "Leader for safety" saturates it; "replicas for scale" breeds stale-read bugs |

## Production Checklist

- [ ] RPO decided and written down; sync policy matches it
- [ ] Every query classified decision vs. display; routing explicit in code
- [ ] Read-your-writes in place for own-data screens
- [ ] Replica lag monitored with alerts tied to what display reads assume
- [ ] Failover automated via consensus, old leader fenced, rehearsed recently
- [ ] Conflict resolution designed (not defaulted) for any multi-writer data
- [ ] Locks/uniqueness/election only on consensus-backed stores

## Key References

- Kleppmann, *DDIA* ch. 5 & 9 — the definitive treatment.
- [jepsen.io/consistency](https://jepsen.io/consistency) — the model lattice in one map.
- Jepsen analyses — real stores failing claimed guarantees.
