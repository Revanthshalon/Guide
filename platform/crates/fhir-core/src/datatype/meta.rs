//! `Meta` and `Narrative`: the infrastructure metadata and the human-readable
//! summary every domain resource carries.

use serde::{Deserialize, Serialize};

use crate::codes::NarrativeStatus;
use crate::element::{Extension, impl_element};
use crate::primitive::{Canonical, FhirString, Id, Instant, Uri, Xhtml};
use crate::validate::{Validate, Validator};

use super::Coding;

/// FHIR `Meta`: metadata maintained by the infrastructure, not by the clinician.
///
/// # `versionId` is the optimistic-locking token
///
/// A conditional update sends the version it read as `If-Match`; the server
/// rejects the write if the resource has moved on. Ignoring this is how two
/// concurrent edits to one patient silently overwrite each other — the classic
/// lost-update, with a demographic record as the casualty.
///
/// # `security` labels travel with the data
///
/// Confidentiality and sensitivity labels on a resource are not decoration:
/// they are inputs to access control at every hop. Copying a resource while
/// dropping `meta.security` strips the marking that says "this record concerns
/// a restricted episode of care".
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Version-specific identifier, changed by the server on every write.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<Id>,

    /// When the resource version last changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<Instant>,

    /// Identifies where the resource came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Uri>,

    /// Profiles this resource claims to conform to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profile: Vec<Canonical>,

    /// Security labels applied to this resource.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<Coding>,

    /// Tags applied to this resource.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tag: Vec<Coding>,
}

impl Meta {
    /// Whether this metadata carries nothing at all.
    pub fn is_empty(&self) -> bool {
        self.version_id.is_none()
            && self.last_updated.is_none()
            && self.source.is_none()
            && self.profile.is_empty()
            && self.security.is_empty()
            && self.tag.is_empty()
            && self.extension.is_empty()
    }

    /// Whether the resource claims conformance to the given profile URL,
    /// ignoring any `|version` suffix.
    pub fn claims_profile(&self, url: &str) -> bool {
        self.profile.iter().any(|profile| profile.url() == url)
    }
}

impl_element!(Meta);

impl Validate for Meta {
    fn validate(&self, validator: &mut Validator) {
        validator.field("security", &self.security);
        validator.field("tag", &self.tag);
        validator.field("extension", &self.extension);
    }
}

/// FHIR `Narrative`: the human-readable rendering of a resource.
///
/// The narrative is what a clinician sees when their system does not
/// understand the structured data — which makes it a *safety* feature, and the
/// reason `dom-6` recommends every resource carry one. It is also attacker-
/// reachable markup; see [`Xhtml`](crate::primitive::Xhtml).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Narrative {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// How the narrative relates to the structured content.
    pub status: NarrativeStatus,

    /// The XHTML content, limited to basic formatting.
    pub div: Xhtml,
}

impl Narrative {
    /// A narrative generated from the structured content.
    pub fn generated(div: Xhtml) -> Self {
        Self {
            id: None,
            extension: Vec::new(),
            status: NarrativeStatus::Generated,
            div,
        }
    }
}

impl_element!(Narrative);

impl Validate for Narrative {
    fn validate(&self, validator: &mut Validator) {
        validator.field("extension", &self.extension);
    }
}
