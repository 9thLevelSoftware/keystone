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
