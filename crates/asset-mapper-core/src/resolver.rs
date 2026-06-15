use crate::schema::{AssemblyPlan, PackRecord, Transform3d};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AssetPlacement {
    pub asset_id: String,
    pub transform: Transform3d,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResolvedScene {
    pub placements: Vec<AssetPlacement>,
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("resolver stub reached for plan root asset `{root_asset_id}`")]
    ResolverStub { root_asset_id: String },
}

pub fn resolve_plan(
    _pack: &PackRecord,
    plan: &AssemblyPlan,
) -> Result<ResolvedScene, ResolveError> {
    Err(ResolveError::ResolverStub {
        root_asset_id: plan.root_asset_id.clone(),
    })
}
