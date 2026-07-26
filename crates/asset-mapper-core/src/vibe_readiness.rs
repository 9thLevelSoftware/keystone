//! Pack readiness report for vibe builders / external assemblers.
//!
//! Scores whether a pack is machine-mappable: connectors present, classes wired
//! with compatibility rules, and no isolated connector graphs within the pack.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::assembly_propose::rule_partner_map;
use crate::schema::{AssetType, PackRecord};

/// Checklist-style vibe readiness report (JSON-friendly).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VibeReadinessReport {
    /// Overall score 0–100.
    pub score: u32,
    /// True when score ≥ 70 and critical blockers are empty.
    pub ready: bool,
    /// Fraction of 3D model assets with ≥1 connector.
    pub coverage: f32,
    /// Model3d assets with no connectors.
    pub assets_without_connectors: Vec<String>,
    /// Classes used on connectors but with no compatibility partner (incl. self).
    pub orphan_classes: Vec<String>,
    /// Classes that appear on only one free "port" side of the graph (single asset uses
    /// them and they only self-rule or have no other class present to mate with).
    pub dead_end_classes: Vec<String>,
    /// Classes present that cannot mate with any *other* class present in the pack
    /// (self-rules alone do not clear this if only one asset uses the class).
    pub connectivity_gaps: Vec<String>,
    /// Human-readable checklist items (pass/fail style).
    pub checklist: Vec<VibeChecklistItem>,
    /// Notes for authors.
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VibeChecklistItem {
    pub id: String,
    pub ok: bool,
    pub detail: String,
}

