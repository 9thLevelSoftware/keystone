use asset_mapper_core::{
    AnalyzeOptions, AssetRecord, AssetType, Axis3, Bounds3, CURRENT_SCHEMA_VERSION,
    ControlledVocabulary, CoordinateConvention, Handedness, MeshGeometry, PackProvenance,
    PackRecord, Pivot, Unit, analyze_pack_with_meshes, suggest_class_from_asset,
};
use std::collections::BTreeMap;

fn asset(id: &str, path: &str, bounds: Bounds3) -> AssetRecord {
    let dims = [
        (bounds.max[0] - bounds.min[0]).abs(),
        (bounds.max[1] - bounds.min[1]).abs(),
        (bounds.max[2] - bounds.min[2]).abs(),
    ];
    AssetRecord {
        asset_id: id.to_owned(),
        source_path: path.to_owned(),
        content_hash: "sha256:x".to_owned(),
        display_name: id.to_owned(),
        asset_type: AssetType::Model3d,
        bounds,
        dimensions: dims,
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

fn pack_with(assets: Vec<AssetRecord>) -> PackRecord {
    PackRecord {
        schema_version: CURRENT_SCHEMA_VERSION,
        pack_id: "t".to_owned(),
        display_name: "T".to_owned(),
        coordinate_convention: CoordinateConvention {
            handedness: Handedness::Right,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
        },
        default_units: Unit::Meters,
        license_summary: "MIT".to_owned(),
        provenance: PackProvenance {
            author: Some("t".to_owned()),
            ..PackProvenance::default()
        },
        vocabulary: ControlledVocabulary::default(),
        connector_classes: vec![],
        compatibility_rules: vec![],
        assets,
    }
}

#[test]
fn path_walls_folder_suggests_wall_edge() {
    let a = asset(
        "wallastra",
        "Walls/WallAstra_Straight.gltf",
        Bounds3 {
            min: [-0.5, 0.0, -2.0],
            max: [0.5, 3.0, 2.0],
        },
    );
    assert_eq!(
        suggest_class_from_asset(&a).as_deref(),
        Some("wall_edge")
    );
}

#[test]
fn path_door_frame_suggests_doorway() {
    let a = asset(
        "door",
        "Platforms/Door_Frame_Square.gltf",
        Bounds3 {
            min: [-2.0, 0.0, -0.2],
            max: [2.0, 5.0, 0.2],
        },
    );
    assert_eq!(suggest_class_from_asset(&a).as_deref(), Some("doorway"));
}

#[test]
fn path_platform_suggests_floor_edge() {
    let a = asset(
        "plat",
        "Platforms/Platform_Simple.gltf",
        Bounds3 {
            min: [-2.0, 0.0, -2.0],
            max: [2.0, 0.2, 2.0],
        },
    );
    assert_eq!(
        suggest_class_from_asset(&a).as_deref(),
        Some("floor_edge")
    );
}

#[test]
fn wall_with_mesh_does_not_become_all_doorway() {
    // Box mesh: surface samples only, no real portal.
    let bounds = Bounds3 {
        min: [-0.5, 0.0, -2.0],
        max: [0.5, 3.0, 2.0],
    };
    let mut positions = Vec::new();
    for x in [bounds.min[0], bounds.max[0]] {
        for y in [bounds.min[1], bounds.max[1]] {
            for z in [bounds.min[2], bounds.max[2]] {
                positions.push([x, y, z]);
            }
        }
    }
    // Dense +Z face samples (wall face) without a big hole.
    let z = bounds.max[2];
    for i in 0..12 {
        for j in 0..12 {
            let u = bounds.min[0] + (i as f32 / 11.0) * (bounds.max[0] - bounds.min[0]);
            let v = bounds.min[1] + (j as f32 / 11.0) * (bounds.max[1] - bounds.min[1]);
            positions.push([u, v, z]);
        }
    }
    let mesh = MeshGeometry {
        positions,
        indices: None,
    };
    let mut pack = pack_with(vec![asset(
        "walls_wallastra_straight",
        "Walls/WallAstra_Straight.gltf",
        bounds,
    )]);
    let mut meshes = BTreeMap::new();
    meshes.insert("walls_wallastra_straight".to_owned(), mesh);
    analyze_pack_with_meshes(
        &mut pack,
        &AnalyzeOptions {
            replace_existing_connectors: true,
            ..AnalyzeOptions::default()
        },
        &meshes,
    );
    let a = &pack.assets[0];
    assert!(!a.connectors.is_empty());
    let doorway_n = a
        .connectors
        .iter()
        .filter(|c| c.class == "doorway")
        .count();
    let wall_n = a
        .connectors
        .iter()
        .filter(|c| c.class == "wall_edge")
        .count();
    assert!(
        wall_n >= doorway_n,
        "expected wall_edge majority, got wall={wall_n} doorway={doorway_n} all={:?}",
        a.connectors
            .iter()
            .map(|c| &c.class)
            .collect::<Vec<_>>()
    );
}

#[test]
fn exclude_glob_skips_decals() {
    let mut pack = pack_with(vec![
        asset(
            "decal",
            "Decals/Decal_0.gltf",
            Bounds3 {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 1.0, 0.1],
            },
        ),
        asset(
            "wall",
            "Walls/Wall_Simple.gltf",
            Bounds3 {
                min: [-1.0, 0.0, -0.1],
                max: [1.0, 2.5, 0.1],
            },
        ),
    ]);
    analyze_pack_with_meshes(
        &mut pack,
        &AnalyzeOptions {
            replace_existing_connectors: true,
            exclude_globs: vec!["Decals/**".to_owned()],
            ..AnalyzeOptions::default()
        },
        &BTreeMap::new(),
    );
    assert!(
        pack.assets
            .iter()
            .find(|a| a.asset_id == "decal")
            .unwrap()
            .connectors
            .is_empty()
    );
    assert!(
        !pack
            .assets
            .iter()
            .find(|a| a.asset_id == "wall")
            .unwrap()
            .connectors
            .is_empty()
    );
}
