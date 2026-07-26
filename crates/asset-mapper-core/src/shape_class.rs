//! Geometry-first modular class inference (names optional, not required).
//!
//! Classifying by filename alone fails on anonymized packs (`mesh_01.glb`).
//! Shape families come from AABB aspect ratios; openings come from strong
//! portal measurements on face sockets.

use crate::mesh_sockets::{ProposedSocket, SocketSource};
use crate::schema::{AssetRecord, Bounds3};

/// Coarse modular shape family from bounds only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeFamily {
    /// Tall slab: wall panel (height dominates thickness).
    WallSlab,
    /// Flat plate: floor/platform (height much smaller than footprint).
    FloorPlate,
    /// Thin tall opening frame (door-like): height large, one horizontal dim thin.
    DoorFrame,
    /// Column / post: tall with roughly square footprint.
    Column,
    /// Generic prop / unknown.
    Module,
}

/// Infer shape family purely from axis-aligned bounds.
pub fn shape_family_from_bounds(bounds: &Bounds3) -> ShapeFamily {
    let dx = (bounds.max[0] - bounds.min[0]).abs().max(1e-6);
    let dy = (bounds.max[1] - bounds.min[1]).abs().max(1e-6);
    let dz = (bounds.max[2] - bounds.min[2]).abs().max(1e-6);

    let horiz_max = dx.max(dz);
    let horiz_min = dx.min(dz);
    let thickness = horiz_min;
    let length = horiz_max;

    // Floor: very flat
    if dy < horiz_max * 0.35 && dy < horiz_min * 0.5 {
        return ShapeFamily::FloorPlate;
    }

    // Door frame: tall, one horizontal axis thin, other moderate-wide
    // (classic modular door: thin in Z, wide in X, tall in Y)
    let thin = thickness / length;
    if dy > length * 0.85 && thin < 0.28 && length > thickness * 2.5 {
        // Prefer door over wall when the *other* horizontal span is large enough
        // for a passage (not a thin wall strip).
        if length > dy * 0.55 {
            return ShapeFamily::DoorFrame;
        }
    }

    // Column: tall, footprint nearly square and compact vs height
    if dy > horiz_max * 1.4 && (horiz_max / horiz_min) < 1.45 && horiz_max < dy * 0.55 {
        return ShapeFamily::Column;
    }

    // Wall slab: tall relative to thickness; elongated in one horizontal axis
    if dy > thickness * 1.5 && (dy > horiz_max * 0.45 || length > thickness * 2.0) {
        return ShapeFamily::WallSlab;
    }

    // Secondary wall: height is longest dim
    if dy >= dx && dy >= dz && dy > thickness * 1.2 {
        return ShapeFamily::WallSlab;
    }

    ShapeFamily::Module
}

impl ShapeFamily {
    pub fn base_class(self) -> &'static str {
        match self {
            ShapeFamily::WallSlab => "wall_edge",
            ShapeFamily::FloorPlate => "floor_edge",
            ShapeFamily::DoorFrame => "doorway",
            ShapeFamily::Column => "module_edge",
            ShapeFamily::Module => "module_edge",
        }
    }
}

/// Soft name/path hint — used only as a confidence boost, not sole authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameHint {
    None,
    Door,
    Window,
    Wall,
    Floor,
    Pipe,
    Corridor,
}

pub fn name_hint_from_text(text: &str) -> NameHint {
    let lower = text.to_ascii_lowercase();
    // Avoid matching "door" inside random tokens when possible by word-ish checks.
    if contains_token(&lower, "window") {
        return NameHint::Window;
    }
    if contains_token(&lower, "door") || contains_token(&lower, "doorway") {
        return NameHint::Door;
    }
    if contains_token(&lower, "floor")
        || contains_token(&lower, "platform")
        || contains_token(&lower, "tile")
    {
        return NameHint::Floor;
    }
    if contains_token(&lower, "wall") {
        return NameHint::Wall;
    }
    if contains_token(&lower, "pipe") {
        return NameHint::Pipe;
    }
    if contains_token(&lower, "corridor") || contains_token(&lower, "hall") {
        return NameHint::Corridor;
    }
    NameHint::None
}

fn contains_token(hay: &str, needle: &str) -> bool {
    if !hay.contains(needle) {
        return false;
    }
    // Require non-letter boundaries for short needles to reduce false positives.
    if needle.len() <= 4 {
        return hay.split(|c: char| !c.is_ascii_alphanumeric()).any(|t| t == needle);
    }
    true
}

pub fn name_hint_for_asset(asset: &AssetRecord) -> NameHint {
    let blob = format!(
        "{} {} {}",
        asset.display_name, asset.asset_id, asset.source_path
    );
    name_hint_from_text(&blob)
}

