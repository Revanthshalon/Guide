# Strangler Fig — Learning Notes

## Mental Model

**Replace a legacy system incrementally, by intercepting its calls at a boundary and redirecting them slice by slice to new implementations, until nothing routes to the old system and it can be deleted.**

The name is Martin Fowler's, from the strangler fig vine that germinates in a host tree's canopy, sends roots down around the trunk, and gradually envelops it — until the host rots away and the fig stands as a hollow column in its shape. The metaphor is precise in a way that matters: the fig grows *around* the tree while the tree is still alive and functioning, and there is never a moment when neither is holding the canopy up.

The alternative — the Big Bang rewrite — fails for reasons that are well documented and structural rather than about competence:

- **The target moves.** The legacy system keeps shipping features during the rewrite, so the rewrite is chasing a system that is itself changing.
- **All risk lands on one day.** Value is delivered only at cutover, so there's no feedback until the riskiest moment, and no partial success.
- **The requirements live in the old code.** Not in documents — in twenty years of edge cases, workarounds, and undocumented behavior that customers depend on. A rewrite from specification reproduces the specification, not the system.
- **There is no rollback.** Once the old system is switched off and data has diverged, going back is another migration.

The strangler fig's actual product is **not the new system — it's reversibility.** It converts one enormous irreversible event into many small reversible ones, each independently deployable, verifiable, and revertible. Every design decision below should be judged by whether it preserves that property.

Three consequences that shape everything that follows:

1. **No interception point, no strangler fig.** The pattern requires a place where you can redirect a call without the caller knowing: an HTTP proxy, a facade class, a feature flag, a message router, a database view. If no such seam exists, *creating one is the first slice* — and sometimes the hardest.
2. **The data is the hard part, not the code.** Rewriting behavior is tractable; migrating the state it operates on, while both systems are live and writing, is where these projects actually stall. Plan the data strategy before the first slice, not after the third.
3. **The pattern has a distinct, commonly-skipped final step: deleting the old system.** A strangler migration that reaches 90% and stops has produced *two* systems to maintain, permanently — strictly worse than either the original or a completed rewrite. Most failures of this pattern are failures to finish, not failures to start.

## Core Concepts

### The interception point (the seam)

