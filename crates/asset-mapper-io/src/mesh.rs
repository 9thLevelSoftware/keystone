//! Load world-space mesh samples for socket proposal.

use std::path::Path;

use asset_mapper_core::MeshGeometry;

use crate::bounds::extract_fbx_vertices;
use crate::error::IoError;

const MAX_VERTICES: usize = 200_000;

/// Load triangle/position samples for glTF, GLB, OBJ, or FBX Vertices.
///
/// Returns `Ok(None)` when the format is unsupported or geometry is empty.
/// glTF is preferred for auto-map quality (indexed triangles + scene transforms).
/// FBX yields a local-space point cloud from `Vertices` arrays (no indices).
pub fn load_mesh_geometry(path: &Path) -> Result<Option<MeshGeometry>, IoError> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    match extension.as_str() {
        "glb" | "gltf" => load_gltf_mesh(path),
        "obj" => load_obj_mesh(path),
        "fbx" => load_fbx_mesh(path),
        _ => Ok(None),
    }
}

fn load_fbx_mesh(path: &Path) -> Result<Option<MeshGeometry>, IoError> {
    let Some(positions) = extract_fbx_vertices(path, MAX_VERTICES)? else {
        return Ok(None);
    };
    if positions.is_empty() {
        return Ok(None);
    }
    Ok(Some(MeshGeometry {
        positions,
        indices: None,
    }))
}

fn load_gltf_mesh(path: &Path) -> Result<Option<MeshGeometry>, IoError> {
    let gltf = gltf::Gltf::open(path).map_err(|source| IoError::MeasureBounds {
        path: path.to_path_buf(),
        message: source.to_string(),
    })?;
    let base = path.parent();
    let buffers = gltf::import_buffers(&gltf.document, base, gltf.blob).map_err(|source| {
        IoError::MeasureBounds {
            path: path.to_path_buf(),
            message: source.to_string(),
        }
    })?;
    let document = gltf.document;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut use_indices = true;

    type Mat4 = [[f32; 4]; 4];
    const IDENTITY: Mat4 = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    fn mul_mat4(a: Mat4, b: Mat4) -> Mat4 {
        let mut out = [[0.0; 4]; 4];
        for col in 0..4 {
            for row in 0..4 {
                out[col][row] = a[0][row] * b[col][0]
                    + a[1][row] * b[col][1]
                    + a[2][row] * b[col][2]
                    + a[3][row] * b[col][3];
            }
        }
        out
    }

    fn transform_point(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
        [
            m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0],
            m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1],
            m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2],
        ]
    }

    fn node_local_matrix(node: &gltf::Node<'_>) -> Mat4 {
        match node.transform() {
            gltf::scene::Transform::Matrix { matrix } => matrix,
            gltf::scene::Transform::Decomposed {
                translation,
                rotation,
                scale,
            } => {
                let [x, y, z, w] = rotation;
                let (x2, y2, z2) = (x + x, y + y, z + z);
                let (xx, xy, xz) = (x * x2, x * y2, x * z2);
                let (yy, yz, zz) = (y * y2, y * z2, z * z2);
                let (wx, wy, wz) = (w * x2, w * y2, w * z2);
                let mut m = IDENTITY;
                m[0][0] = (1.0 - (yy + zz)) * scale[0];
                m[0][1] = (xy + wz) * scale[0];
                m[0][2] = (xz - wy) * scale[0];
                m[1][0] = (xy - wz) * scale[1];
                m[1][1] = (1.0 - (xx + zz)) * scale[1];
                m[1][2] = (yz + wx) * scale[1];
                m[2][0] = (xz + wy) * scale[2];
                m[2][1] = (yz - wx) * scale[2];
                m[2][2] = (1.0 - (xx + yy)) * scale[2];
                m[3][0] = translation[0];
                m[3][1] = translation[1];
                m[3][2] = translation[2];
                m
            }
        }
    }

    fn collect_mesh(
        mesh: &gltf::Mesh<'_>,
        buffers: &[gltf::buffer::Data],
        world: &Mat4,
        positions: &mut Vec<[f32; 3]>,
        indices: &mut Vec<u32>,
        use_indices: &mut bool,
    ) {
        for primitive in mesh.primitives() {
            if positions.len() >= MAX_VERTICES {
                return;
            }
            let reader = primitive.reader(|buffer| Some(buffers[buffer.index()].0.as_slice()));
            let Some(iter) = reader.read_positions() else {
                continue;
            };
            let base = positions.len() as u32;
            let mut count = 0u32;
            for position in iter {
                if positions.len() >= MAX_VERTICES {
                    break;
                }
                positions.push(transform_point(world, position));
                count += 1;
            }
            if count == 0 {
                continue;
            }

            if let Some(idx_iter) = reader.read_indices() {
                for i in idx_iter.into_u32() {
                    if i < count {
                        indices.push(base + i);
                    }
                }
            } else {
                // Non-indexed primitive: if we already have indices from others,
                // append a sequential fan; else leave unindexed overall.
                if !indices.is_empty() || *use_indices {
                    for i in 0..count {
                        indices.push(base + i);
                    }
                } else {
                    *use_indices = false;
                }
            }
        }
    }

    fn visit_node(
        node: &gltf::Node<'_>,
        buffers: &[gltf::buffer::Data],
        parent_world: Mat4,
        positions: &mut Vec<[f32; 3]>,
        indices: &mut Vec<u32>,
        use_indices: &mut bool,
    ) {
        let world = mul_mat4(parent_world, node_local_matrix(node));
        if let Some(mesh) = node.mesh() {
            collect_mesh(&mesh, buffers, &world, positions, indices, use_indices);
        }
        for child in node.children() {
            visit_node(&child, buffers, world, positions, indices, use_indices);
        }
    }

    let scenes: Vec<_> = if document.default_scene().is_some() {
        document.default_scene().into_iter().collect()
    } else {
        document.scenes().collect()
    };

    if scenes.is_empty() {
        for mesh in document.meshes() {
            collect_mesh(
                &mesh,
                &buffers,
                &IDENTITY,
                &mut positions,
                &mut indices,
                &mut use_indices,
            );
        }
    } else {
        for scene in scenes {
            for node in scene.nodes() {
                visit_node(
                    &node,
                    &buffers,
                    IDENTITY,
                    &mut positions,
                    &mut indices,
                    &mut use_indices,
                );
            }
        }
    }

    if positions.is_empty() {
        return Ok(None);
    }

    let indices = if use_indices && indices.len() >= 3 {
        Some(indices)
    } else {
        None
    };

    Ok(Some(MeshGeometry { positions, indices }))
}

