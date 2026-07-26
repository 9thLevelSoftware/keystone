use crate::schema::PackRecord;

/// Short plan-contract instructions for vibe builders / LLMs.
pub const HOW_TO_PLAN: &str = "\
Build an AssemblyPlan JSON: set root_asset_id to a pack asset_id, then list operations. \
Each operation mates placed_asset_id.placed_connector_id onto an already-placed \
anchor_asset_id.anchor_connector_id (root counts as placed). Connector classes must match \
a compatibility_rules pair (order-independent). rotation_choice_deg must be allowed by the \
rule (null/0 when locked). Never invent asset_id or connector_id values — only use ids from \
this bundle. Call resolve with the pack + plan to get world transforms; on failure read the \
error code and fix_target (fix_pack vs fix_plan). Unique assets only per plan: for tile floors \
place multiple instances outside Keystone (same asset_id cannot appear twice in one resolve). \
Prefer glTF/GLB sources for mesh-aware auto-map.";

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct LlmBundle {
    pub pack_id: String,
    pub display_name: String,
    pub license_summary: String,
    pub provenance: crate::schema::PackProvenance,
    pub vocabulary: crate::schema::ControlledVocabulary,
    pub assets: Vec<BundleAsset>,
    pub compatibility_rules: Vec<crate::schema::CompatibilityRule>,
    /// Stable instructions for plan authors (LLMs / vibe tools).
    #[serde(default = "default_how_to_plan")]
    pub how_to_plan: String,
    /// Minimal plan JSON contract description for tooling.
    #[serde(default)]
    pub plan_contract: PlanContract,
}

fn default_how_to_plan() -> String {
    HOW_TO_PLAN.to_owned()
}

/// Documents the AssemblyPlan fields without exposing resolver internals.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct PlanContract {
    pub root_field: String,
    pub operations_field: String,
    pub operation_fields: Vec<String>,
    pub notes: Vec<String>,
}

impl Default for PlanContract {
    fn default() -> Self {
        Self {
            root_field: "root_asset_id".to_owned(),
            operations_field: "operations".to_owned(),
            operation_fields: vec![
                "placed_asset_id".to_owned(),
                "placed_connector_id".to_owned(),
                "anchor_asset_id".to_owned(),
                "anchor_connector_id".to_owned(),
                "rotation_choice_deg".to_owned(),
            ],
            notes: vec![
                "Each asset_id may appear at most once per resolved plan.".to_owned(),
                "Connectors are referenced by connector_id only (no raw transforms in plans)."
                    .to_owned(),
                "Use face_size when present to pair similarly sized openings.".to_owned(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct BundleAsset {
    pub asset_id: String,
    pub display_name: String,
    pub asset_type: crate::schema::AssetType,
    pub dimensions: crate::schema::Vec3,
    pub semantic_tags: Vec<String>,
    pub affordances: Vec<String>,
    pub placement_constraints: Vec<String>,
    pub connectors: Vec<BundleConnector>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct BundleConnector {
    pub connector_id: String,
    pub display_name: String,
    pub class: String,
    pub role: crate::schema::ConnectorRole,
    /// Optional face plane size `[width, height]` in pack units for size-aware pairing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_size: Option<crate::schema::Vec2>,
}

impl LlmBundle {
    pub fn from_pack(pack: &PackRecord) -> Self {
        Self {
            pack_id: pack.pack_id.clone(),
            display_name: pack.display_name.clone(),
            license_summary: pack.license_summary.clone(),
            provenance: pack.provenance.clone(),
            vocabulary: pack.vocabulary.clone(),
            assets: pack
                .assets
                .iter()
                .map(|asset| BundleAsset {
                    asset_id: asset.asset_id.clone(),
                    display_name: asset.display_name.clone(),
                    asset_type: asset.asset_type.clone(),
                    dimensions: asset.dimensions,
                    semantic_tags: asset.semantic_tags.clone(),
                    affordances: asset.affordances.clone(),
                    placement_constraints: asset.placement_constraints.clone(),
                    connectors: asset
                        .connectors
                        .iter()
                        .map(|connector| BundleConnector {
                            connector_id: connector.connector_id.clone(),
                            display_name: connector.display_name.clone(),
                            class: connector.class.clone(),
                            role: connector.role.clone(),
                            face_size: connector.face_size,
                        })
                        .collect(),
                })
                .collect(),
            compatibility_rules: pack.compatibility_rules.clone(),
            how_to_plan: HOW_TO_PLAN.to_owned(),
            plan_contract: PlanContract::default(),
        }
    }
}
