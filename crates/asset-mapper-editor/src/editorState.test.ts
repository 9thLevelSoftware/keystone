import { describe, expect, it } from "vitest";

import {
  addConnector,
  addConnectorClass,
  addCompatibilityRule,
  clearAllReviewFlags,
  duplicateConnector,
  eulerDegToQuat,
  groupDiagnostics,
  quatToEulerDeg,
  removeCompatibilityRule,
  removeConnector,
  removeConnectorClass,
  rotationFromKind,
  selectAsset,
  selectDiagnosticTarget,
  setReviewFlag,
  snapConnectorToBoundsFace,
  suggestClassFromName,
  updateAssetMetadata,
  updateConnector,
  updateConnectorClass,
  updateConnectorFrame,
  updateCompatibilityRule,
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
      schema_version: 2,
      pack_id: "pack",
      display_name: "Pack",
      coordinate_convention: {
        handedness: "right",
        up_axis: "pos_y",
        forward_axis: "pos_z",
      },
      default_units: "meters",
      license_summary: "MIT OR Apache-2.0",
      provenance: { notes: "test" },
      vocabulary: {
        semantic_tags: ["wall", "door"],
        affordances: ["openable"],
        placement_constraints: ["grounded"],
        allow_namespaced_extensions: true,
      },
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
      class: "wall_edge",
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

  it("updates selected connector id when the selected connector is renamed to an empty id", () => {
    const withConnector = addConnector(baseState(), "wall");

    const updated = updateConnector(withConnector, "wall", "connector_1", {
      connector_id: "",
    });

    expect(updated.selectedAssetId).toBe("wall");
    expect(updated.selectedConnectorId).toBe("");
    expect(updated.pack.assets[0].connectors[0].connector_id).toBe("");
  });

  it("does not change selection when renaming the same connector id on another asset", () => {
    const withConnector = addConnector(baseState(), "wall");
    const wall = withConnector.pack.assets[0];
    const door = {
      ...wall,
      asset_id: "door",
      source_path: "door.glb",
      content_hash: "sha256:def",
      display_name: "Door",
      connectors: [
        {
          ...wall.connectors[0],
          display_name: "Door Connector",
        },
      ],
    };
    const withSameConnectorIdOnAnotherAsset = {
      ...withConnector,
      pack: {
        ...withConnector.pack,
        assets: [wall, door],
      },
    };

    const updated = updateConnector(
      withSameConnectorIdOnAnotherAsset,
      "door",
      "connector_1",
      {
        connector_id: "door_connector",
      },
    );

    expect(updated.selectedAssetId).toBe("wall");
    expect(updated.selectedConnectorId).toBe("connector_1");
    expect(updated.pack.assets[1].connectors[0].connector_id).toBe(
      "door_connector",
    );
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

  it("updates connector classes and remaps connector/rule references", () => {
    const withClass = addConnectorClass(baseState(), "doorway", "Doorway");
    const withConnector = updateConnector(addConnector(withClass, "wall"), "wall", "connector_1", {
      class: "doorway",
    });
    const withRule = addCompatibilityRule(withConnector, "doorway", "doorway");

    const renamed = updateConnectorClass(withRule, 0, {
      class: "arch",
      display_name: "Arch",
    });

    expect(renamed.dirty).toBe(true);
    expect(renamed.pack.connector_classes[0]).toEqual({
      class: "arch",
      display_name: "Arch",
    });
    expect(renamed.pack.assets[0].connectors[0].class).toBe("arch");
    expect(renamed.pack.compatibility_rules[0]).toMatchObject({
      a_class: "arch",
      b_class: "arch",
    });
  });

  it("updates compatibility rules by index", () => {
    const withClass = addConnectorClass(baseState(), "doorway", "Doorway");
    const withRule = addCompatibilityRule(withClass, "doorway", "doorway");

    const updatedRule = updateCompatibilityRule(withRule, 0, {
      a_class: "doorway",
      b_class: "doorway",
      rotation: { kind: "free" },
    });

    expect(updatedRule.dirty).toBe(true);
    expect(updatedRule.pack.compatibility_rules[0].rotation).toEqual({
      kind: "free",
    });
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

  it("selects diagnostic targets, deletes rules/classes, and manages review flags", () => {
    const withClass = addConnectorClass(baseState(), "doorway", "Doorway");
    const withRule = addCompatibilityRule(withClass, "doorway", "doorway");
    const withConnector = addConnector(withRule, "wall");
    const selected = selectDiagnosticTarget(withConnector, {
      code: "x",
      severity: "error",
      message: "m",
      asset_id: "wall",
      connector_id: "connector_1",
    });
    expect(selected.selectedConnectorId).toBe("connector_1");

    const noRule = removeCompatibilityRule(withRule, 0);
    expect(noRule.pack.compatibility_rules).toHaveLength(0);

    const noClass = removeConnectorClass(withClass, 0);
    expect(noClass.pack.connector_classes).toHaveLength(0);

    const flagged = setReviewFlag(baseState(), "wall", "bounds_placeholder", true);
    expect(flagged.pack.assets[0].review_flags).toContain("bounds_placeholder");
    const cleared = clearAllReviewFlags(flagged, "wall");
    expect(cleared.pack.assets[0].review_flags).toEqual([]);
  });

  it("supports steps_deg rotation, duplicate, snap, and orientation helpers", () => {
    const withClass = addConnectorClass(baseState(), "wall_edge", "Wall Edge");
    const withRule = addCompatibilityRule(withClass, "wall_edge", "wall_edge");
    const stepped = updateCompatibilityRule(withRule, 0, {
      a_class: "wall_edge",
      b_class: "wall_edge",
      rotation: rotationFromKind("steps_deg"),
    });
    expect(stepped.pack.compatibility_rules[0].rotation).toEqual({
      kind: "steps_deg",
      values: [0, 90, 180, 270],
    });

    const withConnector = addConnector(withClass, "wall");
    const duplicated = duplicateConnector(withConnector, "wall", "connector_1");
    expect(duplicated.pack.assets[0].connectors).toHaveLength(2);
    expect(duplicated.selectedConnectorId).toBe("connector_2");

    const snapped = snapConnectorToBoundsFace(withConnector, "wall", "connector_1", "pos_z");
    expect(snapped.pack.assets[0].connectors[0].frame).toMatchObject({
      kind: "frame3d",
      position: [0, 0, 0.5],
    });
    expect(snapped.pack.assets[0].connectors[0].mating_axis).toBe("pos_z");

    const quat = eulerDegToQuat([0, 90, 0]);
    const euler = quatToEulerDeg(quat);
    expect(Math.abs(euler[1] - 90)).toBeLessThan(0.1);

    for (const input of [
      [30, 45, 60],
      [10, -20, 30],
      [-15, 25, -35],
    ] as const) {
      const q = eulerDegToQuat([...input]);
      const back = quatToEulerDeg(q);
      const q2 = eulerDegToQuat(back);
      const l1 =
        Math.abs(q[0] - q2[0]) +
        Math.abs(q[1] - q2[1]) +
        Math.abs(q[2] - q2[2]) +
        Math.abs(q[3] - q2[3]);
      // Quaternion may flip sign; also compare negated.
      const l1Neg =
        Math.abs(q[0] + q2[0]) +
        Math.abs(q[1] + q2[1]) +
        Math.abs(q[2] + q2[2]) +
        Math.abs(q[3] + q2[3]);
      expect(Math.min(l1, l1Neg)).toBeLessThan(1e-4);
      const e2 = quatToEulerDeg(q2);
      expect(Math.abs(back[0] - e2[0])).toBeLessThan(0.05);
      expect(Math.abs(back[1] - e2[1])).toBeLessThan(0.05);
      expect(Math.abs(back[2] - e2[2])).toBeLessThan(0.05);
    }

    expect(suggestClassFromName("Brick Wall")).toBe("wall_edge");

    const tagged = updateAssetMetadata(baseState(), "wall", {
      semantic_tags: ["modular"],
      affordances: ["cover"],
      placement_constraints: ["upright_only"],
    });
    expect(tagged.pack.assets[0].semantic_tags).toEqual(["modular"]);
    expect(tagged.dirty).toBe(true);
  });

  it("adds 2d connectors for sprites", () => {
    const state = baseState();
    state.pack.assets[0].asset_type = "sprite2d";
    const with2d = addConnector(state, "wall", "frame2d");
    expect(with2d.pack.assets[0].connectors[0].frame.kind).toBe("frame2d");
  });
});
