export type Vec2 = [number, number];
export type Vec3 = [number, number, number];
export type QuatXyzw = [number, number, number, number];

export type Severity = "error" | "warning";
export type AssetType = "model3d" | "sprite2d" | "tile2d";
export type Axis3 = "pos_x" | "neg_x" | "pos_y" | "neg_y" | "pos_z" | "neg_z";
export type ConnectorRole = "symmetric" | "plug" | "receptacle";

export interface Diagnostic {
  code: string;
  severity: Severity;
  message: string;
  asset_id: string | null;
  connector_id: string | null;
}

export interface ValidationReport {
  diagnostics: Diagnostic[];
}

export interface ConnectorClass {
  class: string;
  display_name: string;
}

export type AllowedRotation =
  | { kind: "locked" }
  | { kind: "steps_deg"; values: number[] }
  | { kind: "free" };

export interface CompatibilityRule {
  a_class: string;
  b_class: string;
  rotation: AllowedRotation;
}

export interface Bounds3 {
  min: Vec3;
  max: Vec3;
}

export type ConnectorFrame =
  | {
      kind: "frame3d";
      position: Vec3;
      orientation_quat_xyzw: QuatXyzw;
    }
  | {
      kind: "frame2d";
      position: Vec2;
      normal: Vec2;
      grid_cell: [number, number] | null;
    };

export interface ConnectorRecord {
  connector_id: string;
  display_name: string;
  class: string;
  role: ConnectorRole;
  frame: ConnectorFrame;
  mating_axis: Axis3;
  up_reference: Axis3;
  snap_tolerance: number;
}

export interface AssetRecord {
  asset_id: string;
  source_path: string;
  content_hash: string;
  display_name: string;
  asset_type: AssetType;
  bounds: Bounds3;
  dimensions: Vec3;
  pivot: "origin" | "base_center" | "center" | "custom";
  up_axis: Axis3;
  forward_axis: Axis3;
  semantic_tags: string[];
  affordances: string[];
  placement_constraints: string[];
  review_flags: string[];
  connectors: ConnectorRecord[];
}

export interface PackRecord {
  schema_version: number;
  pack_id: string;
  display_name: string;
  coordinate_convention: {
    handedness: "right" | "left";
    up_axis: Axis3;
    forward_axis: Axis3;
  };
  default_units: "meters" | "centimeters" | "pixels";
  connector_classes: ConnectorClass[];
  compatibility_rules: CompatibilityRule[];
  assets: AssetRecord[];
}

export interface EditorAssetStatus {
  assetId: string;
  sourcePath: string;
  absolutePath: string;
  exists: boolean;
  contentHash: string | null;
  hashMatches: boolean | null;
  previewSupported: boolean;
}

export interface EditorPackState {
  packRoot: string;
  sidecarPath: string;
  pack: PackRecord;
  assets: EditorAssetStatus[];
  selectedAssetId: string | null;
  selectedConnectorId: string | null;
  dirty: boolean;
  validation: ValidationReport;
}

export interface IndexEditorResult {
  report: {
    sidecar_path: string;
    discovered_assets: string[];
    new_assets: string[];
    unchanged_assets: string[];
    drifted_assets: string[];
    missing_assets: string[];
  };
  state: EditorPackState;
}

export interface SaveEditorResult {
  state: EditorPackState;
  validation: ValidationReport;
}

export interface ExportEditorResult {
  outputPath: string;
}

export interface EditorCommandError {
  code: string;
  message: string;
}
