//! The string-shaped FHIR primitives.
//!
//! All of them share one macro-generated shape, so the only thing that differs
//! between `Id` and `Oid` is the checking function and the length cap. The
//! checks are hand-written character scans rather than regexes: the FHIR
//! patterns are simple enough that a scan is both faster and able to report
//! *which* character was rejected and where.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{ParseError, ParseErrorKind};
use crate::primitive::{PrimitiveType, StringPrimitive};
use crate::validate::{Validate, Validator};

/// Maximum length shared by FHIR's unbounded string-ish types (1 MiB).
pub(crate) const MAX_STRING_LENGTH: usize = 1_048_576;

macro_rules! string_primitive {
    (
        $(#[$attr:meta])*
        $name:ident, fhir = $fhir:literal, max = $max:expr, check = $check:path $(,)?
    ) => {
        $(#[$attr])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// The FHIR type name.
            pub const FHIR_TYPE: &'static str = $fhir;

            /// Maximum length in characters.
            pub const MAX_LENGTH: usize = $max;

            /// Validate `value` and wrap it.
            pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
                let value = value.into();
                let length = value.chars().count();
                if length > Self::MAX_LENGTH {
                    return Err(ParseError::new(
                        $fhir,
                        ParseErrorKind::TooLong {
                            max: Self::MAX_LENGTH,
                            actual: length,
                        },
                    ));
                }
                $check(&value).map_err(|kind| ParseError::new($fhir, kind))?;
                Ok(Self(value))
            }

            /// The value's lexical form.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume the newtype, returning the inner `String`.
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl PrimitiveType for $name {
            const FHIR_TYPE: &'static str = $fhir;
        }

        impl StringPrimitive for $name {
            fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = ParseError;

            fn from_str(value: &str) -> Result<Self, ParseError> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = ParseError;

            fn try_from(value: String) -> Result<Self, ParseError> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ParseError;

            fn try_from(value: &str) -> Result<Self, ParseError> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.0
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                Self::new(raw).map_err(serde::de::Error::custom)
            }
        }

        // A primitive is valid by construction, so the structural pass has
        // nothing left to check.
        impl Validate for $name {
            fn validate(&self, _validator: &mut Validator) {}
        }
    };
}

string_primitive! {
    /// FHIR `string`: human-readable text, 1 character to 1 MiB.
    ///
    /// Named `FhirString` rather than `String` so that the standard library's
    /// `String` stays unshadowed in downstream code.
    ///
    /// Rejects values that are empty or entirely whitespace, and control
    /// characters other than tab, carriage return, and line feed — the usual
    /// carriers of a broken upstream encoding.
    FhirString, fhir = "string", max = MAX_STRING_LENGTH, check = check_string,
}

string_primitive! {
    /// FHIR `code`: a token from a code system.
    ///
    /// No leading or trailing whitespace, and internal whitespace only as
    /// single spaces (regex `[^\s]+( [^\s]+)*`). The rule exists because codes
    /// are compared byte-for-byte; a trailing space silently breaks equality
    /// against a terminology server.
    Code, fhir = "code", max = MAX_STRING_LENGTH, check = check_code,
}

string_primitive! {
    /// FHIR `id`: the logical id of a resource, `[A-Za-z0-9\-\.]{1,64}`.
    ///
    /// Note what is *not* allowed: underscores, slashes, and any Unicode beyond
    /// ASCII. Systems that mint ids from an internal database key with a
    /// separator (`org_1234`) will fail here, which is the point — that id
    /// cannot be used in a RESTful URL.
    Id, fhir = "id", max = 64, check = check_id,
}

string_primitive! {
    /// FHIR `uri`: any URI, absolute or relative. No whitespace, non-empty.
    Uri, fhir = "uri", max = MAX_STRING_LENGTH, check = check_uri,
}

string_primitive! {
    /// FHIR `url`: a URI that is expected to be resolvable.
    ///
    /// The specification's regex is identical to `uri`; the type exists to
    /// carry the intent, and keeping it distinct means an `Attachment.url`
    /// cannot be assigned a `Coding.system` by accident.
    Url, fhir = "url", max = MAX_STRING_LENGTH, check = check_uri,
}

string_primitive! {
    /// FHIR `canonical`: a URI referring to a versioned resource, optionally
    /// with `|version` and `#fragment` suffixes.
    ///
    /// Use [`Canonical::url`], [`Canonical::version`], and
    /// [`Canonical::fragment`] rather than splitting the string at call sites —
    /// the order of the two suffixes is fixed by the spec and easy to get wrong.
    Canonical, fhir = "canonical", max = MAX_STRING_LENGTH, check = check_uri,
}

