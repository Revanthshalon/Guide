//! Error types for the two failure boundaries in this crate.
//!
//! The crate distinguishes two kinds of failure, and they are deliberately
//! different types:
//!
//! * [`ParseError`] — a *lexical* failure. A primitive value could not be
//!   constructed from its textual form (`"2020-13-45"` is not a date). It has
//!   no path attached because a primitive does not know where it lives.
//! * [`ValidationError`] — a *structural* failure. One or more [`Issue`]s were
//!   found while walking an element tree: a missing required field, a violated
//!   invariant, a reference pointing at the wrong resource type. Every issue
//!   carries the FHIRPath-style [`ElementPath`] where it was found, so it can
//!   be rendered straight into an `OperationOutcome`.
//!
//! The split matters operationally: a `ParseError` can only happen at the
//! system boundary (deserialization or an explicit constructor call), whereas a
//! `ValidationError` can be produced at any time from a value that is already
//! in memory.

use std::fmt;

/// Failure to construct a FHIR primitive from its lexical form.
///
/// Returned by every primitive constructor (`Id::new`, `Date::new`, …) and
/// surfaced through `serde` as a deserialization error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    type_name: &'static str,
    kind: ParseErrorKind,
}

impl ParseError {
    /// Build an error for the named FHIR type.
    pub const fn new(type_name: &'static str, kind: ParseErrorKind) -> Self {
        Self { type_name, kind }
    }

    /// The FHIR type name that rejected the value, e.g. `"positiveInt"`.
    pub const fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Why the value was rejected.
    pub const fn kind(&self) -> &ParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid FHIR `{}`: {}", self.type_name, self.kind)
    }
}

impl std::error::Error for ParseError {}

/// The specific reason a primitive value was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// The value was empty, or contained only whitespace.
    Empty,
    /// The value exceeded the type's maximum length, counted in characters.
    TooLong {
        /// Maximum number of characters the type allows.
        max: usize,
        /// Number of characters actually supplied.
        actual: usize,
    },
    /// A character not permitted by the type's regex was found.
    IllegalCharacter {
        /// Zero-based character index of the offending character.
        index: usize,
        /// The offending character.
        character: char,
    },
    /// The value did not match the type's overall shape.
    Malformed {
        /// Human-readable description of the shape that was expected.
        expected: &'static str,
    },
    /// A numeric value fell outside the range the type permits.
    OutOfRange {
        /// Smallest accepted value.
        min: i64,
        /// Largest accepted value.
        max: i64,
        /// The value supplied.
        actual: i64,
    },
    /// A code was not a member of a required-binding value set.
    UnknownCode {
        /// The code that was supplied.
        value: String,
        /// Canonical URL of the value set that was expected.
        value_set: &'static str,
    },
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("value is empty or all whitespace"),
            Self::TooLong { max, actual } => {
                write!(f, "value is {actual} characters, maximum is {max}")
            }
            Self::IllegalCharacter { index, character } => {
                write!(f, "illegal character {character:?} at position {index}")
            }
            Self::Malformed { expected } => write!(f, "expected {expected}"),
            Self::OutOfRange { min, max, actual } => {
                write!(f, "value {actual} is outside the range {min}..={max}")
            }
            Self::UnknownCode { value, value_set } => {
                write!(
                    f,
                    "code {value:?} is not in the required value set {value_set}"
                )
            }
        }
    }
}

/// How serious an [`Issue`] is. Mirrors FHIR's `issue-severity` value set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Informational only; nothing needs to change.
    Information,
    /// The resource is usable but something is questionable.
    Warning,
    /// The resource is not valid and must not be persisted as-is.
    Error,
    /// Processing cannot continue at all.
    Fatal,
}

impl Severity {
    /// The FHIR `issue-severity` code.
    pub const fn as_code(self) -> &'static str {
        match self {
            Self::Information => "information",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Fatal => "fatal",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_code())
    }
}

/// Why an [`Issue`] was raised. A subset of FHIR's `issue-type` value set,
/// limited to the codes this crate can actually produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IssueCode {
    /// A structural rule of the specification was broken.
    Structure,
    /// A required element was absent.
    Required,
    /// An element's value was unacceptable.
    Value,
    /// A formal invariant (`pat-1`, `per-1`, …) failed.
    Invariant,
    /// A code was not valid for its binding.
    CodeInvalid,
    /// A reference could not be resolved or pointed at the wrong type.
    NotFound,
    /// An organization-level rule (not part of FHIR itself) failed.
    BusinessRule,
}

