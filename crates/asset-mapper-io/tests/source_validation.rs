use asset_mapper_core::{Severity, validate_pack};
use asset_mapper_io::{
    init_pack_folder, read_pack_from_input, validate_pack_sources, write_pack_sidecar,
};

#[test]
fn source_validation_reports_missing_drifted_and_untracked_files() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::write(temp.path().join("wall.glb"), b"wall-v1").expect("wall is written");
    std::fs::write(temp.path().join("floor.glb"), b"floor-v1").expect("floor is written");
    init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("init succeeds");

    let mut loaded = read_pack_from_input(temp.path()).expect("sidecar reloads");
    loaded
        .pack
        .assets
        .retain(|asset| asset.source_path == "wall.glb");
    write_pack_sidecar(temp.path(), &loaded.pack).expect("sidecar rewrites");

    std::fs::write(temp.path().join("wall.glb"), b"wall-v2").expect("wall drifts");
    std::fs::remove_file(temp.path().join("floor.glb")).expect("floor is removed");
    std::fs::write(temp.path().join("ceiling.glb"), b"ceiling").expect("ceiling is written");

    let report = validate_pack_sources(temp.path(), &loaded.pack).expect("source validation runs");

    let drift = report
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "source_hash_drift"
                && diagnostic.severity == Severity::Warning
                && diagnostic.asset_id.as_deref() == Some("wall")
        })
        .expect("source hash drift diagnostic is present");
    assert!(drift.message.contains("expected `sha256:"));
    assert!(drift.message.contains("current `sha256:"));
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "source_file_untracked"
                && diagnostic.severity == Severity::Warning)
    );
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "source_file_missing")
    );

    let mut missing_pack = loaded.pack.clone();
    missing_pack.assets[0].source_path = "missing.glb".to_owned();
    let missing_report =
        validate_pack_sources(temp.path(), &missing_pack).expect("source validation runs");
    assert!(
        missing_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "source_file_missing"
                && diagnostic.severity == Severity::Warning)
    );

    let mut combined = validate_pack(&missing_pack);
    combined.extend(missing_report.diagnostics);
    assert!(
        combined
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "source_file_missing")
    );
}
