//! `Identifier`: a business identifier such as an MRN, NHS number, or NPI.

use serde::{Deserialize, Serialize};

use crate::codes::IdentifierUse;
use crate::element::{Extension, impl_element};
use crate::error::IssueCode;
use crate::primitive::{FhirString, Uri};
use crate::resource::marker::Organization;
use crate::validate::{Validate, Validator};

use super::{CodeableConcept, Period, Reference};

/// FHIR `Identifier`: an identifier assigned by a business process.
///
/// # `system` is not optional in practice
///
/// The `value` alone is meaningless: MRN `12345` at one hospital and MRN
/// `12345` at another are different patients. The `system` namespaces it. This
/// is the single most important field for patient matching across a multi-site
/// organization, and a bare `value` with no `system` is how duplicate patient
/// records get created at scale — so validation flags it.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identifier {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// The purpose of this identifier.
    #[serde(rename = "use", default, skip_serializing_if = "Option::is_none")]
    pub use_: Option<IdentifierUse>,

    /// Description of the identifier — MRN, driver's licence, and so on.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<CodeableConcept>,

    /// The namespace the value is unique within.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<Uri>,

    /// The value that is unique within the system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<FhirString>,

    /// Time period during which the identifier was or is in use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<Period>,

    /// Organization that issued the identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigner: Option<Box<Reference<Organization>>>,
}

impl Identifier {
    /// An identifier from a namespace and a value — the form that is actually
    /// usable for matching.
    pub fn new(system: Uri, value: FhirString) -> Self {
        Self {
            system: Some(system),
            value: Some(value),
            ..Self::default()
        }
    }

    /// Set the identifier's use.
    pub fn with_use(mut self, use_: IdentifierUse) -> Self {
        self.use_ = Some(use_);
        self
    }

    /// Set the identifier's type.
    pub fn with_type(mut self, type_: CodeableConcept) -> Self {
        self.type_ = Some(type_);
        self
    }

    /// Whether this identifier may be used for matching a patient.
    ///
    /// Retired identifiers ([`IdentifierUse::Old`]) must not be: the value may
    /// since have been reassigned to a different person.
    pub fn is_usable_for_matching(&self) -> bool {
        self.use_ != Some(IdentifierUse::Old) && self.system.is_some() && self.value.is_some()
    }

    /// Whether this identifier carries nothing at all.
    pub fn is_empty(&self) -> bool {
        self.use_.is_none()
            && self.type_.is_none()
            && self.system.is_none()
            && self.value.is_none()
            && self.period.is_none()
            && self.assigner.is_none()
            && self.extension.is_empty()
    }
}

impl_element!(Identifier);

impl Validate for Identifier {
    fn validate(&self, validator: &mut Validator) {
        validator.invariant(
            "ele-1",
            !self.is_empty(),
            "All FHIR elements must have a value or children",
        );

        if self.value.is_some() && self.system.is_none() {
            validator.error_at(
                "system",
                IssueCode::Value,
                "an Identifier with a value must declare the system that namespaces it, \
                 otherwise the value cannot be matched across organizations",
            );
        }

        if self.system.is_some() && self.value.is_none() {
            validator.warn(
                IssueCode::Value,
                "an Identifier with a system but no value carries no information",
            );
        }

        validator.field("type", &self.type_);
        validator.field("period", &self.period);
        validator.field("assigner", &self.assigner);
        validator.field("extension", &self.extension);
    }
}
