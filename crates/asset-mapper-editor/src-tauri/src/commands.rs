use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

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
    let input_path = canonicalize_existing_path(path)?;
    let loaded = read_pack_from_input(&input_path)?;
    let pack_root = loaded
        .resolved
        .pack_root
        .clone()
        .unwrap_or_else(|| input_path.clone());
    state_from_pack(pack_root, loaded.resolved.sidecar_path, loaded.pack, false)
}

pub fn init_pack_folder(
    path: impl AsRef<Path>,
    display_name: String,
) -> Result<EditorPackState, EditorCommandError> {
    let pack_root = canonicalize_existing_path(path)?;
    io_init_pack_folder(&pack_root, display_name)?;
    open_pack_folder(&pack_root)
}

pub fn index_pack_folder(path: impl AsRef<Path>) -> Result<IndexEditorResult, EditorCommandError> {
    let pack_root = canonicalize_existing_path(path)?;
    let loaded = read_pack_from_input(&pack_root)?;
    validate_editor_source_paths(&pack_root, &loaded.pack)?;
    let report = io_index_pack_folder(&pack_root)?;
    let state = open_pack_folder(&pack_root)?;
    Ok(IndexEditorResult { report, state })
}

pub fn validate_pack(state: EditorPackState) -> Result<ValidationReport, EditorCommandError> {
    let pack_root = canonicalize_existing_path(Path::new(&state.pack_root))?;
    validation_report(&pack_root, &state.pack)
}

pub fn save_pack(state: EditorPackState) -> Result<SaveEditorResult, EditorCommandError> {
    let pack_root = canonicalize_existing_path(Path::new(&state.pack_root))?;
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
    let pack_root = canonicalize_existing_path(Path::new(&state.pack_root))?;
    let validation = validation_report(&pack_root, &state.pack)?;
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
    let pack_root = canonicalize_existing_path(&pack_root)?;
    let sidecar_path = canonicalize_existing_path(&sidecar_path)?;
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

fn canonicalize_existing_path(path: impl AsRef<Path>) -> Result<PathBuf, EditorCommandError> {
    Ok(std::fs::canonicalize(path)?)
}

fn validation_report(
    pack_root: &Path,
    pack: &PackRecord,
) -> Result<ValidationReport, EditorCommandError> {
    validate_editor_source_paths(pack_root, pack)?;
    let mut report = validate_core_pack(pack);
    let source_report = validate_pack_sources(pack_root, pack)?;
    report.extend(source_report.diagnostics);
    Ok(report)
}

fn asset_statuses(
    pack_root: &Path,
    pack: &PackRecord,
) -> Result<Vec<EditorAssetStatus>, EditorCommandError> {
    validate_editor_source_paths(pack_root, pack)?;
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

fn validate_editor_source_paths(
    pack_root: &Path,
    pack: &PackRecord,
) -> Result<(), EditorCommandError> {
    for asset in &pack.assets {
        validate_editor_source_path(pack_root, &asset.asset_id, &asset.source_path)?;
    }
    Ok(())
}

fn validate_editor_source_path(
    pack_root: &Path,
    asset_id: &str,
    source_path: &str,
) -> Result<(), EditorCommandError> {
    if source_path.trim().is_empty() {
        return Err(invalid_source_path(asset_id, source_path));
    }

    if source_path.contains('\\') || has_windows_drive_prefix(source_path) {
        return Err(invalid_source_path(asset_id, source_path));
    }

    let path = Path::new(source_path);
    if path.is_absolute() {
        return Err(invalid_source_path(asset_id, source_path));
    }

    for segment in source_path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(invalid_source_path(asset_id, source_path));
        }
    }

    let mut relative_path = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(component) => relative_path.push(component),
            Component::Prefix(_)
            | Component::RootDir
            | Component::CurDir
            | Component::ParentDir => {
                return Err(invalid_source_path(asset_id, source_path));
            }
        }
    }

    let absolute_path = pack_root.join(relative_path);
    if absolute_path.exists() {
        let canonical_root = std::fs::canonicalize(pack_root)?;
        let canonical_asset = std::fs::canonicalize(&absolute_path)?;
        if !canonical_asset.starts_with(canonical_root) {
            return Err(invalid_source_path(asset_id, source_path));
        }
    }

    Ok(())
}

fn invalid_source_path(asset_id: &str, source_path: &str) -> EditorCommandError {
    EditorCommandError::new(
        "invalid_source_path",
        format!(
            "asset `{asset_id}` source_path `{source_path}` must be a non-empty relative path inside the pack root"
        ),
    )
}

fn has_windows_drive_prefix(source_path: &str) -> bool {
    let bytes = source_path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn preview_supported(source_path: &str) -> bool {
    source_path
        .rsplit_once('.')
        .map(|(_, extension)| {
            extension.eq_ignore_ascii_case("glb") || extension.eq_ignore_ascii_case("gltf")
        })
        .unwrap_or(false)
}
