use asset_mapper_core::{PackRecord, ValidationReport};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorPackState {
    pub pack_root: String,
    pub sidecar_path: String,
    pub pack: PackRecord,
    pub assets: Vec<EditorAssetStatus>,
    pub selected_asset_id: Option<String>,
    pub selected_connector_id: Option<String>,
    pub dirty: bool,
    pub validation: ValidationReport,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorAssetStatus {
    pub asset_id: String,
    pub source_path: String,
    pub absolute_path: String,
    pub exists: bool,
    pub content_hash: Option<String>,
    pub hash_matches: Option<bool>,
    pub preview_supported: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEditorResult {
    pub report: asset_mapper_io::IndexReport,
    pub state: EditorPackState,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveEditorResult {
    pub state: EditorPackState,
    pub validation: ValidationReport,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEditorResult {
    pub output_path: String,
}
