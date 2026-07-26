use asset_mapper_core::ReviewFlag;
use asset_mapper_io::{
    accept_hash_drift, apply_measured_bounds, init_pack_folder, measure_asset_bounds,
    measure_pack_bounds, read_pack_from_input, write_pack_sidecar,
};

/// Minimal triangle GLB (accessor min/max [-0.5,0,0]..[0.5,1,0]), identity node.
fn write_simple_triangle_glb(path: &std::path::Path) {
    write_triangle_glb(path, None, None);
}

/// Triangle GLB with node translation + scale applied in the scene graph.
fn write_transformed_triangle_glb(path: &std::path::Path) {
    write_triangle_glb(path, Some([10.0, 0.0, 0.0]), Some([2.0, 2.0, 2.0]));
}

fn write_triangle_glb(
    path: &std::path::Path,
    translation: Option<[f32; 3]>,
    scale: Option<[f32; 3]>,
) {
    let positions = [-0.5_f32, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 1.0, 0.0];
    let mut binary = Vec::with_capacity(positions.len() * 4);
    for value in positions {
        binary.extend_from_slice(&value.to_le_bytes());
    }

    let mut node = serde_json::json!({
        "mesh": 0,
        "name": "Tri"
    });
    if let Some(t) = translation {
        node["translation"] = serde_json::json!([t[0], t[1], t[2]]);
    }
    if let Some(s) = scale {
        node["scale"] = serde_json::json!([s[0], s[1], s[2]]);
    }

    let json = serde_json::json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [node],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0 },
                "mode": 4
            }]
        }],
        "accessors": [{
            "bufferView": 0,
            "componentType": 5126,
            "count": 3,
            "type": "VEC3",
            "min": [-0.5, 0.0, 0.0],
            "max": [0.5, 1.0, 0.0]
        }],
        "bufferViews": [{
            "buffer": 0,
            "byteOffset": 0,
            "byteLength": binary.len()
        }],
        "buffers": [{ "byteLength": binary.len() }]
    });

    let json_bytes = pad_to_4(serde_json::to_vec(&json).expect("json"), 0x20);
    let bin_bytes = pad_to_4(binary, 0x00);
    let total = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&0x46546c67u32.to_le_bytes()); // glTF
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4e4f534au32.to_le_bytes()); // JSON
    out.extend_from_slice(&json_bytes);
    out.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004e4942u32.to_le_bytes()); // BIN
    out.extend_from_slice(&bin_bytes);
    std::fs::write(path, out).expect("write glb");
}

fn pad_to_4(mut data: Vec<u8>, pad: u8) -> Vec<u8> {
    let rem = (4 - (data.len() % 4)) % 4;
    data.extend(std::iter::repeat_n(pad, rem));
    data
}

#[test]
fn measures_real_aabb_from_embedded_glb() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("tri.glb");
    write_simple_triangle_glb(&path);

    let measured = measure_asset_bounds(&path)
        .expect("measure succeeds")
        .expect("glb has positions");

    assert!((measured.bounds.min[0] - (-0.5)).abs() < 0.001);
    assert!((measured.bounds.min[1] - 0.0).abs() < 0.001);
    assert!((measured.bounds.max[0] - 0.5).abs() < 0.001);
    assert!((measured.bounds.max[1] - 1.0).abs() < 0.001);
    assert!((measured.dimensions[0] - 1.0).abs() < 0.001);
    assert!((measured.dimensions[1] - 1.0).abs() < 0.001);
}

#[test]
fn measures_world_aabb_with_node_translation_and_scale() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("xform.glb");
    write_transformed_triangle_glb(&path);

    let measured = measure_asset_bounds(&path)
        .expect("measure succeeds")
        .expect("glb has positions");

    // Local min [-0.5,0,0] max [0.5,1,0], scale 2, translate +10 on X
    // world min ≈ [9.0, 0.0, 0.0], max ≈ [11.0, 2.0, 0.0]
    assert!(
        (measured.bounds.min[0] - 9.0).abs() < 0.01,
        "min.x={:?}",
        measured.bounds.min
    );
    assert!((measured.bounds.max[0] - 11.0).abs() < 0.01);
    assert!((measured.bounds.min[1] - 0.0).abs() < 0.01);
    assert!((measured.bounds.max[1] - 2.0).abs() < 0.01);
    assert!((measured.dimensions[0] - 2.0).abs() < 0.01);
    assert!((measured.dimensions[1] - 2.0).abs() < 0.01);
}

