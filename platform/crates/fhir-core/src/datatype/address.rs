//! `Address`: a postal or physical address.

use serde::{Deserialize, Serialize};

use crate::codes::{AddressType, AddressUse};
use crate::element::{Extension, impl_element};
use crate::primitive::FhirString;
use crate::validate::{Validate, Validator};

use super::Period;

/// FHIR `Address`.
///
/// `country` is documented as ISO 3166 2- or 3-letter codes *or* a plain name;
/// the type cannot enforce a choice, so pick one convention per deployment and
/// enforce it in a profile. Mixed conventions inside one database make address
/// matching unreliable in exactly the way that produces duplicate patients.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// What this address is used for.
    #[serde(rename = "use", default, skip_serializing_if = "Option::is_none")]
    pub use_: Option<AddressUse>,

    /// Whether the address is postal, physical, or both.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<AddressType>,

    /// The whole address as it should be displayed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<FhirString>,

    /// Street name, number, direction, PO box, and so on — in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub line: Vec<FhirString>,

    /// City, town, or village.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<FhirString>,

    /// District or county.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub district: Option<FhirString>,

    /// Sub-unit of a country: state, province, territory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<FhirString>,

    /// Postal code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<FhirString>,

    /// Country — ISO 3166 code or name, consistently one or the other.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<FhirString>,

    /// Time period when this address was or is in use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<Period>,
}

impl Address {
    /// Set the address's use.
    pub fn with_use(mut self, use_: AddressUse) -> Self {
        self.use_ = Some(use_);
        self
    }

    /// Whether this address is currently in use, per [`Address::period`].
    pub fn is_current(&self) -> bool {
        if self.use_ == Some(AddressUse::Old) {
            return false;
        }
        self.period.as_ref().is_none_or(Period::is_ongoing)
    }

    /// Whether this address carries nothing at all.
    pub fn is_empty(&self) -> bool {
        self.use_.is_none()
            && self.type_.is_none()
            && self.text.is_none()
            && self.line.is_empty()
            && self.city.is_none()
            && self.district.is_none()
            && self.state.is_none()
            && self.postal_code.is_none()
            && self.country.is_none()
            && self.period.is_none()
            && self.extension.is_empty()
    }
}

impl_element!(Address);

impl Validate for Address {
    fn validate(&self, validator: &mut Validator) {
        validator.invariant(
            "ele-1",
            !self.is_empty(),
            "All FHIR elements must have a value or children",
        );
        validator.field("period", &self.period);
        validator.field("extension", &self.extension);
    }
}
