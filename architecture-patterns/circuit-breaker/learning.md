# Circuit Breaker — Learning Notes

## Mental Model

**When a dependency is failing, every call you make to it is worse than useless: it costs you a resource slot, it delays the caller, and it denies the dependency the idle time it needs to recover.** A circuit breaker is a stateful wrapper that notices the failures and *stops making the calls* — failing instantly instead of slowly — until evidence suggests recovery.

The critical insight, and the reason timeouts alone aren't enough: **a timeout bounds one request; it doesn't bound the damage.** Consider a dependency that has stopped responding, called by a service handling 500 req/s with a 5-second timeout. Every in-flight request holds a resource — a thread, a task, a connection, a slot in a pool — for the full 5 seconds:

```
in-flight calls ≈ arrival_rate × timeout = 500/s × 5 s = 2 500 held slots
```

That's Little's Law again ([backpressure](../backpressure-and-rate-limiting/learning.md)), and it's the whole argument. A thread pool of 200 is exhausted in under half a second. At that point requests that have *nothing to do with the failing dependency* can't be served either, because there's no capacity left to serve them. **A single failing dependency has taken down the entire service** — a cascading failure, propagating upward to *its* callers, who are now timing out on you.

The breaker interrupts that chain by changing the failure mode from *slow* to *fast*. Failing in 1 ms instead of 5 s reduces held slots by 5000×, which means the service survives its dependency's outage. Note what that reframes: **the breaker's primary job is protecting the caller, not the callee.** Relieving load on the struggling dependency is a real secondary benefit (it gets room to recover instead of being hammered by traffic and retries), but the reason you deploy one is that your own service should not die because something downstream did.

Three states, and the design tension between them:

- **Closed** — normal: calls pass through, outcomes are recorded. If the failure rate crosses a threshold, open.
- **Open** — fail fast: calls are rejected immediately without touching the dependency. After a cooldown, go half-open.
- **Half-Open** — probe: allow a small number of trial calls. Success closes the circuit; failure re-opens it (usually with a longer cooldown).

Half-open is where the design difficulty lives: you must send *some* traffic to discover recovery, but too much traffic to a still-fragile dependency re-breaks it — often worse than the original failure, because a recovering system with cold caches and empty pools is more fragile than a healthy one. That tension shows up in several pitfalls below.

