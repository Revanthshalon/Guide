//! Resource types.
//!
//! One resource is modelled so far — [`Patient`] — together with everything it
//! needs. The point of starting here is that `Patient` exercises nearly every
//! foundational mechanism: primitives with real constraints, choice types,
//! backbone elements, typed references with multi-type targets, required and
//! extensible bindings, and a resource-level invariant (`pat-1`).

mod patient;

pub use patient::{
    Patient, PatientBuilder, PatientCommunication, PatientContact, PatientDeceased, PatientLink,
    PatientMultipleBirth,
};
