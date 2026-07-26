//! Authoring-speed helpers (class suggestions, face snaps, connector clones).

use glam::{Mat3, Quat, Vec3 as GVec3};

use crate::schema::{
    AssetRecord, Axis3, Bounds3, ConnectorFrame, ConnectorRecord, ConnectorRole, QuatXyzw, Vec3,
};

/// Suggest a connector class name from a free-text label (display name or id).
///
/// More specific tokens first. Prefer [`suggest_class_from_asset`] when
/// `source_path` is available so folder categories (Walls/, Platforms/) win.
pub fn suggest_class_from_name(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    // Order matters: window/door before wall so "wall_window" is window_frame.
    let patterns: &[(&str, &str)] = &[
        ("window", "window_frame"),
        ("door", "doorway"),
        ("arch", "archway"),
        ("stair", "stair_landing"),
        ("corridor", "corridor_end"),
        ("hall", "corridor_end"),
        ("ceiling", "ceiling_edge"),
        ("floor", "floor_edge"),
        ("platform", "floor_edge"),
        ("roof", "roof_edge"),
        ("pipe", "pipe_end"),
        ("wall", "wall_edge"),
        ("column", "module_edge"),
        ("tile", "tile_edge"),
        ("socket", "socket"),
        ("plug", "plug"),
    ];

    for (needle, class_name) in patterns {
        if lower.contains(needle) {
            return Some((*class_name).to_owned());
        }
    }
    None
}

/// Path- and name-aware class suggestion for modular marketplace kits.
///
/// Folder categories (e.g. `Walls/`, `Platforms/Door_*`) override ambiguous
/// shape heuristics. Never promotes decorative path noise (decals) to doorway.
pub fn suggest_class_from_asset(asset: &AssetRecord) -> Option<String> {
    let path = asset.source_path.replace('\\', "/");
    let lower_path = path.to_ascii_lowercase();
    let file = lower_path
        .rsplit('/')
        .next()
        .unwrap_or(lower_path.as_str());
    let file_stem = file.rsplit_once('.').map(|(s, _)| s).unwrap_or(file);

    // Explicit non-modular categories: leave to shape or generic module.
    if lower_path.contains("/decals/") || lower_path.contains("/aliens/") {
        return Some("module_edge".to_owned());
    }

    // Door pieces under platforms or named door/frame.
    if file_stem.contains("door")
        || file_stem.contains("frame") && lower_path.contains("door")
        || (lower_path.contains("/platforms/") && file_stem.contains("door"))
    {
        return Some("doorway".to_owned());
    }

    if file_stem.contains("window") {
        return Some("window_frame".to_owned());
    }

    if lower_path.contains("/walls/")
        || file_stem.starts_with("wall")
        || file_stem.contains("wall_")
        || file_stem.contains("_wall")
        || file_stem.starts_with("shortwall")
        || file_stem.starts_with("bottom")
        || file_stem.starts_with("top") && lower_path.contains("/walls/")
    {
        // Wall-mounted window pieces stay window_frame via file name above.
        return Some("wall_edge".to_owned());
    }

    if lower_path.contains("/platforms/") {
        if file_stem.contains("ramp") || file_stem.contains("stair") {
            return Some("floor_edge".to_owned());
        }
        return Some("floor_edge".to_owned());
    }

    if lower_path.contains("/columns/") {
        return Some("module_edge".to_owned());
    }

    if lower_path.contains("/props/") && file_stem.contains("pipe") {
        return Some("pipe_end".to_owned());
    }

    suggest_class_from_name(&asset.display_name)
        .or_else(|| suggest_class_from_name(&asset.asset_id))
        .or_else(|| suggest_class_from_name(file_stem))
}

/// Suggested semantic fields, restricted to pack vocabulary terms only.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SuggestedSemantics {
    pub semantic_tags: Vec<String>,
    pub affordances: Vec<String>,
    pub placement_constraints: Vec<String>,
}

