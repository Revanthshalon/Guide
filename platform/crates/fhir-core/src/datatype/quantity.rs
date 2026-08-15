//! `Quantity`, `Range`, and `Ratio`.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::codes::QuantityComparator;
use crate::element::{Extension, impl_element};
use crate::primitive::{Code, Decimal, FhirString, Uri};
use crate::validate::{Validate, Validator};

/// FHIR `Quantity`: a measured amount.
///
/// # `comparator` changes the meaning of `value`
///
/// A quantity of `0.05` with comparator `<` means *less than* 0.05, not 0.05.
/// Arithmetic and comparison that ignore the comparator invert the clinical
/// meaning of results reported below a detection limit — which is most
/// sensitive assays. [`Quantity::is_exact`] exists so that the check is easy to
/// write and hard to forget.
///
/// # `code` versus `unit`
///
/// `unit` is what a human should see; `code` plus `system` is what a machine
/// should compute with (normally UCUM). Only `code` is safe to convert or
/// compare on — `unit` is free text and "mL" and "ml" are the same unit to a
/// person and different strings to a program.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quantity {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Numerical value, with implicit precision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Decimal>,

    /// How the value relates to the actual measurement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparator: Option<QuantityComparator>,

    /// Unit representation for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<FhirString>,

    /// System that defines the coded unit form, normally UCUM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<Uri>,

    /// Coded form of the unit — the machine-comparable one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Code>,
}

/// The UCUM system URI, the coded unit system FHIR expects for measurements.
pub const UCUM_SYSTEM: &str = "http://unitsofmeasure.org";

impl Quantity {
    /// A quantity in a UCUM-coded unit.
    pub fn ucum(value: Decimal, code: Code) -> Self {
        Self {
            value: Some(value),
            system: Uri::new(UCUM_SYSTEM).ok(),
            code: Some(code),
            ..Self::default()
        }
    }

    /// Whether the value is the measurement itself rather than a bound.
    pub fn is_exact(&self) -> bool {
        self.comparator.is_none()
    }

    /// Whether this quantity carries nothing at all.
    pub fn is_empty(&self) -> bool {
        self.value.is_none()
            && self.comparator.is_none()
            && self.unit.is_none()
            && self.system.is_none()
            && self.code.is_none()
            && self.extension.is_empty()
    }
}

impl_element!(Quantity);

impl Validate for Quantity {
    fn validate(&self, validator: &mut Validator) {
        validator.invariant(
            "ele-1",
            !self.is_empty(),
            "All FHIR elements must have a value or children",
        );

        // qty-3: If a code for the unit is present, the system SHALL also be present.
        validator.invariant(
            "qty-3",
            !(self.code.is_some() && self.system.is_none()),
            "If a code for the unit is present, the system SHALL also be present",
        );

        validator.field("extension", &self.extension);
    }
}

/// FHIR `Range`: a set of ordered quantities defined by a low and a high.
///
/// Both bounds are *simple quantities*: a comparator on either end is
/// meaningless, since the bound already expresses the comparison. That is
/// invariant `rng-2`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Low limit, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low: Option<Quantity>,

    /// High limit, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high: Option<Quantity>,
}

impl_element!(Range);

impl Validate for Range {
    fn validate(&self, validator: &mut Validator) {
        validator.invariant(
            "ele-1",
            self.low.is_some() || self.high.is_some() || !self.extension.is_empty(),
            "All FHIR elements must have a value or children",
        );

        // rng-2: If present, low SHALL have a lower value than high.
        if let (Some(low), Some(high)) = (&self.low, &self.high)
            && let (Some(low_value), Some(high_value)) = (&low.value, &high.value)
        {
            validator.invariant(
                "rng-2",
                low_value.numeric_cmp(high_value) != Some(Ordering::Greater),
                "If present, low SHALL have a lower value than high",
            );
        }

        // The bounds are SimpleQuantity, which forbids a comparator.
        for (field, bound) in [("low", &self.low), ("high", &self.high)] {
            if bound.as_ref().is_some_and(|q| q.comparator.is_some()) {
                validator.enter(field, |v| {
                    v.invariant(
                        "sqty-1",
                        false,
                        "The comparator is not used on a SimpleQuantity",
                    );
                });
            }
        }

        validator.field("low", &self.low);
        validator.field("high", &self.high);
        validator.field("extension", &self.extension);
    }
}

/// FHIR `Ratio`: a relationship between two quantities, such as a dose rate.
///
/// Not a computed value — `1/3` stays as numerator 1, denominator 3, because
/// the pair carries information a division would destroy (a titre of 1:256 is
/// not 0.0039).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ratio {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Numerator value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numerator: Option<Quantity>,

    /// Denominator value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denominator: Option<Quantity>,
}

impl_element!(Ratio);

impl Validate for Ratio {
    fn validate(&self, validator: &mut Validator) {
        // rat-1: numerator and denominator SHALL both be present, or both absent.
        // If both are absent, the ratio SHALL have some extension.
        let numerator = self.numerator.is_some();
        let denominator = self.denominator.is_some();
        validator.invariant(
            "rat-1",
            numerator == denominator && (numerator || !self.extension.is_empty()),
            "numerator and denominator SHALL both be present, or both are absent; \
             if both are absent, there SHALL be some extension present",
        );

        validator.field("numerator", &self.numerator);
        validator.field("denominator", &self.denominator);
        validator.field("extension", &self.extension);
    }
}
