//! The `Patient` resource.
//!
//! # What this resource is for, and what it is not
//!
//! `Patient` is demographic and administrative data about an individual
//! receiving care. It is deliberately *thin*: no clinical content, no
//! encounters, no coverage. Everything else points at it.
//!
//! Three modelling traps are worth stating before the field list, because each
//! one causes real harm rather than mere untidiness:
//!
//! * **`gender` is administrative.** It is not clinical sex and not gender
//!   identity — see [`AdministrativeGender`](crate::codes::AdministrativeGender).
//! * **`active = false` does not mean deceased**, and `deceased` does not mean
//!   inactive. `active` says whether the *record* should be used; `deceased`
//!   says something about the person. A patient who transferred to another
//!   practice is inactive and alive.
//! * **`link` is directional.** `replaced-by` retires *this* record in favour
//!   of the target. Reversing it points every consumer at the dead record; see
//!   [`LinkType`](crate::codes::LinkType).

use serde::{Deserialize, Serialize};

use crate::codes::{AdministrativeGender, LinkType};
use crate::datatype::{
    Address, Attachment, CodeableConcept, ContactPoint, HumanName, Identifier, Meta, Narrative,
    Period, Reference,
};
use crate::element::{Extension, impl_backbone_element};
use crate::error::IssueCode;
use crate::primitive::{Boolean, Code, Date, DateTime, FhirString, Id, Integer, Uri};
use crate::resource::{
    ContainedResource, DomainResource, Resource, ResourceMarker, ResourceTag, ResourceType,
    marker::{Organization, Practitioner, PractitionerRole, RelatedPerson},
};
use crate::validate::{Validate, Validator};

/// Reference targets permitted for `Patient.generalPractitioner`.
pub type GeneralPractitionerReference = Reference<(Organization, Practitioner, PractitionerRole)>;

/// Reference targets permitted for `Patient.link.other`.
pub type PatientLinkReference = Reference<(Patient, RelatedPerson)>;

/// `Patient.deceased[x]`.
///
/// The boolean and the dateTime are not interchangeable: `true` records *that*
/// the patient died, a dateTime records *when*. A system that flattens both to
/// a boolean loses the date; one that flattens to a date invents one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatientDeceased {
    /// `deceasedBoolean` — known to have died, date unknown.
    #[serde(rename = "deceasedBoolean")]
    Boolean(Boolean),
    /// `deceasedDateTime` — died at this (possibly partial) date and time.
    #[serde(rename = "deceasedDateTime")]
    DateTime(DateTime),
}

/// `Patient.multipleBirth[x]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatientMultipleBirth {
    /// `multipleBirthBoolean` — part of a multiple birth, order unknown.
    #[serde(rename = "multipleBirthBoolean")]
    Boolean(Boolean),
    /// `multipleBirthInteger` — the birth order within the multiple birth.
    #[serde(rename = "multipleBirthInteger")]
    Integer(Integer),
}

/// FHIR `Patient`: demographics and administrative information about an
/// individual receiving care.
///
/// Every element is optional, as in the specification itself — a `Patient` with
/// nothing but a `resourceType` is valid FHIR, because a record may legitimately
/// exist before anything is known about the person. `Default` therefore gives an
/// empty patient, and [`Patient::builder`] fills it in.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Patient {
    /// Discriminator; always `"Patient"`.
    pub resource_type: ResourceTag<Patient>,

    // ----- Resource -----
    /// Logical id of this resource, assigned by the server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,

    /// Infrastructure metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,

    /// Rules a consumer must understand to process this resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implicit_rules: Option<Uri>,

    /// Base language of the resource content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Code>,

    // ----- DomainResource -----
    /// Human-readable summary of the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<Narrative>,

    /// Resources with no independent existence, inlined here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contained: Vec<ContainedResource>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Extensions that change the meaning of the resource.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifier_extension: Vec<Extension>,

    // ----- Patient -----
    /// Business identifiers for this patient: MRN, national number, and so on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identifier: Vec<Identifier>,

    /// Whether this patient *record* is in active use. Not a liveness flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<Boolean>,

    /// Names associated with the patient.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub name: Vec<HumanName>,

    /// Contact details for the patient.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub telecom: Vec<ContactPoint>,

    /// Gender for administrative purposes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<AdministrativeGender>,

    /// Date of birth, at whatever precision is actually known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<Date>,

    /// Whether the patient is deceased, and when.
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub deceased: Option<PatientDeceased>,

    /// Addresses for the patient.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub address: Vec<Address>,

    /// Marital (civil) status. Extensibly bound, so a `CodeableConcept`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marital_status: Option<CodeableConcept>,

    /// Whether the patient is part of a multiple birth.
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub multiple_birth: Option<PatientMultipleBirth>,

    /// Image of the patient.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub photo: Vec<Attachment>,

    /// A contact party — guardian, partner, friend — for the patient.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contact: Vec<PatientContact>,

    /// Languages the patient can communicate in about their health.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub communication: Vec<PatientCommunication>,

    /// The organization or practitioner nominally responsible for care.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub general_practitioner: Vec<GeneralPractitionerReference>,

    /// Organization that is the custodian of the patient record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managing_organization: Option<Reference<Organization>>,

    /// Links to other patient resources concerning the same person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub link: Vec<PatientLink>,
}

