//! Structural validation: the [`Validate`] trait, the [`Validator`] walker that
//! tracks where it is in the tree, and [`Validated<T>`] — the type that proves a
//! value has been checked.
//!
//! # Why a separate pass at all
//!
//! Primitive types validate themselves at construction (parse, don't validate),
//! so by the time you hold a [`crate::primitive::Date`] it *is* a date. What a
//! constructor cannot check is anything involving more than one field:
//!
//! * cardinality — `Patient.link.other` is 1..1, but the struct field is only
//!   non-optional if the Rust type says so, and choice/backbone shapes make that
//!   awkward to enforce everywhere;
//! * co-occurrence invariants — `pat-1`, `cpt-2`, `att-1`, `ref-1`;
//! * cross-field ordering — `per-1` (`Period.start <= Period.end`);
//! * binding checks that depend on a sibling field — `qty-3`.
//!
//! Those are what this module walks the tree to check, collecting *every*
//! finding rather than stopping at the first, because an API returning an
//! `OperationOutcome` wants the whole list.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{ElementPath, Issue, IssueCode, Severity, ValidationError};

/// A type that can be checked for structural validity.
///
/// Implementations should report *every* problem they find rather than
/// returning early, and should recurse into children through
/// [`Validator::field`] so paths stay accurate.
pub trait Validate {
    /// Check `self`, recording findings on `validator`.
    fn validate(&self, validator: &mut Validator);
}

/// Tree walker that accumulates [`Issue`]s together with their location.
///
/// The walker owns a mutable path stack: `field` and `index` push a segment,
/// run the child's validation, then pop. Reporting methods snapshot the current
/// path, so an issue always carries the location it was found at.
#[derive(Debug)]
pub struct Validator {
    path: ElementPath,
    issues: Vec<Issue>,
}

impl Validator {
    /// Start a walk rooted at `root` — normally a resource type name.
    pub fn new(root: &'static str) -> Self {
        Self {
            path: ElementPath::root(root),
            issues: Vec::new(),
        }
    }

    /// The location the walker is currently at.
    pub fn path(&self) -> &ElementPath {
        &self.path
    }

    /// Descend into a named child element and validate it.
    pub fn field<T: Validate + ?Sized>(&mut self, name: &'static str, value: &T) {
        self.path.push_field(name);
        value.validate(self);
        self.path.pop();
    }

    /// Descend into a named child element and run arbitrary checks there.
    pub fn enter(&mut self, name: &'static str, f: impl FnOnce(&mut Self)) {
        self.path.push_field(name);
        f(self);
        self.path.pop();
    }

    /// Descend into one position of a repeating element.
    pub fn enter_index(&mut self, index: usize, f: impl FnOnce(&mut Self)) {
        self.path.push_index(index);
        f(self);
        self.path.pop();
    }

    /// Record an issue at the current path.
    pub fn issue(&mut self, severity: Severity, code: IssueCode, message: impl Into<String>) {
        self.issues.push(Issue {
            severity,
            code,
            path: self.path.clone(),
            message: message.into(),
            key: None,
        });
    }

    /// Record an error at the current path.
    pub fn error(&mut self, code: IssueCode, message: impl Into<String>) {
        self.issue(Severity::Error, code, message);
    }

    /// Record an error at a named child of the current path.
    pub fn error_at(&mut self, field: &'static str, code: IssueCode, message: impl Into<String>) {
        self.issues.push(Issue {
            severity: Severity::Error,
            code,
            path: self.path.child(field),
            message: message.into(),
            key: None,
        });
    }

    /// Record a warning at the current path.
    pub fn warn(&mut self, code: IssueCode, message: impl Into<String>) {
        self.issue(Severity::Warning, code, message);
    }

    /// Assert a named specification invariant. `holds == false` produces an
    /// error tagged with the invariant key, the way a FHIR validator reports it.
    pub fn invariant(&mut self, key: &'static str, holds: bool, human: &str) {
        if !holds {
            self.issues.push(Issue {
                severity: Severity::Error,
                code: IssueCode::Invariant,
                path: self.path.clone(),
                message: human.to_owned(),
                key: Some(key),
            });
        }
    }

