use asset_mapper_core::{Diagnostic, PackRecord, Severity, validate_pack};

fn load_pack(path: &str) -> PackRecord {
    let input = std::fs::read_to_string(format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR")))
        .expect("fixture can be read");
    serde_json::from_str(&input).expect("fixture parses")
}

#[test]
fn valid_fixture_has_no_validation_errors() {
    let pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    let report = validate_pack(&pack);

    assert!(
        report.diagnostics.is_empty(),
        "expected no validation diagnostics, got {:#?}",
        report.diagnostics
    );
}

#[test]
fn unknown_connector_class_is_an_error() {
    let pack = load_pack("fixtures/phase0/invalid_pack_unknown_class.assetmap.json");
    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "unknown_connector_class")
        .expect("unknown connector class diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("bad_corridor"));
    assert_eq!(diagnostic.connector_id.as_deref(), Some("front"));
}

#[test]
fn connector_class_without_rule_is_a_warning() {
    let pack = load_pack("fixtures/phase0/invalid_pack_unknown_class.assetmap.json");
    let report = validate_pack(&pack);

    let diagnostic = find_code(&report.diagnostics, "connector_class_has_no_rule")
        .expect("connector class without rule diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Warning);
    assert!(diagnostic.connector_id.is_none());
}

#[test]
fn duplicate_asset_ids_are_errors() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[1].asset_id = pack.assets[0].asset_id.clone();

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "duplicate_asset_id")
        .expect("duplicate asset id diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("corridor_a"));
}

#[test]
fn non_normalized_3d_connector_quaternion_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    if let asset_mapper_core::ConnectorFrame::Frame3d {
        orientation_quat_xyzw,
        ..
    } = &mut pack.assets[0].connectors[0].frame
    {
        *orientation_quat_xyzw = [0.0, 0.0, 0.0, 2.0];
    }

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "connector_quaternion_not_normalized")
        .expect("non-normalized connector quaternion diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("corridor_a"));
    assert_eq!(diagnostic.connector_id.as_deref(), Some("front"));
}

fn find_code<'a>(diagnostics: &'a [Diagnostic], code: &str) -> Option<&'a Diagnostic> {
    diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == code)
}
