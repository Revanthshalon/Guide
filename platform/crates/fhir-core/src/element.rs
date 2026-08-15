//! `Element` and `BackboneElement` — the two base types every other FHIR
//! structure is built from — plus [`Extension`], the mechanism that makes FHIR
//! extensible without forking the schema.
//!
//! # The base types, and why they are traits here
//!
//! In the specification, `Element` is the abstract base of everything: it
//! contributes `id` and `extension`. `BackboneElement` extends it with
//! `modifierExtension` and is the base of elements nested *inside* a resource
//! (`Patient.contact` is a backbone element; `HumanName` is not).
//!
//! Rust has no inheritance, and the obvious workaround — a shared `ElementBase`
//! struct embedded with `#[serde(flatten)]` — makes the JSON mapping harder to
//! reason about and buries `id`/`extension` one level down in every constructor.
//! So the fields are declared directly on each type (which also keeps the Rust
//! field list identical to the FHIR element list), and the *behaviour* is
//! shared through the [`Element`] and [`BackboneElement`] traits. Generic code
//! that needs "anything with extensions" takes `T: Element`.
//!
//! # Why `modifierExtension` is not just another extension
//!
//! An ordinary extension may be ignored safely by a system that does not
//! understand it. A modifier extension may **not**: it changes the meaning of
//! the element containing it. `Patient.contact.modifierExtension` could say
//! "this contact is prohibited from receiving information". Dropping it while
//! keeping the contact is a safety incident, not a data-quality one — which is
//! why the type system keeps them in separate fields and
//! [`BackboneElement::modifier_extension`] exists as its own accessor.

use serde::{Deserialize, Serialize};

use crate::datatype::{
    Address, Annotation, Attachment, CodeableConcept, Coding, ContactPoint, HumanName, Identifier,
    Period, Quantity, Range, Ratio, Reference,
};
use crate::error::IssueCode;
use crate::primitive::{
    Base64Binary, Boolean, Canonical, Code, Date, DateTime, Decimal, FhirString, Id, Instant,
    Integer, Markdown, Oid, PositiveInt, Time, UnsignedInt, Uri, Url, Uuid,
};
use crate::validate::{Validate, Validator};

/// Anything that carries `Element.id` and `Element.extension`.
pub trait Element {
    /// The element's internal id, used as the target of an internal reference.
    fn id(&self) -> Option<&FhirString>;

    /// Extensions attached to this element. Safe to ignore if unrecognised.
    fn extension(&self) -> &[Extension];

    /// Look up an extension by its defining URL.
    ///
    /// Extensions are repeating, so this returns the first match; use
    /// [`Element::extensions_with_url`] when a definition allows repeats.
    fn extension_with_url<'a>(&'a self, url: &'a str) -> Option<&'a Extension> {
        self.extensions_with_url(url).next()
    }

    /// Every extension with the given defining URL.
    fn extensions_with_url<'a>(&'a self, url: &'a str) -> impl Iterator<Item = &'a Extension> {
        self.extension()
            .iter()
            .filter(move |extension| extension.url.as_str() == url)
    }
}

/// An [`Element`] nested inside a resource, which may also carry
/// `modifierExtension`.
pub trait BackboneElement: Element {
    /// Extensions that change the meaning of the containing element. A system
    /// that does not understand one of these **must not** process the element.
    fn modifier_extension(&self) -> &[Extension];
}

/// Generate the [`Element`] implementation for a struct with `id` and
/// `extension` fields.
macro_rules! impl_element {
    ($type:ty) => {
        impl $crate::element::Element for $type {
            fn id(&self) -> Option<&$crate::primitive::FhirString> {
                self.id.as_ref()
            }

            fn extension(&self) -> &[$crate::element::Extension] {
                &self.extension
            }
        }
    };
}

/// Generate [`Element`] and [`BackboneElement`] for a struct that also has a
/// `modifier_extension` field.
macro_rules! impl_backbone_element {
    ($type:ty) => {
        $crate::element::impl_element!($type);

        impl $crate::element::BackboneElement for $type {
            fn modifier_extension(&self) -> &[$crate::element::Extension] {
                &self.modifier_extension
            }
        }
    };
}

pub(crate) use {impl_backbone_element, impl_element};

/// FHIR `Extension`: additional content defined by an implementation.
///
/// The `url` is the extension's *definition*, not a link to fetch — two systems
/// agree on the meaning of an extension by agreeing on this URL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extension {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Nested extensions. Present only when this extension has no value.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Canonical URL identifying the extension's definition.
    pub url: Uri,

    /// The extension's value, serialized as `value[x]`.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub value: Option<ExtensionValue>,
}

impl Extension {
    /// A simple extension with a value.
    pub fn new(url: Uri, value: ExtensionValue) -> Self {
        Self {
            id: None,
            extension: Vec::new(),
            url,
            value: Some(value),
        }
    }

    /// A complex extension whose content is nested extensions.
    pub fn nested(url: Uri, extension: Vec<Extension>) -> Self {
        Self {
            id: None,
            extension,
            url,
            value: None,
        }
    }
}

impl Element for Extension {
    fn id(&self) -> Option<&FhirString> {
        self.id.as_ref()
    }

    fn extension(&self) -> &[Extension] {
        &self.extension
    }
}

