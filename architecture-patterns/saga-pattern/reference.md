# Saga Pattern — Quick Reference

Core model: no transactions across services → a saga is a sequence of local transactions, each with a designed compensating action; failure unwinds in reverse. Compensation ≠ rollback (history stands; intermediates were visible). Isolation is lost — countermeasures approximate it. Details in [learning.md](learning.md).

## When to Use / When Not

| Use it when | Avoid it when |
| --- | --- |
| One business action genuinely spans services/trust domains (you + Stripe) | The steps could be one local transaction if the service boundary moved — fix the boundary |
| Workflow has designable compensations or an acceptable pivot structure | A step's failure after the pivot can be *refused* (not just delayed) — reorder or redesign |
| Eventual consistency + visible limbo states are acceptable to the business | You actually need isolation/atomic visibility (rare, but real) |

## Step Zones (the design method)

| Zone | Rule | Example |
| --- | --- | --- |
| Compensatable (first) | Every step has a written, idempotent, un-refusable undo | ReserveFlight / CancelFlightHold |
| Pivot (once) | Point of no return; place as late as possible (auth-then-capture, hold-then-book) | ChargeCard |
| Retriable (after pivot) | Can only be delayed, never refused; retry until success | IssueTickets |

## Choreography vs. Orchestration

| | Choreography (event-chained) | Orchestration (state-machine executor) |
| --- | --- | --- |
| Fits | ≤3–4 steps, one failure path | Branching, timeouts, money, auditability |
| Workflow lives | Nowhere (smeared across subscriptions) | One queryable place |
| Cost | Hidden coupling, no "where is s-4412?" | A component to run and make HA |
| Default | Start here if trivial | Anything with money in it |

## Pitfalls → Mitigations

| Pitfall | Mitigation | Watch out for |
| --- | --- | --- |
| Compensations stubbed/afterthought | No step merges without written compensation or pivot designation; fault-inject every boundary in CI | Compensations that can fail for business reasons = no way back |
| Lost isolation (double-sell, phantom revenue) | Semantic locks: reserved/pending states, atomic claim inside one local txn, version checks | Read-then-write across step gaps is open season |
| Choreography past its ceiling | Migrate to orchestration at the 3–4 step line; subscription map in CI; saga-state projection | Event cycles between teams ping-pong forever |
| Stuck sagas (silent non-reply) | Timeout policy on every awaiting state; durable timers; TTL'd holds; age/state dashboard | Late reply after compensation needs explicit transitions |
| Sagas everywhere | Each saga is a boundary smell — ask if merging services deletes it | Paying permanent per-workflow cost to preserve an org chart |
| Duplicate commands/replies | Participants dedup on `saga_id + step`; orchestrator persists state via outbox | Every arrow is at-least-once; design for it |

## Production Checklist

- [ ] Compensation table written and product-reviewed before implementation
- [ ] Pivot placed as late as the domain allows; post-pivot steps un-refusable
- [ ] Every participant idempotent on `saga_id + step`
- [ ] All messaging outbox-backed (no dual writes anywhere in the flow)
- [ ] Timeout + action defined for every awaiting state
- [ ] Semantic locks self-expire (TTL) — failed sagas leak no resources
- [ ] `saga_id` correlation end to end; per-instance state queryable
- [ ] Dashboard: sagas by state and age; alert on stuck-past-SLA
- [ ] Fault-injection tests at every step boundary (compensation paths exercised in CI)

## Key References

- Garcia-Molina & Salem, "Sagas" (1987) — the original, short and readable.
- Richardson, *Microservices Patterns* ch. 4 — countermeasures catalog.
- [Temporal docs](https://docs.temporal.io/) — the bought-not-built option.
