# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose

This is a personal reference knowledge base — not a software project with a build/test/lint pipeline. It has two categories of reference documentation:

- `architecture-patterns/` — system architecture patterns (e.g. `event-sourcing/` for Event Sourcing & CQRS): what the pattern is, its pitfalls, and how to handle those pitfalls in a production environment.
- `oss-tools/` — open source software the user is evaluating or adopting, especially open-source alternatives to vendor/licensed tools (e.g. `openbao/` as an alternative to Vault, `opentofu/` as an alternative to Terraform): what it is, how it compares to the tool it replaces, its pitfalls, and migration considerations.

There is no code to build or run here. Work in this repo is writing/organizing Markdown documentation.

## Working in this repo

- Each category has a `_template.md` at its root (`architecture-patterns/_template.md`, `oss-tools/_template.md`). Always scaffold new entries from the matching template rather than improvising structure, so entries stay standardized and quick to scan later.
- When asked to "document" or "learn" a new architecture pattern, create a new folder under `architecture-patterns/` named after the pattern (kebab-case, e.g. `saga-pattern/`, `strangler-fig/`) with a `README.md` copied from `architecture-patterns/_template.md`.
- When asked to "document" or "learn" a new open source tool, create a new folder under `oss-tools/` named after the tool (kebab-case, e.g. `openbao/`, `opentofu/`) with a `README.md` copied from `oss-tools/_template.md`.
- Keep documentation concrete and production-focused (real failure modes, real mitigations, real migration gotchas) rather than abstract textbook/marketing descriptions — the user is building this to reference back when actually implementing these patterns or adopting these tools.
- `architecture-patterns/event-sourcing/` covers Event Sourcing and CQRS together, since the user treats them as one combined topic.
- Update `README.md`'s index whenever a new entry is added.
