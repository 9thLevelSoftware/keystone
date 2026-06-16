use asset_mapper_core::{
    AllowedRotation, Axis3, CompatibilityRule, ConnectorClass, ConnectorFrame, ConnectorRecord,
    ConnectorRole, Severity,
};
use asset_mapper_editor::commands::{
    export_bundle, init_pack_folder, save_pack, validate_pack,
};

fn repo_fixture(path: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path)
}

#[test]
fn phase2_fixture_can_be_authored_saved_validated_and_exported() {
    let source = repo_fixture("fixtures/phase2/modular_pack/wall.glb");
    assert!(source.is_file(), "run npm run fixture:phase2 before this test");

    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::copy(&source, temp.path().join("wall.glb")).expect("fixture copies");

    let mut state =
        init_pack_folder(temp.path(), "Phase 2 Smoke".to_owned()).expect("pack initializes");
    state.pack.connector_classes.push(ConnectorClass {
        class: "doorway".to_owned(),
        display_name: "Doorway".to_owned(),
    });
    state.pack.compatibility_rules.push(CompatibilityRule {
        a_class: "doorway".to_owned(),
        b_class: "doorway".to_owned(),
        rotation: AllowedRotation::Locked,
    });
    state.pack.assets[0].connectors.push(ConnectorRecord {
        connector_id: "front".to_owned(),
        display_name: "Front".to_owned(),
        class: "doorway".to_owned(),
        role: ConnectorRole::Symmetric,
        frame: ConnectorFrame::Frame3d {
            position: [0.0, 0.0, 0.0],
            orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        mating_axis: Axis3::PosZ,
        up_reference: Axis3::PosY,
        snap_tolerance: 0.01,
    });

    let saved = save_pack(state).expect("pack saves");
    let validation = validate_pack(saved.state.clone()).expect("validation runs");
    assert!(
        validation
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity == Severity::Warning)
    );

    let output = temp.path().join("bundle.json");
    export_bundle(saved.state, &output).expect("bundle exports");
    let bundle = std::fs::read_to_string(output).expect("bundle reads");
    assert!(bundle.contains("\"connector_id\": \"front\""));
    assert!(!bundle.contains("orientation_quat_xyzw"));
}
