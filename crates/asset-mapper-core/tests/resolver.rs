use asset_mapper_core::{
    AllowedRotation, AssemblyPlan, Axis3, ConnectorFrame, PackRecord, ResolveError, ResolvedScene,
    resolve_plan,
};
use glam::{Quat, Vec3};
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
    assert_eq!(
        scene.placements[0].transform.rotation_quat_xyzw,
        [0.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(scene.placements[1].asset_id, "corridor_b");
    assert_close(scene.placements[1].transform.translation[0], 0.0);
    assert_close(scene.placements[1].transform.translation[1], 0.0);
    assert_close(scene.placements[1].transform.translation[2], 0.0);
}

#[test]
fn rejects_unknown_root_asset() {
    let pack = load_pack();
    let mut plan = load_plan();
    plan.root_asset_id = "missing_root".to_owned();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::UnknownRootAsset { root_asset_id } if root_asset_id == "missing_root"
    ));
}

#[test]
fn rejects_unknown_placed_asset() {
    let pack = load_pack();
    let mut plan = load_plan();
    plan.operations[0].placed_asset_id = "missing_placed".to_owned();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::UnknownPlacedAsset { asset_id } if asset_id == "missing_placed"
    ));
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
fn rejects_unknown_anchor_asset_with_anchor_specific_error() {
    let pack = load_pack();
    let mut plan = load_plan();
    plan.operations[0].anchor_asset_id = "missing_anchor".to_owned();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert_eq!(
        error.to_string(),
        "anchor asset `missing_anchor` does not exist in the pack"
    );
}

#[test]
fn rejects_unknown_placed_connector() {
    let pack = load_pack();
    let mut plan = load_plan();
    plan.operations[0].placed_connector_id = "missing_connector".to_owned();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::UnknownConnector {
            asset_id,
            connector_id,
        } if asset_id == "corridor_b" && connector_id == "missing_connector"
    ));
}

#[test]
fn rejects_unknown_anchor_connector() {
    let pack = load_pack();
    let mut plan = load_plan();
    plan.operations[0].anchor_connector_id = "missing_connector".to_owned();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::UnknownConnector {
            asset_id,
            connector_id,
        } if asset_id == "corridor_a" && connector_id == "missing_connector"
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
fn step_rotation_accepts_listed_choice_and_rejects_unlisted_choice() {
    let mut pack = load_pack();
    pack.compatibility_rules[0].rotation = AllowedRotation::StepsDeg {
        values: vec![0.0, 90.0],
    };

    let mut accepted_plan = load_plan();
    accepted_plan.operations[0].rotation_choice_deg = Some(90.0);
    resolve_plan(&pack, &accepted_plan).expect("listed step rotation resolves");

    let mut rejected_plan = load_plan();
    rejected_plan.operations[0].rotation_choice_deg = Some(45.0);
    let error = resolve_plan(&pack, &rejected_plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::RotationChoiceNotAllowed { choice } if (choice - 45.0).abs() < 0.001
    ));
}

#[test]
fn rejects_non_finite_rotation_choice() {
    let mut pack = load_pack();
    pack.compatibility_rules[0].rotation = AllowedRotation::Free;
    let mut plan = load_plan();
    plan.operations[0].rotation_choice_deg = Some(f32::NAN);

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::RotationChoiceNotAllowed { choice } if choice.is_nan()
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

#[test]
fn rejects_frame2d_connectors() {
    let mut pack = load_pack();
    let connector = pack
        .assets
        .iter_mut()
        .find(|asset| asset.asset_id == "corridor_b")
        .expect("placed asset exists")
        .connectors
        .iter_mut()
        .find(|connector| connector.connector_id == "back")
        .expect("placed connector exists");
    connector.frame = ConnectorFrame::Frame2d {
        position: [0.0, 0.0],
        normal: [0.0, 1.0],
        grid_cell: None,
    };
    let plan = load_plan();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::Non3dConnector {
            asset_id,
            connector_id,
        } if asset_id == "corridor_b" && connector_id == "back"
    ));
}

#[test]
fn rejects_zero_quaternion_on_placed_connector_orientation() {
    let mut pack = load_pack();
    set_connector_orientation(&mut pack, "corridor_b", "back", [0.0, 0.0, 0.0, 0.0]);
    let plan = load_plan();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert_invalid_connector_orientation(error, "corridor_b", "back");
}

#[test]
fn rejects_zero_quaternion_on_anchor_connector_orientation() {
    let mut pack = load_pack();
    set_connector_orientation(&mut pack, "corridor_a", "front", [0.0, 0.0, 0.0, 0.0]);
    let plan = load_plan();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert_invalid_connector_orientation(error, "corridor_a", "front");
}

#[test]
fn rejects_non_finite_connector_orientation() {
    let mut pack = load_pack();
    set_connector_orientation(
        &mut pack,
        "corridor_b",
        "back",
        [0.0, f32::INFINITY, 0.0, 1.0],
    );
    let plan = load_plan();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert_invalid_connector_orientation(error, "corridor_b", "back");
}

