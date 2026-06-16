use asset_mapper_core::{
    AllowedRotation, CompatibilityRule, ConnectorClass, ConnectorFrame, ConnectorRecord,
    ConnectorRole,
};
use asset_mapper_editor::commands::{
    export_bundle, index_pack_folder, init_pack_folder, open_pack_folder, save_pack, validate_pack,
};
use asset_mapper_editor::dto::EditorPackState;

fn write_asset(path: &std::path::Path, name: &str, bytes: &[u8]) {
    std::fs::write(path.join(name), bytes).expect("asset is written");
}

fn error_code(error: asset_mapper_editor::error::EditorCommandError) -> String {
    error.code
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

#[test]
fn open_and_validate_reject_invalid_source_paths() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    write_asset(temp.path(), "wall.glb", b"wall");
    let mut state =
        init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("pack initializes");

    state.pack.assets[0].source_path = "../outside.glb".to_owned();
    asset_mapper_io::write_pack_sidecar(temp.path(), &state.pack).expect("sidecar is written");
    let error = open_pack_folder(temp.path()).expect_err("open rejects traversal source path");
    assert_eq!(error_code(error), "invalid_source_path");

    state.pack.assets[0].source_path = temp
        .path()
        .join("outside.glb")
        .to_string_lossy()
        .into_owned();
    let error = validate_pack(state).expect_err("validation rejects absolute source path");
    assert_eq!(error_code(error), "invalid_source_path");
}

#[test]
fn save_and_export_reject_invalid_source_paths_before_writing() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    write_asset(temp.path(), "wall.glb", b"wall");
    let mut state =
        init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("pack initializes");
    state.pack.assets[0].source_path = "../outside.glb".to_owned();

    let save_error = save_pack(state.clone()).expect_err("save rejects invalid source path");
    assert_eq!(error_code(save_error), "invalid_source_path");
    let sidecar = std::fs::read_to_string(asset_mapper_io::canonical_sidecar_path(temp.path()))
        .expect("sidecar is readable");
    assert!(!sidecar.contains("../outside.glb"));

    let output = temp.path().join("bundle.json");
    let export_error =
        export_bundle(state, &output).expect_err("export rejects invalid source path");
    assert_eq!(error_code(export_error), "invalid_source_path");
    assert!(!output.exists());
}

#[test]
fn index_pack_folder_returns_state_with_changed_and_new_asset_statuses() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    write_asset(temp.path(), "wall.glb", b"wall");
    init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("pack initializes");

    write_asset(temp.path(), "wall.glb", b"changed wall");
    write_asset(temp.path(), "floor.glb", b"floor");

    let result = index_pack_folder(temp.path()).expect("pack indexes");
    assert_eq!(result.report.drifted_assets, vec!["wall.glb"]);
    assert_eq!(result.report.new_assets, vec!["floor.glb"]);

    let wall = result
        .state
        .assets
        .iter()
        .find(|asset| asset.source_path == "wall.glb")
        .expect("wall asset is present");
    assert!(wall.exists);
    assert_eq!(wall.hash_matches, Some(false));

    let floor = result
        .state
        .assets
        .iter()
        .find(|asset| asset.source_path == "floor.glb")
        .expect("floor asset is present");
    assert!(floor.exists);
    assert_eq!(floor.hash_matches, Some(true));
    assert!(floor.preview_supported);
}
