use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use asset_mapper_core::{
    LlmBundle, PackRecord, Severity, ValidationReport, validate_pack as validate_core_pack,
};
use asset_mapper_io::{
    canonical_sidecar_path, index_pack_folder as io_index_pack_folder,
    init_pack_folder as io_init_pack_folder, read_pack_from_input, scan_assets,
    validate_pack_sources, write_pack_sidecar,
};

use crate::dto::{
    EditorAssetStatus, EditorPackState, ExportEditorResult, IndexEditorResult, SaveEditorResult,
};
use crate::error::EditorCommandError;

pub fn open_pack_folder(path: impl AsRef<Path>) -> Result<EditorPackState, EditorCommandError> {
    let path = path.as_ref();
    let loaded = read_pack_from_input(path)?;
    let pack_root = loaded
        .resolved
        .pack_root
        .clone()
        .unwrap_or_else(|| path.to_path_buf());
    state_from_pack(pack_root, loaded.resolved.sidecar_path, loaded.pack, false)
}

pub fn init_pack_folder(
    path: impl AsRef<Path>,
    display_name: String,
) -> Result<EditorPackState, EditorCommandError> {
    let path = path.as_ref();
    io_init_pack_folder(path, display_name)?;
    open_pack_folder(path)
}

pub fn index_pack_folder(path: impl AsRef<Path>) -> Result<IndexEditorResult, EditorCommandError> {
    let path = path.as_ref();
    let report = io_index_pack_folder(path)?;
    let state = open_pack_folder(path)?;
    Ok(IndexEditorResult { report, state })
}

pub fn validate_pack(state: EditorPackState) -> Result<ValidationReport, EditorCommandError> {
    validation_report(&PathBuf::from(&state.pack_root), &state.pack)
}

pub fn save_pack(state: EditorPackState) -> Result<SaveEditorResult, EditorCommandError> {
    let pack_root = PathBuf::from(&state.pack_root);
    let validation = validation_report(&pack_root, &state.pack)?;
    if validation
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err(EditorCommandError::new(
            "validation_failed",
            "pack has validation errors and was not saved",
        ));
    }

    write_pack_sidecar(&pack_root, &state.pack)?;
    let refreshed = state_from_pack(
        pack_root.clone(),
        canonical_sidecar_path(&pack_root),
        state.pack,
        false,
    )?;
    Ok(SaveEditorResult {
        state: refreshed,
        validation,
    })
}

pub fn export_bundle(
    state: EditorPackState,
    output_path: impl AsRef<Path>,
) -> Result<ExportEditorResult, EditorCommandError> {
    let validation = validation_report(&PathBuf::from(&state.pack_root), &state.pack)?;
    if validation
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err(EditorCommandError::new(
            "validation_failed",
            "pack has validation errors and bundle export was blocked",
        ));
    }

    let bundle = LlmBundle::from_pack(&state.pack);
    let output = serde_json::to_string_pretty(&bundle)?;
    std::fs::write(output_path.as_ref(), format!("{output}\n"))?;
    Ok(ExportEditorResult {
        output_path: output_path.as_ref().to_string_lossy().into_owned(),
    })
}

fn state_from_pack(
    pack_root: PathBuf,
    sidecar_path: PathBuf,
    pack: PackRecord,
    dirty: bool,
) -> Result<EditorPackState, EditorCommandError> {
    let validation = validation_report(&pack_root, &pack)?;
    let assets = asset_statuses(&pack_root, &pack)?;
    let selected_asset_id = pack.assets.first().map(|asset| asset.asset_id.clone());

    Ok(EditorPackState {
        pack_root: pack_root.to_string_lossy().into_owned(),
        sidecar_path: sidecar_path.to_string_lossy().into_owned(),
        pack,
        assets,
        selected_asset_id,
        selected_connector_id: None,
        dirty,
        validation,
    })
}

fn validation_report(
    pack_root: &Path,
    pack: &PackRecord,
) -> Result<ValidationReport, EditorCommandError> {
    let mut report = validate_core_pack(pack);
    let source_report = validate_pack_sources(pack_root, pack)?;
    report.extend(source_report.diagnostics);
    Ok(report)
}

fn asset_statuses(
    pack_root: &Path,
    pack: &PackRecord,
) -> Result<Vec<EditorAssetStatus>, EditorCommandError> {
    let indexed = scan_assets(pack_root)?;
    let indexed_by_source = indexed
        .iter()
        .map(|asset| (asset.source_path.as_str(), asset))
        .collect::<BTreeMap<_, _>>();

    Ok(pack
        .assets
        .iter()
        .map(|asset| {
            let indexed = indexed_by_source.get(asset.source_path.as_str());
            let absolute_path = pack_root.join(&asset.source_path);
            EditorAssetStatus {
                asset_id: asset.asset_id.clone(),
                source_path: asset.source_path.clone(),
                absolute_path: absolute_path.to_string_lossy().into_owned(),
                exists: indexed.is_some(),
                content_hash: indexed.map(|asset| asset.content_hash.clone()),
                hash_matches: indexed.map(|indexed| indexed.content_hash == asset.content_hash),
                preview_supported: preview_supported(&asset.source_path),
            }
        })
        .collect())
}

fn preview_supported(source_path: &str) -> bool {
    source_path
        .rsplit_once('.')
        .map(|(_, extension)| {
            extension.eq_ignore_ascii_case("glb") || extension.eq_ignore_ascii_case("gltf")
        })
        .unwrap_or(false)
}
