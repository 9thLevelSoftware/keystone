//! P0-07: Prove model-style assembly plans resolve against the phase0 pack.

use asset_mapper_core::{AssemblyPlan, PackRecord, ResolveError, resolve_plan};

fn load_pack() -> PackRecord {
    let input = std::fs::read_to_string(format!(
        "{}/../../fixtures/phase0/simple_pack.assetmap.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture pack can be read");
    serde_json::from_str(&input).expect("fixture pack parses")
}

fn load_plan(relative: &str) -> AssemblyPlan {
    let input = std::fs::read_to_string(format!(
        "{}/../../fixtures/phase0/{relative}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture plan can be read");
    serde_json::from_str(&input).expect("fixture plan parses")
}

#[test]
fn llm_style_plan_resolves_against_phase0_pack() {
    let pack = load_pack();
    let plan = load_plan("llm_style_plan.json");

    let scene = resolve_plan(&pack, &plan).expect("LLM-style plan resolves");

    assert_eq!(scene.placements.len(), 2);
    assert_eq!(scene.placements[0].asset_id, "corridor_a");
    assert_eq!(scene.placements[1].asset_id, "corridor_b");
    let z = scene.placements[1].transform.translation[2];
    assert!(
        (z - 2.0).abs() < 0.001,
        "expected corridor_b at z≈2, got {z}"
    );
}

#[test]
fn llm_style_plan_rejects_disallowed_rotation() {
    let pack = load_pack();
    let plan = load_plan("llm_style_plan_invalid_class.json");

    let error = resolve_plan(&pack, &plan).expect_err("locked rule rejects 90°");

    assert!(matches!(
        error,
        ResolveError::RotationChoiceNotAllowed { choice, .. } if (choice - 90.0).abs() < 0.001
    ));
}

#[test]
fn llm_style_plan_rejects_unknown_asset_variant() {
    let pack = load_pack();
    let mut plan = load_plan("llm_style_plan.json");
    plan.operations[0].placed_asset_id = "hallucinated_piece".to_owned();

    let error = resolve_plan(&pack, &plan).expect_err("unknown placed asset fails");

    assert!(matches!(
        error,
        ResolveError::UnknownPlacedAsset { asset_id } if asset_id == "hallucinated_piece"
    ));
}