/// Infer tags / affordances / placement constraints from name, class, and shape.
///
/// Only returns terms already listed in the pack vocabulary (never invents
/// out-of-vocab labels). Empty fields are filled; existing values are kept.
pub fn suggest_semantics_for_asset(
    asset: &AssetRecord,
    connector_classes: &[String],
    vocab: &crate::schema::ControlledVocabulary,
) -> SuggestedSemantics {
    let mut hay = format!(
        "{} {} {}",
        asset.display_name, asset.asset_id, asset.source_path
    );
    for c in connector_classes {
        hay.push(' ');
        hay.push_str(c);
    }
    let lower = hay.to_ascii_lowercase();

    let mut tags = Vec::new();
    let mut affordances = Vec::new();
    let mut constraints = Vec::new();

    let tag_hints: &[(&str, &str)] = &[
        ("wall", "wall"),
        ("floor", "floor"),
        ("door", "door"),
        ("window", "window"),
        ("corridor", "corridor"),
        ("hall", "corridor"),
        ("corner", "corner"),
        ("roof", "roof"),
        ("tile", "floor"),
        ("prop", "prop"),
        ("entry", "entry"),
        ("exit", "exit"),
        ("hazard", "hazard"),
        ("cover", "cover"),
        ("loot", "lootable"),
        ("decor", "decorative"),
        ("walk", "walkable"),
    ];
    for (needle, term) in tag_hints {
        if lower.contains(needle) && vocab.allows_term(&vocab.semantic_tags, term) {
            push_unique(&mut tags, (*term).to_owned());
        }
    }

    // Shape heuristics (still vocabulary-gated).
    let dx = (asset.bounds.max[0] - asset.bounds.min[0]).abs();
    let dy = (asset.bounds.max[1] - asset.bounds.min[1]).abs();
    let dz = (asset.bounds.max[2] - asset.bounds.min[2]).abs();
    if dy > dx.max(dz) * 1.15 && vocab.allows_term(&vocab.semantic_tags, "wall") {
        push_unique(&mut tags, "wall".to_owned());
    }
    if dy < dx.min(dz) * 0.4 && vocab.allows_term(&vocab.semantic_tags, "floor") {
        push_unique(&mut tags, "floor".to_owned());
    }

    let aff_hints: &[(&str, &str)] = &[
        ("door", "openable"),
        ("window", "provide_cover"),
        ("wall", "block_movement"),
        ("floor", "walkable"),
        ("tile", "walkable"),
        ("corridor", "walkable"),
        ("climb", "climbable"),
        ("seat", "sittable"),
        ("sit", "sittable"),
        ("light", "light_source"),
        ("interact", "interactable"),
        ("cover", "provide_cover"),
    ];
    for (needle, term) in aff_hints {
        if lower.contains(needle) && vocab.allows_term(&vocab.affordances, term) {
            push_unique(&mut affordances, (*term).to_owned());
        }
    }
    if tags.iter().any(|t| t == "floor" || t == "walkable")
        && vocab.allows_term(&vocab.affordances, "walkable")
    {
        push_unique(&mut affordances, "walkable".to_owned());
    }
    if tags.iter().any(|t| t == "wall") && vocab.allows_term(&vocab.affordances, "block_movement") {
        push_unique(&mut affordances, "block_movement".to_owned());
    }

    let constraint_hints: &[(&str, &str)] = &[
        ("wall", "grounded"),
        ("floor", "grounded"),
        ("floor", "requires_floor"),
        ("tile", "grounded"),
        ("door", "requires_wall"),
        ("window", "wall_mounted"),
        ("ceiling", "ceiling_mounted"),
        ("roof", "grounded"),
        ("prop", "grounded"),
    ];
    for (needle, term) in constraint_hints {
        if lower.contains(needle) && vocab.allows_term(&vocab.placement_constraints, term) {
            push_unique(&mut constraints, (*term).to_owned());
        }
    }
    if dy > dx.max(dz) * 0.5 && vocab.allows_term(&vocab.placement_constraints, "upright_only") {
        push_unique(&mut constraints, "upright_only".to_owned());
    }

    SuggestedSemantics {
        semantic_tags: tags,
        affordances,
        placement_constraints: constraints,
    }
}

fn push_unique(list: &mut Vec<String>, value: String) {
    if !list.iter().any(|v| v == &value) {
        list.push(value);
    }
}

/// Six axis-aligned faces of an AABB, each as a connector-ready frame on the face center.
///
/// Orientation maps local **+Z** to the outward face normal and local **+Y** toward
/// `up_reference`, so engine importers that only apply the quaternion (assuming +Z
/// forward) face connectors correctly. `mating_axis` is always `pos_z` and
/// `up_reference` is always `pos_y` in local connector space.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceSnap {
    pub name: &'static str,
    pub position: Vec3,
    pub orientation_quat_xyzw: QuatXyzw,
    pub mating_axis: Axis3,
    pub up_reference: Axis3,
}

