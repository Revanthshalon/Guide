# Load Balancing & Service Discovery — Learning Notes

## Mental Model

**Two questions, always paired: *which instances exist and are healthy?* (discovery) and *which one gets this request?* (balancing). The second is only as good as the first — most load-balancer pathologies are actually stale-membership or bad-health-signal problems wearing a routing costume.**

In a static world you configure an IP and forget it. In any modern deployment — autoscaling, rolling deploys, preemptible nodes, containers rescheduled by the minute — the instance set changes continuously, so both questions must be answered *continuously, by machines, with information that is always slightly out of date.*

That staleness is the defining constraint, and it produces the topic's central insight: **load balancing is a distributed decision made with stale information.** No balancer knows the true current load of each backend — by the time a metric arrives, it's history, and the balancer's own decisions are changing the thing it's measuring. Every algorithm is therefore a heuristic for "which instance is least loaded?" answered without the answer.

The most useful result in the field falls straight out of that framing. **The power of two random choices**: instead of picking a backend at random (poor), or querying all of them to find the true minimum (expensive, and stale anyway), pick *two at random and use the less loaded one*. For n requests across n backends, purely random assignment leaves a maximum load of roughly `log n / log log n`; two choices reduces it to about `log log n` — an exponential improvement from one extra sample. That single trick is why "P2C" is the default in Envoy, Finagle, and most modern balancers: near-optimal distribution with no global state and no coordination.

Two framings that prevent common category errors:

1. **Load balancing assumes instances are interchangeable; [sharding](../sharding/learning.md) assumes they are not.** If any instance can serve any request, you're balancing. If a request *must* reach a specific instance because that instance owns the data, you're routing to a shard — a different problem with different failure modes. Confusing them produces "load balancers" that are really shard routers with no rebalancing story.
2. **Health checking is a distributed failure detector**, and inherits the impossibility from [consensus](../consensus-and-leader-election/learning.md): you cannot distinguish "dead" from "slow" or "unreachable from here." Health checks are therefore always a *tuning* problem between reacting too slowly (routing to dead instances) and reacting too eagerly (ejecting healthy ones during a blip — and, in the worst case, ejecting *all* of them).

## Core Concepts

### Service discovery mechanisms

- **What it is:** How a caller learns the current instance set. Four families: **DNS** (universal, but TTL-bounded staleness and clients that cache far longer than they should); **a registry** (Consul, etcd, ZooKeeper — instances register and heartbeat; clients watch for changes); **platform-native** (Kubernetes `Service`/`EndpointSlice` — the control plane maintains membership from readiness probes, and `kube-proxy` or a CNI programs the data path); **service mesh** (a sidecar receives endpoint updates from a control plane and handles both discovery and balancing outside your process).
- **Why it exists:** Each trades propagation speed against operational weight. DNS needs nothing new but propagates in tens of seconds and is routinely cached wrongly by clients (a JVM caching DNS forever is a classic outage). A registry propagates in ~a second and is [consensus-backed](../consensus-and-leader-election/learning.md), at the cost of running it. Platform-native is free if you're already on Kubernetes and is the default answer there. The mesh gives the richest behavior with the heaviest operational surface.
- **Example:** The failure this concept exists to prevent: an instance is terminated, the registry updates in 1 s, but a client's DNS cache holds the old IP for 300 s — five minutes of connection errors that look like a network problem and are actually a staleness problem. This is why **deregistration should precede shutdown** (see graceful drain below) rather than relying on discovery to notice a corpse.

### Health checks: liveness, readiness, and depth

