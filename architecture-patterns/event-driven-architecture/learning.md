# Event-Driven Architecture — Learning Notes

## Mental Model

**EDA inverts control flow. Instead of "A calls B and waits for an answer," A announces that something happened and whoever cares reacts. The producer does not know its consumers, and that single property is the source of every benefit and every cost.**

In request/response, the caller holds the workflow: it knows who to call, in what order, and what to do when a call fails. The logic is centralized, traceable in a stack trace, and synchronously verifiable. In EDA, the producer's responsibility ends at "I have recorded and announced this fact." Adding a consumer requires no change to the producer — which is the decoupling everyone wants — but it also means **no component knows what the system does as a whole**, and the causal chain that produced any given outcome exists only in the logs of five different services.

The honest framing of the trade:

- **What you buy:** extensibility (new consumers without touching producers), temporal decoupling (the consumer can be down and catch up later), independent scaling per consumer, and a natural fit for domains that *are* event-shaped (orders, payments, IoT, user activity).
- **What you pay:** eventual consistency by default, causality that must be deliberately instrumented, guarantees ([ordering, delivery, exactly-once effects](../idempotency-and-delivery-semantics/learning.md)) that become explicit work rather than language features, and debugging that requires tooling you must build before you need it.

This document is the **system-level view** that organizes the machinery you've already studied piece by piece. The [outbox](../outbox-pattern/learning.md) is how events get published reliably. [Idempotency and delivery semantics](../idempotency-and-delivery-semantics/learning.md) is why consumers must be safe to redeliver. [CDC](../change-data-capture/learning.md) is one way to source events. [Sagas](../saga-pattern/learning.md) are how multi-step workflows survive in this world. [Event sourcing](../event-sourcing/learning.md) is a *storage* decision that often accompanies EDA but is emphatically not the same thing. What this doc adds is the layer above: what kind of events to emit, what broker semantics to build on, and how the whole thing stays comprehensible.

The organizing distinction — Martin Fowler's, and the most useful frame in the field — is **what an event carries**:

1. **Event notification** — thin: "order 991 placed." The consumer calls back for details. Minimal coupling to the producer's data model, maximal runtime coupling (the producer must be up to answer the callback, and you've reintroduced the synchronous dependency you were escaping).
2. **Event-carried state transfer** — fat: the event contains everything a consumer needs. No callback, so consumers work while the producer is down, and each keeps a local replica of what it needs. Costs: bigger payloads, a real schema contract, and data duplication that must stay coherent.
3. **Event sourcing** — the event log *is* the system of record, not merely a notification channel. A far larger commitment; see [its own doc](../event-sourcing/learning.md).

Most production systems should default to **event-carried state transfer** for integration events, because the whole point of asynchrony is not needing the producer right now — and a notification that forces a synchronous callback throws that away.

## Core Concepts

### Events vs. commands (and why the distinction governs coupling)

- **What it is:** An **event** is a statement of fact in the past tense, addressed to nobody: `OrderPlaced`. A **command** is an instruction to a specific recipient: `ShipOrder`. Both may travel over the same broker; they are architecturally opposite.
- **Why it exists:** An event's producer must not care who consumes it — that's what makes adding consumers free. A command *has* a recipient, so the sender knows about the receiver by construction. Mislabeling commands as events (`OrderShippingRequested` published to a topic the shipping service is expected to consume) produces the worst of both: the coupling of RPC with the debuggability of async.
- **Example:** `PaymentCaptured` (event) can be consumed by fulfillment, analytics, fraud, and a future loyalty service — none of which the payment service knows about. `SendReceiptEmail` (command) has exactly one correct handler; routing it through a pub/sub topic gains nothing and loses the clarity of a direct call or a dedicated queue.

### Log-based vs. queue-based brokers (the choice that shapes everything else)

