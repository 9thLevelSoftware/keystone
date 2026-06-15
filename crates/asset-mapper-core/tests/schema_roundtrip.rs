use asset_mapper_core::{AssemblyPlan, PackRecord};

#[test]
fn fixture_pack_round_trips_without_data_loss() {
    let input = include_str!("../../../fixtures/phase0/simple_pack.assetmap.json");
    let pack: PackRecord = serde_json::from_str(input).expect("fixture pack parses");

    assert_eq!(
        pack.schema_version,
        asset_mapper_core::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(pack.pack_id, "phase0_corridor");
    assert_eq!(pack.assets.len(), 2);
    assert_eq!(pack.assets[0].connectors[0].connector_id, "front");

    let serialized = serde_json::to_string_pretty(&pack).expect("pack serializes");
    let reparsed: PackRecord = serde_json::from_str(&serialized).expect("serialized pack reparses");
    assert_eq!(pack, reparsed);
}

#[test]
fn fixture_plan_round_trips_without_data_loss() {
    let input = include_str!("../../../fixtures/phase0/simple_plan.json");
    let plan: AssemblyPlan = serde_json::from_str(input).expect("fixture plan parses");

    assert_eq!(plan.root_asset_id, "corridor_a");
    assert_eq!(plan.operations.len(), 1);
    assert_eq!(plan.operations[0].placed_asset_id, "corridor_b");

    let serialized = serde_json::to_string_pretty(&plan).expect("plan serializes");
    let reparsed: AssemblyPlan =
        serde_json::from_str(&serialized).expect("serialized plan reparses");
    assert_eq!(plan, reparsed);
}

#[test]
fn pack_record_has_json_schema() {
    let schema = schemars::schema_for!(PackRecord);
    let schema_json = serde_json::to_value(schema).expect("schema serializes to JSON");
    assert_eq!(
        schema_json["title"],
        serde_json::Value::String("PackRecord".to_owned())
    );
}
