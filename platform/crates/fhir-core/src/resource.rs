//! Resource-level base types: the [`Resource`] and [`DomainResource`] traits,
//! the [`ResourceType`] vocabulary, the zero-sized [`marker`] types that make
//! [`Reference<T>`](crate::datatype::Reference) target-checked, and
//! [`ContainedResource`].

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::element::Extension;
use crate::error::{IssueCode, ParseError, ParseErrorKind, ValidationError};
use crate::primitive::{Code, Id, Uri};
use crate::validate::{Validate, Validated, ValidationReport, Validator};

macro_rules! resource_types {
    ( $( $(#[$attr:meta])* $variant:ident => $name:literal ),+ $(,)? ) => {
        /// The FHIR resource types this build knows about.
        ///
        /// Deliberately not exhaustive over all of R5 — it lists what the
        /// platform models plus the types that appear as reference targets, so
        /// that a typed reference can be checked. Unknown types encountered in
        /// data are reported as warnings rather than errors, because a valid
        /// FHIR server may legitimately reference something this build has
        /// never heard of.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[non_exhaustive]
        pub enum ResourceType {
            $( $(#[$attr])* $variant ),+
        }

        impl ResourceType {
            /// Every known resource type.
            pub const ALL: &'static [Self] = &[ $( Self::$variant ),+ ];

            /// The type's name as it appears in `resourceType` and in URLs.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( Self::$variant => $name ),+
                }
            }

            /// Parse a resource type name, returning `None` if unknown.
            pub fn parse(name: &str) -> Option<Self> {
                match name {
                    $( $name => Some(Self::$variant), )+
                    _ => None,
                }
            }
        }
    };
}

resource_types! {
    /// An individual receiving care.
    Patient => "Patient",
    /// A person with a formal responsibility in the provision of care.
    Practitioner => "Practitioner",
    /// A specific role a practitioner fills at an organization.
    PractitionerRole => "PractitionerRole",
    /// A grouping of people or organizations with a common purpose.
    Organization => "Organization",
    /// A person related to a patient, but not a direct target of care.
    RelatedPerson => "RelatedPerson",
    /// A generic person record, shared across roles.
    Person => "Person",
    /// A defined collection of entities.
    Group => "Group",
    /// A manufactured item used in healthcare.
    Device => "Device",
    /// A physical place where care is delivered.
    Location => "Location",
    /// An interaction between a patient and healthcare providers.
    Encounter => "Encounter",
    /// A technical endpoint for electronic services.
    Endpoint => "Endpoint",
    /// A service offered by an organization.
    HealthcareService => "HealthcareService",
    /// An insurance plan or payment agreement.
    Coverage => "Coverage",
    /// A measurement or simple assertion about a subject.
    Observation => "Observation",
    /// A clinical condition, problem, or diagnosis.
    Condition => "Condition",
    /// The record of a supplier's or provider's charge.
    Account => "Account",
    /// A container for a collection of resources.
    Bundle => "Bundle",
    /// The outcome of an operation, including validation results.
    OperationOutcome => "OperationOutcome",
    /// A record of an activity that produced or changed a resource.
    Provenance => "Provenance",
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ResourceType {
    type Err = ParseError;

    fn from_str(name: &str) -> Result<Self, ParseError> {
        Self::parse(name).ok_or_else(|| {
            ParseError::new(
                "ResourceType",
                ParseErrorKind::UnknownCode {
                    value: name.to_owned(),
                    value_set: "http://hl7.org/fhir/ValueSet/resource-types",
                },
            )
        })
    }
}

/// A type that stands for exactly one FHIR resource type.
///
/// Implemented both by real resource structs (such as
/// [`Patient`](crate::resources::Patient)) and by the zero-sized [`marker`]
/// types that stand in for resources this crate has not modelled yet. That is
/// what lets `Reference<marker::Organization>` be written today and become
/// `Reference<Organization>` later without changing a single call site.
pub trait ResourceMarker: 'static {
    /// The resource type this marker stands for.
    const RESOURCE_TYPE: ResourceType;
}

/// What a [`Reference<T>`](crate::datatype::Reference) is allowed to point at.
///
/// Implemented for every [`ResourceMarker`], for tuples of them (FHIR elements
/// routinely permit several target types — `Patient.generalPractitioner` allows
/// `Organization | Practitioner | PractitionerRole`), and for [`Any`] when the
/// element permits any resource at all.
///
/// ```
/// use fhir_core::datatype::Reference;
/// use fhir_core::resource::marker::{Organization, Practitioner, PractitionerRole};
///
/// // The type spells out the constraint the specification states in prose.
/// type GeneralPractitioner =
///     Reference<(Organization, Practitioner, PractitionerRole)>;
///
/// let gp: GeneralPractitioner = Reference::literal("Practitioner/23").unwrap();
/// assert!(gp.check().is_ok());
///
/// let wrong: GeneralPractitioner = Reference::literal("Device/23").unwrap();
/// assert!(wrong.check().is_err());
/// ```
pub trait ReferenceTarget: 'static {
    /// Whether `resource_type` is an acceptable target.
    fn accepts(resource_type: ResourceType) -> bool;

    /// Collect the acceptable types, for error messages.
    fn expected(into: &mut Vec<ResourceType>);

    /// A human-readable list of the acceptable types.
    fn describe() -> String {
        let mut types = Vec::new();
        Self::expected(&mut types);
        if types.is_empty() {
            return "any resource".to_owned();
        }
        types
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

/// A reference target with no type constraint, used where FHIR says `Any`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Any;

impl ReferenceTarget for Any {
    fn accepts(_resource_type: ResourceType) -> bool {
        true
    }

    fn expected(_into: &mut Vec<ResourceType>) {}
}

macro_rules! tuple_reference_targets {
    ( $( ( $( $param:ident ),+ ) ),+ $(,)? ) => {
        $(
            impl< $( $param: ReferenceTarget ),+ > ReferenceTarget for ( $( $param, )+ ) {
                fn accepts(resource_type: ResourceType) -> bool {
                    false $( || $param::accepts(resource_type) )+
                }

                fn expected(into: &mut Vec<ResourceType>) {
                    $( $param::expected(into); )+
                }
            }
        )+
    };
}

tuple_reference_targets! {
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F),
}

/// Zero-sized stand-ins for resources this crate does not model yet.
///
/// They exist so that references can be *typed* from day one. When
/// `Organization` becomes a real struct, it implements [`ResourceMarker`] the
/// same way and the marker is deleted — `Reference<Organization>` keeps meaning
/// what it means today.
pub mod marker {
    use super::{ReferenceTarget, ResourceMarker, ResourceType};

    macro_rules! resource_markers {
        ( $( $(#[$attr:meta])* $name:ident ),+ $(,)? ) => {
            $(
                $(#[$attr])*
                #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
                pub struct $name;

                impl ResourceMarker for $name {
                    const RESOURCE_TYPE: ResourceType = ResourceType::$name;
                }

                impl ReferenceTarget for $name {
                    fn accepts(resource_type: ResourceType) -> bool {
                        resource_type == ResourceType::$name
                    }

                    fn expected(into: &mut Vec<ResourceType>) {
                        into.push(ResourceType::$name);
                    }
                }
            )+
        };
    }

    resource_markers! {
        /// Stands for the `Practitioner` resource.
        Practitioner,
        /// Stands for the `PractitionerRole` resource.
        PractitionerRole,
        /// Stands for the `Organization` resource.
        Organization,
        /// Stands for the `RelatedPerson` resource.
        RelatedPerson,
        /// Stands for the `Person` resource.
        Person,
        /// Stands for the `Group` resource.
        Group,
        /// Stands for the `Device` resource.
        Device,
        /// Stands for the `Location` resource.
        Location,
        /// Stands for the `Encounter` resource.
        Encounter,
        /// Stands for the `Endpoint` resource.
        Endpoint,
        /// Stands for the `HealthcareService` resource.
        HealthcareService,
        /// Stands for the `Coverage` resource.
        Coverage,
        /// Stands for the `Observation` resource.
        Observation,
        /// Stands for the `Condition` resource.
        Condition,
        /// Stands for the `Account` resource.
        Account,
    }
}

/// The `resourceType` discriminator field.
///
/// A zero-sized field whose only job is to make the JSON right: it serializes
/// as the resource type's name and refuses to deserialize anything else, so a
/// `Patient` cannot be built from an `Observation` payload just because the
/// fields happened to line up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceTag<R: ResourceMarker>(PhantomData<fn() -> R>);

impl<R: ResourceMarker> Default for ResourceTag<R> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<R: ResourceMarker> Serialize for ResourceTag<R> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(R::RESOURCE_TYPE.as_str())
    }
}

impl<'de, R: ResourceMarker> Deserialize<'de> for ResourceTag<R> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        if raw == R::RESOURCE_TYPE.as_str() {
            Ok(Self(PhantomData))
        } else {
            Err(serde::de::Error::custom(format!(
                "expected resourceType {:?}, found {:?}",
                R::RESOURCE_TYPE.as_str(),
                raw
            )))
        }
    }
}

/// Behaviour shared by every FHIR resource.
///
/// Extends [`ResourceMarker`], so `Self::RESOURCE_TYPE` is available here and a
/// real resource can be used directly as a reference target — `Reference<Patient>`
/// names the struct, not a stand-in.
pub trait Resource: Validate + Sized + ResourceMarker {
    /// The logical id, absent until the resource has been assigned one by a
    /// server.
    fn id(&self) -> Option<&Id>;

    /// Metadata maintained by the infrastructure: version, last update, tags.
    fn meta(&self) -> Option<&crate::datatype::Meta>;

    /// A set of rules that must be understood to process this resource.
    fn implicit_rules(&self) -> Option<&Uri>;

    /// The base language of the resource's content.
    fn language(&self) -> Option<&Code>;

    /// Validate, returning errors *and* warnings.
    fn validation_report(&self) -> ValidationReport {
        let mut validator = Validator::new(Self::RESOURCE_TYPE.as_str());
        self.validate(&mut validator);
        validator.into_report()
    }

    /// Validate, discarding warnings.
    fn check(&self) -> Result<(), ValidationError> {
        self.validation_report().into_result()
    }

    /// Validate and, on success, wrap in the proof type.
    fn validated(self) -> Result<Validated<Self>, ValidationError> {
        Validated::new(self, Self::RESOURCE_TYPE.as_str())
    }
}

/// A resource that can carry a human narrative, contained resources, and
/// extensions — which is every resource except the infrastructure ones
/// (`Bundle`, `Binary`, `Parameters`).
pub trait DomainResource: Resource {
    /// The human-readable narrative summarising the resource.
    fn text(&self) -> Option<&crate::datatype::Narrative>;

    /// Resources with no independent existence, inlined into this one.
    fn contained(&self) -> &[ContainedResource];

    /// Extensions on the resource itself.
    fn extension(&self) -> &[Extension];

    /// Extensions that change the meaning of the resource. A system that does
    /// not understand one **must not** process the resource.
    fn modifier_extension(&self) -> &[Extension];
}

/// A resource inlined into another resource's `contained` list.
///
/// Held as raw JSON rather than a typed enum, because `contained` may hold any
/// resource type — including ones this crate does not model — and silently
/// dropping one on deserialization would lose clinically significant data. The
/// structural rules that FHIR states about contained resources (`dom-2`,
/// `dom-4`, `dom-5`) are checked here regardless of the type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContainedResource(serde_json::Value);

impl ContainedResource {
    /// Wrap a JSON object as a contained resource.
    pub fn new(value: serde_json::Value) -> Self {
        Self(value)
    }

    /// The raw JSON.
    pub fn as_json(&self) -> &serde_json::Value {
        &self.0
    }

    /// The value of `resourceType`, if present and a string.
    pub fn resource_type(&self) -> Option<&str> {
        self.0.get("resourceType")?.as_str()
    }

    /// The value of `id`, if present and a string.
    pub fn id(&self) -> Option<&str> {
        self.0.get("id")?.as_str()
    }

    /// Parse into a modelled resource type.
    pub fn parse<R: Resource + serde::de::DeserializeOwned>(&self) -> Result<R, serde_json::Error> {
        serde_json::from_value(self.0.clone())
    }
}

impl Validate for ContainedResource {
    fn validate(&self, validator: &mut Validator) {
        let Some(object) = self.0.as_object() else {
            validator.error(
                IssueCode::Structure,
                "a contained resource must be a JSON object",
            );
            return;
        };

        if self.resource_type().is_none() {
            validator.error_at(
                "resourceType",
                IssueCode::Required,
                "a contained resource must declare its resourceType",
            );
        }

        // dom-2: contained resources SHALL NOT themselves contain resources.
        let nested = object
            .get("contained")
            .and_then(|c| c.as_array())
            .is_some_and(|c| !c.is_empty());
        validator.invariant(
            "dom-2",
            !nested,
            "If the resource is contained in another resource, it SHALL NOT contain nested Resources",
        );

        // dom-4: a contained resource has no independent identity, so version
        // and last-update metadata are meaningless on it.
        let versioned = object
            .get("meta")
            .and_then(|m| m.as_object())
            .is_some_and(|meta| meta.contains_key("versionId") || meta.contains_key("lastUpdated"));
        validator.invariant(
            "dom-4",
            !versioned,
            "If a resource is contained in another resource, it SHALL NOT have a meta.versionId or a meta.lastUpdated",
        );

        // dom-5: a contained resource cannot carry its own security labels;
        // the labels of the container apply.
        let labelled = object
            .get("meta")
            .and_then(|m| m.get("security"))
            .and_then(|s| s.as_array())
            .is_some_and(|s| !s.is_empty());
        validator.invariant(
            "dom-5",
            !labelled,
            "If a resource is contained in another resource, it SHALL NOT have a security label",
        );
    }
}