- **What it is:** **Log-based** (Kafka, Redpanda, Pulsar): an append-only, partitioned, *retained* log; consumers track their own offset; many independent consumer groups read the same events; replay is a matter of resetting an offset. **Queue-based** (SQS, RabbitMQ, NATS core): messages are delivered and *acknowledged away*; competing consumers within a queue split the work; there is no history to re-read.
- **Why it exists:** These are not interchangeable products with different logos — they answer different questions. Log-based gives you **replay** (rebuild a projection, onboard a new consumer over historical data, recover from a consumer bug by reprocessing) and **per-partition ordering**. Queue-based gives you **per-message operations** — individual retry, per-message delay, dead-lettering, and fine-grained concurrency — which log-based brokers make awkward because a stuck message blocks its partition's progress.
- **Example:** Choose log-based when consumers are many and evolving, replay has value, or ordering matters per entity. Choose queue-based for work distribution where each message is an independent task (send this email, resize this image, run this job) and per-message retry semantics matter more than history. Plenty of systems run both, deliberately.

### Partitions, ordering, and the key choice

- **What it is:** Log-based brokers partition a topic; **ordering is guaranteed only within a partition**, and the partition is chosen by a key (usually a hash of it). Consumers in a group are each assigned partitions, so partition count also caps consumer parallelism.
- **Why it exists:** Global ordering would require a single serialized log — a throughput ceiling nobody accepts. Per-partition ordering plus a well-chosen key gives you the ordering that actually matters (all events for *one order*, in order) while allowing unlimited parallelism across keys. This is exactly the [shard-key decision](../sharding/learning.md), and it has the same failure modes: a monotonic or low-cardinality key creates a hot partition, and a key that doesn't match the ordering requirement means consumers see effects before causes.
- **Example:** Key `OrderPlaced`/`OrderShipped` by `order_id`: every event for an order lands in one partition, strictly ordered, while different orders spread across all partitions. Key by `event_type` instead and you get 4 partitions regardless of cluster size, three of them idle. And note the cap: 12 partitions means at most 12 useful consumers in a group — partition count is a scaling decision made at topic-creation time and awkward to change (increasing it re-maps keys, breaking ordering across the change).

### Consumer groups, offsets, and lag

- **What it is:** A consumer group is a set of processes cooperatively consuming a topic, with partitions distributed among them; each group tracks its own committed **offset** per partition. **Lag** = latest offset − committed offset: how far behind reality this consumer is. **Rebalancing** reassigns partitions when members join or leave.
- **Why it exists:** Offsets are what make multiple independent consumers possible on one stream, and they're where delivery semantics live: commit *after* processing gives [at-least-once](../idempotency-and-delivery-semantics/learning.md); commit before gives at-most-once. Lag is the single most important operational metric in an event-driven system — it's the direct measure of how stale every downstream projection is, and unlike an error rate it can grow silently for hours.
- **Example:** The classic incident: a consumer slows (a dependency got slower, a deploy added work per message), lag grows at 200 events/s, nobody alerted because there were no errors, and 6 hours later the retention window is about to overtake the committed offset — at which point catching up becomes *losing data*. Alert on lag *and* on lag's derivative, not just on errors.

### Schema contracts and evolution

- **What it is:** The event's payload structure, versioned, with compatibility rules enforced somewhere — a schema registry (Avro/Protobuf/JSON Schema with backward/forward-compatibility modes) or contract tests.
- **Why it exists:** In EDA the producer doesn't know its consumers, which means it also can't coordinate a breaking change with them. The schema is therefore a **published API with unknown clients**, and it must evolve additively: new optional fields fine, renamed/removed/retyped fields break consumers you've never heard of. Registry compatibility modes turn "consumer broke at 3 a.m." into "producer's CI failed" — the same shift-left that [protobuf field numbers](../../performance-optimization/serialization-and-encoding/learning.md) and [event sourcing's upcasters](../event-sourcing/learning.md) provide, applied at the integration boundary.
- **Example:** With backward compatibility enforced, adding `discount_code` (optional) passes; renaming `total` to `total_amount` is rejected at build time. For a genuinely breaking change, the standard move is a **new topic version** (`orders.v2`) with both published during a migration window — because you cannot deploy all consumers atomically, and pretending otherwise is how integration outages happen.

