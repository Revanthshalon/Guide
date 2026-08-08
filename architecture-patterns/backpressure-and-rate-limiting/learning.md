# Backpressure & Rate Limiting — Learning Notes

## Mental Model

**A queue does not absorb overload. It converts overload into latency, and then into memory exhaustion.** That sentence is the whole topic. When arrival rate exceeds service rate, the excess has to go *somewhere*: into a queue that grows for as long as the imbalance lasts, taking response times with it. The queue feels like a shock absorber and behaves like a debt accumulator.

The arithmetic is **Little's Law**: `L = λ × W` — items in system = arrival rate × time in system. Rearranged for the thing you care about: `W = L / λ`. A queue holding 1000 requests, drained at 100/s, means every request waits **10 seconds**. Nobody chose that number; it emerged from a queue bound nobody set and a capacity nobody measured. And the approach to saturation is not linear — for a simple queue, wait time scales as `ρ/(1−ρ)` where ρ is utilization:

```
utilization 50%  →  relative wait  1×      comfortable
            80%  →                 4×      the knee
            90%  →                 9×      visibly degraded
            95%  →                18×      users are leaving
            99%  →                99×      effectively down
```

Systems don't degrade gracefully into overload; they fall off a cliff located around 70–85% utilization. **Capacity planning that targets 95% utilization is planning to be down.**

Three mechanisms respond to this, and they're routinely conflated:

- **Backpressure** — *flow control*: a slow consumer makes the producer slow down, by blocking it or refusing its writes. Requires a feedback path all the way up the chain. The bound on a queue *is* backpressure — a full bounded queue forces the producer to wait or fail, which is the signal.
- **Rate limiting** — *admission control*: a ceiling enforced at a boundary (`1000 req/min per API key`), regardless of current load. Requires no cooperation from the caller, which is why it's what you use at trust boundaries.
- **Load shedding** — *triage*: deliberately rejecting or dropping work to keep the system alive, ideally shedding the least valuable work first.

The distinction that matters operationally: backpressure asks nicely and needs everyone to participate; rate limiting doesn't ask. Internal pipelines use backpressure; external boundaries need limits.

The fourth idea, and the reason all this is urgent rather than merely tidy: **metastable failure**. Past a certain load, some systems enter a state that *sustains itself after the trigger is gone* — retries from timed-out requests add load, which causes more timeouts, which cause more retries. Remove the original spike and the system stays down, because it is now the cause of its own overload. Bounded queues, retry budgets, and shedding exist primarily to make this state unreachable, because once you're in it the only exit is dropping traffic to zero.

## Core Concepts

### Bounded queues (the primitive everything else is built from)