impl IssueCode {
    /// The FHIR `issue-type` code.
    pub const fn as_code(self) -> &'static str {
        match self {
            Self::Structure => "structure",
            Self::Required => "required",
            Self::Value => "value",
            Self::Invariant => "invariant",
            Self::CodeInvalid => "code-invalid",
            Self::NotFound => "not-found",
            Self::BusinessRule => "business-rule",
        }
    }
}

impl fmt::Display for IssueCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_code())
    }
}

/// One step in an [`ElementPath`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    /// A named element, e.g. `name`.
    Field(&'static str),
    /// A position within a repeating element, e.g. `[0]`.
    Index(usize),
}

/// A FHIRPath-style location, such as `Patient.contact[0].telecom[1].value`.
///
/// Paths are built by the walker in [`crate::validate::Validator`] as it
/// descends, so an issue reported deep in a tree comes back with the exact
/// place a client should look.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ElementPath {
    segments: Vec<PathSegment>,
}

impl ElementPath {
    /// An empty path.
    pub const fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// A path rooted at a resource or datatype name.
    pub fn root(name: &'static str) -> Self {
        Self {
            segments: vec![PathSegment::Field(name)],
        }
    }

    /// The path's segments, outermost first.
    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }

    /// Append a named element.
    pub fn push_field(&mut self, name: &'static str) {
        self.segments.push(PathSegment::Field(name));
    }

    /// Append a position within a repeating element.
    pub fn push_index(&mut self, index: usize) {
        self.segments.push(PathSegment::Index(index));
    }

    /// Remove the last segment.
    pub fn pop(&mut self) {
        self.segments.pop();
    }

    /// This path with one more named element, leaving `self` untouched.
    pub fn child(&self, name: &'static str) -> Self {
        let mut next = self.clone();
        next.push_field(name);
        next
    }
}

impl fmt::Display for ElementPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for segment in &self.segments {
            match segment {
                PathSegment::Field(name) => {
                    if !first {
                        f.write_str(".")?;
                    }
                    f.write_str(name)?;
                }
                PathSegment::Index(index) => write!(f, "[{index}]")?,
            }
            first = false;
        }
        Ok(())
    }
}

/// A single finding produced by validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// How serious the finding is.
    pub severity: Severity,
    /// Why it was raised.
    pub code: IssueCode,
    /// Where in the tree it was found.
    pub path: ElementPath,
    /// Human-readable description.
    pub message: String,
    /// The formal invariant key (`pat-1`, `per-1`, …) when the finding comes
    /// from a named specification invariant.
    pub key: Option<&'static str>,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.path, self.message)?;
        if let Some(key) = self.key {
            write!(f, " ({key})")?;
        }
        Ok(())
    }
}

/// One or more [`Issue`]s of severity [`Severity::Error`] or worse.
///
/// Guaranteed non-empty: it is only constructible from a validation run that
/// actually found errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    issues: Vec<Issue>,
}

impl ValidationError {
    pub(crate) fn from_issues(issues: Vec<Issue>) -> Option<Self> {
        if issues.iter().any(|i| i.severity >= Severity::Error) {
            Some(Self { issues })
        } else {
            None
        }
    }

    /// Every issue found during the run, including warnings that accompanied
    /// the errors.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Only the issues that made the resource invalid.
    pub fn errors(&self) -> impl Iterator<Item = &Issue> {
        self.issues.iter().filter(|i| i.severity >= Severity::Error)
    }

    /// Render as a FHIR `OperationOutcome`, ready to return from an API.
    pub fn to_operation_outcome(&self) -> serde_json::Value {
        let issues: Vec<serde_json::Value> = self
            .issues
            .iter()
            .map(|issue| {
                serde_json::json!({
                    "severity": issue.severity.as_code(),
                    "code": issue.code.as_code(),
                    "diagnostics": issue.message,
                    "expression": [issue.path.to_string()],
                })
            })
            .collect();
        serde_json::json!({ "resourceType": "OperationOutcome", "issue": issues })
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let errors = self.errors().count();
        write!(f, "{errors} validation error(s)")?;
        for issue in self.errors() {
            write!(f, "\n  {issue}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}
