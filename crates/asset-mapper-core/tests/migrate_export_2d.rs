use asset_mapper_core::{
    AllowedRotation, AssemblyOperation, AssemblyPlan, AssetRecord, AssetType, Axis3, Bounds3,
    CURRENT_SCHEMA_VERSION, ConnectorClass, ConnectorFrame, ConnectorRecord, ConnectorRole,
    CoordinateConvention, Handedness, PackRecord, Pivot, ResolveError, Unit, bounds_face_snaps,
    duplicate_connector, export_connectors_csv, export_godot, export_unity, export_unreal,
    gltf_keystone_extras, migrate_pack, pack_from_legacy_json, resolve_plan,
    snap_connector_to_nearest_face, suggest_class_from_name,
};

fn sample_pack_v0() -> PackRecord {
    PackRecord {
        schema_version: 0,
        pack_id: "legacy".to_owned(),
        display_name: "Legacy".to_owned(),
        coordinate_convention: CoordinateConvention {
            handedness: Handedness::Right,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
        },
        default_units: Unit::Meters,
        license_summary: "MIT OR Apache-2.0".to_owned(),
        provenance: asset_mapper_core::PackProvenance {
            notes: Some("test fixture".to_owned()),
            ..asset_mapper_core::PackProvenance::default()
        },
        vocabulary: asset_mapper_core::ControlledVocabulary::default(),
        connector_classes: vec![ConnectorClass {
            class: "edge".to_owned(),
            display_name: "Edge".to_owned(),
        }],
        compatibility_rules: vec![],
        assets: vec![AssetRecord {
            asset_id: "tile".to_owned(),
            source_path: "tile.png".to_owned(),
            content_hash: "sha256:x".to_owned(),
            display_name: "Tile".to_owned(),
            asset_type: AssetType::Sprite2d,
            bounds: Bounds3 {
                min: [-0.5, -0.5, -0.5],
                max: [0.5, 0.5, 0.5],
            },
            dimensions: [1.0, 1.0, 1.0],
            pivot: Pivot::Origin,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
            semantic_tags: vec![],
            affordances: vec![],
            placement_constraints: vec![],
            review_flags: vec![],
            connectors: vec![],
        }],
    }
}

#[test]
fn migrates_v0_to_current() {
    let pack = sample_pack_v0();
    let (migrated, report) = migrate_pack(pack).expect("migrates");
    assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(report.from_version, 0);
    assert_eq!(report.to_version, CURRENT_SCHEMA_VERSION);
    assert!(!report.steps.is_empty());
    assert!(
        migrated.assets[0]
            .review_flags
            .contains(&asset_mapper_core::ReviewFlag::BoundsPlaceholder)
    );
    assert!(!migrated.license_summary.is_empty());
    assert!(!migrated.provenance.is_empty());
    assert!(!migrated.vocabulary.semantic_tags.is_empty());
}

#[test]
fn migrates_v1_to_v2_fills_production_metadata() {
    let mut pack = sample_pack_v0();
    pack.schema_version = 1;
    pack.license_summary.clear();
    pack.provenance = asset_mapper_core::PackProvenance::default();
    pack.vocabulary = asset_mapper_core::ControlledVocabulary {
        semantic_tags: vec![],
        affordances: vec![],
        placement_constraints: vec![],
        allow_namespaced_extensions: true,
    };
    let (migrated, report) = migrate_pack(pack).expect("migrates");
    assert_eq!(migrated.schema_version, 2);
    assert!(report.steps.iter().any(|s| s.contains("v1→v2")));
    assert!(!migrated.license_summary.is_empty());
    assert!(!migrated.provenance.is_empty());
    assert!(!migrated.vocabulary.semantic_tags.is_empty());
}

#[test]
fn migrate_rejects_already_current() {
    let mut pack = sample_pack_v0();
    pack.schema_version = CURRENT_SCHEMA_VERSION;
    let err = migrate_pack(pack).expect_err("already current");
    assert!(matches!(
        err,
        asset_mapper_core::MigrationError::AlreadyCurrent { .. }
    ));
}

