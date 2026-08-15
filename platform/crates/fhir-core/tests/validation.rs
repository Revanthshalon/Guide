//! What the structural pass catches that the type system cannot.
//!
//! Each test here is a document that *parses* — every primitive in it is
//! individually legal — and is still wrong when the elements are considered
//! together. That is precisely the boundary between the two mechanisms.

use fhir_core::error::Severity;
use fhir_core::prelude::*;
use serde_json::json;

fn minimal_patient() -> Patient {
    Patient::builder()
        .id("example".parse().expect("valid id"))
        .text(Narrative::generated(
            "<div xmlns=\"http://www.w3.org/1999/xhtml\"><p>Jane Doe</p></div>"
                .parse()
                .expect("valid xhtml"),
        ))
        .name(HumanName::new(
            "Doe".parse().expect("valid string"),
            vec!["Jane".parse().expect("valid string")],
        ))
        .build()
}

#[test]
fn a_contact_with_no_way_to_reach_it_fails_pat_1() {
    let document = json!({
        "resourceType": "Patient",
        "contact": [{
            "relationship": [{ "text": "Next of kin" }]
        }]
    });

    let patient: Patient = serde_json::from_value(document).expect("parses");
    let error = patient.check().expect_err("pat-1 must fail");

    let issue = error
        .errors()
        .find(|issue| issue.key == Some("pat-1"))
        .expect("pat-1 is reported");
    assert_eq!(issue.path.to_string(), "Patient.contact[0]");
}

#[test]
fn every_problem_is_reported_not_just_the_first() {
    let document = json!({
        "resourceType": "Patient",
        "contact": [
            { "relationship": [{ "text": "Next of kin" }] },
            { "relationship": [{ "text": "Neighbour" }] }
        ],
        "telecom": [{ "value": "+44 20 7946 0100" }],
        "identifier": [{ "value": "MRN-0001" }]
    });

    let patient: Patient = serde_json::from_value(document).expect("parses");
    let error = patient.check().expect_err("several problems");

    let paths: Vec<String> = error.errors().map(|issue| issue.path.to_string()).collect();
    assert!(paths.contains(&"Patient.contact[0]".to_owned()));
    assert!(paths.contains(&"Patient.contact[1]".to_owned()));
    assert!(paths.contains(&"Patient.telecom[0]".to_owned()));
    assert!(paths.contains(&"Patient.identifier[0].system".to_owned()));
    assert!(error.errors().count() >= 4, "got {paths:?}");
}

#[test]
fn a_period_that_ends_before_it_starts_fails_per_1() {
    let document = json!({
        "resourceType": "Patient",
        "name": [{
            "family": "Doe",
            "period": { "start": "2001-06-30", "end": "1974-12-25" }
        }]
    });

    let patient: Patient = serde_json::from_value(document).expect("parses");
    let error = patient.check().expect_err("per-1 must fail");
    assert!(
        error.errors().any(|issue| issue.key == Some("per-1")
            && issue.path.to_string() == "Patient.name[0].period")
    );
}

#[test]
fn an_indeterminate_period_comparison_is_not_a_violation() {
    // start "2020" could be any instant in 2020, and the end is inside it.
    // Reporting a violation here would reject valid data.
    let document = json!({
        "resourceType": "Patient",
        "name": [{
            "family": "Doe",
            "period": { "start": "2020", "end": "2020-06-01" }
        }]
    });

    let patient: Patient = serde_json::from_value(document).expect("parses");
    let report = patient.validation_report();
    assert!(report.errors().all(|issue| issue.key != Some("per-1")));
}

#[test]
fn a_telecom_value_with_no_system_fails_cpt_2() {
    let document = json!({
        "resourceType": "Patient",
        "telecom": [{ "value": "jane.doe@example.org" }]
    });

    let patient: Patient = serde_json::from_value(document).expect("parses");
    let error = patient.check().expect_err("cpt-2 must fail");
    assert!(error.errors().any(|issue| issue.key == Some("cpt-2")));
}

#[test]
fn a_reference_to_the_wrong_resource_type_is_caught() {
    // Parses fine — it is a well-formed relative reference. It is only wrong
    // because of where it was put.
    let document = json!({
        "resourceType": "Patient",
        "managingOrganization": { "reference": "Practitioner/23" }
    });

    let patient: Patient = serde_json::from_value(document).expect("parses");
    let error = patient.check().expect_err("wrong reference target");
    let issue = error
        .errors()
        .find(|issue| issue.path.to_string() == "Patient.managingOrganization.reference")
        .expect("reported against the reference");
    assert!(issue.message.contains("accepts Organization"), "{issue}");
}