impl Patient {
    /// Start building a patient.
    pub fn builder() -> PatientBuilder {
        PatientBuilder::default()
    }

    /// Whether the patient is recorded as deceased, by either form of
    /// `deceased[x]`.
    ///
    /// Note that `Some(false)` and `None` are different: the first is a
    /// positive statement that the patient is alive, the second says nothing.
    pub fn is_deceased(&self) -> Option<bool> {
        match &self.deceased {
            None => None,
            Some(PatientDeceased::Boolean(value)) => Some(*value),
            Some(PatientDeceased::DateTime(_)) => Some(true),
        }
    }

    /// The record this one has been replaced by, if it has been superseded.
    ///
    /// Anything reading a patient record should follow this before using the
    /// data: a `replaced-by` record is a tombstone.
    pub fn replaced_by(&self) -> Option<&PatientLinkReference> {
        self.link
            .iter()
            .find(|link| link.type_ == LinkType::ReplacedBy)
            .map(|link| &link.other)
    }

    /// The identifiers that may safely be used for patient matching:
    /// system-qualified and not retired.
    pub fn matchable_identifiers(&self) -> impl Iterator<Item = &Identifier> {
        self.identifier
            .iter()
            .filter(|identifier| identifier.is_usable_for_matching())
    }

    /// The name to display: the first current official name, else the first
    /// current name, else the first name of any kind.
    pub fn display_name(&self) -> Option<String> {
        use crate::codes::NameUse;

        let official = self
            .name
            .iter()
            .find(|name| name.use_ == Some(NameUse::Official) && name.is_current());
        let current = self.name.iter().find(|name| name.is_current());
        official
            .or(current)
            .or(self.name.first())
            .map(HumanName::display)
    }
}

impl ResourceMarker for Patient {
    const RESOURCE_TYPE: ResourceType = ResourceType::Patient;
}

impl crate::resource::ReferenceTarget for Patient {
    fn accepts(resource_type: ResourceType) -> bool {
        resource_type == ResourceType::Patient
    }

    fn expected(into: &mut Vec<ResourceType>) {
        into.push(ResourceType::Patient);
    }
}

impl Resource for Patient {
    fn id(&self) -> Option<&Id> {
        self.id.as_ref()
    }

    fn meta(&self) -> Option<&Meta> {
        self.meta.as_ref()
    }

    fn implicit_rules(&self) -> Option<&Uri> {
        self.implicit_rules.as_ref()
    }

    fn language(&self) -> Option<&Code> {
        self.language.as_ref()
    }
}

impl DomainResource for Patient {
    fn text(&self) -> Option<&Narrative> {
        self.text.as_ref()
    }

    fn contained(&self) -> &[ContainedResource] {
        &self.contained
    }

    fn extension(&self) -> &[Extension] {
        &self.extension
    }

    fn modifier_extension(&self) -> &[Extension] {
        &self.modifier_extension
    }
}

// `Patient` implements `DomainResource`, not `Element`: a resource's `id` is
// the FHIR `id` type (URL-safe, 64 characters), whereas `Element.id` is a
// plain string. Sharing one accessor would mean widening `Resource::id` to
// `string` and losing that constraint.

