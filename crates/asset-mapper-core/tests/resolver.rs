use asset_mapper_core::{AllowedRotation, AssemblyPlan, PackRecord, ResolveError, resolve_plan};
use proptest::prelude::*;

fn load_pack() -> PackRecord {
    let input = std::fs::read_to_string(format!(
        "{}/../../fixtures/phase0/simple_pack.assetmap.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture pack can be read");
    serde_json::from_str(&input).expect("fixture pack parses")
}

fn load_plan() -> AssemblyPlan {
    let input = std::fs::read_to_string(format!(
        "{}/../../fixtures/phase0/simple_plan.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture plan can be read");
    serde_json::from_str(&input).expect("fixture plan parses")
}

#[test]
fn resolves_simple_corridor_attachment() {
    let pack = load_pack();
    let plan = load_plan();

    let scene = resolve_plan(&pack, &plan).expect("plan resolves");

    assert_eq!(scene.placements.len(), 2);
    assert_eq!(scene.placements[0].asset_id, "corridor_a");
    assert_eq!(scene.placements[0].transform.translation, [0.0, 0.0, 0.0]);
    assert_eq!(scene.placements[1].asset_id, "corridor_b");
    assert_close(scene.placements[1].transform.translation[0], 0.0);
    assert_close(scene.placements[1].transform.translation[1], 0.0);
    assert_close(scene.placements[1].transform.translation[2], 0.0);
}

#[test]
fn rejects_unplaced_anchor_asset() {
    let pack = load_pack();
    let mut plan = load_plan();
    plan.operations[0].anchor_asset_id = "corridor_b".to_owned();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::AnchorAssetNotPlaced { anchor_asset_id } if anchor_asset_id == "corridor_b"
    ));
}

#[test]
fn locked_rotation_rejects_non_zero_choice() {
    let pack = load_pack();
    let mut plan = load_plan();
    plan.operations[0].rotation_choice_deg = Some(90.0);

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::RotationChoiceNotAllowed { choice } if (choice - 90.0).abs() < 0.001
    ));
}

#[test]
fn rejects_incompatible_connector_classes() {
    let mut pack = load_pack();
    pack.compatibility_rules.clear();
    let plan = load_plan();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::IncompatibleConnectorClasses { placed_class, anchor_class }
        if placed_class == "corridor_end" && anchor_class == "corridor_end"
    ));
}

proptest! {
    #[test]
    fn resolved_quaternion_is_normalized(rotation_choice in -360.0_f32..360.0_f32) {
        let mut pack = load_pack();
        pack.compatibility_rules[0].rotation = AllowedRotation::Free;
        let mut plan = load_plan();
        plan.operations[0].rotation_choice_deg = Some(rotation_choice);

        let scene = resolve_plan(&pack, &plan).expect("free rotation fixture resolves");
        let quat = scene.placements[1].transform.rotation_quat_xyzw;
        let length_squared = quat.iter().map(|component| component * component).sum::<f32>();
        prop_assert!((length_squared - 1.0).abs() < 0.001);
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {actual} to be close to {expected}"
    );
}
