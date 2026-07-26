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
fn non_finite_rotation_steps_are_errors() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.compatibility_rules[0].rotation = asset_mapper_core::AllowedRotation::StepsDeg {
        values: vec![0.0, f32::NAN],
    };

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "non_finite_rotation_steps")
        .expect("non-finite rotation steps diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(diagnostic.asset_id.is_none());
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
fn non_finite_3d_connector_position_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    if let asset_mapper_core::ConnectorFrame::Frame3d { position, .. } =
        &mut pack.assets[0].connectors[0].frame
    {
        position[2] = f32::INFINITY;
    }

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "non_finite_connector_position")
        .expect("non-finite connector position diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("corridor_a"));
    assert_eq!(diagnostic.connector_id.as_deref(), Some("front"));
}

#[test]
fn non_finite_2d_connector_position_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[0].connectors[0].frame = asset_mapper_core::ConnectorFrame::Frame2d {
        position: [f32::NEG_INFINITY, 0.0],
        normal: [1.0, 0.0],
        grid_cell: None,
    };

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "non_finite_connector_position")
        .expect("non-finite connector position diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("corridor_a"));
    assert_eq!(diagnostic.connector_id.as_deref(), Some("front"));
}

#[test]
fn non_finite_2d_connector_normal_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[0].connectors[0].frame = asset_mapper_core::ConnectorFrame::Frame2d {
        position: [0.0, 0.0],
        normal: [f32::NAN, 0.0],
        grid_cell: None,
    };

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "non_finite_connector_2d_normal")
        .expect("non-finite connector 2D normal diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("corridor_a"));
    assert_eq!(diagnostic.connector_id.as_deref(), Some("front"));
    assert!(
        find_code(&report.diagnostics, "connector_2d_normal_degenerate").is_none(),
        "non-finite 2D normal should not also emit a degenerate-normal diagnostic"
    );
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

#[test]
fn missing_license_summary_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.license_summary = "   ".to_owned();

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "missing_license_summary")
        .expect("missing license summary diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
}

#[test]
fn unspecified_placeholder_license_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.license_summary = asset_mapper_core::PLACEHOLDER_LICENSE_SUMMARY.to_owned();

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "missing_license_summary")
        .expect("placeholder license must fail production gate");
    assert_eq!(diagnostic.severity, Severity::Error);
}

#[test]
fn missing_provenance_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.provenance = asset_mapper_core::PackProvenance::default();

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "missing_provenance")
        .expect("missing provenance diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
}

#[test]
fn notes_only_provenance_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.provenance = asset_mapper_core::PackProvenance {
        notes: Some("Migrated to schema v2; set source or author for production.".to_owned()),
        ..asset_mapper_core::PackProvenance::default()
    };

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "missing_provenance")
        .expect("notes-only provenance must fail production gate");
    assert_eq!(diagnostic.severity, Severity::Error);
}

#[test]
fn empty_vocabulary_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.vocabulary.semantic_tags.clear();
    pack.vocabulary.affordances.clear();
    pack.vocabulary.placement_constraints.clear();
    // Avoid cascading unknown-term errors from asset tags against empty lists.
    for asset in &mut pack.assets {
        asset.semantic_tags.clear();
        asset.affordances.clear();
        asset.placement_constraints.clear();
    }

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "empty_vocabulary")
        .expect("empty vocabulary diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
}

#[test]
fn unknown_semantic_tag_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[0].semantic_tags.push("not_in_vocab".to_owned());

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "unknown_semantic_tag")
        .expect("unknown semantic tag diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("corridor_a"));
}

#[test]
fn namespaced_tag_allowed_when_enabled() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.vocabulary.allow_namespaced_extensions = true;
    pack.assets[0]
        .semantic_tags
        .push("project:custom_tag".to_owned());

    let report = validate_pack(&pack);

    assert!(
        find_code(&report.diagnostics, "unknown_semantic_tag").is_none(),
        "namespaced tag should be accepted when enabled: {:#?}",
        report.diagnostics
    );
}

#[test]
fn namespaced_tag_rejected_when_disabled() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.vocabulary.allow_namespaced_extensions = false;
    pack.assets[0]
        .semantic_tags
        .push("project:custom_tag".to_owned());

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "unknown_semantic_tag")
        .expect("namespaced tag should be rejected when disabled");
    assert_eq!(diagnostic.severity, Severity::Error);
}

#[test]
fn validation_uses_allows_term_for_vocab_acceptance() {
    // Guard against validate.rs re-implementing namespaced rules separately from
    // ControlledVocabulary::allows_term.
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.vocabulary.allow_namespaced_extensions = true;
    let listed = pack.vocabulary.semantic_tags[0].clone();
    assert!(
        pack.vocabulary
            .allows_term(&pack.vocabulary.semantic_tags, &listed)
    );
    assert!(
        pack.vocabulary
            .allows_term(&pack.vocabulary.semantic_tags, "project:from_allows_term")
    );
    assert!(
        !pack
            .vocabulary
            .allows_term(&pack.vocabulary.semantic_tags, "not_listed")
    );

    pack.assets[0].semantic_tags = vec![listed, "project:from_allows_term".to_owned()];
    let report = validate_pack(&pack);
    assert!(
        find_code(&report.diagnostics, "unknown_semantic_tag").is_none(),
        "validate must accept exactly what allows_term accepts: {:#?}",
        report.diagnostics
    );

    pack.assets[0].semantic_tags.push("not_listed".to_owned());
    let report = validate_pack(&pack);
    assert!(find_code(&report.diagnostics, "unknown_semantic_tag").is_some());
}

fn find_code<'a>(diagnostics: &'a [Diagnostic], code: &str) -> Option<&'a Diagnostic> {
    diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == code)
}
