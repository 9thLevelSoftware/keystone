use std::path::PathBuf;

pub mod commands;
pub mod dto;
pub mod error;

use dto::{EditorPackState, ExportEditorResult, IndexEditorResult, SaveEditorResult};
use error::EditorCommandError;

#[tauri::command]
fn open_pack_folder(path: String) -> Result<EditorPackState, EditorCommandError> {
    commands::open_pack_folder(PathBuf::from(path))
}

#[tauri::command]
fn init_pack_folder(
    path: String,
    display_name: String,
) -> Result<EditorPackState, EditorCommandError> {
    commands::init_pack_folder(PathBuf::from(path), display_name)
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Asset Mapper");
}
