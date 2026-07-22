import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

import type {
  EditorPackState,
  ExportEditorResult,
  IndexEditorResult,
  SaveEditorResult,
  ValidationReport,
} from "./types";

export async function choosePackFolder(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}

export async function chooseBundleOutputPath(): Promise<string | null> {
  const selected = await save({
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export function openPackFolder(path: string): Promise<EditorPackState> {
  return invoke("open_pack_folder", { path });
}

export function initPackFolder(
  path: string,
  displayName: string,
): Promise<EditorPackState> {
  return invoke("init_pack_folder", { path, displayName });
}

export function indexPackFolder(path: string): Promise<IndexEditorResult> {
  return invoke("index_pack_folder", { path });
}

export function savePack(state: EditorPackState): Promise<SaveEditorResult> {
  return invoke("save_pack", { state });
}

export function validatePack(
  state: EditorPackState,
): Promise<ValidationReport> {
  return invoke("validate_pack", { state });
}

export function exportBundle(
  state: EditorPackState,
  outputPath: string,
): Promise<ExportEditorResult> {
  return invoke("export_bundle", { state, outputPath });
}