string_primitive! {
    /// FHIR `oid`: an ISO OID in URN form, `urn:oid:1.2.840.113619.2.62`.
    Oid, fhir = "oid", max = MAX_STRING_LENGTH, check = check_oid,
}

string_primitive! {
    /// FHIR `uuid`: a UUID in URN form, `urn:uuid:c757873d-ec9a-4326-a141-556f43239520`.
    Uuid, fhir = "uuid", max = MAX_STRING_LENGTH, check = check_uuid,
}

string_primitive! {
    /// FHIR `markdown`: a string that may contain GitHub-flavoured markdown.
    ///
    /// Same lexical rules as `string`. Rendering is a downstream concern, but
    /// note that markdown permits embedded HTML — anything that renders this
    /// must sanitise, exactly as with [`Xhtml`].
    Markdown, fhir = "markdown", max = MAX_STRING_LENGTH, check = check_string,
}

string_primitive! {
    /// FHIR `base64Binary`: base64-encoded data, whitespace permitted.
    Base64Binary, fhir = "base64Binary", max = MAX_STRING_LENGTH, check = check_base64,
}

string_primitive! {
    /// FHIR `xhtml`: the narrative body of a resource.
    ///
    /// This is the one primitive with a *security* rule rather than a
    /// formatting one. `Narrative.div` is rendered by clients, so FHIR's
    /// invariant `txt-1` limits it to basic formatting elements: no scripts, no
    /// forms, no external content, no event handlers. The check here enforces
    /// the dangerous half of that list. It is a defence in depth, not a
    /// substitute for sanitising at render time.
    Xhtml, fhir = "xhtml", max = MAX_STRING_LENGTH, check = check_xhtml,
}

impl Canonical {
    /// The URL part, with any `|version` and `#fragment` removed.
    pub fn url(&self) -> &str {
        let without_fragment = self.0.split('#').next().unwrap_or(&self.0);
        without_fragment
            .split('|')
            .next()
            .unwrap_or(without_fragment)
    }

    /// The `|version` suffix, if present.
    pub fn version(&self) -> Option<&str> {
        let without_fragment = self.0.split('#').next().unwrap_or(&self.0);
        let mut parts = without_fragment.splitn(2, '|');
        parts.next();
        parts.next()
    }

    /// The `#fragment` suffix, if present.
    pub fn fragment(&self) -> Option<&str> {
        let mut parts = self.0.splitn(2, '#');
        parts.next();
        parts.next()
    }
}

impl Base64Binary {
    /// Number of bytes the encoded payload decodes to.
    ///
    /// Useful for enforcing an attachment size limit before decoding, which is
    /// the whole point of checking it here rather than after allocation.
    pub fn decoded_len(&self) -> usize {
        let significant: Vec<char> = self.0.chars().filter(|c| !c.is_whitespace()).collect();
        let padding = significant.iter().rev().take_while(|c| **c == '=').count();
        significant.len() / 4 * 3 - padding
    }
}

// ---------------------------------------------------------------------------
// Checking functions
// ---------------------------------------------------------------------------

fn check_string(value: &str) -> Result<(), ParseErrorKind> {
    if value.is_empty() || value.chars().all(char::is_whitespace) {
        return Err(ParseErrorKind::Empty);
    }
    for (index, character) in value.chars().enumerate() {
        let allowed =
            !character.is_control() || character == '\t' || character == '\r' || character == '\n';
        if !allowed {
            return Err(ParseErrorKind::IllegalCharacter { index, character });
        }
    }
    Ok(())
}

fn check_code(value: &str) -> Result<(), ParseErrorKind> {
    if value.is_empty() {
        return Err(ParseErrorKind::Empty);
    }
    let last_index = value.chars().count() - 1;
    let mut previous_was_space = false;
    for (index, character) in value.chars().enumerate() {
        if character.is_whitespace() {
            let at_edge = index == 0 || index == last_index;
            if character != ' ' || at_edge || previous_was_space {
                return Err(ParseErrorKind::IllegalCharacter { index, character });
            }
            previous_was_space = true;
        } else {
            if character.is_control() {
                return Err(ParseErrorKind::IllegalCharacter { index, character });
            }
            previous_was_space = false;
        }
    }
    Ok(())
}

fn check_id(value: &str) -> Result<(), ParseErrorKind> {
    if value.is_empty() {
        return Err(ParseErrorKind::Empty);
    }
    for (index, character) in value.chars().enumerate() {
        if !(character.is_ascii_alphanumeric() || character == '-' || character == '.') {
            return Err(ParseErrorKind::IllegalCharacter { index, character });
        }
    }
    Ok(())
}

