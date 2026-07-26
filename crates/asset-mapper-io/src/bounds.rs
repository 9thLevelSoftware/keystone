//! Mesh / image bounds extraction for pack indexing.
//!
//! glTF measurement walks the default scene graph and applies node world
//! transforms. Skinned meshes and morph targets are out of scope (bind pose /
//! base positions only when POSITION accessors are present).

use std::path::Path;

use asset_mapper_core::{Bounds3, Vec3};

use crate::error::IoError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasuredBounds {
    pub bounds: Bounds3,
    pub dimensions: Vec3,
}

/// Attempt to measure axis-aligned bounds for a supported asset file.
///
/// Returns `Ok(None)` when the format is recognized but geometry could not be
/// measured (e.g. empty mesh, unsupported image codec). Callers should keep
/// placeholders in that case.
pub fn measure_asset_bounds(path: &Path) -> Result<Option<MeasuredBounds>, IoError> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    match extension.as_str() {
        "glb" | "gltf" => measure_gltf(path),
        "obj" => measure_obj(path),
        "png" => measure_png(path),
        "jpg" | "jpeg" => measure_jpeg(path),
        "webp" => measure_webp(path),
        "fbx" => measure_fbx(path),
        _ => Ok(None),
    }
}

/// Measure FBX bounds from ASCII or binary FBX `Vertices` geometry arrays.
///
/// Binary path walks the Kaydara node tree and expands AABB from every
/// `Vertices` float/double array (raw or zlib-compressed). Nested node
/// transforms are not applied — same local-space model as ASCII.
fn measure_fbx(path: &Path) -> Result<Option<MeasuredBounds>, IoError> {
    let bytes = std::fs::read(path).map_err(|source| IoError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    if bytes.starts_with(b"Kaydara FBX Binary") {
        return Ok(parse_fbx_binary_vertices(&bytes));
    }

    let contents = String::from_utf8_lossy(&bytes);
    if let Some(bounds) = parse_fbx_ascii_vertices(&contents) {
        return Ok(Some(bounds));
    }
    Ok(None)
}

fn parse_fbx_ascii_vertices(contents: &str) -> Option<MeasuredBounds> {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let mut found = false;

    // Match: Vertices: *N { a,b,c, ... }
    for segment in contents.split("Vertices:") {
        let Some(brace) = segment.find('{') else {
            continue;
        };
        let after = &segment[brace + 1..];
        let Some(end) = after.find('}') else {
            continue;
        };
        let body = &after[..end];
        let mut coords = Vec::new();
        for token in body.split(|c: char| c == ',' || c.is_whitespace()) {
            let token = token.trim().trim_end_matches(',');
            if token.is_empty() {
                continue;
            }
            if let Ok(value) = token.parse::<f32>() {
                coords.push(value);
            }
        }
        if coords.len() < 3 {
            continue;
        }
        for chunk in coords.chunks_exact(3) {
            let point = [chunk[0], chunk[1], chunk[2]];
            if !point.iter().all(|v| v.is_finite()) {
                continue;
            }
            expand(&mut min, &mut max, point);
            found = true;
        }
    }

    if found { measured(min, max) } else { None }
}

/// Kaydara binary FBX magic is 23 bytes (`"Kaydara FBX Binary  \0\x1a\0"`), then
/// a little-endian `u32` version. Versions ≥ 7500 use 64-bit node headers.
const FBX_BINARY_HEADER_LEN: usize = 27;
const FBX_BINARY_MAGIC: &[u8] = b"Kaydara FBX Binary  \0\x1a\0";
/// Hard cap on Vertices array elements (floats/doubles). ~16M scalars ≈ 5M
/// xyz points — enough for real assets; rejects adversarial multi-GiB claims.
const FBX_MAX_ARRAY_ELEMENTS: usize = 16_777_216;
/// Nesting limit for untrusted binary trees (legitimate FBX is far shallower).
const FBX_MAX_NODE_DEPTH: u32 = 128;

fn parse_fbx_binary_vertices(bytes: &[u8]) -> Option<MeasuredBounds> {
    if bytes.len() < FBX_BINARY_HEADER_LEN || !bytes.starts_with(FBX_BINARY_MAGIC) {
        return None;
    }
    let version = u32::from_le_bytes(bytes[23..27].try_into().ok()?);
    let large = version >= 7500;

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let mut found = false;
    let mut cursor = FBX_BINARY_HEADER_LEN;

    while cursor < bytes.len() {
        match read_fbx_binary_node(bytes, &mut cursor, large, 0, &mut min, &mut max) {
            Ok(true) => found = true,
            Ok(false) => {}
            // Null terminator, depth limit, or truncated trailer ends the walk.
            Err(_) => break,
        }
    }

    if found { measured(min, max) } else { None }
}

/// Read one FBX binary node (and nested children). Returns `Ok(true)` if any
/// `Vertices` geometry was folded into the AABB. `Ok(false)` for a normal
/// node without vertices. Errors on null records (caller stops) or truncation.
fn read_fbx_binary_node(
    bytes: &[u8],
    cursor: &mut usize,
    large: bool,
    depth: u32,
    min: &mut [f32; 3],
    max: &mut [f32; 3],
) -> Result<bool, ()> {
    if depth > FBX_MAX_NODE_DEPTH {
        return Err(());
    }

    let header_size = if large { 25 } else { 13 };
    let header_end = cursor.checked_add(header_size).ok_or(())?;
    if header_end > bytes.len() {
        return Err(());
    }

    let (end_offset, num_properties, property_list_len) = if large {
        let end = usize::try_from(read_u64(bytes, cursor)?).map_err(|_| ())?;
        let num = usize::try_from(read_u64(bytes, cursor)?).map_err(|_| ())?;
        let plen = usize::try_from(read_u64(bytes, cursor)?).map_err(|_| ())?;
        (end, num, plen)
    } else {
        let end = read_u32(bytes, cursor)? as usize;
        let num = read_u32(bytes, cursor)? as usize;
        let plen = read_u32(bytes, cursor)? as usize;
        (end, num, plen)
    };

    let name_len = *bytes.get(*cursor).ok_or(())? as usize;
    *cursor = cursor.checked_add(1).ok_or(())?;

    // Null record: end_offset == 0 (and typically empty name/props).
    if end_offset == 0 {
        return Err(());
    }

    let name_end = cursor.checked_add(name_len).ok_or(())?;
    if name_end > bytes.len() {
        return Err(());
    }
    let name = std::str::from_utf8(&bytes[*cursor..name_end]).unwrap_or("");
    *cursor = name_end;

    let props_start = *cursor;
    // checked_add: v7500 property_list_len is u64-derived and must not wrap.
    let props_end = props_start.checked_add(property_list_len).ok_or(())?;
    if props_end > bytes.len() {
        return Err(());
    }
    let props = &bytes[props_start..props_end];
    *cursor = props_end;

    let mut found = false;
    if name == "Vertices" && expand_from_fbx_property_list(props, num_properties, min, max) {
        found = true;
    }

    // Nested children fill the remaining span up to end_offset.
    while *cursor < end_offset && *cursor < bytes.len() {
        match read_fbx_binary_node(bytes, cursor, large, depth + 1, min, max) {
            Ok(child_found) => {
                if child_found {
                    found = true;
                }
            }
            Err(()) => break,
        }
    }

    // Ensure we don't re-read trailing padding if the writer left slack.
    if *cursor < end_offset && end_offset <= bytes.len() {
        *cursor = end_offset;
    }

    Ok(found)
}

fn expand_from_fbx_property_list(
    props: &[u8],
    num_properties: usize,
    min: &mut [f32; 3],
    max: &mut [f32; 3],
) -> bool {
    let mut offset = 0usize;
    let mut found = false;
    for _ in 0..num_properties {
        let Some((next, values)) = read_fbx_property_array(props, offset) else {
            break;
        };
        offset = next;
        if let Some(coords) = values {
            if coords.len() < 3 {
                continue;
            }
            for chunk in coords.chunks_exact(3) {
                let point = [chunk[0], chunk[1], chunk[2]];
                if !point.iter().all(|v| v.is_finite()) {
                    continue;
                }
                expand(min, max, point);
                found = true;
            }
        }
    }
    found
}

/// Advance `from` by `nbytes`, requiring the result to stay within `len`.
fn advance_within(from: usize, nbytes: usize, len: usize) -> Option<usize> {
    let next = from.checked_add(nbytes)?;
    if next > len { None } else { Some(next) }
}

/// Parse one FBX property. Returns the next offset and, when the property is a
/// float/double array, the decoded coordinate stream as `f32` values.
fn read_fbx_property_array(props: &[u8], offset: usize) -> Option<(usize, Option<Vec<f32>>)> {
    if offset >= props.len() {
        return None;
    }
    let type_code = props[offset];
    let mut i = offset + 1;

    match type_code {
        b'Y' => {
            i = advance_within(i, 2, props.len())?;
            Some((i, None))
        }
        b'C' => {
            i = advance_within(i, 1, props.len())?;
            Some((i, None))
        }
        b'I' | b'F' => {
            i = advance_within(i, 4, props.len())?;
            Some((i, None))
        }
        b'D' | b'L' => {
            i = advance_within(i, 8, props.len())?;
            Some((i, None))
        }
        b'S' | b'R' => {
            let header_end = i.checked_add(4)?;
            if header_end > props.len() {
                return None;
            }
            let len = u32::from_le_bytes(props[i..header_end].try_into().ok()?) as usize;
            i = advance_within(i, 4, props.len())?;
            i = advance_within(i, len, props.len())?;
            Some((i, None))
        }
        b'f' | b'd' | b'i' | b'l' | b'b' => {
            let header_end = i.checked_add(12)?;
            if header_end > props.len() {
                return None;
            }
            let array_len = u32::from_le_bytes(props[i..i + 4].try_into().ok()?) as usize;
            let encoding = u32::from_le_bytes(props[i + 4..i + 8].try_into().ok()?);
            let compressed_len =
                u32::from_le_bytes(props[i + 8..header_end].try_into().ok()?) as usize;
            i = header_end;
            let payload_end = i.checked_add(compressed_len)?;
            if payload_end > props.len() {
                return None;
            }
            let payload = &props[i..payload_end];
            i = payload_end;

            // Cap before any allocation / inflate to avoid OOM on corrupt files.
            if array_len > FBX_MAX_ARRAY_ELEMENTS {
                return Some((i, None));
            }

            let element_size = match type_code {
                b'f' | b'i' => 4usize,
                b'd' | b'l' => 8,
                b'b' => 1,
                _ => return Some((i, None)),
            };
            let raw_len = array_len.checked_mul(element_size)?;
            let raw = decode_fbx_array_payload(payload, encoding, raw_len)?;

            let values = match type_code {
                b'f' => {
                    let mut out = Vec::with_capacity(array_len);
                    for chunk in raw.chunks_exact(4) {
                        out.push(f32::from_le_bytes(chunk.try_into().ok()?));
                    }
                    Some(out)
                }
                b'd' => {
                    let mut out = Vec::with_capacity(array_len);
                    for chunk in raw.chunks_exact(8) {
                        out.push(f64::from_le_bytes(chunk.try_into().ok()?) as f32);
                    }
                    Some(out)
                }
                _ => None,
            };
            Some((i, values))
        }
        _ => None,
    }
}

fn decode_fbx_array_payload(
    payload: &[u8],
    encoding: u32,
    expected_raw_len: usize,
) -> Option<Vec<u8>> {
    match encoding {
        0 => {
            if payload.len() < expected_raw_len {
                return None;
            }
            Some(payload[..expected_raw_len].to_vec())
        }
        1 => {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            // Bound inflate output to the declared array size so a tiny compressed
            // bomb cannot expand past expected_raw_len into unbounded memory.
            let mut decoder = ZlibDecoder::new(payload).take(expected_raw_len as u64);
            let mut raw = Vec::new();
            // Cap was already applied to array_len; reserve is safe and bounded.
            raw.try_reserve_exact(expected_raw_len).ok()?;
            decoder.read_to_end(&mut raw).ok()?;
            if raw.len() != expected_raw_len {
                return None;
            }
            Some(raw)
        }
        _ => None,
    }
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, ()> {
    let end = cursor.checked_add(4).ok_or(())?;
    if end > bytes.len() {
        return Err(());
    }
    let value = u32::from_le_bytes(bytes[*cursor..end].try_into().map_err(|_| ())?);
    *cursor = end;
    Ok(value)
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, ()> {
    let end = cursor.checked_add(8).ok_or(())?;
    if end > bytes.len() {
        return Err(());
    }
    let value = u64::from_le_bytes(bytes[*cursor..end].try_into().map_err(|_| ())?);
    *cursor = end;
    Ok(value)
}

fn measured(min: Vec3, max: Vec3) -> Option<MeasuredBounds> {
    if !min.iter().all(|v| v.is_finite()) || !max.iter().all(|v| v.is_finite()) {
        return None;
    }
    if max[0] < min[0] || max[1] < min[1] || max[2] < min[2] {
        return None;
    }
    Some(MeasuredBounds {
        bounds: Bounds3 { min, max },
        dimensions: [
            (max[0] - min[0]).max(0.0),
            (max[1] - min[1]).max(0.0),
            (max[2] - min[2]).max(0.0),
        ],
    })
}

/// Load glTF document + buffers without decoding image payloads.
fn measure_gltf(path: &Path) -> Result<Option<MeasuredBounds>, IoError> {
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

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let mut found = false;

    let scenes: Vec<_> = if document.default_scene().is_some() {
        document.default_scene().into_iter().collect()
    } else {
        document.scenes().collect()
    };

    if scenes.is_empty() {
        // No scene graph: fall back to raw mesh space (identity).
        for mesh in document.meshes() {
            if expand_mesh_local(&mesh, &buffers, &IDENTITY, &mut min, &mut max) {
                found = true;
            }
        }
    } else {
        for scene in scenes {
            for node in scene.nodes() {
                if visit_node(&node, &buffers, IDENTITY, &mut min, &mut max) {
                    found = true;
                }
            }
        }
    }

    if !found {
        return Ok(None);
    }

    Ok(measured(min, max))
}

type Mat4 = [[f32; 4]; 4];

const IDENTITY: Mat4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

fn visit_node(
    node: &gltf::Node<'_>,
    buffers: &[gltf::buffer::Data],
    parent_world: Mat4,
    min: &mut [f32; 3],
    max: &mut [f32; 3],
) -> bool {
    let world = mul_mat4(parent_world, node_local_matrix(node));
    let mut found = false;

    if let Some(mesh) = node.mesh() {
        if expand_mesh_local(&mesh, buffers, &world, min, max) {
            found = true;
        }
    }

    for child in node.children() {
        if visit_node(&child, buffers, world, min, max) {
            found = true;
        }
    }

    found
}

fn node_local_matrix(node: &gltf::Node<'_>) -> Mat4 {
    match node.transform() {
        gltf::scene::Transform::Matrix { matrix } => matrix,
        gltf::scene::Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => mat4_from_trs(translation, rotation, scale),
    }
}

fn mat4_from_trs(translation: [f32; 3], rotation: [f32; 4], scale: [f32; 3]) -> Mat4 {
    // rotation is XYZW quaternion in glTF
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

fn expand_mesh_local(
    mesh: &gltf::Mesh<'_>,
    buffers: &[gltf::buffer::Data],
    world: &Mat4,
    min: &mut [f32; 3],
    max: &mut [f32; 3],
) -> bool {
    let mut found = false;
    for primitive in mesh.primitives() {
        if let Some(accessor) = primitive.get(&gltf::Semantic::Positions) {
            if let Some((local_min, local_max)) = accessor_min_max(&accessor) {
                // Transform the 8 AABB corners into world space.
                for &x in &[local_min[0], local_max[0]] {
                    for &y in &[local_min[1], local_max[1]] {
                        for &z in &[local_min[2], local_max[2]] {
                            expand(min, max, transform_point(world, [x, y, z]));
                            found = true;
                        }
                    }
                }
                continue;
            }

            let reader = primitive.reader(|buffer| Some(buffers[buffer.index()].0.as_slice()));
            if let Some(iter) = reader.read_positions() {
                for position in iter {
                    expand(min, max, transform_point(world, position));
                    found = true;
                }
            }
        }
    }
    found
}

fn accessor_min_max(accessor: &gltf::Accessor<'_>) -> Option<(Vec3, Vec3)> {
    let min = accessor.min()?;
    let max = accessor.max()?;
    let min_arr = json_to_vec3(&min)?;
    let max_arr = json_to_vec3(&max)?;
    Some((min_arr, max_arr))
}

fn json_to_vec3(value: &gltf::json::Value) -> Option<Vec3> {
    match value {
        gltf::json::Value::Array(items) if items.len() >= 3 => {
            let x = items[0].as_f64()? as f32;
            let y = items[1].as_f64()? as f32;
            let z = items[2].as_f64()? as f32;
            Some([x, y, z])
        }
        _ => None,
    }
}

fn expand(min: &mut [f32; 3], max: &mut [f32; 3], point: [f32; 3]) {
    for axis in 0..3 {
        if point[axis] < min[axis] {
            min[axis] = point[axis];
        }
        if point[axis] > max[axis] {
            max[axis] = point[axis];
        }
    }
}

fn measure_obj(path: &Path) -> Result<Option<MeasuredBounds>, IoError> {
    let contents = std::fs::read_to_string(path).map_err(|source| IoError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let mut found = false;

    for line in contents.lines() {
        let line = line.trim();
        if !line.starts_with('v') || line.starts_with("vt") || line.starts_with("vn") {
            continue;
        }
        let mut parts = line.split_whitespace();
        if parts.next() != Some("v") {
            continue;
        }
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
        expand(&mut min, &mut max, [x, y, z]);
        found = true;
    }

    if !found {
        return Ok(None);
    }
    Ok(measured(min, max))
}

/// 2D image bounds: min at origin, max at (width, height, 0) in pixel units.
fn image_bounds(width: u32, height: u32) -> Option<MeasuredBounds> {
    if width == 0 || height == 0 {
        return None;
    }
    measured([0.0, 0.0, 0.0], [width as f32, height as f32, 0.0])
}

fn measure_png(path: &Path) -> Result<Option<MeasuredBounds>, IoError> {
    let bytes = std::fs::read(path).map_err(|source| IoError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Ok(None);
    }
    if &bytes[12..16] != b"IHDR" {
        return Ok(None);
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Ok(image_bounds(width, height))
}

fn measure_jpeg(path: &Path) -> Result<Option<MeasuredBounds>, IoError> {
    let bytes = std::fs::read(path).map_err(|source| IoError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Ok(None);
    }

    let mut i = 2usize;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        i += 2;
        if marker == 0xD8 || marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }
        if i + 1 >= bytes.len() {
            break;
        }
        let length = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        if length < 2 || i + length > bytes.len() {
            break;
        }
        let is_sof = matches!(
            marker,
            0xC0 | 0xC1
                | 0xC2
                | 0xC3
                | 0xC5
                | 0xC6
                | 0xC7
                | 0xC9
                | 0xCA
                | 0xCB
                | 0xCD
                | 0xCE
                | 0xCF
        );
        if is_sof && length >= 7 {
            let height = u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]) as u32;
            let width = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            return Ok(image_bounds(width, height));
        }
        i += length;
    }
    Ok(None)
}

fn measure_webp(path: &Path) -> Result<Option<MeasuredBounds>, IoError> {
    let bytes = std::fs::read(path).map_err(|source| IoError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    if bytes.len() < 30 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return Ok(None);
    }
    let chunk = &bytes[12..];
    if chunk.len() < 10 {
        return Ok(None);
    }
    let fourcc = &chunk[0..4];
    match fourcc {
        b"VP8 " if chunk.len() >= 18 => {
            let data = &chunk[8..];
            if data.len() < 10 || data[0] != 0x9D || data[1] != 0x01 || data[2] != 0x2A {
                return Ok(None);
            }
            let width = u16::from_le_bytes([data[6], data[7]]) as u32 & 0x3FFF;
            let height = u16::from_le_bytes([data[8], data[9]]) as u32 & 0x3FFF;
            Ok(image_bounds(width, height))
        }
        b"VP8L" if chunk.len() >= 13 => {
            let data = &chunk[8..];
            if data.is_empty() || data[0] != 0x2F {
                return Ok(None);
            }
            let b1 = data[1] as u32;
            let b2 = data[2] as u32;
            let b3 = data[3] as u32;
            let b4 = data[4] as u32;
            let bits = b1 | (b2 << 8) | (b3 << 16) | (b4 << 24);
            let width = (bits & 0x3FFF) + 1;
            let height = ((bits >> 14) & 0x3FFF) + 1;
            Ok(image_bounds(width, height))
        }
        b"VP8X" if chunk.len() >= 18 => {
            let data = &chunk[8..];
            let width = 1 + u32::from_le_bytes([data[4], data[5], data[6], 0]);
            let height = 1 + u32::from_le_bytes([data[7], data[8], data[9], 0]);
            Ok(image_bounds(width, height))
        }
        _ => Ok(None),
    }
}