- **What it is:** The layer at which traffic can be routed to old or new implementations per slice, transparently to callers. Common forms: an HTTP reverse proxy or API gateway (route by path/header), a facade or adapter class inside the monolith (Michael Feathers' "seam"), a feature flag around a call site, a message router directing events to one consumer or the other, a database view masking a table swap.
- **Why it exists:** It is the mechanism that makes incremental replacement possible *and* reversible — flipping a slice back is a routing change, not a deploy. Its granularity is also a constraint you inherit: a proxy that can only route by URL path forces slices to be path-shaped; a facade at a class boundary allows finer, method-level slices.
- **Example:** Put a reverse proxy in front of the monolith on day one, routing 100% to legacy. That deploy changes no behavior and is trivially revertible — but it establishes the control point every later slice depends on. This "no-op first slice" is the standard opening move, and skipping it is why some migrations never find a clean way to start.

### Slicing strategy

- **What it is:** How the system is cut into independently migratable pieces. The main axes: **by route/endpoint** (simplest with a proxy), **by domain/bounded context** (aligned with data ownership — usually the *right* cut), **by operation type** (reads before writes), and **by user cohort** (internal users, then 1%, then 10% — orthogonal to the others and combinable with all of them).
- **Why it exists:** Slice size is the direct control on risk: each slice should be small enough that its failure is survivable and its rollback is boring. Reads-before-writes is the highest-value ordering heuristic because a read slice is *idempotent and verifiable* — you can run it in parallel with the old path and compare outputs without any consequence to state.
- **Example:** For an orders monolith: `GET /orders/{id}` first (read-only, comparable, trivially revertible), then `GET /orders` (list — pagination edge cases), then `POST /orders` (write — needs the data strategy), then the batch jobs (usually last and usually forgotten in planning).

### The data strategy (the hard part)

- **What it is:** How state moves from old to new while both systems are live. The usual progression: (1) **shared database** — the new service reads and writes the legacy schema; (2) **new system owns new data**, with sync back to legacy via [CDC](../change-data-capture/learning.md) or [outbox](../outbox-pattern/learning.md) events; (3) **dual write** during a transition window with reconciliation; (4) **full ownership** — legacy tables become read-only, then dropped.
- **Why it exists:** Code can be swapped in an afternoon; data cannot. Sharing the database initially is what makes the first slices cheap — the new service is just another writer to the same tables, so no migration is needed to get started. But a shared database is also *permanent coupling*: two systems bound to one schema, neither able to evolve it independently. The migration is therefore a two-phase thing — get behavior out first (cheap, shared DB), then get data out (expensive, and the phase that gets abandoned).
- **Example:** The direction of sync is the decision that matters most. **New-system-owns with sync back to legacy** is strongly preferable to the reverse, because it makes progress monotonic: each migrated entity permanently leaves legacy's ownership. Legacy-owns-with-sync-forward feels safer and produces a system that can never finish, because the new side is always a replica.

### Verification: shadow traffic and parallel run

- **What it is:** Running the new implementation against real production traffic *without* its results being used. **Shadowing/dark traffic**: send a copy of each request to the new path, discard the response, compare and log differences. **Parallel run**: execute both, serve the old result, alert on mismatches. **Canary**: serve the new result to a small percentage.
- **Why it exists:** The legacy system's behavior is the actual specification, including bugs that callers depend on. Shadowing tests the new implementation against the only complete source of truth available — real traffic — before anything depends on it. The comparison output is also a *requirements discovery tool*: every mismatch is either a bug in the new code or an undocumented behavior in the old one, and both are worth finding before cutover rather than after.
- **Example:** Shadow `GET /orders/{id}` for a week and diff the JSON: you'll find date-format differences, field-ordering assumptions callers depend on, a rounding discrepancy, and one endpoint that returns 200-with-empty rather than 404. All of it cheap to fix now. Two cautions: shadowed *writes* must be either safe or suppressed (a shadowed `POST` that actually charges a card is a very bad day), and shadowing doubles load on shared dependencies.

### The anti-corruption layer

- **What it is:** A translation layer between the legacy model and the new one (Eric Evans' term), so the new system speaks its own domain language and converts at the boundary rather than importing legacy's concepts.
- **Why it exists:** Without it, the new system inherits the old data model — the misnamed columns, the overloaded status field, the entity that means three different things depending on a flag — and you've rebuilt the monolith with newer syntax. The whole justification for the migration (a better model) evaporates while all the cost remains. The ACL is what makes the new system genuinely new rather than a port.
- **Example:** Legacy has `orders.status` as a string with eleven values, three of which are unreachable and two of which mean the same thing. The new service models an explicit state machine and translates in its adapter. When legacy is finally deleted, the ACL is deleted too — its lifetime is exactly the migration's. Note the cost: two models and a mapping to maintain during the transition, which is real work that must be planned for.

### The deletion plan

- **What it is:** The explicit commitment — with owner, dates, and tracking — to remove legacy code, tables, and infrastructure once a slice's traffic is at 100% and stable. Including the observability to *prove* nothing calls it anymore.
- **Why it exists:** Deletion is the only step that realizes the benefit. Until the old path is gone you are paying for both systems: two deploy pipelines, two on-call surfaces, two places every new feature might need to change, and the ongoing cognitive load of "which one handles this?" Because deletion delivers no visible feature, it loses every prioritization argument unless it's committed to in advance.
- **Example:** Per slice: after 100% traffic for two weeks with no rollback, (a) verify zero calls to the legacy path via metrics/logging on the old code path, (b) delete the code, (c) drop the routing rule, (d) drop the legacy tables after a backup-verified retention window. Track "slices migrated" *and* "slices deleted" as separate numbers — the gap between them is the debt the migration is accruing.

## Worked Example

An order-management monolith: 200 K lines, one database, twelve years old, the team that wrote it is gone. Goal: extract order management into a new service.

**Stage 0 — establish the seam (no behavior change).**

```
clients → [reverse proxy] → monolith          100% legacy, zero risk
```

A deploy that changes nothing, and the control point everything else needs. Add per-route metrics here now — you'll want the baseline for comparison later.

**Stage 1 — the first slice, read-only, shadowed.**

```
GET /orders/{id}:
  proxy → monolith (served to client)
        ↘ new order-service (response discarded, diffed against legacy)

week 1 diffs: 4.2% mismatch
  - timestamps: legacy emits local time without offset (callers parse it that way)
  - `discount` absent vs null for zero discounts
  - one 200-with-empty-body where new returns 404
week 2 after fixes: 0.01% mismatch (all legitimate races on concurrent updates)
```

Those three findings are the requirements nobody could have told you. Then shift traffic: 1% → 10% → 50% → 100%, with an instant revert at every step, watching error rate and latency against the Stage 0 baseline.

**Stage 2 — the first write slice, with the data question.**

`POST /orders` writes state, so shadowing isn't free and rollback isn't just a routing flip. The chosen strategy — new service owns new orders, syncs back to legacy so untouched legacy code keeps working:

```
POST /orders → new order-service
                 ├─ writes to the NEW schema (its own database)
                 └─ outbox → relay → legacy sync consumer → writes legacy tables
                    (so legacy reporting, batch jobs, and un-migrated code still see it)

GET /orders/{id} → new service: check new store, fall back to legacy for old orders
```

Two things make this work rather than become a distributed-consistency nightmare. The [outbox](../outbox-pattern/learning.md) means "order created" and "legacy will be told" commit atomically — no dual write. And the sync direction is *new → legacy*, so every order created after the flip permanently belongs to the new system: **progress is monotonic**. A daily reconciliation job compares counts and checksums across both stores and alerts on divergence, because a sync pipeline that fails silently produces exactly the split-brain this pattern is supposed to avoid.

**Stage 3 — migrate the historical data and cut the tie.**

```
1. backfill legacy orders into the new schema (resumable, chunked, idempotent)
2. verify: row counts, checksums, spot-diff a sample
3. flip reads fully to the new store (legacy fallback removed)
4. reverse the sync OFF — legacy tables become read-only
5. repoint remaining legacy consumers (reporting, that one batch job) at the
   new service's API or its event stream
```

Step 5 is where these projects discover the consumers nobody knew about — a nightly export, a BI tool with direct database credentials, a partner integration reading a view. Finding them requires *instrumenting the legacy tables* (audit logging on reads, or connection-source tracking) before assuming the list is complete.

**Stage 4 — delete (the step that realizes the benefit).**

```
per slice, after 2 weeks at 100% with no rollback:
  - confirm zero invocations of the legacy path (log-on-call, then check for silence)
  - delete the legacy controller, service, and DAO code
  - remove the proxy's legacy route
  - drop legacy tables after the backup-retention window
```

**Scoreboard for the whole migration:** twelve slices, each independently reverted at least once during rollout (the reversibility being used, as designed), zero big-bang cutover nights, and — critically — twelve slices *deleted*. The migration is over because there's nothing left to route to.

## Pitfalls in Depth

### Pitfall: Never finishing (the dominant failure mode)

- **What goes wrong:** The migration reaches 70–90% and stops. The remaining slices are the hard ones — the batch jobs, the reporting integration, the one endpoint with bizarre semantics — and each is individually unattractive next to new feature work. The organization now runs two systems permanently: two deploy pipelines, two on-call rotations, and every new feature requiring a decision about which system owns it (and often changes to both).
- **Why it happens (the mechanism):** The pattern's own strength causes it. Because each slice delivers value independently, the *marginal* value of the next slice keeps declining while its difficulty rises — so at every decision point, "do the next slice" loses to "ship the feature." Meanwhile deletion delivers no visible value at all and loses every prioritization argument. The migration doesn't get cancelled; it gets deprioritized indefinitely, which looks identical in six months.
- **How to handle it in production, and why that works:** Commit to completion up front, in a way that survives priority shifts: a named owner for the *whole* migration (not per-slice), a target date for legacy decommission treated as a real deliverable, and — the mechanism that actually works — **a feature freeze on the legacy system**. If new work can only land in the new system, the incentive inverts: teams migrate their slice because it's the only way to ship. Track "slices deleted" separately from "slices migrated," and make the gap visible in the same review where feature progress is discussed. Order slices so the hardest ones come *earlier* than their value alone would justify, because their difficulty won't decrease and your appetite will.
- **Trade-offs of the fix:** A legacy feature freeze has real business cost and needs executive backing; getting that requires being explicit that the alternative is paying double maintenance forever. Doing hard slices early front-loads risk, which is in tension with the reads-first heuristic — resolve by doing one easy slice first to build the machinery, then attacking the hard ones.

### Pitfall: The shared database that never gets split

- **What goes wrong:** Both systems read and write the same tables. This is fine and correct as a *starting* position — it's what makes early slices cheap. But it becomes the permanent state: neither system can change the schema without coordinating with the other, the new service is bound to the legacy data model it was supposed to escape, and "extract the data later" never gets scheduled because the system works.
- **Why it happens (the mechanism):** Behavior extraction produces visible progress (endpoints migrated, dashboards moving); data extraction produces none — the system behaves identically before and after, at high risk and high effort. So the phase that provides the actual decoupling is the one that's invisible to everyone deciding priorities. Meanwhile the shared schema keeps working, so nothing forces the issue.
- **How to handle it in production, and why that works:** Treat the shared database as an explicitly *temporary* state with a written expiry, and make data ownership the unit of slicing (bounded contexts, not endpoints) so each slice's completion includes its data. Enforce direction: new system owns migrated entities, syncs *back* to legacy — so ownership only ever moves one way. Add a hard rule that the legacy schema is frozen for new columns; anything new lives in the new store. Track, per entity, which system is authoritative — a table anyone can read that makes the remaining coupling countable.
- **Trade-offs of the fix:** Splitting data means cross-system queries during the transition (joins that used to be free become API calls or event-fed replicas), and dual-write windows with reconciliation. That cost is genuinely large, which is why it must be planned and funded up front rather than discovered mid-migration.

### Pitfall: Slices too big (mini big-bangs)

- **What goes wrong:** "The orders module" is treated as one slice — three months of work, forty endpoints, six tables, cut over in one weekend. It fails at 2 a.m. for a reason unrelated to anything tested, the rollback touches data, and the team is now debugging a system where both old and new paths have been partially exercised.
- **Why it happens (the mechanism):** Slicing is done along *architectural* lines (modules, layers) rather than along the lines that preserve reversibility. Big slices also feel more efficient — less proxy configuration, less duplicated plumbing, fewer intermediate states — and that efficiency argument is correct about effort and wrong about risk.
- **How to handle it in production, and why that works:** The sizing test is "can this be rolled back with a routing change and no data cleanup?" If not, it's too big or the data strategy isn't ready. Prefer weeks over months; a slice that can't be finished in a sprint or two should be split further. Reads before writes, low-traffic endpoints before high, internal consumers before external. If the intermediate state feels awkward — old and new both partially handling something — that awkwardness is the price of reversibility and it's temporary.
- **Trade-offs of the fix:** More slices means more total plumbing, more intermediate states to reason about, and more routing configuration. It's a real overhead and it buys the property the pattern exists for.

### Pitfall: No rollback, so the pattern's benefit is fictional

- **What goes wrong:** Slices are migrated but each cutover is effectively one-way: the new system has written data the old one can't read, the feature flag was removed a day after the flip, or the legacy code path was deleted immediately. When a problem surfaces three days later, the only options are fix-forward under pressure or a data migration in reverse.
- **Why it happens (the mechanism):** Rollback capability is invisible when it isn't used, so it's the first thing dropped for expediency — and the pressure to clean up the "temporary" dual path arrives immediately after a successful flip, when confidence is highest and the risk window isn't over.
- **How to handle it in production, and why that works:** Make rollback an explicit, *tested* deliverable per slice: before shifting traffic, verify the revert works in staging with representative data, and state how long the rollback window stays open (typically 1–2 weeks at 100%). Keep the legacy path *executable* for that window, not just present. For write slices, ensure the sync-back keeps legacy's data current enough that reverting is viable — which is precisely what the sync direction buys you. Then delete deliberately at the end of the window (see the deletion plan) rather than opportunistically.
- **Trade-offs of the fix:** Maintaining both paths for a defined window costs the duplication the migration is trying to end, temporarily. Bounding the window with a date is what keeps "temporarily" honest.

### Pitfall: The new system inherits the old model

- **What goes wrong:** To move fast, the new service reuses legacy's schema, field names, and semantics — the eleven-value status string, the nullable columns that encode state, the entity that means different things by flag. Migration completes; the new system is a monolith in a new repository with the same modeling problems, and the promised improvements never materialize.
- **Why it happens (the mechanism):** Every deviation from the legacy model requires translation work *now* for benefit *later*, and during a migration under time pressure the direct port is always the cheaper next step. There's also a correctness argument for it — matching legacy exactly reduces behavioral risk — which is genuinely true and quietly converts the project from "replace" to "relocate."
- **How to handle it in production, and why that works:** Put an **anti-corruption layer** at the boundary from the first slice: the new service models the domain properly and translates in its adapter. Keep the ACL's lifetime explicitly tied to the migration — it is deleted when legacy is. Be honest about which slices deserve remodeling: for a genuinely-fine legacy model, a direct port is correct, and forcing novelty is its own waste. The decision should be per bounded context and written down, not defaulted.
- **Trade-offs of the fix:** Two models and a mapping during transition — real complexity and a real source of translation bugs, which is exactly why the ACL should be well-tested and narrow. Remodeling also makes behavioral diffing harder (outputs differ by design), so the verification story needs adjusting for those slices.

### Pitfall: Flipping without verification

- **What goes wrong:** Traffic is shifted based on passing tests and a staging run. Subtle behavioral differences — a date format, a rounding rule, a field that was always present — break integrations that were never documented and don't fail loudly. The mismatch is discovered days later by a partner, a report, or a customer.
- **Why it happens (the mechanism):** Tests encode *known* requirements; a legacy system's contract includes everything callers observed and depended on, most of which is unknown and undocumented. Staging traffic doesn't contain the weird cases; production traffic is the only complete specification available.
- **How to handle it in production, and why that works:** Shadow every read slice against real traffic and **diff structurally** (not string equality — normalize ordering and known-irrelevant fields) for at least a full business cycle, so weekly and monthly patterns are included. Treat every mismatch as a finding to explain, not a number to reduce: some are new-code bugs, some are legacy bugs you must decide whether to replicate (often yes, at least initially — callers depend on them). For writes, shadow with side effects suppressed, or parallel-run and compare resulting state rather than responses. Then shift traffic in stages with automatic revert on error-rate or latency regression against the Stage 0 baseline.
- **Trade-offs of the fix:** Shadowing doubles load on shared dependencies (size for it, or sample) and the comparison harness is real engineering. The payoff is discovering the undocumented contract while nothing depends on you being right.

## Design Decisions & Trade-offs

**Establish the interception point as slice zero.** A no-op proxy deploy that routes 100% to legacy is the cheapest, safest, highest-leverage first change — and its absence is why some migrations can't find a starting move. Add per-route metrics at the same time; you need the baseline before you change anything.

**Slice by bounded context, sequence by risk.** Domain boundaries are the cut that lets data ownership move with the behavior — endpoint-shaped slices tend to leave data behind. Within that, sequence: reads before writes, low-traffic before high, internal consumers before external. Do one easy slice first to build the machinery (routing, shadowing, comparison, rollback), then deliberately pull the *hard* slices earlier than their value justifies, because they won't get easier and enthusiasm will not increase.

**Sync direction is the decision that determines whether you finish.** New-owns-with-sync-back-to-legacy makes ownership transfer monotonic and each slice permanent. Legacy-owns-with-sync-forward feels safer, keeps legacy authoritative, and produces a migration with no natural end. Use [outbox](../outbox-pattern/learning.md) or [CDC](../change-data-capture/learning.md) for the sync so there's no dual write, and run reconciliation continuously — a silently-failing sync is the worst outcome available.

**Bound every temporary state with a date and an owner.** Shared database, dual paths, sync pipelines, anti-corruption layers, feature flags: all are legitimate transitional mechanisms and all become permanent by default. Each needs an expiry in the plan and a named person, or the migration's end state is "both systems, forever, plus scaffolding."

**Budget for the discovery phase.** Finding every consumer of the legacy data — the nightly export, the BI tool with direct database access, the partner integration — requires instrumentation (read auditing, connection-source logging) and takes longer than expected. Assume the documented list is incomplete, because it is.

**Know when not to use this pattern.** For a small system (a few weeks to rewrite), the strangler's overhead — proxy, dual paths, sync, comparison harness — exceeds the risk it removes; just rewrite it behind a flag. For a system being decommissioned rather than replaced, migrate the data and turn it off. And if no interception point can be created at any reasonable cost, this pattern isn't available and the honest options are a big-bang with a serious rollback plan, or leaving it alone.

## Exercises & Self-Test

Answer from the model, then check against the doc:

1. Name the four structural reasons big-bang rewrites fail, and state what the strangler fig's actual product is (it isn't the new system).
2. What is an interception point, why is it a prerequisite rather than a detail, and what should slice zero be?
3. Why is "reads before writes" the highest-value sequencing heuristic? What property of read slices makes it work?
4. Compare the two sync directions during data migration. Explain precisely why one produces a migration that can finish and the other doesn't.
5. Give the sizing test for a slice. What does failing it tell you about either the slice or the data strategy?
6. Shadow traffic finds a 4% mismatch rate. Enumerate the categories a mismatch can fall into and what you'd do about each — including the case where legacy is wrong.
7. Why is "never finishing" the dominant failure mode, in incentive terms? Name the intervention that most reliably fixes it and what it costs.
8. What is an anti-corruption layer, when should you *not* build one, and when is it deleted?

Design exercises:

- Take a real legacy system you know and identify its interception point. If there isn't one, design the cheapest change that creates one — that design *is* the first slice.
- Write the slice list with sequencing and, for each, its rollback mechanism and data strategy. The slices whose rollback you can't describe are the ones hiding a data problem.
- For one slice, write the deletion criteria: what evidence proves nothing calls the legacy path, who deletes it, and when. Most migration plans have no such section — its absence predicts the dominant failure mode.

## Open Questions

- Shadow-traffic tooling: is there something off-the-shelf for structural response diffing at scale (normalizing ordering, ignoring known-variant fields), or is this always bespoke?
- Discovering unknown consumers of legacy tables: what actually works — database audit logging, connection-source analysis, network capture — and at what operational cost?
- Feature-freeze politics: how do teams that succeeded actually get the freeze approved, and what escape valves (critical bugs only?) keep it credible?
- Reconciliation job design for the dual-write window: continuous streaming comparison versus periodic batch checksums — what do real migrations use and how do they handle legitimate in-flight divergence?
- Does the pattern change materially when the legacy system is a third-party product (no code access, only its API and database)? Which techniques survive?

## References

- Martin Fowler, ["StranglerFigApplication"](https://martinfowler.com/bliki/StranglerFigApplication.html) — the original naming and the metaphor, short and worth reading first.
- Sam Newman, *Monolith to Microservices* — the most complete practical treatment: interception patterns, database decomposition strategies (the hard part), and the sequencing arguments; the reference for this doc's data section.
- Michael Feathers, *Working Effectively with Legacy Code* — "seams" and how to create interception points in code that wasn't designed for them; the missing prerequisite when no clean boundary exists.
- Eric Evans, *Domain-Driven Design* — the anti-corruption layer and bounded contexts, i.e. how to slice by domain rather than by module.
- GitHub, Shopify, and Stripe engineering write-ups on their monolith extractions — the honest operational accounts, especially about consumer discovery and how long the tail takes.
- Related topics in this repo: [Change Data Capture](../change-data-capture/learning.md) + [Outbox Pattern](../outbox-pattern/learning.md) (the sync machinery, without dual writes), [Event-Driven Architecture](../event-driven-architecture/learning.md) (how migrated services integrate with what remains), [Sharding](../sharding/learning.md) (the dual-write/backfill/verify/cutover sequence rhymes exactly), [Load Balancing & Service Discovery](../load-balancing-and-service-discovery/learning.md) (traffic shifting and canary mechanics), [Replication & Consistency](../replication-and-consistency/learning.md) (what "synced" actually promises during the transition).
