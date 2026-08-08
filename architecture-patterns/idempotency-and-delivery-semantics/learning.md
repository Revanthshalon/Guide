# Idempotency & Delivery Semantics — Learning Notes

## Mental Model

**Over an unreliable network, "do this exactly once" is not an option you can select. You choose between "maybe zero times" and "maybe many times" — and engineer the many-times case to be harmless.**

The root cause is one sentence: when a request times out, the sender cannot distinguish "it never happened" from "it happened but the acknowledgment was lost." Every retry decision is made in that fog. Don't retry → operations are sometimes lost (**at-most-once**). Retry → operations are sometimes duplicated (**at-least-once**). There is no third setting at the transport level; the network's ambiguity is irreducible.

So where does "exactly-once" marketing come from? From moving the goalposts — usefully. What systems actually deliver is at-least-once delivery plus **effectively-once processing**: the operation may *arrive* many times, but its *effect* is applied once, because the receiver deduplicates or the operation is designed so that repeats change nothing. That receiver-side property is **idempotency**: `f(f(x)) = f(x)` — applying the operation twice leaves the same state as once.

The mental model to carry everywhere: **every arrow in your architecture diagram — HTTP call, queue delivery, webhook, event to projection — is at-least-once or at-most-once. For each arrow, ask: what happens on duplicate? What happens on loss?** If the answer to duplicates is "we charge the card twice," the arrow is a bug that hasn't fired yet. Retries + idempotent receivers is the standard answer, and it's the foundation the rest of this repo's patterns quietly assume: the [outbox](../outbox-pattern/learning.md) produces duplicates by design, [event-sourcing projections](../event-sourcing/learning.md) redeliver on crash, [sagas](../saga-pattern/learning.md) retry steps.

## Core Concepts

### The two-generals ambiguity (why the timeout is unresolvable)

- **What it is:** A sender that gets no response cannot know which side of the failure it's on: request lost (safe to retry) or response lost (retry duplicates). No protocol over a lossy channel can eliminate this.
- **Why it exists:** It's the impossibility result that generates this whole topic. Understanding it stops you searching for a transport that "just does exactly-once" and redirects effort to the receiver, where the problem is actually solvable.
- **Example:** Payment service call times out after 30 s. The charge may or may not have been placed. The *only* safe designs: ask (query by your own reference id), or retry with an idempotency key so the repeat is harmless. Blind retry and blind give-up are both wrong.

### At-most-once / at-least-once / effectively-once

- **What it is:** At-most-once: fire and forget — no retry, occasional loss. At-least-once: retry until acknowledged — occasional duplicates. Effectively-once: at-least-once delivery + idempotent/deduplicating processing = each operation's *effect* applied once.
- **Why it exists:** These are the only honest labels. Naming which one each interface provides — and writing it into the contract — is half the engineering.
- **Example:** Metrics/telemetry: at-most-once is fine (a lost data point is cheaper than the machinery). Money movement: at-least-once + idempotency, always. Kafka's "exactly-once semantics" = effectively-once within its transactional producer→consumer→producer pipelines (offsets and outputs committed atomically); the moment an effect leaves that boundary (an email, an HTTP call), you're back to at-least-once.

### Idempotency key

- **What it is:** A caller-generated unique id for the *operation* (not the payload): `Idempotency-Key: 7f3e...` on an HTTP POST, `message_id` on a queue message. The receiver records completed keys with their responses; a repeat key returns the stored response without re-executing.
- **Why it exists:** Most business operations aren't naturally idempotent ("charge $50" twice ≠ once). The key converts any operation into an idempotent one by giving the receiver identity to deduplicate on — this is *the* general-purpose tool.
- **Example:** Stripe's API: `POST /charges` with an idempotency key; network dies; client retries with the *same key*; Stripe returns the original result — one charge. Two subtleties that make it work: the key must be **generated at the source of intent** (one user click = one key, minted before the first attempt, surviving retries — a key per attempt deduplicates nothing); and key-recording must be **atomic with the effect** (same transaction), or the crash window reopens between them.

### Natural idempotency

- **What it is:** Operations that are repeat-safe by shape, no dedup table needed: absolute sets (`status = 'shipped'`), keyed upserts (`INSERT ... ON CONFLICT DO UPDATE`), deletes by id, "ensure X exists."
- **Why it exists:** Free idempotency is better than bookkept idempotency. Often a small contract change — send the target state instead of the delta — removes the entire dedup apparatus.
- **Example:** `balance += 50` is the classic trap (relative update — every duplicate compounds). `balance = 170` is repeat-safe but has a second failure mode: two *different* concurrent updates can overwrite each other (lost update), so pair it with optimistic concurrency (`WHERE version = 6`) — idempotent *and* race-safe. Rule of thumb: deltas need dedup; absolutes need versioning.

