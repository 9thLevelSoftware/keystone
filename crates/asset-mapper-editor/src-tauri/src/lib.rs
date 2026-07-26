use std::path::PathBuf;

pub mod commands;
pub mod dto;
pub mod error;

use asset_mapper_core::AssemblyPlan;
use dto::{
    AnalyzeEditorResult, EditorPackState, ExportEditorResult, IndexEditorResult,
    MeasureEditorResult, ResolveEditorResult, SaveEditorResult,
};
use error::EditorCommandError;

#[tauri::command]
fn open_pack_folder(path: String) -> Result<EditorPackState, EditorCommandError> {
    commands::open_pack_folder(PathBuf::from(path))
}

#[tauri::command]
fn init_pack_folder(
    path: String,
    display_name: String,
    license_summary: String,
    author: Option<String>,
    source: Option<String>,
) -> Result<EditorPackState, EditorCommandError> {
    commands::init_pack_folder(
        PathBuf::from(path),
        asset_mapper_io::InitPackOptions {
            display_name,
            license_summary,
            author,
            source,
        },
    )
}

#[tauri::command]
fn index_pack_folder(path: String) -> Result<IndexEditorResult, EditorCommandError> {
    commands::index_pack_folder(PathBuf::from(path))
}

#[tauri::command]
fn save_pack(state: EditorPackState) -> Result<SaveEditorResult, EditorCommandError> {
    commands::save_pack(state)
}

#[tauri::command]
fn validate_pack(
    state: EditorPackState,
) -> Result<asset_mapper_core::ValidationReport, EditorCommandError> {
    commands::validate_pack(state)
}

#[tauri::command]
fn export_bundle(
    state: EditorPackState,
    output_path: String,
) -> Result<ExportEditorResult, EditorCommandError> {
    commands::export_bundle(state, PathBuf::from(output_path))
}

#[tauri::command]
fn accept_hash_drift(
    path: String,
    assets: Vec<String>,
) -> Result<IndexEditorResult, EditorCommandError> {
    commands::accept_hash_drift(PathBuf::from(path), assets)
}

#[tauri::command]
fn measure_pack_bounds(path: String) -> Result<MeasureEditorResult, EditorCommandError> {
    commands::measure_pack_bounds(PathBuf::from(path))
}

#[tauri::command]
fn analyze_pack_folder(
    path: String,
    replace_existing: bool,
) -> Result<AnalyzeEditorResult, EditorCommandError> {
    commands::analyze_pack_folder(PathBuf::from(path), replace_existing)
}

#[tauri::command]
fn read_pack_asset_bytes(
    pack_root: String,
    source_path: String,
) -> Result<Vec<u8>, EditorCommandError> {
    commands::read_pack_asset_bytes(PathBuf::from(pack_root), &source_path)
}

#[tauri::command]
fn resolve_assembly_plan(
    state: EditorPackState,
    plan: AssemblyPlan,
) -> Result<ResolveEditorResult, EditorCommandError> {
    commands::resolve_assembly_plan(state, plan)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            open_pack_folder,
            init_pack_folder,
            index_pack_folder,
            save_pack,
            validate_pack,
            export_bundle,
            accept_hash_drift,
            measure_pack_bounds,
            analyze_pack_folder,
            read_pack_asset_bytes,
            resolve_assembly_plan,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Asset Mapper");
}
