//! Auto-analyze packs: propose mesh/bounds connectors and class/rule wiring.
//!
//! This is the default authoring path: machine proposes, human tweaks.

use std::collections::{BTreeMap, BTreeSet};

use crate::mesh_geometry::MeshGeometry;
use crate::mesh_sockets::{
    ProposedSocket, SocketProposeOptions, propose_sockets_from_bounds, propose_sockets_from_mesh,
};
use crate::schema::{
    AllowedRotation, AssetRecord, AssetType, CompatibilityRule, ConnectorClass, ConnectorFrame,
    ConnectorRecord, ConnectorRole, PackRecord, ReviewFlag,
};
use crate::shape_class::{base_class_geometry_first, class_for_socket_geometry_first};
use crate::suggest::suggest_semantics_for_asset;

/// Options controlling auto-analysis.
#[derive(Debug, Clone)]
pub struct AnalyzeOptions {
    /// Replace existing connectors on each asset (default: only fill empty).
    pub replace_existing_connectors: bool,
    /// Skip faces thinner than this fraction of the longest dimension (default 0.05).
    pub min_face_span_ratio: f32,
    /// Prefer horizontal mates (wall_edge) vs vertical (floor_edge) heuristics.
    pub modular_kit_mode: bool,
    /// Skip mesh socket detection and use AABB faces only.
    pub aabb_only: bool,
    /// Skip auto-connectors when `source_path` matches any of these globs
    /// (`*` and `**` supported; case-insensitive). Example: `Decals/**`, `*.png`.
    pub exclude_globs: Vec<String>,
    /// When true, do not propose connectors on Sprite2d / image assets.
    pub skip_images: bool,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            replace_existing_connectors: false,
            min_face_span_ratio: 0.05,
            modular_kit_mode: true,
            aabb_only: false,
            exclude_globs: Vec::new(),
            skip_images: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AnalyzeReport {
    pub assets_processed: usize,
    pub connectors_added: usize,
    pub classes_added: usize,
    pub rules_added: usize,
    /// Assets that received mesh-derived sockets.
    #[serde(default)]
    pub mesh_socket_assets: usize,
    /// Assets that fell back to AABB face centers.
    #[serde(default)]
    pub bounds_fallback_assets: usize,
    pub skipped_assets: Vec<String>,
    pub notes: Vec<String>,
}

/// Mutate `pack` in place using AABB-only proposals (no mesh data).
///
/// Prefer [`analyze_pack_with_meshes`] when geometry is available.
pub fn analyze_pack(pack: &mut PackRecord, options: &AnalyzeOptions) -> AnalyzeReport {
    analyze_pack_with_meshes(pack, options, &BTreeMap::new())
}

/// Analyze with optional per-asset mesh geometry (`asset_id` → mesh).
///
/// When a mesh is present and `aabb_only` is false, connectors are placed from
/// mesh surface / portal detection. Otherwise AABB face centers are used.
pub fn analyze_pack_with_meshes(
    pack: &mut PackRecord,
    options: &AnalyzeOptions,
    meshes: &BTreeMap<String, MeshGeometry>,
) -> AnalyzeReport {
    let mut report = AnalyzeReport {
        assets_processed: 0,
        connectors_added: 0,
        classes_added: 0,
        rules_added: 0,
        mesh_socket_assets: 0,
        bounds_fallback_assets: 0,
        skipped_assets: Vec::new(),
        notes: Vec::new(),
    };

    let mut used_classes: BTreeSet<String> = pack
        .connector_classes
        .iter()
        .map(|c| c.class.clone())
        .collect();
    let mut new_classes: Vec<(String, String)> = Vec::new();

    let socket_opts = SocketProposeOptions {
        max_sockets: 8,
        min_face_span_ratio: options.min_face_span_ratio,
        skip_vertical_for_walls: options.modular_kit_mode,
    };
    let vocabulary = pack.vocabulary.clone();

    for asset in &mut pack.assets {
        report.assets_processed += 1;

        // Skipped assets must remain untouched: do not clear connectors when
        // --replace is set. Only assets we actually re-propose for get cleared.
        if path_excluded(&asset.source_path, &options.exclude_globs) {
            report
                .skipped_assets
                .push(format!("{} (excluded by glob)", asset.asset_id));
            continue;
        }

        if options.skip_images
            && matches!(asset.asset_type, AssetType::Sprite2d | AssetType::Tile2d)
        {
            report
                .skipped_assets
                .push(format!("{} (image/sprite skipped)", asset.asset_id));
            continue;
        }

        if !options.replace_existing_connectors && !asset.connectors.is_empty() {
            report.skipped_assets.push(format!(
                "{} (already has {} connector(s))",
                asset.asset_id,
                asset.connectors.len()
            ));
            continue;
        }

        if options.replace_existing_connectors {
            asset.connectors.clear();
        }

        let base_class = class_for_asset(asset, options);
        let wall_like = base_class.contains("wall") || base_class.contains("corridor");

        let (sockets, used_mesh) =
            if matches!(asset.asset_type, AssetType::Tile2d | AssetType::Sprite2d) {
                (Vec::new(), false)
            } else if options.aabb_only {
                (
                    propose_sockets_from_bounds(&asset.bounds, &socket_opts, wall_like),
                    false,
                )
            } else if let Some(mesh) = meshes.get(&asset.asset_id) {
                let mesh_socks =
                    propose_sockets_from_mesh(mesh, &asset.bounds, &socket_opts, wall_like);
                if mesh_socks.is_empty() {
                    (
                        propose_sockets_from_bounds(&asset.bounds, &socket_opts, wall_like),
                        false,
                    )
                } else {
                    (mesh_socks, true)
                }
            } else {
                (
                    propose_sockets_from_bounds(&asset.bounds, &socket_opts, wall_like),
                    false,
                )
            };

        if matches!(asset.asset_type, AssetType::Tile2d | AssetType::Sprite2d) {
            let class = base_class.clone();
            if used_classes.insert(class.clone()) {
                new_classes.push((class.clone(), title_case(&class)));
                report.classes_added += 1;
            }
            let added = auto_connectors_2d(asset, &class);
            if added == 0 {
                report
                    .skipped_assets
                    .push(format!("{} (no eligible 2d edges)", asset.asset_id));
            } else {
                report.connectors_added += added;
            }
            continue;
        }

        if used_mesh {
            report.mesh_socket_assets += 1;
        } else if !sockets.is_empty() {
            report.bounds_fallback_assets += 1;
            if !asset
                .review_flags
                .contains(&ReviewFlag::AutoFromBoundsFallback)
            {
                asset.review_flags.push(ReviewFlag::AutoFromBoundsFallback);
            }
        }

        let mut used_ids: BTreeSet<String> = asset
            .connectors
            .iter()
            .map(|c| c.connector_id.clone())
            .collect();

        let mut added = 0usize;
        let mut classes_on_asset: Vec<String> = Vec::new();
        for sock in &sockets {
            let class = class_for_socket(asset, sock, &base_class, options);
            if used_classes.insert(class.clone()) {
                new_classes.push((class.clone(), title_case(&class)));
                report.classes_added += 1;
            }
            classes_on_asset.push(class.clone());

            let base_id = format!("{}_{}", asset.asset_id, sock.name);
            let connector_id = unique_id(&base_id, &mut used_ids);
            let snap = snap_tolerance_for(asset, sock);
            let face_size = Some([sock.face_span[0].max(1e-6), sock.face_span[1].max(1e-6)]);
            // Modular kits: default symmetric; only keep inferred roles on door-like classes.
            let role = if class == "doorway" || class == "window_frame" {
                sock.suggested_role.clone()
            } else {
                ConnectorRole::Symmetric
            };
            asset.connectors.push(ConnectorRecord {
                connector_id: connector_id.clone(),
                display_name: title_case(&connector_id),
                class,
                role,
                frame: ConnectorFrame::Frame3d {
                    position: sock.position,
                    orientation_quat_xyzw: sock.orientation_quat_xyzw,
                },
                mating_axis: sock.mating_axis,
                up_reference: sock.up_reference,
                snap_tolerance: snap,
                face_size,
            });
            added += 1;
        }

        if added == 0 {
            report.skipped_assets.push(format!(
                "{} (no eligible faces — check bounds)",
                asset.asset_id
            ));
        } else {
            report.connectors_added += added;
            apply_semantic_suggestions(asset, &classes_on_asset, &vocabulary);
            if asset.review_flags.contains(&ReviewFlag::BoundsPlaceholder) {
                report.notes.push(format!(
                    "{}: connectors proposed on placeholder bounds — re-measure when possible",
                    asset.asset_id
                ));
            }
        }
    }

    for (class, display_name) in new_classes {
        pack.connector_classes.push(ConnectorClass {
            class,
            display_name,
        });
    }

    report.rules_added += synthesize_compatibility_rules(pack);

    if report.connectors_added == 0 {
        report.notes.push(
            "No connectors were added. Measure bounds and run Analyze again, or enable replace."
                .to_owned(),
        );
    } else {
        report.notes.push(format!(
            "Connectors and rules proposed (mesh sockets on {}, AABB fallback on {}). Review and tweak in the editor.",
            report.mesh_socket_assets, report.bounds_fallback_assets
        ));
    }

    report
}

fn class_for_asset(asset: &AssetRecord, options: &AnalyzeOptions) -> String {
    if matches!(asset.asset_type, AssetType::Tile2d | AssetType::Sprite2d) {
        return "tile_edge".to_owned();
    }

    if options.modular_kit_mode {
        // Geometry-first (AABB family + soft name boost). Works without "wall"/"door" filenames.
        return base_class_geometry_first(asset);
    }

    "module_edge".to_owned()
}

/// Class per socket: geometry / portal openings first; filenames optional.
fn class_for_socket(
    asset: &AssetRecord,
    sock: &ProposedSocket,
    base_class: &str,
    options: &AnalyzeOptions,
) -> String {
    if !options.modular_kit_mode {
        return base_class.to_owned();
    }
    class_for_socket_geometry_first(asset, sock, base_class)
}

fn path_excluded(source_path: &str, globs: &[String]) -> bool {
    if globs.is_empty() {
        return false;
    }
    let path = source_path.replace('\\', "/");
    let path_l = path.to_ascii_lowercase();
    globs.iter().any(|g| {
        let pat = g.replace('\\', "/").to_ascii_lowercase();
        glob_match(&pat, &path_l)
    })
}

/// Minimal glob: `*` (segment), `**` (any), case already lowercased.
fn glob_match(pattern: &str, path: &str) -> bool {
    if pattern == "**" || pattern == "*" {
        return true;
    }
    // Exact
    if pattern == path {
        return true;
    }
    // *.ext
    if let Some(ext) = pattern.strip_prefix("*.") {
        return path.ends_with(&format!(".{ext}")) || path.ends_with(ext);
    }
    // prefix/**
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    // **/name
    if let Some(suffix) = pattern.strip_prefix("**/") {
        return path.ends_with(suffix) || path.contains(&format!("/{suffix}"));
    }
    // contains * mid
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.is_empty() {
            return true;
        }
        let mut rest = path;
        if !parts[0].is_empty() {
            if let Some(stripped) = rest.strip_prefix(parts[0]) {
                rest = stripped;
            } else {
                return false;
            }
        }
        for (i, part) in parts.iter().enumerate().skip(1) {
            if part.is_empty() {
                continue;
            }
            if i == parts.len() - 1 {
                return rest.ends_with(part) || rest.contains(part);
            }
            if let Some(idx) = rest.find(part) {
                rest = &rest[idx + part.len()..];
            } else {
                return false;
            }
        }
        return true;
    }
    path.contains(pattern)
}

