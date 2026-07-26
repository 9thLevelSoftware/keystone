import { useMemo, useState } from "react";

import { resolveAssemblyPlan } from "../tauriApi";
import type { EditorPackState, ResolvedScene } from "../types";

interface Props {
  state: EditorPackState;
  busy: boolean;
  onScene: (scene: ResolvedScene | null) => void;
  onStatus: (message: string) => void;
}

/**
 * Build a two-piece mate plan from compatible connectors and preview the result.
 */
export default function AssemblyPreview({
  state,
  busy,
  onScene,
  onStatus,
}: Props) {
  const assetsWithConnectors = useMemo(
    () => state.pack.assets.filter((a) => a.connectors.length > 0),
    [state.pack.assets],
  );

  const [rootId, setRootId] = useState(assetsWithConnectors[0]?.asset_id ?? "");
  const [placedId, setPlacedId] = useState(assetsWithConnectors[1]?.asset_id ?? "");

  const rootAsset = state.pack.assets.find((a) => a.asset_id === rootId);
  const placedAsset = state.pack.assets.find((a) => a.asset_id === placedId);

  const [rootConnectorId, setRootConnectorId] = useState(
    rootAsset?.connectors[0]?.connector_id ?? "",
  );
  const [placedConnectorId, setPlacedConnectorId] = useState(
    placedAsset?.connectors[0]?.connector_id ?? "",
  );

  async function previewMate() {
    if (!rootAsset || !placedAsset) {
      onStatus("Need two assets with connectors for assembly preview.");
      return;
    }
    const anchor = rootAsset.connectors.find(
      (c) => c.connector_id === rootConnectorId,
    );
    const placed = placedAsset.connectors.find(
      (c) => c.connector_id === placedConnectorId,
    );
    if (!anchor || !placed) {
      onStatus("Select connectors on both assets.");
      return;
    }

    try {
      const result = await resolveAssemblyPlan(state, {
        root_asset_id: rootId,
        operations: [
          {
            placed_asset_id: placedId,
            placed_connector_id: placedConnectorId,
            anchor_asset_id: rootId,
            anchor_connector_id: rootConnectorId,
            rotation_choice_deg: 0,
          },
        ],
      });
      onScene(result.scene);
      onStatus(
        `Assembly preview: ${placedId}.${placedConnectorId} → ${rootId}.${rootConnectorId}`,
      );
    } catch (error: unknown) {
      onScene(null);
      onStatus(
        error instanceof Error ? error.message : `Resolve failed: ${String(error)}`,
      );
    }
  }

  if (assetsWithConnectors.length < 1) {
    return (
      <section className="assembly-preview muted">
        <h3>Assembly preview</h3>
        <p>Run <strong>Analyze</strong> first to propose connectors, then mate two pieces here.</p>
      </section>
    );
  }

  return (
    <section className="assembly-preview">
      <h3>Assembly preview</h3>
      <p className="muted">
        Mate two connectors with the resolver and view the result.
      </p>
      <label>
        Root asset
        <select
          value={rootId}
          onChange={(e) => {
            setRootId(e.currentTarget.value);
            const asset = state.pack.assets.find(
              (a) => a.asset_id === e.currentTarget.value,
            );
            setRootConnectorId(asset?.connectors[0]?.connector_id ?? "");
          }}
        >
          {assetsWithConnectors.map((a) => (
            <option key={a.asset_id} value={a.asset_id}>
              {a.display_name}
            </option>
          ))}
        </select>
      </label>
      <label>
        Root connector
        <select
          value={rootConnectorId}
          onChange={(e) => setRootConnectorId(e.currentTarget.value)}
        >
          {(rootAsset?.connectors ?? []).map((c) => (
            <option key={c.connector_id} value={c.connector_id}>
              {c.display_name} ({c.class})
            </option>
          ))}
        </select>
      </label>
      <label>
        Placed asset
        <select
          value={placedId}
          onChange={(e) => {
            setPlacedId(e.currentTarget.value);
            const asset = state.pack.assets.find(
              (a) => a.asset_id === e.currentTarget.value,
            );
            setPlacedConnectorId(asset?.connectors[0]?.connector_id ?? "");
          }}
        >
          {assetsWithConnectors.map((a) => (
            <option key={a.asset_id} value={a.asset_id}>
              {a.display_name}
            </option>
          ))}
        </select>
      </label>
      <label>
        Placed connector
        <select
          value={placedConnectorId}
          onChange={(e) => setPlacedConnectorId(e.currentTarget.value)}
        >
          {(placedAsset?.connectors ?? []).map((c) => (
            <option key={c.connector_id} value={c.connector_id}>
              {c.display_name} ({c.class})
            </option>
          ))}
        </select>
      </label>
      <div className="toolbar">
        <button type="button" disabled={busy} onClick={() => void previewMate()}>
          Preview mate
        </button>
        <button type="button" disabled={busy} onClick={() => onScene(null)}>
          Clear assembly
        </button>
      </div>
    </section>
  );
}
