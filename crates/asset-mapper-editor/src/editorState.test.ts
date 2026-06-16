import { describe, expect, it } from "vitest";

import {
  addConnector,
  addConnectorClass,
  addCompatibilityRule,
  groupDiagnostics,
  removeConnector,
  selectAsset,
  updateConnector,
  updateConnectorFrame,
} from "./editorState";
import type { EditorPackState } from "./types";

function baseState(): EditorPackState {
  return {
    packRoot: "C:/pack",
    sidecarPath: "C:/pack/.asset-mapper/pack.assetmap.json",
    dirty: false,
    selectedAssetId: "wall",
    selectedConnectorId: null,
    validation: { diagnostics: [] },
    assets: [
      {
        assetId: "wall",
        sourcePath: "wall.glb",
        absolutePath: "C:/pack/wall.glb",
        exists: true,
        contentHash: "sha256:abc",
        hashMatches: true,
        previewSupported: true,
      },
    ],
    pack: {
      schema_version: 1,
      pack_id: "pack",
      display_name: "Pack",
      coordinate_convention: {
        handedness: "right",
        up_axis: "pos_y",
        forward_axis: "pos_z",
      },
      default_units: "meters",
      connector_classes: [],
      compatibility_rules: [],
      assets: [
        {
          asset_id: "wall",
          source_path: "wall.glb",
          content_hash: "sha256:abc",
          display_name: "Wall",
          asset_type: "model3d",
          bounds: { min: [-0.5, -0.5, -0.5], max: [0.5, 0.5, 0.5] },
          dimensions: [1, 1, 1],
          pivot: "origin",
          up_axis: "pos_y",
          forward_axis: "pos_z",
          semantic_tags: [],
          affordances: [],
          placement_constraints: [],
          review_flags: [],
          connectors: [],
        },
      ],
    },
  };
}

describe("editorState", () => {
  it("selectAsset sets the selected asset and clears the selected connector", () => {
    const state = {
      ...baseState(),
      selectedAssetId: null,
      selectedConnectorId: "connector_1",
    };

    const nextState = selectAsset(state, "wall");

    expect(nextState.selectedAssetId).toBe("wall");
    expect(nextState.selectedConnectorId).toBeNull();
  });

  it("adds, moves, and removes a 3D connector from an asset", () => {
    const withConnector = addConnector(baseState(), "wall");

    expect(withConnector.dirty).toBe(true);
    expect(withConnector.selectedAssetId).toBe("wall");
    expect(withConnector.selectedConnectorId).toBe("connector_1");
    expect(withConnector.pack.assets[0].connectors).toHaveLength(1);
    expect(withConnector.pack.assets[0].connectors[0]).toMatchObject({
      connector_id: "connector_1",
      display_name: "Connector 1",
      class: "",
      role: "symmetric",
      frame: {
        kind: "frame3d",
        position: [0, 0, 0],
        orientation_quat_xyzw: [0, 0, 0, 1],
      },
      mating_axis: "pos_z",
      up_reference: "pos_y",
      snap_tolerance: 0.01,
    });

    const moved = updateConnectorFrame(withConnector, "wall", "connector_1", {
      position: [1, 2, 3],
      orientation_quat_xyzw: [0, 0, 0, 1],
    });

    expect(moved.pack.assets[0].connectors[0].frame).toMatchObject({
      kind: "frame3d",
      position: [1, 2, 3],
    });

    const removed = removeConnector(moved, "wall", "connector_1");

    expect(removed.pack.assets[0].connectors).toHaveLength(0);
    expect(removed.selectedConnectorId).toBeNull();
  });

  it("updates connector metadata, frame, and selection when the connector id changes", () => {
    const withConnector = addConnector(baseState(), "wall");

    const updated = updateConnector(withConnector, "wall", "connector_1", {
      connector_id: "door_connector",
      display_name: "Door Connector",
      frame: {
        kind: "frame3d",
        position: [4, 5, 6],
        orientation_quat_xyzw: [0, 0, 0, 1],
      },
    });

    expect(updated.dirty).toBe(true);
    expect(updated.selectedConnectorId).toBe("door_connector");
    expect(updated.pack.assets[0].connectors[0]).toMatchObject({
      connector_id: "door_connector",
      display_name: "Door Connector",
      frame: {
        kind: "frame3d",
        position: [4, 5, 6],
      },
    });
  });

  it("appends connector classes and locked compatibility rules", () => {
    const withClass = addConnectorClass(baseState(), "doorway", "Doorway");

    expect(withClass.dirty).toBe(true);
    expect(withClass.pack.connector_classes).toEqual([
      { class: "doorway", display_name: "Doorway" },
    ]);

    const withRule = addCompatibilityRule(withClass, "doorway", "doorway");

    expect(withRule.dirty).toBe(true);
    expect(withRule.pack.compatibility_rules).toEqual([
      {
        a_class: "doorway",
        b_class: "doorway",
        rotation: { kind: "locked" },
      },
    ]);
  });

  it("groups diagnostics by connector, asset, and pack", () => {
    const grouped = groupDiagnostics({
      diagnostics: [
        {
          code: "connector",
          severity: "error",
          message: "Connector problem",
          asset_id: "wall",
          connector_id: "connector_1",
        },
        {
          code: "asset",
          severity: "warning",
          message: "Asset problem",
          asset_id: "wall",
          connector_id: null,
        },
        {
          code: "pack",
          severity: "error",
          message: "Pack problem",
          asset_id: null,
          connector_id: null,
        },
      ],
    });

    expect(grouped).toEqual({
      "asset:wall/connector:connector_1": [
        {
          code: "connector",
          severity: "error",
          message: "Connector problem",
          asset_id: "wall",
          connector_id: "connector_1",
        },
      ],
      "asset:wall": [
        {
          code: "asset",
          severity: "warning",
          message: "Asset problem",
          asset_id: "wall",
          connector_id: null,
        },
      ],
      pack: [
        {
          code: "pack",
          severity: "error",
          message: "Pack problem",
          asset_id: null,
          connector_id: null,
        },
      ],
    });
  });
});
