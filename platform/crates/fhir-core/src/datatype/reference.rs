//! `Reference<T>`: a link to another resource, with the permitted target types
//! encoded in the Rust type.
//!
//! # The idea
//!
//! FHIR states reference constraints in prose: *"Patient.managingOrganization —
//! Reference(Organization)"*. In an untyped model, that constraint lives in a
//! profile and is checked, if at all, by a validator at the far end of the
//! pipeline. Here it is a type parameter, so
//! `Reference<marker::Organization>` cannot be assigned a `Practitioner` link
//! by a mistaken refactor, and the multi-target case is spelled out as a tuple:
//! `Reference<(Organization, Practitioner, PractitionerRole)>`.
//!
//! The check is not purely compile-time — a literal reference string arrives
//! from JSON at runtime — so [`Reference::check`] parses the literal and
//! confirms the target type. What the type parameter guarantees is that the
//! *check that runs* is the right one for that field, chosen by the compiler
//! rather than by whoever wrote the call site.

use std::fmt;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::element::{Element, Extension};
use crate::error::{IssueCode, ParseError, Severity, ValidationError};
use crate::primitive::{FhirString, Uri};
use crate::resource::{Any, ReferenceTarget, ResourceMarker, ResourceType};
use crate::validate::{Validate, Validator};

use super::Identifier;

/// FHIR `Reference`, parameterised by what it is allowed to point at.
///
/// `R` defaults to [`Any`], matching the FHIR elements that permit any resource.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", bound(serialize = "", deserialize = ""))]
pub struct Reference<R = Any> {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Literal reference: a relative, internal, or absolute URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<FhirString>,

    /// The type the reference refers to, as a URI. Useful when `reference`
    /// is absent and only a logical `identifier` is supplied.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub target_type: Option<Uri>,

    /// A logical reference, used when the resource has no URL yet — an MRN
    /// instead of `Patient/123`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Box<Identifier>>,

    /// Text alternative for the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<FhirString>,

    #[serde(skip)]
    target: PhantomData<fn() -> R>,
}

/// How a literal reference string is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    /// `#contained-id` — points into the containing resource's `contained` list.
    Internal,
    /// `Patient/123` or `Patient/123/_history/2` — relative to a server base.
    Relative,
    /// `http://server/fhir/Patient/123` — an absolute URL.
    Absolute,
    /// `urn:uuid:…` or `urn:oid:…` — used inside transaction bundles.
    Urn,
}

/// The parts of a literal reference this crate can recover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedReference<'a> {
    /// How the reference is written.
    pub kind: ReferenceKind,
    /// The resource type segment, when the form has one and it is known.
    pub resource_type: Option<ResourceType>,
    /// The resource type segment as written, when the form has one.
    pub resource_type_name: Option<&'a str>,
    /// The logical id segment, when the form has one.
    pub id: Option<&'a str>,
    /// The `_history` version segment, when present.
    pub version_id: Option<&'a str>,
}

impl<R> Default for Reference<R> {
    fn default() -> Self {
        Self {
            id: None,
            extension: Vec::new(),
            reference: None,
            target_type: None,
            identifier: None,
            display: None,
            target: PhantomData,
        }
    }
}

// Derived `Clone`/`Debug`/`PartialEq` would demand `R: Clone` and friends even
// though `PhantomData<fn() -> R>` needs nothing of the sort, so they are
// written out.
impl<R> Clone for Reference<R> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            extension: self.extension.clone(),
            reference: self.reference.clone(),
            target_type: self.target_type.clone(),
            identifier: self.identifier.clone(),
            display: self.display.clone(),
            target: PhantomData,
        }
    }
}

impl<R> fmt::Debug for Reference<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Reference")
            .field("id", &self.id)
            .field("extension", &self.extension)
            .field("reference", &self.reference)
            .field("type", &self.target_type)
            .field("identifier", &self.identifier)
            .field("display", &self.display)
            .finish()
    }
}

impl<R> PartialEq for Reference<R> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.extension == other.extension
            && self.reference == other.reference
            && self.target_type == other.target_type
            && self.identifier == other.identifier
            && self.display == other.display
    }
}

