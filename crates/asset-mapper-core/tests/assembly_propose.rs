use asset_mapper_core::{
    AllowedRotation, AnalyzeOptions, AssemblyOperation, AssemblyPlan, AssetRecord, AssetType,
    Axis3, Bounds3, CURRENT_SCHEMA_VERSION, CompatibilityRule, ConnectorClass, ConnectorFrame,
    ConnectorRecord, ConnectorRole, ControlledVocabulary, CoordinateConvention, Handedness,
    PackProvenance, PackRecord, Pivot, ProposeAssemblyOptions, Unit, analyze_pack,
    propose_assembly_plan, resolve_plan,
};

fn pack_with_three_walls() -> PackRecord {
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
        connector_classes: vec![ConnectorClass {
            class: "wall_edge".to_owned(),
            display_name: "Wall Edge".to_owned(),
        }],
        compatibility_rules: vec![CompatibilityRule {
            a_class: "wall_edge".to_owned(),
            b_class: "wall_edge".to_owned(),
            rotation: AllowedRotation::StepsDeg {
                values: vec![0.0, 90.0, 180.0, 270.0],
            },
        }],
        assets: vec![],
    };

    for (id, x) in [("wall_a", 0.0), ("wall_b", 2.0), ("wall_c", 4.0)] {
        pack.assets.push(AssetRecord {
            asset_id: id.to_owned(),
            source_path: format!("{id}.glb"),
            content_hash: format!("sha256:{id}"),
            display_name: id.to_owned(),
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
            connectors: vec![
                ConnectorRecord {
                    connector_id: format!("{id}_neg_x"),
                    display_name: "Neg X".to_owned(),
                    class: "wall_edge".to_owned(),
                    role: ConnectorRole::Symmetric,
                    frame: ConnectorFrame::Frame3d {
                        position: [-1.0 + x * 0.0, 1.25, 0.0],
                        orientation_quat_xyzw: [
                            0.0,
                            std::f32::consts::FRAC_1_SQRT_2,
                            0.0,
                            std::f32::consts::FRAC_1_SQRT_2,
                        ],
                    },
                    mating_axis: Axis3::PosZ,
                    up_reference: Axis3::PosY,
                    snap_tolerance: 0.01,
                    face_size: None,
                },
                ConnectorRecord {
                    connector_id: format!("{id}_pos_x"),
                    display_name: "Pos X".to_owned(),
                    class: "wall_edge".to_owned(),
                    role: ConnectorRole::Symmetric,
                    frame: ConnectorFrame::Frame3d {
                        position: [1.0, 1.25, 0.0],
                        orientation_quat_xyzw: [
                            0.0,
                            -std::f32::consts::FRAC_1_SQRT_2,
                            0.0,
                            std::f32::consts::FRAC_1_SQRT_2,
                        ],
                    },
                    mating_axis: Axis3::PosZ,
                    up_reference: Axis3::PosY,
                    snap_tolerance: 0.01,
                    face_size: None,
                },
            ],
        });
    }
    pack
}

#[test]
fn propose_connects_multiple_pieces() {
    let pack = pack_with_three_walls();
    let report = propose_assembly_plan(
        &pack,
        &ProposeAssemblyOptions {
            max_pieces: 3,
            root_asset_id: Some("wall_a".to_owned()),
            ..ProposeAssemblyOptions::default()
        },
    );
    assert!(
        report.placed_asset_ids.len() >= 2,
        "expected multi-piece plan, got {:?}",
        report
    );
    let scene = resolve_plan(&pack, &report.plan).expect("proposed plan resolves");
    assert!(scene.placements.len() >= 2);
}

#[test]
fn analyze_then_propose_on_empty_kit() {
    let mut pack = pack_with_three_walls();
    for asset in &mut pack.assets {
        asset.connectors.clear();
    }
    pack.compatibility_rules.clear();
    pack.connector_classes.clear();
    analyze_pack(&mut pack, &AnalyzeOptions::default());
    assert!(pack.assets.iter().any(|a| !a.connectors.is_empty()));
    let report = propose_assembly_plan(&pack, &ProposeAssemblyOptions::default());
    // May place 1+ depending on rules; should not panic.
    let _ = report.plan;
}