/// Compute vibe-builder readiness for a pack.
pub fn vibe_readiness(pack: &PackRecord) -> VibeReadinessReport {
    let model_assets: Vec<_> = pack
        .assets
        .iter()
        .filter(|a| matches!(a.asset_type, AssetType::Model3d))
        .collect();
    let model_count = model_assets.len();
    let with_connectors: Vec<_> = model_assets
        .iter()
        .filter(|a| !a.connectors.is_empty())
        .collect();
    let coverage = if model_count == 0 {
        0.0
    } else {
        with_connectors.len() as f32 / model_count as f32
    };

    let assets_without_connectors: Vec<String> = model_assets
        .iter()
        .filter(|a| a.connectors.is_empty())
        .map(|a| a.asset_id.clone())
        .collect();

    let classes_in_use: BTreeSet<String> = pack
        .assets
        .iter()
        .flat_map(|a| a.connectors.iter().map(|c| c.class.clone()))
        .filter(|c| !c.is_empty())
        .collect();

    let partners = rule_partner_map(pack);

    let mut orphan_classes = Vec::new();
    for class in &classes_in_use {
        let has_rule = partners.get(class).map(|p| !p.is_empty()).unwrap_or(false);
        if !has_rule {
            orphan_classes.push(class.clone());
        }
    }

    // Class → assets that expose it
    let mut class_assets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for asset in &pack.assets {
        for c in &asset.connectors {
            if c.class.is_empty() {
                continue;
            }
            class_assets
                .entry(c.class.clone())
                .or_default()
                .insert(asset.asset_id.clone());
        }
    }

    let mut dead_end_classes = Vec::new();
    let mut connectivity_gaps = Vec::new();

    for class in &classes_in_use {
        let partner_list = partners.get(class).cloned().unwrap_or_default();
        let mates_other_present: Vec<_> = partner_list
            .iter()
            .filter(|p| classes_in_use.contains(*p))
            .cloned()
            .collect();

        if mates_other_present.is_empty() {
            connectivity_gaps.push(class.clone());
            continue;
        }

        // Dead-end: class only mates with itself and appears on a single asset
        // (cannot form multi-piece kits without reuse, which resolver does not support).
        let only_self = mates_other_present.len() == 1 && mates_other_present[0] == *class;
        let asset_count = class_assets.get(class).map(|s| s.len()).unwrap_or(0);
        if only_self && asset_count < 2 {
            dead_end_classes.push(class.clone());
        }
    }

    // Graph connectivity among assets that have connectors.
    let connected_components = asset_connectivity_components(pack, &partners);
    let multi_asset_component = connected_components.iter().any(|c| c.len() >= 2);

    let mut checklist = Vec::new();
    let has_models = model_count > 0;
    checklist.push(VibeChecklistItem {
        id: "has_3d_assets".to_owned(),
        ok: has_models,
        detail: if has_models {
            format!("{model_count} model3d asset(s)")
        } else {
            "No model3d assets in pack".to_owned()
        },
    });

    let coverage_ok = coverage >= 0.8 || (model_count > 0 && assets_without_connectors.is_empty());
    checklist.push(VibeChecklistItem {
        id: "connector_coverage".to_owned(),
        ok: coverage_ok,
        detail: format!(
            "{:.0}% of model3d assets have ≥1 connector ({}/{})",
            coverage * 100.0,
            with_connectors.len(),
            model_count
        ),
    });

    let rules_ok = orphan_classes.is_empty() && !classes_in_use.is_empty();
    checklist.push(VibeChecklistItem {
        id: "class_rules".to_owned(),
        ok: rules_ok,
        detail: if classes_in_use.is_empty() {
            "No connector classes in use".to_owned()
        } else if orphan_classes.is_empty() {
            format!(
                "All {} used class(es) have compatibility rules",
                classes_in_use.len()
            )
        } else {
            format!("{} orphan class(es) lack rules", orphan_classes.len())
        },
    });

    // Checklist multi-piece: N/A when fewer than two mapped assets is still "ok" as a check,
    // but ready requires real multi-piece connectivity (see below).
    let multi_na = with_connectors.len() < 2;
    let multi_ok = multi_asset_component || multi_na;
    checklist.push(VibeChecklistItem {
        id: "multi_piece_connectivity".to_owned(),
        ok: multi_ok,
        detail: if multi_na {
            "Fewer than two assets with connectors — multi-piece N/A".to_owned()
        } else if multi_asset_component {
            "At least two assets can mate via rules".to_owned()
        } else {
            "No pair of assets can mate with current rules/classes".to_owned()
        },
    });

    let no_gaps = connectivity_gaps.is_empty();
    let gaps_waived = !no_gaps && multi_asset_component;
    checklist.push(VibeChecklistItem {
        id: "no_connectivity_gaps".to_owned(),
        ok: no_gaps || gaps_waived,
        detail: if no_gaps {
            "Every used class can mate with a class present in the pack".to_owned()
        } else if gaps_waived {
            format!(
                "{} class(es) lack partners, but a multi-piece mate path still exists",
                connectivity_gaps.len()
            )
        } else {
            format!(
                "{} class(es) cannot mate with any present class",
                connectivity_gaps.len()
            )
        },
    });

    // Score: weighted checklist + coverage
    let mut score = 0u32;
    if has_models {
        score += 15;
    }
    score += (coverage * 35.0).round() as u32;
    if rules_ok {
        score += 20;
    } else if orphan_classes.len() < classes_in_use.len() && !classes_in_use.is_empty() {
        score += 8;
    }
    // Full multi-piece points only when ≥2 assets can actually mate; N/A is a soft partial.
    if multi_asset_component {
        score += 20;
    } else if multi_na && !with_connectors.is_empty() {
        score += 5;
    }
    if dead_end_classes.is_empty() {
        score += 5;
    }
    if assets_without_connectors.is_empty() && model_count > 0 {
        score += 5;
    }
    score = score.min(100);

    // Class monopoly (e.g. everything labeled doorway) is not vibe-ready quality.
    let mut class_counts: HashMap<String, usize> = HashMap::new();
    let mut total_connectors = 0usize;
    for asset in &pack.assets {
        for c in &asset.connectors {
            if c.class.is_empty() {
                continue;
            }
            *class_counts.entry(c.class.clone()).or_default() += 1;
            total_connectors += 1;
        }
    }
    let (dominant_class, dominant_frac) = class_counts
        .iter()
        .map(|(k, v)| (k.as_str(), *v as f32 / total_connectors.max(1) as f32))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(("", 0.0));
    let monopoly = total_connectors >= 8 && dominant_frac > 0.6;
    checklist.push(VibeChecklistItem {
        id: "no_class_monopoly".to_owned(),
        ok: !monopoly,
        detail: if total_connectors == 0 {
            "No connectors".to_owned()
        } else if monopoly {
            format!(
                "Class '{dominant_class}' is {:.0}% of connectors — re-run Analyze / fix classification",
                dominant_frac * 100.0
            )
        } else {
            format!(
                "Largest class '{dominant_class}' is {:.0}% of connectors",
                dominant_frac * 100.0
            )
        },
    });
    if monopoly {
        score = score.saturating_sub(25);
    }

    let diversity_ok = classes_in_use.len() >= 2 || total_connectors < 8;
    checklist.push(VibeChecklistItem {
        id: "class_diversity".to_owned(),
        ok: diversity_ok,
        detail: format!("{} distinct connector class(es) in use", classes_in_use.len()),
    });

    // Ready = machine-mappable multi-piece kit: ≥2 connected assets, rules/coverage ok.
    let multi_piece_ready = with_connectors.len() >= 2 && multi_asset_component;
    let blockers = !coverage_ok || !rules_ok || !multi_piece_ready || monopoly;
    let ready = score >= 70 && !blockers;

    let mut notes = Vec::new();
    if !assets_without_connectors.is_empty() {
        notes.push(format!(
            "Run Analyze on assets without connectors: {}",
            assets_without_connectors.join(", ")
        ));
    }
    if !orphan_classes.is_empty() {
        notes.push(format!(
            "Add compatibility rules for: {}",
            orphan_classes.join(", ")
        ));
    }
    if monopoly {
        notes.push(format!(
            "Class monopoly: '{dominant_class}' dominates connectors — pack is not reliable for vibe tools until classes are fixed"
        ));
    }
    if !dead_end_classes.is_empty() {
        notes.push(format!(
            "Dead-end classes (single asset, self-only rule): {} — add partner assets or cross-class rules",
            dead_end_classes.join(", ")
        ));
    }
    if multi_na && !with_connectors.is_empty() {
        notes.push(
            "Only one mapped asset — multi-piece vibe assembly needs ≥2 assets that can mate."
                .to_owned(),
        );
    }
    if ready {
        notes
            .push("Pack looks vibe-ready: export LLM bundle and hand plans to resolve.".to_owned());
    } else {
        notes
            .push("Not ready for unattended vibe assembly — fix checklist items first.".to_owned());
    }

    VibeReadinessReport {
        score,
        ready,
        coverage,
        assets_without_connectors,
        orphan_classes,
        dead_end_classes,
        connectivity_gaps,
        checklist,
        notes,
    }
}