### Deduplication window and the consumer's contract

- **What it is:** The receiver's dedup memory: seen-ids table, unique constraint, or broker feature. Every practical implementation has a **window** — dedup by id for 24 h, unique index on the last N million keys — because remembering forever is a cost decision.
- **Why it exists:** Duplicates cluster near the original (immediate retries, redelivery after crash-restart, consumer-group rebalances) but the tail is long: a queue's dead-letter replay or an operator's backfill can resurrect a message days later. The window must cover the *real* redelivery tail, including operational replays — not just the retry policy's few minutes.
- **Example:** Consumer dedups on `message_id` kept 1 h. Three weeks later, an operator replays a day of the stream to rebuild a downstream store; every replayed effect applies a second time. Fixes: dedup keyed by immutable business identity with a unique constraint (permanent, enforced by the database); or make replay-targets rebuildable-from-scratch (projection style: wipe and refold) so replay means *rebuild*, not *re-apply*.

## Worked Example

An order-placement flow: browser → API → orders DB, then an `OrderPlaced` event → queue → email service + loyalty-points service.

**1. The browser arrow.** User clicks "Place order"; the request times out at the gateway; the browser retries. Without protection: two orders, two charges. Design: on checkout-page load, mint `order_attempt_id = k1` (UUID); every retry of this purchase carries `k1`.

```
POST /orders  Idempotency-Key: k1
  server, in ONE transaction:
    INSERT INTO idempotency_keys(key, status) VALUES ('k1','in_progress')   -- unique constraint
    ... create order ord-991, enqueue outbox row ...
    UPDATE idempotency_keys SET status='done', response={order_id: ord-991}
retry arrives with k1:
  unique-constraint hit → read stored row → return {order_id: ord-991}     -- no second order
concurrent duplicate (in_progress): hold or return 409-retry-later — do NOT run the operation again
```

The unique constraint on `key` is the linchpin: the database's atomicity turns "check then execute" into a race-free claim.

**2. The queue arrow.** The outbox relay publishes `OrderPlaced{order_id: ord-991}` — at-least-once by construction (it may crash after publish, before marking the row done, and republish). No fight; consumers own the problem.

**3. The email consumer.** Sending email is an *external effect* — no transaction covers SMTP. Order of operations decides the failure mode:

```
record-then-send:  INSERT processed(msg_id) COMMIT; send email
                   → crash between = email LOST        (at-most-once effect)
send-then-record:  send email; INSERT processed(msg_id)
                   → crash between = email DUPLICATED  (at-least-once effect)
```

For email, duplicate beats lost: send-then-record, accept rare double emails. This choice — *which failure you prefer per effect* — cannot be automated away; it's a domain judgment you must make arrow by arrow.

**4. The loyalty-points consumer.** `points += 10` on duplicate = customer enriched. Two correct shapes:

```
dedup:    INSERT INTO applied(order_id) ...; UPDATE points SET total = total + 10;  -- one txn; unique(order_id)
absolute: points_ledger(order_id PK, points) — INSERT ON CONFLICT DO NOTHING; total = SUM(ledger)
```

The ledger shape is the event-sourcing insight in miniature: store the *fact* keyed by business identity, derive the total — duplicates collapse on the primary key, and the dedup "window" is permanent for free.

**5. End to end:** every arrow is at-least-once; every receiver is idempotent (key table, unique constraint, or ledger); the one unprotectable external effect (email) has a *chosen* failure mode. That is effectively-once, engineered — and it's the same anatomy at any scale.

## Pitfalls in Depth

### Pitfall: The retry that isn't safe (retrying non-idempotent calls)

- **What goes wrong:** A timeout wrapper with retries is added around an HTTP client "for resilience" — around a call that creates orders or moves money. Resilience machinery converts transient network noise into duplicated business effects. Symptom: duplicates spike exactly during incidents (when timeouts fire most), compounding the outage.
- **Why it happens (the mechanism):** Retry policy and idempotency live in different layers owned by different people: the retry is infrastructure config (mesh, client middleware), the endpoint's semantics are application code. Nothing forces them to agree; POST-with-side-effects + generic retry middleware is the default failure.
- **How to handle it in production, and why that works:** Rule: **no retry without a statement of why the target is idempotent.** Make unsafe-by-default explicit — retries enabled per-endpoint, not globally; mutating endpoints require an idempotency key to accept retried requests at all. Service meshes/gateways should retry only idempotent methods unless configured with evidence. This works because it moves the invariant to where it can be reviewed.
- **Trade-offs of the fix:** Per-endpoint retry config is more tedious than a blanket policy — that tedium is the review surface. Some calls end up unretried and fail more visibly; visible failure beats silent duplication.

### Pitfall: Key recorded separately from the effect