#[test]
fn wall_door_cross_rules_from_analyze() {
    let mut pack = pack_with_three_walls();
    pack.assets.push(AssetRecord {
        asset_id: "door_01".to_owned(),
        source_path: "door.glb".to_owned(),
        content_hash: "sha256:door".to_owned(),
        display_name: "Door Unit".to_owned(),
        asset_type: AssetType::Model3d,
        bounds: Bounds3 {
            min: [-0.5, 0.0, -0.05],
            max: [0.5, 2.1, 0.05],
        },
        dimensions: [1.0, 2.1, 0.1],
        pivot: Pivot::Origin,
        up_axis: Axis3::PosY,
        forward_axis: Axis3::PosZ,
        semantic_tags: vec![],
        affordances: vec![],
        placement_constraints: vec![],
        review_flags: vec![],
        connectors: vec![],
    });
    for asset in &mut pack.assets {
        asset.connectors.clear();
    }
    pack.compatibility_rules.clear();
    pack.connector_classes.clear();
    analyze_pack(
        &mut pack,
        &AnalyzeOptions {
            replace_existing_connectors: true,
            ..AnalyzeOptions::default()
        },
    );
    let has_cross = pack.compatibility_rules.iter().any(|r| {
        (r.a_class == "doorway" && r.b_class == "wall_edge")
            || (r.a_class == "wall_edge" && r.b_class == "doorway")
    });
    // Door name suggests doorway class; walls suggest wall_edge.
    let door_classes: Vec<_> = pack
        .assets
        .iter()
        .find(|a| a.asset_id == "door_01")
        .map(|a| a.connectors.iter().map(|c| c.class.clone()).collect())
        .unwrap_or_default();
    assert!(
        door_classes.iter().any(|c| c == "doorway") || has_cross,
        "expected doorway class or cross rule; door classes={door_classes:?} rules={:?}",
        pack.compatibility_rules
    );
}

#[test]
fn pick_root_avoids_lonely_self_rule_class() {
    // One window-only piece with 2 connectors must not beat a wall that can mate.
    let mut pack = pack_with_three_walls();
    pack.assets.push(AssetRecord {
        asset_id: "lonely_window".to_owned(),
        source_path: "window.glb".to_owned(),
        content_hash: "sha256:w".to_owned(),
        display_name: "Lonely Window".to_owned(),
        asset_type: AssetType::Model3d,
        bounds: Bounds3 {
            min: [-0.5, 1.0, -0.05],
            max: [0.5, 1.8, 0.05],
        },
        dimensions: [1.0, 0.8, 0.1],
        pivot: Pivot::Origin,
        up_axis: Axis3::PosY,
        forward_axis: Axis3::PosZ,
        semantic_tags: vec![],
        affordances: vec![],
        placement_constraints: vec![],
        review_flags: vec![],
        connectors: vec![
            ConnectorRecord {
                connector_id: "win_a".to_owned(),
                display_name: "A".to_owned(),
                class: "window_frame".to_owned(),
                role: ConnectorRole::Symmetric,
                frame: ConnectorFrame::Frame3d {
                    position: [0.0, 1.4, 0.05],
                    orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
                },
                mating_axis: Axis3::PosZ,
                up_reference: Axis3::PosY,
                snap_tolerance: 0.01,
                face_size: Some([1.0, 0.8]),
            },
            ConnectorRecord {
                connector_id: "win_b".to_owned(),
                display_name: "B".to_owned(),
                class: "window_frame".to_owned(),
                role: ConnectorRole::Symmetric,
                frame: ConnectorFrame::Frame3d {
                    position: [0.0, 1.4, -0.05],
                    orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
                },
                mating_axis: Axis3::PosZ,
                up_reference: Axis3::PosY,
                snap_tolerance: 0.01,
                face_size: Some([1.0, 0.8]),
            },
        ],
    });
    pack.connector_classes.push(ConnectorClass {
        class: "window_frame".to_owned(),
        display_name: "Window".to_owned(),
    });
    pack.compatibility_rules.push(CompatibilityRule {
        a_class: "window_frame".to_owned(),
        b_class: "window_frame".to_owned(),
        rotation: AllowedRotation::Locked,
    });

    let report = propose_assembly_plan(&pack, &ProposeAssemblyOptions::default());
    assert_ne!(
        report.plan.root_asset_id, "lonely_window",
        "root should not be a class with no multi-asset mates: {:?}",
        report
    );
    assert!(
        report.placed_asset_ids.len() >= 2,
        "expected multi-piece plan from walls, got {:?}",
        report
    );
}