### Dead letter queues and poison messages

- **What it is:** A **poison message** fails processing repeatedly — a malformed payload, a schema the consumer can't parse, a permanent business-rule violation. A **dead letter queue** is where such messages go after N failed attempts so the consumer can continue.
- **Why it exists:** Without a DLQ, a poison message in a log-based system **blocks its entire partition forever** (offset can't advance past it), and in a queue-based system it cycles endlessly, consuming capacity. DLQs convert an availability failure into a triage backlog. The catch that undermines most implementations: **ordering is lost for that key** the moment you skip a message — if `OrderShipped` dead-letters and `OrderDelivered` is then processed, the consumer's state is now built from a subsequence. Whether that's acceptable is a domain question that must be answered per consumer, not assumed.
- **Example:** A workable policy: retry in place with backoff for transient failures (a downstream timeout), dead-letter immediately for permanent ones (unparseable payload, unknown schema version) — distinguishing the two is the consumer's job. Then *monitor DLQ depth as a first-class alert*: an unwatched DLQ is a data-loss queue with extra steps.

## Worked Example

An order flow, designed twice — badly then well — to make the notification-vs-state-transfer choice concrete.

**Design A — event notification (thin events).**

```
order-service:  publish OrderPlaced { order_id: "ord-991" }
fulfillment:    receives → GET /orders/ord-991     → needs order-service UP
email:          receives → GET /orders/ord-991     → needs order-service UP
                        → GET /customers/c-77      → needs customer-service UP
analytics:      receives → GET /orders/ord-991     → needs order-service UP
```

Three consumers, four synchronous callbacks per order. Order-service now receives a **fan-in read spike on every publish** — often larger than the write traffic that caused it. Worse, the temporal decoupling is fictional: if order-service is down, every consumer is blocked, so the async architecture has the availability of the synchronous one it replaced. And a consumer processing yesterday's backlog reads *today's* order state — a subtle correctness bug where the event says "placed" and the callback returns a cancelled order.

**Design B — event-carried state transfer (fat events).**

```json
{
  "event_id": "evt-4711", "event_type": "OrderPlaced", "version": 1,
  "occurred_at": "2026-03-14T10:22:01Z",
  "correlation_id": "req-88f3", "causation_id": "cmd-1201",
  "partition_key": "ord-991",
  "data": {
    "order_id": "ord-991",
    "customer": { "id": "c-77", "email": "ana@example.com", "name": "Ana" },
    "lines": [ { "sku": "A-1", "qty": 2, "unit_price_cents": 1999 } ],
    "total_cents": 3998, "currency": "EUR"
  }
}
```

Every consumer works from the event alone: no callbacks, no read amplification, and — crucially — the data is a **point-in-time fact**, so a consumer replaying last week's events sees last week's truth. Consumers that need more keep their own local projections built from the stream.

The costs are real and worth naming: the payload is a contract (a consumer now depends on `customer.email` being present, so removing it is breaking), data is duplicated across consumers' stores, and PII now travels through the broker and lands in every consumer's storage — which is a [retention and encryption](../encryption-and-key-management/learning.md) question, not just an architecture one.

**Publishing it reliably** — the piece EDA diagrams always omit:

```
order-service, ONE transaction:
  INSERT INTO orders (...)
  INSERT INTO outbox (event_id, key, payload)     ← atomic with the state change
COMMIT
  → relay/CDC tails outbox → publishes to orders topic, keyed by order_id
```

Without this, "save the order and publish the event" is a [dual write](../outbox-pattern/learning.md) that silently loses events on crash. Every EDA rests on this, and it is the most commonly skipped step.

**Consuming it safely:**

```
fulfillment consumer, ONE transaction:
  INSERT INTO processed_events (event_id)   ← unique constraint = inbox dedup
  INSERT INTO shipment_jobs (...)
COMMIT; then commit the offset
```

At-least-once delivery is a given, so the [inbox pattern](../outbox-pattern/learning.md) is mandatory, not optional.

**Adding a consumer later** — the payoff:

```
loyalty-service deployed 8 months later:
  subscribe to orders topic, offset = earliest
  replay 8 months of history to build its point balances
  → catches up, switches to live
  order-service: unchanged, unaware, undeployed
```

That last line is what EDA is *for*. Note it required log-based retention (a queue-based broker has nothing to replay) and self-contained events (thin notifications would have hammered order-service with 8 months of callbacks against *current* state — producing wrong balances).

## Pitfalls in Depth

### Pitfall: The distributed monolith (events used as RPC)

- **What goes wrong:** A service publishes an event and then *waits* for a corresponding response event before continuing — request/response reimplemented over a broker. Now you have synchronous coupling *and* asynchronous complexity: latency is worse, failures are harder to attribute, timeouts are hand-rolled, and there's no stack trace. Related: events named as commands (`ProcessPaymentRequested`) with exactly one legitimate consumer, which is RPC with extra hops.
- **Why it happens (the mechanism):** Teams adopt EDA as a technology decision ("we're using Kafka now") without changing the *design* decision, so existing call graphs get transliterated onto the broker one call at a time. Each individual translation looks reasonable; the aggregate is a monolith whose method calls have become network hops with no transaction, no ordering, and no stack.
- **How to handle it in production, and why that works:** Use the naming test as a design gate: if the message is imperative and has exactly one correct handler, it's a **command** — send it directly (gRPC/HTTP) or to a dedicated queue, and don't pretend it's an event. Events are past-tense facts whose producer would be unaffected if every consumer disappeared. When a workflow genuinely needs multi-step coordination with outcomes, use an explicit [saga/process manager](../saga-pattern/learning.md) that *owns* the state machine — the workflow then has an address and a queryable state, instead of being an emergent property of subscriptions.
- **Trade-offs of the fix:** Keeping synchronous calls synchronous means accepting runtime coupling where it's honest — which is usually correct, and is much better than coupling disguised as decoupling. An explicit orchestrator centralizes what was distributed; that's the point.

### Pitfall: Losing causality (nobody can answer "why did this happen?")

- **What goes wrong:** A customer reports a duplicate charge. There is no way to reconstruct the chain: which request produced which event, which consumer reacted, what it published in turn. Each service's logs are individually fine and collectively useless. Investigations take days and often end in "we couldn't reproduce it."
- **Why it happens (the mechanism):** In request/response, causality is implicit in the call stack and the trace. In EDA, the causal chain is *not recorded anywhere* unless events carry it deliberately — the broker knows only "this event was published at this offset," not what caused it. This is the single largest operational cost of the architecture, and it is entirely preventable at design time and nearly unfixable retroactively (old events can't be given ids they never had).
- **How to handle it in production, and why that works:** Every event carries a **correlation id** (the same value across the entire business flow, propagated from the originating request) and a **causation id** (the id of the specific message that caused *this* one) — [as the event-sourcing doc insists](../event-sourcing/learning.md), and for the same reason. Together they let you reconstruct the full tree: filter by correlation to see everything in a flow, follow causation to walk cause-to-effect. Add distributed tracing with context propagated through message headers (OpenTelemetry supports broker propagation) so the trace spans producer and consumer. Make both mandatory in the event envelope schema, so an event without them can't be published.
- **Trade-offs of the fix:** A few dozen bytes per event and propagation plumbing through every consumer. Trivial next to one multi-day investigation, and it is a *day-one* decision — retrofitting it leaves a permanent blind spot over historical data.

### Pitfall: Assuming ordering that the broker never promised

- **What goes wrong:** A consumer sees `OrderShipped` before `OrderPlaced` and creates a shipment for an order it doesn't know about — or crashes, or silently no-ops on a zero-row update. The events were published in the right order; they arrived out of order because they went to different partitions, or because they came from different *topics* with no ordering relationship at all.
- **Why it happens (the mechanism):** Ordering holds **within a partition, within a topic** — nothing more. Two topics have no mutual ordering; two partitions have no mutual ordering; and a key change (or a partition-count change) moves an entity between partitions, breaking ordering across the transition. Meanwhile the code was written by someone reasoning about a single, globally ordered stream, because that's what the whiteboard diagram showed.
- **How to handle it in production, and why that works:** Key all events for an entity by that entity's id, so its events share a partition and stay ordered — and keep events that must be mutually ordered **in one topic** (splitting `orders` into `orders-placed` and `orders-shipped` destroys the guarantee). Then make consumers defensive anyway: version or sequence-number every event per entity and ignore stale ones; upsert placeholder state on a reference to an unknown entity rather than failing ([the same tolerance the event-sourcing doc prescribes](../event-sourcing/learning.md)); or park-and-retry with monitoring. Never change partition count on a keyed topic without a planned migration.
- **Trade-offs of the fix:** Per-entity keying caps that entity's throughput at one partition's (correct — its ordering *demands* serialization). Defensive consumers carry extra state and code paths that fire rarely, which makes them easy to get wrong and worth testing explicitly.

### Pitfall: Consumer lag discovered as a crisis

- **What goes wrong:** A consumer falls behind — a deploy made it slower, a dependency degraded, a partition got hot. No errors are raised; everything "works." Six hours later a downstream report is wrong, or the retention window is about to pass the committed offset, converting a lag problem into permanent data loss. Alternatively, a rebalance storm (consumers repeatedly joining/leaving because processing exceeds the poll timeout) means the group spends its time rebalancing rather than consuming.
- **Why it happens (the mechanism):** Lag is a *silent* failure: the consumer is healthy, the broker is healthy, requests aren't failing. Only the *staleness* of downstream state is wrong, and nothing measures staleness by default. Teams alert on error rates and CPU, which stay flat while the system drifts further from reality.
- **How to handle it in production, and why that works:** Make lag a primary SLI: alert on **absolute lag** (this projection is >N events stale) and on **lag derivative** (lag is growing — the leading indicator that catches it hours earlier), plus **time-to-drain** as the human-legible version ("at current rate, 4 hours to catch up"). Set retention comfortably longer than your worst realistic outage (a 7-day retention with a consumer that can be down 2 days is fine; 24-hour retention is a tripwire). Prevent rebalance storms by keeping per-message processing well under the poll timeout — offload slow work rather than holding the poll loop ([the async doc's blocking hazard](../../performance-optimization/async-and-io/learning.md) in consumer form).
- **Trade-offs of the fix:** Long retention costs storage — cheap, and it's also what makes replay possible, so it's dual-purpose. Lag alerting needs per-consumer thresholds because acceptable staleness varies (analytics: hours; fraud: seconds).

### Pitfall: Event storms and feedback loops

- **What goes wrong:** Service A's event triggers B, whose event triggers C, whose event triggers A — a cycle that runs until something breaks. Or a batch job publishes 10 million events in minutes and every downstream consumer is buried, including ones that had nothing to do with the batch. Or one legitimate change fans out to hundreds of derived events ("recompute all 400 000 affected prices").
- **Why it happens (the mechanism):** No component sees the whole graph, so cycles form from locally sensible subscriptions added by different teams at different times — the [choreography ceiling](../saga-pattern/learning.md) at system scale. And EDA makes fan-out *free for the producer* while the cost lands entirely on consumers, so nothing in the publishing path pushes back.
- **How to handle it in production, and why that works:** Maintain a **generated topology map** (who produces what, who consumes what — derived from code or configuration and rendered in CI) so cycles are visible before they're deployed; make adding a subscription a reviewable change against that map. Rate-limit or batch bulk publishing paths ([backpressure](../backpressure-and-rate-limiting/learning.md) applies to producers too), and give backfills their own topic so they can't drown live traffic. Add a hop-count/TTL field to the envelope so runaway chains terminate and alert rather than looping indefinitely.
- **Trade-offs of the fix:** A topology map is only as good as the discipline maintaining it — generating it from code beats hand-drawing. Separate backfill topics mean consumers must handle two sources (usually the same handler, different priority).

### Pitfall: EDA applied where request/response was correct

- **What goes wrong:** Every interaction becomes an event, including "get the user's current permissions" and "validate this coupon" — operations whose caller needs an answer *now*. The result is either a fake-async round trip (the distributed-monolith pitfall) or a system where simple queries have become impossible and every screen reads from an eventually-consistent projection that's sometimes wrong.
- **Why it happens (the mechanism):** EDA gets adopted as an architectural identity rather than a tool, so "should this be an event?" stops being asked. The pattern's real domain — facts that others may care about, where the producer shouldn't wait — is narrower than enthusiasm suggests.
- **How to handle it in production, and why that works:** Keep the test explicit: **queries stay synchronous** (the caller needs an answer and can't proceed without it); **commands with one handler stay direct**; **facts others may react to become events**. Most systems are healthiest as a hybrid: synchronous request/response for reads and user-facing commands, events for propagating state changes and triggering side effects. Consistency requirements decide too — anything needing read-your-writes or a transactional invariant is a poor fit for an eventually-consistent event path.
- **Trade-offs of the fix:** A hybrid system has two interaction styles to learn and operate, and boundaries between them need thought. That's cheaper than either extreme.

## Design Decisions & Trade-offs

**Default to event-carried state transfer for integration events.** Thin notifications reintroduce runtime coupling and read amplification, and they break replay (a callback returns *current* state, not the state at event time). Pay the schema-contract cost and make events self-contained — with the caveat that PII in events is a [retention and crypto-shredding](../encryption-and-key-management/learning.md) commitment, so decide what *not* to include.

**Broker choice follows the replay question.** If new consumers should be able to build state from history, if projections need rebuilding, or if per-entity ordering matters — log-based (Kafka/Redpanda). If work items are independent tasks needing per-message retry, delay, and DLQ semantics — queue-based (SQS/RabbitMQ). Running both for different jobs is normal and better than forcing one to do the other's work.

**The partition key is the shard key.** Same decision, same failure modes: key by the entity whose ordering matters, watch for hot partitions from skew or low cardinality, and know that changing partition count re-maps keys and breaks ordering across the change. Decide it deliberately, and over-provision partitions like [logical shards](../sharding/learning.md) since increasing them later is disruptive.

**Envelope schema is a day-one decision.** Mandate `event_id` (dedup), `event_type` + `version` (routing and evolution), `occurred_at` (semantics — note this is *valid time*, distinct from broker append time), `correlation_id` and `causation_id` (debuggability), and `partition_key`. These cannot be retrofitted onto history, and their absence is felt exactly when you're already in an incident.

**Enforce schema compatibility mechanically.** A registry with backward-compatibility mode turns unknown-consumer breakage into a CI failure. For breaking changes, version the topic and dual-publish through a migration — there is no atomic deploy of all consumers.

**Reliability is not the broker's job alone.** [Outbox](../outbox-pattern/learning.md) on the producer (no dual writes), [inbox/idempotency](../idempotency-and-delivery-semantics/learning.md) on the consumer (at-least-once is guaranteed), DLQs with monitored depth, and retention longer than your worst outage. A broker with strong durability guarantees does not save a producer that lost the event before publishing it.

**Build the observability before you need it.** Correlation-based log search, distributed tracing across broker hops, per-consumer lag dashboards, DLQ depth alerts, and a generated topology map. In request/response you can debug with a stack trace and get lucky; in EDA, if you didn't build the tooling, you cannot investigate at all.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. State the three Fowler patterns and, for each, what coupling it *removes* and what it *adds*. Why does thin notification undermine the main reason for going async?
2. Give the test that distinguishes an event from a command, and explain what goes wrong when a command is published as an event.
3. Log-based vs. queue-based: name two capabilities each has that the other structurally can't, and pick correctly for (a) rebuilding a projection, (b) resizing 100 000 images.
4. What exactly does a broker guarantee about ordering? List three distinct ways a consumer can legitimately observe effect-before-cause.
5. Why is consumer lag more dangerous than an error rate? Name the three signals to alert on and why the derivative catches problems earliest.
6. A poison message arrives. Trace what happens with and without a DLQ in a log-based broker, and name the guarantee a DLQ silently sacrifices.
7. Reconstruct why correlation and causation ids cannot be added retroactively, and what each one lets you answer that the other doesn't.
8. Your team wants "everything to be an event." Give the three-way test that decides per interaction, and one example that should stay synchronous.

Design exercises:

- Take one existing synchronous flow and redesign it as EDA: define the events (with full envelope), pick partition keys, choose the broker model, and — the important part — write down what becomes *harder* (which invariants weaken, what debugging requires). If nothing got harder, the analysis isn't finished.
- Draw the topology map for a system you know: producers, topics, consumers. Look for cycles, for topics with one consumer (probably a command), and for consumers whose lag nobody monitors.
- Take a real event schema and attempt three changes — add optional field, rename field, change a type. Predict which break consumers, then check against your registry's compatibility mode.

## Open Questions

- Kafka vs. Redpanda vs. Pulsar for a mid-size system in 2026: operational burden versus features that actually get used (tiered storage? geo-replication?) — trial before assuming Kafka by default.
- OpenTelemetry trace propagation through broker headers: how well does context survive producer → broker → consumer in practice across languages, and what's the sampling story for high-volume topics?
- Topology maps generated from code: does tooling exist (AsyncAPI-based? static analysis?) or is this universally hand-maintained and therefore stale?
- Transactional outbox versus Kafka transactions for exactly-once producer semantics — when is the broker's own transaction support sufficient, and what does it cost in throughput?
- Schema registry adoption in Rust: what's the current tooling for Avro/Protobuf registry integration compared to the JVM ecosystem's maturity?

## References

- Martin Fowler, ["What do you mean by 'Event-Driven'?"](https://martinfowler.com/articles/201701-event-driven.html) — the notification / state-transfer / event-sourcing / CQRS taxonomy this doc is organized around; short and clarifying.
- Martin Kleppmann, *Designing Data-Intensive Applications*, ch. 11 ("Stream Processing") — logs, partitioning, consumer offsets, and derived state, rigorously.
- Ben Stopford, *Designing Event-Driven Systems* (free from Confluent) — the log-based view at length, including schema evolution and event-carried state transfer trade-offs.
- [Kafka documentation](https://kafka.apache.org/documentation/) — the design section on partitions, ordering, and consumer groups is the authoritative statement of what is and isn't guaranteed.
- Gregor Hohpe & Bobby Woolf, *Enterprise Integration Patterns* — the vocabulary (channels, routers, DLQ, message endpoints) that most brokers still implement; worth skimming for the pattern names alone.
- Related topics in this repo: [Outbox Pattern](../outbox-pattern/learning.md) (reliable publishing — the step diagrams omit), [Idempotency & Delivery Semantics](../idempotency-and-delivery-semantics/learning.md) (why consumers must dedup), [Saga Pattern](../saga-pattern/learning.md) (multi-step workflows; choreography's ceiling), [Event Sourcing & CQRS](../event-sourcing/learning.md) (the storage decision often confused with this one), [Change Data Capture](../change-data-capture/learning.md) (row-diffs vs. authored events as event sources), [Sharding](../sharding/learning.md) (the partition key is the shard key), [Serialization & Encoding](../../performance-optimization/serialization-and-encoding/learning.md) (schema evolution mechanics), [Backpressure](../backpressure-and-rate-limiting/learning.md) (consumer lag and producer rate limits).
