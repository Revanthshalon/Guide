# Event Sourcing & CQRS — Learning Notes

## Mental Model

**Stop storing state. Store the facts that produced it.**

A conventional system keeps current state (a row: `balance = 120`) and destroys history on every update. Event sourcing inverts this: the system of record is an **append-only log of events** — immutable facts about things that happened (`AccountOpened`, `MoneyDeposited{100}`, `MoneyWithdrawn{30}`). Current state is not stored as truth; it is **derived** by replaying the events:

```
state = fold(apply, initial_state, events)
```

The accountant's ledger is the canonical analogy: accountants never erase a wrong entry — they append a correcting entry. The ledger is truth; the account balance is a running total computed from it.

The key insight that makes the whole pattern click: **current state is just a cache.** Any state — the aggregate you validate commands against, every read-model table, every search index — can be thrown away and rebuilt from the log. The log is the only thing that can never be lost.

**CQRS** (Command Query Responsibility Segregation) is a separate but naturally paired idea: use different models for writing and reading. The write side processes commands against a model optimized for enforcing invariants; the read side serves queries from models optimized for each screen or API. Event sourcing makes CQRS almost inevitable, because an event log is a terrible thing to query directly — so you project it into read models. You can do CQRS without event sourcing (two models over a normal database), but doing event sourcing without CQRS is rare and painful.

Why anyone accepts this complexity:

- **Complete audit trail for free** — the history *is* the database, not a bolted-on audit table that drifts from truth.
- **Temporal queries** — "what was this account's state on March 3rd?" is a replay, not archaeology.
- **Retroactive fixes and new questions** — a bug in derived state is fixed by correcting the projector and replaying; a new business question ("how often do users undo?") is answered from events you already captured.
- **Debugging superpower** — reproduce any production issue by replaying the exact event sequence locally.

What it costs: eventual consistency between write and read sides, schema evolution of immutable data, and a mental model most teams haven't operated before. This is the trade the whole document is about.

## Core Concepts

### Event

- **What it is:** An immutable record of something that happened, named in past tense from the business domain: `OrderPlaced`, `PaymentCaptured`, `ShipmentDelayed`. Carries the data that describes the fact, plus metadata (event id, stream id, version number, timestamp, correlation/causation ids).
- **Why it exists:** Events are the atoms of truth. Past tense is not a style preference — it enforces that events record decisions already made and validated. An event can never be rejected or rolled back, only compensated by a later event.
- **Example:** `MoneyWithdrawn { account_id, amount: 30, withdrawn_at, event_id, version: 3 }`. Note what it is *not*: it is not `WithdrawMoney` (that's a command, a request that may be refused) and not `BalanceChanged { new_balance: 90 }` (that's state leakage — it records the *effect*, losing the *intent*).

### Command