fn snap_tolerance_for(asset: &AssetRecord, sock: &ProposedSocket) -> f32 {
    let longest = asset.dimensions[0]
        .max(asset.dimensions[1])
        .max(asset.dimensions[2])
        .max(1e-3);
    let face = sock.face_span[0].max(sock.face_span[1]).max(1e-3);
    (face * 0.02).clamp(0.005, longest * 0.05)
}

fn auto_connectors_2d(asset: &mut AssetRecord, class: &str) -> usize {
    let min = asset.bounds.min;
    let max = asset.bounds.max;
    let cx = (min[0] + max[0]) * 0.5;
    let cy = (min[1] + max[1]) * 0.5;
    let edges = [
        ("north", [cx, max[1]], [0.0, 1.0]),
        ("south", [cx, min[1]], [0.0, -1.0]),
        ("east", [max[0], cy], [1.0, 0.0]),
        ("west", [min[0], cy], [-1.0, 0.0]),
    ];

    let mut used_ids: BTreeSet<String> = asset
        .connectors
        .iter()
        .map(|c| c.connector_id.clone())
        .collect();
    let mut added = 0usize;

    for (name, position, normal) in edges {
        let base_id = format!("{}_{}", asset.asset_id, name);
        let connector_id = unique_id(&base_id, &mut used_ids);
        asset.connectors.push(ConnectorRecord {
            connector_id: connector_id.clone(),
            display_name: title_case(&connector_id),
            class: class.to_owned(),
            role: ConnectorRole::Symmetric,
            frame: ConnectorFrame::Frame2d {
                position,
                normal,
                grid_cell: None,
            },
            mating_axis: crate::schema::Axis3::PosZ,
            up_reference: crate::schema::Axis3::PosY,
            snap_tolerance: 0.5,
            face_size: None,
        });
        added += 1;
    }
    added
}

