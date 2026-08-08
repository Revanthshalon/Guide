# <Tool Name> — Setup & Operations Runbook

<!-- Procedural, not explanatory: the commands and configs to run, in order, with the
     failure each step prevents. Concepts live in learning.md; scannable command
     lookup lives in reference.md. Add an accuracy note if the tool moves fast. -->

> **Accuracy note:** <version/date this reflects; where to verify>. Concepts are in [learning.md](learning.md); this is the procedure.

## Part 1 — Development setup

<!-- Dev mode / local install. Include a table of what dev mode does differently and
     why each difference disqualifies it for production. End with a minimal working
     example, in Rust where the tool has an API. -->

## Part 2 — Production installation

<!-- Topology decision table, host prerequisites, the annotated config file, service
     unit. Call out the handful of settings that most often break a first deployment. -->

## Part 3 — Initialization / bootstrap

<!-- One-time ceremonies, and what to do with any credentials they emit. -->

## Part 4 — Day-1 hardening

<!-- The ordered steps before the tool holds anything real. Order usually matters —
     say why. -->

## Part 5 — <Tool-specific core operation>

<!-- The thing operators get wrong most: policy authoring, state locking, etc. -->

## Part 6 — Integration

<!-- How applications/pipelines actually talk to it, including the credential-bootstrap
     problem and its correct answer. -->

## Part 7 — Day-2 operations

<!-- Backups (with a restore drill), upgrades (with ordering), rotation, monitoring
     table of signals and alert thresholds. -->

## Part 8 — Dev → production checklist

<!-- Checkbox list grouped by phase. Every unchecked line should map to a known way
     to get hurt. -->

## Common mistakes → what actually happens

| Mistake | Consequence |
| --- | --- |
| | |

## References

<!-- Official docs first, then the repo's related topics. -->