- **What it is:** A request to do something, imperative mood: `WithdrawMoney { account_id, amount }`. Commands are validated against current aggregate state and are **rejectable**.
- **Why it exists:** The command/event distinction is the write-side contract: commands go in, get validated against invariants, and zero or more events come out. The core of the write side is a pure decision function: `decide(state, command) -> Result<Vec<Event>, Error>`. Keeping it pure (no I/O) makes it trivially testable: given these past events, when this command, then expect these events.
- **Example:** `WithdrawMoney{amount: 200}` against a state with `balance: 120` returns `Err(InsufficientFunds)` — and *no event is written*. Failed commands leave no trace unless the business explicitly wants a `WithdrawalRefused` event (sometimes it does — that's a domain decision, not a technical one).

### Aggregate (the consistency boundary)

- **What it is:** A cluster of domain state that must be kept consistent *transactionally* — one aggregate maps to one event **stream** (e.g. stream `account-42` holds all events for account 42). Commands are processed against exactly one aggregate, whose state is rehydrated by folding its stream.
- **Why it exists:** You cannot enforce invariants across the whole system atomically at scale. The aggregate is the deliberate answer to "what must be immediately consistent?" — inside the boundary, invariants are absolute (balance never goes negative); across boundaries, consistency is eventual and handled by [sagas](../saga-pattern/learning.md) or process managers.
- **Example:** `Account` is an aggregate: `deposit`/`withdraw` invariants are enforced within it. "Customer's total balance across all accounts must not exceed $1M" is a *cross-aggregate* rule — it cannot be an absolute invariant and must be handled reactively (detect and compensate) or via a reservation flow.

### Event Store

- **What it is:** The database of streams. Its API is small: `append(stream_id, expected_version, events)` and `read(stream_id)`, plus a way to subscribe to all events in order for projections. It can be a purpose-built product (EventStoreDB/Kurrent, Axon) or a table in Postgres (`(stream_id, version)` unique index, global sequence for subscriptions).
- **Why it exists:** Append with `expected_version` is the concurrency mechanism: **optimistic concurrency control**. Two commands race on the same aggregate; both rehydrate at version 5; both try to append at version 6; one wins, the other gets a version-conflict error and retries (rehydrate again, re-validate — the command may now be invalid). No locks, and invariants stay safe.
- **Example:** Kafka is *not* an event store in this sense — this trips many teams. Kafka has no per-stream conditional append (no optimistic concurrency per aggregate) and no efficient "read one aggregate's events" (topics partition by throughput, not by aggregate). Kafka is excellent *downstream* of the event store for distributing events; it cannot safely be the write-side system of record for aggregates.

### Projection / Read Model

- **What it is:** A consumer that subscribes to the event log and folds events into a queryable shape: a SQL table of account balances, an Elasticsearch index of orders, a materialized "screen" document. Each projection tracks a **checkpoint** (position of the last event processed). One log, many projections — each shaped for the query it serves.
- **Why it exists:** This is the CQRS read side and the answer to "how do I query an event log?" — you don't; you project it. Projections are disposable by design: to change one, or fix a bug in one, you delete it, reset its checkpoint to zero, and replay. That disposability is what makes the "state is a cache" mental model operational.
- **Example:** `AccountSummaryProjection` handles `MoneyDeposited` with `UPDATE summaries SET balance = balance + $amount WHERE id = $account_id` — plus idempotency protection (see pitfalls), because event delivery to projections is at-least-once.

### Snapshot

- **What it is:** A cached fold of an aggregate's state at some version, stored so rehydration reads `snapshot + events since` instead of the whole stream.
- **Why it exists:** Pure performance optimization for long streams. It is *not* part of the model: snapshots can always be deleted and recomputed, snapshot schema changes are non-events (throw them away, re-fold). Add snapshots only when rehydration latency measurably hurts — most aggregates with well-chosen boundaries have short streams (dozens of events) and never need them.
- **Example:** Policy: snapshot every 200 events. Rehydrating account at version 1,850: load snapshot@1800, fold 50 events. Without: fold 1,850.

### Internal vs. Integration Events

- **What it is:** Internal (domain) events are the aggregate's persisted facts — fine-grained, schema owned by the bounded context, free to evolve. Integration events are the *published contract* other services consume — coarser, stable, versioned deliberately, typically translated from internal events at the context boundary.
- **Why it exists:** If other services consume your internal events directly, your event store's schema becomes a public API and you can never refactor it. This is the event-sourcing version of "don't let other teams query your database tables."
- **Example:** Internally: `ItemAddedToCart`, `ItemRemovedFromCart`, `CartCheckedOut` (17 events). Published to other services: one `OrderPlaced` integration event with the final line items. Publication itself needs the [outbox pattern](../outbox-pattern/learning.md) or [CDC](../change-data-capture/learning.md) on the event store to avoid dual-write inconsistency (see pitfalls).

## Worked Example

A bank account, end to end. Write side first.

**1. Open and use the account — commands become events.**

```
Command: OpenAccount{id: "acc-42", owner: "Dana"}
  state = None                          → OK
  emit  AccountOpened{owner: "Dana"}                     stream acc-42, v1

Command: Deposit{amount: 100}
  rehydrate: fold [AccountOpened] → {balance: 0}
  emit  MoneyDeposited{amount: 100}                      v2

Command: Withdraw{amount: 30}
  rehydrate: fold [v1, v2] → {balance: 100}
  30 ≤ 100                              → OK
  emit  MoneyWithdrawn{amount: 30}                       v3

Command: Withdraw{amount: 200}
  rehydrate: fold [v1..v3] → {balance: 70}
  200 > 70                              → Err(InsufficientFunds), nothing written
```

The stream `acc-42` is now `[AccountOpened, MoneyDeposited{100}, MoneyWithdrawn{30}]`. That list — not any balance column — is the account.

**2. The decision logic is two pure functions.**

```rust
fn apply(state: &mut Account, event: &AccountEvent) {          // fold step
    match event {
        AccountOpened { .. }        => state.balance = 0,
        MoneyDeposited { amount }   => state.balance += amount,
        MoneyWithdrawn { amount }   => state.balance -= amount,
    }
}

fn decide(state: &Account, cmd: Withdraw) -> Result<Vec<AccountEvent>, Error> {
    if cmd.amount > state.balance { return Err(InsufficientFunds); }
    Ok(vec![MoneyWithdrawn { amount: cmd.amount }])
}
```

`apply` must never fail and never decide anything (events are settled facts); `decide` holds every invariant. All domain tests are: given events → when command → expect events/error. No mocks, no database.

**3. A race, resolved by optimistic concurrency.**

Two clients withdraw simultaneously from acc-42 (balance 70):

```
A: rehydrate at v3 (70)   decide Withdraw{50} → OK   append expect_v3 → success, writes v4
B: rehydrate at v3 (70)   decide Withdraw{40} → OK   append expect_v3 → VERSION CONFLICT
B: retry — rehydrate at v4 (20)   decide Withdraw{40} → Err(InsufficientFunds)
```

Without `expected_version`, both would succeed and the balance would go negative. The conflict-and-retry loop is what makes invariants hold under concurrency.

**4. The read side catches up.**

A projection subscribed to the log (checkpoint tracking global position):

```
event                       SQL effect                                checkpoint
AccountOpened               INSERT INTO summaries (id,owner,balance)      101
MoneyDeposited{100}         UPDATE summaries SET balance = balance+100    102
MoneyWithdrawn{30}          UPDATE ... balance = balance-30               103
MoneyWithdrawn{50}          UPDATE ... balance = balance-50               104
```

`GET /accounts/acc-42` reads `summaries` — cheap, indexed, shaped for the query. It may briefly lag the write side (say, event 104 written but not yet projected): that lag is the price, and handling it is a pitfall below.

**5. Time travel, because the log is truth.**

"Balance on March 3rd?" → fold only events with `timestamp ≤ March 3`. "Why did this account go to zero?" → read the stream; the answer is literally written there.

## Pitfalls in Depth

### Pitfall: Event schema evolution (the immutability trap)

- **What goes wrong:** Eighteen months in, `MoneyDeposited` needs a `currency` field. There are 40M old events without it, and they are immutable — you cannot `ALTER TABLE` history. Naive deserialization now fails on old events; every projector and the aggregate fold must handle every version that ever existed, forever.
- **Why it happens (the mechanism):** In a CRUD system, migration rewrites data to the new schema once. An event store's contract is that the past is never rewritten, so the *reader* must absorb all historical variety. Teams that never planned for versioning discover the constraint only at the first breaking change.
- **How to handle it in production, and why that works:** (1) Prefer **additive changes** with defaults — new optional field, old events get `currency = "USD"` on read. (2) For real shape changes, use **upcasters**: on read, a pipeline transforms `MoneyDeposited.v1 → v2 → v3` before deserialization, so domain code only ever sees the latest shape; versions are absorbed at one choke point instead of everywhere. (3) For a rare full overhaul, **copy-transform**: write a new stream/store, migrating events through the transformer, then cut over — heavyweight, last resort. (4) Never let internal events leak as integration contracts (see internal-vs-integration above), or every consumer inherits this problem too.
- **Trade-offs of the fix:** Upcaster chains are code you maintain forever and must test against archived real events. Additive-only discipline pushes toward weakly-typed event bodies. Budget for this from day one; retrofit is miserable.

### Pitfall: Dual write — appending and publishing are two writes

- **What goes wrong:** The command handler appends to the event store, then publishes the event to the message broker. The process crashes between the two: the event is stored but never announced. Downstream projections and services silently miss it — a warehouse never ships an order that definitely exists.
- **Why it happens (the mechanism):** Two independent systems cannot be updated atomically without distributed transactions. Any "append then publish" (or worse, "publish then append") sequence has a crash window. The same mechanism as any [dual-write problem](../outbox-pattern/learning.md), but event-sourced systems hit it on *every single command*.
- **How to handle it in production, and why that works:** Make the event store the *only* write, and derive publication from it: either consumers **subscribe to the event store's own log** (its global ordered stream — this is the purest form; the store is its own outbox), or run a relay that tails the store ([CDC](../change-data-capture/learning.md)) and publishes to the broker with at-least-once delivery. Either way, "stored" and "will be published" become the same fact, closing the crash window.
- **Trade-offs of the fix:** Delivery becomes at-least-once — consumers *must* be idempotent (next pitfall). Publication lags the write by the relay's latency (usually milliseconds; occasionally spiky).

### Pitfall: Projections that assume exactly-once delivery

- **What goes wrong:** A projector crashes after applying event 104 but before saving checkpoint 104. On restart it re-applies 104: `balance = balance - 50` runs twice, and the read model is silently wrong — no error, just a corrupt number that surfaces weeks later as a support ticket.
- **Why it happens (the mechanism):** Applying the event and saving the checkpoint are — again — two writes. If they aren't atomic, redelivery is inevitable, and any non-idempotent handler corrupts on redelivery. This is [delivery semantics](../idempotency-and-delivery-semantics/learning.md) applied to the read side.
- **How to handle it in production, and why that works:** For SQL read models, write the projection update and the checkpoint **in the same transaction** — redelivery becomes impossible because "applied" and "recorded as applied" commit together. For stores without transactions (search indexes, caches), make handlers idempotent: store the last-applied event position *in the document* and skip stale events, or design updates to be naturally idempotent (set-to-value rather than increment).
- **Trade-offs of the fix:** Same-transaction checkpointing couples the checkpoint store to the read-model store (fine — keep them together). Idempotent-by-design handlers constrain how you can shape updates; increments are the classic trap.

### Pitfall: Eventual consistency surprises the user (and the developer)

- **What goes wrong:** User saves, the next screen reads a projection that hasn't caught up, and their change is "gone." They retry, creating duplicates. Meanwhile a developer validates a command against a *projection* ("is this email taken?") and makes decisions on stale data.
- **Why it happens (the mechanism):** The read model lags the write model by design. Anything that treats a projection as immediately-consistent truth — UI read-after-write, or command validation reading a read model — imports a race condition.
- **How to handle it in production, and why that works:** For UX: **read-your-own-writes** — the command returns the new stream version; the subsequent query waits until the projection's checkpoint reaches that version (or the client renders optimistically from the command's result). For correctness: commands validate *only* against the aggregate's own rehydrated stream — that read is fully consistent (it *is* the write model). Cross-aggregate uniqueness (unique email) can't be absolutely guaranteed by any single aggregate: either make it its own tiny aggregate/stream per email (a reservation, giving a real transactional guarantee via the version check), or accept detect-and-compensate.
- **Trade-offs of the fix:** Version-waiting adds tail latency to reads after writes. Reservation streams add a step to registration flows. Detect-and-compensate requires a business answer to "what do we do with the duplicate?" — which is often the honest question anyway.