impl<R> Reference<R> {
    /// A reference from a literal reference string.
    pub fn literal(reference: &str) -> Result<Self, ParseError> {
        Ok(Self {
            reference: Some(FhirString::new(reference)?),
            ..Self::default()
        })
    }

    /// A logical reference: no URL, just a business identifier such as an MRN.
    ///
    /// This is the form to use when a record is being created from a feed that
    /// only knows the patient by their hospital number.
    pub fn logical(identifier: Identifier) -> Self {
        Self {
            identifier: Some(Box::new(identifier)),
            ..Self::default()
        }
    }

    /// Add a display string.
    pub fn with_display(mut self, display: FhirString) -> Self {
        self.display = Some(display);
        self
    }

    /// Break the literal reference into its parts.
    pub fn parse_literal(&self) -> Option<ParsedReference<'_>> {
        parse_literal(self.reference.as_ref()?.as_str())
    }
}

impl<R: ResourceMarker> Reference<R> {
    /// A relative reference to a resource of the target type: `Patient/123`.
    ///
    /// Available only when the target is a single type, because that is the
    /// only case in which the resource type can be filled in for you.
    pub fn to_id(id: &crate::primitive::Id) -> Self {
        let literal = format!("{}/{}", R::RESOURCE_TYPE.as_str(), id.as_str());
        Self {
            reference: Some(FhirString::new(literal).expect("resource type and id are non-empty")),
            target_type: Uri::new(R::RESOURCE_TYPE.as_str()).ok(),
            ..Self::default()
        }
    }
}

impl<R: ReferenceTarget> Reference<R> {
    /// Validate this reference on its own, outside a resource.
    pub fn check(&self) -> Result<(), ValidationError> {
        let mut validator = Validator::new("Reference");
        self.validate(&mut validator);
        validator.into_report().into_result()
    }
}

impl<R> Element for Reference<R> {
    fn id(&self) -> Option<&FhirString> {
        self.id.as_ref()
    }

    fn extension(&self) -> &[Extension] {
        &self.extension
    }
}

impl<R: ReferenceTarget> Validate for Reference<R> {
    fn validate(&self, validator: &mut Validator) {
        // ref-1: SHALL have a contained resource if a local reference is
        // provided; in practice, a reference must say *something*.
        let has_content = self.reference.is_some()
            || self.identifier.is_some()
            || self.display.is_some()
            || !self.extension.is_empty();
        validator.invariant(
            "ref-1",
            has_content,
            "A Reference SHALL have a reference, an identifier, or a display",
        );

        if let Some(parsed) = self.parse_literal() {
            match (parsed.resource_type, parsed.resource_type_name) {
                (Some(resource_type), _) if !R::accepts(resource_type) => {
                    validator.error_at(
                        "reference",
                        IssueCode::Value,
                        format!(
                            "reference points at {resource_type}, but this element accepts {}",
                            R::describe()
                        ),
                    );
                }
                (None, Some(name)) => {
                    // A syntactically fine reference to a type this build does
                    // not model. Refusing it would break interoperability with
                    // servers that support more of FHIR than we do.
                    validator.enter("reference", |v| {
                        v.issue(
                            Severity::Warning,
                            IssueCode::NotFound,
                            format!("resource type {name:?} is not known to this build"),
                        );
                    });
                }
                _ => {}
            }

            // A `type` element that disagrees with the literal reference is a
            // data-integrity bug — a consumer may trust either one.
            if let (Some(declared), Some(actual)) =
                (self.target_type.as_ref(), parsed.resource_type_name)
                && declared.as_str() != actual
                && !declared.as_str().ends_with(&format!("/{actual}"))
            {
                validator.error_at(
                    "type",
                    IssueCode::Value,
                    format!(
                        "Reference.type is {:?} but the literal reference points at {actual:?}",
                        declared.as_str()
                    ),
                );
            }
        } else if self.reference.is_some() {
            validator.error_at(
                "reference",
                IssueCode::Value,
                "literal reference is not a recognised URL, relative reference, internal reference, or URN",
            );
        }

        validator.field("identifier", &self.identifier);
        validator.field("extension", &self.extension);
    }
}