#[test]
fn init_pack_with_glb_clears_bounds_placeholder() {
    let temp = tempfile::tempdir().expect("temp");
    write_simple_triangle_glb(&temp.path().join("wall.glb"));

    init_pack_folder(temp.path(), "Wall Pack".to_owned()).expect("init");
    let loaded = read_pack_from_input(temp.path()).expect("load");
    let asset = &loaded.pack.assets[0];

    assert!(
        !asset.review_flags.contains(&ReviewFlag::BoundsPlaceholder),
        "real bounds should clear BoundsPlaceholder, flags={:?}",
        asset.review_flags
    );
    assert!((asset.dimensions[0] - 1.0).abs() < 0.001);
    assert!((asset.bounds.min[0] - (-0.5)).abs() < 0.001);
}

#[test]
fn apply_measured_bounds_updates_asset() {
    let temp = tempfile::tempdir().expect("temp");
    write_simple_triangle_glb(&temp.path().join("real.glb"));

    init_pack_folder(temp.path(), "Pack".to_owned()).expect("init");
    let mut loaded = read_pack_from_input(temp.path()).expect("load");
    let asset = loaded
        .pack
        .assets
        .iter_mut()
        .find(|a| a.source_path == "real.glb")
        .expect("real asset");
    asset.review_flags.push(ReviewFlag::BoundsPlaceholder);
    asset.bounds.min = [-0.5, -0.5, -0.5];
    asset.bounds.max = [0.5, 0.5, 0.5];

    let ok = apply_measured_bounds(asset, &temp.path().join("real.glb")).expect("measure");
    assert!(ok);
    assert!(!asset.review_flags.contains(&ReviewFlag::BoundsPlaceholder));
}

#[test]
fn measure_pack_bounds_reports_measured_failed_missing() {
    let temp = tempfile::tempdir().expect("temp");
    write_simple_triangle_glb(&temp.path().join("ok.glb"));
    std::fs::write(temp.path().join("nope.fbx"), b"fbx-stub").expect("fbx");
    init_pack_folder(temp.path(), "Report Pack".to_owned()).expect("init");

    // Add a missing sidecar asset manually.
    let mut loaded = read_pack_from_input(temp.path()).expect("load");
    let mut ghost = loaded.pack.assets[0].clone();
    ghost.asset_id = "ghost".to_owned();
    ghost.source_path = "ghost.glb".to_owned();
    loaded.pack.assets.push(ghost);
    write_pack_sidecar(temp.path(), &loaded.pack).expect("write");

    let report = measure_pack_bounds(temp.path()).expect("measure pack");
    assert!(report.measured.iter().any(|p| p == "ok.glb"));
    assert!(report.failed.iter().any(|p| p == "nope.fbx"));
    assert!(report.missing.iter().any(|p| p == "ghost.glb"));
}

#[test]
fn measure_pack_bounds_continues_when_one_glb_is_corrupt() {
    let temp = tempfile::tempdir().expect("temp");
    write_simple_triangle_glb(&temp.path().join("good.glb"));
    // Extension says glTF but content is not a valid GLB → MeasureBounds Err.
    std::fs::write(temp.path().join("bad.glb"), b"not-a-glb").expect("corrupt glb");
    init_pack_folder(temp.path(), "Mixed Pack".to_owned()).expect("init");

    let report = measure_pack_bounds(temp.path()).expect("pack measure must not abort");
    assert!(
        report.measured.iter().any(|p| p == "good.glb"),
        "valid asset should still measure: {report:?}"
    );
    assert!(
        report.failed.iter().any(|p| p == "bad.glb"),
        "corrupt glb should land in failed: {report:?}"
    );

    let reloaded = read_pack_from_input(temp.path()).expect("sidecar written");
    let good = reloaded
        .pack
        .assets
        .iter()
        .find(|a| a.source_path == "good.glb")
        .expect("good asset");
    assert!(!good.review_flags.contains(&ReviewFlag::BoundsPlaceholder));
    assert!((good.dimensions[0] - 1.0).abs() < 0.001);
}