- **What it is:** **Liveness** — "is this process alive?" (failing means *restart me*). **Readiness** — "should I receive traffic right now?" (failing means *route around me*, without restarting). **Active** checks are polled by the balancer; **passive** checks (outlier detection) infer health from real request outcomes.
- **Why it exists:** The distinction is load-bearing and constantly conflated. A shallow liveness check that returns 200 whenever the process is up will happily keep a broken instance in rotation (the process lives; the database connection pool is exhausted). A *too-deep* readiness check that queries every downstream dependency does something far worse: when a shared dependency blips, **every instance fails readiness simultaneously and the entire service is removed from rotation** — a total outage caused by the health check, not the dependency.
- **Example:** The workable split: liveness = "the event loop is responsive" (cheap, local, no dependencies — restarting is the correct response to it failing); readiness = "my own resources are usable" (my connection pool has capacity, my caches are warm, I'm not shutting down) but **not** "my dependencies are healthy" (that's the [circuit breaker's](../circuit-breaker/learning.md) job, per-dependency, and it shouldn't remove *me* from rotation). Passive outlier detection then catches what active checks miss — an instance returning errors to real traffic while cheerfully passing `/health`.

### Balancing algorithms

- **What it is:** The choice function. **Round-robin** (rotate — ignores load entirely); **random** (stateless, poor tail); **least-connections / least-request** (send to the fewest in-flight — a good proxy for load, and self-correcting when instances differ in speed); **latency-aware / EWMA** (weight by observed response time); **power-of-two-choices** (sample two, pick the better — near-optimal without global state); **consistent hashing** (route by key, for cache affinity — see below); **weighted** variants of all of the above (for heterogeneous instances or canary traffic).
- **Why it exists:** Round-robin's flaw is that it assumes all requests and all instances are equal. One instance running on a noisy neighbor, one request that takes 100× longer, one instance with a cold cache — round-robin keeps feeding them at the same rate and they accumulate a backlog. **Least-request is the single biggest upgrade** because in-flight count is an *automatically correct* signal: a slow instance holds requests longer, so its count rises, so it receives less — a control loop with no configuration. P2C then delivers most of least-request's benefit with O(1) work and no shared counters, which is why it's the modern default.
- **Example:** 20 backends, one of them 5× slower (degraded disk). Round-robin sends it 5% of traffic and its queue grows without bound — p99 for the whole service becomes that instance's latency. Least-request naturally converges to sending it ~1% of traffic. Nobody configured anything; the algorithm noticed.

### Consistent hashing for affinity (the exception to interchangeability)

- **What it is:** Routing by a request key (`user_id`, cache key) so the same key reaches the same backend, using a [hash ring](../sharding/learning.md) so topology changes remap only ~1/N of keys.
- **Why it exists:** Interchangeable instances are the ideal, but sometimes locality is worth real money — a backend holding a warm in-process cache for a key set, a sticky session, a connection to a stateful upstream. Consistent hashing buys that affinity while keeping the *balancing* property (roughly even distribution) and bounded disruption when instances come and go.
- **Example:** A cache tier where each node holds a subset of hot entries: hashing by cache key gives high hit rates per node. The cost is that key popularity skew becomes *instance* skew (the [hot-shard problem](../sharding/learning.md) again), which is why implementations add **bounded-load consistent hashing** — spill to the next node on the ring once a node exceeds a load factor, trading a little affinity for a cap on imbalance.

### Where the balancer lives

- **What it is:** **L4 (transport)**: routes connections by IP/port — fast, protocol-agnostic, and unable to see requests. **L7 (application)**: parses HTTP/gRPC — per-request routing, retries, header-based rules, at higher cost. **Client-side**: the caller holds the endpoint list and chooses — no extra hop, no shared bottleneck, but every client language needs the logic. **Sidecar/mesh**: client-side behavior in a separate process — language-agnostic, uniform policy, plus a proxy per pod to operate.
- **Why it exists:** The decisive question is usually **HTTP/2 and gRPC**, because they multiplex many requests over one long-lived connection. An L4 balancer distributes *connections*; with one connection per client, all of that client's requests pin to a single backend forever — the balancing silently stops working, and adding backends changes nothing. This is the most common "our load balancer isn't balancing" incident, and it requires an L7/gRPC-aware balancer (or client-side/mesh balancing) to fix.
- **Example:** A gRPC service behind an L4 load balancer with 10 backends: 3 clients hold 3 connections, so 3 backends receive 100% of traffic and 7 sit idle. The dashboards show a healthy fleet average and a terrible p99, which is exactly the signature to recognize.

### Graceful drain and slow start (the two lifecycle edges)

- **What it is:** **Drain**: on shutdown, first deregister/fail readiness, *then* wait for in-flight requests to complete (and for the balancer's view to update), *then* exit. **Slow start**: a newly added instance receives gradually increasing traffic rather than its full share immediately.
- **Why it exists:** Both edges are where deploys turn into incidents. Without drain, a rolling deploy kills instances that the balancer still believes are healthy — every deploy produces a burst of connection errors, which teams normalize as "deploy noise" rather than fixing. Without slow start, a cold instance (empty caches, unwarmed connection pools, cold JIT/branch predictors) is handed its full share instantly — and under *least-request* balancing it's handed **more** than its share, because zero in-flight requests makes it look like the least-loaded backend in the fleet. The cold instance is buried at the exact moment it's slowest.
- **Example:** The drain sequence that works: SIGTERM → readiness starts failing → wait ~2× the balancer's check interval (so the balancer definitely stopped sending) → stop accepting new connections → wait for in-flight to complete up to a timeout → exit. Kubernetes: a `preStop` sleep plus `terminationGracePeriodSeconds` longer than the drain, because the pod is removed from endpoints *and* sent SIGTERM concurrently — the sleep is what covers the propagation gap.

## Worked Example

A gRPC service: 20 instances, 10 K req/s, behind an L4 network load balancer. Four problems surface in sequence; each fix reveals the next.

**1. The balancing isn't balancing.**

```
observed: 4 instances at 85% CPU, 16 at 6%. Fleet average looks fine (22%).
cause:    L4 balances CONNECTIONS; gRPC multiplexes all requests over one
          long-lived connection per client. ~24 client connections → pinned
          to ~4 backends. Adding instances changes nothing.
fix:      L7/gRPC-aware balancing — an Envoy/mesh sidecar, or client-side
          balancing with the round_robin gRPC policy over resolved endpoints.
```

**2. Round-robin meets a degraded instance.**

```
instance-7's disk degrades: p99 goes 40 ms → 900 ms
round-robin keeps sending it 1/20 of traffic; its queue grows
service p99 = instance-7's p99 (5% of requests, but they're the tail)
fix:      least-request → in-flight count rises on instance-7 → it receives
          proportionally less, automatically. Then P2C for O(1) selection.
```

**3. A deploy adds instances and latency spikes.**

```
new instance joins with 0 in-flight → least-request sees "least loaded"
→ it receives a DISPROPORTIONATE burst while its caches are cold
→ its latency spikes → in-flight climbs → traffic swings away → oscillation
fix:      slow start — ramp the new instance's weight over ~60 s so it warms
          before carrying a full share.
```

Note the irony worth internalizing: the algorithm that fixed problem 2 *caused* problem 3. Least-request's signal (in-flight count) is a proxy for load that reads a cold instance as an idle one.

**4. A dependency blip removes the entire service.**

```
14:02  the shared metadata DB has a 20 s hiccup
14:02  every instance's /ready probe (which queries that DB) fails
14:02  ALL 20 instances marked unhealthy → removed from rotation
14:02  100% of traffic fails — a 20 s dependency blip became a full outage
fix:   readiness checks OWN resources only; dependency health belongs to
       per-dependency circuit breakers, which degrade the FEATURE, not the
       INSTANCE. Plus a balancer-level panic threshold: if >50% of the fleet
       is unhealthy, ignore health status and load-balance across everything
       (Envoy calls this "panic mode") — because when everything looks broken,
       the health signal is more likely wrong than the fleet.
```

**The final configuration:**

```
discovery:  platform endpoints (Kubernetes EndpointSlice), ~1 s propagation
balancer:   L7 sidecar, P2C over least-request, zone-aware preference
health:     liveness = event loop responsive (restart on fail)
            readiness = own pool capacity + not-draining (route around on fail)
            passive = outlier ejection on consecutive 5xx, max 30% ejected
lifecycle:  slow start 60 s; drain = fail readiness → 10 s → stop accepting →
            finish in-flight → exit
safety:     panic threshold at 50% unhealthy
```

Every line traces to one of the four incidents.

## Pitfalls in Depth

### Pitfall: Health checks that ejaculate the whole fleet (deep readiness probes)

- **What goes wrong:** The readiness endpoint checks the database, the cache, and two downstream services "to be thorough." A shared dependency has a brief problem; every instance fails readiness at the same instant; the balancer removes all of them; the service is 100% down for a dependency degradation that would have caused partial failures at worst.
- **Why it happens (the mechanism):** Deep health checks encode a category error — they conflate *"is this instance able to serve?"* with *"is the whole system healthy?"* Since all instances share the same dependencies, a deep check makes every instance's health perfectly correlated, which is precisely the opposite of what you want from a fleet. The failure is invisible in normal operation and catastrophic in exactly the scenario health checks exist for.
- **How to handle it in production, and why that works:** Readiness answers only about *this instance's own resources*: connection pool has capacity, local caches initialized, not currently draining. Dependency problems belong to [circuit breakers](../circuit-breaker/learning.md), which degrade a *feature* rather than removing a *server*. Add a **panic threshold** at the balancer (Envoy's `healthy_panic_threshold`, typically 50%): when more than half the fleet appears unhealthy, ignore health status and balance across everything — reasoning that a signal implicating the entire fleet is more likely broken than true. Finally, make health checks cheap enough that checking frequently is free, so detection is fast without hammering dependencies.
- **Trade-offs of the fix:** A shallow readiness check will occasionally keep an instance in rotation that's technically able to accept requests but will fail them — which is what passive outlier detection is for. And panic mode deliberately sends traffic to possibly-unhealthy backends; that's correct when the alternative is sending traffic nowhere.

### Pitfall: L4 balancing in front of HTTP/2 or gRPC

- **What goes wrong:** Connection-level balancing with request-level multiplexing: a handful of long-lived connections pin to a handful of backends, and the rest of the fleet idles. Adding capacity has no effect. Averages look healthy; p99 is terrible; CPU distribution is wildly uneven.
- **Why it happens (the mechanism):** The balancer is doing exactly what it was designed to do — distributing *connections* evenly. HTTP/1.1 with short-lived connections made that a good proxy for distributing requests; HTTP/2 and gRPC broke the proxy by multiplexing thousands of requests over one connection that, once established, is never rebalanced.
- **How to handle it in production, and why that works:** Use L7 balancing that understands streams (Envoy, an L7 cloud balancer, or a mesh sidecar), or move the decision **client-side** (gRPC's built-in load balancing over a resolved endpoint list, which is why gRPC ships with a name resolver and LB policy interface). If you're stuck with L4, the mitigations are ugly but real: cap connection lifetime (`max_connection_age` with jitter, forcing periodic redistribution) and open multiple connections per client. The connection-age trick is what makes L4 tolerable for gRPC in practice.
- **Trade-offs of the fix:** L7 costs more CPU per request and terminates TLS at the proxy. Client-side balancing means every client language needs a correct implementation and clients must receive endpoint updates — which is a good chunk of what a mesh sells you.

### Pitfall: Cold instances hammered by least-request

- **What goes wrong:** A new instance joins after a deploy or scale-out. Its in-flight count is zero, so least-request/P2C rates it the *least loaded* backend in the fleet and directs a disproportionate share of traffic to it — while its caches are empty, its connection pools unwarmed, and (in JIT runtimes) its code uninterpreted. Latency spikes, sometimes badly enough to fail health checks and get it ejected, restarted, and re-cold: a deploy that never converges.
- **Why it happens (the mechanism):** In-flight count is a proxy for load, and the proxy inverts at exactly one moment: for a cold instance, low in-flight means *not yet warm*, not *idle capacity*. The better the balancer is at finding idle capacity, the harder it hits the coldest instance.
- **How to handle it in production, and why that works:** **Slow start** — ramp the new endpoint's weight from near-zero to full over a warmup window (Envoy `slow_start_config`, typically 30–120 s), so the instance warms under a load it can handle. Pair with a **readiness gate that means warm** (pools pre-established, critical caches primed) rather than merely "process started," and pre-warm in the startup path where feasible. For deploys specifically, this is also an argument for smaller rolling batches: fewer simultaneously-cold instances.
- **Trade-offs of the fix:** Slow start delays a new instance's contribution, which matters when scaling out *because* you're overloaded — the ramp must be fast enough to help in time. And a warmth-aware readiness gate lengthens deploys.

### Pitfall: Stale membership and ungraceful shutdown

- **What goes wrong:** Every deploy produces a burst of connection resets and 502s. It's normalized as "deploy noise." The cause is that instances are killed while the balancer still believes they're healthy — the balancer's view lags reality by its health-check interval, and shutdown didn't wait for that lag.
- **Why it happens (the mechanism):** Discovery is eventually consistent, and shutdown is usually written as "receive SIGTERM, close, exit." Kubernetes makes it easy to get wrong: endpoint removal and SIGTERM are delivered *concurrently*, so a pod that exits promptly on SIGTERM disappears before the endpoint update has propagated to every client and proxy.
- **How to handle it in production, and why that works:** Order the shutdown so the balancer learns *first*: fail readiness (or deregister) → wait at least 2× the check interval / propagation delay → stop accepting new connections → drain in-flight up to a timeout → exit. On Kubernetes that's a `preStop` sleep with `terminationGracePeriodSeconds` set comfortably above the total. Verify by watching error rates during a deploy: a correct drain shows **zero** connection errors, and "some errors during deploy" should be treated as a bug rather than weather.
- **Trade-offs of the fix:** Deploys take longer by the drain duration per batch. In exchange, deploys stop being a source of user-visible errors — which also removes the alert-fatigue that makes real deploy incidents easy to miss.

### Pitfall: Retries and balancing amplifying each other

- **What goes wrong:** A backend starts failing. Clients retry — and depending on configuration, retry *to the same instance* (useless), or retry across the fleet fast enough to multiply total load exactly when capacity is reduced, pushing healthy instances over the edge. The failure spreads from one instance to the fleet.
- **Why it happens (the mechanism):** The balancer and the retry policy are usually configured independently, in different layers, and neither knows about the other. Meanwhile removing a failing backend *concentrates* its traffic on the survivors — so an ejection during a capacity-constrained moment can itself trigger the next ejection, cascading through the fleet.
- **How to handle it in production, and why that works:** Retries must **select a different host** (Envoy and gRPC support host-selection retry predicates) — a retry to the same failing instance is guaranteed waste. Bound amplification with a [retry budget](../backpressure-and-rate-limiting/learning.md) (≤10–20% of traffic) rather than per-request retry counts. Cap outlier ejection (`max_ejection_percent`, e.g. 30%) so the fleet can never eject itself into an outage, and make ejection duration exponential so flapping instances aren't re-admitted immediately. And size capacity with N+2 headroom, since ejection redistributes load rather than removing it.
- **Trade-offs of the fix:** Retry budgets mean some transient failures surface to users instead of being retried away — correct, given the alternative. Ejection caps mean genuinely-bad instances stay in rotation once the cap is hit; that's the right trade when the alternative is ejecting everything.

### Pitfall: Balancing something that should be routed (or vice versa)

- **What goes wrong:** Requests are round-robined across instances that hold *per-instance state* — in-memory sessions, a local cache the request depends on, a WebSocket-bound client. Users see randomly inconsistent behavior depending on which instance answers. The mirror image: sticky sessions used where instances are interchangeable, producing permanent imbalance because the sticky keys are unevenly popular and never rebalance.
- **Why it happens (the mechanism):** [Sharding and balancing](../sharding/learning.md) are opposite assumptions — interchangeable vs. not — and the same infrastructure component implements both. Stickiness is often adopted as a quick fix for a state problem, converting a stateless service into a stateful one without anyone deciding to.
- **How to handle it in production, and why that works:** Make instances genuinely interchangeable wherever possible: sessions in a shared store (Redis) or in signed tokens, caches treated as [per-instance optimizations rather than correctness dependencies](../caching-strategies/learning.md). Where affinity earns real value (a large warm cache), use **bounded-load consistent hashing** rather than naive stickiness, so a hot key can spill and the imbalance has a ceiling. Where state genuinely belongs to a specific instance (stateful streaming, in-memory game sessions), acknowledge you're doing shard routing and adopt its machinery: an explicit ownership map, rebalancing, and a handoff story.
- **Trade-offs of the fix:** Externalizing session state adds a lookup per request and a dependency. Bounded-load hashing sacrifices some hit rate for balance. Both beat a stateless service that is accidentally stateful.

## Design Decisions & Trade-offs

**Use the platform's discovery unless you have a reason not to.** Kubernetes endpoints, ECS service discovery, or a cloud load balancer are already integrated with health, deploys, and scaling. A standalone Consul/etcd registry is worth it when you're multi-platform or need richer metadata — and it's a [consensus system](../consensus-and-leader-election/learning.md) you must then operate.

**Algorithm defaults that are hard to beat:** P2C over least-request, with slow start for new endpoints and passive outlier detection on top. Round-robin only where requests are genuinely uniform and instances identical (rare). Consistent hashing only when affinity has a measured payoff, and then bounded-load.

**The health-check split is the highest-leverage decision in this doc:** shallow liveness (restart-worthy, dependency-free), own-resources readiness (route-around-worthy), per-dependency circuit breakers (feature-degradation-worthy). Getting this wrong turns dependency blips into total outages, and it's wrong by default in a lot of code.

**Client-side vs. proxy vs. mesh.** Client-side is fastest (no hop) and costs a correct implementation per language. A central L7 proxy is simplest to operate and is a shared failure domain plus an extra hop. A mesh is client-side behavior with central control at the cost of a sidecar per pod and a control plane. Pick by how many languages you have and how much uniform policy you need — not by fashion.

**Zone-aware routing is nearly free money in multi-AZ deployments:** prefer same-zone backends (lower latency, no cross-AZ data charges) with automatic spillover when the local zone lacks capacity or health. The trap is *insufficient* spillover logic — a zone with two unhealthy instances must overflow rather than hammer the remaining one.

**Capacity plan for ejection, not just for load.** Losing 20% of the fleet redistributes that traffic onto the rest; if the remainder is at 80% utilization, you've just pushed it past [the knee](../backpressure-and-rate-limiting/learning.md) and the ejection cascades. N+2 and a `max_ejection_percent` cap are the two defenses.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the power-of-two-choices result and explain why one extra sample buys an exponential improvement over random. Why is this preferable to querying all backends?
2. A gRPC service behind an L4 balancer has 4 of 20 instances at 85% CPU. Diagnose it, name the mechanism, and give three fixes (one proper, two mitigations).
3. Distinguish liveness from readiness by *what action failure should trigger*. Then explain how a "thorough" readiness probe produces a total outage from a partial dependency failure.
4. Why does least-request balancing *cause* the cold-start problem it seems unrelated to? What's the fix, and what does that fix cost when you're scaling out under load?
5. Write the correct shutdown sequence and justify each wait. Why is "some errors during deploys" a bug rather than weather?
6. Why does ejecting an unhealthy instance risk cascading? Name the two configuration defenses and the capacity-planning rule.
7. When is consistent hashing the right choice, what problem does it import from the sharding doc, and what's the standard bound on that problem?
8. Explain the panic threshold. Why is "ignore health checks when >50% look unhealthy" a sound default rather than a hack?

Design exercises:

- Audit one service: what discovery mechanism, what balancing algorithm, what exactly do liveness and readiness check, is slow start configured, and what's the drain sequence? The readiness probe is where most audits find a latent total-outage.
- Run a deploy while watching error rate at 1-second resolution. Any non-zero blip is a drain bug — find whether it's ordering, propagation delay, or in-flight abandonment.
- Simulate a degraded instance (add 500 ms latency to one backend with `tc netem`) and observe how much traffic it keeps receiving under your current algorithm. That number is your algorithm's quality, measured.

## Open Questions

- Bounded-load consistent hashing in practice: which proxies implement it (Envoy's `maglev`/`ring_hash` with load factor?) and what load factor works for a cache tier with power-law key popularity?
- gRPC client-side LB in Rust: what does tonic actually support today for endpoint resolution and P2C, and how does it compare to putting a sidecar in front?
- Health-check frequency vs. detection latency: is there a principled way to derive the interval from the deploy cadence and acceptable error budget, or is it universally hand-tuned?
- Zone-aware routing spillover algorithms: how do real implementations decide when to cross zones, and how much oscillation does that introduce?
- Do mesh outlier ejection and application circuit breakers conflict in practice when both are enabled, and what's the recommended division?

## References

- Mitzenmacher, ["The Power of Two Choices in Randomized Load Balancing"](https://www.eecs.harvard.edu/~michaelm/postscripts/handbook2001.pdf) — the result and its derivation; the single most useful piece of theory in this topic.
- [Envoy documentation](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/upstream/load_balancing/load_balancing) — load balancing, outlier detection, slow start, and panic mode; the most complete parameter surface available and a de facto design checklist even if you run something else.
- Marc Brooker, ["Load Balancing"](https://brooker.co.za/blog/) posts — especially on why least-connections works and what randomness buys; short and clarifying.
- Google SRE Book, ch. 20 ("Load Balancing in the Datacenter") — subsetting, weighted round robin, and why utilization-based signals mislead.
- Consul / Kubernetes EndpointSlice documentation — the two mainstream discovery models, with their propagation semantics spelled out.
- Related topics in this repo: [Sharding](../sharding/learning.md) (the opposite assumption — and consistent hashing's home), [Circuit Breaker](../circuit-breaker/learning.md) (dependency health belongs there, not in readiness; outlier ejection is instance-level breaking), [Backpressure & Rate Limiting](../backpressure-and-rate-limiting/learning.md) (retry budgets, ejection cascades, capacity headroom), [Consensus & Leader Election](../consensus-and-leader-election/learning.md) (registries, and the failure-detector impossibility health checks inherit), [Caching Strategies](../caching-strategies/learning.md) (affinity's payoff and cold-instance cost).