#[test]
fn cross_class_face_size_mismatch_still_mates() {
    // Full wall face vs smaller doorway portal must still propose an attach.
    let mut pack = pack_with_three_walls();
    pack.assets.truncate(1);
    pack.assets[0].connectors[0].face_size = Some([2.0, 2.5]);
    pack.assets[0].connectors[1].face_size = Some([2.0, 2.5]);
    pack.connector_classes.push(ConnectorClass {
        class: "doorway".to_owned(),
        display_name: "Door".to_owned(),
    });
    pack.compatibility_rules.push(CompatibilityRule {
        a_class: "doorway".to_owned(),
        b_class: "wall_edge".to_owned(),
        rotation: AllowedRotation::Locked,
    });
    pack.assets.push(AssetRecord {
        asset_id: "door_unit".to_owned(),
        source_path: "door.glb".to_owned(),
        content_hash: "sha256:d".to_owned(),
        display_name: "Door".to_owned(),
        asset_type: AssetType::Model3d,
        bounds: Bounds3 {
            min: [-0.4, 0.0, -0.05],
            max: [0.4, 2.0, 0.05],
        },
        dimensions: [0.8, 2.0, 0.1],
        pivot: Pivot::Origin,
        up_axis: Axis3::PosY,
        forward_axis: Axis3::PosZ,
        semantic_tags: vec![],
        affordances: vec![],
        placement_constraints: vec![],
        review_flags: vec![],
        connectors: vec![ConnectorRecord {
            connector_id: "door_front".to_owned(),
            display_name: "Front".to_owned(),
            class: "doorway".to_owned(),
            role: ConnectorRole::Symmetric,
            frame: ConnectorFrame::Frame3d {
                position: [0.0, 1.0, 0.05],
                orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
            },
            mating_axis: Axis3::PosZ,
            up_reference: Axis3::PosY,
            snap_tolerance: 0.01,
            face_size: Some([0.8, 2.0]), // much smaller than wall face
        }],
    });

    let report = propose_assembly_plan(
        &pack,
        &ProposeAssemblyOptions {
            root_asset_id: Some("wall_a".to_owned()),
            max_pieces: 2,
            ..ProposeAssemblyOptions::default()
        },
    );
    assert!(
        report.placed_asset_ids.len() >= 2,
        "cross-class size mismatch should not block: {:?}",
        report
    );
}

#[test]
fn reuse_flags_emit_honesty_note_without_duplicate_assets() {
    let pack = pack_with_three_walls();
    let report = propose_assembly_plan(
        &pack,
        &ProposeAssemblyOptions {
            max_pieces: 8,
            allow_asset_reuse: true,
            max_instances_per_asset: 4,
            ..ProposeAssemblyOptions::default()
        },
    );
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("not applied") || n.contains("once")),
        "expected honesty note, got {:?}",
        report.notes
    );
    let mut ids = report.placed_asset_ids.clone();
    ids.sort();
    ids.dedup();
    assert_eq!(
        ids.len(),
        report.placed_asset_ids.len(),
        "each asset_id at most once: {:?}",
        report.placed_asset_ids
    );
}

#[test]
fn face_size_accepts_swapped_uv() {
    // Axis-aligned mate: one edge publishes [2.0, 2.5], the other [2.5, 2.0] (90° UV swap).
    let mut pack = pack_with_three_walls();
    for asset in &mut pack.assets {
        for c in &mut asset.connectors {
            c.face_size = Some([2.0, 2.0]);
        }
    }
    pack.assets[0].connectors[0].face_size = Some([2.0, 2.4]);
    pack.assets[1].connectors[0].face_size = Some([2.4, 2.0]);
    // Without UV-swap tolerance, axis-aligned compare would need ratio 2.4/2.0 = 1.2 on both axes;
    // with swap, both axes ratio 1.0. Tighten max so swap is required for a match.
    let report = propose_assembly_plan(
        &pack,
        &ProposeAssemblyOptions {
            max_pieces: 2,
            size_ratio_max: 1.05,
            ..ProposeAssemblyOptions::default()
        },
    );
    assert!(
        report.placed_asset_ids.len() >= 2,
        "swapped face_size should still mate: {:?}",
        report
    );
}

#[test]
fn empty_connectors_report() {
    let pack = PackRecord {
        schema_version: CURRENT_SCHEMA_VERSION,
        pack_id: "empty".to_owned(),
        display_name: "Empty".to_owned(),
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
        assets: vec![],
    };
    let report = propose_assembly_plan(&pack, &ProposeAssemblyOptions::default());
    assert!(report.plan.operations.is_empty());
    assert!(!report.notes.is_empty());
}

// silence unused import warnings in some rustc versions
#[allow(dead_code)]
fn _types() -> Option<AssemblyOperation> {
    let _p: AssemblyPlan = AssemblyPlan {
        root_asset_id: String::new(),
        operations: vec![],
    };
    None
}
