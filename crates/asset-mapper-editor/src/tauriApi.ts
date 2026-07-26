import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

import type {
  AssemblyPlan,
  EditorPackState,
  ExportEditorResult,
  IndexEditorResult,
  MeasureEditorResult,
  ResolvedScene,
  SaveEditorResult,
  ValidationReport,
} from "./types";

export interface AnalyzeReport {
  assets_processed: number;
  connectors_added: number;
  classes_added: number;
  rules_added: number;
  mesh_socket_assets?: number;
  bounds_fallback_assets?: number;
  skipped_assets: string[];
  notes: string[];
}

export interface ProposeAssemblyReport {
  plan: AssemblyPlan;
  placed_asset_ids: string[];
  unplaced_asset_ids: string[];
  notes: string[];
}

export interface ProposeAssemblyEditorResult {
  report: ProposeAssemblyReport;
  scene: ResolvedScene | null;
}

export interface AnalyzeEditorResult {
  report: AnalyzeReport;
  state: EditorPackState;
}

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
  licenseSummary: string,
  author?: string | null,
  source?: string | null,
): Promise<EditorPackState> {
  return invoke("init_pack_folder", {
    path,
    displayName,
    licenseSummary,
    author: author ?? null,
    source: source ?? null,
  });
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

export function acceptHashDrift(
  path: string,
  assets: string[],
): Promise<IndexEditorResult> {
  return invoke("accept_hash_drift", { path, assets });
}

export function measurePackBounds(path: string): Promise<MeasureEditorResult> {
  return invoke("measure_pack_bounds", { path });
}

export function analyzePackFolder(
  path: string,
  replaceExisting = false,
): Promise<AnalyzeEditorResult> {
  return invoke("analyze_pack_folder", { path, replaceExisting });
}

export function readPackAssetBytes(
  packRoot: string,
  sourcePath: string,
): Promise<number[] | Uint8Array> {
  return invoke("read_pack_asset_bytes", { packRoot, sourcePath });
}

export function resolveAssemblyPlan(
  state: EditorPackState,
  plan: AssemblyPlan,
): Promise<{ scene: ResolvedScene }> {
  return invoke("resolve_assembly_plan", { state, plan });
}

export function proposeAssembly(
  state: EditorPackState,
  maxPieces = 8,
  rootAssetId?: string | null,
): Promise<ProposeAssemblyEditorResult> {
  return invoke("propose_assembly", {
    state,
    maxPieces,
    rootAssetId: rootAssetId ?? null,
  });
}