pub fn bounds_face_snaps(bounds: &Bounds3) -> [FaceSnap; 6] {
    let cx = (bounds.min[0] + bounds.max[0]) * 0.5;
    let cy = (bounds.min[1] + bounds.max[1]) * 0.5;
    let cz = (bounds.min[2] + bounds.max[2]) * 0.5;

    [
        face_snap("pos_x", [bounds.max[0], cy, cz], GVec3::X, GVec3::Y),
        face_snap("neg_x", [bounds.min[0], cy, cz], -GVec3::X, GVec3::Y),
        face_snap("pos_y", [cx, bounds.max[1], cz], GVec3::Y, GVec3::Z),
        face_snap("neg_y", [cx, bounds.min[1], cz], -GVec3::Y, GVec3::Z),
        face_snap("pos_z", [cx, cy, bounds.max[2]], GVec3::Z, GVec3::Y),
        face_snap("neg_z", [cx, cy, bounds.min[2]], -GVec3::Z, GVec3::Y),
    ]
}

fn face_snap(name: &'static str, position: Vec3, outward: GVec3, up_hint: GVec3) -> FaceSnap {
    FaceSnap {
        name,
        position,
        orientation_quat_xyzw: orientation_facing(outward, up_hint),
        // Local mating / up after orientation: +Z faces outward, +Y is up.
        mating_axis: Axis3::PosZ,
        up_reference: Axis3::PosY,
    }
}

/// Build a quaternion that maps local +Z → `outward` and local +Y → projected `up_hint`.
fn orientation_facing(outward: GVec3, up_hint: GVec3) -> QuatXyzw {
    let z = outward.normalize();
    let mut y = up_hint - z * up_hint.dot(z);
    if y.length_squared() < 1e-8 {
        let alt = if z.x.abs() < 0.9 { GVec3::X } else { GVec3::Y };
        y = alt - z * alt.dot(z);
    }
    y = y.normalize();
    let x = y.cross(z).normalize();
    y = z.cross(x).normalize();
    Quat::from_mat3(&Mat3::from_cols(x, y, z))
        .normalize()
        .to_array()
}

/// Snap a 3D connector onto the nearest AABB face center (by position distance).
pub fn snap_connector_to_nearest_face(connector: &mut ConnectorRecord, bounds: &Bounds3) {
    let ConnectorFrame::Frame3d { position, .. } = &connector.frame else {
        return;
    };
    let pos = *position;
    let snaps = bounds_face_snaps(bounds);
    let mut best = &snaps[0];
    let mut best_dist = f32::INFINITY;
    for snap in &snaps {
        let dx = snap.position[0] - pos[0];
        let dy = snap.position[1] - pos[1];
        let dz = snap.position[2] - pos[2];
        let dist = dx * dx + dy * dy + dz * dz;
        if dist < best_dist {
            best_dist = dist;
            best = snap;
        }
    }
    connector.frame = ConnectorFrame::Frame3d {
        position: best.position,
        orientation_quat_xyzw: best.orientation_quat_xyzw,
    };
    connector.mating_axis = best.mating_axis;
    connector.up_reference = best.up_reference;
}

/// Deep-clone a connector with a new id/name, offset slightly so it is visible.
pub fn duplicate_connector(source: &ConnectorRecord, new_id: String) -> ConnectorRecord {
    let mut clone = source.clone();
    clone.connector_id = new_id;
    clone.display_name = format!("{} Copy", source.display_name);
    match &mut clone.frame {
        ConnectorFrame::Frame3d { position, .. } => {
            position[0] += 0.1;
        }
        ConnectorFrame::Frame2d { position, .. } => {
            position[0] += 1.0;
        }
    }
    clone
}

/// Create a 3D connector on a named bounds face (`pos_x`, `neg_z`, …).
pub fn connector_on_face(
    asset: &AssetRecord,
    face: &str,
    connector_id: String,
    class: String,
) -> Option<ConnectorRecord> {
    let snap = bounds_face_snaps(&asset.bounds)
        .into_iter()
        .find(|snap| snap.name == face)?;
    Some(ConnectorRecord {
        connector_id: connector_id.clone(),
        display_name: title_from_id(&connector_id),
        class,
        role: ConnectorRole::Symmetric,
        frame: ConnectorFrame::Frame3d {
            position: snap.position,
            orientation_quat_xyzw: snap.orientation_quat_xyzw,
        },
        mating_axis: snap.mating_axis,
        up_reference: snap.up_reference,
        snap_tolerance: 0.01,
        face_size: None,
    })
}

fn title_from_id(id: &str) -> String {
    id.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
