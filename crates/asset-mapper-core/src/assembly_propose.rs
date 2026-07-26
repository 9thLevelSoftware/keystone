//! Greedy multi-piece assembly plan synthesis from pack connectors + rules.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::resolver::resolve_plan;
use crate::schema::{
    AssemblyOperation, AssemblyPlan, CompatibilityRule, ConnectorFrame, ConnectorRecord, PackRecord,
};

/// Options for whole-pack assembly proposal.
#[derive(Debug, Clone)]
pub struct ProposeAssemblyOptions {
    /// Maximum placements including root.
    pub max_pieces: usize,
    /// Optional root asset id (default: most connectors, then first).
    pub root_asset_id: Option<String>,
    /// Prefer dimension-compatible face spans (0.7–1.3 default).
    pub size_ratio_min: f32,
    pub size_ratio_max: f32,
}

impl Default for ProposeAssemblyOptions {
    fn default() -> Self {
        Self {
            max_pieces: 8,
            root_asset_id: None,
            size_ratio_min: 0.65,
            size_ratio_max: 1.55,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProposeAssemblyReport {
    pub plan: AssemblyPlan,
    pub placed_asset_ids: Vec<String>,
    pub unplaced_asset_ids: Vec<String>,
    pub notes: Vec<String>,
}

/// Build a connected multi-piece plan using existing compatibility rules.
///
/// Each asset is used at most once (kit of unique pieces). Operations are
/// validated incrementally with [`resolve_plan`].
pub fn propose_assembly_plan(
    pack: &PackRecord,
    options: &ProposeAssemblyOptions,
) -> ProposeAssemblyReport {
    let mut notes = Vec::new();
    let assets_with: Vec<_> = pack
        .assets
        .iter()
        .filter(|a| !a.connectors.is_empty())
        .collect();

    if assets_with.is_empty() {
        return ProposeAssemblyReport {
            plan: AssemblyPlan {
                root_asset_id: String::new(),
                operations: vec![],
            },
            placed_asset_ids: vec![],
            unplaced_asset_ids: pack.assets.iter().map(|a| a.asset_id.clone()).collect(),
            notes: vec!["No assets with connectors — run Analyze first.".to_owned()],
        };
    }

    let root_id = options
        .root_asset_id
        .clone()
        .filter(|id| assets_with.iter().any(|a| a.asset_id == *id))
        .unwrap_or_else(|| pick_root(&assets_with));

    let max_pieces = options.max_pieces.max(1);
    let mut placed: BTreeSet<String> = BTreeSet::new();
    placed.insert(root_id.clone());

    // Free connectors: (asset_id, connector_id)
    let mut free: HashSet<(String, String)> = HashSet::new();
    if let Some(root) = pack.assets.iter().find(|a| a.asset_id == root_id) {
        for c in &root.connectors {
            free.insert((root_id.clone(), c.connector_id.clone()));
        }
    }

    let mut operations: Vec<AssemblyOperation> = Vec::new();
    let mut used_connectors: HashSet<(String, String)> = HashSet::new();

    while placed.len() < max_pieces {
        let mut best: Option<(f32, AssemblyOperation, String, String)> = None;

        for (anchor_asset_id, anchor_connector_id) in free.iter() {
            if used_connectors.contains(&(anchor_asset_id.clone(), anchor_connector_id.clone())) {
                continue;
            }
            let Some(anchor_asset) = pack.assets.iter().find(|a| a.asset_id == *anchor_asset_id)
            else {
                continue;
            };
            let Some(anchor_c) = anchor_asset
                .connectors
                .iter()
                .find(|c| c.connector_id == *anchor_connector_id)
            else {
                continue;
            };

            for candidate in pack.assets.iter().filter(|a| !placed.contains(&a.asset_id)) {
                for placed_c in &candidate.connectors {
                    if !classes_compatible(
                        &pack.compatibility_rules,
                        &placed_c.class,
                        &anchor_c.class,
                    ) {
                        continue;
                    }
                    if !size_compatible(placed_c, anchor_c, options) {
                        continue;
                    }
                    let op = AssemblyOperation {
                        placed_asset_id: candidate.asset_id.clone(),
                        placed_connector_id: placed_c.connector_id.clone(),
                        anchor_asset_id: anchor_asset_id.clone(),
                        anchor_connector_id: anchor_connector_id.clone(),
                        rotation_choice_deg: Some(0.0),
                    };

                    // Score: prefer same class, then size closeness.
                    let score = pair_score(placed_c, anchor_c);
                    let better = best.as_ref().map(|(s, ..)| score > *s).unwrap_or(true);
                    if better {
                        best = Some((
                            score,
                            op,
                            candidate.asset_id.clone(),
                            placed_c.connector_id.clone(),
                        ));
                    }
                }
            }
        }

        let Some((_score, op, new_asset, new_conn)) = best else {
            notes.push("No further compatible free connectors found.".to_owned());
            break;
        };

        // Incremental resolve check.
        let trial = AssemblyPlan {
            root_asset_id: root_id.clone(),
            operations: {
                let mut ops = operations.clone();
                ops.push(op.clone());
                ops
            },
        };
        match resolve_plan(pack, &trial) {
            Ok(_) => {
                used_connectors
                    .insert((op.anchor_asset_id.clone(), op.anchor_connector_id.clone()));
                used_connectors.insert((new_asset.clone(), new_conn.clone()));
                free.remove(&(op.anchor_asset_id.clone(), op.anchor_connector_id.clone()));
                // Add free connectors from newly placed asset (except used).
                if let Some(asset) = pack.assets.iter().find(|a| a.asset_id == new_asset) {
                    for c in &asset.connectors {
                        let key = (new_asset.clone(), c.connector_id.clone());
                        if !used_connectors.contains(&key) {
                            free.insert(key);
                        }
                    }
                }
                placed.insert(new_asset);
                operations.push(op);
            }
            Err(err) => {
                // Mark this pair as used so we don't infinite-loop on it.
                used_connectors
                    .insert((op.anchor_asset_id.clone(), op.anchor_connector_id.clone()));
                free.remove(&(op.anchor_asset_id.clone(), op.anchor_connector_id.clone()));
                notes.push(format!(
                    "Skipped {}→{}: {err}",
                    op.placed_asset_id, op.anchor_asset_id
                ));
            }
        }
    }

    let unplaced: Vec<String> = pack
        .assets
        .iter()
        .map(|a| a.asset_id.clone())
        .filter(|id| !placed.contains(id))
        .collect();

    if operations.is_empty() && assets_with.len() > 1 {
        notes.push(
            "Could not attach any secondary asset. Check compatibility rules and classes."
                .to_owned(),
        );
    } else if placed.len() >= 3 {
        notes.push(format!(
            "Connected {} pieces (max {}).",
            placed.len(),
            max_pieces
        ));
    }

    ProposeAssemblyReport {
        plan: AssemblyPlan {
            root_asset_id: root_id,
            operations,
        },
        placed_asset_ids: placed.into_iter().collect(),
        unplaced_asset_ids: unplaced,
        notes,
    }
}

fn pick_root(assets: &[&crate::schema::AssetRecord]) -> String {
    assets
        .iter()
        .max_by_key(|a| a.connectors.len())
        .map(|a| a.asset_id.clone())
        .unwrap_or_default()
}

fn classes_compatible(rules: &[CompatibilityRule], a: &str, b: &str) -> bool {
    rules
        .iter()
        .any(|r| (r.a_class == a && r.b_class == b) || (r.a_class == b && r.b_class == a))
}

fn size_compatible(
    a: &ConnectorRecord,
    b: &ConnectorRecord,
    options: &ProposeAssemblyOptions,
) -> bool {
    // Use snap_tolerance as a weak size proxy when face span unknown; prefer always true
    // if either tolerance is default-small.
    let ta = a.snap_tolerance.max(1e-6);
    let tb = b.snap_tolerance.max(1e-6);
    let ratio = ta.max(tb) / ta.min(tb);
    // snap_tolerance often identical — allow. If ratio extreme, still allow when both small.
    if (ta - tb).abs() < 1e-5 {
        return true;
    }
    ratio >= options.size_ratio_min && ratio <= options.size_ratio_max * 2.0 || ratio < 3.0
}

fn pair_score(a: &ConnectorRecord, b: &ConnectorRecord) -> f32 {
    let mut score = 1.0f32;
    if a.class == b.class {
        score += 2.0;
    }
    // Prefer horizontal mates (not pos_y heavy) — approximate via position y similarity.
    if let (
        ConnectorFrame::Frame3d { position: pa, .. },
        ConnectorFrame::Frame3d { position: pb, .. },
    ) = (&a.frame, &b.frame)
    {
        let dy = (pa[1] - pb[1]).abs();
        score += 1.0 / (1.0 + dy);
    }
    score
}

/// Build a lookup of class → partner classes from rules (for diagnostics).
pub fn rule_partner_map(pack: &PackRecord) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, BTreeSet<String>> = HashMap::new();
    for r in &pack.compatibility_rules {
        map.entry(r.a_class.clone())
            .or_default()
            .insert(r.b_class.clone());
        map.entry(r.b_class.clone())
            .or_default()
            .insert(r.a_class.clone());
    }
    map.into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect()
}
