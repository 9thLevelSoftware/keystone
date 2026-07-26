use asset_mapper_core::ReviewFlag;
use asset_mapper_io::{
    InitPackOptions, accept_hash_drift, apply_measured_bounds, extract_fbx_vertices,
    init_pack_folder, load_mesh_geometry, measure_asset_bounds, measure_pack_bounds,
    read_pack_from_input, write_pack_sidecar,
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

    init_pack_folder(temp.path(), InitPackOptions::for_tests("Wall Pack")).expect("init");
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

    init_pack_folder(temp.path(), InitPackOptions::for_tests("Pack")).expect("init");
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
    init_pack_folder(temp.path(), InitPackOptions::for_tests("Report Pack")).expect("init");

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
    init_pack_folder(temp.path(), InitPackOptions::for_tests("Mixed Pack")).expect("init");

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
    init_pack_folder(temp.path(), InitPackOptions::for_tests("Drift Pack")).expect("init");

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
            face_size: None,
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
    init_pack_folder(temp.path(), InitPackOptions::for_tests("Drift Pack")).expect("init");

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

#[test]
fn extract_ascii_fbx_vertices_respects_cap() {
    let ascii = r#"
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

    let all = extract_fbx_vertices(&path, 100)
        .expect("ok")
        .expect("positions");
    assert_eq!(all.len(), 3);
    assert!(all.iter().all(|p| p.iter().all(|v| v.is_finite())));

    let capped = extract_fbx_vertices(&path, 2).expect("ok").expect("capped");
    assert_eq!(capped.len(), 2);

    let mesh = load_mesh_geometry(&path).expect("ok").expect("fbx mesh");
    assert_eq!(mesh.positions.len(), 3);
    assert!(mesh.indices.is_none());
}

#[test]
fn extract_binary_fbx_float_vertices() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("cube_f.fbx");
    let vertices = [-1.5f32, 0.0, -0.5, 2.5, 3.0, 1.0, 0.0, 1.0, 0.5];
    write_minimal_binary_fbx(
        &path,
        7400,
        encode_fbx_float_array_property(&vertices, false),
    );
    let positions = extract_fbx_vertices(&path, 10)
        .expect("ok")
        .expect("positions");
    assert_eq!(positions.len(), 3);
    assert!((positions[0][0] - -1.5).abs() < 0.001);
}

#[test]
fn extract_fbx_without_vertices_returns_none() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("empty.fbx");
    let mut file = Vec::new();
    file.extend_from_slice(b"Kaydara FBX Binary  \0\x1a\0");
    file.extend_from_slice(&7400u32.to_le_bytes());
    let tree = fbx_parent("Objects", vec![fbx_leaf("NotGeometry", 0, Vec::new())]);
    write_fbx_node_at(&mut file, &tree, false);
    file.extend_from_slice(&[0u8; 13]);
    std::fs::write(&path, file).expect("write");

    let positions = extract_fbx_vertices(&path, 100).expect("ok");
    assert!(positions.is_none());
    assert!(load_mesh_geometry(&path).expect("ok").is_none());
}

/// Kaydara binary FBX node draft. EndOffset is patched when written.
struct FbxNodeDraft {
    name: String,
    num_properties: u32,
    property_list: Vec<u8>,
    children: Vec<FbxNodeDraft>,
    is_leaf: bool,
}

fn fbx_leaf(name: &str, num_properties: u32, property_list: Vec<u8>) -> FbxNodeDraft {
    FbxNodeDraft {
        name: name.to_owned(),
        num_properties,
        property_list,
        children: Vec::new(),
        is_leaf: true,
    }
}

fn fbx_parent(name: &str, children: Vec<FbxNodeDraft>) -> FbxNodeDraft {
    FbxNodeDraft {
        name: name.to_owned(),
        num_properties: 0,
        property_list: Vec::new(),
        children,
        is_leaf: false,
    }
}

fn zlib_compress(raw: &[u8]) -> Vec<u8> {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(raw).expect("zlib write");
    encoder.finish().expect("zlib finish")
}