fn load_obj_mesh(path: &Path) -> Result<Option<MeshGeometry>, IoError> {
    let contents = std::fs::read_to_string(path).map_err(|source| IoError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with("v ") {
            let mut parts = line.split_whitespace();
            let _ = parts.next();
            let Some(x) = parts.next().and_then(|v| v.parse::<f32>().ok()) else {
                continue;
            };
            let Some(y) = parts.next().and_then(|v| v.parse::<f32>().ok()) else {
                continue;
            };
            let z = parts
                .next()
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.0);
            if positions.len() < MAX_VERTICES {
                positions.push([x, y, z]);
            }
        } else if line.starts_with('f') {
            let mut verts: Vec<u32> = Vec::new();
            for token in line.split_whitespace().skip(1) {
                let idx_str = token.split('/').next().unwrap_or("");
                if let Ok(raw) = idx_str.parse::<i32>() {
                    let idx = if raw < 0 {
                        (positions.len() as i32 + raw) as u32
                    } else if raw > 0 {
                        (raw - 1) as u32
                    } else {
                        continue;
                    };
                    verts.push(idx);
                }
            }
            // Fan triangulate
            if verts.len() >= 3 {
                for i in 1..verts.len() - 1 {
                    indices.push(verts[0]);
                    indices.push(verts[i]);
                    indices.push(verts[i + 1]);
                }
            }
        }
    }

    if positions.is_empty() {
        return Ok(None);
    }
    let indices = if indices.len() >= 3 {
        Some(indices)
    } else {
        None
    };
    Ok(Some(MeshGeometry { positions, indices }))
}