fn asset_connectivity_components(
    pack: &PackRecord,
    partners: &HashMap<String, Vec<String>>,
) -> Vec<Vec<String>> {
    let ids: Vec<String> = pack
        .assets
        .iter()
        .filter(|a| !a.connectors.is_empty())
        .map(|a| a.asset_id.clone())
        .collect();
    if ids.is_empty() {
        return Vec::new();
    }

    let mut class_to_assets: HashMap<String, Vec<String>> = HashMap::new();
    for asset in &pack.assets {
        for c in &asset.connectors {
            class_to_assets
                .entry(c.class.clone())
                .or_default()
                .push(asset.asset_id.clone());
        }
    }

    // Undirected graph: edge if two assets share compatible class pairs.
    let mut adj: HashMap<String, HashSet<String>> = HashMap::new();
    for id in &ids {
        adj.entry(id.clone()).or_default();
    }

    for asset in pack.assets.iter().filter(|a| !a.connectors.is_empty()) {
        for c in &asset.connectors {
            let Some(plist) = partners.get(&c.class) else {
                continue;
            };
            for partner_class in plist {
                if let Some(others) = class_to_assets.get(partner_class) {
                    for other in others {
                        if other != &asset.asset_id {
                            adj.entry(asset.asset_id.clone())
                                .or_default()
                                .insert(other.clone());
                            adj.entry(other.clone())
                                .or_default()
                                .insert(asset.asset_id.clone());
                        }
                    }
                }
            }
        }
    }

    let mut seen = HashSet::new();
    let mut components = Vec::new();
    for id in &ids {
        if !seen.insert(id.clone()) {
            continue;
        }
        let mut stack = vec![id.clone()];
        let mut comp = Vec::new();
        while let Some(n) = stack.pop() {
            comp.push(n.clone());
            if let Some(neigh) = adj.get(&n) {
                for m in neigh {
                    if seen.insert(m.clone()) {
                        stack.push(m.clone());
                    }
                }
            }
        }
        components.push(comp);
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        AllowedRotation, AssetRecord, AssetType, Axis3, Bounds3, CURRENT_SCHEMA_VERSION,
        CompatibilityRule, ConnectorClass, ConnectorFrame, ConnectorRecord, ConnectorRole,
        ControlledVocabulary, CoordinateConvention, Handedness, PackProvenance, Pivot, Unit,
    };

    fn emptyish_pack() -> PackRecord {
        PackRecord {
            schema_version: CURRENT_SCHEMA_VERSION,
            pack_id: "t".into(),
            display_name: "T".into(),
            coordinate_convention: CoordinateConvention {
                handedness: Handedness::Right,
                up_axis: Axis3::PosY,
                forward_axis: Axis3::PosZ,
            },
            default_units: Unit::Meters,
            license_summary: "MIT".into(),
            provenance: PackProvenance {
                author: Some("t".into()),
                ..Default::default()
            },
            vocabulary: ControlledVocabulary::default(),
            connector_classes: vec![ConnectorClass {
                class: "wall_edge".into(),
                display_name: "Wall Edge".into(),
            }],
            compatibility_rules: vec![CompatibilityRule {
                a_class: "wall_edge".into(),
                b_class: "wall_edge".into(),
                rotation: AllowedRotation::Locked,
            }],
            assets: vec![model("a", true), model("b", true)],
        }
    }

    fn model(id: &str, with_conn: bool) -> AssetRecord {
        AssetRecord {
            asset_id: id.into(),
            source_path: format!("{id}.glb"),
            content_hash: "sha256:x".into(),
            display_name: id.into(),
            asset_type: AssetType::Model3d,
            bounds: Bounds3 {
                min: [-1.0, 0.0, -0.1],
                max: [1.0, 2.0, 0.1],
            },
            dimensions: [2.0, 2.0, 0.2],
            pivot: Pivot::Origin,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
            semantic_tags: vec![],
            affordances: vec![],
            placement_constraints: vec![],
            review_flags: vec![],
            connectors: if with_conn {
                vec![ConnectorRecord {
                    connector_id: format!("{id}_edge"),
                    display_name: "Edge".into(),
                    class: "wall_edge".into(),
                    role: ConnectorRole::Symmetric,
                    frame: ConnectorFrame::Frame3d {
                        position: [1.0, 1.0, 0.0],
                        orientation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
                    },
                    mating_axis: Axis3::PosZ,
                    up_reference: Axis3::PosY,
                    snap_tolerance: 0.01,
                    face_size: Some([2.0, 2.0]),
                }]
            } else {
                vec![]
            },
        }
    }

    #[test]
    fn ready_when_two_assets_share_rule() {
        let pack = emptyish_pack();
        let report = vibe_readiness(&pack);
        assert!(report.coverage >= 1.0);
        assert!(report.orphan_classes.is_empty());
        assert!(
            report.ready,
            "score={} notes={:?}",
            report.score, report.notes
        );
        assert!(report.score >= 70);
    }

    #[test]
    fn not_ready_without_connectors() {
        let mut pack = emptyish_pack();
        for a in &mut pack.assets {
            a.connectors.clear();
        }
        let report = vibe_readiness(&pack);
        assert!(!report.ready);
        assert_eq!(report.assets_without_connectors.len(), 2);
    }

    #[test]
    fn empty_pack_not_ready() {
        let mut pack = emptyish_pack();
        pack.assets.clear();
        pack.compatibility_rules.clear();
        let report = vibe_readiness(&pack);
        assert!(!report.ready);
        assert_eq!(report.coverage, 0.0);
        assert!(
            report
                .checklist
                .iter()
                .any(|c| c.id == "has_3d_assets" && !c.ok)
        );
    }

    #[test]
    fn orphan_class_not_ready() {
        let mut pack = emptyish_pack();
        pack.compatibility_rules.clear();
        let report = vibe_readiness(&pack);
        assert!(!report.orphan_classes.is_empty());
        assert!(!report.ready);
        assert!(
            report
                .checklist
                .iter()
                .any(|c| c.id == "class_rules" && !c.ok)
        );
    }

    #[test]
    fn single_asset_not_ready_even_with_self_rule() {
        let mut pack = emptyish_pack();
        pack.assets.truncate(1);
        let report = vibe_readiness(&pack);
        assert!(!report.ready);
        assert!(
            report.notes.iter().any(|n| n.contains("Only one mapped")),
            "notes={:?}",
            report.notes
        );
        // multi-piece checklist is N/A-ok, but ready still false
        assert!(
            report
                .checklist
                .iter()
                .any(|c| c.id == "multi_piece_connectivity" && c.ok)
        );
    }

    #[test]
    fn dead_end_class_on_single_asset() {
        let mut pack = emptyish_pack();
        pack.assets.truncate(1);
        let report = vibe_readiness(&pack);
        assert!(
            report.dead_end_classes.contains(&"wall_edge".to_owned()),
            "dead_end={:?}",
            report.dead_end_classes
        );
    }

    #[test]
    fn connectivity_gaps_checklist_not_ok_without_multi_path() {
        let mut pack = emptyish_pack();
        // Two assets, different orphan classes, no rules → no multi path, gaps present
        pack.compatibility_rules.clear();
        pack.assets[0].connectors[0].class = "alpha".into();
        pack.assets[1].connectors[0].class = "beta".into();
        let report = vibe_readiness(&pack);
        let gaps = report
            .checklist
            .iter()
            .find(|c| c.id == "no_connectivity_gaps")
            .expect("gaps item");
        assert!(!gaps.ok, "detail={}", gaps.detail);
        assert!(
            !gaps.detail.contains("cannot mate") || !gaps.ok,
            "ok must match failure detail"
        );
        assert!(!report.ready);
    }
}