#[test]
fn a_reference_type_that_contradicts_the_literal_is_caught() {
    let document = json!({
        "resourceType": "Patient",
        "managingOrganization": {
            "reference": "Organization/1",
            "type": "Practitioner"
        }
    });

    let patient: Patient = serde_json::from_value(document).expect("parses");
    let error = patient.check().expect_err("type contradicts reference");
    assert!(
        error
            .errors()
            .any(|issue| issue.path.to_string() == "Patient.managingOrganization.type")
    );
}

#[test]
fn a_reference_to_an_unmodelled_resource_type_is_only_a_warning() {
    let document = json!({
        "resourceType": "Patient",
        "generalPractitioner": [{ "reference": "CareTeam/7" }]
    });

    let patient: Patient = serde_json::from_value(document).expect("parses");
    let report = patient.validation_report();
    assert!(
        report.is_valid(),
        "an unknown target type must not be fatal"
    );
    assert!(
        report
            .warnings()
            .any(|issue| issue.message.contains("CareTeam"))
    );
}

#[test]
fn missing_narrative_is_a_warning_not_an_error() {
    let patient = Patient::builder().build();
    let report = patient.validation_report();

    assert!(report.is_valid());
    let warning = report
        .warnings()
        .find(|issue| issue.key == Some("dom-6"))
        .expect("dom-6 is reported");
    assert_eq!(warning.severity, Severity::Warning);
}

#[test]
fn a_death_before_birth_is_a_business_rule_failure() {
    let patient = Patient::builder()
        .birth_date("1974-12-25".parse().expect("valid date"))
        .deceased(PatientDeceased::DateTime(
            "1970-01-01T00:00:00Z".parse().expect("valid dateTime"),
        ))
        .build();

    let error = patient.check().expect_err("death precedes birth");
    assert!(
        error
            .errors()
            .any(|issue| issue.code == IssueCode::BusinessRule)
    );
}

#[test]
fn an_extension_with_both_a_value_and_children_fails_ext_1() {
    let document = json!({
        "resourceType": "Patient",
        "extension": [{
            "url": "http://example.org/StructureDefinition/thing",
            "valueBoolean": true,
            "extension": [{
                "url": "http://example.org/StructureDefinition/nested",
                "valueBoolean": false
            }]
        }]
    });

    let patient: Patient = serde_json::from_value(document).expect("parses");
    let error = patient.check().expect_err("ext-1 must fail");
    assert!(error.errors().any(|issue| issue.key == Some("ext-1")));
}

#[test]
fn contained_resources_may_not_carry_version_metadata() {
    let document = json!({
        "resourceType": "Patient",
        "contained": [{
            "resourceType": "Organization",
            "id": "org-1",
            "meta": { "versionId": "2" }
        }]
    });

    let patient: Patient = serde_json::from_value(document).expect("parses");
    let error = patient.check().expect_err("dom-4 must fail");
    let issue = error
        .errors()
        .find(|issue| issue.key == Some("dom-4"))
        .expect("dom-4 is reported");
    assert_eq!(issue.path.to_string(), "Patient.contained[0]");
}

#[test]
fn validated_is_the_proof_a_resource_passed() {
    let ok = minimal_patient().validated().expect("valid");
    assert_eq!(ok.get().id.as_ref().map(Id::as_str), Some("example"));

    let mut broken = minimal_patient();
    broken.telecom.push(ContactPoint {
        value: Some("+44 20 7946 0100".parse().expect("valid string")),
        ..ContactPoint::default()
    });
    assert!(broken.validated().is_err());
}

#[test]
fn errors_render_as_an_operation_outcome() {
    let document = json!({
        "resourceType": "Patient",
        "contact": [{ "relationship": [{ "text": "Next of kin" }] }]
    });

    let patient: Patient = serde_json::from_value(document).expect("parses");
    let outcome = patient
        .check()
        .expect_err("pat-1 must fail")
        .to_operation_outcome();

    assert_eq!(outcome["resourceType"], "OperationOutcome");

    // Warnings travel with the errors — the outcome is the whole report, in
    // the order the tree was walked, not just the fatal half.
    let issues = outcome["issue"].as_array().expect("issue list");
    assert!(issues.iter().any(|issue| issue["severity"] == "warning"));

    let error = issues
        .iter()
        .find(|issue| issue["severity"] == "error")
        .expect("the pat-1 error is present");
    assert_eq!(error["code"], "invariant");
    assert_eq!(error["expression"][0], "Patient.contact[0]");
}
