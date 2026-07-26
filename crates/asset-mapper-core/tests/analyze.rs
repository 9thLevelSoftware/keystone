use asset_mapper_core::{
    AnalyzeOptions, AssetRecord, AssetType, Axis3, Bounds3, CURRENT_SCHEMA_VERSION,
    ControlledVocabulary, CoordinateConvention, Handedness, PackProvenance, PackRecord, Pivot,
    Unit, analyze_pack,
};

fn base_pack() -> PackRecord {
    PackRecord {
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
        assets: vec![
            AssetRecord {
                asset_id: "wall_a".to_owned(),
                source_path: "wall.glb".to_owned(),
                content_hash: "sha256:a".to_owned(),
                display_name: "Wall A".to_owned(),
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
            },
            AssetRecord {
                asset_id: "wall_b".to_owned(),
                source_path: "wall_b.glb".to_owned(),
                content_hash: "sha256:b".to_owned(),
                display_name: "Wall B".to_owned(),
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
            },
        ],
    }
}

#[test]
fn analyze_proposes_connectors_and_rules() {
    let mut pack = base_pack();
    let report = analyze_pack(&mut pack, &AnalyzeOptions::default());
    assert!(report.connectors_added >= 2);
    assert!(!pack.assets[0].connectors.is_empty());
    assert!(!pack.connector_classes.is_empty());
    assert!(!pack.compatibility_rules.is_empty());
    assert!(
        pack.assets[0]
            .connectors
            .iter()
            .all(|c| !c.class.is_empty())
    );
}

#[test]
fn analyze_skips_assets_with_connectors_unless_replace() {
    let mut pack = base_pack();
    analyze_pack(&mut pack, &AnalyzeOptions::default());
    let first_count = pack.assets[0].connectors.len();
    let report = analyze_pack(&mut pack, &AnalyzeOptions::default());
    assert_eq!(report.connectors_added, 0);
    assert_eq!(pack.assets[0].connectors.len(), first_count);

    let report2 = analyze_pack(
        &mut pack,
        &AnalyzeOptions {
            replace_existing_connectors: true,
            ..AnalyzeOptions::default()
        },
    );
    assert!(report2.connectors_added > 0);
}
