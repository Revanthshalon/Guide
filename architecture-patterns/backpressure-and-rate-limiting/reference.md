# Backpressure & Rate Limiting — Quick Reference

Core model: a queue doesn't absorb overload, it converts it into latency then memory exhaustion. Little's Law: `L = λW` → `bound = throughput × acceptable_latency`. Wait scales as `ρ/(1−ρ)` — systems fall off a cliff at ~70–85% utilization, they don't degrade gracefully. Details in [learning.md](learning.md).

## The Three Mechanisms

| Mechanism | What it does | Where it belongs |
| --- | --- | --- |
| **Backpressure** | Producer slows/blocks when consumer is full (needs a feedback path) | Internal pipelines; the queue bound *is* the signal |
| **Rate limiting** | Ceiling enforced regardless of cooperation | Trust boundaries, contracts, per-tenant fairness |
| **Load shedding** | Deliberately reject/drop to stay alive | Everywhere over capacity — shed by value, not arrival order |

## Utilization → Latency

| Utilization | Relative wait |
| --- | --- |
| 50% | 1× |
| 80% | 4× ← the knee |
| 90% | 9× |
| 95% | 18× |
| 99% | 99× |

## Rate-Limit Algorithms

| Algorithm | Burst behavior | Use when |
| --- | --- | --- |
| Fixed window | Allows 2N across the boundary | Never at a real boundary |
| Sliding window counter | Smooth approximation | The usual production choice |
| **Token bucket** | Burst up to B, average R | APIs — matches how clients behave |
| Leaky bucket | Strictly smoothed output | Downstream can't tolerate bursts |

## Rules of Thumb

- **Concurrency limits for internal dependencies** (self-adjust via Little's Law when latency changes); **rate limits at trust boundaries** (contracts, fairness).
- Bound every queue; derive the bound from a *written* latency budget. `unbounded_channel()` never on a data path.
- Define full-behavior per queue: reject (external), block (internal producer — that's the backpressure), drop-oldest (telemetry).
- Fail fast beats queueing over capacity: a 2 ms 503 is useful to the client; a 30 s timeout is not.
- Retry budget ≤ ~10% of traffic; retry at **one** layer only; exponential backoff with **full jitter** always.
- Propagate **deadlines**, not per-hop timeouts; check before starting work — dead work is free to reject.
- FIFO is worst under sustained overload (serves the most-likely-abandoned first); LIFO + max-age is the counterintuitive fix.
- Layer limits: global/adaptive (capacity) + per-tenant (fairness) + per-endpoint (expensive resources); weight by cost when request expense varies.
- Backpressure is end-to-end or nothing — one unbounded hop (`tokio::spawn` per request!) swallows it all.
- Go adaptive when a static limit gets edited after incidents in both directions — that's a human control loop.

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Unbounded queues (framework default) | Bound + explicit full-behavior + alert on rejections | Invisible in tests; needs sustained imbalance to show |
| Retry amplification → metastable collapse | Retry budgets, single-layer retries, jittered backoff, circuit breaker | 3 tiers × 3 retries = 27×; self-sustaining after the trigger passes |
| Backpressure stops propagating | Audit every buffer/spawn/hand-off for a bound | `spawn`-per-request looks like handling, is an unbounded queue |
| Wrong limit granularity | Layer global + per-tenant + per-endpoint; weighted costs | Global misses noisy neighbors; per-user misses aggregate overload |
| Serving dead work | Deadline propagation + check before work; LIFO under overload | 100% CPU, 0% useful responses |
| Only happy-path load tests | Ramp to 2–3×, hold, drop to 50%, **measure recovery time** | Metastable failure is only visible in the recovery test |

## Benchmark / Test Checklist

- [ ] Load test past the knee (2–3× capacity), not just to it
- [ ] Recovery test: drop to 50% and time the return to health
- [ ] Client behavior modeled with real timeouts + retries (not a patient generator)
- [ ] Throughput and p99 charted against *offered* load to find the real knee
- [ ] Rejection metrics per limit layer (which limit fired, and why)

## Key References

- [Metastable Failures in Distributed Systems](https://sigops.org/s/conferences/hotos/2021/papers/hotos21-s11-bronson.pdf) (HotOS 2021).
- Amazon Builders' Library: [timeouts/retries/backoff](https://aws.amazon.com/builders-library/timeouts-retries-and-backoff-with-jitter/), [load shedding](https://aws.amazon.com/builders-library/using-load-shedding-to-avoid-overload/).
- Netflix, ["Performance Under Load"](https://netflixtechblog.medium.com/performance-under-load-3e6fa9a60581) — adaptive concurrency limits.
