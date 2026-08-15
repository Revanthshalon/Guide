//! Foundational FHIR R5 types for a healthcare operations platform.
//!
//! This crate is the schema layer: FHIR's primitive types, base elements,
//! general-purpose datatypes, and — so far — the `Patient` resource. Everything
//! above it (persistence, APIs, workflow) is expected to speak in these types
//! rather than in `serde_json::Value`.
//!
//! # The three ideas the crate is built on
//!
//! **1. Parse, don't validate.** Every primitive validates in its constructor
//! and there is no other way in, including from `serde`. Holding an
//! [`Id`](primitive::Id) means you hold something that is legal in a FHIR URL;
//! holding a [`Date`](primitive::Date) means the calendar has already been
//! checked. Nothing downstream re-checks, because nothing downstream can be
//! handed a bad one.
//!
//! **2. Constraints the spec states in prose become type parameters.** FHIR
//! says `Patient.managingOrganization` is a `Reference(Organization)`. Here it
//! *is* [`Reference<Organization>`](datatype::Reference), and the multi-target
//! case is a tuple —
//! `Reference<(Organization, Practitioner, PractitionerRole)>`. The compiler
//! picks the check; the runtime only confirms what the incoming string says.
//!
//! **3. Whatever a type cannot enforce, a validation pass reports — all of
//! it.** Co-occurrence invariants (`pat-1`, `cpt-2`, `att-1`, `ref-1`) and
//! cardinality live in [`Validate`](validate::Validate), which walks the tree
//! collecting every issue with its FHIRPath location, ready to be rendered as
//! an `OperationOutcome`. It never stops at the first problem, because an API
//! client fixing one field at a time is an API client making ten round trips.
//!
//! # Example
//!
//! ```
//! use fhir_core::prelude::*;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let patient = Patient::builder()
//!     .id("example".parse()?)
//!     .identifier(
//!         Identifier::new(
//!             "http://hospital.example.org/mrn".parse()?,
//!             "MRN-0001".parse()?,
//!         )
//!         .with_use(IdentifierUse::Official),
//!     )
//!     .name(HumanName::new("Doe".parse()?, vec!["Jane".parse()?]).with_use(NameUse::Official))
//!     .gender(AdministrativeGender::Female)
//!     .birth_date("1974-12".parse()?)
//!     .telecom(ContactPoint::new(
//!         ContactPointSystem::Phone,
//!         "+44 20 7946 0100".parse()?,
//!     ))
//!     .build_validated()?;
//!
//! assert_eq!(patient.display_name().as_deref(), Some("Jane Doe"));
//!
//! let json = serde_json::to_value(&patient)?;
//! assert_eq!(json["resourceType"], "Patient");
//! assert_eq!(json["birthDate"], "1974-12");
//! # Ok(())
//! # }
//! ```
//!
//! # What is deliberately not here yet
//!
//! Primitive extensions (`"_birthDate"`), terminology validation against a
//! server, FHIRPath, and profile/`StructureDefinition` conformance. See
//! `DESIGN.md` in the crate root for why each was staged and what it will take.

pub mod codes;
pub mod datatype;
pub mod element;
pub mod error;
pub mod primitive;
pub mod resource;
pub mod resources;
pub mod validate;

/// The imports a caller almost always wants.
pub mod prelude {
    pub use crate::codes::{
        AddressType, AddressUse, AdministrativeGender, CodedEnum, ContactPointSystem,
        ContactPointUse, IdentifierUse, LinkType, NameUse, NarrativeStatus, QuantityComparator,
    };
    pub use crate::datatype::{
        Address, Annotation, Attachment, CodeableConcept, Coding, ContactPoint, HumanName,
        Identifier, Meta, Narrative, Period, Quantity, Range, Ratio, Reference,
    };
    pub use crate::element::{BackboneElement, Element, Extension, ExtensionValue};
    pub use crate::error::{Issue, IssueCode, Severity, ValidationError};
    pub use crate::primitive::{
        Base64Binary, Boolean, Canonical, Code, Date, DateTime, Decimal, FhirString, Id, Instant,
        Integer, Integer64, Markdown, Oid, PositiveInt, Time, UnsignedInt, Uri, Url, Uuid, Xhtml,
    };
    pub use crate::resource::{
        Any, ContainedResource, DomainResource, ReferenceTarget, Resource, ResourceMarker,
        ResourceType, marker,
    };
    pub use crate::resources::{
        Patient, PatientBuilder, PatientCommunication, PatientContact, PatientDeceased,
        PatientLink, PatientMultipleBirth,
    };
    pub use crate::validate::{Validate, Validated, ValidationReport, Validator};
}