- **What goes wrong:** Dedup check reads a Redis set, then the operation runs against Postgres, then the key is written to Redis. Crashes between steps either lose the key (duplicate applies later) or record it without the effect (operation *lost* — the retry is turned away because the key "exists"). Also a straight race: two concurrent duplicates both pass the check before either records.
- **Why it happens (the mechanism):** Check-then-act across two systems is not atomic; every gap is a crash/race window. Exactly the [dual-write problem](../outbox-pattern/learning.md), miniaturized into the dedup mechanism itself — the tool meant to fix duplication reintroduces it.
- **How to handle it in production, and why that works:** Key and effect must commit **in the same transaction in the same store**: idempotency-keys table in the same Postgres as the business tables, unique constraint doing the claim (insert-first, not check-first — the constraint violation *is* the dedup signal, race-free). If the effect's store has no transactions, use the effect's own uniqueness (natural key, conditional put) as the dedup.
- **Trade-offs of the fix:** Couples dedup storage to the business store (fine — that coupling is the correctness). Key table needs pruning (see window pitfall). Redis-only dedup remains acceptable *only* where occasional duplicates are tolerable — i.e., where the stakes were low anyway.

### Pitfall: In-progress duplicates (concurrency, not just sequence)

- **What goes wrong:** First request is mid-flight (slow payment provider); the impatient client's retry arrives *while it's still running*. Naive dedup only stores *completed* keys, so the retry sails through and both execute. Result: duplicates precisely for the slowest operations — the ones most likely to be retried.
- **Why it happens (the mechanism):** The operation has three states — never-seen, in-progress, done — but the dedup was modeled with two. The in-progress window is exactly the timeout window, so it's *the* window in which retries occur; the model missed the common case, not the edge case.
- **How to handle it in production, and why that works:** Claim the key **before** executing (insert `in_progress` row; the unique constraint makes the claim atomic). A duplicate hitting `in_progress` waits or gets `409/Retry-After` — it must not execute and must not be told "failed." On `done`, return stored response. Add a claim lease/expiry so a crashed-mid-flight worker doesn't wedge the key forever: expired claim → safe to re-execute *only if* the underlying effect can be verified or is itself idempotent.
- **Trade-offs of the fix:** The lease brings back a judgment call (crashed vs. slow) — verify-before-reexecute (query the payment provider by your reference) is the robust answer where stakes are high. State machine + expiry is more code than a seen-set; it's the difference between dedup that works in demos and dedup that works during incidents.

### Pitfall: Trusting broker redelivery counts / "exactly-once" flags

- **What goes wrong:** Team enables Kafka EOS / SQS FIFO dedup / broker feature X, declares the problem solved, and strips consumer-side idempotency. Duplicates return via paths the feature never covered: SQS FIFO's 5-minute dedup window (a redrive hours later isn't caught), Kafka EOS ending where the pipeline calls an external API, consumer-group rebalances reprocessing uncommitted batches, DLQ replays, cross-cluster mirroring.
- **Why it happens (the mechanism):** Each feature is real but **scoped**, and the scope is in the fine print: a time window, a transactional boundary, a single cluster. "Exactly-once" reads as a global property; it's a local one. Effects outside the boundary — HTTP calls, emails, writes to another store — were never included.
- **How to handle it in production, and why that works:** Keep the invariant at the *terminal effect*: business-identity dedup (unique constraints, ledgers) at every consumer that produces an effect, regardless of broker features. Use broker EOS as an optimization that shrinks duplicate frequency, not as the correctness mechanism. Read the feature's boundary statement and write it into the consumer's docs.
- **Trade-offs of the fix:** "Belt and suspenders" costs a dedup table you'd hoped the broker made unnecessary. But consumer-side dedup is also what makes *operational replay* safe — a capability you want anyway (see event-sourcing rebuilds), so the cost buys two things.

### Pitfall: Losing instead of duplicating (the silent at-most-once)

- **What goes wrong:** Everyone hunts duplicates; loss hides in ack-before-process consumers: auto-ack on receive (message removed, then process crashes — gone), catch-log-and-continue exception handlers (failure acknowledged as success), fire-and-forget producer sends with no delivery confirmation. No error, no duplicate — just an order that never got its email, discovered by the customer.
- **Why it happens (the mechanism):** At-least-once is a *discipline*, not a broker default: it requires ack-only-after-durable-effect at every hop. Any single hop that acks early silently downgrades the whole chain to at-most-once — and the chain's guarantee is its weakest hop.
- **How to handle it in production, and why that works:** Audit every hop for ack discipline: producers use confirmed sends (or an outbox); consumers process-then-ack; failures go to retry/DLQ, never swallowed. Then verify end-to-end with **reconciliation**: a periodic job comparing source of truth to downstream effects ("orders placed yesterday vs. confirmation emails sent") — the mechanism that catches loss *whatever* caused it, including the causes you didn't foresee.
- **Trade-offs of the fix:** Process-then-ack maximizes duplicates (fine — receivers are idempotent now). Reconciliation jobs are unglamorous plumbing; they are also, in practice, the highest-value pages in money-touching systems.

