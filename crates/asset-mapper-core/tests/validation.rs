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

#[test]
fn duplicate_source_paths_are_errors() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[1].source_path = pack.assets[0].source_path.clone();

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "duplicate_source_path")
        .expect("duplicate source path diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
}

#[test]
fn non_finite_dimensions_are_errors() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[0].dimensions[0] = f32::NAN;

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "non_finite_dimensions")
        .expect("non-finite dimensions diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("corridor_a"));
}

#[test]
fn non_finite_bounds_are_errors() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[0].bounds.max[1] = f32::INFINITY;

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "non_finite_bounds")
        .expect("non-finite bounds diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("corridor_a"));
}

#[test]
fn non_finite_snap_tolerance_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[0].connectors[0].snap_tolerance = f32::NAN;

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "non_finite_snap_tolerance")
        .expect("non-finite snap tolerance diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("corridor_a"));
    assert_eq!(diagnostic.connector_id.as_deref(), Some("front"));
}

#[test]
fn non_finite_3d_connector_quaternion_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    if let asset_mapper_core::ConnectorFrame::Frame3d {
        orientation_quat_xyzw,
        ..
    } = &mut pack.assets[0].connectors[0].frame
    {
        orientation_quat_xyzw[0] = f32::NAN;
    }

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "non_finite_connector_quaternion")
        .expect("non-finite connector quaternion diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("corridor_a"));
    assert_eq!(diagnostic.connector_id.as_deref(), Some("front"));
}

#[test]
fn placeholder_review_flags_are_warnings() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[0]
        .review_flags
        .push(asset_mapper_core::ReviewFlag::BoundsPlaceholder);
    pack.assets[0]
        .review_flags
        .push(asset_mapper_core::ReviewFlag::OrientationPlaceholder);
    pack.assets[0]
        .review_flags
        .push(asset_mapper_core::ReviewFlag::PivotPlaceholder);

    let report = validate_pack(&pack);

    let bounds = find_code(&report.diagnostics, "placeholder_bounds")
        .expect("placeholder bounds diagnostic is present");
    let orientation = find_code(&report.diagnostics, "placeholder_orientation")
        .expect("placeholder orientation diagnostic is present");
    let pivot = find_code(&report.diagnostics, "placeholder_pivot")
        .expect("placeholder pivot diagnostic is present");
    assert_eq!(bounds.severity, Severity::Warning);
    assert_eq!(orientation.severity, Severity::Warning);
    assert_eq!(pivot.severity, Severity::Warning);
}

fn find_code<'a>(diagnostics: &'a [Diagnostic], code: &str) -> Option<&'a Diagnostic> {
    diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == code)
}
