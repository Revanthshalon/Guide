//! `HumanName`: a name a human is known by.

use serde::{Deserialize, Serialize};

use crate::codes::NameUse;
use crate::element::{Extension, impl_element};
use crate::error::IssueCode;
use crate::primitive::FhirString;
use crate::validate::{Validate, Validator};

use super::Period;

/// FHIR `HumanName`.
///
/// # Do not model this as first name + last name
///
/// `given` is a *list*, in order, covering what Western systems split into
/// first and middle names. `family` is a single string that may itself contain
/// spaces or particles (`van der Berg`). Many cultures use neither in the way
/// the field names suggest, and some record only [`HumanName::text`].
///
/// The practical rule: `text` is what a human should read, and the parts are
/// for matching and correspondence. If a source system gives you only a full
/// name string, put it in `text` rather than guessing at a split — a wrong
/// split is much harder to detect downstream than an absent one.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanName {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// What this name is used for.
    #[serde(rename = "use", default, skip_serializing_if = "Option::is_none")]
    pub use_: Option<NameUse>,

    /// The whole name as it should be displayed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<FhirString>,

    /// Family name — surname in many cultures, but not all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<FhirString>,

    /// Given names, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub given: Vec<FhirString>,

    /// Titles preceding the name — Dr, Mrs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefix: Vec<FhirString>,

    /// Qualifications following the name — MD, Jr.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suffix: Vec<FhirString>,

    /// Time period when this name was or is in use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<Period>,
}

impl HumanName {
    /// A name recorded only as display text — the honest representation when
    /// the source has not parsed it.
    pub fn from_text(text: FhirString) -> Self {
        Self {
            text: Some(text),
            ..Self::default()
        }
    }

    /// A name with a family name and given names.
    pub fn new(family: FhirString, given: Vec<FhirString>) -> Self {
        Self {
            family: Some(family),
            given,
            ..Self::default()
        }
    }

    /// Set the name's use.
    pub fn with_use(mut self, use_: NameUse) -> Self {
        self.use_ = Some(use_);
        self
    }

    /// Text suitable for display: `text` when present, otherwise the parts
    /// joined in the order they were supplied.
    pub fn display(&self) -> String {
        if let Some(text) = &self.text {
            return text.as_str().to_owned();
        }
        let mut parts: Vec<&str> = Vec::new();
        parts.extend(self.prefix.iter().map(FhirString::as_str));
        parts.extend(self.given.iter().map(FhirString::as_str));
        if let Some(family) = &self.family {
            parts.push(family.as_str());
        }
        parts.extend(self.suffix.iter().map(FhirString::as_str));
        parts.join(" ")
    }

    /// Whether this name is currently in use, per [`HumanName::period`].
    ///
    /// A name marked [`NameUse::Old`] is never current, regardless of period.
    pub fn is_current(&self) -> bool {
        if self.use_ == Some(NameUse::Old) {
            return false;
        }
        self.period.as_ref().is_none_or(Period::is_ongoing)
    }

    /// Whether this name carries nothing at all.
    pub fn is_empty(&self) -> bool {
        self.use_.is_none()
            && self.text.is_none()
            && self.family.is_none()
            && self.given.is_empty()
            && self.prefix.is_empty()
            && self.suffix.is_empty()
            && self.period.is_none()
            && self.extension.is_empty()
    }
}

impl_element!(HumanName);

impl Validate for HumanName {
    fn validate(&self, validator: &mut Validator) {
        validator.invariant(
            "ele-1",
            !self.is_empty(),
            "All FHIR elements must have a value or children",
        );

        // Not a specification invariant, but a name with only a `use` and a
        // period identifies nobody.
        let has_name_content =
            self.text.is_some() || self.family.is_some() || !self.given.is_empty();
        if !has_name_content && !self.is_empty() {
            validator.warn(
                IssueCode::Value,
                "HumanName carries no text, family, or given name",
            );
        }

        validator.field("period", &self.period);
        validator.field("extension", &self.extension);
    }
}
