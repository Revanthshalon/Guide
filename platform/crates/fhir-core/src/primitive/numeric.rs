//! The numeric FHIR primitives.
//!
//! Two of these carry surprises that cost real interoperability bugs:
//!
//! * `integer64` is serialized in JSON **as a string**, because JavaScript
//!   cannot represent the full 64-bit range. [`Integer64`] does that for you.
//! * `decimal` must **preserve precision**: `1.50` and `1.5` are the same
//!   number but not the same FHIR value, because trailing zeros carry the
//!   measurement's precision. A lab result of `1.50 mmol/L` says something
//!   different from `1.5 mmol/L`. [`Decimal`] therefore stores the lexical form
//!   and re-emits it byte-for-byte.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{ParseError, ParseErrorKind};
use crate::primitive::PrimitiveType;
use crate::validate::{Validate, Validator};

/// The largest value FHIR's 32-bit integer types accept.
const MAX_INT32: i64 = i32::MAX as i64;

macro_rules! bounded_int {
    (
        $(#[$attr:meta])*
        $name:ident, fhir = $fhir:literal, min = $min:expr $(,)?
    ) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(i32);

        impl $name {
            /// Smallest accepted value.
            pub const MIN: i32 = $min;

            /// Largest accepted value.
            pub const MAX: i32 = i32::MAX;

            /// Validate `value` and wrap it.
            pub fn new(value: i32) -> Result<Self, ParseError> {
                if value < Self::MIN {
                    return Err(ParseError::new(
                        $fhir,
                        ParseErrorKind::OutOfRange {
                            min: Self::MIN as i64,
                            max: MAX_INT32,
                            actual: value as i64,
                        },
                    ));
                }
                Ok(Self(value))
            }

            /// The wrapped value.
            pub const fn get(self) -> i32 {
                self.0
            }
        }

        impl PrimitiveType for $name {
            const FHIR_TYPE: &'static str = $fhir;
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<i32> for $name {
            type Error = ParseError;

            fn try_from(value: i32) -> Result<Self, ParseError> {
                Self::new(value)
            }
        }

        impl From<$name> for i32 {
            fn from(value: $name) -> i32 {
                value.0
            }
        }

        impl FromStr for $name {
            type Err = ParseError;

            fn from_str(value: &str) -> Result<Self, ParseError> {
                let parsed = value.parse::<i32>().map_err(|_| {
                    ParseError::new(
                        $fhir,
                        ParseErrorKind::Malformed {
                            expected: "a 32-bit decimal integer",
                        },
                    )
                })?;
                Self::new(parsed)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_i32(self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = i32::deserialize(deserializer)?;
                Self::new(raw).map_err(de::Error::custom)
            }
        }

        impl Validate for $name {
            fn validate(&self, _validator: &mut Validator) {}
        }
    };
}

bounded_int! {
    /// FHIR `positiveInt`: an integer of 1 or more.
    ///
    /// Used for ranks and counts where zero is meaningless — `ContactPoint.rank`
    /// is 1-based, and a rank of 0 would silently sort a phone number first.
    PositiveInt, fhir = "positiveInt", min = 1,
}

bounded_int! {
    /// FHIR `unsignedInt`: an integer of 0 or more.
    UnsignedInt, fhir = "unsignedInt", min = 0,
}

/// FHIR `integer64`: a signed 64-bit integer.
///
/// **Serialized as a JSON string**, per the R5 specification, because JSON
/// numbers are read as IEEE-754 doubles by most JavaScript clients and would
/// lose precision above 2^53. The Rust API deals in [`i64`]; only the wire form
/// is a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Integer64(i64);

impl Integer64 {
    /// Wrap a 64-bit integer.
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// The wrapped value.
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl PrimitiveType for Integer64 {
    const FHIR_TYPE: &'static str = "integer64";
}

impl fmt::Display for Integer64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for Integer64 {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<Integer64> for i64 {
    fn from(value: Integer64) -> i64 {
        value.0
    }
}

impl FromStr for Integer64 {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, ParseError> {
        value.parse::<i64>().map(Self).map_err(|_| {
            ParseError::new(
                "integer64",
                ParseErrorKind::Malformed {
                    expected: "a 64-bit decimal integer",
                },
            )
        })
    }
}

impl Serialize for Integer64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Integer64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Integer64Visitor;

        impl Visitor<'_> for Integer64Visitor {
            type Value = Integer64;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a 64-bit integer, as a JSON string or number")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Integer64, E> {
                value.parse::<Integer64>().map_err(de::Error::custom)
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Integer64, E> {
                Ok(Integer64(value))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Integer64, E> {
                i64::try_from(value)
                    .map(Integer64)
                    .map_err(|_| de::Error::custom("value does not fit in a signed 64-bit integer"))
            }
        }

        deserializer.deserialize_any(Integer64Visitor)
    }
}

impl Validate for Integer64 {
    fn validate(&self, _validator: &mut Validator) {}
}

/// FHIR `decimal`, stored in its lexical form so that precision survives a
/// round trip.
///
/// # Why not `f64`
///
/// `0.1 + 0.2 != 0.3` is the usual objection, but the decisive one for health
/// data is precision *reporting*: FHIR states that the number of significant
/// digits in a decimal is meaningful. Parsing `"1.50"` into an `f64` and
/// re-serializing gives `1.5`, silently changing what a lab reported. This type
/// keeps the original digits and exposes [`Decimal::as_f64`] for arithmetic,
/// making the lossy step explicit at the call site.
///
/// # Serialization
///
/// Serializing emits a bare JSON number carrying the stored digits, using
/// `serde_json`'s raw-value support. Non-JSON formats receive the lexical form
/// as a string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decimal {
    lexical: String,
}

impl Decimal {
    /// FHIR limits a decimal to 18 significant digits.
    pub const MAX_SIGNIFICANT_DIGITS: usize = 18;

    /// Validate a lexical decimal and wrap it.
    pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
        let lexical = value.into();
        check_decimal(&lexical).map_err(|kind| ParseError::new("decimal", kind))?;
        Ok(Self { lexical })
    }

    /// The lexical form, exactly as supplied.
    pub fn as_str(&self) -> &str {
        &self.lexical
    }

    /// The value as an `f64`. Lossy for values needing more than 15–17
    /// significant digits; that is why it is a named method and not a `Deref`.
    pub fn as_f64(&self) -> f64 {
        self.lexical.parse().unwrap_or(f64::NAN)
    }

    /// Number of significant digits in the lexical form — the reported
    /// precision of the measurement.
    pub fn significant_digits(&self) -> usize {
        let mantissa = self
            .lexical
            .split(['e', 'E'])
            .next()
            .unwrap_or(&self.lexical);
        let digits: Vec<char> = mantissa.chars().filter(char::is_ascii_digit).collect();
        let has_fraction = mantissa.contains('.');
        // Leading zeros are never significant; trailing zeros are, but only
        // when a decimal point is present ("100" is 1 sig-fig, "100." is 3).
        let leading_zeros = digits.iter().take_while(|c| **c == '0').count();
        let significant = digits.len() - leading_zeros;
        if !has_fraction {
            let trailing_zeros = digits
                .iter()
                .rev()
                .take_while(|c| **c == '0')
                .count()
                .min(significant.saturating_sub(1));
            return significant - trailing_zeros;
        }
        significant
    }
}