fn encode_fbx_array_property(
    type_code: u8,
    element_size: usize,
    raw: &[u8],
    zlib: bool,
) -> Vec<u8> {
    let array_len = (raw.len() / element_size) as u32;
    let (encoding, payload) = if zlib {
        (1u32, zlib_compress(raw))
    } else {
        (0u32, raw.to_vec())
    };

    let mut props = Vec::new();
    props.push(type_code);
    props.extend_from_slice(&array_len.to_le_bytes());
    props.extend_from_slice(&encoding.to_le_bytes());
    props.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    props.extend_from_slice(&payload);
    props
}

fn encode_fbx_double_array_property(values: &[f64], zlib: bool) -> Vec<u8> {
    let mut raw = Vec::with_capacity(values.len() * 8);
    for value in values {
        raw.extend_from_slice(&value.to_le_bytes());
    }
    encode_fbx_array_property(b'd', 8, &raw, zlib)
}

fn encode_fbx_float_array_property(values: &[f32], zlib: bool) -> Vec<u8> {
    let mut raw = Vec::with_capacity(values.len() * 4);
    for value in values {
        raw.extend_from_slice(&value.to_le_bytes());
    }
    encode_fbx_array_property(b'f', 4, &raw, zlib)
}

fn write_fbx_node_at(file: &mut Vec<u8>, node: &FbxNodeDraft, large: bool) {
    let header_pos = file.len();
    if large {
        file.extend_from_slice(&0u64.to_le_bytes()); // end_offset
        file.extend_from_slice(&(node.num_properties as u64).to_le_bytes());
        file.extend_from_slice(&(node.property_list.len() as u64).to_le_bytes());
    } else {
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&node.num_properties.to_le_bytes());
        file.extend_from_slice(&(node.property_list.len() as u32).to_le_bytes());
    }
    file.push(node.name.len() as u8);
    file.extend_from_slice(node.name.as_bytes());
    file.extend_from_slice(&node.property_list);

    if !node.is_leaf {
        for child in &node.children {
            write_fbx_node_at(file, child, large);
        }
        // Null child terminator: 25 bytes (v7500+) or 13 bytes (older).
        if large {
            file.extend_from_slice(&[0u8; 25]);
        } else {
            file.extend_from_slice(&[0u8; 13]);
        }
    }

    let end_offset = file.len();
    if large {
        file[header_pos..header_pos + 8].copy_from_slice(&(end_offset as u64).to_le_bytes());
    } else {
        file[header_pos..header_pos + 4].copy_from_slice(&(end_offset as u32).to_le_bytes());
    }
}

fn write_minimal_binary_fbx(path: &std::path::Path, version: u32, props: Vec<u8>) {
    let large = version >= 7500;
    let mut file = Vec::new();
    file.extend_from_slice(b"Kaydara FBX Binary  \0\x1a\0");
    file.extend_from_slice(&version.to_le_bytes());
    let tree = fbx_parent(
        "Objects",
        vec![fbx_parent("Geometry", vec![fbx_leaf("Vertices", 1, props)])],
    );
    write_fbx_node_at(&mut file, &tree, large);
    if large {
        file.extend_from_slice(&[0u8; 25]);
    } else {
        file.extend_from_slice(&[0u8; 13]);
    }
    std::fs::write(path, file).expect("write binary fbx");
}

#[test]
fn measures_binary_fbx_vertices_raw() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("cube_bin.fbx");
    let vertices = [-1.0_f64, 0.0, -1.0, 1.0, 0.0, -1.0, 1.0, 2.0, 1.0];
    write_minimal_binary_fbx(
        &path,
        7400,
        encode_fbx_double_array_property(&vertices, false),
    );

    let measured = measure_asset_bounds(&path)
        .expect("ok")
        .expect("binary fbx bounds");
    assert!((measured.bounds.min[0] - -1.0).abs() < 0.001);
    assert!((measured.bounds.max[1] - 2.0).abs() < 0.001);
    assert!((measured.dimensions[0] - 2.0).abs() < 0.001);
    assert!((measured.dimensions[1] - 2.0).abs() < 0.001);
}