#[test]
fn engine_exports_include_connectors_and_rules() {
    let mut pack = sample_pack_v0();
    pack.schema_version = CURRENT_SCHEMA_VERSION;
    pack.assets[0].connectors.push(ConnectorRecord {
        connector_id: "n".to_owned(),
        display_name: "North".to_owned(),
        class: "edge".to_owned(),
        role: ConnectorRole::Symmetric,
        frame: ConnectorFrame::Frame3d {
            position: [0.0, 0.5, 0.0],
            orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        mating_axis: Axis3::PosY,
        up_reference: Axis3::PosZ,
        snap_tolerance: 0.01,
        face_size: None,
    });
    pack.compatibility_rules
        .push(asset_mapper_core::CompatibilityRule {
            a_class: "edge".to_owned(),
            b_class: "edge".to_owned(),
            rotation: AllowedRotation::StepsDeg {
                values: vec![0.0, 90.0],
            },
        });

    let unreal = export_unreal(&pack);
    assert_eq!(unreal.connectors.len(), 1);
    assert_eq!(unreal.rules[0].rotation_kind, "steps_deg");
    assert_eq!(unreal.rules[0].rotation_steps_deg, vec![0.0, 90.0]);

    let unity = export_unity(&pack);
    assert_eq!(unity.assets[0].connectors[0].class_name, "edge");

    let godot = export_godot(&pack);
    assert_eq!(godot.resource_type, "KeystonePack");
    assert_eq!(godot.connectors.len(), 1);
    assert_eq!(godot.connectors[0].role, "symmetric");
    assert!((godot.connectors[0].snap_tolerance - 0.01).abs() < f32::EPSILON);

    let extras = gltf_keystone_extras(&pack);
    assert_eq!(extras.schema, "keystone.gltf.extras/v1");
    assert_eq!(extras.assets[0].connectors.len(), 1);
}

#[test]
fn export_connectors_csv_escapes_commas_and_quotes() {
    let mut pack = sample_pack_v0();
    pack.schema_version = CURRENT_SCHEMA_VERSION;
    pack.pack_id = r#"pack,"quoted""#.to_owned();
    pack.assets[0].asset_id = "asset,1".to_owned();
    pack.assets[0].connectors.push(ConnectorRecord {
        connector_id: r#"conn "a""#.to_owned(),
        display_name: "N".to_owned(),
        class: "edge".to_owned(),
        role: ConnectorRole::Plug,
        frame: ConnectorFrame::Frame3d {
            position: [1.0, 2.0, 3.0],
            orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        mating_axis: Axis3::PosZ,
        up_reference: Axis3::PosY,
        snap_tolerance: 0.5,
        face_size: None,
    });

    let csv = export_connectors_csv(&pack);
    assert!(csv.starts_with(
        "pack_id,asset_id,connector_id,class,role,x,y,z,qx,qy,qz,qw,mating_axis,up_reference,snap_tolerance\n"
    ));
    assert!(csv.contains(r#""pack,""quoted"""#));
    assert!(csv.contains(r#""asset,1""#));
    assert!(csv.contains(r#""conn ""a""""#));
    assert!(csv.contains(",plug,"));
}

#[test]
fn resolves_2d_frame_attachment() {
    let pack = PackRecord {
        schema_version: CURRENT_SCHEMA_VERSION,
        pack_id: "tiles".to_owned(),
        display_name: "Tiles".to_owned(),
        coordinate_convention: CoordinateConvention {
            handedness: Handedness::Right,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
        },
        default_units: Unit::Pixels,
        license_summary: "MIT OR Apache-2.0".to_owned(),
        provenance: asset_mapper_core::PackProvenance {
            notes: Some("test fixture".to_owned()),
            ..asset_mapper_core::PackProvenance::default()
        },
        vocabulary: asset_mapper_core::ControlledVocabulary::default(),
        connector_classes: vec![ConnectorClass {
            class: "tile_edge".to_owned(),
            display_name: "Tile Edge".to_owned(),
        }],
        compatibility_rules: vec![asset_mapper_core::CompatibilityRule {
            a_class: "tile_edge".to_owned(),
            b_class: "tile_edge".to_owned(),
            rotation: AllowedRotation::Locked,
        }],
        assets: vec![
            AssetRecord {
                asset_id: "a".to_owned(),
                source_path: "a.png".to_owned(),
                content_hash: "sha256:a".to_owned(),
                display_name: "A".to_owned(),
                asset_type: AssetType::Tile2d,
                bounds: Bounds3 {
                    min: [0.0, 0.0, 0.0],
                    max: [32.0, 32.0, 0.0],
                },
                dimensions: [32.0, 32.0, 0.0],
                pivot: Pivot::Origin,
                up_axis: Axis3::PosY,
                forward_axis: Axis3::PosZ,
                semantic_tags: vec![],
                affordances: vec![],
                placement_constraints: vec![],
                review_flags: vec![],
                connectors: vec![ConnectorRecord {
                    connector_id: "right".to_owned(),
                    display_name: "Right".to_owned(),
                    class: "tile_edge".to_owned(),
                    role: ConnectorRole::Symmetric,
                    frame: ConnectorFrame::Frame2d {
                        position: [32.0, 16.0],
                        normal: [1.0, 0.0],
                        grid_cell: Some([0, 0]),
                    },
                    mating_axis: Axis3::PosX,
                    up_reference: Axis3::PosY,
                    snap_tolerance: 0.5,
                    face_size: None,
                }],
            },
            AssetRecord {
                asset_id: "b".to_owned(),
                source_path: "b.png".to_owned(),
                content_hash: "sha256:b".to_owned(),
                display_name: "B".to_owned(),
                asset_type: AssetType::Tile2d,
                bounds: Bounds3 {
                    min: [0.0, 0.0, 0.0],
                    max: [32.0, 32.0, 0.0],
                },
                dimensions: [32.0, 32.0, 0.0],
                pivot: Pivot::Origin,
                up_axis: Axis3::PosY,
                forward_axis: Axis3::PosZ,
                semantic_tags: vec![],
                affordances: vec![],
                placement_constraints: vec![],
                review_flags: vec![],
                connectors: vec![ConnectorRecord {
                    connector_id: "left".to_owned(),
                    display_name: "Left".to_owned(),
                    class: "tile_edge".to_owned(),
                    role: ConnectorRole::Symmetric,
                    frame: ConnectorFrame::Frame2d {
                        position: [0.0, 16.0],
                        normal: [-1.0, 0.0],
                        grid_cell: Some([1, 0]),
                    },
                    mating_axis: Axis3::NegX,
                    up_reference: Axis3::PosY,
                    snap_tolerance: 0.5,
                    face_size: None,
                }],
            },
        ],
    };

    let plan = AssemblyPlan {
        root_asset_id: "a".to_owned(),
        operations: vec![AssemblyOperation {
            placed_asset_id: "b".to_owned(),
            placed_connector_id: "left".to_owned(),
            anchor_asset_id: "a".to_owned(),
            anchor_connector_id: "right".to_owned(),
            rotation_choice_deg: Some(0.0),
        }],
    };

    let scene = resolve_plan(&pack, &plan).expect("2d plan resolves");
    assert_eq!(scene.placements.len(), 2);
    // B's left edge (local x=0) should land on A's right edge world x=32
    let b = &scene.placements[1];
    assert!((b.transform.translation[0] - 32.0).abs() < 0.01);
    assert!(b.transform.translation[1].abs() < 0.01);
}

fn two_tile_pack(rotation: AllowedRotation) -> PackRecord {
    PackRecord {
        schema_version: CURRENT_SCHEMA_VERSION,
        pack_id: "tiles".to_owned(),
        display_name: "Tiles".to_owned(),
        coordinate_convention: CoordinateConvention {
            handedness: Handedness::Right,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
        },
        default_units: Unit::Pixels,
        license_summary: "MIT OR Apache-2.0".to_owned(),
        provenance: asset_mapper_core::PackProvenance {
            notes: Some("test fixture".to_owned()),
            ..asset_mapper_core::PackProvenance::default()
        },
        vocabulary: asset_mapper_core::ControlledVocabulary::default(),
        connector_classes: vec![ConnectorClass {
            class: "tile_edge".to_owned(),
            display_name: "Tile Edge".to_owned(),
        }],
        compatibility_rules: vec![asset_mapper_core::CompatibilityRule {
            a_class: "tile_edge".to_owned(),
            b_class: "tile_edge".to_owned(),
            rotation,
        }],
        assets: vec![
            AssetRecord {
                asset_id: "a".to_owned(),
                source_path: "a.png".to_owned(),
                content_hash: "sha256:a".to_owned(),
                display_name: "A".to_owned(),
                asset_type: AssetType::Tile2d,
                bounds: Bounds3 {
                    min: [0.0, 0.0, 0.0],
                    max: [32.0, 32.0, 0.0],
                },
                dimensions: [32.0, 32.0, 0.0],
                pivot: Pivot::Origin,
                up_axis: Axis3::PosY,
                forward_axis: Axis3::PosZ,
                semantic_tags: vec![],
                affordances: vec![],
                placement_constraints: vec![],
                review_flags: vec![],
                connectors: vec![ConnectorRecord {
                    connector_id: "right".to_owned(),
                    display_name: "Right".to_owned(),
                    class: "tile_edge".to_owned(),
                    role: ConnectorRole::Symmetric,
                    frame: ConnectorFrame::Frame2d {
                        position: [32.0, 16.0],
                        normal: [1.0, 0.0],
                        grid_cell: None,
                    },
                    mating_axis: Axis3::PosX,
                    up_reference: Axis3::PosY,
                    snap_tolerance: 0.5,
                    face_size: None,
                }],
            },
            AssetRecord {
                asset_id: "b".to_owned(),
                source_path: "b.png".to_owned(),
                content_hash: "sha256:b".to_owned(),
                display_name: "B".to_owned(),
                asset_type: AssetType::Tile2d,
                bounds: Bounds3 {
                    min: [0.0, 0.0, 0.0],
                    max: [32.0, 32.0, 0.0],
                },
                dimensions: [32.0, 32.0, 0.0],
                pivot: Pivot::Origin,
                up_axis: Axis3::PosY,
                forward_axis: Axis3::PosZ,
                semantic_tags: vec![],
                affordances: vec![],
                placement_constraints: vec![],
                review_flags: vec![],
                connectors: vec![ConnectorRecord {
                    connector_id: "left".to_owned(),
                    display_name: "Left".to_owned(),
                    class: "tile_edge".to_owned(),
                    role: ConnectorRole::Symmetric,
                    frame: ConnectorFrame::Frame2d {
                        position: [0.0, 16.0],
                        normal: [-1.0, 0.0],
                        grid_cell: None,
                    },
                    mating_axis: Axis3::NegX,
                    up_reference: Axis3::PosY,
                    snap_tolerance: 0.5,
                    face_size: None,
                }],
            },
        ],
    }
}

#[test]
fn resolves_2d_with_ninety_degree_rotation_choice() {
    let pack = two_tile_pack(AllowedRotation::StepsDeg {
        values: vec![0.0, 90.0],
    });
    let plan = AssemblyPlan {
        root_asset_id: "a".to_owned(),
        operations: vec![AssemblyOperation {
            placed_asset_id: "b".to_owned(),
            placed_connector_id: "left".to_owned(),
            anchor_asset_id: "a".to_owned(),
            anchor_connector_id: "right".to_owned(),
            rotation_choice_deg: Some(90.0),
        }],
    };
    let scene = resolve_plan(&pack, &plan).expect("2d 90° resolves");
    let b = &scene.placements[1];
    let q = b.transform.rotation_quat_xyzw;
    // Significant Z rotation expected for 90°.
    assert!(
        q[2].abs() > 0.5,
        "expected significant Z rotation for 90°, got {q:?}"
    );
    // Connector co-location: rotate local left (0,16) by resolved Z angle, then translate.
    let angle = 2.0 * q[2].atan2(q[3]);
    let (s, c) = angle.sin_cos();
    let (lx, ly) = (0.0_f32, 16.0_f32);
    let rx = c * lx - s * ly;
    let ry = s * lx + c * ly;
    let wx = b.transform.translation[0] + rx;
    let wy = b.transform.translation[1] + ry;
    assert!(
        (wx - 32.0).abs() < 0.1 && (wy - 16.0).abs() < 0.1,
        "connectors should co-locate, got world ({wx}, {wy}), t={:?}",
        b.transform.translation
    );
}

#[test]
fn rejects_non_finite_2d_connector_at_resolve() {
    let mut pack = two_tile_pack(AllowedRotation::Locked);
    if let ConnectorFrame::Frame2d { position, .. } = &mut pack.assets[1].connectors[0].frame {
        position[0] = f32::NAN;
    }
    let plan = AssemblyPlan {
        root_asset_id: "a".to_owned(),
        operations: vec![AssemblyOperation {
            placed_asset_id: "b".to_owned(),
            placed_connector_id: "left".to_owned(),
            anchor_asset_id: "a".to_owned(),
            anchor_connector_id: "right".to_owned(),
            rotation_choice_deg: Some(0.0),
        }],
    };
    let err = resolve_plan(&pack, &plan).expect_err("nan should fail");
    assert!(matches!(err, ResolveError::Invalid2dNormal { .. }));
}

#[test]
fn steps_deg_accepts_360_as_equivalent_to_0() {
    let pack = two_tile_pack(AllowedRotation::StepsDeg {
        values: vec![0.0, 90.0],
    });
    let plan = AssemblyPlan {
        root_asset_id: "a".to_owned(),
        operations: vec![AssemblyOperation {
            placed_asset_id: "b".to_owned(),
            placed_connector_id: "left".to_owned(),
            anchor_asset_id: "a".to_owned(),
            anchor_connector_id: "right".to_owned(),
            rotation_choice_deg: Some(360.0),
        }],
    };
    resolve_plan(&pack, &plan).expect("360 ≡ 0 for steps");
}

#[test]
fn migrate_rejects_future_schema_version() {
    let mut pack = sample_pack_v0();
    pack.schema_version = 99;
    let err = migrate_pack(pack).expect_err("future unsupported");
    assert!(matches!(
        err,
        asset_mapper_core::MigrationError::UnsupportedVersion { found: 99, .. }
    ));
}

#[test]
fn pack_from_legacy_json_defaults_missing_schema_and_arrays() {
    let value = serde_json::json!({
        "pack_id": "legacy",
        "display_name": "Legacy",
        "coordinate_convention": {
            "handedness": "right",
            "up_axis": "pos_y",
            "forward_axis": "pos_z"
        },
        "default_units": "meters",
        "connector_classes": [],
        "compatibility_rules": [],
        "assets": [{
            "asset_id": "a",
            "source_path": "a.glb",
            "content_hash": "sha256:x",
            "display_name": "A",
            "asset_type": "model3d",
            "bounds": { "min": [-0.5, -0.5, -0.5], "max": [0.5, 0.5, 0.5] },
            "dimensions": [1.0, 1.0, 1.0],
            "pivot": "origin",
            "up_axis": "pos_y",
            "forward_axis": "pos_z"
        }]
    });
    let pack = pack_from_legacy_json(value).expect("legacy loads");
    assert_eq!(pack.schema_version, 0);
    assert!(pack.assets[0].review_flags.is_empty());
    assert!(pack.assets[0].connectors.is_empty());
}

#[test]
fn authoring_helpers_suggest_snap_duplicate() {
    assert_eq!(
        suggest_class_from_name("Stone Wall Piece"),
        Some("wall_edge".to_owned())
    );
    assert_eq!(
        suggest_class_from_name("Wooden Door"),
        Some("doorway".to_owned())
    );

    let bounds = Bounds3 {
        min: [-1.0, 0.0, -1.0],
        max: [1.0, 2.0, 1.0],
    };
    let faces = bounds_face_snaps(&bounds);
    assert_eq!(faces.len(), 6);

    let mut connector = ConnectorRecord {
        connector_id: "c".to_owned(),
        display_name: "C".to_owned(),
        class: "wall_edge".to_owned(),
        role: ConnectorRole::Symmetric,
        frame: ConnectorFrame::Frame3d {
            position: [0.9, 1.0, 0.0],
            orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        mating_axis: Axis3::PosZ,
        up_reference: Axis3::PosY,
        snap_tolerance: 0.01,
        face_size: None,
    };
    snap_connector_to_nearest_face(&mut connector, &bounds);
    if let ConnectorFrame::Frame3d {
        position,
        orientation_quat_xyzw,
    } = connector.frame
    {
        assert!((position[0] - 1.0).abs() < 0.001);
        // Local +Z faces outward; identity would mean +Z outward which is wrong for +X face.
        assert!(
            orientation_quat_xyzw != [0.0, 0.0, 0.0, 1.0],
            "pos_x face should rotate local +Z to +X"
        );
    } else {
        panic!("expected 3d frame");
    }
    assert_eq!(connector.mating_axis, Axis3::PosZ);
    assert_eq!(connector.up_reference, Axis3::PosY);

    let pos_z_face = faces.iter().find(|f| f.name == "pos_z").expect("pos_z");
    assert_eq!(pos_z_face.orientation_quat_xyzw, [0.0, 0.0, 0.0, 1.0]);

    let dup = duplicate_connector(&connector, "c2".to_owned());
    assert_eq!(dup.connector_id, "c2");
    assert_ne!(dup.frame, connector.frame);
}