    /// Assert a named invariant as a warning rather than an error — used for
    /// the `SHOULD` rules (`dom-6`) that must not block persistence.
    pub fn invariant_warning(&mut self, key: &'static str, holds: bool, human: &str) {
        if !holds {
            self.issues.push(Issue {
                severity: Severity::Warning,
                code: IssueCode::Invariant,
                path: self.path.clone(),
                message: human.to_owned(),
                key: Some(key),
            });
        }
    }

    /// Assert that a required (`1..1` or `1..*`) child element is present.
    pub fn required(&mut self, field: &'static str, present: bool) {
        if !present {
            self.issues.push(Issue {
                severity: Severity::Error,
                code: IssueCode::Required,
                path: self.path.child(field),
                message: format!("{field} is required (minimum cardinality 1)"),
                key: None,
            });
        }
    }

    /// Finish the walk and take everything that was found.
    pub fn into_report(self) -> ValidationReport {
        ValidationReport {
            issues: self.issues,
        }
    }
}

/// The full result of a validation run: errors *and* warnings.
///
/// Warnings are worth keeping — `dom-6` (a resource should carry a human
/// narrative) does not make a resource invalid, but an ingestion pipeline may
/// well want to count how often it fires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    issues: Vec<Issue>,
}

impl ValidationReport {
    /// Everything that was found.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Only the findings at [`Severity::Error`] or worse.
    pub fn errors(&self) -> impl Iterator<Item = &Issue> {
        self.issues.iter().filter(|i| i.severity >= Severity::Error)
    }

    /// Only the findings below [`Severity::Error`].
    pub fn warnings(&self) -> impl Iterator<Item = &Issue> {
        self.issues.iter().filter(|i| i.severity < Severity::Error)
    }

    /// Whether the run found nothing that invalidates the resource.
    pub fn is_valid(&self) -> bool {
        self.errors().next().is_none()
    }

    /// Collapse into a `Result`, discarding warnings on the success path.
    pub fn into_result(self) -> Result<(), ValidationError> {
        match ValidationError::from_issues(self.issues) {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

/// A value that has been validated, and cannot be mutated afterwards.
///
/// This is the "parse, don't validate" boundary for whole resources: a service
/// layer that takes `Validated<Patient>` cannot be handed an unchecked one, and
/// no `DerefMut` or `&mut` accessor exists to invalidate it after the fact.
/// Deserializing a `Validated<T>` validates as part of parsing, so an API
/// handler can put it directly in its request type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validated<T>(T);

impl<T: Validate> Validated<T> {
    /// Validate `value`, keeping it only if the run produced no errors.
    ///
    /// `root` names the path root used in issue locations — pass the resource
    /// type name. For resources, prefer [`crate::resource::Resource::validated`],
    /// which supplies it from the type.
    pub fn new(value: T, root: &'static str) -> Result<Self, ValidationError> {
        let mut validator = Validator::new(root);
        value.validate(&mut validator);
        validator.into_report().into_result()?;
        Ok(Self(value))
    }
}

impl<T> Validated<T> {
    /// Borrow the validated value.
    pub fn get(&self) -> &T {
        &self.0
    }

    /// Take the value back out, giving up the proof.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Validated<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Serialize> Serialize for Validated<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Validated<T>
where
    T: Deserialize<'de> + Validate + crate::resource::Resource,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = T::deserialize(deserializer)?;
        Self::new(value, T::RESOURCE_TYPE.as_str()).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// Blanket implementations for the container shapes FHIR elements are made of.
// ---------------------------------------------------------------------------

impl<T: Validate> Validate for Option<T> {
    fn validate(&self, validator: &mut Validator) {
        if let Some(value) = self {
            value.validate(validator);
        }
    }
}

impl<T: Validate> Validate for Vec<T> {
    fn validate(&self, validator: &mut Validator) {
        for (index, item) in self.iter().enumerate() {
            validator.enter_index(index, |v| item.validate(v));
        }
    }
}

impl<T: Validate + ?Sized> Validate for Box<T> {
    fn validate(&self, validator: &mut Validator) {
        (**self).validate(validator);
    }
}

impl Validate for bool {
    fn validate(&self, _validator: &mut Validator) {}
}

impl Validate for i32 {
    fn validate(&self, _validator: &mut Validator) {}
}

impl Validate for String {
    fn validate(&self, _validator: &mut Validator) {}
}
