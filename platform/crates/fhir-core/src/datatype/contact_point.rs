//! `ContactPoint`: a phone number, email address, or other contact channel.

use serde::{Deserialize, Serialize};

use crate::codes::{ContactPointSystem, ContactPointUse};
use crate::element::{Extension, impl_element};
use crate::primitive::{FhirString, PositiveInt};
use crate::validate::{Validate, Validator};

use super::Period;

/// FHIR `ContactPoint`.
///
/// `rank` is 1-based and lower means *more* preferred — rank 1 is the first
/// number to try. Sorting ascending is correct; treating it as a score and
/// sorting descending calls the patient's least-preferred number first.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactPoint {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// The channel: phone, email, and so on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<ContactPointSystem>,

    /// The actual contact point details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<FhirString>,

    /// The context this contact point is used in.
    #[serde(rename = "use", default, skip_serializing_if = "Option::is_none")]
    pub use_: Option<ContactPointUse>,

    /// Preference order: 1 is the most preferred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<PositiveInt>,

    /// Time period when this contact point was or is in use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<Period>,
}

impl ContactPoint {
    /// A contact point on a channel with a value.
    pub fn new(system: ContactPointSystem, value: FhirString) -> Self {
        Self {
            system: Some(system),
            value: Some(value),
            ..Self::default()
        }
    }

    /// Set the contact point's use.
    pub fn with_use(mut self, use_: ContactPointUse) -> Self {
        self.use_ = Some(use_);
        self
    }

    /// Set the contact point's preference rank.
    pub fn with_rank(mut self, rank: PositiveInt) -> Self {
        self.rank = Some(rank);
        self
    }

    /// Whether this contact point is currently in use.
    pub fn is_current(&self) -> bool {
        if self.use_ == Some(ContactPointUse::Old) {
            return false;
        }
        self.period.as_ref().is_none_or(Period::is_ongoing)
    }

    /// Whether this contact point carries nothing at all.
    pub fn is_empty(&self) -> bool {
        self.system.is_none()
            && self.value.is_none()
            && self.use_.is_none()
            && self.rank.is_none()
            && self.period.is_none()
            && self.extension.is_empty()
    }
}

impl_element!(ContactPoint);

impl Validate for ContactPoint {
    fn validate(&self, validator: &mut Validator) {
        validator.invariant(
            "ele-1",
            !self.is_empty(),
            "All FHIR elements must have a value or children",
        );

        // cpt-2: A system is required if a value is provided.
        validator.invariant(
            "cpt-2",
            !(self.value.is_some() && self.system.is_none()),
            "A system is required if a value is provided",
        );

        validator.field("period", &self.period);
        validator.field("extension", &self.extension);
    }
}