Finally, a breaker is one member of a family and is nearly useless alone. **Timeouts** bound each call, **[bulkheads](#core-concepts)** isolate resource pools so one dependency's failure can't consume all capacity, **[retries with budgets](../backpressure-and-rate-limiting/learning.md)** handle transient blips without amplifying, **fallbacks** decide what to serve when the circuit is open, and the breaker decides *when to stop trying*. Deploying a breaker without a timeout is the most common configuration error in the pattern, and it does nothing at all.

## Core Concepts

### The state machine and its thresholds

- **What it is:** Closed → Open on a failure condition; Open → Half-Open after a cooldown timer; Half-Open → Closed on probe success or → Open on probe failure. The parameters: failure threshold (rate or count), a minimum request volume before the rate is meaningful, the rolling window over which failures are counted, the cooldown duration, and the number of probes required to close.
- **Why it exists:** Each parameter encodes a decision about false positives versus slow detection. A pure *consecutive-failures* threshold (e.g. 5 in a row) is simple but blind to a dependency failing 40% of the time — never five consecutive, permanently broken. A *failure-rate* threshold (e.g. >50% over 10 s) catches partial failure but is meaningless on low volume: 1 failure out of 1 request is a 100% failure rate. This is why **minimum-volume is not optional** — without it, the first failed request after an idle period trips the breaker.
- **Example:** A workable starting configuration: `failure_rate > 50%` over a `10 s` rolling window, `minimum_requests = 20`, `cooldown = 5 s`, `half_open_probes = 3`. Every number should ultimately come from the dependency's measured baseline (see the methodology in Design Decisions) — but *minimum volume* is the one whose absence causes immediate, obvious misbehavior.

### What counts as a failure

- **What it is:** The classification rule mapping each outcome to failure / success / ignored. Failures: timeouts, connection errors, 5xx responses, resource-exhaustion rejections. **Not** failures: 4xx client errors (400, 404, 422 — the dependency is *working correctly* and telling you your request was wrong), and usually not 429/503-with-Retry-After (the dependency is healthy and shedding deliberately — though these should trigger backoff).
- **Why it exists:** Counting 4xx as failures is the most common way to break a breaker: a client sending malformed requests trips the circuit for *everyone*, taking out a perfectly healthy dependency for all callers. The rule that keeps this straight: a failure means **"the dependency could not tell me the answer"**, not "I didn't like the answer."
- **Example:** A validation endpoint returning 422 for a third of requests is functioning perfectly. A breaker counting 422 as failure would sit permanently open, and the service would report the dependency as down while it serves every other caller fine. Meanwhile 429 deserves its own handling — it means "you're over your limit," which calls for [backoff and rate-limit compliance](../backpressure-and-rate-limiting/learning.md), not circuit opening.

### Bulkheads (isolation — the breaker's indispensable partner)

- **What it is:** Separate resource pools per dependency — a connection pool, a semaphore, or a thread pool sized independently for each downstream — so exhaustion in one cannot consume the resources needed by others. Named for a ship's compartments: a hull breach floods one compartment, not the vessel.
- **Why it exists:** Without isolation, all dependencies share one pool, so the *slowest* one determines everyone's availability — the 2 500-held-slots arithmetic from the mental model. Bulkheads bound the blast radius *before* the breaker even notices anything is wrong, which matters because the breaker needs a detection window (seconds) during which damage would otherwise be unbounded. Bulkheads are the structural protection; the breaker is the reactive one.
- **Example:** Give recommendations 20 concurrent slots, payments 50, and search 30. Recommendations hanging consumes 20 slots and nothing else — checkout continues serving with its own untouched capacity, and the recommendations breaker opens a few seconds later to stop even that waste. In Rust this is a `Semaphore` per dependency (see the worked example); in most JVM stacks it's a bounded thread pool per client.

### Fallbacks and graceful degradation

- **What it is:** What the caller returns when the circuit is open. In descending order of quality: **cached/stale data** (serve last-known-good — often indistinguishable from success), **a default** (empty recommendations list, generic ranking), **degraded functionality** (skip the enrichment step), **queue for later** (accept and process asynchronously if the operation permits), and **fail fast with a clear error** (correct when there is no honest alternative).
- **Why it exists:** The breaker converts a slow failure into a fast one; the fallback converts a fast failure into a *degraded success* where the business allows it. Which is available is a domain question, not a technical one — and it's the question that determines whether an outage of a non-critical dependency is invisible to users or a visible incident.
- **Example:** Recommendations circuit open → serve a cached popular-items list. Users see slightly worse recommendations; nobody sees an error page. Contrast payment authorization: there is no honest fallback — fail fast, explicitly, and let the caller decide. **Deciding "no fallback" is a valid outcome** as long as it's decided rather than defaulted; the anti-pattern is a fallback that fabricates authoritative-looking data (an empty balance, a "no permissions" answer that fails open).

### Where the breaker lives: per-instance, shared, or in the mesh

- **What it is:** Breaker state can be **per-process** (each instance tracks its own view — the standard), **shared/distributed** (state in Redis so all instances agree), or **in the infrastructure** (a service mesh like Envoy/Istio implements outlier detection and ejection outside your code).
- **Why it exists:** Per-instance state is simple, has no external dependency, and is usually *correct enough* — every instance observes the same failing dependency and opens within seconds of one another. Shared state trips faster and more uniformly but adds a network call to the hot path, a new dependency that can itself fail, and consistency questions during partitions — usually a bad trade for a mechanism whose job is surviving failure. Mesh-level breakers (outlier detection ejecting bad *endpoints*) compose well with application-level ones because they operate at a different granularity: the mesh removes a bad instance from the pool, the application breaker stops calling the service entirely.
- **Example:** 50 instances each with a local breaker: all open within a few seconds of a dependency failing, with each having sent a handful of failed requests. Total wasted calls ≈ 50 × threshold — acceptable. The distributed alternative saves those calls and costs a Redis round trip on every downstream call forever, plus a new failure mode.

## Worked Example

A checkout service calling three dependencies: payments (critical), inventory (critical), recommendations (nice-to-have). 500 req/s, 200 worker slots. Recommendations starts hanging — responses never arrive.

**Stage 0 — no protection.**

```
t+0.0s   recommendations stops responding
t+0.4s   200 workers all blocked awaiting recommendations (500/s × 0.4 s)
t+0.4s+  checkout serves NOTHING — payments and inventory are healthy and unreachable
t+30s    upstream API gateway times out on checkout, opens ITS circuit
         → the failure has propagated up one level (cascading failure)
```

A non-critical dependency has taken down revenue-generating traffic. Note there was no timeout configured — the default of "wait forever" is the worst possible setting and is the default in many HTTP clients.

**Stage 1 — add a timeout (necessary, insufficient).** A 2-second timeout on recommendations:

```
in-flight = 500/s × 2 s = 1 000 slots wanted, 200 available
→ still fully saturated, just with a 2 s ceiling per request instead of infinity
```

Better — the system now recovers on its own once recommendations does — but it is still completely down during the outage. Timeouts bound the individual call, not the aggregate damage.

**Stage 2 — add a bulkhead.** Cap recommendations concurrency independently:

```rust
static RECS: Semaphore = Semaphore::const_new(20);      // recommendations may hold ≤ 20
static PAY:  Semaphore = Semaphore::const_new(80);      // payments has its own budget
static INV:  Semaphore = Semaphore::const_new(80);

async fn get_recs(req: &Req) -> Option<Recs> {
    let _permit = RECS.try_acquire().ok()?;             // no slot → skip, don't wait
    timeout(Duration::from_millis(300), recs_client.fetch(req)).await.ok()?.ok()
}
```

```
recommendations hangs → consumes 20 slots, blocked there
checkout continues at full rate with 180 slots for payments + inventory
users see checkout working, with no recommendations
```

The outage is now *contained*. But the service still burns 20 slots and one 300 ms timeout per request on a dependency that is definitely down — pure waste, and enough to add latency to every checkout.

**Stage 3 — add the breaker.**

```rust
// Sketch of the state machine wrapping a call.
match breaker.state() {
    State::Open => return fallback(),                     // ~1 µs, no slot consumed
    State::HalfOpen if !breaker.allow_probe() => return fallback(),
    _ => {}
}
let outcome = timeout(Duration::from_millis(300), recs_client.fetch(req)).await;
match &outcome {
    Ok(Ok(_))                        => breaker.record_success(),
    Err(_elapsed)                    => breaker.record_failure(),   // timeout = failure
    Ok(Err(e)) if e.is_server_error() => breaker.record_failure(),  // 5xx = failure
    Ok(Err(_))                       => {}                          // 4xx = NOT a failure
}
```

```
t+0.0s   recommendations hangs
t+0–5s   ~20 concurrent calls time out; failure rate crosses 50% over 20+ requests
t+5s     circuit OPENS
t+5s+    calls cost ~1 µs and zero slots; cached popular-items fallback served
         checkout latency returns to baseline; no capacity wasted
t+35s    HALF-OPEN: 3 probe requests allowed
         still failing → OPEN again, cooldown doubled (5s → 10s → 20s, capped)
t+~4min  recommendations recovers; probes succeed → CLOSED, full traffic resumes
```

**The scoreboard across stages**, during a 5-minute dependency outage:

```
stage 0:  checkout 0% available          (cascading failure, propagated upward)
stage 1:  checkout 0% available          (recovers automatically afterward)
stage 2:  checkout 100%, recs 0%, +300 ms latency on every request
stage 3:  checkout 100%, recs degraded (cached), no added latency
```

Stage 2 → 3 is the breaker's actual contribution, and it's smaller than people expect — **the bulkhead did most of the work.** That ordering is the lesson: isolate first, then break. A breaker without a bulkhead protects you only after its detection window; a bulkhead without a breaker wastes a bounded amount of capacity forever. You want both, in that order of importance.

## Pitfalls in Depth

### Pitfall: A breaker with no timeout (or a timeout longer than the caller's patience)

- **What goes wrong:** The breaker is configured, thresholds tuned, dashboards built — and the HTTP client has no timeout, or a 30-second default. Calls hang; the breaker records neither success nor failure because *nothing has completed*; the circuit stays closed while every worker slot fills with pending calls. The breaker is present and completely inert.
- **Why it happens (the mechanism):** Circuit breakers are *outcome*-driven state machines — they can only react to calls that finish. A hung call is not a failure yet, just a pending one, so the failure counter never moves. Meanwhile many HTTP clients default to no timeout, and the default is invisible until it matters.
- **How to handle it in production, and why that works:** Every remote call gets an explicit timeout, and the timeout must be *shorter than the caller's own deadline* ([deadline propagation](../backpressure-and-rate-limiting/learning.md)) — otherwise you're holding a slot for a response your caller will never wait for. Set timeouts from the dependency's measured p99, not from a round number: p99 × ~1.5 is a reasonable start, and anything above p99.9 is indistinguishable from hanging. Then verify the whole chain in a failure drill: block the dependency at the network level (not a mocked error — a *hang*) and confirm the breaker actually opens.
- **Trade-offs of the fix:** Tight timeouts turn some slow-but-successful requests into failures, which can trip the breaker during a legitimate slow period. That's usually the right call — a response arriving after the caller gave up is worthless — but it means timeouts must be revisited when the dependency's latency distribution changes.

### Pitfall: Counting client errors as failures

- **What goes wrong:** The breaker counts every non-2xx as a failure. One buggy client sends malformed requests at volume, generating 400s; the failure rate crosses the threshold; the circuit opens **for all callers**, including the ones sending valid requests. A healthy dependency has been taken offline by a classification bug.
- **Why it happens (the mechanism):** "Not a success" is easier to implement than a real classification, and in a mock-based test suite the distinction never surfaces because tests exercise the failure path with 5xx. The semantic difference — the dependency *answered correctly* versus the dependency *could not answer* — is only visible with realistic traffic.
- **How to handle it in production, and why that works:** Classify explicitly: 5xx, timeouts, and connection errors are failures; 4xx are successes from the breaker's perspective (the dependency worked); 429 and 503-with-Retry-After get [backoff](../backpressure-and-rate-limiting/learning.md) rather than circuit-opening, because the dependency is healthy and deliberately shedding. Encode the rule in one place — a shared `is_breaker_failure(outcome)` function per client — so it can be reviewed and tested, rather than being re-derived at each call site.
- **Trade-offs of the fix:** A dependency that returns 200 with an error payload defeats the classification entirely (it "answered," so it looks healthy) — those need application-level inspection to be counted properly, which couples the breaker to the payload schema. Worth doing only where such APIs exist.

### Pitfall: Half-open thundering herd

- **What goes wrong:** The circuit opens across 50 instances at roughly the same moment (they all saw the same failure), so they all start the same cooldown timer and all transition to half-open *simultaneously*. The recovering dependency — with cold caches, empty connection pools, and reduced capacity — receives a synchronized burst that immediately re-breaks it. The system oscillates: open, herd, break, open.
- **Why it happens (the mechanism):** Synchronized failure produces synchronized recovery attempts; the fixed cooldown preserves that phase alignment indefinitely. And the moment of half-open is precisely the moment the dependency is *least* able to absorb a burst — recovery is when a system is most fragile, not least.
- **How to handle it in production, and why that works:** **Jitter the cooldown** (`cooldown × random(0.5, 1.5)`) so instances de-synchronize — the same fix as [jittered TTLs and backoff](../backpressure-and-rate-limiting/learning.md), for the same reason. **Limit probes strictly** (1–3 concurrent per instance, not "resume normal traffic"). **Ramp rather than switch**: on probe success, admit 10% of traffic, then 25%, 50%, 100%, re-opening on failure at any step — this treats recovery as a gradient rather than a binary. **Exponential cooldown growth** on repeated failure (5 s → 10 s → 20 s, capped) so a long outage isn't probed every 5 seconds by every instance forever.
- **Trade-offs of the fix:** Ramped recovery means slower return to full capacity after a genuine recovery — seconds to a minute, usually irrelevant next to the outage it prevents. Jitter means instances have differing views of the dependency's health, which is harmless but makes dashboards noisier.

### Pitfall: No fallback decided (or a dishonest one)

- **What goes wrong:** Two failure modes. Either the circuit opens and the caller simply propagates the error — so the user-visible outcome is identical to no breaker at all, just faster (which *is* sometimes correct, but here it's an accident). Or worse: a fallback returns fabricated data that looks authoritative — an empty permissions list interpreted as "no access," a zero balance, an empty cart — and the system makes *wrong decisions* rather than reporting unavailability.
- **Why it happens (the mechanism):** The breaker is added as an infrastructure concern by whoever is on the resilience task, while the fallback is a *product* decision requiring someone to say what the user should see. That conversation often doesn't happen, so the code path defaults to whatever's easiest — usually an empty value, which is silently indistinguishable from a legitimate empty result.
- **How to handle it in production, and why that works:** For every breaker, write down the fallback and its justification — including "fail fast, no fallback" where that's honest (payments, authorization). Make degraded responses **explicitly marked** so callers and users can tell (`{"recommendations": [], "degraded": true}`, a stale-data banner, a `Warning` header) — the difference between "we have nothing to recommend" and "we couldn't check" matters both to users and to any downstream logic. For anything security-adjacent, decide fail-open vs fail-closed *deliberately*: a permissions service that fails open is a vulnerability; one that fails closed is an outage; the right answer is domain-specific and must be written down.
- **Trade-offs of the fix:** Explicit degradation markers mean callers must handle a third state (success / failure / degraded), which propagates through APIs. That's a genuine cost and still much better than fabricated data flowing silently into business logic.

### Pitfall: Breakers hiding the problem instead of surfacing it

- **What goes wrong:** Breakers work so well that a dependency is failing continuously and nobody notices — the fallbacks are good, error rates look fine, dashboards are green. The dependency stays broken for weeks; the fallback quietly becomes the production path; and when the fallback itself eventually fails (a stale cache expires, a default becomes wrong), the incident is now two failures deep with no memory of the first.
- **Why it happens (the mechanism):** A circuit breaker's success is *invisible by design* — it converts a visible outage into a silent degradation. If the only monitored signals are user-facing errors and latency, a permanently-open circuit is indistinguishable from health.
- **How to handle it in production, and why that works:** Treat circuit state as a first-class monitored signal: **alert on circuit-open events** (immediately, not on a dashboard nobody watches), and separately alert on *sustained* open state (a circuit open for more than a few minutes is an incident regardless of how well the fallback is working). Track the fallback-serve rate as an SLI — "12% of recommendation responses were degraded today" is the number that keeps a slow rot visible. Include circuit state in health endpoints so orchestration and ops tooling can see it.
- **Trade-offs of the fix:** Alerting on every open event produces noise from transient blips — tune with a short duration threshold rather than removing the alert, because "the breaker opened and nobody knew" is the failure mode you're preventing.

### Pitfall: Retries and breakers fighting each other

- **What goes wrong:** A retry layer sits *outside* the breaker, so every rejected call is immediately retried — three times, instantly, against an open circuit. Or worse, retries sit *inside* the breaker's measurement window, so one logical failure records three failures and the circuit opens three times faster than intended. Both mis-wire the two mechanisms into working against each other.
- **Why it happens (the mechanism):** The two patterns are usually added at different times by different people, often in different layers (client middleware, service mesh, application code), and neither's configuration mentions the other. Composition order is invisible unless someone deliberately checks.
- **How to handle it in production, and why that works:** Fix the order: **retry inside, breaker outside** — retries handle transient single-call failures; the breaker observes the *final outcome after retries* and decides whether the dependency is systemically down. A retried-then-failed call is one failure, not three. When the circuit is open, retries must not fire at all (a fast rejection is a *decision*, not a transient error worth retrying) — which the ordering handles automatically. Add a [retry budget](../backpressure-and-rate-limiting/learning.md) so amplification is bounded even before the breaker trips, and audit for retries hidden in layers you don't control (client libraries, the mesh, the SDK).
- **Trade-offs of the fix:** With retries inside, the breaker sees fewer, slower-arriving failures — detection is a bit less sensitive, and the timeout budget must accommodate the retry chain (3 attempts × 300 ms + backoff must still fit inside the caller's deadline, which frequently means fewer retries than people initially want).

## Design Decisions & Trade-offs

**Bulkhead first, breaker second.** The worked example's scoreboard makes the priority concrete: isolation contained the failure completely, while the breaker recovered the remaining waste and latency. If you can only do one thing this quarter, cap per-dependency concurrency. (Both are cheap; do both.)

**Per-instance state is the right default.** Local breakers open within seconds of each other, cost nothing, and add no dependency to the failure path. Distributed breaker state buys marginally faster tripping in exchange for a network call on every downstream request and a new component that can fail during exactly the incidents you're protecting against. Reach for shared state only with a specific reason (very low request volume per instance, so local windows never reach minimum volume — a real case, and the main one).

**Derive thresholds from the dependency's measured baseline.** Timeout ≈ p99 × 1.5; minimum volume high enough that the failure *rate* is statistically meaningful over your window (a service receiving 5 req/s per instance needs a longer window or a shared view); failure threshold above the dependency's normal error rate with margin. Copying `50% / 20 requests / 5 s` from a blog post is a starting point, not a configuration.

**Classify outcomes in exactly one place per client.** The 4xx-vs-5xx rule is simple to state and easy to get wrong at the seventeenth call site. One `is_breaker_failure()` function, tested, reused.

**Every breaker needs a written fallback decision, including "none."** And degraded responses should be *marked as degraded* so the distinction between "no results" and "couldn't check" survives into the caller's logic and the user's screen.

**Prefer a library — or the mesh — to hand-rolling.** The state machine is easy; the rolling-window bookkeeping, jitter, ramped recovery, and metrics are where implementations go subtly wrong. In Rust, `tower` middleware layers compose timeouts, concurrency limits, retries, and load-shedding cleanly, and `failsafe`/`circuitbreaker`-family crates provide the state machine (verify current maintenance status). A service mesh's outlier detection covers a *different* granularity — ejecting a bad instance from a pool — and composes with, rather than replaces, an application-level breaker on a whole dependency.

**Test the breaker in a failure drill, not a unit test.** Unit tests verify the state machine; they cannot verify that the timeout is set, the classification is right, the bulkhead is sized sensibly, and the fallback is sane under real traffic. Block a dependency at the network layer in staging (make it *hang*, not error — hangs are the case that finds missing timeouts) and watch the whole chain behave.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Derive the held-slots figure for 800 req/s against a hung dependency with a 3-second timeout. Explain why a 200-slot worker pool means total service failure, and which law you used.
2. Why is the breaker's *primary* beneficiary the caller rather than the failing dependency? What's the secondary benefit?
3. Give the three states and every transition, then explain precisely why half-open is the hardest state to get right.
4. A dependency returns 422 for 40% of requests. Should the circuit open? State the general rule in one sentence.
5. Why does a consecutive-failures threshold miss a dependency failing 40% of the time, and why is a failure-*rate* threshold dangerous without a minimum-volume parameter?
6. Fifty instances open their circuits within the same second. Describe what happens 30 seconds later without jitter, and give the three mitigations.
7. Retries and a breaker in the same call path: state the correct nesting order and what goes wrong in each of the two incorrect arrangements.
8. Your breaker has been open for six days and no alert fired. What did the monitoring miss, and what two signals should have caught it?

Design exercises:

- Pick a real service and inventory its outbound dependencies: for each, record timeout, bulkhead size, breaker config, and fallback. Missing timeouts are the usual finding; missing *written* fallbacks are nearly universal.
- Run the hang drill: in staging, `iptables -j DROP` one dependency (hang, not error) and observe whether the breaker opens, how long it takes, whether other endpoints stay healthy, and what users see. Then restore it and watch the recovery for herd behavior.
- Compute correct timeouts from the last 30 days of a dependency's latency histogram (p99 × 1.5) and compare to what's configured. The gap is usually large in one direction or the other.

## Open Questions

- Rust ecosystem status: which circuit-breaker crate is currently maintained and idiomatic with `tower` layers, and is there a standard way to compose breaker + bulkhead + retry-budget in one stack?
- Adaptive breakers: does anyone derive thresholds automatically from observed baselines (like [adaptive concurrency limits](../backpressure-and-rate-limiting/learning.md) do), or is threshold-tuning still universally manual?
- Mesh vs. application breakers in practice: what's the right division of responsibility when both are present, and do teams actually run both without conflicting behavior?
- Low-volume services: the minimum-volume problem is real below a few req/s per instance — is shared state the only answer, or do longer windows suffice?
- Fallback observability: what's a good SLI shape for "degraded but successful" responses so slow rot stays visible without alert fatigue?

## References

- Michael Nygard, *Release It!* (2nd ed.) — the book that introduced the circuit breaker pattern along with bulkheads and timeouts; the stability-patterns chapters remain the best treatment and read as a catalogue of outages you can now recognize.
- Martin Fowler, ["CircuitBreaker"](https://martinfowler.com/bliki/CircuitBreaker.html) — the concise canonical description with the state diagram.
- [Resilience4j documentation](https://resilience4j.readme.io/) — the clearest modern reference for the *parameters* (sliding windows, minimum volume, half-open permits) even if you never write Java; its config surface is essentially the design checklist.
- Amazon Builders' Library, ["Avoiding fallback in distributed systems"](https://aws.amazon.com/builders-library/avoiding-fallback-in-distributed-systems/) — the contrarian case that fallbacks are often worse than failing, and why; read it before designing one.
- Netflix Hystrix's archived documentation — historically the reference implementation; the design wiki explains bulkhead-per-dependency reasoning particularly well.
- Related topics in this repo: [Backpressure & Rate Limiting](../backpressure-and-rate-limiting/learning.md) (the companion — Little's Law, retry budgets, load shedding), [Caching Strategies](../caching-strategies/learning.md) (the fallback source, and the origin the breaker protects), [Async & I/O](../../performance-optimization/async-and-io/learning.md) (timeouts, cancellation, and why blocked slots are fatal), [Saga Pattern](../saga-pattern/learning.md) (what a failed step means in a workflow), [Load Balancing & Service Discovery](../load-balancing-and-service-discovery/learning.md) (outlier ejection at instance granularity).
