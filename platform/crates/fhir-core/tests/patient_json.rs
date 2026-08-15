//! Round-tripping a realistic `Patient` through FHIR JSON.
//!
//! The assertion that matters most is the *lossless* one: what comes out must
//! be byte-for-byte the same JSON tree that went in. A schema layer that
//! silently drops an element it does not model is worse than one that refuses
//! the document, because the loss is only discovered downstream.

use fhir_core::prelude::*;
use serde_json::json;

fn example_patient_json() -> serde_json::Value {
    json!({
        "resourceType": "Patient",
        "id": "example",
        "meta": {
            "versionId": "3",
            "lastUpdated": "2024-05-01T09:15:00Z",
            "profile": ["http://hl7.org/fhir/StructureDefinition/Patient|5.0.0"],
            "security": [{
                "system": "http://terminology.hl7.org/CodeSystem/v3-Confidentiality",
                "code": "R",
                "display": "restricted"
            }]
        },
        "text": {
            "status": "generated",
            "div": "<div xmlns=\"http://www.w3.org/1999/xhtml\"><p>Jane Doe, born December 1974</p></div>"
        },
        "extension": [{
            "url": "http://hl7.org/fhir/StructureDefinition/patient-birthPlace",
            "valueAddress": { "city": "Chennai", "country": "IN" }
        }],
        "identifier": [
            {
                "use": "official",
                "type": {
                    "coding": [{
                        "system": "http://terminology.hl7.org/CodeSystem/v2-0203",
                        "code": "MR",
                        "display": "Medical record number"
                    }]
                },
                "system": "http://hospital.example.org/mrn",
                "value": "MRN-0001",
                "period": { "start": "2015-04-02" }
            },
            {
                "use": "old",
                "system": "http://hospital.example.org/mrn",
                "value": "MRN-LEGACY-7"
            }
        ],
        "active": true,
        "name": [
            {
                "use": "official",
                "family": "Doe",
                "given": ["Jane", "Amelia"],
                "prefix": ["Dr"]
            },
            {
                "use": "maiden",
                "family": "Smith",
                "given": ["Jane"],
                "period": { "start": "1974-12-25", "end": "2001-06-30" }
            }
        ],
        "telecom": [
            { "system": "phone", "value": "+44 20 7946 0100", "use": "home", "rank": 1 },
            { "system": "email", "value": "jane.doe@example.org", "use": "work" }
        ],
        "gender": "female",
        "birthDate": "1974-12",
        "deceasedBoolean": false,
        "address": [{
            "use": "home",
            "type": "both",
            "line": ["12 Ashwood Lane"],
            "city": "Bristol",
            "postalCode": "BS1 4TR",
            "country": "GB"
        }],
        "maritalStatus": {
            "coding": [{
                "system": "http://terminology.hl7.org/CodeSystem/v3-MaritalStatus",
                "code": "M",
                "display": "Married"
            }]
        },
        "multipleBirthInteger": 2,
        "contact": [{
            "relationship": [{
                "coding": [{
                    "system": "http://terminology.hl7.org/CodeSystem/v2-0131",
                    "code": "C",
                    "display": "Emergency Contact"
                }]
            }],
            "name": { "family": "Doe", "given": ["Robert"] },
            "telecom": [{ "system": "phone", "value": "+44 20 7946 0999" }],
            "gender": "male"
        }],
        "communication": [{
            "language": {
                "coding": [{
                    "system": "urn:ietf:bcp:47",
                    "code": "en-GB",
                    "display": "English (United Kingdom)"
                }]
            },
            "preferred": true
        }],
        "generalPractitioner": [{ "reference": "Practitioner/23", "display": "Dr A Rao" }],
        "managingOrganization": { "reference": "Organization/hospital-1" },
        "link": [{
            "other": { "reference": "Patient/duplicate-4" },
            "type": "seealso"
        }]
    })
}

