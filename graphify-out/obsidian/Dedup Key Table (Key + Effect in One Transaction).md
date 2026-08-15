---
source_file: "architecture-patterns/idempotency-and-delivery-semantics/reference.md"
type: "rationale"
community: "Delivery Semantics and Idempotency"
tags:
  - graphify/rationale
  - graphify/EXTRACTED
  - community/Delivery_Semantics_and_Idempotency
---

# Dedup Key Table (Key + Effect in One Transaction)

## Connections
- [[Effectively-Once (At-Least-Once + Idempotent Receiver)]] - `implements` [EXTRACTED]
- [[Ledger Keyed by Business Id (Deltas vs Absolutes)]] - `conceptually_related_to` [EXTRACTED]
- [[The Inbox Pattern (Consumer-Side Dedup)]] - `implements` [EXTRACTED]

#graphify/rationale #graphify/EXTRACTED #community/Delivery_Semantics_and_Idempotency