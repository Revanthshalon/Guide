# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose

This is a personal reference knowledge base of system architecture patterns — not a software project with a build/test/lint pipeline. Each top-level folder documents one architecture pattern (e.g. `event-sourcing/` for Event Sourcing & CQRS): what it is, its pitfalls, and how to handle those pitfalls in a production environment.

There is no code to build or run here. Work in this repo is writing/organizing Markdown documentation.

## Working in this repo

- When asked to "document" or "learn" a new architecture pattern, create a new top-level folder named after the pattern (kebab-case, e.g. `saga-pattern/`, `strangler-fig/`) with a `README.md` scaffolded like `event-sourcing/README.md`: Overview, Pitfalls (one subsection per pitfall), Production Mitigations (one subsection per pitfall, same names), References.
- Each pattern folder should cover: what the pattern is, common pitfalls, and concrete production mitigations for those pitfalls — this is the recurring structure the user wants across all patterns, since the goal is quick reference later.
- Keep documentation concrete and production-focused (real failure modes, real mitigations) rather than abstract textbook definitions — the user is building this to reference back when actually implementing these patterns.
- `event-sourcing/` covers Event Sourcing and CQRS together, since the user treats them as one combined topic.