fn check_uri(value: &str) -> Result<(), ParseErrorKind> {
    if value.is_empty() {
        return Err(ParseErrorKind::Empty);
    }
    for (index, character) in value.chars().enumerate() {
        if character.is_whitespace() || character.is_control() {
            return Err(ParseErrorKind::IllegalCharacter { index, character });
        }
    }
    Ok(())
}

fn check_oid(value: &str) -> Result<(), ParseErrorKind> {
    const EXPECTED: &str = "`urn:oid:` followed by dot-separated decimal arcs";
    let Some(arcs) = value.strip_prefix("urn:oid:") else {
        return Err(ParseErrorKind::Malformed { expected: EXPECTED });
    };
    let mut arc_count = 0;
    for arc in arcs.split('.') {
        arc_count += 1;
        if arc.is_empty() || !arc.chars().all(|c| c.is_ascii_digit()) {
            return Err(ParseErrorKind::Malformed { expected: EXPECTED });
        }
        // Leading zeros are not permitted: `0` is fine, `01` is not.
        if arc.len() > 1 && arc.starts_with('0') {
            return Err(ParseErrorKind::Malformed { expected: EXPECTED });
        }
    }
    if arc_count < 2 {
        return Err(ParseErrorKind::Malformed { expected: EXPECTED });
    }
    Ok(())
}

fn check_uuid(value: &str) -> Result<(), ParseErrorKind> {
    const EXPECTED: &str = "`urn:uuid:` followed by 8-4-4-4-12 hexadecimal digits";
    let Some(body) = value.strip_prefix("urn:uuid:") else {
        return Err(ParseErrorKind::Malformed { expected: EXPECTED });
    };
    let groups: Vec<&str> = body.split('-').collect();
    let expected_lengths = [8usize, 4, 4, 4, 12];
    if groups.len() != expected_lengths.len() {
        return Err(ParseErrorKind::Malformed { expected: EXPECTED });
    }
    for (group, length) in groups.iter().zip(expected_lengths) {
        if group.len() != length || !group.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ParseErrorKind::Malformed { expected: EXPECTED });
        }
    }
    Ok(())
}

fn check_base64(value: &str) -> Result<(), ParseErrorKind> {
    const EXPECTED: &str = "base64 data whose length is a multiple of 4";
    let mut significant = 0usize;
    let mut padding = 0usize;
    for (index, character) in value.chars().enumerate() {
        if character.is_whitespace() {
            continue;
        }
        if character == '=' {
            padding += 1;
            significant += 1;
            continue;
        }
        if padding > 0 {
            // Data after padding — the payload was concatenated, not appended.
            return Err(ParseErrorKind::IllegalCharacter { index, character });
        }
        let allowed = character.is_ascii_alphanumeric() || character == '+' || character == '/';
        if !allowed {
            return Err(ParseErrorKind::IllegalCharacter { index, character });
        }
        significant += 1;
    }
    if significant == 0 {
        return Err(ParseErrorKind::Empty);
    }
    if significant % 4 != 0 || padding > 2 {
        return Err(ParseErrorKind::Malformed { expected: EXPECTED });
    }
    Ok(())
}

/// Elements and attribute prefixes that make narrative active content.
///
/// `txt-1` phrases the rule as an allow-list of formatting elements; enforcing
/// the allow-list fully means parsing XHTML, which this crate does not do. The
/// deny-list below covers the constructs that turn a rendered narrative into an
/// XSS vector, and is checked case-insensitively because HTML tag names are.
const FORBIDDEN_MARKUP: &[&str] = &[
    "<script",
    "<style",
    "<iframe",
    "<object",
    "<embed",
    "<applet",
    "<form",
    "<input",
    "<button",
    "<base",
    "<link",
    "<meta",
    "javascript:",
    "vbscript:",
    "data:text/html",
];

fn check_xhtml(value: &str) -> Result<(), ParseErrorKind> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ParseErrorKind::Empty);
    }
    if !trimmed.starts_with("<div") {
        return Err(ParseErrorKind::Malformed {
            expected: "an XHTML fragment whose root element is <div>",
        });
    }
    if !trimmed.contains("xmlns=\"http://www.w3.org/1999/xhtml\"")
        && !trimmed.contains("xmlns='http://www.w3.org/1999/xhtml'")
    {
        return Err(ParseErrorKind::Malformed {
            expected: "the root <div> to declare xmlns=\"http://www.w3.org/1999/xhtml\"",
        });
    }
    let lowercase = trimmed.to_ascii_lowercase();
    for forbidden in FORBIDDEN_MARKUP {
        if lowercase.contains(forbidden) {
            return Err(ParseErrorKind::Malformed {
                expected: "narrative without active content (script, style, form, or external references)",
            });
        }
    }
    if contains_event_handler(&lowercase) {
        return Err(ParseErrorKind::Malformed {
            expected: "narrative without `on*` event handler attributes",
        });
    }
    // txt-2: the narrative must say something.
    if !has_text_content(trimmed) {
        return Err(ParseErrorKind::Empty);
    }
    Ok(())
}

