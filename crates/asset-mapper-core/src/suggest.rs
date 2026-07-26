//! Authoring-speed helpers (class suggestions, face snaps, connector clones).

use glam::{Mat3, Quat, Vec3 as GVec3};

use crate::schema::{
    AssetRecord, Axis3, Bounds3, ConnectorFrame, ConnectorRecord, ConnectorRole, QuatXyzw, Vec3,
};

/// Suggest a connector class name from a free-text label (display name or id).
pub fn suggest_class_from_name(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let patterns: &[(&str, &str)] = &[
        ("door", "doorway"),
        ("arch", "archway"),
        ("window", "window_frame"),
        ("floor", "floor_edge"),
        ("ceiling", "ceiling_edge"),
        ("wall", "wall_edge"),
        ("corridor", "corridor_end"),
        ("hall", "corridor_end"),
        ("stair", "stair_landing"),
        ("roof", "roof_edge"),
        ("pipe", "pipe_end"),
        ("socket", "socket"),
        ("plug", "plug"),
        ("tile", "tile_edge"),
        ("edge", "edge"),
        ("corner", "corner"),
        ("center", "center"),
    ];

    for (needle, class_name) in patterns {
        if lower.contains(needle) {
            return Some((*class_name).to_owned());
        }
    }
    None
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
