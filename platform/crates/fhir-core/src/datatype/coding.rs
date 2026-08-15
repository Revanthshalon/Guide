//! `Coding`, `CodeableConcept`, and `CodeableReference`.

use serde::{Deserialize, Serialize};

use crate::element::{Extension, impl_element};
use crate::error::IssueCode;
use crate::primitive::{Boolean, Code, FhirString, Uri};
use crate::resource::Any;
use crate::validate::{Validate, Validator};

use super::Reference;

/// FHIR `Coding`: one code, from one code system.
///
/// The `system` is what gives `code` meaning. A `Coding` with a code and no
/// system is not a small omission — `"F"` means "female" in one system and
/// "Fahrenheit" in another. Validation therefore treats a code without a system
/// as an error rather than a warning.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Coding {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Identity of the terminology system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<Uri>,

    /// Version of the system, if relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<FhirString>,

    /// Symbol in syntax defined by the system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Code>,

    /// Representation defined by the system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<FhirString>,

    /// Whether this coding was chosen directly by the user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_selected: Option<Boolean>,
}

impl Coding {
    /// A coding from a system and a code.
    pub fn new(system: Uri, code: Code) -> Self {
        Self {
            system: Some(system),
            code: Some(code),
            ..Self::default()
        }
    }

    /// Add a display string.
    pub fn with_display(mut self, display: FhirString) -> Self {
        self.display = Some(display);
        self
    }

    /// Whether this coding carries nothing at all.
    pub fn is_empty(&self) -> bool {
        self.system.is_none()
            && self.version.is_none()
            && self.code.is_none()
            && self.display.is_none()
            && self.user_selected.is_none()
            && self.extension.is_empty()
    }
}

impl_element!(Coding);

impl Validate for Coding {
    fn validate(&self, validator: &mut Validator) {
        validator.invariant(
            "ele-1",
            !self.is_empty(),
            "All FHIR elements must have a value or children",
        );
        if self.code.is_some() && self.system.is_none() {
            validator.error_at(
                "system",
                IssueCode::Value,
                "a Coding with a code must identify the system the code comes from",
            );
        }
        validator.field("extension", &self.extension);
    }
}

/// FHIR `CodeableConcept`: a concept, expressed as codings and/or free text.
///
/// The repeating `coding` is a set of *translations of the same concept* — SNOMED
/// CT and ICD-10 codings of one diagnosis — not a list of different concepts.
/// Code that iterates and treats each entry as a separate finding is a common
/// and quiet source of duplicated clinical data.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeableConcept {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Codes defined by a terminology system.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coding: Vec<Coding>,

    /// Plain text representation of the concept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<FhirString>,
}

impl CodeableConcept {
    /// A concept expressed as a single coding.
    pub fn from_coding(coding: Coding) -> Self {
        Self {
            coding: vec![coding],
            ..Self::default()
        }
    }

    /// A concept expressed as text only — valid FHIR, and often the honest
    /// representation of what a source system actually recorded.
    pub fn from_text(text: FhirString) -> Self {
        Self {
            text: Some(text),
            ..Self::default()
        }
    }

    /// The first coding from the given system, if any.
    pub fn coding_from(&self, system: &str) -> Option<&Coding> {
        self.coding
            .iter()
            .find(|coding| coding.system.as_ref().is_some_and(|s| s.as_str() == system))
    }

    /// Whether this concept carries nothing at all.
    pub fn is_empty(&self) -> bool {
        self.coding.is_empty() && self.text.is_none() && self.extension.is_empty()
    }
}

impl_element!(CodeableConcept);

impl Validate for CodeableConcept {
    fn validate(&self, validator: &mut Validator) {
        validator.invariant(
            "ele-1",
            !self.is_empty(),
            "All FHIR elements must have a value or children",
        );
        validator.field("coding", &self.coding);
        validator.field("extension", &self.extension);
    }
}

/// FHIR `CodeableReference`: either a concept or a reference to a resource
/// that carries it.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeableReference {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// A reference to a concept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept: Option<CodeableConcept>,

    /// A reference to a resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<Reference<Any>>,
}

impl_element!(CodeableReference);

impl Validate for CodeableReference {
    fn validate(&self, validator: &mut Validator) {
        validator.invariant(
            "ele-1",
            self.concept.is_some() || self.reference.is_some() || !self.extension.is_empty(),
            "All FHIR elements must have a value or children",
        );
        validator.field("concept", &self.concept);
        validator.field("reference", &self.reference);
        validator.field("extension", &self.extension);
    }
}
