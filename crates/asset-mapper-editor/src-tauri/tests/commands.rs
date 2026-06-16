use asset_mapper_core::{
    AllowedRotation, CompatibilityRule, ConnectorClass, ConnectorFrame, ConnectorRecord,
    ConnectorRole,
};
use asset_mapper_editor::commands::{
    export_bundle, init_pack_folder, open_pack_folder, save_pack, validate_pack,
};
use asset_mapper_editor::dto::EditorPackState;

fn write_asset(path: &std::path::Path, name: &str, bytes: &[u8]) {
    std::fs::write(path.join(name), bytes).expect("asset is written");
}

#[test]
fn init_open_validate_and_save_pack_state() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    write_asset(temp.path(), "wall.glb", b"wall");

    let state: EditorPackState =
        init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("pack initializes");
    assert_eq!(state.pack.display_name, "Dungeon Kit");
    assert_eq!(state.assets.len(), 1);
    assert_eq!(state.assets[0].source_path, "wall.glb");
    assert!(state.assets[0].preview_supported);
    assert!(state.assets[0].absolute_path.ends_with("wall.glb"));

    let opened = open_pack_folder(temp.path()).expect("pack opens");
    assert_eq!(opened.pack.pack_id, state.pack.pack_id);

    let validation = validate_pack(opened.clone()).expect("validation runs");
    assert!(validation.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "placeholder_bounds" && diagnostic.asset_id.as_deref() == Some("wall")
    }));

    let saved = save_pack(opened).expect("pack saves with warnings");
    assert!(
        saved
            .validation
            .diagnostics
            .iter()
            .all(|diagnostic| { diagnostic.severity == asset_mapper_core::Severity::Warning })
    );
}

#[test]
fn export_bundle_is_blocked_by_validation_errors() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    write_asset(temp.path(), "wall.glb", b"wall");
    let mut state =
        init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("pack initializes");
    state.pack.assets[0].connectors.push(ConnectorRecord {
        connector_id: "front".to_owned(),
        display_name: "Front".to_owned(),
        class: "doorway".to_owned(),
        role: ConnectorRole::Symmetric,
        frame: ConnectorFrame::Frame3d {
            position: [0.0, 0.0, 0.5],
            orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        mating_axis: asset_mapper_core::Axis3::PosZ,
        up_reference: asset_mapper_core::Axis3::PosY,
        snap_tolerance: 0.01,
    });

    let output = temp.path().join("bundle.json");
    let error = export_bundle(state, output).expect_err("export rejects unknown connector class");
    assert_eq!(error.code, "validation_failed");
}

#[test]
fn export_bundle_writes_llm_bundle_when_valid() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    write_asset(temp.path(), "wall.glb", b"wall");
    let mut state =
        init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("pack initializes");
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
            position: [0.0, 0.0, 0.5],
            orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        mating_axis: asset_mapper_core::Axis3::PosZ,
        up_reference: asset_mapper_core::Axis3::PosY,
        snap_tolerance: 0.01,
    });

    let output = temp.path().join("bundle.json");
    let result = export_bundle(state, &output).expect("export succeeds");

    assert_eq!(result.output_path, output.to_string_lossy());
    let bundle = std::fs::read_to_string(output).expect("bundle is written");
    assert!(bundle.contains("\"pack_id\": \"dungeon_kit\""));
    assert!(!bundle.contains("orientation_quat_xyzw"));
}
