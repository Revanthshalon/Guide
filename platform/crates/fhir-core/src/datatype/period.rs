//! `Period`: a time range with a start and an end.

use serde::{Deserialize, Serialize};

use crate::element::{Extension, impl_element};
use crate::primitive::{DateTime, FhirString};
use crate::validate::{Validate, Validator};

/// FHIR `Period`: a range of time defined by a start and an end.
///
/// Both ends are optional and both are *inclusive*. An absent `end` means
/// "ongoing", not "unknown" — that distinction decides whether an address or an
/// identifier is still current, so treating a missing end as an expired period
/// silently drops a patient's active contact details.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Start of the period, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<DateTime>,

    /// End of the period, inclusive. Absent means the period has not ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<DateTime>,
}

impl Period {
    /// A period with both ends.
    pub fn new(start: DateTime, end: DateTime) -> Self {
        Self {
            start: Some(start),
            end: Some(end),
            ..Self::default()
        }
    }

    /// A period that has started and has not ended.
    pub fn starting(start: DateTime) -> Self {
        Self {
            start: Some(start),
            ..Self::default()
        }
    }

    /// Whether the period has no end, i.e. is still current.
    pub fn is_ongoing(&self) -> bool {
        self.end.is_none()
    }

    /// Whether this period carries nothing at all.
    pub fn is_empty(&self) -> bool {
        self.start.is_none() && self.end.is_none() && self.extension.is_empty()
    }
}

impl_element!(Period);

impl Validate for Period {
    fn validate(&self, validator: &mut Validator) {
        validator.invariant(
            "ele-1",
            !self.is_empty(),
            "All FHIR elements must have a value or children",
        );

        // per-1: If present, start SHALL have a lower or equal value than end.
        // `chronological_cmp` returns None when the two values' precisions make
        // the comparison indeterminate (`start: "2020"`, `end: "2020-06-01"`);
        // an indeterminate comparison is not a violation, so only a definite
        // Greater is reported.
        if let (Some(start), Some(end)) = (&self.start, &self.end) {
            let definitely_after =
                start.chronological_cmp(end) == Some(std::cmp::Ordering::Greater);
            validator.invariant(
                "per-1",
                !definitely_after,
                "If present, start SHALL have a lower or equal value than end",
            );
        }

        validator.field("extension", &self.extension);
    }
}
