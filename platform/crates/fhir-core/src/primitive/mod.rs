//! FHIR primitive types as validating newtypes.
//!
//! Every FHIR primitive here follows the same contract: **you cannot hold one
//! that is invalid**. Construction goes through `new` / `FromStr` / `TryFrom`,
//! all of which return [`ParseError`](crate::error::ParseError), and `serde`
//! deserialization routes through the same constructor, so a value that arrives
//! over the wire is validated exactly like one built in code.
//!
//! ## Which primitives are newtypes, and which are not
//!
//! A newtype earns its place when it either (a) constrains the value beyond
//! what the Rust type says, or (b) prevents a mix-up between two values with
//! the same representation. `boolean` and `integer` do neither, so they are
//! plain [`bool`] and [`i32`] behind the aliases [`Boolean`] and [`Integer`].
//! Everything else — even `string` — is a newtype: FHIR's `string` forbids
//! whitespace-only values and caps length at 1 MiB, which `String` does not.
//!
//! | FHIR type | Rust type | What construction enforces |
//! | --- | --- | --- |
//! | `boolean` | [`Boolean`] (= `bool`) | nothing to enforce |
//! | `integer` | [`Integer`] (= `i32`) | 32-bit range comes free |
//! | `integer64` | [`Integer64`] | 64-bit range; **serialized as a JSON string** |
//! | `positiveInt` | [`PositiveInt`] | `1..=2147483647` |
//! | `unsignedInt` | [`UnsignedInt`] | `0..=2147483647` |
//! | `decimal` | [`Decimal`] | lexical form; **precision preserved** |
//! | `string` | [`FhirString`] | non-blank, ≤ 1 MiB, no control characters |
//! | `code` | [`Code`] | no leading/trailing/double whitespace |
//! | `id` | [`Id`] | `[A-Za-z0-9\-\.]{1,64}` |
//! | `uri` | [`Uri`] | non-empty, no whitespace |
//! | `url` | [`Url`] | as `uri` |
//! | `canonical` | [`Canonical`] | as `uri`, with `\|version` and `#fragment` parts split out |
//! | `oid` | [`Oid`] | `urn:oid:` + dotted decimal arcs |
//! | `uuid` | [`Uuid`] | `urn:uuid:` + 8-4-4-4-12 hex |
//! | `markdown` | [`Markdown`] | as `string` |
//! | `base64Binary` | [`Base64Binary`] | alphabet, length, padding position |
//! | `xhtml` | [`Xhtml`] | `<div>` root, XHTML namespace, no active content |
//! | `date` | [`Date`] | `YYYY`, `YYYY-MM`, or `YYYY-MM-DD`, calendar-checked |
//! | `dateTime` | [`DateTime`] | as `date`, plus time with a **mandatory** offset |
//! | `instant` | [`Instant`] | full precision, offset required |
//! | `time` | [`Time`] | `hh:mm:ss[.sss]`, no offset |
//!
//! ## A note on primitive extensions
//!
//! FHIR JSON allows an extension on a primitive to travel in a sibling key:
//! `"birthDate": "1970-01-01"` alongside `"_birthDate": {"extension": [...]}`.
//! This crate does not model that yet; see `DESIGN.md` in the crate root for the
//! planned `Primitive<P>` wrapper and why it was staged rather than rushed.

mod numeric;
mod string;
mod temporal;

pub use numeric::{Decimal, Integer64, PositiveInt, UnsignedInt};
pub use string::{
    Base64Binary, Canonical, Code, FhirString, Id, Markdown, Oid, Uri, Url, Uuid, Xhtml,
};
pub use temporal::{Date, DateTime, Instant, Precision, Time};

/// FHIR `boolean`. Nothing to constrain, so it is not a newtype.
pub type Boolean = bool;

/// FHIR `integer`: a signed 32-bit integer. The Rust type is the constraint.
pub type Integer = i32;

/// Common behaviour of every FHIR primitive type.
///
/// The associated constant is what makes generic error reporting possible: a
/// helper that fails while handling `P: PrimitiveType` can name the FHIR type
/// in its message without the caller passing a string along.
pub trait PrimitiveType: Sized {
    /// The FHIR type name, e.g. `"positiveInt"`.
    const FHIR_TYPE: &'static str;
}

/// A FHIR primitive whose lexical form is its value.
///
/// Implemented by every string-shaped primitive, so code that only needs "the
/// text of this thing" can be generic over `S: StringPrimitive` instead of
/// being written once per type.
pub trait StringPrimitive: PrimitiveType + std::str::FromStr {
    /// The value's lexical form.
    fn as_str(&self) -> &str;
}

impl PrimitiveType for bool {
    const FHIR_TYPE: &'static str = "boolean";
}

impl PrimitiveType for i32 {
    const FHIR_TYPE: &'static str = "integer";
}
