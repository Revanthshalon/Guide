# fhir-core — design decisions

Why the crate looks the way it does, and what was deliberately left out. Read
this before changing a type's shape; most of the odd-looking choices are load
bearing.

## 1. Two failure boundaries, two error types

| | `ParseError` | `ValidationError` |
| --- | --- | --- |
| Raised by | primitive constructors, `serde` deserialization | `Validate::validate` |
| Means | this text is not a legal value of this type | these elements are legal but wrong together |
| Carries a path | no — a primitive does not know where it lives | yes, FHIRPath-style |
| Stops at first problem | yes | **no** — collects everything |

The split exists because the two happen at different times. A `ParseError` can
only occur at the system boundary; once a `Date` exists it is a date forever.
A `ValidationError` can be produced from a value already in memory, at any point,
and is what an API returns as an `OperationOutcome`
(`ValidationError::to_operation_outcome`).

Validation collects rather than short-circuits: a client fixing one field per
round trip is a client making ten round trips.

## 2. Newtypes only where they earn their place

`boolean` is `bool` and `integer` is `i32`, behind aliases. Neither constrains
anything the Rust type does not already, and wrapping them would cost ergonomics
for nothing.

Everything else is a newtype, including `string` — FHIR's `string` forbids
whitespace-only values and caps length at 1 MiB, and `Id` forbids everything
outside `[A-Za-z0-9\-\.]{1,64}`. The rule applied throughout: a newtype must
either constrain the value or prevent a mix-up between two values with the same
representation (`Uri` versus `Url` is the second kind).

Validation is hand-written character scanning, not regex. The FHIR patterns are
simple, and a scan can report *which* character failed and at what index, which
a regex match cannot.

## 3. Decimal keeps its lexical form

FHIR states that the number of significant digits in a `decimal` is meaningful:
`1.50` reports a different measurement precision than `1.5`. So `Decimal` stores
the digits as written and re-emits them unchanged, exposing `as_f64()` as an
explicitly lossy escape hatch and `numeric_cmp` for numeric ordering.

Serialization uses `serde_json`'s `RawValue` to emit a bare JSON number carrying
the stored digits — going through `f64` would defeat the whole type.

**Known limit on the way in.** When `serde_json` parses `1.50` it produces an
`f64` before this crate sees it, so a value that arrives over the wire comes back
as `"1.5"`. Fixing this end-to-end requires `serde_json`'s `arbitrary_precision`
feature (or a `RawValue`-based custom deserializer); the test
`decimal_precision_survives_the_round_trip` pins the current behaviour so a
change to it is visible.

## 4. Partial dates are partial

`Date`, `DateTime`, and `Instant` keep both the lexical form and the precision.
`birthDate: "1974"` stays `1974`; normalising it to `1974-01-01` would invent a
January birthday that then propagates into every downstream report.

The consequence is that dates are **not** `PartialOrd`. Comparison is
`chronological_cmp`, returning `Option<Ordering>`, where `None` means *the
comparison is genuinely indeterminate* — `1974` versus `1974-12-25` overlaps.
An operator that returned `false` there would be quietly wrong. `per-1` therefore
reports a violation only on a definite `Greater`.

`Instant` is fully specified by definition and so *is* totally ordered.

## 5. Reference targets are type parameters

FHIR states reference constraints in prose; here they are types:

```rust
Reference<Organization>                                    // exactly one target
Reference<(Organization, Practitioner, PractitionerRole)>  // several
Reference<Any>                                             // Reference(Any)
```

`ReferenceTarget` is implemented for each marker and for tuples up to six.
Resources this crate has not modelled yet get a zero-sized `marker::*` type
implementing `ResourceMarker`; when the real struct lands it implements the same
trait and the marker is deleted, leaving every `Reference<Organization>` at call
sites meaning exactly what it meant before.

The check still runs at runtime — the literal string arrives from JSON — but the
*choice of which check* is made by the compiler rather than by whoever wrote the
call site.

