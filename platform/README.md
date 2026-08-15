# platform

Rust workspace for the healthcare operations platform. Everything else in this
repository is reference documentation; this directory is the code.

```
platform/
└── crates/
    └── fhir-core/     FHIR R5 foundational types: primitives, elements,
                       datatypes, and the Patient resource
```

## fhir-core

The schema layer. FHIR R5 is the DTO vocabulary; Rust's type system carries the
constraints the specification states in prose.

- **Primitives are validating newtypes.** `Id`, `Code`, `Uri`, `Date`,
  `Decimal`, `Base64Binary`, `Xhtml`, and the rest validate in their
  constructors, including via `serde`. There is no way to hold an invalid one.
- **Reference targets are type parameters.** `Reference<Organization>` and
  `Reference<(Organization, Practitioner, PractitionerRole)>` — the compiler
  picks which check runs.
- **Required bindings are enums**, extensible bindings stay `CodeableConcept`,
  so real-world data still parses.
- **Everything a type cannot express is a validation pass** that walks the tree,
  collects *every* issue with its FHIRPath location, and renders as an
  `OperationOutcome`.

See [`crates/fhir-core/DESIGN.md`](crates/fhir-core/DESIGN.md) for why each
decision was made and what is deliberately not implemented yet.

```rust
use fhir_core::prelude::*;

let patient = Patient::builder()
    .id("example".parse()?)
    .identifier(
        Identifier::new(
            "http://hospital.example.org/mrn".parse()?,
            "MRN-0001".parse()?,
        )
        .with_use(IdentifierUse::Official),
    )
    .name(HumanName::new("Doe".parse()?, vec!["Jane".parse()?]).with_use(NameUse::Official))
    .gender(AdministrativeGender::Female)
    .birth_date("1974-12".parse()?)          // partial precision is preserved
    .build_validated()?;                     // -> Validated<Patient>
```

## Working in the workspace

```sh
cd platform
cargo test          # unit, integration, and doc tests
cargo clippy --all-targets
cargo fmt
cargo doc --open    # the type docs are the reference material
```

Requires Rust 1.85 or later (edition 2024).

## Related documentation in this repository

- [language-best-practices/rust](../language-best-practices/rust/learning.md) —
  the conventions this code follows, and its testing and benchmarking practice.
- [architecture-patterns](../architecture-patterns/LEARNING-INDEX.md) —
  event sourcing, outbox, idempotency, and the rest of the patterns this
  platform will be built on.
- [oss-tools/postgres](../oss-tools/postgres/learning.md) — the intended store.