/// Base connector class for an asset: geometry primary, name as soft override.
pub fn base_class_geometry_first(asset: &AssetRecord) -> String {
    let family = shape_family_from_bounds(&asset.bounds);
    let hint = name_hint_for_asset(asset);

    // Geometry first.
    let mut class = family.base_class().to_owned();

    // Soft overrides only when they don't fight geometry absurdly.
    match hint {
        NameHint::Door => {
            // Only force doorway if shape is door-like or wall-like (not a floor plate).
            if matches!(
                family,
                ShapeFamily::DoorFrame | ShapeFamily::WallSlab | ShapeFamily::Module
            ) {
                class = "doorway".to_owned();
            }
        }
        NameHint::Window => {
            if matches!(family, ShapeFamily::WallSlab | ShapeFamily::Module) {
                class = "window_frame".to_owned();
            }
        }
        NameHint::Floor => {
            if matches!(family, ShapeFamily::FloorPlate | ShapeFamily::Module) {
                class = "floor_edge".to_owned();
            }
        }
        NameHint::Wall => {
            if matches!(family, ShapeFamily::WallSlab | ShapeFamily::Module) {
                class = "wall_edge".to_owned();
            }
        }
        NameHint::Pipe => class = "pipe_end".to_owned(),
        NameHint::Corridor => class = "corridor_end".to_owned(),
        NameHint::None => {}
    }

    class
}

/// Opening kind from portal geometry relative to the asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpeningKind {
    None,
    Door,
    Window,
}

/// Classify a strong portal opening using **geometry only** (opening size vs asset).
pub fn opening_kind_from_portal(
    asset: &AssetRecord,
    sock: &ProposedSocket,
) -> OpeningKind {
    if !sock.is_strong_portal && sock.source != SocketSource::MeshPortal {
        return OpeningKind::None;
    }
    if sock.portal_empty_frac < 0.10 {
        return OpeningKind::None;
    }

    let dims_y = (asset.bounds.max[1] - asset.bounds.min[1]).abs().max(1e-3);
    let dx = (asset.bounds.max[0] - asset.bounds.min[0]).abs().max(1e-3);
    let dz = (asset.bounds.max[2] - asset.bounds.min[2]).abs().max(1e-3);
    let horiz = dx.max(dz);

    // face_span is [u,v] in face plane; for vertical faces typically width × height-ish.
    let w = sock.face_span[0].max(1e-4);
    let h = sock.face_span[1].max(1e-4);
    let open_h = h.max(w * 0.35);
    let open_w = w.min(h.max(w));

    let height_frac = open_h / dims_y;
    let width_frac = open_w / horiz;
    let aspect = open_h / open_w;

    // Door: tall opening (large fraction of wall height), moderate width, tall aspect.
    if sock.is_strong_portal
        && height_frac >= 0.55
        && width_frac >= 0.12
        && width_frac <= 0.75
        && aspect >= 1.15
        && sock.portal_empty_frac >= 0.14
    {
        return OpeningKind::Door;
    }

    // Window: elevated / shorter opening, wider aspect range.
    if sock.is_strong_portal
        && height_frac >= 0.18
        && height_frac < 0.55
        && width_frac >= 0.10
        && sock.portal_empty_frac >= 0.12
    {
        return OpeningKind::Window;
    }

    // Very strong centered portal on a thin frame asset → door even if ratios odd.
    let family = shape_family_from_bounds(&asset.bounds);
    if family == ShapeFamily::DoorFrame && sock.is_strong_portal && sock.portal_empty_frac >= 0.12 {
        return OpeningKind::Door;
    }

    OpeningKind::None
}