### Pitfall: Mis-drawn aggregate boundaries

- **What goes wrong:** Too big (aggregate = whole customer with all orders): every command rehydrates a giant stream and concurrent commands on unrelated orders fight over one version counter — throughput collapses into retry storms. Too small (aggregate = order line): the invariant "order total ≤ credit limit" spans aggregates and can no longer be enforced transactionally — it silently stops being a guarantee.
- **Why it happens (the mechanism):** The aggregate is simultaneously the unit of *consistency*, of *concurrency*, and of *stream length*. Boundaries drawn from the data model ("Customer has Orders, so Customer is the aggregate") instead of from invariants get all three wrong at once.
- **How to handle it in production, and why that works:** Draw boundaries from **invariants**: the aggregate is the smallest cluster of state that must be transactionally consistent to enforce a real business rule. ("Real" matters — challenge each supposed invariant: must it *never* be violated, or is violated-then-compensated acceptable? The second is cheaper.) Watch two production signals: version-conflict rate (boundary too big / too contended) and cross-aggregate "invariants" appearing in sagas (boundary too small). Changing a boundary later means new streams and a migration — possible, but expensive enough that this deserves real design time up front.
- **Trade-offs of the fix:** Invariant-driven boundaries often feel unnatural versus the entity-relationship view, and small aggregates push workflow logic into [sagas](../saga-pattern/learning.md) — more moving parts, but honest about what's actually guaranteed.

