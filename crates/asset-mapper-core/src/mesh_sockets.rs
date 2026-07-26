//! Mesh-aware mating socket proposal for modular kits.
//!
//! Strategy: for each AABB extremal face, sample mesh points near the face,
//! place a socket at the **surface centroid** (better than box center for
//! L-shapes). If a large interior empty region exists on the face occupancy
//! grid (door/window opening), place the socket at the **portal center**.

use glam::{Mat3, Quat, Vec3 as GVec3};

use crate::mesh_geometry::MeshGeometry;
use crate::schema::{Axis3, Bounds3, QuatXyzw, Vec3};

const MAX_SAMPLES: usize = 20_000;
const GRID: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketSource {
    MeshSurface,
    MeshPortal,
    BoundsFallback,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProposedSocket {
    pub name: String,
    pub position: Vec3,
    pub orientation_quat_xyzw: QuatXyzw,
    pub mating_axis: Axis3,
    pub up_reference: Axis3,
    /// Relative importance (higher = prefer keep when capping).
    pub score: f32,
    /// Face-plane span [u, v] for class heuristics.
    pub face_span: [f32; 2],
    pub source: SocketSource,
}

#[derive(Debug, Clone)]
pub struct SocketProposeOptions {
    /// Max sockets per asset (after scoring).
    pub max_sockets: usize,
    /// Min face span as fraction of longest bounds dim.
    pub min_face_span_ratio: f32,
    /// Prefer skipping top/bottom for wall-like modules when class hint says wall.
    pub skip_vertical_for_walls: bool,
}

impl Default for SocketProposeOptions {
    fn default() -> Self {
        Self {
            max_sockets: 12,
            min_face_span_ratio: 0.05,
            skip_vertical_for_walls: true,
        }
    }
}

/// Propose sockets from mesh geometry + asset bounds.
pub fn propose_sockets_from_mesh(
    mesh: &MeshGeometry,
    bounds: &Bounds3,
    options: &SocketProposeOptions,
    wall_like: bool,
) -> Vec<ProposedSocket> {
    if mesh.is_empty() {
        return Vec::new();
    }

    let dims = [
        (bounds.max[0] - bounds.min[0]).abs(),
        (bounds.max[1] - bounds.min[1]).abs(),
        (bounds.max[2] - bounds.min[2]).abs(),
    ];
    let longest = dims[0].max(dims[1]).max(dims[2]).max(1e-6);
    let min_span = longest * options.min_face_span_ratio;
    let plane_eps = (longest * 0.04).max(1e-4);

    let samples = mesh.sample_points(MAX_SAMPLES);
    if samples.is_empty() {
        return Vec::new();
    }

    let faces: [(&str, usize, bool, GVec3, GVec3); 6] = [
        ("pos_x", 0, true, GVec3::X, GVec3::Y),
        ("neg_x", 0, false, -GVec3::X, GVec3::Y),
        ("pos_y", 1, true, GVec3::Y, GVec3::Z),
        ("neg_y", 1, false, -GVec3::Y, GVec3::Z),
        ("pos_z", 2, true, GVec3::Z, GVec3::Y),
        ("neg_z", 2, false, -GVec3::Z, GVec3::Y),
    ];

    let mut proposed = Vec::new();

    for (name, axis, is_max, outward, up_hint) in faces {
        if options.skip_vertical_for_walls && wall_like && (name == "pos_y" || name == "neg_y") {
            continue;
        }

        let face_ok = match name {
            "pos_x" | "neg_x" => dims[1] >= min_span && dims[2] >= min_span,
            "pos_y" | "neg_y" => dims[0] >= min_span && dims[2] >= min_span,
            "pos_z" | "neg_z" => dims[0] >= min_span && dims[1] >= min_span,
            _ => true,
        };
        if !face_ok {
            continue;
        }

        let plane = if is_max {
            bounds.max[axis]
        } else {
            bounds.min[axis]
        };

        let mut face_pts: Vec<Vec3> = samples
            .iter()
            .copied()
            .filter(|p| (p[axis] - plane).abs() <= plane_eps)
            .collect();

        // If nothing sits on the slab, try slightly thicker for thin walls.
        if face_pts.len() < 3 {
            let thick = plane_eps * 2.5;
            face_pts = samples
                .iter()
                .copied()
                .filter(|p| (p[axis] - plane).abs() <= thick)
                .collect();
        }
        if face_pts.len() < 3 {
            continue;
        }

        let (u_axis, v_axis) = match axis {
            0 => (1usize, 2usize),
            1 => (0usize, 2usize),
            _ => (0usize, 1usize),
        };

        let mut u_min = f32::INFINITY;
        let mut u_max = f32::NEG_INFINITY;
        let mut v_min = f32::INFINITY;
        let mut v_max = f32::NEG_INFINITY;
        let mut cu = 0.0f32;
        let mut cv = 0.0f32;
        for p in &face_pts {
            let u = p[u_axis];
            let v = p[v_axis];
            u_min = u_min.min(u);
            u_max = u_max.max(u);
            v_min = v_min.min(v);
            v_max = v_max.max(v);
            cu += u;
            cv += v;
        }
        let n = face_pts.len() as f32;
        cu /= n;
        cv /= n;

        let span_u = (u_max - u_min).max(1e-6);
        let span_v = (v_max - v_min).max(1e-6);
        let face_span = [span_u, span_v];

        // Occupancy grid for portal detection.
        let mut grid = [[false; GRID]; GRID];
        for p in &face_pts {
            let iu = (((p[u_axis] - u_min) / span_u) * (GRID as f32 - 1e-4)) as usize;
            let iv = (((p[v_axis] - v_min) / span_v) * (GRID as f32 - 1e-4)) as usize;
            let iu = iu.min(GRID - 1);
            let iv = iv.min(GRID - 1);
            grid[iv][iu] = true;
        }

        let portal = largest_interior_empty(&grid);
        let (su, sv, source, score_boost) = if let Some((pu, pv, cells)) = portal {
            // Require empty region to be meaningful vs face.
            let area_frac = cells as f32 / (GRID * GRID) as f32;
            if area_frac >= 0.04 && cells >= 6 {
                let u = u_min + (pu as f32 + 0.5) / GRID as f32 * span_u;
                let v = v_min + (pv as f32 + 0.5) / GRID as f32 * span_v;
                (u, v, SocketSource::MeshPortal, 2.0 + area_frac)
            } else {
                (cu, cv, SocketSource::MeshSurface, 1.0)
            }
        } else {
            (cu, cv, SocketSource::MeshSurface, 1.0)
        };

        let mut position = [0.0; 3];
        position[axis] = plane;
        position[u_axis] = su;
        position[v_axis] = sv;

        // Density score: more samples near placement → more reliable.
        let coverage = (face_pts.len() as f32).ln().max(1.0);
        let score = score_boost * coverage * (span_u * span_v).sqrt() / longest;

        proposed.push(ProposedSocket {
            name: name.to_owned(),
            position,
            orientation_quat_xyzw: orientation_facing(outward, up_hint),
            mating_axis: Axis3::PosZ,
            up_reference: Axis3::PosY,
            score,
            face_span,
            source,
        });
    }

    // Sort by score descending, dedup close positions, cap.
    proposed.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut kept: Vec<ProposedSocket> = Vec::new();
    let dedup_dist = longest * 0.08;
    for sock in proposed {
        let too_close = kept.iter().any(|k| {
            let dx = k.position[0] - sock.position[0];
            let dy = k.position[1] - sock.position[1];
            let dz = k.position[2] - sock.position[2];
            (dx * dx + dy * dy + dz * dz).sqrt() < dedup_dist
        });
        if too_close {
            continue;
        }
        kept.push(sock);
        if kept.len() >= options.max_sockets {
            break;
        }
    }

    kept
}

/// AABB face-center fallback sockets (same as classic analyze).
pub fn propose_sockets_from_bounds(
    bounds: &Bounds3,
    options: &SocketProposeOptions,
    wall_like: bool,
) -> Vec<ProposedSocket> {
    let dims = [
        (bounds.max[0] - bounds.min[0]).abs(),
        (bounds.max[1] - bounds.min[1]).abs(),
        (bounds.max[2] - bounds.min[2]).abs(),
    ];
    let longest = dims[0].max(dims[1]).max(dims[2]).max(1e-6);
    let min_span = longest * options.min_face_span_ratio;
    let cx = (bounds.min[0] + bounds.max[0]) * 0.5;
    let cy = (bounds.min[1] + bounds.max[1]) * 0.5;
    let cz = (bounds.min[2] + bounds.max[2]) * 0.5;

    let faces = [
        (
            "pos_x",
            [bounds.max[0], cy, cz],
            GVec3::X,
            GVec3::Y,
            [dims[1], dims[2]],
        ),
        (
            "neg_x",
            [bounds.min[0], cy, cz],
            -GVec3::X,
            GVec3::Y,
            [dims[1], dims[2]],
        ),
        (
            "pos_y",
            [cx, bounds.max[1], cz],
            GVec3::Y,
            GVec3::Z,
            [dims[0], dims[2]],
        ),
        (
            "neg_y",
            [cx, bounds.min[1], cz],
            -GVec3::Y,
            GVec3::Z,
            [dims[0], dims[2]],
        ),
        (
            "pos_z",
            [cx, cy, bounds.max[2]],
            GVec3::Z,
            GVec3::Y,
            [dims[0], dims[1]],
        ),
        (
            "neg_z",
            [cx, cy, bounds.min[2]],
            -GVec3::Z,
            GVec3::Y,
            [dims[0], dims[1]],
        ),
    ];

    let mut out = Vec::new();
    for (name, position, outward, up_hint, face_span) in faces {
        if options.skip_vertical_for_walls && wall_like && (name == "pos_y" || name == "neg_y") {
            continue;
        }
        if face_span[0] < min_span || face_span[1] < min_span {
            continue;
        }
        out.push(ProposedSocket {
            name: name.to_owned(),
            position,
            orientation_quat_xyzw: orientation_facing(outward, up_hint),
            mating_axis: Axis3::PosZ,
            up_reference: Axis3::PosY,
            score: 0.5,
            face_span,
            source: SocketSource::BoundsFallback,
        });
    }
    out
}

/// Find largest empty axis-aligned rectangle fully interior (not on border).
/// Returns (center_u, center_v, cell_count).
fn largest_interior_empty(grid: &[[bool; GRID]; GRID]) -> Option<(usize, usize, usize)> {
    let mut best_area = 0usize;
    let mut best = None;

    // Histogram method for largest empty rectangle in binary matrix (empty = free).
    let mut height = [0usize; GRID];
    for (row, grid_row) in grid.iter().enumerate() {
        for (col, occupied) in grid_row.iter().enumerate() {
            if *occupied {
                height[col] = 0;
            } else {
                height[col] += 1;
            }
        }
        // Largest rectangle in histogram (sentinel col == GRID with height 0).
        let mut stack: Vec<(usize, usize)> = Vec::new(); // (start_col, height)
        for col in 0..=GRID {
            let h = height.get(col).copied().unwrap_or(0);
            let mut start = col;
            while let Some(&(sc, sh)) = stack.last() {
                if sh <= h {
                    break;
                }
                stack.pop();
                let width = col - sc;
                let area = sh * width;
                // Prefer interior: center not on outer border of full grid.
                let c_row = row + 1 - sh / 2;
                let c_col = sc + width / 2;
                let interior = c_row > 0 && c_row < GRID - 1 && c_col > 0 && c_col < GRID - 1;
                if interior && area > best_area {
                    best_area = area;
                    best = Some((c_col, c_row.min(GRID - 1), area));
                }
                start = sc;
            }
            stack.push((start, h));
        }
    }

    best
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_geometry::MeshGeometry;

    fn box_mesh(min: Vec3, max: Vec3) -> MeshGeometry {
        // 8 corners only (point cloud) — surface samples still hit faces.
        let mut positions = Vec::new();
        for x in [min[0], max[0]] {
            for y in [min[1], max[1]] {
                for z in [min[2], max[2]] {
                    positions.push([x, y, z]);
                }
            }
        }
        // Dense face samples on +Z face, offset portal hole (empty middle).
        let z = max[2];
        for i in 0..20 {
            for j in 0..20 {
                let u = min[0] + (i as f32 / 19.0) * (max[0] - min[0]);
                let v = min[1] + (j as f32 / 19.0) * (max[1] - min[1]);
                // Leave a hole on the right half.
                let in_hole = u > (min[0] + max[0]) * 0.55
                    && u < max[0] - 0.05
                    && v > min[1] + 0.2
                    && v < max[1] - 0.2;
                if !in_hole {
                    positions.push([u, v, z]);
                }
            }
        }
        MeshGeometry {
            positions,
            indices: None,
        }
    }

    #[test]
    fn mesh_portal_shifts_from_box_center() {
        let min = [-1.0, 0.0, -0.1];
        let max = [1.0, 2.0, 0.1];
        let bounds = Bounds3 { min, max };
        let mesh = box_mesh(min, max);
        let socks =
            propose_sockets_from_mesh(&mesh, &bounds, &SocketProposeOptions::default(), true);
        let pos_z = socks.iter().find(|s| s.name == "pos_z");
        assert!(pos_z.is_some(), "expected pos_z socket");
        let sock = pos_z.unwrap();
        // Portal / surface should bias +X (hole is on the right), not dead center.
        assert!(
            sock.position[0] > 0.05 || sock.source == SocketSource::MeshPortal,
            "expected socket shifted toward portal, got {:?}",
            sock.position
        );
    }

    #[test]
    fn bounds_fallback_emits_faces() {
        let bounds = Bounds3 {
            min: [-1.0, 0.0, -0.1],
            max: [1.0, 2.0, 0.1],
        };
        let socks = propose_sockets_from_bounds(&bounds, &SocketProposeOptions::default(), true);
        assert!(socks.len() >= 2);
        assert!(
            socks
                .iter()
                .all(|s| s.source == SocketSource::BoundsFallback)
        );
    }
}
