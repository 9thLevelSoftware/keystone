//! Auto-analyze packs: propose connectors on bounds faces and class/rule wiring.
//!
//! This is the default authoring path: machine proposes, human tweaks.

use std::collections::BTreeSet;

use crate::schema::{
    AllowedRotation, AssetRecord, AssetType, CompatibilityRule, ConnectorClass, ConnectorFrame,
    ConnectorRecord, ConnectorRole, PackRecord, ReviewFlag,
};
use crate::suggest::{bounds_face_snaps, suggest_class_from_name};

/// Options controlling auto-analysis.
#[derive(Debug, Clone)]
pub struct AnalyzeOptions {
    /// Replace existing connectors on each asset (default: only fill empty).
    pub replace_existing_connectors: bool,
    /// Skip faces thinner than this fraction of the longest dimension (default 0.05).
    pub min_face_span_ratio: f32,
    /// Prefer horizontal mates (wall_edge) vs vertical (floor_edge) heuristics.
    pub modular_kit_mode: bool,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            replace_existing_connectors: false,
            min_face_span_ratio: 0.05,
            modular_kit_mode: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AnalyzeReport {
    pub assets_processed: usize,
    pub connectors_added: usize,
    pub classes_added: usize,
    pub rules_added: usize,
    pub skipped_assets: Vec<String>,
    pub notes: Vec<String>,
}

/// Mutate `pack` in place: measure-independent auto connectors + class/rules.
///
/// Call after bounds measurement for best results. Assets with placeholder
/// unit-cube bounds still get face connectors (authors should re-measure).
pub fn analyze_pack(pack: &mut PackRecord, options: &AnalyzeOptions) -> AnalyzeReport {
    let mut report = AnalyzeReport {
        assets_processed: 0,
        connectors_added: 0,
        classes_added: 0,
        rules_added: 0,
        skipped_assets: Vec::new(),
        notes: Vec::new(),
    };

    // Track which class names we introduce this pass.
    let mut used_classes: BTreeSet<String> = pack
        .connector_classes
        .iter()
        .map(|c| c.class.clone())
        .collect();
    let mut new_classes: Vec<(String, String)> = Vec::new();

    for asset in &mut pack.assets {
        report.assets_processed += 1;

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

        let class = class_for_asset(asset, options);
        if used_classes.insert(class.clone()) {
            new_classes.push((class.clone(), title_case(&class)));
            report.classes_added += 1;
        }

        let added = auto_connectors_for_asset(asset, &class, options);
        if added == 0 {
            report.skipped_assets.push(format!(
                "{} (no eligible faces — check bounds)",
                asset.asset_id
            ));
        } else {
            report.connectors_added += added;
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

    // After assets, ensure every used class participates in a rule.
    report.rules_added += ensure_compatibility_rules(pack);

    if report.connectors_added == 0 {
        report.notes.push(
            "No connectors were added. Measure bounds and run Analyze again, or enable replace."
                .to_owned(),
        );
    } else {
        report.notes.push(
            "Connectors and rules were proposed automatically. Review classes and tweak frames in the editor."
                .to_owned(),
        );
    }

    report
}

fn class_for_asset(asset: &AssetRecord, options: &AnalyzeOptions) -> String {
    if let Some(from_name) =
        suggest_class_from_name(&asset.display_name).or_else(|| suggest_class_from_name(&asset.asset_id))
    {
        return from_name;
    }

    if matches!(asset.asset_type, AssetType::Tile2d | AssetType::Sprite2d) {
        return "tile_edge".to_owned();
    }

    if options.modular_kit_mode {
        // Tall thin → wall; flat → floor; else generic edge.
        let dx = (asset.bounds.max[0] - asset.bounds.min[0]).abs();
        let dy = (asset.bounds.max[1] - asset.bounds.min[1]).abs();
        let dz = (asset.bounds.max[2] - asset.bounds.min[2]).abs();
        if dy > dx.max(dz) * 1.2 {
            return "wall_edge".to_owned();
        }
        if dy < dx.min(dz) * 0.35 {
            return "floor_edge".to_owned();
        }
    }

    "module_edge".to_owned()
}

fn auto_connectors_for_asset(
    asset: &mut AssetRecord,
    class: &str,
    options: &AnalyzeOptions,
) -> usize {
    if matches!(asset.asset_type, AssetType::Tile2d | AssetType::Sprite2d) {
        return auto_connectors_2d(asset, class);
    }

    let dims = [
        (asset.bounds.max[0] - asset.bounds.min[0]).abs(),
        (asset.bounds.max[1] - asset.bounds.min[1]).abs(),
        (asset.bounds.max[2] - asset.bounds.min[2]).abs(),
    ];
    let longest = dims[0].max(dims[1]).max(dims[2]).max(1e-6);
    let min_span = longest * options.min_face_span_ratio;

    let snaps = bounds_face_snaps(&asset.bounds);
    let mut added = 0usize;
    let mut used_ids: BTreeSet<String> = asset
        .connectors
        .iter()
        .map(|c| c.connector_id.clone())
        .collect();

    for snap in &snaps {
        // Face spans on the two axes orthogonal to the outward normal.
        let face_ok = match snap.name {
            "pos_x" | "neg_x" => dims[1] >= min_span && dims[2] >= min_span,
            "pos_y" | "neg_y" => dims[0] >= min_span && dims[2] >= min_span,
            "pos_z" | "neg_z" => dims[0] >= min_span && dims[1] >= min_span,
            _ => true,
        };
        if !face_ok {
            continue;
        }

        // Modular kits: skip top/bottom for wall-like pieces (prefer horizontal mates).
        if options.modular_kit_mode && class.contains("wall") && (snap.name == "pos_y" || snap.name == "neg_y")
        {
            continue;
        }
        if options.modular_kit_mode
            && class.contains("floor")
            && (snap.name == "pos_y" || snap.name == "neg_y")
        {
            continue;
        }

        let base_id = format!("{}_{}", asset.asset_id, snap.name);
        let connector_id = unique_id(&base_id, &mut used_ids);
        asset.connectors.push(ConnectorRecord {
            connector_id: connector_id.clone(),
            display_name: title_case(&connector_id),
            class: class.to_owned(),
            role: ConnectorRole::Symmetric,
            frame: ConnectorFrame::Frame3d {
                position: snap.position,
                orientation_quat_xyzw: snap.orientation_quat_xyzw,
            },
            mating_axis: snap.mating_axis,
            up_reference: snap.up_reference,
            snap_tolerance: 0.01,
        });
        added += 1;
    }

    added
}

fn auto_connectors_2d(asset: &mut AssetRecord, class: &str) -> usize {
    // Four edge midpoints in XY (z=0 plane).
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
        });
        added += 1;
    }
    added
}

fn ensure_compatibility_rules(pack: &mut PackRecord) -> usize {
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
    for class in &classes {
        let key = (class.clone(), class.clone());
        if existing.insert(key) {
            pack.compatibility_rules.push(CompatibilityRule {
                a_class: class.clone(),
                b_class: class.clone(),
                rotation: AllowedRotation::StepsDeg {
                    values: vec![0.0, 90.0, 180.0, 270.0],
                },
            });
            added += 1;
        }
    }

    // Cross-class mates that often pair in modular kits.
    let cross = [("doorway", "wall_edge"), ("window_frame", "wall_edge")];
    for (a, b) in cross {
        if classes.contains(a) && classes.contains(b) {
            let mut pair = [a.to_owned(), b.to_owned()];
            pair.sort();
            let key = (pair[0].clone(), pair[1].clone());
            if existing.insert(key) {
                pack.compatibility_rules.push(CompatibilityRule {
                    a_class: a.to_owned(),
                    b_class: b.to_owned(),
                    rotation: AllowedRotation::Locked,
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
