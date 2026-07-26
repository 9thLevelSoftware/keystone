import { useMemo, useState } from "react";

import {
  resolveAssemblyPlan,
  proposeAssembly,
  type ResolveErrorReport,
} from "../tauriApi";
import type { AssemblyPlan, EditorPackState, ResolvedScene } from "../types";

interface Props {
  state: EditorPackState;
  busy: boolean;
  onScene: (scene: ResolvedScene | null) => void;
  onStatus: (message: string) => void;
  onSelectConnector?: (assetId: string, connectorId: string | null) => void;
}

type Mode = "two-piece" | "pack" | "plan";

/**
 * Mate connectors (two-piece debug), auto-layout a multi-piece pack assembly,
 * or import a plan JSON and resolve with failure guidance.
 */
export default function AssemblyPreview({
  state,
  busy,
  onScene,
  onStatus,
  onSelectConnector,
}: Props) {
  const assetsWithConnectors = useMemo(
    () => state.pack.assets.filter((a) => a.connectors.length > 0),
    [state.pack.assets],
  );

  const [mode, setMode] = useState<Mode>("pack");
  const [maxPieces, setMaxPieces] = useState(8);
  const [rootId, setRootId] = useState(assetsWithConnectors[0]?.asset_id ?? "");
  const [placedId, setPlacedId] = useState(assetsWithConnectors[1]?.asset_id ?? "");
  const [lastOps, setLastOps] = useState<string[]>([]);
  const [planJson, setPlanJson] = useState("");
  const [lastError, setLastError] = useState<ResolveErrorReport | null>(null);

  const rootAsset = state.pack.assets.find((a) => a.asset_id === rootId);
  const placedAsset = state.pack.assets.find((a) => a.asset_id === placedId);

  const [rootConnectorId, setRootConnectorId] = useState(
    rootAsset?.connectors[0]?.connector_id ?? "",
  );
  const [placedConnectorId, setPlacedConnectorId] = useState(
    placedAsset?.connectors[0]?.connector_id ?? "",
  );

  function applyResolveError(error: ResolveErrorReport) {
    setLastError(error);
    onScene(null);
    if (error.asset_id && onSelectConnector) {
      onSelectConnector(error.asset_id, error.connector_id ?? null);
    }
    onStatus(
      `Resolve failed [${error.code}] (${error.fix_target}): ${error.guidance}`,
    );
  }

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
      if (result.error) {
        applyResolveError(result.error);
        return;
      }
      setLastError(null);
      if (result.scene) {
        onScene(result.scene);
      }
      setLastOps([
        `${placedId}.${placedConnectorId} → ${rootId}.${rootConnectorId}`,
      ]);
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

  async function previewPackAssembly() {
    if (assetsWithConnectors.length < 1) {
      onStatus("Run Analyze first to propose connectors.");
      return;
    }
    try {
      const result = await proposeAssembly(
        state,
        maxPieces,
        rootId || null,
      );
      if (result.error) {
        applyResolveError(result.error);
        const ops = result.report.plan.operations.map(
          (op) =>
            `${op.placed_asset_id}.${op.placed_connector_id} → ${op.anchor_asset_id}.${op.anchor_connector_id}`,
        );
        setLastOps(ops);
        return;
      }
      if (result.scene) {
        onScene(result.scene);
      } else {
        onScene(null);
      }
      setLastError(null);
      const ops = result.report.plan.operations.map(
        (op) =>
          `${op.placed_asset_id}.${op.placed_connector_id} → ${op.anchor_asset_id}.${op.anchor_connector_id}`,
      );
      setLastOps(ops);
      const placed = result.report.placed_asset_ids.length;
      const note = result.report.notes[0] ?? "";
      onStatus(
        `Pack assembly: ${placed} piece(s)` +
          (ops.length ? `, ${ops.length} attach(es)` : "") +
          (note ? ` — ${note}` : ""),
      );
    } catch (error: unknown) {
      onScene(null);
      onStatus(
        error instanceof Error
          ? error.message
          : `Pack assembly failed: ${String(error)}`,
      );
    }
  }

  async function resolveImportedPlan() {
    let plan: AssemblyPlan;
    try {
      plan = JSON.parse(planJson) as AssemblyPlan;
    } catch {
      onStatus("Plan JSON is not valid JSON.");
      return;
    }
    if (!plan.root_asset_id || !Array.isArray(plan.operations)) {
      onStatus("Plan must include root_asset_id and operations[].");
      return;
    }
    try {
      const result = await resolveAssemblyPlan(state, plan);
      if (result.error) {
        applyResolveError(result.error);
        return;
      }
      setLastError(null);
      if (result.scene) {
        onScene(result.scene);
      }
      const ops = plan.operations.map(
        (op) =>
          `${op.placed_asset_id}.${op.placed_connector_id} → ${op.anchor_asset_id}.${op.anchor_connector_id}`,
      );
      setLastOps(ops);
      onStatus(`Resolved plan with ${result.scene?.placements.length ?? 0} placement(s).`);
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
        <p>
          Run <strong>Analyze</strong> first to propose connectors, then preview
          a multi-piece pack layout here.
        </p>
      </section>
    );
  }

  return (
    <section className="assembly-preview">
      <h3>Assembly preview</h3>
      <div className="toolbar">
        <button
          type="button"
          className={mode === "pack" ? "active" : undefined}
          disabled={busy}
          onClick={() => setMode("pack")}
        >
          Pack assembly
        </button>
        <button
          type="button"
          className={mode === "two-piece" ? "active" : undefined}
          disabled={busy}
          onClick={() => setMode("two-piece")}
        >
          Two-piece mate
        </button>
        <button
          type="button"
          className={mode === "plan" ? "active" : undefined}
          disabled={busy}
          onClick={() => setMode("plan")}
        >
          Import plan
        </button>
      </div>

      {mode === "pack" ? (
        <>
          <p className="muted">
            Auto-connect unique kit pieces with the resolver (greedy layout).
            Each asset_id is used at most once; tile reuse is for external tools.
          </p>
          <label>
            Root asset
            <select
              value={rootId}
              onChange={(e) => setRootId(e.currentTarget.value)}
            >
              {assetsWithConnectors.map((a) => (
                <option key={a.asset_id} value={a.asset_id}>
                  {a.display_name}
                </option>
              ))}
            </select>
          </label>
          <label>
            Max pieces
            <input
              type="number"
              min={2}
              max={32}
              value={maxPieces}
              onChange={(e) =>
                setMaxPieces(
                  Math.max(2, Math.min(32, Number(e.currentTarget.value) || 8)),
                )
              }
            />
          </label>
          <div className="toolbar">
            <button
              type="button"
              disabled={busy}
              onClick={() => void previewPackAssembly()}
            >
              Auto layout pack
            </button>
            <button type="button" disabled={busy} onClick={() => onScene(null)}>
              Clear assembly
            </button>
          </div>
        </>
      ) : null}

      {mode === "two-piece" ? (
        <>
          <p className="muted">
            Mate two connectors with the resolver for precise debugging.
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
        </>
      ) : null}

      {mode === "plan" ? (
        <>
          <p className="muted">
            Paste an AssemblyPlan JSON from a vibe builder. On failure, the
            implicated asset/connector is selected with fix_pack vs fix_plan guidance.
          </p>
          <label>
            Plan JSON
            <textarea
              rows={8}
              value={planJson}
              onChange={(e) => setPlanJson(e.currentTarget.value)}
              placeholder='{"root_asset_id":"...","operations":[...]}'
            />
          </label>
          <div className="toolbar">
            <button
              type="button"
              disabled={busy || !planJson.trim()}
              onClick={() => void resolveImportedPlan()}
            >
              Resolve plan
            </button>
            <button type="button" disabled={busy} onClick={() => onScene(null)}>
              Clear assembly
            </button>
          </div>
        </>
      ) : null}

      {lastError ? (
        <div className="resolve-error-panel" role="alert">
          <strong>
            {lastError.code} · {lastError.fix_target}
          </strong>
          <p>{lastError.message}</p>
          <p>{lastError.guidance}</p>
          {lastError.asset_id ? (
            <p className="muted">
              Asset: {lastError.asset_id}
              {lastError.connector_id ? ` · ${lastError.connector_id}` : ""}
            </p>
          ) : null}
        </div>
      ) : null}

      {lastOps.length > 0 && (
        <ul className="assembly-ops muted">
          {lastOps.map((op) => (
            <li key={op}>{op}</li>
          ))}
        </ul>
      )}
    </section>
  );
}