#[test]
fn resolves_connector_orientation_from_mating_axis_and_up_reference() {
    let mut pack = load_pack();
    for asset in &mut pack.assets {
        for connector in &mut asset.connectors {
            connector.mating_axis = Axis3::PosY;
            connector.up_reference = Axis3::PosZ;
        }
    }
    let plan = load_plan();

    let scene = resolve_plan(&pack, &plan).expect("plan resolves");

    let (anchor_mating, anchor_up) = connector_world_axes(&pack, &scene, "corridor_a", "front");
    let (placed_mating, placed_up) = connector_world_axes(&pack, &scene, "corridor_b", "back");
    assert_vec3_close(placed_mating.normalize(), -anchor_mating.normalize());
    assert_vec3_close(placed_up.normalize(), anchor_up.normalize());
}

#[test]
fn accepts_compatibility_rule_declared_in_reverse_order() {
    let mut pack = load_pack();
    pack.assets[0].connectors[0].class = "anchor_end".to_owned();
    pack.assets[1].connectors[0].class = "placed_end".to_owned();
    pack.compatibility_rules = vec![asset_mapper_core::CompatibilityRule {
        a_class: "anchor_end".to_owned(),
        b_class: "placed_end".to_owned(),
        rotation: AllowedRotation::Locked,
    }];
    let plan = load_plan();

    let scene = resolve_plan(&pack, &plan).expect("reverse-order compatibility resolves");

    assert_eq!(scene.placements.len(), 2);
}

#[test]
fn resolves_operations_in_plan_order() {
    let mut pack = load_pack();
    let mut corridor_c = pack.assets[1].clone();
    corridor_c.asset_id = "corridor_c".to_owned();
    corridor_c.display_name = "Corridor Segment C".to_owned();
    corridor_c.source_path = "corridor_c.glb".to_owned();
    corridor_c.content_hash = "sha256:fixture-corridor-c".to_owned();
    pack.assets.push(corridor_c);

    let mut plan = load_plan();
    let mut second_operation = plan.operations[0].clone();
    second_operation.placed_asset_id = "corridor_c".to_owned();
    second_operation.anchor_asset_id = "corridor_b".to_owned();
    second_operation.anchor_connector_id = "back".to_owned();
    plan.operations.push(second_operation);

    let scene = resolve_plan(&pack, &plan).expect("multi-operation plan resolves");

    assert_eq!(scene.placements.len(), 3);
    assert_eq!(scene.placements[0].asset_id, "corridor_a");
    assert_eq!(scene.placements[1].asset_id, "corridor_b");
    assert_eq!(scene.placements[2].asset_id, "corridor_c");
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

fn set_connector_orientation(
    pack: &mut PackRecord,
    asset_id: &str,
    connector_id: &str,
    orientation: [f32; 4],
) {
    let connector = pack
        .assets
        .iter_mut()
        .find(|asset| asset.asset_id == asset_id)
        .expect("asset exists")
        .connectors
        .iter_mut()
        .find(|connector| connector.connector_id == connector_id)
        .expect("connector exists");
    match &mut connector.frame {
        ConnectorFrame::Frame3d {
            orientation_quat_xyzw,
            ..
        } => *orientation_quat_xyzw = orientation,
        ConnectorFrame::Frame2d { .. } => panic!("test connector must be 3D"),
    }
}

fn assert_invalid_connector_orientation(
    error: ResolveError,
    expected_asset_id: &str,
    expected_connector_id: &str,
) {
    assert!(matches!(
        error,
        ResolveError::InvalidConnectorOrientation {
            asset_id,
            connector_id,
        } if asset_id == expected_asset_id && connector_id == expected_connector_id
    ));
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {actual} to be close to {expected}"
    );
}

fn connector_world_axes(
    pack: &PackRecord,
    scene: &ResolvedScene,
    asset_id: &str,
    connector_id: &str,
) -> (Vec3, Vec3) {
    let placement = scene
        .placements
        .iter()
        .find(|placement| placement.asset_id == asset_id)
        .expect("asset has placement");
    let asset = pack
        .assets
        .iter()
        .find(|asset| asset.asset_id == asset_id)
        .expect("asset exists");
    let connector = asset
        .connectors
        .iter()
        .find(|connector| connector.connector_id == connector_id)
        .expect("connector exists");
    let connector_rotation = match &connector.frame {
        ConnectorFrame::Frame3d {
            orientation_quat_xyzw,
            ..
        } => Quat::from_array(*orientation_quat_xyzw).normalize(),
        ConnectorFrame::Frame2d { .. } => panic!("test connector must be 3D"),
    };
    let asset_rotation = Quat::from_array(placement.transform.rotation_quat_xyzw).normalize();
    let world_rotation = (asset_rotation * connector_rotation).normalize();

    (
        world_rotation * axis_to_vec(connector.mating_axis),
        world_rotation * axis_to_vec(connector.up_reference),
    )
}

fn axis_to_vec(axis: Axis3) -> Vec3 {
    match axis {
        Axis3::PosX => Vec3::X,
        Axis3::NegX => -Vec3::X,
        Axis3::PosY => Vec3::Y,
        Axis3::NegY => -Vec3::Y,
        Axis3::PosZ => Vec3::Z,
        Axis3::NegZ => -Vec3::Z,
    }
}

fn assert_vec3_close(actual: Vec3, expected: Vec3) {
    assert!(
        (actual - expected).length() < 0.001,
        "expected {actual:?} to be close to {expected:?}"
    );
}