#[test]
fn measures_binary_fbx_vertices_zlib() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("cube_z.fbx");
    let vertices = [-2.0_f64, -3.0, -4.0, 5.0, 6.0, 7.0];
    write_minimal_binary_fbx(
        &path,
        7400,
        encode_fbx_double_array_property(&vertices, true),
    );

    let measured = measure_asset_bounds(&path)
        .expect("ok")
        .expect("zlib binary fbx bounds");
    assert!((measured.bounds.min[0] - -2.0).abs() < 0.001);
    assert!((measured.bounds.min[1] - -3.0).abs() < 0.001);
    assert!((measured.bounds.min[2] - -4.0).abs() < 0.001);
    assert!((measured.bounds.max[0] - 5.0).abs() < 0.001);
    assert!((measured.bounds.max[1] - 6.0).abs() < 0.001);
    assert!((measured.bounds.max[2] - 7.0).abs() < 0.001);
}

#[test]
fn measures_binary_fbx_vertices_v7500() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("cube_v75.fbx");
    let vertices = [-1.0_f64, 0.0, -1.0, 1.0, 0.0, -1.0, 1.0, 2.0, 1.0];
    write_minimal_binary_fbx(
        &path,
        7500,
        encode_fbx_double_array_property(&vertices, false),
    );

    let measured = measure_asset_bounds(&path)
        .expect("ok")
        .expect("v7500 binary fbx bounds");
    assert!((measured.bounds.min[0] - -1.0).abs() < 0.001);
    assert!((measured.bounds.max[1] - 2.0).abs() < 0.001);
    assert!((measured.dimensions[0] - 2.0).abs() < 0.001);
}

#[test]
fn measures_binary_fbx_float_vertices() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("cube_f.fbx");
    let vertices = [-1.5_f32, 0.0, -0.5, 2.5, 3.0, 0.5];
    write_minimal_binary_fbx(
        &path,
        7400,
        encode_fbx_float_array_property(&vertices, false),
    );

    let measured = measure_asset_bounds(&path)
        .expect("ok")
        .expect("float binary fbx bounds");
    assert!((measured.bounds.min[0] - -1.5).abs() < 0.001);
    assert!((measured.bounds.min[2] - -0.5).abs() < 0.001);
    assert!((measured.bounds.max[0] - 2.5).abs() < 0.001);
    assert!((measured.bounds.max[1] - 3.0).abs() < 0.001);
    assert!((measured.dimensions[0] - 4.0).abs() < 0.001);
}

#[test]
fn binary_fbx_without_vertices_returns_none() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("empty.fbx");
    let mut file = Vec::new();
    file.extend_from_slice(b"Kaydara FBX Binary  \0\x1a\0");
    file.extend_from_slice(&7400u32.to_le_bytes());
    let tree = fbx_parent("Objects", vec![fbx_leaf("NotGeometry", 0, Vec::new())]);
    write_fbx_node_at(&mut file, &tree, false);
    file.extend_from_slice(&[0u8; 13]);
    std::fs::write(&path, file).expect("write");

    let measured = measure_asset_bounds(&path).expect("ok");
    assert!(measured.is_none());
}

/// Adversarial v7500 node with `property_list_len = u64::MAX` must not panic
/// (checked_add on the property span) and returns `None`.
#[test]
fn binary_fbx_huge_property_list_len_does_not_panic() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().join("evil.fbx");
    let mut file = Vec::new();
    file.extend_from_slice(b"Kaydara FBX Binary  \0\x1a\0");
    file.extend_from_slice(&7500u32.to_le_bytes());
    // Node header (64-bit): end_offset, num_properties, property_list_len, name_len=0
    file.extend_from_slice(&100u64.to_le_bytes());
    file.extend_from_slice(&0u64.to_le_bytes());
    file.extend_from_slice(&u64::MAX.to_le_bytes());
    file.push(0); // empty name
    std::fs::write(&path, &file).expect("write");

    let measured = measure_asset_bounds(&path).expect("measure must not panic");
    assert!(measured.is_none());
}