/// Fill empty semantic fields from name/class/shape using pack vocabulary only.
fn apply_semantic_suggestions(
    asset: &mut AssetRecord,
    connector_classes: &[String],
    vocabulary: &crate::schema::ControlledVocabulary,
) {
    let suggested = suggest_semantics_for_asset(asset, connector_classes, vocabulary);
    if asset.semantic_tags.is_empty() {
        asset.semantic_tags = suggested.semantic_tags;
    }
    if asset.affordances.is_empty() {
        asset.affordances = suggested.affordances;
    }
    if asset.placement_constraints.is_empty() {
        asset.placement_constraints = suggested.placement_constraints;
    }
}

/// Rich modular-kit rule ontology + same-class self rules.
fn synthesize_compatibility_rules(pack: &mut PackRecord) -> usize {
    let classes: BTreeSet<String> = pack
        .assets
        .iter()
        .flat_map(|a| a.connectors.iter().map(|c| c.class.clone()))
        .filter(|c| !c.is_empty())
        .collect();

    let mut existing: BTreeSet<(String, String)> = pack
        .compatibility_rules
        .iter()
        .map(|r| {
            let mut pair = [r.a_class.clone(), r.b_class.clone()];
            pair.sort();
            (pair[0].clone(), pair[1].clone())
        })
        .collect();

    let mut added = 0usize;

    // Same-class self-mates with modular rotations.
    for class in &classes {
        let key = (class.clone(), class.clone());
        if existing.insert(key) {
            let rotation = self_rule_rotation(class);
            pack.compatibility_rules.push(CompatibilityRule {
                a_class: class.clone(),
                b_class: class.clone(),
                rotation,
            });
            added += 1;
        }
    }

    // Cross-class ontology (only if both classes present).
    let cross: &[(&str, &str, AllowedRotation)] = &[
        (
            "doorway",
            "wall_edge",
            AllowedRotation::StepsDeg {
                values: vec![0.0, 180.0],
            },
        ),
        ("window_frame", "wall_edge", AllowedRotation::Locked),
        (
            "archway",
            "wall_edge",
            AllowedRotation::StepsDeg {
                values: vec![0.0, 180.0],
            },
        ),
        (
            "archway",
            "corridor_end",
            AllowedRotation::StepsDeg {
                values: vec![0.0, 180.0],
            },
        ),
        (
            "corridor_end",
            "wall_edge",
            AllowedRotation::StepsDeg {
                values: vec![0.0, 180.0],
            },
        ),
        (
            "corridor_end",
            "doorway",
            AllowedRotation::StepsDeg {
                values: vec![0.0, 180.0],
            },
        ),
        (
            "module_edge",
            "wall_edge",
            AllowedRotation::StepsDeg {
                values: vec![0.0, 90.0, 180.0, 270.0],
            },
        ),
        (
            "module_edge",
            "floor_edge",
            AllowedRotation::StepsDeg {
                values: vec![0.0, 90.0, 180.0, 270.0],
            },
        ),
        (
            "module_edge",
            "corridor_end",
            AllowedRotation::StepsDeg {
                values: vec![0.0, 180.0],
            },
        ),
        (
            "tile_edge",
            "floor_edge",
            AllowedRotation::StepsDeg {
                values: vec![0.0, 90.0, 180.0, 270.0],
            },
        ),
        ("pipe_end", "pipe_end", AllowedRotation::Locked),
        (
            "roof_edge",
            "wall_edge",
            AllowedRotation::StepsDeg {
                values: vec![0.0, 180.0],
            },
        ),
    ];

    for (a, b, rotation) in cross {
        if classes.contains(*a) && classes.contains(*b) {
            let mut pair = [(*a).to_owned(), (*b).to_owned()];
            pair.sort();
            let key = (pair[0].clone(), pair[1].clone());
            if existing.insert(key) {
                pack.compatibility_rules.push(CompatibilityRule {
                    a_class: (*a).to_owned(),
                    b_class: (*b).to_owned(),
                    rotation: rotation.clone(),
                });
                added += 1;
            }
        }
    }

    // Ensure pack.connector_classes includes every class used on assets.
    let mut known: BTreeSet<String> = pack
        .connector_classes
        .iter()
        .map(|c| c.class.clone())
        .collect();
    for class in &classes {
        if known.insert(class.clone()) {
            pack.connector_classes.push(ConnectorClass {
                class: class.clone(),
                display_name: title_case(class),
            });
        }
    }

    added
}

fn self_rule_rotation(class: &str) -> AllowedRotation {
    match class {
        "doorway" | "window_frame" | "archway" | "pipe_end" => AllowedRotation::Locked,
        "corridor_end" => AllowedRotation::StepsDeg {
            values: vec![0.0, 180.0],
        },
        _ => AllowedRotation::StepsDeg {
            values: vec![0.0, 90.0, 180.0, 270.0],
        },
    }
}

fn unique_id(base: &str, used: &mut BTreeSet<String>) -> String {
    if used.insert(base.to_owned()) {
        return base.to_owned();
    }
    let mut i = 2;
    loop {
        let candidate = format!("{base}_{i}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        i += 1;
    }
}

fn title_case(id: &str) -> String {
    id.split('_')
        .filter(|p| !p.is_empty())
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
