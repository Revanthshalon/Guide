# System Architecture Reference

A personal reference library covering:

- **Architecture patterns** — what they are, where they break in production, and how to handle it.
- **Open source tools** — especially open-source alternatives to vendor/licensed tools, how they compare, and what to watch for when adopting them.

Every entry is scaffolded from a standard template so the docs stay consistent and fast to scan.

## Architecture Patterns

Template: [`architecture-patterns/_template.md`](architecture-patterns/_template.md)

Each entry covers: Overview, Pitfalls, Production Mitigations, References.

- [Event Sourcing & CQRS](architecture-patterns/event-sourcing/README.md)

## OSS Tools

Template: [`oss-tools/_template.md`](oss-tools/_template.md)

Each entry covers: Overview, Comparison (vs. the tool it replaces), Pitfalls, Production Mitigations, Migration Notes, References.

- [OpenBao](oss-tools/openbao/README.md) — alternative to Vault
- [OpenTofu](oss-tools/opentofu/README.md) — alternative to Terraform