impl Validate for Patient {
    fn validate(&self, validator: &mut Validator) {
        // dom-6: a resource should have narrative for robust management. A
        // warning, not an error — refusing to store a patient because it has no
        // human summary would be worse than the problem.
        validator.invariant_warning(
            "dom-6",
            self.text.is_some(),
            "A resource should have narrative for robust management",
        );

        // Platform rule, not a FHIR invariant: a death that precedes the birth
        // is always a data error, and the comparison is only made when the
        // recorded precisions make it definite.
        if let (Some(birth), Some(PatientDeceased::DateTime(died))) =
            (&self.birth_date, &self.deceased)
            && died.chronological_cmp(&DateTime::from(birth.clone()))
                == Some(std::cmp::Ordering::Less)
        {
            validator.error_at(
                "deceasedDateTime",
                IssueCode::BusinessRule,
                "deceased date precedes the recorded birth date",
            );
        }

        validator.field("meta", &self.meta);
        validator.field("text", &self.text);
        validator.field("contained", &self.contained);
        validator.field("extension", &self.extension);
        validator.field("modifierExtension", &self.modifier_extension);
        validator.field("identifier", &self.identifier);
        validator.field("name", &self.name);
        validator.field("telecom", &self.telecom);
        validator.field("address", &self.address);
        validator.field("maritalStatus", &self.marital_status);
        validator.field("photo", &self.photo);
        validator.field("contact", &self.contact);
        validator.field("communication", &self.communication);
        validator.field("generalPractitioner", &self.general_practitioner);
        validator.field("managingOrganization", &self.managing_organization);
        validator.field("link", &self.link);
    }
}

/// `Patient.contact`: a party who may be contacted about the patient.
///
/// Invariant `pat-1` requires that a contact carry *something* usable — a name,
/// a telecom, an address, or an organization. A contact consisting only of a
/// relationship code says "there is a next of kin" without saying how to reach
/// them, which is worse than recording nothing because it looks complete.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientContact {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Extensions that change the meaning of this contact.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifier_extension: Vec<Extension>,

    /// The kind of relationship — guardian, emergency contact, and so on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationship: Vec<CodeableConcept>,

    /// A name associated with the contact person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<HumanName>,

    /// Contact details for the person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub telecom: Vec<ContactPoint>,

    /// Address for the contact person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<Address>,

    /// Administrative gender of the contact person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<AdministrativeGender>,

    /// Organization that is associated with the contact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<Reference<Organization>>,

    /// Period during which this contact should be used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<Period>,
}

impl_backbone_element!(PatientContact);

impl Validate for PatientContact {
    fn validate(&self, validator: &mut Validator) {
        // pat-1: SHALL at least contain a contact's details or a reference to
        // an organization.
        let reachable = self.name.is_some()
            || !self.telecom.is_empty()
            || self.address.is_some()
            || self.organization.is_some();
        validator.invariant(
            "pat-1",
            reachable,
            "SHALL at least contain a contact's details or a reference to an organization",
        );

        validator.field("extension", &self.extension);
        validator.field("modifierExtension", &self.modifier_extension);
        validator.field("relationship", &self.relationship);
        validator.field("name", &self.name);
        validator.field("telecom", &self.telecom);
        validator.field("address", &self.address);
        validator.field("organization", &self.organization);
        validator.field("period", &self.period);
    }
}

/// `Patient.communication`: a language the patient may communicate in.
///
/// `preferred` marks the language for *health communication*, which is not
/// necessarily the patient's first language. Only one entry should be
/// preferred; more than one leaves an interpreter booking system to guess.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientCommunication {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Extensions that change the meaning of this element.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifier_extension: Vec<Extension>,

    /// The language, ideally as a BCP-47 code. Required (1..1).
    pub language: CodeableConcept,

    /// Whether this is the preferred language for health communication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred: Option<Boolean>,
}

impl_backbone_element!(PatientCommunication);

impl Validate for PatientCommunication {
    fn validate(&self, validator: &mut Validator) {
        validator.field("extension", &self.extension);
        validator.field("modifierExtension", &self.modifier_extension);
        validator.field("language", &self.language);
    }
}

/// `Patient.link`: a link to another patient record about the same person.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientLink {
    /// Element id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<FhirString>,

    /// Extensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension: Vec<Extension>,

    /// Extensions that change the meaning of this link.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifier_extension: Vec<Extension>,

    /// The other patient or related person this link refers to. Required (1..1).
    pub other: PatientLinkReference,

    /// The direction and nature of the link. Required (1..1).
    #[serde(rename = "type")]
    pub type_: LinkType,
}

