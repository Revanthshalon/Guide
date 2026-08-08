# Saga Pattern — Learning Notes

## Mental Model

**There are no transactions across services. A saga replaces the transaction you can't have with a sequence of local transactions you can — plus a prepared apology for each one.**

Inside one database, "reserve inventory AND charge the card AND create the shipment" is a transaction: all or nothing, isolation included, rollback free of charge. Split those steps across three services and that machinery is gone — distributed transactions (2PC) across heterogeneous services are unavailable in practice (they demand a coordinator every participant trusts, block on the slowest node, and third parties like payment providers simply won't join). The saga accepts this and restructures the workflow:

- Each step is a **local transaction** in one service — atomic within its boundary (that boundary being the aggregate, as established in [event sourcing](../event-sourcing/learning.md)).
- Each step that isn't last has a **compensating action** — a business-level undo (`ReleaseInventory`, `RefundPayment`) to run if a *later* step fails.
- Forward path: T1 → T2 → T3. Failure at T3: run C2, then C1 — unwinding in reverse.

The crucial mental shift: **compensation is not rollback.** Rollback erases history — as if nothing happened. Compensation is a *new action* that reverses the business effect while the history stands: the charge happened, then the refund happened; the customer's statement shows both. Between a step and its compensation, the intermediate state was *visible to the world* — other transactions may have read it, emails may have been sent. A saga therefore doesn't restore the ACID guarantee; it trades **isolation** away and keeps a weaker but workable promise: *the system ends in either the success state or a deliberately-designed "undone" state — never stranded halfway.*

This is the pattern's true nature: less a technical trick than a **business-design discipline**. "What's the compensation for a shipped package?" is not an engineering question — it's a question about what the business does when the ship has sailed (recall it? eat the cost? bill anyway?). Sagas force those answers to be designed up front rather than improvised during incidents. The infrastructure underneath is exactly the two previous topics: steps and their triggers ride on [outbox-grade messaging](../outbox-pattern/learning.md), and every step must be idempotent because everything retries ([delivery semantics](../idempotency-and-delivery-semantics/learning.md)).

## Core Concepts

### Local transaction (the step)

- **What it is:** One service doing one atomic thing in its own database, then announcing it (via its outbox): `InventoryReserved`, `PaymentCaptured`. Steps are ordered so that each commits only after its predecessor's announcement arrives.
- **Why it exists:** The step boundary is where atomicity lives now. Everything between steps is the network — retried, duplicated, delayed — so each step must be idempotent (keyed by `saga_id + step`, per the [idempotency playbook](../idempotency-and-delivery-semantics/learning.md)).
- **Example:** Inventory service receives `ReserveInventory{saga: s-17, order: ord-991}`: one transaction inserts the reservation (unique on `s-17`, so retries collapse) and an outbox row `InventoryReserved{s-17}`. Crash anywhere → redelivery → the unique constraint makes the retry a no-op that re-announces.

### Compensating action

- **What it is:** The designed business undo for a committed step: `ReleaseInventory` for `ReserveInventory`, `RefundPayment` for `CapturePayment`. Compensations run when a *later* step fails permanently, in reverse order of the steps they undo.
- **Why it exists:** Committed local transactions can't be uncommitted — the undo must be another forward action. Compensations must themselves be idempotent *and* must not be able to fail permanently for business reasons (a refund can be retried through outages; "refund rejected" must be impossible by design, or the saga has no way back).
- **Example:** `RefundPayment{s-17}`: idempotent via the provider's idempotency key (`s-17-refund`), retried until acknowledged. Contrast a *non-compensatable* step: `ShipPackage` once the truck leaves. Such steps are **pivots** — see below — and their placement is the core of saga design.

### The pivot and the three step zones

- **What it is:** Order the steps and a structure appears: **compensatable steps** (can be undone) must come first; the **pivot** is the point of no return (the last step whose failure can still unwind everything — equivalently, the first non-compensatable step); **retriable steps** after the pivot must be guaranteed to eventually succeed (retry forever, no business rejection possible).
- **Why it exists:** This zoning *is* the design method. Before the pivot, failure → compensate backward to "never happened (business-wise)." After the pivot, there is no backward — so anything after it must be a step that cannot be refused, only delayed. If a step after your pivot can genuinely fail (e.g. payment declined after shipping), the ordering is wrong or the business must accept the loss — an explicit decision, not an accident.
- **Example:** Order flow: `ReserveInventory` (compensatable) → `CapturePayment` (pivot — after this, the money moved) → `CreateShipment` (retriable: warehouse service might be down for hours, but it cannot *refuse*). Payment declines happen before anything irreversible; shipment problems are delays, not failures.

### Choreography (event-chained sagas)

- **What it is:** No central coordinator. Each service subscribes to the previous step's event and knows its own next move: Order service emits `OrderPlaced` → Inventory hears it, reserves, emits `InventoryReserved` → Payment hears *that*, charges, emits `PaymentCaptured` → … Failure events (`PaymentFailed`) trigger listening compensators the same way.
- **Why it exists:** Zero new infrastructure (it's just events and the outbox you already have), no coordinator to deploy or make highly available, and loose coupling — services don't know the workflow, only their neighbors' events.
- **Example:** Fine at 3 steps. At 6 steps with two failure paths, the workflow exists *nowhere* — it's smeared across six services' subscription lists. "What happens after `InventoryReserved`?" requires reading every service's code; adding a step means coordinating changes in services that shouldn't care. This scaling cliff is the pattern's main fork (below).

### Orchestration (the saga executor)

- **What it is:** A dedicated orchestrator owns the workflow definition and the saga's state machine: it sends *commands* (`ReserveInventory`), awaits *replies*, decides the next command or compensation, and persists saga state (`s-17: step 2 of 4, compensating=false`) — durably, via its own outbox, so a crashed orchestrator resumes where it stopped.
- **Why it exists:** The workflow becomes a first-class artifact: readable in one place, testable as a state machine, evolvable by changing one service. Timeouts, retries, and compensation logic live where they can be seen. This is what saga frameworks and durable-workflow engines (Temporal et al.) industrialize.
- **Example:** Orchestrator for `s-17` records `awaiting: PaymentReply` before sending `CapturePayment`. It crashes; restarts; reloads state; re-sends the command (participant idempotency absorbs the duplicate). The reply `PaymentFailed` flips it to `compensating`: it issues `ReleaseInventory`, awaits confirmation, marks the saga `aborted`. Every arrow is at-least-once; every box is idempotent; the state machine is the truth.

### Semantic locks and other isolation countermeasures

- **What it is:** Techniques to blunt the lost isolation: a **semantic lock** marks in-flight state at the domain level (`order.status = PENDING_PAYMENT`, inventory "reserved" rather than decremented) so other transactions can see and respect the limbo; **commutative/reordering designs** make interleaved sagas produce the same result regardless of order; **version checks** reject stale updates.
- **Why it exists:** Between T1 and the saga's end, intermediate states are readable and other sagas interleave freely — anomalies ACID would have prevented (two sagas both "seeing" the last unit in stock) are now the application's problem. Countermeasures re-create just enough isolation, in domain terms, where it matters.
- **Example:** Inventory holds `available = 10, reserved = 3` instead of decrementing to 7: concurrent sagas reserve atomically (each reservation is a local transaction claiming from `available`), a dashboard reads honestly ("7 free, 3 in-flight"), and compensation is trivially the release of the reservation. The reserve/confirm/release triple is the semantic-lock idiom — and it's why "reserve then capture" appears in every serious commerce flow.

## Worked Example

Travel booking: flight + hotel + charge, three services, no shared database.

**1. Design before code — the compensation table and the pivot.**

```
step                    compensation           zone
ReserveFlight           CancelFlightHold       compensatable
ReserveHotel            CancelHotelHold        compensatable
ChargeCard              (—)                    PIVOT: charge only after both holds exist
IssueTickets            (—)                    retriable: cannot be refused, only delayed
```

Both reservations are *holds* (semantic locks with a 30-minute expiry — the airline model), not purchases: cheap to compensate, self-cleaning if everything dies.

**2. Happy path (orchestrated).** Orchestrator persists state at every hop via its outbox; participants dedup on `saga_id`:

```
s-88: ReserveFlight → FlightReserved     [state: 1/4 done]
s-88: ReserveHotel  → HotelReserved      [state: 2/4 done]
s-88: ChargeCard    → CardCharged        [state: 3/4 done, past pivot]
s-88: IssueTickets  → TicketsIssued      [state: complete]
```

**3. Failure before the pivot — hotel is full.**

```
s-88: ReserveFlight → FlightReserved
s-88: ReserveHotel  → HotelUnavailable           [state: compensating]
s-88: CancelFlightHold → FlightHoldCancelled     [state: aborted, reason: no-hotel]
```

Customer never charged; flight hold released (and would have expired anyway — defense in depth). The end state is *designed*: "aborted, nothing owed," not a stack trace.

**4. Failure at the pivot — card declined.** Compensate both holds in reverse. The intermediate state was visible (seats showed as held for two minutes) — harmless *because holds were designed to be visible limbo*. That's the isolation countermeasure doing its job.

**5. "Failure" after the pivot — ticketing service down.** No backward path exists (money moved), and none is needed: `IssueTickets` is retriable by construction — the orchestrator retries with backoff for as long as it takes (minutes, hours), the saga sits in `state: 3/4, retrying`, and an alert fires if it exceeds the SLA so a human can watch. The customer gets tickets late — annoying, recoverable. Had `ChargeCard` been placed *first*, a full hotel would have meant refund-after-charge on the common path: same mechanics, worse customer experience, worse dispute surface. **Pivot placement is the design.**

**6. The crash test.** Kill the orchestrator between `ChargeCard` command and reply. Restart: state says `awaiting PaymentReply`; re-send; payment service's idempotency (`s-88` already charged) returns the original result; saga proceeds. Every component crash at every point yields: resume, retry, converge — never a stranded booking. That property came from outbox + idempotency + persisted state, which is why those three topics precede this one.

## Pitfalls in Depth

### Pitfall: Compensation designed as an afterthought (or missing)

- **What goes wrong:** The happy path ships; compensations are stubs ("TODO: refund"). First real mid-saga failure: the system strands orders half-processed, and engineers improvise refunds at 2 a.m. through admin consoles — the exact outcome the pattern exists to prevent. Or compensations exist but can *fail for business reasons* (refund window expired), leaving the saga with no forward and no backward path.
- **Why it happens (the mechanism):** Forward flow is demo-able; compensation is only exercised by failures nobody schedules. And designing a compensation forces uncomfortable business questions ("what do we do about the loyalty points already granted?") that teams defer — so the design debt hides in the unhappy path.
- **How to handle it in production, and why that works:** Rule: **a step without a written compensation (or an explicit pivot/retriable designation) doesn't merge.** The compensation table from the worked example is the design artifact — reviewable by product, not just engineering. Compensations must be idempotent and *un-refusable* (retryable to success by construction). Test them as first-class flows: fault-injection at every step boundary in CI, not as a game-day novelty.
- **Trade-offs of the fix:** Designing undo for every step can double the domain-design work — that's not overhead, that *is* the pattern. If a step's compensation is genuinely impossible, the design pressure it creates (move it after the pivot, or make it a hold) is the method working.

### Pitfall: Isolation loss discovered in production (the double-sell)

- **What goes wrong:** Two sagas read "1 seat left," both proceed, both reserve — oversold. Or a report reads mid-saga state (charge exists, order not yet confirmed) and finance reconciliation flags phantom revenue. Or saga A's compensation releases inventory that saga B already claimed via a read of the in-between state.
- **Why it happens (the mechanism):** Developers carry ACID intuitions — "between my read and my write, nothing changes" — into a world where every gap between local transactions is open season. The anomalies (dirty reads of saga-intermediate state, lost updates between interleaved sagas) are precisely the ones isolation levels used to absorb silently.
- **How to handle it in production, and why that works:** Make in-flight state *explicit in the domain* (the semantic-lock idiom): reservations instead of decrements, `PENDING` statuses instead of final ones, so both concurrent sagas and human-facing reads see honest limbo. Put the *decision* inside one local transaction (claim from `available` atomically — the aggregate enforces it), never in read-then-write across the gap. Add version checks as the backstop. Audit each saga: "between every pair of steps, who can read this state, and what happens if another saga interleaves here?"
- **Trade-offs of the fix:** Pending/reserved states leak into UI, reporting, and analytics — everyone learns the domain has limbo. That visibility is a feature: the alternative was limbo that existed anyway but lied about it.

### Pitfall: Choreography past its complexity ceiling

- **What goes wrong:** The event-chained saga grows to seven steps, three failure paths, one timeout rule. Now: nobody can state the workflow without archaeology across seven repos; a new step means changing three other services' subscriptions; two teams add reactions to the same event and accidentally create a cycle (`InventoryReleased` → re-reserve logic → …) that ping-pongs forever; and "where is saga s-4412 stuck?" has no answer because *no component knows the saga exists*.
- **Why it happens (the mechanism):** Choreography encodes the workflow as a distributed emergent property. Each added edge is locally cheap and globally compounding — coupling doesn't disappear, it hides in subscription topology, where no type system or review process sees it.
- **How to handle it in production, and why that works:** Heuristic: choreography for ≤3–4 steps with one failure path; orchestrate beyond that, and *migrate when you cross the line* rather than defending the sunk cost. If staying choreographed: maintain a generated map of who-consumes-what (subscription topology as CI artifact), stamp every event with `saga_id` + correlation id end to end, and build the saga-state view as a projection over the event stream so "where is s-4412?" has an answer.
- **Trade-offs of the fix:** An orchestrator is a component to run, deploy, and make HA, and it centralizes what was distributed — teams protective of autonomy will push back. The counter: the workflow was already centralized *conceptually* (the business owns it); orchestration just gives it an address.

### Pitfall: The stuck saga (timeouts and the missing reply)

- **What goes wrong:** A participant neither replies nor fails — it's down, or the reply message was lost, or a human-approval step just takes days. Sagas accumulate in `awaiting` states; inventory holds pin stock; customers wait. Nobody notices until "why are 4,000 orders stuck since Tuesday?"
- **Why it happens (the mechanism):** In a synchronous call, absence-of-response is an exception you handle in-line. In a message-driven saga, absence is *silence* — there is no built-in place where "it's been too long" fires. Timeout handling must be designed as an explicit part of the state machine, and by default it isn't.
- **How to handle it in production, and why that works:** Every `awaiting` state gets a **timeout policy**: duration + action (retry the command / compensate and abort / escalate to a human queue). Orchestrators make this natural (durable timers — the killer feature of workflow engines like Temporal); choreography needs scheduled watchdogs scanning for over-age sagas. Pair with **self-expiring semantic locks** (holds with TTLs, like the airline's 30-minute hold) so even a wholly failed saga leaks no permanent resources. Dashboard: saga counts by state and age — stuck sagas should be a graph someone looks at, not a discovery.
- **Trade-offs of the fix:** Timeout-then-compensate races the late reply ("compensated, *then* the payment confirmation arrived"): the state machine needs late-arrival transitions (e.g. compensate the newly-confirmed step too — refund the late charge). More states, more tests — but these are exactly the states reality has; the design is just catching up.

### Pitfall: Sagas everywhere (the boundary smell)

- **What goes wrong:** Every feature seems to need a multi-service saga; simple operations take four network hops and an orchestrator; the team is drowning in compensation logic for workflows that feel like they should be… a transaction. Latency, debugging surface, and design overhead all balloon.
- **Why it happens (the mechanism):** Saga count is a *symptom*: service boundaries were drawn through the middle of things that change together (the [aggregate](../event-sourcing/learning.md) boundary question at system scale). When one business action routinely spans three services, the distribution — not the saga — is the problem. Conway's law and premature decomposition are the usual authors.
- **How to handle it in production, and why that works:** Treat each new saga as a design signal: *could this be one local transaction if the boundary moved?* Merging two chatty services (or moving an entity across the line) deletes the saga entirely — the best saga is the one made unnecessary. Reserve sagas for *genuinely* irreducible distribution: different trust domains (you + Stripe), different scaling/ownership realities, actual third parties. A modular monolith with real transactions beats a microservice archipelago of compensation logic at most team sizes.
- **Trade-offs of the fix:** Boundary changes are expensive and political; sometimes you saga on anyway with eyes open. But the calculus must be run — the pattern's cost is real, permanent, per-workflow overhead, and paying it to preserve an org chart is a choice worth making explicit.

## Design Decisions & Trade-offs

**Choreography vs. orchestration.** Short and stable → choreography (no new infrastructure; the outbox events you already publish are the mechanism). Long, branching, timeout-laden, or business-critical-to-observe → orchestration. The honest default for anything with money in it is orchestration: the workflow gets an address, a state you can query, and durable timers. Migrating choreography→orchestration later is routine; start simple *knowing* the line.

**Build vs. framework vs. workflow engine.** A hand-rolled orchestrator is a persisted state machine + outbox + timeout scanner — genuinely buildable, and the exercise teaches the pattern (see Open Questions). Frameworks (Axon, Eventuate, MassTransit sagas) give the state-machine scaffolding inside your app. Durable-execution engines (Temporal, Restate) go further: saga state persistence, timers, and retries become the platform, and your workflow is ordinary code — at the cost of a serious new infrastructure dependency and its failure modes. Choose by how many sagas you'll run: one saga → build; a product full of them → engine.

**Step ordering is risk ordering.** Beyond the pivot rule: order compensatable steps *cheapest-to-compensate first* and most-likely-to-fail *early* (fail before spending). Validation-ish steps (can this card even be charged? — an auth, not a capture) front-load failure discovery. The reserve/confirm pattern (auth-then-capture, hold-then-book) exists to *push the pivot as late as possible* — adopt it wherever a counterpart offers it.

**Observability is not optional.** Minimum: correlation id (`saga_id`) stamped through every command, event, and log line; a queryable state per saga instance (orchestrator table or choreography projection); dashboards of saga age/state distribution; alerts on stuck-past-SLA. During incidents, "show me all sagas touching order-991" is the question — build for it on day one.

**What sagas do not give you.** No isolation (countermeasures approximate it), no atomic visibility (observers see intermediates), no exactly-once (idempotency absorbs retries), and no escape from designing failure states with the business. A saga is *eventual consistency with a designed apology path* — that framing keeps expectations honest.

## Open Questions

- Build the toy: a Rust orchestrator as persisted state machine (sqlx + the outbox from that topic's notes) driving the travel example — where does the complexity actually concentrate? (Suspicion: timeout/late-reply races.)
- Temporal's model: how do durable timers and event-sourced workflow histories actually work internally, and what are the failure modes when the Temporal cluster itself degrades?
- Semantic-lock TTLs: how do real reservation systems pick hold durations, and what's the cleanup architecture for expired holds at scale?
- Late-reply-after-compensation: catalog the concrete state-machine transitions needed — is there a general pattern, or is it per-workflow design every time?
- How do event-sourced services and orchestrated sagas compose in practice — is the orchestrator itself best event-sourced (its state as a stream of saga events)?

## References

- Hector Garcia-Molina & Kenneth Salem, "Sagas" (SIGMOD 1987) — the original paper: long-lived transactions split into compensable steps; short, readable, and the vocabulary source.
- Chris Richardson, *Microservices Patterns*, ch. 4 — the modern treatment: choreography vs. orchestration, and the isolation countermeasures (semantic lock, commutative updates, version file) catalogued.
- [microservices.io/patterns/data/saga.html](https://microservices.io/patterns/data/saga.html) — the compact online version of the same material.
- Caitie McCaffrey, "Applying the Saga Pattern" (talk) — sagas in real distributed systems, with the failure-handling emphasis this document inherits.
- [Temporal documentation](https://docs.temporal.io/) — the durable-execution take: what saga infrastructure looks like when bought rather than built.
- Related topics in this repo: [Outbox Pattern](../outbox-pattern/learning.md) (the messaging substrate every step rides on), [Idempotency & Delivery Semantics](../idempotency-and-delivery-semantics/learning.md) (why every participant dedups), [Event Sourcing & CQRS](../event-sourcing/learning.md) (aggregates = the local-transaction boundary; cross-aggregate workflows land here), [Replication & Consistency](../replication-and-consistency/learning.md) (the vocabulary for what "eventual" promises).