## Design Decisions & Trade-offs

**Per arrow: which failure do you prefer?** Loss-tolerable (metrics, cache warms) → at-most-once, no machinery. Everything else → at-least-once + idempotent receiver. For external effects with no transactional cover (email, SMS, third-party APIs): choose duplicate-vs-loss consciously per effect — duplicate email, yes; duplicate payout, never (record-then-send + verify-by-reference for those).

**Where the key comes from.** Idempotency keys are minted at the *source of intent* (client/UI at action creation), flow with the request, and derived ids propagate: consumers producing follow-on operations key them deterministically from the incoming identity (`order_id + step-name`), so the whole chain stays deduplicable — this is exactly how [saga](../saga-pattern/learning.md) steps stay safe under retry.

**Dedup mechanism, in order of preference:** (1) natural idempotency — absolute writes + version guard; (2) business-identity unique constraint / ledger row (permanent, free window); (3) idempotency-key table with claim states and lease (the general tool); (4) broker features (an optimization layer only). Prefer earlier: less machinery, fewer windows.

**Window sizing is about operations, not clients.** Client retries span seconds; *operators* replay days. If dedup is windowed, document the window as a hard operational constraint ("replays older than X are unsafe") — or escape the problem: key on permanent business identity, or make the target rebuildable so replay = rebuild.

**API contract:** mutating endpoints accept an idempotency key, return the original response on repeat (indistinguishable to the caller), `409`-or-wait on concurrent duplicates, and *document* the endpoint's semantics. An endpoint whose duplicate behavior is unspecified is unspecified, period.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. A request times out. Enumerate the two possible world-states, and explain why no protocol over that channel can collapse them into one.
2. Why must an idempotency key be minted at the *source of intent* rather than per attempt? What exactly does a per-attempt key fail to deduplicate?
3. Dedup via Redis check → Postgres effect → Redis record. List every crash/race window and say which failure (loss or duplication) each produces.
4. Why does a two-state dedup model (seen / not-seen) duplicate precisely the slowest operations? What's the third state, and what claims it atomically?
5. "We enabled Kafka exactly-once, so consumers don't need dedup." Name three paths that still deliver duplicates.
6. `points += 10` vs `points = 170` vs a ledger row keyed by `order_id` — give the failure mode of each of the first two and why the third escapes both.

Build exercises:

- Build the three-state idempotency-key table in Postgres (unique-constraint claim, `in_progress`/`done`, stored response) behind a toy payment endpoint. Attack it with a concurrent duplicate racer and a kill-mid-flight test; verify one charge in every interleaving.
- Write a consumer with deliberate ack-before-process, kill it under load, and count the lost messages — then flip to process-then-ack with an idempotent handler and repeat. Seeing loss vs. duplication in real counts cements which side you're on.

## Open Questions

- What's the storage/latency cost of an idempotency-key table at real volume (row per write op) — and the right pruning policy given the operational-replay window?
- Kafka EOS mechanics: how exactly do transactional producer + consumer offsets commit atomically, and what does the `read_committed` consumer see during an aborted transaction?
- Reconciliation job design: push (emit completion events, compare streams) vs. pull (query both stores) — which scales better and which finds loss faster?
- Rust: does a solid actor/handler-level idempotency middleware exist (tower layer keyed on a request id), or is this always hand-rolled per service?
- How do payment providers implement verify-by-reference lookups, and what's the contract when *their* side is the ambiguous one?

## References

- Martin Kleppmann, *Designing Data-Intensive Applications*, ch. 8 ("The Trouble with Distributed Systems") and ch. 11 ("fault tolerance" section) — the ambiguity argument and effectively-once framing, rigorously.
- [Stripe: Designing robust and predictable APIs with idempotency](https://stripe.com/blog/idempotency) — the canonical production write-up of idempotency keys, claim states included.
- Pat Helland, *Life Beyond Distributed Transactions* — the paper behind "at-least-once + idempotent receivers" as the scalable architecture; origin of much of this doctrine.
- Confluent, "Exactly-Once Semantics in Apache Kafka" (blog + KIP-98) — read to learn precisely where the EOS boundary sits; the boundary statement is the valuable part.
- Related topics in this repo: [Outbox Pattern](../outbox-pattern/learning.md) (the producer side of at-least-once), [Event Sourcing & CQRS](../event-sourcing/learning.md) (projection checkpointing = the same-transaction trick), [Saga Pattern](../saga-pattern/learning.md) (retried steps demand idempotent participants), [Replication & Consistency](../replication-and-consistency/learning.md) (why the network forces all of this).