#[test]
fn measures_png_pixel_dimensions() {
    let png: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x08, 0x02, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00,
    ];
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("tile.png");
    std::fs::write(&path, png).expect("write png");

    let measured = measure_asset_bounds(&path)
        .expect("ok")
        .expect("dimensions");
    assert_eq!(measured.dimensions, [2.0, 3.0, 0.0]);
    assert_eq!(measured.bounds.max, [2.0, 3.0, 0.0]);
}

#[test]
fn accept_hash_drift_updates_hash_keeps_connectors() {
    let temp = tempfile::tempdir().expect("temp");
    std::fs::write(temp.path().join("wall.glb"), b"v1").expect("write");
    init_pack_folder(temp.path(), "Drift Pack".to_owned()).expect("init");

    let mut loaded = read_pack_from_input(temp.path()).expect("load");
    let original_hash = loaded.pack.assets[0].content_hash.clone();
    loaded.pack.assets[0]
        .connectors
        .push(asset_mapper_core::ConnectorRecord {
            connector_id: "edge".to_owned(),
            display_name: "Edge".to_owned(),
            class: "tmp".to_owned(),
            role: asset_mapper_core::ConnectorRole::Symmetric,
            frame: asset_mapper_core::ConnectorFrame::Frame3d {
                position: [0.0, 0.0, 0.0],
                orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
            },
            mating_axis: asset_mapper_core::Axis3::PosZ,
            up_reference: asset_mapper_core::Axis3::PosY,
            snap_tolerance: 0.01,
        });
    loaded
        .pack
        .connector_classes
        .push(asset_mapper_core::ConnectorClass {
            class: "tmp".to_owned(),
            display_name: "Tmp".to_owned(),
        });
    write_pack_sidecar(temp.path(), &loaded.pack).expect("write");

    std::fs::write(temp.path().join("wall.glb"), b"v2-changed").expect("mutate");

    let report = accept_hash_drift(temp.path(), None, false).expect("accept");
    assert!(report.drifted_assets.is_empty());
    assert!(report.unchanged_assets.contains(&"wall.glb".to_owned()));

    let reloaded = read_pack_from_input(temp.path()).expect("reload");
    assert_ne!(reloaded.pack.assets[0].content_hash, original_hash);
    assert_eq!(reloaded.pack.assets[0].connectors.len(), 1);
    assert_eq!(reloaded.pack.assets[0].connectors[0].connector_id, "edge");
}

#[test]
fn accept_hash_drift_unknown_asset_filter() {
    let temp = tempfile::tempdir().expect("temp");
    std::fs::write(temp.path().join("wall.glb"), b"v1").expect("write");
    init_pack_folder(temp.path(), "Drift Pack".to_owned()).expect("init");

    let err = accept_hash_drift(temp.path(), Some(vec!["nope".to_owned()]), false)
        .expect_err("unknown filter");
    assert!(matches!(
        err,
        asset_mapper_io::IoError::UnknownAsset { asset_id } if asset_id == "nope"
    ));
}

#[test]
fn measures_ascii_fbx_vertices() {
    let ascii = r#"
; FBX 7.4.0 project file
FBXHeaderExtension:  {
}
Objects:  {
    Geometry: 123, "Geometry::Cube", "Mesh" {
        Vertices: *9 {
            a: -1.0,0.0,-1.0,1.0,0.0,-1.0,1.0,2.0,1.0
        }
    }
}
"#;
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("cube.fbx");
    std::fs::write(&path, ascii).expect("write fbx");
    let measured = measure_asset_bounds(&path)
        .expect("ok")
        .expect("ascii fbx bounds");
    assert!((measured.bounds.min[0] - -1.0).abs() < 0.001);
    assert!((measured.bounds.max[1] - 2.0).abs() < 0.001);
    assert!((measured.dimensions[0] - 2.0).abs() < 0.001);
}