impl PatientLink {
    /// A link of the given type to another record.
    pub fn new(other: PatientLinkReference, type_: LinkType) -> Self {
        Self {
            id: None,
            extension: Vec::new(),
            modifier_extension: Vec::new(),
            other,
            type_,
        }
    }
}

impl_backbone_element!(PatientLink);

impl Validate for PatientLink {
    fn validate(&self, validator: &mut Validator) {
        validator.field("extension", &self.extension);
        validator.field("modifierExtension", &self.modifier_extension);
        validator.field("other", &self.other);
    }
}

/// Builder for [`Patient`].
///
/// Everything on `Patient` is optional in FHIR, so this is a convenience rather
/// than a correctness device — the correctness device is
/// [`PatientBuilder::build_validated`], which will not hand back a
/// [`Validated<Patient>`](crate::validate::Validated) unless the resource
/// passes.
#[derive(Debug, Clone, Default)]
pub struct PatientBuilder {
    patient: Patient,
}

impl PatientBuilder {
    /// Set the logical id.
    pub fn id(mut self, id: Id) -> Self {
        self.patient.id = Some(id);
        self
    }

    /// Set the resource metadata.
    pub fn meta(mut self, meta: Meta) -> Self {
        self.patient.meta = Some(meta);
        self
    }

    /// Set the human-readable narrative.
    pub fn text(mut self, text: Narrative) -> Self {
        self.patient.text = Some(text);
        self
    }

    /// Add a business identifier.
    pub fn identifier(mut self, identifier: Identifier) -> Self {
        self.patient.identifier.push(identifier);
        self
    }

    /// Set whether the record is in active use.
    pub fn active(mut self, active: bool) -> Self {
        self.patient.active = Some(active);
        self
    }

    /// Add a name.
    pub fn name(mut self, name: HumanName) -> Self {
        self.patient.name.push(name);
        self
    }

    /// Add a contact point.
    pub fn telecom(mut self, telecom: ContactPoint) -> Self {
        self.patient.telecom.push(telecom);
        self
    }

    /// Set the administrative gender.
    pub fn gender(mut self, gender: AdministrativeGender) -> Self {
        self.patient.gender = Some(gender);
        self
    }

    /// Set the birth date.
    pub fn birth_date(mut self, birth_date: Date) -> Self {
        self.patient.birth_date = Some(birth_date);
        self
    }

    /// Record that the patient is deceased.
    pub fn deceased(mut self, deceased: PatientDeceased) -> Self {
        self.patient.deceased = Some(deceased);
        self
    }

    /// Add an address.
    pub fn address(mut self, address: Address) -> Self {
        self.patient.address.push(address);
        self
    }

    /// Set the marital status.
    pub fn marital_status(mut self, marital_status: CodeableConcept) -> Self {
        self.patient.marital_status = Some(marital_status);
        self
    }

    /// Record multiple-birth information.
    pub fn multiple_birth(mut self, multiple_birth: PatientMultipleBirth) -> Self {
        self.patient.multiple_birth = Some(multiple_birth);
        self
    }

    /// Add a contact party.
    pub fn contact(mut self, contact: PatientContact) -> Self {
        self.patient.contact.push(contact);
        self
    }

    /// Add a communication preference.
    pub fn communication(mut self, communication: PatientCommunication) -> Self {
        self.patient.communication.push(communication);
        self
    }

    /// Add a general practitioner.
    pub fn general_practitioner(mut self, reference: GeneralPractitionerReference) -> Self {
        self.patient.general_practitioner.push(reference);
        self
    }

    /// Set the managing organization.
    pub fn managing_organization(mut self, reference: Reference<Organization>) -> Self {
        self.patient.managing_organization = Some(reference);
        self
    }

    /// Add a link to another patient record.
    pub fn link(mut self, link: PatientLink) -> Self {
        self.patient.link.push(link);
        self
    }

    /// Add an extension.
    pub fn extension(mut self, extension: Extension) -> Self {
        self.patient.extension.push(extension);
        self
    }

    /// Finish, without validating.
    pub fn build(self) -> Patient {
        self.patient
    }

    /// Finish and validate, returning the proof type on success.
    pub fn build_validated(
        self,
    ) -> Result<crate::validate::Validated<Patient>, crate::error::ValidationError> {
        self.patient.validated()
    }
}