impl PrimitiveType for Decimal {
    const FHIR_TYPE: &'static str = "decimal";
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.lexical)
    }
}

impl FromStr for Decimal {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, ParseError> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Decimal {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, ParseError> {
        Self::new(value)
    }
}

impl From<i32> for Decimal {
    fn from(value: i32) -> Self {
        Self {
            lexical: value.to_string(),
        }
    }
}

impl Decimal {
    /// Compare two decimals *numerically*.
    ///
    /// Deliberately not a [`PartialOrd`] implementation: `Decimal` derives
    /// [`PartialEq`] on the lexical form, so `"1.50" != "1.5"` (different
    /// reported precision) while this method calls them
    /// [`Ordering::Equal`] (same quantity). Having `==` and `partial_cmp`
    /// disagree in a trait implementation is a trap; as a named method, the
    /// caller is choosing which of the two questions they are asking.
    pub fn numeric_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_f64().partial_cmp(&other.as_f64())
    }
}

impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            // Emit the stored digits as a bare JSON number. `RawValue` is the
            // only way to do this without going through `f64` and losing the
            // precision this type exists to keep.
            let raw = serde_json::value::RawValue::from_string(self.lexical.clone())
                .map_err(serde::ser::Error::custom)?;
            raw.serialize(serializer)
        } else {
            serializer.serialize_str(&self.lexical)
        }
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DecimalVisitor;

        impl Visitor<'_> for DecimalVisitor {
            type Value = Decimal;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a decimal number")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Decimal, E> {
                Decimal::new(value).map_err(de::Error::custom)
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Decimal, E> {
                // Reached only when the JSON parser has already turned the
                // literal into a double, which is where precision is lost. See
                // the note on `Decimal` about `arbitrary_precision`.
                Decimal::new(format!("{value}")).map_err(de::Error::custom)
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Decimal, E> {
                Decimal::new(value.to_string()).map_err(de::Error::custom)
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Decimal, E> {
                Decimal::new(value.to_string()).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(DecimalVisitor)
    }
}

impl Validate for Decimal {
    fn validate(&self, _validator: &mut Validator) {}
}

fn check_decimal(value: &str) -> Result<(), ParseErrorKind> {
    const EXPECTED: &str =
        "a decimal number matching -?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?";
    if value.is_empty() {
        return Err(ParseErrorKind::Empty);
    }

    let mut chars = value.chars().peekable();
    if chars.peek() == Some(&'-') {
        chars.next();
    }

    // Integer part: a lone `0`, or a non-zero digit followed by digits.
    let mut integer_digits = 0usize;
    let leading_zero = chars.peek() == Some(&'0');
    while chars.peek().is_some_and(char::is_ascii_digit) {
        chars.next();
        integer_digits += 1;
    }
    if integer_digits == 0 || (leading_zero && integer_digits > 1) {
        return Err(ParseErrorKind::Malformed { expected: EXPECTED });
    }

    let mut total_digits = integer_digits;

    if chars.peek() == Some(&'.') {
        chars.next();
        let mut fraction_digits = 0usize;
        while chars.peek().is_some_and(char::is_ascii_digit) {
            chars.next();
            fraction_digits += 1;
        }
        if fraction_digits == 0 {
            return Err(ParseErrorKind::Malformed { expected: EXPECTED });
        }
        total_digits += fraction_digits;
    }

    if matches!(chars.peek(), Some('e' | 'E')) {
        chars.next();
        if matches!(chars.peek(), Some('+' | '-')) {
            chars.next();
        }
        let mut exponent_digits = 0usize;
        while chars.peek().is_some_and(char::is_ascii_digit) {
            chars.next();
            exponent_digits += 1;
        }
        if exponent_digits == 0 {
            return Err(ParseErrorKind::Malformed { expected: EXPECTED });
        }
    }

    if chars.next().is_some() {
        return Err(ParseErrorKind::Malformed { expected: EXPECTED });
    }
    if total_digits > Decimal::MAX_SIGNIFICANT_DIGITS {
        return Err(ParseErrorKind::TooLong {
            max: Decimal::MAX_SIGNIFICANT_DIGITS,
            actual: total_digits,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_int_rejects_zero() {
        assert!(PositiveInt::new(1).is_ok());
        assert!(PositiveInt::new(0).is_err());
        assert!(UnsignedInt::new(0).is_ok());
        assert!(UnsignedInt::new(-1).is_err());
    }

    #[test]
    fn integer64_is_a_json_string() {
        let value = Integer64::new(9_007_199_254_740_993);
        assert_eq!(
            serde_json::to_string(&value).expect("serializes"),
            "\"9007199254740993\""
        );
        let parsed: Integer64 = serde_json::from_str("\"9007199254740993\"").expect("deserializes");
        assert_eq!(parsed, value);
    }

    #[test]
    fn decimal_preserves_trailing_zeros() {
        let value = Decimal::new("1.50").expect("valid decimal");
        assert_eq!(serde_json::to_string(&value).expect("serializes"), "1.50");
        assert_eq!(value.significant_digits(), 3);
        assert_ne!(value, Decimal::new("1.5").expect("valid decimal"));
        assert_eq!(
            value.numeric_cmp(&Decimal::new("1.5").expect("valid decimal")),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn decimal_rejects_malformed_forms() {
        assert!(Decimal::new("01.5").is_err());
        assert!(Decimal::new("1.").is_err());
        assert!(Decimal::new(".5").is_err());
        assert!(Decimal::new("1e").is_err());
        assert!(Decimal::new("+1.5").is_err());
        assert!(Decimal::new("1.5e-3").is_ok());
        assert!(Decimal::new("-0.001").is_ok());
    }
}
