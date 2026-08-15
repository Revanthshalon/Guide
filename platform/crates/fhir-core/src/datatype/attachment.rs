//! `Attachment`: content referred to in-line or by URL.

use serde::{Deserialize, Serialize};

use crate::element::{Extension, impl_element};
use crate::error::IssueCode;
use crate::primitive::{
    Base64Binary, Code, DateTime, Decimal, FhirString, Integer64, PositiveInt, Url,
};
use crate::validate::{Validate, Validator};

/// FHIR `Attachment`: a photo, scanned document, or other binary content.
///
/// On `Patient.photo` this is normally a small identification photograph. Two
/// operational cautions:
///
/// * inline `data` is base64 and inflates by a third — a 2 MB photo becomes a
///   2.7 MB JSON field that every read of the patient will now carry;
/// * `url` may point anywhere, including inside your network. Anything that
///   fetches it is performing a request on behalf of untrusted data, so treat
///   attachment URLs as an SSRF surface and resolve them through an allow-list.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Mime type of the content, with charset etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<Code>,

    /// Human language of the content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Code>,

    /// Data inline, base64 encoded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Base64Binary>,

    /// URI where the data can be found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,

    /// Number of bytes of content, before base64 encoding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Integer64>,

    /// SHA-1 hash of the data, base64 encoded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<Base64Binary>,

    /// Label to display in place of the data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<FhirString>,

    /// Date the attachment was first created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation: Option<DateTime>,

    /// Height of the image in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<PositiveInt>,

    /// Width of the image in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<PositiveInt>,

    /// Number of frames in a multi-frame image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frames: Option<PositiveInt>,

    /// Length of the content in seconds, for audio and video.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<Decimal>,

    /// Number of printed pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<PositiveInt>,
}

impl Attachment {
    /// Whether this attachment carries nothing at all.
    pub fn is_empty(&self) -> bool {
        self.content_type.is_none()
            && self.language.is_none()
            && self.data.is_none()
            && self.url.is_none()
            && self.size.is_none()
            && self.hash.is_none()
            && self.title.is_none()
            && self.creation.is_none()
            && self.height.is_none()
            && self.width.is_none()
            && self.frames.is_none()
            && self.duration.is_none()
            && self.pages.is_none()
            && self.extension.is_empty()
    }
}

impl_element!(Attachment);

impl Validate for Attachment {
    fn validate(&self, validator: &mut Validator) {
        validator.invariant(
            "ele-1",
            !self.is_empty(),
            "All FHIR elements must have a value or children",
        );

        // att-1: If the Attachment has data, it SHALL have a contentType.
        validator.invariant(
            "att-1",
            !(self.data.is_some() && self.content_type.is_none()),
            "If the Attachment has data, it SHALL have a contentType",
        );

        // A stated size that contradicts the inline data means one of the two
        // is wrong, and consumers pick different ones.
        if let (Some(data), Some(size)) = (&self.data, &self.size) {
            let decoded = data.decoded_len() as i64;
            if decoded != size.get() {
                validator.error_at(
                    "size",
                    IssueCode::Value,
                    format!(
                        "size is {} but the inline data decodes to {decoded} bytes",
                        size.get()
                    ),
                );
            }
        }

        validator.field("extension", &self.extension);
    }
}
