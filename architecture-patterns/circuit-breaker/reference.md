# Circuit Breaker — Quick Reference

Core model: a timeout bounds one call; it doesn't bound the damage. `in-flight = rate × timeout` — 500/s × 5 s = 2500 held slots, so one hung dependency exhausts the pool and takes down *unrelated* traffic. The breaker changes the failure from slow to fast (~5000× fewer held slots). Its primary beneficiary is the **caller**, not the failing dependency. Details in [learning.md](learning.md).

## The Family (deploy in this priority order)

| Mechanism | Job | Note |
| --- | --- | --- |
| **Timeout** | Bound each call | Breaker without one is *inert* — hung calls never complete, so nothing is recorded |
| **Bulkhead** | Per-dependency resource pool | Does most of the work — contains the blast radius before detection even starts |
| **Circuit breaker** | Stop calling what's already down | Removes the residual waste and latency |
| **Retry (budgeted)** | Handle transient blips | Must nest *inside* the breaker |
| **Fallback** | What to serve when open | A product decision, including "none" |

## States

| State | Behavior | Exit |
| --- | --- | --- |
| Closed | Pass through, record outcomes | Failure rate > threshold (with min volume) → Open |
| Open | Reject instantly, no slot consumed | Cooldown (jittered!) elapses → Half-Open |
| Half-Open | 1–3 probes only | Success → Closed (ramp traffic); failure → Open, cooldown ×2 |

## Failure Classification

| Outcome | Counts as failure? |
| --- | --- |
| Timeout, connection error, 5xx | **Yes** — the dependency couldn't answer |
| 4xx (400/404/422) | **No** — it answered correctly; your request was wrong |
| 429 / 503 + Retry-After | No — back off instead; it's healthy and shedding |
| 200 with an error payload | Needs app-level inspection (defeats naive classification) |

Rule: failure = *"could not tell me the answer"*, not *"I didn't like the answer."*

## Rules of Thumb

- Timeout ≈ dependency p99 × 1.5, and always shorter than the caller's deadline.
- Minimum-volume threshold is mandatory — 1 failure in 1 request is a 100% failure rate.
- Consecutive-failure thresholds miss partial failure (40% broken is never 5 in a row); use rate + min volume.
- **Jitter the cooldown**; limit probes to 1–3; ramp traffic back (10→25→50→100%), don't switch.
- Exponential cooldown growth on repeated failure (5 s → 10 s → 20 s, capped).
- **Retry inside, breaker outside** — a retried-then-failed call is one failure, not three; open circuit ⇒ no retries.
- Per-instance state is the default; distributed state costs a hot-path round trip and a new failure mode (justify it — low per-instance volume is the real case).
- Mark degraded responses explicitly (`degraded: true`) — "no results" ≠ "couldn't check".
- Never fabricate authoritative data in a fallback (empty permissions, zero balance).
- Classify outcomes in **one** function per client, tested.

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Breaker with no timeout | Explicit timeout on every remote call | Hung calls never complete → nothing recorded → breaker inert |
| 4xx counted as failures | Explicit classification function | One buggy client opens the circuit for everyone |
| Half-open thundering herd | Jitter cooldown, 1–3 probes, ramped recovery | Recovery is when the dependency is *most* fragile |
| No fallback decided | Write one per breaker, "fail fast" allowed | Silent fabricated data is worse than an error |
| Breaker hides a rotting dependency | Alert on open events **and** sustained-open; track fallback-serve rate as an SLI | Green dashboards while the fallback is the production path |
| Retry/breaker mis-nested | Retry inner, breaker outer, retry budget | Retries in the window trip the circuit 3× faster |

## Starting Configuration (then tune from baselines)

```
timeout            = p99 × 1.5
failure_rate       > 50% over a 10 s rolling window
minimum_requests   = 20
cooldown           = 5 s × random(0.5, 1.5), doubling to a cap
half_open_probes   = 3, then ramp 10/25/50/100%
bulkhead           = per-dependency semaphore, sized from its share of capacity
```

## Test Checklist

- [ ] Hang drill (`DROP` the dependency, don't mock an error) — finds missing timeouts
- [ ] Verify unrelated endpoints stay healthy during the drill (bulkhead works)
- [ ] Watch recovery for herd behavior after restoring
- [ ] Dependency inventory: timeout, bulkhead, breaker config, written fallback — per dependency

## Key References

- Nygard, *Release It!* — the pattern's origin, with bulkheads and timeouts.
- [Resilience4j docs](https://resilience4j.readme.io/) — the parameter surface as a design checklist.
- AWS, ["Avoiding fallback in distributed systems"](https://aws.amazon.com/builders-library/avoiding-fallback-in-distributed-systems/) — read before designing one.
