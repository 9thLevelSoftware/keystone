use asset_mapper_core::{
    AnalyzeOptions, AssetRecord, AssetType, Axis3, Bounds3, CURRENT_SCHEMA_VERSION,
    ControlledVocabulary, CoordinateConvention, Handedness, PackProvenance, PackRecord, Pivot,
    Unit, analyze_pack, suggest_semantics_for_asset,
};

fn wall_asset(id: &str, name: &str) -> AssetRecord {
    AssetRecord {
        asset_id: id.to_owned(),
        source_path: format!("{id}.glb"),
        content_hash: "sha256:x".to_owned(),
        display_name: name.to_owned(),
        asset_type: AssetType::Model3d,
        bounds: Bounds3 {
            min: [-1.0, 0.0, -0.1],
            max: [1.0, 2.5, 0.1],
        },
        dimensions: [2.0, 2.5, 0.2],
        pivot: Pivot::Origin,
        up_axis: Axis3::PosY,
        forward_axis: Axis3::PosZ,
        semantic_tags: vec![],
        affordances: vec![],
        placement_constraints: vec![],
        review_flags: vec![],
        connectors: vec![],
    }
}

#[test]
fn default_vocab_suggests_wall_terms_for_wall_door_name() {
    let asset = wall_asset("wall_door", "Wall Door Opening");
    let vocab = ControlledVocabulary::default();
    let suggested = suggest_semantics_for_asset(&asset, &["doorway".to_owned()], &vocab);
    assert!(
        suggested
            .semantic_tags
            .iter()
            .any(|t| t == "wall" || t == "door"),
        "tags={:?}",
        suggested.semantic_tags
    );
    assert!(
        suggested
            .affordances
            .iter()
            .any(|a| a == "block_movement" || a == "openable"),
        "affordances={:?}",
        suggested.affordances
    );
    // All terms must be in vocab lists (or namespaced).
    for t in &suggested.semantic_tags {
        assert!(
            vocab.allows_term(&vocab.semantic_tags, t),
            "out of vocab tag {t}"
        );
    }
}

#[test]
fn empty_semantic_vocab_yields_no_tags() {
    let asset = wall_asset("wall_door", "Wall Door");
    let vocab = ControlledVocabulary {
        semantic_tags: vec![],
        affordances: ControlledVocabulary::default().affordances,
        placement_constraints: ControlledVocabulary::default().placement_constraints,
        allow_namespaced_extensions: false,
    };
    let suggested = suggest_semantics_for_asset(&asset, &["wall_edge".to_owned()], &vocab);
    assert!(
        suggested.semantic_tags.is_empty(),
        "expected no tags with empty vocab list, got {:?}",
        suggested.semantic_tags
    );
}

#[test]
fn analyze_preserves_existing_semantics() {
    let mut pack = PackRecord {
        schema_version: CURRENT_SCHEMA_VERSION,
        pack_id: "kit".to_owned(),
        display_name: "Kit".to_owned(),
        coordinate_convention: CoordinateConvention {
            handedness: Handedness::Right,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
        },
        default_units: Unit::Meters,
        license_summary: "MIT".to_owned(),
        provenance: PackProvenance {
            author: Some("Test".to_owned()),
            ..PackProvenance::default()
        },
        vocabulary: ControlledVocabulary::default(),
        connector_classes: vec![],
        compatibility_rules: vec![],
        assets: vec![wall_asset("wall_a", "Wall A")],
    };
    pack.assets[0].semantic_tags = vec!["prop".to_owned()];
    pack.assets[0].affordances = vec!["interactable".to_owned()];
    pack.assets[0].placement_constraints = vec!["indoor_only".to_owned()];

    analyze_pack(
        &mut pack,
        &AnalyzeOptions {
            replace_existing_connectors: true,
            ..AnalyzeOptions::default()
        },
    );

    assert_eq!(pack.assets[0].semantic_tags, vec!["prop".to_owned()]);
    assert_eq!(pack.assets[0].affordances, vec!["interactable".to_owned()]);
    assert_eq!(
        pack.assets[0].placement_constraints,
        vec!["indoor_only".to_owned()]
    );
}
