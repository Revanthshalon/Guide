//! Required-binding code enums.
//!
//! # Which bindings become enums
//!
//! FHIR binds coded elements to value sets at four strengths. Only **required**
//! bindings become Rust enums here, because only they promise that the set of
//! valid codes is closed and will not grow in a patch release. `Patient.gender`
//! is required-bound to `administrative-gender`, so [`AdministrativeGender`] is
//! an enum and an unknown code is a parse error.
//!
//! Everything weaker stays a [`CodeableConcept`](crate::datatype::CodeableConcept):
//! `Patient.maritalStatus` is *extensible*, meaning a system may send a code
//! outside the value set when nothing in it fits. Modelling that as an enum
//! would mean rejecting valid data — the classic way a strongly-typed FHIR
//! client becomes unable to ingest real-world records.
//!
//! # What the enum buys
//!
//! Beyond rejecting typos at the boundary, an enum makes the code *system* a
//! property of the type rather than something each call site has to remember:
//! [`CodedEnum::to_coding`] produces a fully-qualified [`Coding`] with the right
//! system URL, so a code can never be written to the database detached from the
//! system that gives it meaning.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::datatype::Coding;
use crate::error::{ParseError, ParseErrorKind};
use crate::primitive::Code;
use crate::validate::{Validate, Validator};

/// A Rust enum standing for a FHIR required-binding value set.
pub trait CodedEnum: Sized + Copy + 'static {
    /// Canonical URL of the code system the codes come from.
    const SYSTEM: &'static str;

    /// Canonical URL of the value set the binding names.
    const VALUE_SET: &'static str;

    /// Every code in the value set, in specification order.
    const ALL: &'static [Self];

    /// This value's code.
    fn as_code(self) -> &'static str;

    /// Parse a code, rejecting anything outside the value set.
    fn from_code(code: &str) -> Result<Self, ParseError>;

    /// The value as a fully-qualified [`Coding`], carrying its system.
    fn to_coding(self) -> Coding {
        Coding {
            id: None,
            extension: Vec::new(),
            system: Some(
                Self::SYSTEM
                    .parse()
                    .expect("code system URLs in this crate are valid URIs"),
            ),
            version: None,
            code: Some(
                Code::new(self.as_code()).expect("codes in this crate are valid FHIR codes"),
            ),
            display: None,
            user_selected: None,
        }
    }
}

