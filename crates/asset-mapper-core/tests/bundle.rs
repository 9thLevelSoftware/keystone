use asset_mapper_core::{LlmBundle, PackRecord};

#[test]
fn bundle_omits_raw_connector_transforms() {
    let input = std::fs::read_to_string(format!(
        "{}/../../fixtures/phase0/simple_pack.assetmap.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture pack can be read");
    let pack: PackRecord = serde_json::from_str(&input).expect("fixture pack parses");

    let bundle = LlmBundle::from_pack(&pack);
    let bundle_json = serde_json::to_value(&bundle).expect("bundle serializes");

    assert!(bundle_json.get("assets").is_some());
    assert!(
        !bundle_json.to_string().contains("orientation_quat_xyzw"),
        "LLM bundle must not expose raw quaternion data"
    );
    assert!(
        !bundle_json.to_string().contains("frame3d"),
        "LLM bundle must not expose raw connector frame data"
    );

    insta::assert_json_snapshot!(bundle_json);
}