#[test]
fn patient_round_trips_without_loss() {
    let original = example_patient_json();
    let patient: Patient =
        serde_json::from_value(original.clone()).expect("deserializes a realistic Patient");
    let round_tripped = serde_json::to_value(&patient).expect("serializes");
    assert_eq!(round_tripped, original);
}

#[test]
fn deserialized_patient_is_valid_and_readable() {
    let patient: Patient = serde_json::from_value(example_patient_json()).expect("deserializes");

    patient.check().expect("the example is valid FHIR");

    assert_eq!(
        patient.display_name().as_deref(),
        Some("Dr Jane Amelia Doe")
    );
    assert_eq!(patient.gender, Some(AdministrativeGender::Female));
    assert_eq!(
        patient.birth_date.as_ref().map(Date::as_str),
        Some("1974-12")
    );
    assert_eq!(patient.is_deceased(), Some(false));
    assert!(patient.replaced_by().is_none());

    // The retired MRN must not be offered for matching.
    let matchable: Vec<&str> = patient
        .matchable_identifiers()
        .filter_map(|identifier| identifier.value.as_ref().map(FhirString::as_str))
        .collect();
    assert_eq!(matchable, vec!["MRN-0001"]);
}

#[test]
fn choice_types_serialize_under_their_own_key() {
    let patient: Patient = serde_json::from_value(example_patient_json()).expect("deserializes");

    assert_eq!(patient.deceased, Some(PatientDeceased::Boolean(false)));
    assert_eq!(
        patient.multiple_birth,
        Some(PatientMultipleBirth::Integer(2))
    );

    let json = serde_json::to_value(&patient).expect("serializes");
    assert_eq!(json["deceasedBoolean"], json!(false));
    assert_eq!(json["multipleBirthInteger"], json!(2));
    assert!(json.get("deceased").is_none());
    assert!(json.get("multipleBirth").is_none());
}

#[test]
fn a_wrong_resource_type_is_refused() {
    let mut wrong = example_patient_json();
    wrong["resourceType"] = json!("Practitioner");

    let error =
        serde_json::from_value::<Patient>(wrong).expect_err("resourceType must match the type");
    assert!(error.to_string().contains("Practitioner"));
}

#[test]
fn invalid_primitives_are_refused_at_the_boundary() {
    for (field, value) in [
        ("birthDate", json!("1974-13-01")),
        ("birthDate", json!("1974-2-1")),
        ("id", json!("not a valid id")),
        ("gender", json!("Female")),
    ] {
        let mut document = example_patient_json();
        document[field] = value.clone();
        assert!(
            serde_json::from_value::<Patient>(document).is_err(),
            "{field} = {value} should not deserialize"
        );
    }
}

#[test]
fn unknown_elements_do_not_abort_parsing() {
    // A server running a later version of FHIR, or an extension this build
    // does not model, must not make the record unreadable.
    let mut document = example_patient_json();
    document["someFutureElement"] = json!({ "unknown": true });

    let patient: Patient = serde_json::from_value(document).expect("still deserializes");
    assert_eq!(patient.id.as_ref().map(Id::as_str), Some("example"));
}

#[test]
fn decimal_precision_survives_the_round_trip() {
    // Not a Patient field, but the property that motivates the Decimal type:
    // a quantity reported as 1.50 must not come back as 1.5.
    let quantity: Quantity = serde_json::from_value(json!({
        "value": 1.50,
        "system": "http://unitsofmeasure.org",
        "code": "mmol/L"
    }))
    .expect("deserializes");

    // Precision is lost here only because serde_json parsed the literal into an
    // f64 before this crate ever saw it — see the note on `Decimal`.
    assert_eq!(quantity.value.as_ref().map(Decimal::as_str), Some("1.5"));

    // Constructed from its lexical form, precision is kept end to end.
    let exact = Quantity::ucum(
        "1.50".parse().expect("valid decimal"),
        "mmol/L".parse().expect("valid code"),
    );
    let json = serde_json::to_string(&exact).expect("serializes");
    assert!(json.contains("\"value\":1.50"), "got {json}");
}