- **What it is:** Every buffer — channel, thread-pool queue, connection pool, HTTP server accept backlog — has a maximum size, and a defined behavior when full: block the producer, reject the item, or drop the oldest.
- **Why it exists:** The bound converts an invisible failure (unbounded growth → latency → OOM) into a visible, handleable one (a rejection you can count, alert on, and respond to). Choosing the bound *is* choosing your latency ceiling, via Little's Law: if you can serve 200/s and the latency budget is 1 second, the queue may hold at most 200 items. Anything beyond that is a request that will time out before it's served — accepting it is worse than rejecting it, because you'll spend capacity on work nobody wants anymore.
- **Example:** Rust makes the choice explicit and should be treated as a design decision, not a default: `tokio::sync::mpsc::channel(200)` (bounded — `send().await` waits when full, which *is* the backpressure propagating to the producer) versus `unbounded_channel()` (no bound, no signal, unbounded memory). The [async doc's](../../performance-optimization/async-and-io/learning.md) rule stands: unbounded channels do not belong on data paths.

### Rate limiting algorithms

- **What it is:** Four families, in increasing fidelity. **Fixed window** (N per calendar minute — trivial, but allows a 2N burst across a boundary). **Sliding window log** (timestamps of every request — exact, memory-heavy). **Sliding window counter** (weighted blend of current and previous window — the usual production compromise). **Token bucket** (tokens refill at rate R up to capacity B; a request costs a token — allows bursts of B while enforcing average R, which matches how real clients behave). **Leaky bucket** (requests drain at a fixed rate — smooths output rather than permitting bursts).
- **Why it exists:** The choice encodes a policy about burstiness. Token bucket is the default for APIs because clients are naturally bursty and a strict average is hostile to legitimate use; leaky bucket is right when the *downstream* can't tolerate bursts (a legacy system, a hardware device). Fixed windows are attractive for their simplicity and wrong at the boundary — 100 requests at 11:59:59 plus 100 at 12:00:00 is 200 in a second under a "100/minute" limit.
- **Example:** Token bucket with `rate = 100/s, burst = 300`: a client idle for 3 seconds may fire 300 immediately, then settles to 100/s. That's usually the intended contract, and it's one line of state per client (`tokens`, `last_refill`) — cheap enough to run per-key at the edge.

### Concurrency limits (usually better than rate limits)

- **What it is:** A cap on *in-flight* requests rather than requests per second — a semaphore of size N around a resource. Little's Law ties them: `concurrency = rate × latency`.
- **Why it exists:** Rate limits are fixed guesses about capacity; concurrency limits **self-adjust to reality**. When a dependency slows from 50 ms to 500 ms, a rate limit of 200/s keeps admitting 200/s (and the queue explodes); a concurrency limit of 10 automatically admits 10× less work, because each request occupies a slot for longer. The limit tracks the resource's *actual* capacity without anyone updating a config. This is why concurrency limiting is the right default for protecting internal dependencies, and rate limiting is for enforcing contracts at the boundary.
- **Example:** A semaphore of 20 around database calls: at 50 ms/query it admits ~400/s; when the database degrades to 400 ms/query it admits ~50/s — automatically shedding load in proportion to the damage, with no configuration change and no alert-driven human intervention.

### Adaptive limits

- **What it is:** Concurrency limits that *discover* the right value at runtime by watching latency, using congestion-control algorithms borrowed from TCP: **AIMD** (additive increase, multiplicative decrease — grow the limit slowly while healthy, halve it on overload) and **Vegas/gradient** approaches (compare current latency to the observed minimum; shrink the limit as the ratio grows).
- **Why it exists:** Every static limit is wrong: too low wastes capacity, too high fails to protect, and the right value changes with deploys, hardware, cache state, and neighbors. Adaptive limits treat "what can this dependency handle right now?" as a continuously-measured quantity rather than a config value that someone tuned once in 2023.
- **Example:** Netflix's `concurrency-limits` library is the reference implementation of the idea. The tell that you need it: a limit that's been raised three times after incidents and lowered twice after wasting capacity — that oscillation is a control loop being run manually by humans on an incident cadence.

### Load shedding and prioritization

- **What it is:** When over capacity, deciding *what* to reject rather than rejecting whatever arrives next. Dimensions: by **priority** (drop analytics before checkout), by **cost** (reject expensive queries first), by **deadline** (drop work whose client has already given up), and by **tenant** (protect fair shares).
- **Why it exists:** Uniform shedding under overload degrades every feature equally — including the one that makes money. Explicit prioritization means a system at 50% capacity still serves 100% of critical traffic. **Deadline-based shedding is the highest-value and least-implemented**: a request that has been queued for 10 seconds under a 5-second client timeout is *dead work* — the client is gone, the response goes nowhere, and serving it consumes capacity that would have served a live request. Checking "is this still wanted?" before starting work is nearly free and directly attacks the metastable-collapse loop.
- **Example:** The counterintuitive companion: **LIFO queueing under overload**. FIFO serves the oldest request first — the one most likely to have already timed out — so under sustained overload FIFO can serve *nothing* successfully while LIFO serves the newest (freshest, most likely still-wanted) requests and fails the rest fast. Some load balancers and thread pools offer this deliberately.

### Deadline propagation

- **What it is:** A request carries a wall-clock deadline (not a per-hop timeout) through every downstream call: the gateway stamps "this dies at T+2s," each service passes the remaining budget, and any service can check whether there's time left before starting work.
- **Why it exists:** Per-hop timeouts compose badly — a chain of five services with 2-second timeouts each can legitimately take 10 seconds while the client gave up at 3. Deadlines make the whole chain honest and make the dead-work check possible at every layer. gRPC has this built in; HTTP conventions vary (an `X-Request-Deadline` header or the `Deadline` gRPC-web mapping).
- **Example:** A gateway sets a 2 s deadline; service A spends 1.8 s, then calls B with 200 ms remaining. B sees a budget it cannot meet for a query averaging 400 ms and fails immediately rather than burning 400 ms on a response nobody will read.

## Worked Example

An API service backed by a database that saturates at ~200 queries/s. Four stages, each fixing the failure the previous stage exposed.

**Stage 0 — unbounded everything (the default).** Framework defaults: an unbounded request queue, an unbounded internal channel, a connection pool that waits indefinitely.

```
traffic spike: 500 req/s arrives, capacity is 200 req/s
t+0s    queue grows 300/s
t+10s   3,000 queued → p99 latency 15s → clients time out at 5s and RETRY
t+20s   effective arrival 500 + retries ≈ 900/s; queue 9,000; memory climbing
t+60s   OOM kill, or: every response is to a client that left long ago
        → traffic subsides to 300/s (below capacity!) and the system STAYS DOWN
```

That last line is the metastable failure: the input is now serviceable, but the backlog plus retries keep the system saturated. No amount of waiting fixes it — only dropping the queue does.

**Stage 1 — bound the queues.** Little's Law with a 1-second latency budget at 200/s capacity → queue of 200:

```rust
let (tx, rx) = tokio::sync::mpsc::channel::<Request>(200);
match tx.try_send(req) {
    Ok(_) => { /* accepted */ }
    Err(TrySendError::Full(_)) => return Response::status(503).retry_after(1),
}
```

```
500 req/s arrives → 200/s served at ~1s latency, 300/s rejected in microseconds
memory flat. Recovers instantly when the spike ends.
```

The system now *fails partially instead of totally* — the fundamental trade of this topic. But two problems remain: rejection is indiscriminate (checkout dies alongside analytics), and the bound of 200 is a guess that's wrong the moment the database slows.

**Stage 2 — concurrency limit instead of a queue-size guess.**

```rust
static DB_SLOTS: Semaphore = Semaphore::const_new(20);   // 20 in-flight DB calls

let permit = match DB_SLOTS.try_acquire() {
    Ok(p) => p,
    Err(_) => return Response::status(503),   // shed immediately, don't queue
};
let rows = db.query(sql).await;    // permit released on drop
```

Now the protection is self-adjusting: at 50 ms/query, 20 slots admit ~400/s; when the database degrades to 200 ms, the same 20 slots admit ~100/s — automatically. Nobody changed a config; Little's Law did the work.

**Stage 3 — prioritize and check deadlines.**

```rust
// Reserve capacity for critical traffic; shed low-priority first.
let slots = match req.priority {
    Priority::Critical => &CRITICAL_SLOTS,   // 20 slots, always available
    Priority::Normal   => &NORMAL_SLOTS,     // 15 slots
    Priority::Bulk     => &BULK_SLOTS,       // 5 slots, first to starve
};

// Don't start work nobody is waiting for.
if req.deadline < Instant::now() + estimated_duration {
    return Response::status(504);            // dead work, rejected for free
}
```

**The outcome across stages**, at 500 req/s against 200 req/s of capacity:

```
stage 0:  0 successful  (collapsed, stays collapsed)
stage 1:  200/s successful, 300/s fast 503s, instant recovery
stage 2:  same, and correct automatically when DB latency changes
stage 3:  100% of critical traffic served; bulk traffic shed first
```

Every stage served *fewer or equal* total requests than the naive version attempted — and vastly more than it *completed*. That's the reframe: capacity you don't have cannot be conjured by queueing, and pretending otherwise converts a partial outage into a total one.

## Pitfalls in Depth

### Pitfall: Unbounded queues (the default almost everywhere)

- **What goes wrong:** An unbounded channel, an unbounded executor queue, a connection pool with no wait cap, `unbounded_channel()` chosen because bounding "seemed like it might drop things." Under overload, memory grows until the process dies — and *before* it dies, every queued request sits past its client's timeout, so the system does maximum work for zero successful responses.
- **Why it happens (the mechanism):** Bounding forces you to answer "what do I do when full?", which is a real design question with no free answer. Unbounded defers it — and the deferral is invisible in testing, because test load never sustains an arrival/service imbalance long enough to matter. Many frameworks ship unbounded as the default, so the choice is often never consciously made.
- **How to handle it in production, and why that works:** Bound every queue, and derive the bound from Little's Law rather than intuition: `bound = throughput × acceptable_latency`. Define full-behavior explicitly per queue — reject (external requests: return 503 + `Retry-After`), block (internal producers: `send().await` is exactly the backpressure signal propagating), or drop-oldest (telemetry, where freshness beats completeness). Then *alert on rejections* — they are the visible symptom of an imbalance that was previously invisible.
- **Trade-offs of the fix:** You will drop requests you previously "accepted" — which feels worse and is better, because the accepted ones were failing anyway, more expensively. Blocking producers propagates slowness upstream, which is the point but must be handled at the top (the edge must shed, or the block just moves the queue into the client).

### Pitfall: Retry amplification and metastable collapse

- **What goes wrong:** A dependency slows; clients time out and retry; retries multiply load exactly when the system is weakest; more requests time out; more retries. Load can reach 3–5× the organic rate at the worst possible moment. When the original trigger passes, the retry-generated load sustains the overload and the system does not recover on its own.
- **Why it happens (the mechanism):** Retries are locally rational (this request might succeed) and globally catastrophic (everyone retrying converts a latency blip into a positive feedback loop). Layered retries compound multiplicatively: three tiers each retrying three times is 27× amplification. And the loop is *self-sustaining* — it needs no external input once started, which is what makes it metastable rather than merely bad.
- **How to handle it in production, and why that works:** **Retry budgets** are the key idea: cap retries as a fraction of total traffic (e.g. ≤10% — the [gRPC/Envoy retry-budget model](https://grpc.io/docs/guides/retry/)) so amplification is bounded no matter how many failures occur. **Retry only at one layer** (usually the outermost) — nested retries multiply. Always **exponential backoff with full jitter** (`sleep = random(0, min(cap, base × 2^attempt))`; without jitter, retries synchronize into waves). Never retry non-idempotent operations without an [idempotency key](../idempotency-and-delivery-semantics/learning.md). Pair with a [circuit breaker](../circuit-breaker/learning.md), which is the mechanism that stops retrying a dependency that is *definitely* down.
- **Trade-offs of the fix:** Retry budgets mean some transient failures aren't retried and surface to the user — correct, because the alternative is everyone failing. Backoff increases individual latency for the retried request in exchange for system survival.

### Pitfall: Backpressure that stops propagating

- **What goes wrong:** The pipeline is carefully bounded — except for one unbounded channel between stages, or a `tokio::spawn` per request that detaches work from any queue, or a handler that fires an async task and returns 202 immediately. The bounded stages are protected; the unbounded link silently absorbs the entire overload, and the system fails there instead.
- **Why it happens (the mechanism):** Backpressure is an **end-to-end property**, and a chain has the strength of its weakest link. Every place that accepts work without a bound — an unbounded channel, an unlimited spawn, a fire-and-forget publish — is a place where the signal is swallowed and the pressure accumulates invisibly. `tokio::spawn` is especially deceptive: it looks like handling the request, and is actually an unbounded queue of tasks with no admission control at all.
- **How to handle it in production, and why that works:** Trace the path of a single request through every buffer and hand-off, and check each one for a bound and a full-behavior. Replace per-request `spawn` with a bounded worker pool or a `Semaphore` guard. For async accept-then-process APIs, the bound moves to the *durable* queue (and its depth becomes the thing you alert on). Where work legitimately fans out, cap the fan-out width. The audit is mechanical and usually finds two or three holes.
- **Trade-offs of the fix:** Full end-to-end backpressure makes the slowest stage govern the whole pipeline's throughput — visible and sometimes unwelcome, but it *is* the truth about your capacity; the alternative was hiding it in a queue until it became an outage.

### Pitfall: Rate limits at the wrong granularity

- **What goes wrong:** A single global limit (`10 000 req/s total`) lets one aggressive tenant consume everything and starve everyone else — the noisy-neighbor problem, which a global limit is structurally unable to see. Or the opposite: per-user limits with no global cap, so a legitimate surge across many users saturates the system while every individual limit reads as fine.
- **Why it happens (the mechanism):** A limit protects the dimension it counts. A global limit protects the system from *total* overload but says nothing about distribution; a per-tenant limit enforces fairness but says nothing about aggregate capacity. Systems typically implement one and discover the other's absence during an incident.
- **How to handle it in production, and why that works:** Layer them: a **global** limit (or better, an adaptive concurrency limit) protects the system; **per-tenant/per-key** limits enforce fairness and contracts; **per-endpoint** limits protect specific expensive resources. Add **weighted costs** where request expense varies wildly (a search query costing 10 tokens and a health check costing 1) — otherwise a "1000 req/min" limit is meaningless across a heterogeneous API. In multi-tenant systems, prefer *fair queuing* over hard per-tenant caps: idle tenants' unused capacity is available to others until contention appears, at which point everyone converges to a fair share.
- **Trade-offs of the fix:** Multiple limit layers mean more configuration, more ways to be surprised by a rejection, and a real observability requirement (which limit rejected this request, and why?). Weighted costs need per-endpoint cost estimates that drift as code changes.

### Pitfall: Serving dead work

- **What goes wrong:** Under load, a request waits 8 seconds in a queue, is finally processed, and the response goes to a client that timed out at 3 seconds and has already retried twice. The system is at 100% CPU with a 0% useful-response rate — every unit of capacity spent on work whose outcome nobody will observe.
- **Why it happens (the mechanism):** Queues have no notion of the client's patience. Nothing in a FIFO tells a worker that the item it's about to process is stale, so the system's throughput is entirely spent on the *oldest*, i.e. most-likely-abandoned, requests. This is also why FIFO is the worst discipline under sustained overload — it systematically prioritizes the requests least likely to still be wanted.
- **How to handle it in production, and why that works:** Propagate **deadlines** (not per-hop timeouts) and check them *before* starting work and again before expensive sub-steps — a check that costs nanoseconds and can free enormous capacity during an incident. Consider **LIFO under overload** (serve freshest first, fail the rest fast): counterintuitive, but under sustained overload LIFO delivers successful responses where FIFO delivers none. Cancel in-flight work when the client disconnects (in Rust, dropping the future does this automatically for well-structured async code — [cancellation](../../performance-optimization/async-and-io/learning.md) working in your favor for once).
- **Trade-offs of the fix:** LIFO is unfair — some requests starve indefinitely under continuous overload (mitigate with a max-age eviction so starved items are failed explicitly rather than left forever). Deadline propagation requires plumbing through every service and a convention everyone honors.

### Pitfall: Load-testing only the happy path

- **What goes wrong:** Load tests establish that the system handles 200 req/s. They never test 400 req/s, so nobody knows whether the system *degrades* or *collapses* past capacity — and those are wildly different outcomes that look identical at 200 req/s. The first observation of overload behavior happens in production, during the incident.
- **Why it happens (the mechanism):** Load testing is usually framed as "find the capacity number," so tests stop at the point of interest. But the valuable information is on the other side of the knee: does throughput plateau (good) or *fall* (retry amplification, thrashing)? Does the system recover when load drops (good) or stay collapsed (metastable — the failure mode you most need to know about)?
- **How to handle it in production, and why that works:** Test past the knee deliberately: ramp to 2–3× capacity, hold, then drop back to 50% and measure **time to recovery**. That last measurement is the one that finds metastable failure, and it's the one almost nobody runs. Chart throughput and p99 against offered load to find the actual knee (not the target). Include realistic client behavior — timeouts and retries — because a load generator that patiently waits forever cannot reproduce the amplification loop that causes real outages ([coordinated omission](../../performance-optimization/async-and-io/learning.md) again, from the other side).
- **Trade-offs of the fix:** Overload testing is destructive and needs an environment where that's acceptable, plus discipline to run it periodically rather than once. It's still cheaper than learning the answer at 3 a.m.

## Design Decisions & Trade-offs

**Concurrency limits for internal dependencies; rate limits at trust boundaries.** Concurrency limits self-adjust as latency changes (Little's Law does the work) and are the right default for protecting a database, a downstream service, or a thread pool. Rate limits enforce *contracts* — per-API-key quotas, per-tenant fairness, abuse prevention — where you need a number a customer can be told and a limit that doesn't depend on cooperation.

**Every bound derives from a stated latency budget.** `bound = throughput × acceptable_latency` turns an arbitrary number into a decision someone can review. Write the budget down; a queue depth with no budget behind it is a guess that will be wrong in both directions.

**Fail fast beats queueing when you're over capacity.** A 503 in 2 ms lets the client retry elsewhere, back off, or degrade — all useful. A 30-second wait ending in a timeout gives the client nothing and costs you a slot. The instinct to "at least try" is exactly backwards under sustained overload.

**Shed by value, not by arrival order.** Priority tiers with reserved capacity mean a system at 40% of needed capacity still serves 100% of checkout traffic. This requires classifying endpoints by business value — a product conversation, not an engineering one, and worth having before the incident rather than during.

**Adaptive limits when the manual ones oscillate.** Static limits are fine when capacity is stable and known. The signal to go adaptive is a config value that gets edited after incidents in both directions — that's a human control loop, and machines run control loops better.

**Backpressure is end-to-end or it is nothing.** One unbounded hop absorbs all the pressure the rest of the chain carefully propagated. The audit — trace a request through every buffer, spawn, and hand-off — is a half-day exercise that reliably finds real holes.

**These patterns compose with the rest of the resilience family.** Rate limiting caps input; [circuit breakers](../circuit-breaker/learning.md) stop calling a dependency that's failing; [bulkheads](../circuit-breaker/learning.md) contain a failure to one resource pool; [caching](../caching-strategies/learning.md) reduces the load that reaches the origin at all; timeouts and deadlines bound how long anything may consume. Deploying one without the others leaves an obvious gap — most commonly rate limiting without retry budgets, which shifts the amplification to the client side without reducing it.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State Little's Law and use it twice: derive the wait time for a 500-item queue drained at 250/s, and derive the correct queue bound for a 1-second latency budget at 250/s.
2. Why does wait time scale as `ρ/(1−ρ)` rather than linearly, and what does that imply about targeting 95% utilization in capacity planning?
3. Distinguish backpressure, rate limiting, and load shedding by *mechanism* and by *where each is appropriate*. Why can't backpressure alone protect a public API?
4. A dependency's latency goes from 50 ms to 500 ms. Trace what a 200/s rate limit does versus a 20-slot concurrency limit, and explain why one is self-correcting.
5. Define metastable failure. Walk the retry-amplification loop that produces it, and name the three mechanisms that make it unreachable.
6. Why is FIFO the worst queue discipline under sustained overload? What's the counterintuitive alternative, what does it buy, and what does it cost?
7. Your pipeline is bounded everywhere except one `tokio::spawn` per request. Explain precisely what that does to end-to-end backpressure.
8. Design a limit layering for a multi-tenant API where request costs vary 100×. Name each layer, what it protects, and how weighted costs enter.

Design exercises:

- Audit one real service: trace a request through every queue, channel, pool, and spawn; record each one's bound and full-behavior. Unbounded hops found is the score — most services score 2–4.
- Compute the correct bound for one of them from a stated latency budget, change it, and observe rejection metrics under a load test at 2× capacity.
- Run the recovery test: ramp to 3× capacity, hold two minutes, drop to 50%, and measure time-to-recovery. If it doesn't recover, you've found a metastable failure in a controlled setting — the best possible place to find one.

## Open Questions

- Adaptive concurrency in Rust: what's the current state of `tower`'s load-shed/concurrency-limit layers versus a port of Netflix's gradient algorithm — is anything production-grade off the shelf?
- Deadline propagation over plain HTTP: is there a converging convention (a `Deadline`/`X-Request-Deadline` header) or does everyone hand-roll it outside gRPC?
- Fair queuing implementations for multi-tenant APIs: what do real systems use (stochastic fair queuing? deficit round robin?) and what's the per-request cost at edge scale?
- LIFO-under-overload in practice: which production systems actually ship it, and how do they bound the starvation it introduces?
- Measuring the knee automatically: can utilization-vs-latency curves be fitted continuously in production to detect capacity drift after deploys, rather than rediscovered by load test?

## References

- Marc Brooker's blog ([brooker.co.za](https://brooker.co.za/blog/)) — the clearest practical writing on queueing, load shedding, and retry behavior from someone who operates it at AWS scale; search "backpressure," "retries," and "metastable."
- Bronson et al., ["Metastable Failures in Distributed Systems"](https://sigops.org/s/conferences/hotos/2021/papers/hotos21-s11-bronson.pdf) (HotOS 2021) — the paper that named the phenomenon; short, and it will change how you read incident reports.
- Amazon Builders' Library, ["Timeouts, retries, and backoff with jitter"](https://aws.amazon.com/builders-library/timeouts-retries-and-backoff-with-jitter/) and ["Using load shedding to avoid overload"](https://aws.amazon.com/builders-library/using-load-shedding-to-avoid-overload/) — operational doctrine, concretely argued.
- Netflix, ["Performance Under Load"](https://netflixtechblog.medium.com/performance-under-load-3e6fa9a60581) — the adaptive concurrency-limits work, including why static limits fail.
- Kingsbury/Jepsen and the queueing-theory basics in any operations-research text for `ρ/(1−ρ)` — the math is simple and worth deriving once by hand.
- Related topics in this repo: [Circuit Breaker](../circuit-breaker/learning.md) (the companion — stop calling what's already failing), [Caching Strategies](../caching-strategies/learning.md) (protecting the origin on the miss path), [Async & I/O](../../performance-optimization/async-and-io/learning.md) (bounded channels, cancellation, coordinated omission), [Batching & Amortization](../../performance-optimization/batching-and-amortization/learning.md) (buffer bloat and Little's Law from the throughput side), [Idempotency & Delivery Semantics](../idempotency-and-delivery-semantics/learning.md) (what makes a retry safe at all), [Saga Pattern](../saga-pattern/learning.md) (timeouts and stuck work at workflow scale).
