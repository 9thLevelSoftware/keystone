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
fn init_rejects_missing_license() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");
    command
        .args([
            "init",
            temp.path().to_str().expect("utf8"),
            "--name",
            "Kit",
            "--author",
            "Org",
        ])
        .assert()
        .failure();
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
            "--license",
            "MIT",
            "--author",
            "Test Author",
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
        "--license",
        "MIT",
        "--author",
        "Test Author",
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

#[test]
fn measure_bounds_clears_placeholder_for_glb() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    // Minimal triangle GLB with known AABB.
    let positions = [-0.5_f32, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 1.0, 0.0];
    let mut binary = Vec::new();
    for v in positions {
        binary.extend_from_slice(&v.to_le_bytes());
    }
    let json = serde_json::json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 }, "mode": 4 }] }],
        "accessors": [{
            "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
            "min": [-0.5, 0.0, 0.0], "max": [0.5, 1.0, 0.0]
        }],
        "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": binary.len() }],
        "buffers": [{ "byteLength": binary.len() }]
    });
    let mut json_bytes = serde_json::to_vec(&json).expect("json");
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(0x20);
    }
    while binary.len() % 4 != 0 {
        binary.push(0);
    }
    let total = 12 + 8 + json_bytes.len() + 8 + binary.len();
    let mut glb = Vec::new();
    glb.extend_from_slice(&0x46546c67u32.to_le_bytes());
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&(total as u32).to_le_bytes());
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4e4f534au32.to_le_bytes());
    glb.extend_from_slice(&json_bytes);
    glb.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004e4942u32.to_le_bytes());
    glb.extend_from_slice(&binary);
    std::fs::write(temp.path().join("wall.glb"), glb).expect("write glb");

    let mut init = Command::cargo_bin("asset-mapper").expect("binary exists");
    init.args([
        "init",
        temp.path().to_str().expect("utf8"),
        "--name",
        "Measure Pack",
        "--license",
        "MIT",
        "--author",
        "Test Author",
    ])
    .assert()
    .success();

    // Force placeholder bounds after init (in case measure already ran).
    let sidecar = temp.path().join(".asset-mapper").join("pack.assetmap.json");
    let mut pack: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&sidecar).expect("read")).expect("parse");
    pack["assets"][0]["bounds"] = serde_json::json!({
        "min": [-0.5, -0.5, -0.5],
        "max": [0.5, 0.5, 0.5]
    });
    pack["assets"][0]["dimensions"] = serde_json::json!([1.0, 1.0, 1.0]);
    pack["assets"][0]["review_flags"] = serde_json::json!(["bounds_placeholder"]);
    std::fs::write(&sidecar, serde_json::to_string_pretty(&pack).expect("ser")).expect("write");

    let mut measure = Command::cargo_bin("asset-mapper").expect("binary exists");
    measure
        .args(["measure-bounds", temp.path().to_str().expect("utf8")])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"measured\""))
        .stdout(predicate::str::contains("wall.glb"));

    let reloaded: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&sidecar).expect("read")).expect("parse");
    let flags = reloaded["assets"][0]["review_flags"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        !flags
            .iter()
            .any(|f| f.as_str() == Some("bounds_placeholder")),
        "bounds_placeholder should be cleared: {flags:?}"
    );
    let dims = reloaded["assets"][0]["dimensions"]
        .as_array()
        .expect("dimensions");
    assert!((dims[0].as_f64().unwrap() - 1.0).abs() < 0.01);
    assert!((dims[1].as_f64().unwrap() - 1.0).abs() < 0.01);
}

#[test]
fn accept_drift_updates_hash() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::write(temp.path().join("wall.glb"), b"wall-v1").expect("asset is written");

    let mut init = Command::cargo_bin("asset-mapper").expect("binary exists");
    init.args([
        "init",
        temp.path().to_str().expect("temp path is utf-8"),
        "--name",
        "Drift Pack",
        "--license",
        "MIT",
        "--author",
        "Test Author",
    ])
    .assert()
    .success();

    std::fs::write(temp.path().join("wall.glb"), b"wall-v2").expect("asset changes");

    let mut accept = Command::cargo_bin("asset-mapper").expect("binary exists");
    accept
        .args([
            "accept-drift",
            temp.path().to_str().expect("temp path is utf-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"drifted_assets\": []"))
        .stdout(predicate::str::contains("wall.glb"));
}

#[test]
fn export_engine_and_gltf_extras() {
    let mut engine = Command::cargo_bin("asset-mapper").expect("binary exists");
    engine
        .args([
            "export-engine",
            &fixture_path("fixtures/phase0/simple_pack.assetmap.json"),
            "--target",
            "unity",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("corridor_end"))
        .stdout(predicate::str::contains("local_position"));

    let temp = tempfile::tempdir().expect("temp");
    let out = temp.path().join("extras.json");
    let mut gltf = Command::cargo_bin("asset-mapper").expect("binary exists");
    gltf.args([
        "export-gltf-extras",
        &fixture_path("fixtures/phase0/simple_pack.assetmap.json"),
        "--output",
        out.to_str().expect("utf8"),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("keystone.gltf.extras"));

    let body = std::fs::read_to_string(&out).expect("extras written");
    assert!(body.contains("phase0_corridor"));
}

#[test]
fn migrate_legacy_sidecar() {
    let temp = tempfile::tempdir().expect("temp");
    let mut pack: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixture_path("fixtures/phase0/simple_pack.assetmap.json"))
            .expect("read"),
    )
    .expect("parse");
    pack["schema_version"] = serde_json::json!(0);
    let path = temp.path().join("legacy.assetmap.json");
    std::fs::write(&path, serde_json::to_string_pretty(&pack).expect("ser")).expect("write");

    let mut migrate = Command::cargo_bin("asset-mapper").expect("binary exists");
    migrate
        .args(["migrate", path.to_str().expect("utf8")])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"to_version\": 2"));

    let migrated: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse");
    assert_eq!(migrated["schema_version"], 2);
    assert!(
        migrated["license_summary"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    assert!(migrated["vocabulary"]["semantic_tags"].as_array().is_some());
}