A reference to a resource type this build does not know (`CareTeam/7`) is a
**warning**, not an error. A conformant server may reference more of FHIR than we
model, and rejecting the record would be our bug, not theirs.

## 6. Enums only for required bindings

`Patient.gender` is required-bound, so `AdministrativeGender` is an enum and an
unknown code is a parse error. `Patient.maritalStatus` is *extensible*, so it
stays a `CodeableConcept`: modelling an extensible binding as a closed enum
means rejecting data the specification explicitly permits, which is the classic
way a strongly-typed FHIR client becomes unable to ingest real records.

## 7. Base types as traits, fields declared per struct

`Element` and `BackboneElement` are traits, implemented by macro. The fields
(`id`, `extension`, `modifierExtension`) are declared on each struct rather than
inherited from an embedded `ElementBase` with `#[serde(flatten)]`, so that the
Rust field list matches the FHIR element list one-for-one and the JSON mapping
stays obvious.

`Patient` implements `DomainResource`, not `Element`: a resource's `id` is the
FHIR `id` type (URL-safe, ≤ 64 chars) while `Element.id` is a plain string.
Sharing an accessor would mean weakening `Resource::id`.

`modifierExtension` is a separate field from `extension` because the safety rule
differs: an unrecognised extension may be ignored, an unrecognised *modifier*
extension may not — it can invert the meaning of the element containing it.

## 8. Choice types are enums, flattened

`deceased[x]` is `Option<PatientDeceased>` with `#[serde(flatten)]` over an
externally-tagged enum, which produces exactly `{"deceasedBoolean": true}`.
The alternative — one optional field per type — makes "at most one is set"
a runtime check instead of a type-level fact.

## 9. `Validated<T>` is the proof

`Validated<Patient>` can only be produced by a successful validation run, has no
`DerefMut`, and validates as part of `Deserialize`. A service layer that accepts
`Validated<Patient>` cannot be handed an unchecked resource. `Patient` itself
stays constructible and mutable, because building one up field by field before it
is complete is a normal thing to do.

## 10. Contained resources stay as raw JSON

`contained` may hold any resource type, including ones this crate does not model.
A typed enum with an "unknown" arm would drop clinically significant data on
round trip, so `ContainedResource` wraps `serde_json::Value` and still enforces
the structural rules that apply regardless of type (`dom-2`, `dom-4`, `dom-5`).
It gains a typed `parse::<R>()` for the resources that *are* modelled.

---

## Deliberately not here yet

| Gap | Why it was staged | What it will take |
| --- | --- | --- |
| **Primitive extensions** (`"_birthDate"`) | Needs every primitive field wrapped in a `Primitive<P> { value, id, extension }` and paired-key (de)serialization generated per struct — a derive macro's worth of work. Rare in practice outside `data-absent-reason`. | A `fhir-derive` proc macro emitting the `field`/`_field` pair, plus `Primitive<P>` in `primitive/`. Until then, `_`-prefixed keys are ignored on parse and lost on re-serialization. |
| **Exact decimal round-trip on input** | See §3. | `serde_json/arbitrary_precision`, or a `RawValue` deserializer for `Decimal`. |
| **Terminology validation** | Requires a terminology server or a bundled snapshot; code *systems* are checked, membership is not. | A `TerminologyService` trait with an offline `CodeSystem` cache, called from an extended validation pass. |
| **Profile / `StructureDefinition` conformance** | Profiles constrain cardinality, bindings, and slicing at runtime; this layer is the base spec only. | A profile engine, or generated Rust types per profile (the US Core / national-programme route). |
| **FHIRPath** | Only needed for invariants beyond the hand-written ones and for search parameters. | An interpreter, or a subset compiler for the invariant expressions actually used. |
| **XML** | JSON is the working format everywhere in this platform. | `quick-xml` plus attribute/element mapping; the newtypes and validation carry over unchanged. |
| **More resources** | `Patient` first because it exercises every foundational mechanism. | `Organization`, `Practitioner`, `Encounter`, `Coverage` — each replacing its `marker::` stand-in. |