impl Validate for Extension {
    fn validate(&self, validator: &mut Validator) {
        // ext-1: Must have either extensions or value[x], not both.
        let has_value = self.value.is_some();
        let has_nested = !self.extension.is_empty();
        validator.invariant(
            "ext-1",
            has_value ^ has_nested,
            "An extension SHALL have either a value[x] or nested extensions, not both and not neither",
        );

        // An extension URL must be absolute so that its meaning is globally
        // resolvable; a relative URL is only meaningful inside a profile.
        let absolute = self.url.as_str().contains(':');
        validator.enter("url", |v| {
            if !absolute {
                v.error(
                    IssueCode::Value,
                    "extension URL must be an absolute URI identifying the extension definition",
                );
            }
        });

        validator.field("extension", &self.extension);
        validator.field("value", &self.value);
    }
}

/// The `value[x]` of an [`Extension`].
///
/// FHIR permits every datatype here; this enum carries the subset the crate
/// models. Serialization is externally tagged and flattened into the parent
/// object, which is exactly the FHIR JSON shape: `{"url": "...",
/// "valueBoolean": true}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExtensionValue {
    /// `valueBase64Binary`
    #[serde(rename = "valueBase64Binary")]
    Base64Binary(Base64Binary),
    /// `valueBoolean`
    #[serde(rename = "valueBoolean")]
    Boolean(Boolean),
    /// `valueCanonical`
    #[serde(rename = "valueCanonical")]
    Canonical(Canonical),
    /// `valueCode`
    #[serde(rename = "valueCode")]
    Code(Code),
    /// `valueDate`
    #[serde(rename = "valueDate")]
    Date(Date),
    /// `valueDateTime`
    #[serde(rename = "valueDateTime")]
    DateTime(DateTime),
    /// `valueDecimal`
    #[serde(rename = "valueDecimal")]
    Decimal(Decimal),
    /// `valueId`
    #[serde(rename = "valueId")]
    Id(Id),
    /// `valueInstant`
    #[serde(rename = "valueInstant")]
    Instant(Instant),
    /// `valueInteger`
    #[serde(rename = "valueInteger")]
    Integer(Integer),
    /// `valueMarkdown`
    #[serde(rename = "valueMarkdown")]
    Markdown(Markdown),
    /// `valueOid`
    #[serde(rename = "valueOid")]
    Oid(Oid),
    /// `valuePositiveInt`
    #[serde(rename = "valuePositiveInt")]
    PositiveInt(PositiveInt),
    /// `valueString`
    #[serde(rename = "valueString")]
    String(FhirString),
    /// `valueTime`
    #[serde(rename = "valueTime")]
    Time(Time),
    /// `valueUnsignedInt`
    #[serde(rename = "valueUnsignedInt")]
    UnsignedInt(UnsignedInt),
    /// `valueUri`
    #[serde(rename = "valueUri")]
    Uri(Uri),
    /// `valueUrl`
    #[serde(rename = "valueUrl")]
    Url(Url),
    /// `valueUuid`
    #[serde(rename = "valueUuid")]
    Uuid(Uuid),
    /// `valueAddress`
    #[serde(rename = "valueAddress")]
    Address(Address),
    /// `valueAnnotation`
    #[serde(rename = "valueAnnotation")]
    Annotation(Annotation),
    /// `valueAttachment`
    #[serde(rename = "valueAttachment")]
    Attachment(Attachment),
    /// `valueCodeableConcept`
    #[serde(rename = "valueCodeableConcept")]
    CodeableConcept(CodeableConcept),
    /// `valueCoding`
    #[serde(rename = "valueCoding")]
    Coding(Coding),
    /// `valueContactPoint`
    #[serde(rename = "valueContactPoint")]
    ContactPoint(ContactPoint),
    /// `valueHumanName`
    #[serde(rename = "valueHumanName")]
    HumanName(HumanName),
    /// `valueIdentifier`
    #[serde(rename = "valueIdentifier")]
    Identifier(Identifier),
    /// `valuePeriod`
    #[serde(rename = "valuePeriod")]
    Period(Period),
    /// `valueQuantity`
    #[serde(rename = "valueQuantity")]
    Quantity(Quantity),
    /// `valueRange`
    #[serde(rename = "valueRange")]
    Range(Range),
    /// `valueRatio`
    #[serde(rename = "valueRatio")]
    Ratio(Ratio),
    /// `valueReference`
    #[serde(rename = "valueReference")]
    Reference(Box<Reference>),
}

impl Validate for ExtensionValue {
    fn validate(&self, validator: &mut Validator) {
        match self {
            Self::Address(value) => value.validate(validator),
            Self::Annotation(value) => value.validate(validator),
            Self::Attachment(value) => value.validate(validator),
            Self::CodeableConcept(value) => value.validate(validator),
            Self::Coding(value) => value.validate(validator),
            Self::ContactPoint(value) => value.validate(validator),
            Self::HumanName(value) => value.validate(validator),
            Self::Identifier(value) => value.validate(validator),
            Self::Period(value) => value.validate(validator),
            Self::Quantity(value) => value.validate(validator),
            Self::Range(value) => value.validate(validator),
            Self::Ratio(value) => value.validate(validator),
            Self::Reference(value) => value.validate(validator),
            // Primitive values validated themselves at construction.
            _ => {}
        }
    }
}
