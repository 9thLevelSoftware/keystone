use assert_cmd::Command;
use predicates::prelude::*;

fn fixture_path(relative: &str) -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn validate_accepts_valid_fixture() {
    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");

    command
        .args([
            "validate",
            &fixture_path("fixtures/phase0/simple_pack.assetmap.json"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"diagnostics\": []"));
}

#[test]
fn validate_rejects_invalid_fixture() {
    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");

    command
        .args([
            "validate",
            &fixture_path("fixtures/phase0/invalid_pack_unknown_class.assetmap.json"),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("unknown_connector_class"));
}

#[test]
fn validate_missing_relative_path_fails() {
    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");

    command
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["validate", "fixtures/phase0/simple_pack.assetmap.json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "fixtures/phase0/simple_pack.assetmap.json",
        ));
}

#[test]
fn bundle_emits_llm_context() {
    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");

    command
        .args([
            "bundle",
            &fixture_path("fixtures/phase0/simple_pack.assetmap.json"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"pack_id\": \"phase0_corridor\""))
        .stdout(predicate::str::contains("\"connector_id\": \"front\""))
        .stdout(predicate::str::contains("orientation_quat_xyzw").not());
}

#[test]
fn resolve_emits_resolved_scene() {
    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");

    command
        .args([
            "resolve",
            &fixture_path("fixtures/phase0/simple_pack.assetmap.json"),
            &fixture_path("fixtures/phase0/simple_plan.json"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"asset_id\": \"corridor_a\""))
        .stdout(predicate::str::contains("\"asset_id\": \"corridor_b\""));
}

#[test]
fn resolve_rejects_invalid_connector_orientation_without_null_scene_json() {
    let fixture_input =
        std::fs::read_to_string(fixture_path("fixtures/phase0/simple_pack.assetmap.json"))
            .expect("fixture pack can be read");
    let mut pack: serde_json::Value =
        serde_json::from_str(&fixture_input).expect("fixture pack parses");
    pack["assets"][1]["connectors"][0]["frame"]["orientation_quat_xyzw"] =
        serde_json::json!([0.0, 0.0, 0.0, 0.0]);

    let temp_dir = tempfile::tempdir().expect("temp dir can be created");
    let temp_pack = temp_dir.path().join("invalid_quaternion.assetmap.json");
    std::fs::write(
        &temp_pack,
        serde_json::to_string_pretty(&pack).expect("pack serializes"),
    )
    .expect("temp pack can be written");
    let plan_path = std::fs::canonicalize(fixture_path("fixtures/phase0/simple_plan.json"))
        .expect("fixture plan path can be canonicalized");

    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");
    command
        .args([
            "resolve",
            temp_pack.to_str().expect("temp pack path is utf-8"),
            plan_path.to_str().expect("plan path is utf-8"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid connector orientation"))
        .stdout(predicate::str::contains("null").not());
}

#[test]
fn init_creates_sidecar_for_pack_folder() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::write(temp.path().join("wall.glb"), b"wall").expect("asset is written");

    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");
    command
        .args([
            "init",
            temp.path().to_str().expect("temp path is utf-8"),
            "--name",
            "Dungeon Kit",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"new_assets\""))
        .stdout(predicate::str::contains("wall.glb"));

    assert!(
        temp.path()
            .join(".asset-mapper")
            .join("pack.assetmap.json")
            .is_file()
    );
}

#[test]
fn index_reports_drift_and_new_assets() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::write(temp.path().join("wall.glb"), b"wall-v1").expect("asset is written");

    let mut init = Command::cargo_bin("asset-mapper").expect("binary exists");
    init.args([
        "init",
        temp.path().to_str().expect("temp path is utf-8"),
        "--name",
        "Dungeon Kit",
    ])
    .assert()
    .success();

    std::fs::write(temp.path().join("wall.glb"), b"wall-v2").expect("asset changes");
    std::fs::write(temp.path().join("floor.glb"), b"floor").expect("new asset is written");

    let mut index = Command::cargo_bin("asset-mapper").expect("binary exists");
    index
        .args(["index", temp.path().to_str().expect("temp path is utf-8")])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"drifted_assets\""))
        .stdout(predicate::str::contains("wall.glb"))
        .stdout(predicate::str::contains("\"new_assets\""))
        .stdout(predicate::str::contains("floor.glb"));
}

#[test]
fn validate_bundle_and_resolve_accept_pack_folder() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let metadata_dir = temp.path().join(".asset-mapper");
    std::fs::create_dir_all(&metadata_dir).expect("metadata dir is created");
    std::fs::copy(
        fixture_path("fixtures/phase0/simple_pack.assetmap.json"),
        metadata_dir.join("pack.assetmap.json"),
    )
    .expect("fixture sidecar copies");

    let mut validate = Command::cargo_bin("asset-mapper").expect("binary exists");
    validate
        .args([
            "validate",
            temp.path().to_str().expect("temp path is utf-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("source_file_missing"));

    let mut validate_sidecar = Command::cargo_bin("asset-mapper").expect("binary exists");
    validate_sidecar
        .args([
            "validate",
            metadata_dir
                .join("pack.assetmap.json")
                .to_str()
                .expect("sidecar path is utf-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("source_file_missing").not());

    let mut bundle = Command::cargo_bin("asset-mapper").expect("binary exists");
    bundle
        .args(["bundle", temp.path().to_str().expect("temp path is utf-8")])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"pack_id\": \"phase0_corridor\""))
        .stdout(predicate::str::contains("orientation_quat_xyzw").not());

    let mut resolve = Command::cargo_bin("asset-mapper").expect("binary exists");
    resolve
        .args([
            "resolve",
            temp.path().to_str().expect("temp path is utf-8"),
            &fixture_path("fixtures/phase0/simple_plan.json"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"asset_id\": \"corridor_b\""))
        .stdout(predicate::str::contains("2.0"));
}
