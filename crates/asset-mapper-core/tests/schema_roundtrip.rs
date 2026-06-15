use asset_mapper_core::{AssemblyPlan, PackRecord};
use serde_json::Value;

#[test]
fn fixture_pack_round_trips_without_data_loss() {
    let input = include_str!("../../../fixtures/phase0/simple_pack.assetmap.json");
    let original_json: Value = serde_json::from_str(input).expect("fixture pack is valid JSON");
    let pack: PackRecord = serde_json::from_str(input).expect("fixture pack parses");

    assert_eq!(
        pack.schema_version,
        asset_mapper_core::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(pack.pack_id, "phase0_corridor");
    assert_eq!(pack.assets.len(), 2);
    assert_eq!(pack.assets[0].connectors[0].connector_id, "front");

    let serialized = serde_json::to_string_pretty(&pack).expect("pack serializes");
    let serialized_json: Value =
        serde_json::from_str(&serialized).expect("serialized pack is valid JSON");
    assert_eq!(original_json, serialized_json);

    let reparsed: PackRecord = serde_json::from_str(&serialized).expect("serialized pack reparses");
    assert_eq!(pack, reparsed);
}

#[test]
fn fixture_plan_round_trips_without_data_loss() {
    let input = include_str!("../../../fixtures/phase0/simple_plan.json");
    let original_json: Value = serde_json::from_str(input).expect("fixture plan is valid JSON");
    let plan: AssemblyPlan = serde_json::from_str(input).expect("fixture plan parses");

    assert_eq!(plan.root_asset_id, "corridor_a");
    assert_eq!(plan.operations.len(), 1);
    assert_eq!(plan.operations[0].placed_asset_id, "corridor_b");

    let serialized = serde_json::to_string_pretty(&plan).expect("plan serializes");
    let serialized_json: Value =
        serde_json::from_str(&serialized).expect("serialized plan is valid JSON");
    assert_eq!(original_json, serialized_json);

    let reparsed: AssemblyPlan =
        serde_json::from_str(&serialized).expect("serialized plan reparses");
    assert_eq!(plan, reparsed);
}

#[test]
fn invalid_fixture_intentionally_references_missing_connector_class() {
    let input = include_str!("../../../fixtures/phase0/invalid_pack_unknown_class.assetmap.json");
    let pack: PackRecord =
        serde_json::from_str(input).expect("invalid validation fixture still parses");

    let declared_classes: Vec<&str> = pack
        .connector_classes
        .iter()
        .map(|class| class.class.as_str())
        .collect();
    assert_eq!(declared_classes, vec!["corridor_end"]);
    assert_eq!(pack.assets[0].connectors[0].class, "missing_class");
    assert!(!declared_classes.contains(&"missing_class"));
}

#[test]
fn pack_record_has_meaningful_json_schema() {
    let schema = schemars::schema_for!(PackRecord);
    let schema_json = serde_json::to_value(schema).expect("schema serializes to JSON");

    assert_eq!(schema_json["title"], Value::String("PackRecord".to_owned()));
    assert!(
        schema_json["required"]
            .as_array()
            .expect("required is array")
            .contains(&Value::String("assets".to_owned()))
    );
    assert!(
        schema_json["properties"]
            .get("coordinate_convention")
            .is_some()
    );
    assert!(schema_json["properties"].get("connector_classes").is_some());
    assert!(
        schema_json["properties"]
            .get("compatibility_rules")
            .is_some()
    );
    assert!(schema_json.to_string().contains("frame3d"));
    assert!(schema_json.to_string().contains("model3d"));
    assert!(schema_json.to_string().contains("base_center"));
}
