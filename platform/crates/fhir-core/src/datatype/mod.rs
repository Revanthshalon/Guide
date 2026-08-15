//! The general-purpose complex datatypes: the vocabulary every FHIR resource
//! is assembled from.
//!
//! Each type here is a plain struct with the FHIR element names, plus:
//!
//! * `id` and `extension` fields, exposed through the
//!   [`Element`](crate::element::Element) trait;
//! * a [`Validate`](crate::validate::Validate) implementation carrying that
//!   type's specification invariants (`per-1`, `cpt-2`, `att-1`, `ref-1`, …);
//! * constructors for the shapes that are actually common, so that the 90%
//!   case is one call rather than a twelve-field struct literal.
//!
//! The one type that is not a plain struct is [`Reference<T>`], which is
//! generic over what it may point at.

mod address;
mod annotation;
mod attachment;
mod coding;
mod contact_point;
mod human_name;
mod identifier;
mod meta;
mod period;
mod quantity;
mod reference;

pub use address::Address;
pub use annotation::{Annotation, AnnotationAuthor, AnnotationAuthorTarget};
pub use attachment::Attachment;
pub use coding::{CodeableConcept, CodeableReference, Coding};
pub use contact_point::ContactPoint;
pub use human_name::HumanName;
pub use identifier::Identifier;
pub use meta::{Meta, Narrative};
pub use period::Period;
pub use quantity::{Quantity, Range, Ratio};
pub use reference::Reference;