### Pitfall: Unbounded streams (the aggregate that never ends)

- **What goes wrong:** Most aggregates end (an order completes at ~20 events). Some don't: a ledger, a long-lived account, an IoT device stream — thousands of events per year, forever. Rehydration latency climbs; even snapshots only mask it while replay/rebuild time grows without bound.
- **Why it happens (the mechanism):** Stream length is proportional to aggregate *lifetime*, and some domain objects live forever. Snapshots fix rehydration but not full-stream operations (rebuilds, migrations, upcaster sweeps).
- **How to handle it in production, and why that works:** **Close the books** — the accounting trick: periodically end the stream and open a successor seeded with a summary event (`YearClosed{closing_balance}` → new stream `acc-42-2026` starting from that balance). Old streams become cold archives. This bounds *every* stream by construction, which is strictly stronger than snapshotting. Design the closing cadence into the domain (fiscal year, billing period) rather than bolting it on.
- **Trade-offs of the fix:** Queries spanning periods must stitch streams (usually a projection's job anyway). The closing event must capture *everything* the successor needs — an under-specified summary is a subtle data loss.

### Pitfall: GDPR / PII vs. immutability

- **What goes wrong:** `UserRegistered{name, email}` is in the immutable log, and a deletion request (GDPR Art. 17) arrives. You cannot delete the event without breaking the log's integrity, versions, and every consumer's assumptions — but "we never delete" is not a legal defense.
- **Why it happens (the mechanism):** Append-only storage and right-to-erasure are directly opposed. Retrofitting is brutal because PII is by then smeared across events, snapshots, projections, *and backups*.
- **How to handle it in production, and why that works:** **Crypto-shredding**: encrypt PII fields with a per-person key stored outside the log; on erasure, destroy the key — ciphertext remains in the log (integrity preserved) but is unrecoverable, which regulators have accepted as erasure. Alternatively **keep PII out of events entirely**: events carry `user_id`, PII lives in a small mutable store, deletion is a plain delete. Projections re-project or redact on the same trigger. Decide *before* the first PII-bearing event is written.
- **Trade-offs of the fix:** Crypto-shredding adds key management (rotation, backup of the keystore — which is now itself PII-critical) and per-read decryption cost. PII-out-of-events reintroduces a join and makes events less self-contained. Both beat the retrofit.

### Pitfall: Event-sourcing everything (the complexity tax)

- **What goes wrong:** The team, sold on the pattern, event-sources the entire system — including the product catalog, user preferences, and other plain-CRUD corners. Every trivial feature now costs commands, events, projections, versioning discipline, and eventual-consistency handling. Delivery slows; the team concludes "event sourcing doesn't work."
- **Why it happens (the mechanism):** Event sourcing is a *bounded-context-level* decision, not an architecture-wide one. Its costs are per-model and fixed; its benefits (audit, temporality, replay) only pay in domains that actually need them. Applied to a domain with no interesting history, it's pure overhead.
- **How to handle it in production, and why that works:** Ask per context: does the business care about *how state came to be* (money, inventory, legal, workflow)? Event-source those. CRUD the rest, and let the two coexist — an event-sourced core emitting integration events, CRUD satellites consuming them. This spends the complexity budget where the pattern's benefits are real.
- **Trade-offs of the fix:** A heterogeneous system: two persistence styles, and the CRUD parts lack the audit/replay properties. That's fine — uniformity is not a goal; fitness per context is.

## Design Decisions & Trade-offs

**Event store technology.** Purpose-built (EventStoreDB/Kurrent) gives you streams, `expected_version`, ordered global subscriptions, and projections out of the box — at the cost of a new operational dependency. Postgres-as-event-store (an events table with a `(stream_id, version)` unique constraint and a global sequence) keeps your ops surface unchanged and is entirely adequate up to serious scale; you write the subscription plumbing, or use an ecosystem library. Kafka alone: no (see Core Concepts). Default: start with Postgres unless you already know you need the specialized store.

**The global ordering detail.** Per-stream reads use `(stream_id, version)`; projections need a **total order across all streams** (a global position). Getting a gap-free global ordering out of a plain SQL sequence under concurrency is the one genuinely tricky bit of DIY stores — in-flight transactions can commit out of sequence order, letting a tailing projector skip events. Solve with a single-writer append path, transactional-outbox-style polling that tolerates gaps, or a store/library that has already solved it. This detail decides whether projections can silently lose events.

**Sync vs. async projections.** Async (subscribe + checkpoint) is the default — decoupled, rebuildable, scalable. Synchronous (update the read model in the append transaction) buys read-your-writes for one critical screen at the cost of coupling write latency to every projector and losing independent rebuild. If one view needs immediacy, sync *that one* and keep the rest async — or use version-waiting instead.

**Event granularity.** Fine-grained, intention-revealing events (`ItemAdded`, `ItemRemoved`) preserve the most information and make the best projections; coarse events (`CartUpdated{full_state}`) are barely better than CRUD. Rule: name events after *business decisions*, and if a screen's worth of fields changed for one reason, that's one event named after the reason.

**Snapshot policy.** None until measured need; then every-N-events, stored separately, treated as disposable. Closing-the-books beats snapshots when the domain has a natural period.

**CQRS without event sourcing.** A read-heavy CRUD context can still split models (write model + denormalized read tables fed by [CDC](../change-data-capture/learning.md) or triggers). Keep the two tools independent in your head; conflating them causes both to be over-applied.

## Open Questions

- What does the retry loop on version conflict look like concretely (max attempts, backoff, when to surface failure to the user)? Sketch one in Rust.
- At what stream length does Postgres rehydration latency actually hurt on our hardware? Benchmark before believing any snapshot advice.
- How do teams test upcaster chains against years of archived events in practice — golden-file corpora?
- Command handlers that emit *multiple* events atomically: which invariants make that necessary, and does it change boundary design?
- How do Kurrent's `$all` and persistent subscriptions differ in delivery guarantees from tailing a Postgres events table?

## References

- Martin Fowler, [Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) — the canonical short introduction; good for the core idea, thin on production concerns.
- Greg Young, *Versioning in an Event Sourced System* (free online book) — the definitive treatment of the schema-evolution pitfall; read before writing the first event schema.
- Greg Young, "CQRS and Event Sourcing" (talk, multiple recordings) — the origin of most of this vocabulary, including why CQRS ≠ event sourcing.
- Martin Kleppmann, *Designing Data-Intensive Applications*, ch. 11 ("Stream Processing") — situates event logs, ordering, and derived state in the wider data-systems picture; best grounding for the projection/consistency material.
- Vaughn Vernon, *Implementing Domain-Driven Design*, ch. 10 ("Aggregates") — the invariant-driven boundary rules the mis-drawn-aggregate pitfall depends on.
- [Kurrent (EventStoreDB) documentation](https://docs.kurrent.io/) — concrete semantics of streams, `expected_version`, subscriptions; a useful reference model even if you build on Postgres.
- Related topics in this repo: [Outbox Pattern](../outbox-pattern/learning.md) (dual-write), [Idempotency & Delivery Semantics](../idempotency-and-delivery-semantics/learning.md) (projection redelivery), [Saga Pattern](../saga-pattern/learning.md) (cross-aggregate workflows), [Change Data Capture](../change-data-capture/learning.md) (publishing the log), [Replication & Consistency Models](../replication-and-consistency/learning.md) (the consistency vocabulary used throughout).
