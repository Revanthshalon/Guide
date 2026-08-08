# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose

This is a personal reference knowledge base — not a software project with a build/test/lint pipeline. It has four categories of reference documentation:

- `architecture-patterns/` — system architecture patterns (e.g. `event-sourcing/` for Event Sourcing & CQRS): what the pattern is, its pitfalls, and how to handle those pitfalls in a production environment.
- `oss-tools/` — open source software the user is evaluating or adopting, especially open-source alternatives to vendor/licensed tools (e.g. `openbao/` as an alternative to Vault, `opentofu/` as an alternative to Terraform): what it is, how it compares to the tool it replaces, its pitfalls, and migration considerations.
- `language-best-practices/` — per-language conventions and idioms (e.g. `rust/`): best practices, anti-patterns, and tooling. Rust is the primary language of focus.
- `performance-optimization/` — techniques for building high-performance applications (e.g. `cache-locality/`, `memory-layout/`, `simd/`): what the technique exploits, when it helps vs. hurts, implementation notes, and how to benchmark it. Code examples, crates, and tooling in this category should be Rust-first (other languages only when illustrating something Rust can't).

There is no code to build or run here. Work in this repo is writing/organizing Markdown documentation.

## Doc structure: learning vs. reference

Every topic folder contains exactly two documents serving different purposes:

- `learning.md` — study material, written to be read top-to-bottom and understood: mental models, mechanisms, worked examples, the "why" behind everything. This is where new material lands when the user is learning a topic.
- `reference.md` — quick-reference cheat sheet, scannable in under a minute: tables, checklists, rules of thumb, commands. Distilled from `learning.md`, never explanatory prose.

When adding content, put depth in `learning.md` and distill the actionable summary into `reference.md`. Don't duplicate explanations across both.

## Working in this repo

- Each category root has two templates: `_template-learning.md` and `_template-reference.md`. Always scaffold new entries from both templates (replace the `<...>` title placeholder) rather than improvising structure.
- When asked to "document" or "learn" a new topic, create a new folder under the relevant category named after the topic (kebab-case, e.g. `saga-pattern/`, `openbao/`, `cache-locality/`) containing `learning.md` and `reference.md` copied from that category's templates.
- Keep documentation concrete and production-focused (real failure modes, real mitigations, real migration gotchas, real benchmarking pitfalls) rather than abstract textbook/marketing descriptions — the user is building this to reference back when actually implementing these patterns, adopting these tools, or optimizing real code.
- `architecture-patterns/event-sourcing/` covers Event Sourcing and CQRS together, since the user treats them as one combined topic.
- Update `README.md`'s index whenever a new entry is added.

## Cross-linking convention

- Reference other docs with folder/file paths in kebab-case, matching the actual path on disk (e.g. `architecture-patterns/event-sourcing/learning.md`, `oss-tools/openbao/reference.md`).
- Use inline Markdown links (`[text](path)`) only. Do not use reference-style/alias link definitions (e.g. `[text][ref]` with a separate `[ref]: path` line).