/// Detect `on…=` attributes (`onclick=`, `onerror=`, …) in already-lowercased
/// markup, allowing whitespace around the `=`.
fn contains_event_handler(lowercase: &str) -> bool {
    let bytes = lowercase.as_bytes();
    for (index, window) in bytes.windows(3).enumerate() {
        let preceded_by_space = index > 0 && (bytes[index - 1] as char).is_whitespace();
        if !preceded_by_space || window[0] != b'o' || window[1] != b'n' {
            continue;
        }
        let mut cursor = index + 2;
        while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_alphabetic() {
            cursor += 1;
        }
        if cursor == index + 2 {
            continue;
        }
        while cursor < bytes.len() && (bytes[cursor] as char).is_whitespace() {
            cursor += 1;
        }
        if cursor < bytes.len() && bytes[cursor] == b'=' {
            return true;
        }
    }
    false
}

/// Whether any non-whitespace text exists outside of tags.
fn has_text_content(markup: &str) -> bool {
    let mut inside_tag = false;
    for character in markup.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            c if !inside_tag && !c.is_whitespace() => return true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_rejects_underscore_and_overlong_values() {
        assert!(Id::new("abc-123.4").is_ok());
        assert!(Id::new("org_1234").is_err());
        assert!(Id::new("a".repeat(64)).is_ok());
        assert!(Id::new("a".repeat(65)).is_err());
    }

    #[test]
    fn code_rejects_padding_whitespace() {
        assert!(Code::new("final").is_ok());
        assert!(Code::new("in progress").is_ok());
        assert!(Code::new(" final").is_err());
        assert!(Code::new("final ").is_err());
        assert!(Code::new("in  progress").is_err());
    }

    #[test]
    fn string_rejects_blank_and_control_characters() {
        assert!(FhirString::new("Jane").is_ok());
        assert!(FhirString::new("   ").is_err());
        assert!(FhirString::new("bad\u{0}value").is_err());
        assert!(FhirString::new("line\nbreak").is_ok());
    }

    #[test]
    fn canonical_splits_version_and_fragment() {
        let canonical = Canonical::new("http://example.org/StructureDefinition/x|1.2.0#frag")
            .expect("valid canonical");
        assert_eq!(canonical.url(), "http://example.org/StructureDefinition/x");
        assert_eq!(canonical.version(), Some("1.2.0"));
        assert_eq!(canonical.fragment(), Some("frag"));
    }

    #[test]
    fn oid_and_uuid_shapes() {
        assert!(Oid::new("urn:oid:2.16.840.1.113883.4.1").is_ok());
        assert!(Oid::new("2.16.840.1").is_err());
        assert!(Oid::new("urn:oid:2.016.840").is_err());
        assert!(Uuid::new("urn:uuid:c757873d-ec9a-4326-a141-556f43239520").is_ok());
        assert!(Uuid::new("urn:uuid:c757873d-ec9a-4326-a141-556f4323952").is_err());
    }

    #[test]
    fn base64_padding_rules_and_decoded_length() {
        let value = Base64Binary::new("aGVsbG8=").expect("valid base64");
        assert_eq!(value.decoded_len(), 5);
        assert!(Base64Binary::new("aGVsbG8").is_err());
        assert!(Base64Binary::new("aGVs=bG8=").is_err());
    }

    #[test]
    fn xhtml_rejects_active_content() {
        let ok = Xhtml::new("<div xmlns=\"http://www.w3.org/1999/xhtml\"><p>Jane Doe</p></div>");
        assert!(ok.is_ok());

        let script = Xhtml::new(
            "<div xmlns=\"http://www.w3.org/1999/xhtml\"><script>steal()</script></div>",
        );
        assert!(script.is_err());

        let handler = Xhtml::new(
            "<div xmlns=\"http://www.w3.org/1999/xhtml\"><p onclick=\"x()\">hi</p></div>",
        );
        assert!(handler.is_err());

        let no_namespace = Xhtml::new("<div><p>Jane Doe</p></div>");
        assert!(no_namespace.is_err());

        let empty = Xhtml::new("<div xmlns=\"http://www.w3.org/1999/xhtml\"><p></p></div>");
        assert!(empty.is_err());
    }
}
