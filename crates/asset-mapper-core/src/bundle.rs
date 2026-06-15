use crate::schema::PackRecord;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct LlmBundle {
    pub pack_id: String,
    pub display_name: String,
    pub assets: Vec<BundleAsset>,
    pub compatibility_rules: Vec<crate::schema::CompatibilityRule>,
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
}

impl LlmBundle {
    pub fn from_pack(pack: &PackRecord) -> Self {
        Self {
            pack_id: pack.pack_id.clone(),
            display_name: pack.display_name.clone(),
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
                        })
                        .collect(),
                })
                .collect(),
            compatibility_rules: pack.compatibility_rules.clone(),
        }
    }
}
