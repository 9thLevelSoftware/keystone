use asset_mapper_core::{
    Axis3, CURRENT_SCHEMA_VERSION, CoordinateConvention, Handedness, PackRecord, Unit,
};
use asset_mapper_io::{
    IoError, PackInputKind, SIDECAR_FILE, canonical_sidecar_path, read_pack_from_input,
    resolve_pack_input_path, write_pack_sidecar,
};

fn minimal_pack() -> PackRecord {
    PackRecord {
        schema_version: CURRENT_SCHEMA_VERSION,
        pack_id: "test_pack".to_owned(),
        display_name: "Test Pack".to_owned(),
        coordinate_convention: CoordinateConvention {
            handedness: Handedness::Right,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
        },
        default_units: Unit::Meters,
        connector_classes: Vec::new(),
        compatibility_rules: Vec::new(),
        assets: Vec::new(),
    }
}

#[test]
fn canonical_sidecar_path_lives_under_metadata_directory() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let sidecar = canonical_sidecar_path(temp.path());

    assert_eq!(
        sidecar.file_name().and_then(|name| name.to_str()),
        Some(SIDECAR_FILE)
    );
    assert_eq!(
        sidecar
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str()),
        Some(".asset-mapper")
    );
}

#[test]
fn resolves_pack_folder_to_canonical_sidecar() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let resolved = resolve_pack_input_path(temp.path()).expect("folder input resolves");

    assert_eq!(resolved.kind, PackInputKind::PackFolder);
    assert_eq!(resolved.sidecar_path, canonical_sidecar_path(temp.path()));
    assert_eq!(resolved.pack_root.as_deref(), Some(temp.path()));
}

#[test]
fn resolves_direct_sidecar_file() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let sidecar = temp.path().join("custom.assetmap.json");
    std::fs::write(&sidecar, "{}").expect("sidecar file can be written");

    let resolved = resolve_pack_input_path(&sidecar).expect("file input resolves");

    assert_eq!(resolved.kind, PackInputKind::DirectSidecar);
    assert_eq!(resolved.sidecar_path, sidecar);
    assert_eq!(resolved.pack_root, None);
}

#[test]
fn resolves_canonical_direct_sidecar_with_inferred_pack_root() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let sidecar = canonical_sidecar_path(temp.path());
    std::fs::create_dir_all(sidecar.parent().expect("sidecar has parent"))
        .expect("metadata dir can be created");
    std::fs::write(&sidecar, "{}").expect("sidecar file can be written");

    let resolved = resolve_pack_input_path(&sidecar).expect("canonical sidecar resolves");

    assert_eq!(resolved.kind, PackInputKind::DirectSidecar);
    assert_eq!(resolved.sidecar_path, sidecar);
    assert_eq!(resolved.pack_root.as_deref(), Some(temp.path()));
}

#[test]
fn non_canonical_metadata_sidecar_does_not_infer_pack_root() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let metadata_dir = temp.path().join(".asset-mapper");
    std::fs::create_dir_all(&metadata_dir).expect("metadata dir can be created");
    let sidecar = metadata_dir.join("custom.assetmap.json");
    std::fs::write(&sidecar, "{}").expect("sidecar file can be written");

    let resolved = resolve_pack_input_path(&sidecar).expect("direct sidecar resolves");

    assert_eq!(resolved.kind, PackInputKind::DirectSidecar);
    assert_eq!(resolved.sidecar_path, sidecar);
    assert_eq!(resolved.pack_root, None);
}

#[test]
fn rejects_existing_non_sidecar_file_input() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let file = temp.path().join("wall.glb");
    std::fs::write(&file, "not a sidecar").expect("file can be written");

    let error = resolve_pack_input_path(&file).expect_err("non-sidecar file is invalid");

    assert!(matches!(
        error,
        IoError::InvalidPackInput { path } if path == file
    ));
}

#[test]
fn rejects_non_sidecar_file_under_metadata_directory() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let metadata_dir = temp.path().join(".asset-mapper");
    std::fs::create_dir_all(&metadata_dir).expect("metadata dir can be created");
    let file = metadata_dir.join("README.md");
    std::fs::write(&file, "not a sidecar").expect("file can be written");

    let error = resolve_pack_input_path(&file).expect_err("metadata file is invalid");

    assert!(matches!(
        error,
        IoError::InvalidPackInput { path } if path == file
    ));
}

#[test]
fn reports_missing_sidecar_for_folder_input() {
    let temp = tempfile::tempdir().expect("temp dir is created");

    let error = read_pack_from_input(temp.path()).expect_err("missing sidecar fails");

    assert!(matches!(
        error,
        IoError::MissingSidecar { path } if path == canonical_sidecar_path(temp.path())
    ));
}

#[test]
fn reports_malformed_json_parse_error() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let sidecar = canonical_sidecar_path(temp.path());
    std::fs::create_dir_all(sidecar.parent().expect("sidecar has parent"))
        .expect("metadata dir can be created");
    std::fs::write(&sidecar, "{").expect("malformed sidecar can be written");

    let error = read_pack_from_input(temp.path()).expect_err("malformed sidecar fails");

    assert!(matches!(
        error,
        IoError::ParseJson { path, .. } if path == sidecar
    ));
}

#[test]
fn reads_and_writes_pack_sidecar() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let pack = minimal_pack();

    let sidecar = write_pack_sidecar(temp.path(), &pack).expect("sidecar writes");
    assert_eq!(sidecar, canonical_sidecar_path(temp.path()));

    let loaded = read_pack_from_input(temp.path()).expect("pack reloads from folder");
    assert_eq!(loaded.pack.pack_id, "test_pack");
    assert_eq!(loaded.resolved.kind, PackInputKind::PackFolder);
}