/// Split a literal reference into its parts.
///
/// Recognises the four forms FHIR defines. Returns `None` when the string is
/// none of them, which validation reports as an error.
fn parse_literal(reference: &str) -> Option<ParsedReference<'_>> {
    if reference.is_empty() {
        return None;
    }

    if let Some(id) = reference.strip_prefix('#') {
        return Some(ParsedReference {
            kind: ReferenceKind::Internal,
            resource_type: None,
            resource_type_name: None,
            id: (!id.is_empty()).then_some(id),
            version_id: None,
        });
    }

    if reference.starts_with("urn:") {
        return Some(ParsedReference {
            kind: ReferenceKind::Urn,
            resource_type: None,
            resource_type_name: None,
            id: Some(reference),
            version_id: None,
        });
    }

    let absolute = reference.contains("://");
    let segments: Vec<&str> = reference.split('/').filter(|s| !s.is_empty()).collect();

    // Walk back from the end: […]/{type}/{id}[/_history/{vid}]
    let (type_index, id_index, version_id) = match segments.as_slice() {
        [.., type_segment, _id, history, version]
            if *history == "_history" && !type_segment.is_empty() =>
        {
            (segments.len() - 4, segments.len() - 3, Some(*version))
        }
        [.., _type, _id] => (segments.len() - 2, segments.len() - 1, None),
        _ => return None,
    };

    let type_name = segments[type_index];
    if !type_name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
    {
        return None;
    }

    Some(ParsedReference {
        kind: if absolute {
            ReferenceKind::Absolute
        } else {
            ReferenceKind::Relative
        },
        resource_type: ResourceType::parse(type_name),
        resource_type_name: Some(type_name),
        id: Some(segments[id_index]),
        version_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::marker::{Organization, Practitioner, PractitionerRole};

    #[test]
    fn typed_reference_rejects_the_wrong_target() {
        let good: Reference<Organization> =
            Reference::literal("Organization/hospital-1").expect("valid literal");
        assert!(good.check().is_ok());

        let bad: Reference<Organization> =
            Reference::literal("Practitioner/23").expect("valid literal");
        let error = bad.check().expect_err("wrong target type");
        assert!(
            error
                .errors()
                .any(|issue| issue.message.contains("accepts Organization"))
        );
    }

    #[test]
    fn tuple_targets_accept_any_member() {
        type Gp = Reference<(Organization, Practitioner, PractitionerRole)>;

        for literal in [
            "Organization/1",
            "Practitioner/2",
            "PractitionerRole/3",
            "http://example.org/fhir/Practitioner/4",
        ] {
            let reference: Gp = Reference::literal(literal).expect("valid literal");
            assert!(reference.check().is_ok(), "{literal} should be accepted");
        }

        let wrong: Gp = Reference::literal("Device/5").expect("valid literal");
        assert!(wrong.check().is_err());
    }

    #[test]
    fn literal_forms_are_recognised() {
        let history: Reference<Organization> =
            Reference::literal("http://example.org/fhir/Organization/1/_history/3")
                .expect("valid literal");
        let parsed = history.parse_literal().expect("parses");
        assert_eq!(parsed.kind, ReferenceKind::Absolute);
        assert_eq!(parsed.resource_type, Some(ResourceType::Organization));
        assert_eq!(parsed.id, Some("1"));
        assert_eq!(parsed.version_id, Some("3"));

        let internal: Reference<Any> = Reference::literal("#org-1").expect("valid literal");
        assert_eq!(
            internal.parse_literal().expect("parses").kind,
            ReferenceKind::Internal
        );

        let urn: Reference<Any> =
            Reference::literal("urn:uuid:c757873d-ec9a-4326-a141-556f43239520")
                .expect("valid literal");
        assert_eq!(
            urn.parse_literal().expect("parses").kind,
            ReferenceKind::Urn
        );

        let nonsense: Reference<Any> = Reference::literal("not-a-reference").expect("valid string");
        assert!(nonsense.parse_literal().is_none());
        assert!(nonsense.check().is_err());
    }

    #[test]
    fn empty_reference_fails_ref_1() {
        let empty: Reference<Any> = Reference::default();
        let error = empty.check().expect_err("empty reference");
        assert!(error.errors().any(|issue| issue.key == Some("ref-1")));
    }
}