/// Per-socket class from geometry (+ optional soft name boost).
pub fn class_for_socket_geometry_first(
    asset: &AssetRecord,
    sock: &ProposedSocket,
    base_class: &str,
) -> String {
    // Horizontal faces → floor when asset is floor-like or face is top/bottom of flat piece.
    if sock.name == "pos_y" || sock.name == "neg_y" {
        let family = shape_family_from_bounds(&asset.bounds);
        if matches!(family, ShapeFamily::FloorPlate)
            || base_class == "floor_edge"
            || base_class == "tile_edge"
        {
            return "floor_edge".to_owned();
        }
        // On walls/doors, top/bottom sockets keep base (or skip class noise).
        return base_class.to_owned();
    }

    let opening = opening_kind_from_portal(asset, sock);
    let hint = name_hint_for_asset(asset);

    match opening {
        OpeningKind::Door => {
            // Geometry says door opening — apply even without names.
            return "doorway".to_owned();
        }
        OpeningKind::Window => {
            // Name can refine window vs keep wall if we're unsure; geometry alone → window_frame.
            if matches!(hint, NameHint::Wall) && sock.portal_empty_frac < 0.16 {
                // weak window on wall stays wall_edge
            } else {
                return "window_frame".to_owned();
            }
        }
        OpeningKind::None => {}
    }

    // Door-frame shaped assets: all vertical exterior sockets are doorway mates.
    if base_class == "doorway" || shape_family_from_bounds(&asset.bounds) == ShapeFamily::DoorFrame
    {
        return "doorway".to_owned();
    }

    if base_class == "window_frame" {
        return "window_frame".to_owned();
    }

    // Soft name boost without geometry opening: do not force doorway on plain wall faces.
    base_class.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{AssetType, Axis3, Pivot, ReviewFlag};

    fn bounds(min: [f32; 3], max: [f32; 3]) -> Bounds3 {
        Bounds3 { min, max }
    }

    fn dummy_asset(bounds: Bounds3, path: &str, name: &str) -> AssetRecord {
        AssetRecord {
            asset_id: name.to_owned(),
            source_path: path.to_owned(),
            content_hash: "x".to_owned(),
            display_name: name.to_owned(),
            asset_type: AssetType::Model3d,
            bounds,
            dimensions: [
                (bounds.max[0] - bounds.min[0]).abs(),
                (bounds.max[1] - bounds.min[1]).abs(),
                (bounds.max[2] - bounds.min[2]).abs(),
            ],
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

    #[test]
    fn anonymous_wall_slab_is_wall() {
        let b = bounds([-0.1, 0.0, -2.0], [0.1, 3.0, 2.0]);
        assert_eq!(shape_family_from_bounds(&b), ShapeFamily::WallSlab);
        let a = dummy_asset(b, "mesh_01.gltf", "mesh_01");
        assert_eq!(base_class_geometry_first(&a), "wall_edge");
    }

    #[test]
    fn anonymous_floor_plate_is_floor() {
        let b = bounds([-2.0, 0.0, -2.0], [2.0, 0.15, 2.0]);
        assert_eq!(shape_family_from_bounds(&b), ShapeFamily::FloorPlate);
        let a = dummy_asset(b, "a/b/c.gltf", "piece");
        assert_eq!(base_class_geometry_first(&a), "floor_edge");
    }

    #[test]
    fn anonymous_door_frame_shape() {
        // Wide in X, thin in Z, tall Y — door frame silhouette
        let b = bounds([-1.2, 0.0, -0.08], [1.2, 2.4, 0.08]);
        assert_eq!(shape_family_from_bounds(&b), ShapeFamily::DoorFrame);
        let a = dummy_asset(b, "export/item_7.gltf", "item_7");
        assert_eq!(base_class_geometry_first(&a), "doorway");
    }

    #[test]
    fn strong_portal_becomes_doorway_without_name() {
        let b = bounds([-0.15, 0.0, -2.0], [0.15, 3.0, 2.0]);
        let a = dummy_asset(b, "x/y.gltf", "anon");
        let sock = ProposedSocket {
            name: "pos_x".to_owned(),
            position: [0.15, 1.2, 0.0],
            orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
            mating_axis: Axis3::PosZ,
            up_reference: Axis3::PosY,
            score: 2.0,
            face_span: [1.0, 2.2], // width 1, height 2.2 on a 3m wall
            source: SocketSource::MeshPortal,
            portal_empty_frac: 0.2,
            is_strong_portal: true,
            suggested_role: crate::schema::ConnectorRole::Symmetric,
        };
        assert_eq!(
            class_for_socket_geometry_first(&a, &sock, "wall_edge"),
            "doorway"
        );
    }

    #[test]
    fn weak_portal_stays_wall() {
        let b = bounds([-0.15, 0.0, -2.0], [0.15, 3.0, 2.0]);
        let a = dummy_asset(b, "x/y.gltf", "anon");
        let sock = ProposedSocket {
            name: "pos_x".to_owned(),
            position: [0.15, 1.5, 0.0],
            orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
            mating_axis: Axis3::PosZ,
            up_reference: Axis3::PosY,
            score: 1.0,
            face_span: [0.3, 0.3],
            source: SocketSource::MeshPortal,
            portal_empty_frac: 0.06,
            is_strong_portal: false,
            suggested_role: crate::schema::ConnectorRole::Symmetric,
        };
        assert_eq!(
            class_for_socket_geometry_first(&a, &sock, "wall_edge"),
            "wall_edge"
        );
    }

    #[test]
    fn unused_review_flag_import() {
        let _ = ReviewFlag::BoundsPlaceholder;
    }
}
