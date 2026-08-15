//! `Annotation`: a note, with an author and a time.

use serde::{Deserialize, Serialize};

use crate::element::{Extension, impl_element};
use crate::primitive::{DateTime, FhirString, Markdown};
use crate::resource::{ReferenceTarget, ResourceType};
use crate::validate::{Validate, Validator};

use super::Reference;

/// The `author[x]` of an [`Annotation`]: either a reference or a plain string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnnotationAuthor {
    /// `authorReference` — a Practitioner, Patient, RelatedPerson, or Organization.
    #[serde(rename = "authorReference")]
    Reference(Box<Reference<AnnotationAuthorTarget>>),
    /// `authorString` — a name, when the author is not a resource in the system.
    #[serde(rename = "authorString")]
    String(FhirString),
}

/// Reference targets permitted for `Annotation.author`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnotationAuthorTarget;

impl ReferenceTarget for AnnotationAuthorTarget {
    fn accepts(resource_type: ResourceType) -> bool {
        matches!(
            resource_type,
            ResourceType::Practitioner
                | ResourceType::PractitionerRole
                | ResourceType::Patient
                | ResourceType::RelatedPerson
                | ResourceType::Organization
        )
    }

    fn expected(into: &mut Vec<ResourceType>) {
        into.extend([
            ResourceType::Practitioner,
            ResourceType::PractitionerRole,
            ResourceType::Patient,
            ResourceType::RelatedPerson,
            ResourceType::Organization,
        ]);
    }
}

/// FHIR `Annotation`: text with an author and a timestamp.
///
/// The text is markdown and may be authored by a human, so it is untrusted
/// input for any renderer — see [`Xhtml`](crate::primitive::Xhtml) for the same
/// concern stated in full.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Who authored the annotation.
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub author: Option<AnnotationAuthor>,

    /// When the annotation was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<DateTime>,

    /// The annotation text, as markdown.
    pub text: Markdown,
}

impl Annotation {
    /// An annotation with only text.
    pub fn new(text: Markdown) -> Self {
        Self {
            id: None,
            extension: Vec::new(),
            author: None,
            time: None,
            text,
        }
    }
}

impl_element!(Annotation);

impl Validate for Annotation {
    fn validate(&self, validator: &mut Validator) {
        if let Some(AnnotationAuthor::Reference(reference)) = &self.author {
            validator.field("author", reference.as_ref());
        }
        validator.field("extension", &self.extension);
    }
}