macro_rules! code_enum {
    (
        $(#[$attr:meta])*
        $name:ident, system = $system:literal, value_set = $value_set:literal {
            $( $(#[$variant_attr:meta])* $variant:ident => $code:literal ),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $( $(#[$variant_attr])* $variant ),+
        }

        impl CodedEnum for $name {
            const SYSTEM: &'static str = $system;
            const VALUE_SET: &'static str = $value_set;
            const ALL: &'static [Self] = &[ $( Self::$variant ),+ ];

            fn as_code(self) -> &'static str {
                match self {
                    $( Self::$variant => $code ),+
                }
            }

            fn from_code(code: &str) -> Result<Self, ParseError> {
                match code {
                    $( $code => Ok(Self::$variant), )+
                    other => Err(ParseError::new(
                        stringify!($name),
                        ParseErrorKind::UnknownCode {
                            value: other.to_owned(),
                            value_set: $value_set,
                        },
                    )),
                }
            }
        }

        impl $name {
            /// This value's code.
            pub const fn code(self) -> &'static str {
                match self {
                    $( Self::$variant => $code ),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.code())
            }
        }

        impl FromStr for $name {
            type Err = ParseError;

            fn from_str(code: &str) -> Result<Self, ParseError> {
                <Self as CodedEnum>::from_code(code)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ParseError;

            fn try_from(code: &str) -> Result<Self, ParseError> {
                <Self as CodedEnum>::from_code(code)
            }
        }

        impl From<$name> for Code {
            fn from(value: $name) -> Code {
                Code::new(value.code()).expect("codes in this crate are valid FHIR codes")
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.code())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                <Self as CodedEnum>::from_code(&raw).map_err(serde::de::Error::custom)
            }
        }

        impl Validate for $name {
            fn validate(&self, _validator: &mut Validator) {}
        }
    };
}

code_enum! {
    /// `Patient.gender` — the gender used for **administrative** purposes.
    ///
    /// This is not clinical sex, and not gender identity. FHIR is explicit that
    /// this element exists for record matching and correspondence; clinical sex
    /// observations and gender identity belong in `Observation` and the
    /// `patient-genderIdentity` extension respectively. Conflating them here is
    /// the single most common Patient modelling error.
    AdministrativeGender,
    system = "http://hl7.org/fhir/administrative-gender",
    value_set = "http://hl7.org/fhir/ValueSet/administrative-gender" {
        /// Male.
        Male => "male",
        /// Female.
        Female => "female",
        /// Neither male nor female.
        Other => "other",
        /// Not stated, or not known.
        Unknown => "unknown",
    }
}

code_enum! {
    /// `HumanName.use` — the purpose a name is used for.
    NameUse,
    system = "http://hl7.org/fhir/name-use",
    value_set = "http://hl7.org/fhir/ValueSet/name-use" {
        /// Known as, or conventional, or the one the patient prefers.
        Usual => "usual",
        /// The formal name as registered officially.
        Official => "official",
        /// A temporary name.
        Temp => "temp",
        /// A name the patient is known by but which is not their official name.
        Nickname => "nickname",
        /// Anonymous assigned name for privacy.
        Anonymous => "anonymous",
        /// A name no longer in use, e.g. a maiden name.
        Old => "old",
        /// A name used only since marriage.
        Maiden => "maiden",
    }
}

code_enum! {
    /// `ContactPoint.system` — the channel a contact point uses.
    ContactPointSystem,
    system = "http://hl7.org/fhir/contact-point-system",
    value_set = "http://hl7.org/fhir/ValueSet/contact-point-system" {
        /// Telephone, including mobile.
        Phone => "phone",
        /// Fax.
        Fax => "fax",
        /// Email.
        Email => "email",
        /// Pager.
        Pager => "pager",
        /// A contactable URL such as a web or chat address.
        Url => "url",
        /// SMS, specifically.
        Sms => "sms",
        /// Anything else, described in `ContactPoint.value`.
        Other => "other",
    }
}

code_enum! {
    /// `ContactPoint.use` — the context a contact point is used in.
    ContactPointUse,
    system = "http://hl7.org/fhir/contact-point-use",
    value_set = "http://hl7.org/fhir/ValueSet/contact-point-use" {
        /// A home contact.
        Home => "home",
        /// A work contact.
        Work => "work",
        /// A temporary contact.
        Temp => "temp",
        /// No longer in use.
        Old => "old",
        /// A mobile contact, reachable anywhere.
        Mobile => "mobile",
    }
}

code_enum! {
    /// `Address.use` — the context an address is used in.
    AddressUse,
    system = "http://hl7.org/fhir/address-use",
    value_set = "http://hl7.org/fhir/ValueSet/address-use" {
        /// A home address.
        Home => "home",
        /// A work address.
        Work => "work",
        /// A temporary address.
        Temp => "temp",
        /// No longer in use.
        Old => "old",
        /// An address to send bills to.
        Billing => "billing",
    }
}

code_enum! {
    /// `Address.type` — whether an address is physical, postal, or both.
    AddressType,
    system = "http://hl7.org/fhir/address-type",
    value_set = "http://hl7.org/fhir/ValueSet/address-type" {
        /// A mailing address only; may be a PO box.
        Postal => "postal",
        /// A physical location that can be visited.
        Physical => "physical",
        /// Both a mailing address and a physical location.
        Both => "both",
    }
}

code_enum! {
    /// `Identifier.use` — the purpose an identifier serves.
    ///
    /// [`IdentifierUse::Old`] matters more than it looks: an identifier marked
    /// old must not be used for matching, and a matching implementation that
    /// ignores this will happily merge a patient onto a retired MRN.
    IdentifierUse,
    system = "http://hl7.org/fhir/identifier-use",
    value_set = "http://hl7.org/fhir/ValueSet/identifier-use" {
        /// The identifier recommended for display and use.
        Usual => "usual",
        /// The officially assigned identifier.
        Official => "official",
        /// A temporary identifier.
        Temp => "temp",
        /// An identifier issued for a specific, secondary purpose.
        Secondary => "secondary",
        /// A retired identifier; must not be used for matching.
        Old => "old",
    }
}

code_enum! {
    /// `Patient.link.type` — how two patient records relate.
    ///
    /// The direction is the trap. `replaced-by` means *this* record is retired
    /// and the target is current; `replaces` means the opposite. Getting them
    /// backwards points every client at the dead record.
    LinkType,
    system = "http://hl7.org/fhir/link-type",
    value_set = "http://hl7.org/fhir/ValueSet/link-type" {
        /// This record is no longer current; use the target instead.
        ReplacedBy => "replaced-by",
        /// This record replaces the target, which is retired.
        Replaces => "replaces",
        /// The target contains additional data about the same patient.
        Refer => "refer",
        /// The two records are believed to be the same patient, unmerged.
        Seealso => "seealso",
    }
}

code_enum! {
    /// `Narrative.status` — how the narrative relates to the structured data.
    NarrativeStatus,
    system = "http://hl7.org/fhir/narrative-status",
    value_set = "http://hl7.org/fhir/ValueSet/narrative-status" {
        /// The narrative contains only the structured content.
        Generated => "generated",
        /// The narrative contains extra content not in the structured data.
        Extensions => "extensions",
        /// The narrative contains content additional to the structured data.
        Additional => "additional",
        /// The contents are not represented in the structured data at all.
        Empty => "empty",
    }
}

code_enum! {
    /// `Quantity.comparator` — how the value relates to the true measurement.
    ///
    /// Silently dropping this turns "less than 0.05" into "0.05". For a
    /// viral load or a drug level, that inverts the clinical meaning.
    QuantityComparator,
    system = "http://hl7.org/fhir/quantity-comparator",
    value_set = "http://hl7.org/fhir/ValueSet/quantity-comparator" {
        /// The actual value is less than the given value.
        LessThan => "<",
        /// The actual value is less than or equal to the given value.
        LessOrEqual => "<=",
        /// The actual value is greater than or equal to the given value.
        GreaterOrEqual => ">=",
        /// The actual value is greater than the given value.
        GreaterThan => ">",
        /// The actual value is sufficient for the total quantity to be reached.
        Sufficient => "ad",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_codes_are_rejected_at_the_boundary() {
        assert_eq!(
            AdministrativeGender::from_code("female").expect("known code"),
            AdministrativeGender::Female
        );
        assert!(AdministrativeGender::from_code("Female").is_err());
        assert!(AdministrativeGender::from_code("f").is_err());

        let error = serde_json::from_str::<AdministrativeGender>("\"nonbinary\"")
            .expect_err("unknown code");
        assert!(error.to_string().contains("administrative-gender"));
    }

    #[test]
    fn coding_carries_the_system() {
        let coding = AdministrativeGender::Other.to_coding();
        assert_eq!(
            coding.system.as_ref().map(|s| s.as_str()),
            Some("http://hl7.org/fhir/administrative-gender")
        );
        assert_eq!(coding.code.as_ref().map(|c| c.as_str()), Some("other"));
    }

    #[test]
    fn every_code_round_trips() {
        for gender in AdministrativeGender::ALL {
            let json = serde_json::to_string(gender).expect("serializes");
            let parsed: AdministrativeGender = serde_json::from_str(&json).expect("deserializes");
            assert_eq!(parsed, *gender);
        }
    }
}
