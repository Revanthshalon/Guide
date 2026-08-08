# Load Balancing & Service Discovery — Quick Reference

Core model: two paired questions — *which instances are healthy?* (discovery) and *which one gets this request?* (balancing) — both answered continuously with stale information. Most "balancer" bugs are really stale-membership or bad-health-signal bugs. Balancing assumes instances are **interchangeable**; [sharding](../sharding/learning.md) assumes they aren't. Details in [learning.md](learning.md).

## Algorithms

| Algorithm | Signal | Use when |
| --- | --- | --- |
| Round-robin | none | Requests and instances genuinely uniform (rare) |
| Least-request | in-flight count | **Big upgrade** — self-correcting for slow instances |
| **P2C (two random choices)** | in-flight of 2 samples | **Default** — near-optimal, O(1), no shared state |
| EWMA / latency-aware | observed latency | Heterogeneous backends, latency-sensitive |
| Consistent hashing (bounded-load) | request key | Affinity has measured payoff (warm caches) |

Power of two choices: random leaves max load ~`log n / log log n`; sampling **two** gives ~`log log n` — exponential gain from one extra sample.

## The Health-Check Split (highest-leverage decision here)

| Check | Asks | Failure means | Must NOT include |
| --- | --- | --- | --- |
| **Liveness** | Is the process responsive? | Restart me | Any dependency |
| **Readiness** | Can *I* serve right now? | Route around me | **Dependency health** ← causes total outages |
| **Passive/outlier** | Are real requests failing? | Eject me temporarily | — |
| Dependency health | Is X usable? | Degrade the *feature* | Belongs to [circuit breakers](../circuit-breaker/learning.md) |

## Rules of Thumb

- **L4 + gRPC/HTTP2 = broken balancing** — connections are balanced, requests are multiplexed. Fix: L7/mesh/client-side; mitigate with jittered `max_connection_age`.
- Slow start (30–120 s) for new endpoints — least-request reads a *cold* instance as an *idle* one and buries it.
- Drain order: fail readiness → wait ≥2× check interval → stop accepting → finish in-flight → exit. Any deploy-time errors are a bug, not weather.
- Panic threshold ~50%: if most of the fleet looks unhealthy, the *signal* is more likely wrong — balance across everything.
- Cap outlier ejection (`max_ejection_percent` ≈ 30%) and use exponential ejection duration — ejection concentrates load and can cascade.
- Retries must select a **different host**, under a retry budget (≤10–20%).
- Capacity for ejection, not just load: N+2, since losing 20% redistributes onto the rest (mind [the knee](../backpressure-and-rate-limiting/learning.md)).
- Zone-aware routing with real spillover — near-free latency and cross-AZ cost savings.
- Prefer platform discovery (K8s EndpointSlice); a standalone registry is a consensus system you must operate.
- DNS caching lies: deregister *before* shutdown rather than waiting for discovery to notice a corpse.

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Deep readiness probe | Own-resources only; panic threshold | Dependency blip ejects the *entire* fleet |
| L4 in front of gRPC | L7 / client-side LB; connection-age cap | Healthy fleet average, terrible p99, uneven CPU |
| Cold instance hammered | Slow start + warmth-aware readiness | The better the LB, the harder it hits the coldest node |
| Ungraceful shutdown | Ordered drain with propagation wait | "Deploy noise" normalized as acceptable |
| Retry + ejection amplification | Different-host retries, budgets, ejection cap | Ejection concentrates load → next ejection |
| Stickiness on interchangeable instances | External session store; bounded-load hashing if affinity is real | Permanent imbalance from popular sticky keys |

## Diagnostic Signatures

| Symptom | Likely cause |
| --- | --- |
| Few instances hot, most idle, adding capacity does nothing | L4 + multiplexed protocol |
| Service p99 = one instance's p99 | Round-robin with a degraded backend |
| Latency spike right after every deploy | No slow start (or cold-cache readiness) |
| Connection errors only during deploys | Drain ordering / propagation gap |
| Entire service down from a dependency blip | Deep readiness probe |

## Key References

- Mitzenmacher, [The Power of Two Choices](https://www.eecs.harvard.edu/~michaelm/postscripts/handbook2001.pdf).
- [Envoy LB docs](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/upstream/load_balancing/load_balancing) — outlier detection, slow start, panic mode as a design checklist.
- Google SRE Book ch. 20 — datacenter load balancing, subsetting.
